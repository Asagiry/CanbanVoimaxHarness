//! Stage Movement & Execution Cancellation Logic (§ 2.2, § 2.6, § 2.18, § 2.22)

use std::collections::HashSet;
use std::path::Path;
use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::agent::Task;
use crate::commands::state::AppState;
use crate::config::{load_boards_config, load_pipeline_config, resolve_actual_repo_root};
use crate::commands::tasks::healing::{stage_id_matches, task_id_matches};
use crate::commands::tasks::runner::internal_abort_agent;
use crate::commands::tasks::storage::try_load_tasks_from_disk;
use crate::commands::tasks::timing::freeze_stage_duration;
use super::transition_hooks::{
    auto_complete_task_todos, sync_single_task_in_app_state, trigger_column_on_enter_script,
    TaskTransitionPipeline,
};
use super::resolve::{resolve_board_for_stage, resolve_source_and_target_columns};
use super::subtasks::propagate_parent_subtask_advance;

#[allow(clippy::too_many_arguments)]
pub async fn internal_move_task_stage(
    root: &Path,
    task_id: &str,
    target_stage_id: &str,
    verdict: Option<String>,
    status_override: Option<String>,
    handover_summary: Option<String>,
    app_handle: Option<&AppHandle>,
    is_autonomous_advance: bool,
    board_id_override: Option<String>,
) -> Result<Task, String> {
    let actual_root = resolve_actual_repo_root(root);
    let root = &actual_root;
    crate::logger::cvh_log_with_details(
        Some(root),
        "INFO",
        "MOVE_STAGE",
        &format!(
            "internal_move_task_stage: task_id='{}', target_stage='{}', board_override={:?}, verdict={:?}, auto_adv={}",
            task_id, target_stage_id, board_id_override, verdict, is_autonomous_advance
        ),
        Some(&format!("actual_root='{}'", root.display())),
    );
    let mut initial_tasks = try_load_tasks_from_disk(root).await?;

    // In-memory fallback: if task was just created and not yet on disk,
    // or if memory has tasks not yet on disk, retrieve from AppState (§ 2.22).
    if !initial_tasks.iter().any(|t| task_id_matches(&t.id, task_id)) {
        crate::logger::cvh_log(
            Some(root),
            "WARN",
            "MOVE_STAGE",
            &format!("Task '{}' not found in {} disk tasks. Checking in-memory AppState tasks...", task_id, initial_tasks.len()),
        );
        if let Some(app) = app_handle {
            if let Some(state) = app.try_state::<AppState>() {
                let guard = state.tasks.lock().unwrap_or_else(|p| p.into_inner());
                for mem_t in guard.iter() {
                    if !initial_tasks.iter().any(|t| task_id_matches(&t.id, &mem_t.id)) {
                        initial_tasks.push(mem_t.clone());
                    }
                }
            }
        }
    }

    let mut moving_task = match initial_tasks.iter().find(|t| task_id_matches(&t.id, task_id)).cloned() {
        Some(t) => t,
        None => {
            let disk_ids: Vec<String> = initial_tasks.iter().map(|t| t.id.clone()).collect();
            let err_msg = format!("Task {} not found", task_id);
            crate::logger::cvh_log_with_details(
                Some(root),
                "ERROR",
                "MOVE_STAGE",
                &format!("REJECTED move_task_stage: {}", err_msg),
                Some(&format!("Available tasks on disk/memory (count: {}): [{}]", disk_ids.len(), disk_ids.join(", "))),
            );
            return Err(err_msg);
        }
    };
    drop(initial_tasks);

    // --- Board resolution & fallback (support alias and fallback when board_id missing/stale) ---
    // ZERO INVISIBLE TASKS (§ 2.18)
    let boards_cfg = load_boards_config(root).await.ok();
    let (effective_board, assigned_board_id) = resolve_board_for_stage(
        target_stage_id,
        moving_task.board_id.as_deref(),
        board_id_override.as_deref(),
        boards_cfg.as_ref(),
    );
    if let Some(bid) = assigned_board_id {
        moving_task.board_id = Some(bid);
    }

    // --- on_finish / on_leave hooks of source column ---
    let source_stage_id = moving_task.stage_id.clone();
    let (source_col, target_col) = resolve_source_and_target_columns(
        root,
        effective_board.as_ref(),
        &source_stage_id,
        target_stage_id,
    )
    .await?;

    let canonical_target_stage_id = target_col.id.clone();
    let is_backlog = target_col.is_intake == Some(true)
        || stage_id_matches(&canonical_target_stage_id, "backlog");
    let is_terminal_col = target_col.terminal == Some(true)
        || stage_id_matches(&canonical_target_stage_id, "ready")
        || stage_id_matches(&canonical_target_stage_id, "done");
    let target_has_autorun = !is_backlog && !is_terminal_col && (
        target_col.auto_run == Some(true)
            || (target_col.auto_run.is_none() && target_col.persona.is_some())
    );
    let was_running = moving_task.status.as_deref() == Some("running");

    // Immediate Cancellation (§ 2 specifications):
    if !is_autonomous_advance && (is_backlog || !target_has_autorun || was_running) {
        if let Some(app) = app_handle {
            if let Some(state_ref) = app.try_state::<AppState>() {
                let _ = internal_abort_agent(root, task_id, state_ref.inner(), Some(app)).await;
            }
        }
    }

    let is_source_intake = {
        let cols = if let Some(b) = &effective_board {
            b.stages.clone()
        } else if let Some(bc) = &boards_cfg {
            bc.boards.iter().find(|b| b.id == bc.active_board_id).map(|b| b.stages.clone()).unwrap_or_default()
        } else if let Ok(pipe) = load_pipeline_config(root).await {
            pipe.stages
        } else {
            Vec::new()
        };
        source_stage_id.is_empty()
            || cols
                .iter()
                .position(|c| stage_id_matches(&c.id, &source_stage_id))
                .map(|idx| idx == 0 || cols[idx].is_intake == Some(true))
                .unwrap_or(false)
    };

    let pipeline = TaskTransitionPipeline::new(
        root,
        source_col.as_ref(),
        &target_col,
        is_source_intake,
        verdict.clone(),
        true, // emit_task_moved
        app_handle,
    );
    pipeline.execute(&mut moving_task).await?;

    // Trigger on_enter_script if configured on target column
    trigger_column_on_enter_script(
        &target_col,
        &mut moving_task,
        &canonical_target_stage_id,
        verdict.as_deref(),
        root,
        app_handle,
    )
    .await;

    // --- LOST UPDATE & RESURRECTION GUARD ---
    // Re-load the fresh task list from SQLite AFTER all async hooks complete.
    // Do NOT resurrect the task from memory if it was deleted while hooks ran.
    let mut tasks = try_load_tasks_from_disk(root).await?;

    let task_idx = match tasks.iter().position(|t| task_id_matches(&t.id, &moving_task.id)) {
        Some(idx) => idx,
        None => {
            if let Some(app) = app_handle {
                if let Some(state) = app.try_state::<AppState>() {
                    if state.is_same_root(root) {
                        let mut guard = state.tasks.lock().unwrap_or_else(|p| p.into_inner());
                        guard.retain(|t| !task_id_matches(&t.id, &moving_task.id));
                    }
                }
            }
            return Err(format!("Task {} not found after stage transition hooks (may have been deleted)", task_id));
        }
    };

    let old_stage = source_stage_id.clone();
    {
        let current_task = &mut tasks[task_idx];
        current_task.board_id = moving_task.board_id.clone();
        current_task.worktree_path = moving_task.worktree_path.clone();
        current_task.branch = moving_task.branch.clone();
        current_task.hook_states = moving_task.hook_states.clone();
        current_task.stage_id = canonical_target_stage_id.clone();
        if verdict.is_some() {
            current_task.verdict = verdict.clone();
        }
        if let Some(summary) = handover_summary {
            current_task.handover_summary = Some(summary);
        }
        if is_backlog {
            current_task.status = Some("idle".to_string());
        } else if let Some(st) = status_override {
            current_task.status = Some(st);
        } else if is_terminal_col {
            current_task.status = Some("done".to_string());
        } else if target_has_autorun {
            current_task.status = Some("running".to_string());
        } else {
            current_task.status = Some("idle".to_string());
        }
        if is_terminal_col {
            current_task.completed_at = Some(Utc::now().timestamp_millis());
            current_task.waiting_for_subtask_id = None;
        } else if current_task.status.as_deref() == Some("done") {
            freeze_stage_duration(current_task, Utc::now().timestamp_millis());
        } else {
            current_task.completed_at = None;
        }
        auto_complete_task_todos(root, current_task, app_handle);
        current_task.updated_at = Utc::now().timestamp_millis();
        if old_stage != canonical_target_stage_id || current_task.stage_entered_at.is_none() {
            current_task.stage_entered_at = Some(Utc::now().timestamp_millis());
        }
    }

    if old_stage != canonical_target_stage_id {
        let cur_id = tasks[task_idx].id.clone();
        let max_pos = tasks
            .iter()
            .filter(|t| !task_id_matches(&t.id, &cur_id) && stage_id_matches(&t.stage_id, &canonical_target_stage_id))
            .filter_map(|t| t.position)
            .fold(0.0f64, f64::max);
        tasks[task_idx].position = Some(max_pos + 1000.0);
    } else if tasks[task_idx].position.is_none() {
        tasks[task_idx].position = Some(0.0);
    }

    let mut mutated_ids: HashSet<String> = HashSet::new();
    mutated_ids.insert(tasks[task_idx].id.clone());

    let updated = tasks[task_idx].clone();
    let parent_id_opt = updated.parent_id.clone();

    // If a top-level parent task moves to terminal, ensure all child subtasks are also marked done (§ 3)
    if is_terminal_col && parent_id_opt.is_none() {
        let cur_id = updated.id.clone();
        for t in tasks.iter_mut() {
            if task_id_matches(&t.id, &cur_id) {
                t.waiting_for_subtask_id = None;
                t.status = Some("done".to_string());
                if t.completed_at.is_none() {
                    t.completed_at = Some(Utc::now().timestamp_millis());
                }
                t.updated_at = Utc::now().timestamp_millis();
                auto_complete_task_todos(root, t, app_handle);
                mutated_ids.insert(t.id.clone());
            }
            if t.parent_id.as_deref() == Some(&cur_id) {
                t.status = Some("done".to_string());
                if t.completed_at.is_none() {
                    t.completed_at = Some(Utc::now().timestamp_millis());
                }
                t.updated_at = Utc::now().timestamp_millis();
                auto_complete_task_todos(root, t, app_handle);
                mutated_ids.insert(t.id.clone());
            }
        }
    }

    // Relational symmetry (§ 2.7 & § 2.22):
    let mut parent_moved_events: Vec<serde_json::Value> = Vec::new();
    let mut parent_created_tasks: Vec<Task> = Vec::new();
    let mut parent_autorun_ids: Vec<String> = Vec::new();
    let mut parent_hook_events: Vec<serde_json::Value> = Vec::new();

    if verdict.as_deref() == Some("CHANGES_REQUESTED") {
        // 1. When CHANGES_REQUESTED is emitted by a subtask, unlock the parent task in its development stage:
        if let Some(ref pid) = parent_id_opt {
            if let Some(parent_idx) = tasks.iter().position(|t| &t.id == pid) {
                if tasks[parent_idx].status.as_deref() == Some("waiting_subtask")
                    || tasks[parent_idx].waiting_for_subtask_id.as_deref() == Some(task_id)
                {
                    tasks[parent_idx].status = Some("idle".to_string());
                    tasks[parent_idx].verdict = Some("CHANGES_REQUESTED".to_string());
                    tasks[parent_idx].waiting_for_subtask_id = None;
                    tasks[parent_idx].updated_at = Utc::now().timestamp_millis();
                    mutated_ids.insert(tasks[parent_idx].id.clone());
                    let parent_clone = tasks[parent_idx].clone();
                    parent_moved_events.push(serde_json::json!({
                        "taskId": pid,
                        "fromStage": &parent_clone.stage_id,
                        "toStage": &parent_clone.stage_id,
                        "stageId": &parent_clone.stage_id,
                        "status": parent_clone.status,
                        "verdict": parent_clone.verdict,
                        "task": &parent_clone
                    }));
                }
            }
        }
    } else if is_terminal_col {
        // 2. POSITIVE APPROVAL / TERMINAL ADVANCE:
        // When a child subtask advances to a terminal column (ready/done),
        // propagate advancement to the parent task (and grandparents recursively)
        // advancing them past the subtask column into the next pipeline stage (§ 2.2, § 2.7 & § 2.22).
        let pipeline_opt = load_pipeline_config(root).await.ok();
        let parent_res = propagate_parent_subtask_advance(
            &mut tasks,
            task_id,
            &old_stage,
            verdict.as_deref(),
            boards_cfg.as_ref(),
            pipeline_opt.as_ref(),
        );
        parent_moved_events = parent_res.moved_events;
        parent_created_tasks = parent_res.created_tasks;
        parent_autorun_ids = parent_res.autorun_task_ids;
        parent_hook_events = parent_res.hook_finish_events;

        for mid in parent_res.mutated_task_ids {
            mutated_ids.insert(mid);
        }

        for t in tasks.iter_mut() {
            if mutated_ids.contains(&t.id) && t.status.as_deref() == Some("done") {
                auto_complete_task_todos(root, t, app_handle);
            }
        }
    }

    for ctask in &parent_created_tasks {
        mutated_ids.insert(ctask.id.clone());
    }

    let mut tasks_to_persist: Vec<Task> = Vec::new();
    for t in &mut tasks {
        if mutated_ids.contains(&t.id) {
            if t.project_path.is_none() && !root.as_os_str().is_empty() {
                t.project_path = Some(root.to_string_lossy().to_string());
            }
            tasks_to_persist.push(t.clone());
        }
    }
    for ctask in &mut parent_created_tasks {
        if ctask.project_path.is_none() && !root.as_os_str().is_empty() {
            ctask.project_path = Some(root.to_string_lossy().to_string());
        }
        tasks_to_persist.push(ctask.clone());
    }

    if let Err(e) = crate::db::db_upsert_all_tasks(root, &tasks_to_persist) {
        crate::logger::cvh_log(
            Some(root),
            "ERROR",
            "MOVE_STAGE",
            &format!("Failed to batch upsert tasks during move_stage: {}", e),
        );
        return Err(e);
    }

    // Update only the mutated/created tasks in AppState without overwriting concurrent updates to other tasks
    for t in &tasks_to_persist {
        sync_single_task_in_app_state(root, t, app_handle);
    }

    crate::logger::cvh_log_with_details(
        Some(root),
        "INFO",
        "MOVE_STAGE",
        &format!(
            "Task '{}' stage successfully updated from '{}' to '{}' (status: {:?}, board: {:?})",
            task_id, old_stage, canonical_target_stage_id, updated.status, updated.board_id
        ),
        Some(&format!("worktree={:?}, branch={:?}", updated.worktree_path, updated.branch)),
    );

    let final_task = tasks
        .iter()
        .find(|t| task_id_matches(&t.id, task_id))
        .cloned()
        .unwrap_or(updated);

    if let Some(app) = app_handle {
        let project_path_str = root.to_string_lossy().to_string();
        if let Err(e) = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks, "projectPath": &project_path_str })) {
            eprintln!("CVH [WARN]: Failed to emit tasks:updated in internal_move_task_stage: {}", e);
        }
        if let Err(e) = app.emit(
            "task:moved",
            serde_json::json!({
                "taskId": task_id,
                "fromStage": old_stage,
                "toStage": &canonical_target_stage_id,
                "stageId": &canonical_target_stage_id,
                "status": final_task.status,
                "verdict": final_task.verdict,
                "task": &final_task,
                "projectPath": &project_path_str,
            }),
        ) {
            eprintln!("CVH [WARN]: Failed to emit task:moved in internal_move_task_stage: {}", e);
        }
        for ctask in &parent_created_tasks {
            let _ = app.emit("task:created", serde_json::json!({ "task": ctask, "projectPath": &project_path_str }));
            let _ = app.emit("task:updated", serde_json::json!({ "task": ctask, "projectPath": &project_path_str }));
        }
        for p_event in &parent_moved_events {
            let _ = app.emit("task:moved", p_event);
            if let Some(ptask) = p_event.get("task") {
                let _ = app.emit("task:updated", serde_json::json!({ "task": ptask, "projectPath": &project_path_str }));
            }
        }
        if target_has_autorun {
            let _ = app.emit("agent:autorun", serde_json::json!({ "taskId": task_id, "projectPath": &project_path_str }));
        }
        for auto_pid in parent_autorun_ids {
            let _ = app.emit("agent:autorun", serde_json::json!({ "taskId": auto_pid, "projectPath": &project_path_str }));
        }
        for hook_event in &parent_hook_events {
            if hook_event.get("_hookStart").and_then(|v| v.as_bool()) == Some(true) {
                let _ = app.emit("hook:start", hook_event);
            } else {
                let _ = app.emit("hook:finish", hook_event);
            }
        }
    }

    Ok(final_task)
}

pub async fn move_task_stage(
    task_id: String,
    target_stage_id: String,
    verdict: Option<String>,
    handover_summary: Option<String>,
    board_id: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    let root = state.get_root();
    crate::logger::cvh_log_with_details(
        Some(&root),
        "INFO",
        "MOVE_STAGE",
        &format!(
            "move_task_stage IPC called: task_id='{}', target_stage='{}', board_id={:?}, verdict={:?}",
            task_id, target_stage_id, board_id, verdict
        ),
        Some(&format!("root='{}'", root.display())),
    );
    let is_intake = stage_id_matches(&target_stage_id, "backlog") || {
        let boards_cfg = crate::config::load_boards_config(&root).await.ok();
        boards_cfg.as_ref().is_some_and(|cfg| {
            let active = cfg.boards.iter().find(|b| b.id == cfg.active_board_id);
            active.is_some_and(|b| {
                b.stages.iter().find(|c| stage_id_matches(&c.id, &target_stage_id)).is_some_and(|c| c.is_intake == Some(true))
            })
        })
    };
    let is_running_or_active = {
        let map = state.active_agents.lock().unwrap_or_else(|p| p.into_inner());
        let tid_trimmed = task_id.trim();
        map.keys().any(|k| {
            task_id_matches(k, tid_trimmed)
                || k.to_ascii_lowercase().starts_with(&format!("{}.", tid_trimmed.to_ascii_lowercase()))
                || k.to_ascii_lowercase().starts_with(&format!("{}:sub:", tid_trimmed.to_ascii_lowercase()))
                || k.to_ascii_lowercase().starts_with(&format!("{}.sub-", tid_trimmed.to_ascii_lowercase()))
        })
    };

    if is_intake || is_running_or_active {
        let _ = internal_abort_agent(&root, &task_id, &state, Some(&app_handle)).await;
    }

    internal_move_task_stage(
        &root,
        &task_id,
        &target_stage_id,
        verdict,
        None,
        handover_summary,
        Some(&app_handle),
        false,
        board_id,
    )
    .await
}

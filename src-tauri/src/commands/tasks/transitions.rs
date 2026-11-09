//! Task Pipeline Transitions & Stage Progression Subsystem (§ 2.2, § 2.6, § 2.7, § 2.27)
//! Extracted from commands/tasks.rs per ub-mode Clean Code principles.

use std::collections::HashSet;
use std::path::Path;
use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::agent::Task;
use crate::commands::dialogs::append_task_dialog_activity;
use crate::commands::state::AppState;
use crate::config::{load_boards_config, load_pipeline_config, resolve_actual_repo_root};
use super::healing::{stage_id_matches, task_id_matches};
use super::lifecycle::internal_create_subtask_with_hook_states;
use super::timing::freeze_stage_duration;
use super::runner::internal_abort_agent;
use super::storage::{get_next_subtask_index, load_tasks_from_disk};

#[path = "transition_hooks.rs"]
pub mod transition_hooks;
pub use transition_hooks::*;

#[derive(Debug, Default)]
pub struct ParentAdvanceResult {
    pub moved_events: Vec<serde_json::Value>,
    pub created_tasks: Vec<Task>,
    pub autorun_task_ids: Vec<String>,
    pub hook_finish_events: Vec<serde_json::Value>,
    pub mutated_task_ids: Vec<String>,
}

/// Propagates advancement of completed/terminal subtasks up the hierarchy to parent tasks (§ 2.2 & § 2.7).
/// Universal relay progression: if next stage is non-terminal, parent spawns next isolated child subtask;
/// if next stage is terminal (ready/done), parent moves to ready with status done.
pub fn propagate_parent_subtask_advance(
    tasks: &mut Vec<Task>,
    completed_subtask_id: &str,
    subtask_source_stage: &str,
    verdict: Option<&str>,
    boards_cfg: Option<&crate::config::BoardsConfig>,
    pipeline: Option<&crate::config::PipelineConfig>,
) -> ParentAdvanceResult {
    let mut moved_events = Vec::new();
    let mut created_tasks = Vec::new();
    let mut autorun_task_ids = Vec::new();
    let mut hook_finish_events = Vec::new();
    let mut mutated_task_ids = Vec::new();

    let mut current_child_id = completed_subtask_id.to_string();
    let mut current_source_stage = subtask_source_stage.to_string();

    // 1. Ensure the completed subtask has status "done"
    if let Some(sub_idx) = tasks.iter().position(|t| task_id_matches(&t.id, completed_subtask_id)) {
        tasks[sub_idx].status = Some("done".to_string());
        if tasks[sub_idx].completed_at.is_none() {
            tasks[sub_idx].completed_at = Some(Utc::now().timestamp_millis());
        }
        tasks[sub_idx].updated_at = Utc::now().timestamp_millis();
        mutated_task_ids.push(tasks[sub_idx].id.clone());
    }

    while let Some(parent_idx) = tasks.iter().position(|t| {
        t.waiting_for_subtask_id.as_deref() == Some(&current_child_id)
            || (t.status.as_deref() == Some("waiting_subtask")
                && tasks
                    .iter()
                    .find(|sub| sub.id == current_child_id)
                    .and_then(|sub| sub.parent_id.as_deref())
                    == Some(&t.id))
            || tasks
                .iter()
                .find(|sub| sub.id == current_child_id)
                .and_then(|sub| sub.parent_id.as_deref())
                == Some(&t.id)
    }) {
        let parent_id = tasks[parent_idx].id.clone();
        let old_parent_stage = tasks[parent_idx].stage_id.clone();
        let parent_board_id = tasks[parent_idx].board_id.clone();
        let parent_title = tasks[parent_idx].title.clone();

        // Resolve stages for parent task's board
        let cols = {
            let from_boards = boards_cfg.and_then(|bc| {
                if let Some(ref bid) = parent_board_id {
                    if let Some(b) = bc.boards.iter().find(|b| &b.id == bid) {
                        return Some(b.stages.clone());
                    }
                }
                if let Some(b) = bc.boards.iter().find(|b| b.id == bc.active_board_id) {
                    return Some(b.stages.clone());
                }
                bc.boards.first().map(|b| b.stages.clone())
            });
            from_boards.or_else(|| pipeline.map(|p| p.stages.clone())).unwrap_or_default()
        };

        // Find the next stage for the parent task:
        // Look up index of the subtask's source stage (e.g. "dev" or "review"):
        let sub_stage_idx = cols
            .iter()
            .position(|c| stage_id_matches(&c.id, &current_source_stage))
            .or_else(|| cols.iter().position(|c| stage_id_matches(&c.id, &old_parent_stage)));

        let next_col_opt = if let Some(idx) = sub_stage_idx {
            if idx + 1 < cols.len() {
                Some(cols[idx + 1].clone())
            } else {
                cols.last().cloned()
            }
        } else {
            cols.iter()
                .find(|c| c.terminal == Some(true) || stage_id_matches(&c.id, "ready") || stage_id_matches(&c.id, "done"))
                .cloned()
                .or_else(|| cols.last().cloned())
        };

        let next_col = match next_col_opt {
            Some(c) => c,
            None => break,
        };

        let is_terminal = next_col.terminal == Some(true)
            || stage_id_matches(&next_col.id, "ready")
            || stage_id_matches(&next_col.id, "done");

        if is_terminal {
            let old_stage_id = tasks[parent_idx].stage_id.clone();
            let old_entered = tasks[parent_idx].stage_entered_at;
            tasks[parent_idx].waiting_for_subtask_id = None;
            tasks[parent_idx].stage_id = next_col.id.clone();
            tasks[parent_idx].verdict = Some(verdict.unwrap_or("APPROVED").to_string());
            tasks[parent_idx].status = Some("done".to_string());
            super::timing::freeze_stage_duration_for(
                &mut tasks[parent_idx],
                &old_stage_id,
                old_entered,
                Utc::now().timestamp_millis(),
            );
            tasks[parent_idx].updated_at = Utc::now().timestamp_millis();
            mutated_task_ids.push(parent_id.clone());

            // Also mark any child subtasks of this parent as done so they collapse into parent (§ 3)
            for sub in tasks.iter_mut() {
                if sub.parent_id.as_deref() == Some(&parent_id) {
                    sub.status = Some("done".to_string());
                    if sub.completed_at.is_none() {
                        sub.completed_at = Some(Utc::now().timestamp_millis());
                    }
                    sub.updated_at = Utc::now().timestamp_millis();
                    mutated_task_ids.push(sub.id.clone());
                }
            }

            let parent_clone = tasks[parent_idx].clone();
            moved_events.push(serde_json::json!({
                "taskId": parent_id,
                "fromStage": old_parent_stage,
                "toStage": &next_col.id,
                "stageId": &next_col.id,
                "status": parent_clone.status,
                "verdict": parent_clone.verdict,
                "task": &parent_clone
            }));

            // If parent task also advanced to a terminal column, check if grandparent was waiting on it!
            current_child_id = parent_id;
            current_source_stage = next_col.id;
        } else {
            // Next stage is not terminal -> universal progression spawns next isolated child subtask!
            let next_subtask_idx = get_next_subtask_index(tasks, &parent_id);
            let new_subtask_id = format!("{}.{}", parent_id, next_subtask_idx);
            let now = Utc::now().timestamp_millis();

            tasks[parent_idx].waiting_for_subtask_id = Some(new_subtask_id.clone());
            tasks[parent_idx].status = Some("done".to_string());
            freeze_stage_duration(&mut tasks[parent_idx], now);
            tasks[parent_idx].updated_at = now;
            mutated_task_ids.push(parent_id.clone());

            let has_autorun = next_col.persona.is_some() || next_col.auto_run == Some(true);
            let subtask_status = if has_autorun { "running" } else { "idle" };

            let parent_wt = tasks[parent_idx].worktree_path.clone();
            let parent_branch = tasks[parent_idx].branch.clone();
            let parent_model = tasks[parent_idx].model.clone();
            let parent_skip_wt = tasks[parent_idx].skip_worktree;
            let parent_priority = tasks[parent_idx].priority.clone();

            let next_subtask_pos = tasks
                .iter()
                .filter(|t| t.stage_id == next_col.id)
                .filter_map(|t| t.position)
                .fold(0.0f64, f64::max) + 1000.0;

            let new_subtask = Task {
                id: new_subtask_id.clone(),
                title: format!("{} ({})", parent_title, next_col.title),
                description: format!(
                    "### Подзадача этапа: {}\nРодительская задача: {}\n\n{}",
                    next_col.title, parent_id, tasks[parent_idx].description
                ),
                stage_id: next_col.id.clone(),
                board_id: tasks[parent_idx].board_id.clone(),
                worktree_path: parent_wt,
                branch: parent_branch,
                verdict: None,
                custom_flow: None,
                status: Some(subtask_status.to_string()),
                model: parent_model.clone(),
                provider_id: tasks[parent_idx].provider_id.clone(),
                reasoning_effort: tasks[parent_idx].reasoning_effort.clone(),
                parent_id: Some(parent_id.clone()),
                subtask_index: Some(next_subtask_idx),
                waiting_for_subtask_id: None,
                handover_summary: None,
                cover_image: None,
                attachments: None,
                todos: None,
                priority: parent_priority,
                tags: None,
                current_tokens: None,
                skip_worktree: parent_skip_wt,
                created_at: now,
                updated_at: now,
                stage_entered_at: Some(now),
                completed_at: None,
                position: Some(next_subtask_pos),
                hook_states: tasks[parent_idx].hook_states.clone(),
                project_path: tasks[parent_idx].project_path.clone(),
                stage_durations: None,
                total_elapsed: None,
            };

            if has_autorun {
                autorun_task_ids.push(new_subtask_id.clone());
            }

            if let Some(hook_states) = tasks[parent_idx].hook_states.clone() {
                if let Some(enter) = hook_states.get("on_enter").cloned() {
                    match enter.get("status").and_then(|s| s.as_str()) {
                        Some("success") | Some("failed") => {
                            let success = enter.get("status").and_then(|s| s.as_str()) == Some("success");
                            hook_finish_events.push(serde_json::json!({
                                "taskId": new_subtask_id,
                                "stageId": next_col.id,
                                "hookType": "on_enter",
                                "success": success,
                                "exitCode": enter.get("exitCode"),
                                "stdout": enter.get("stdout").and_then(|v| v.as_str()).unwrap_or(""),
                                "stderr": enter.get("stderr").and_then(|v| v.as_str()).unwrap_or(""),
                                "error": enter.get("error"),
                                "hookName": enter.get("hookName").and_then(|v| v.as_str()).unwrap_or("on_enter"),
                            }));
                        }
                        Some("running") => {
                            hook_finish_events.push(serde_json::json!({
                                "taskId": new_subtask_id,
                                "stageId": next_col.id,
                                "hookType": "on_enter",
                                "hookName": enter.get("hookName").and_then(|v| v.as_str()).unwrap_or("on_enter"),
                                "description": enter.get("description"),
                                "_hookStart": true,
                            }));
                        }
                        _ => {}
                    }
                }
            }

            let parent_clone = tasks[parent_idx].clone();
            moved_events.push(serde_json::json!({
                "taskId": parent_id,
                "fromStage": &old_parent_stage,
                "toStage": &old_parent_stage,
                "stageId": &old_parent_stage,
                "status": "idle",
                "waitingForSubtaskId": &new_subtask_id,
                "verdict": parent_clone.verdict,
                "task": &parent_clone
            }));

            created_tasks.push(new_subtask.clone());
            tasks.push(new_subtask);

            // Parent is now waiting on the newly created subtask; break recursion
            break;
        }
    }

    ParentAdvanceResult {
        moved_events,
        created_tasks,
        autorun_task_ids,
        hook_finish_events,
        mutated_task_ids,
    }
}

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
    let mut initial_tasks = load_tasks_from_disk(root).await;

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
                if let Ok(guard) = state.tasks.lock() {
                    for mem_t in guard.iter() {
                        if !initial_tasks.iter().any(|t| task_id_matches(&t.id, &mem_t.id)) {
                            initial_tasks.push(mem_t.clone());
                        }
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
    // ZERO INVISIBLE TASKS (§ 2.18):
    // 1. If explicit board_id_override is provided and contains target stage, stay on it.
    // 2. Otherwise if moving_task.board_id contains target stage, preserve it (NO accidental board hopping!).
    // 3. Otherwise if active_board contains target stage, use active_board.
    // 4. Only if no priority board contains the stage, search any other board with matching target stage.
    let boards_cfg = load_boards_config(root).await.ok();
    let mut effective_board: Option<crate::config::BoardDef> = None;

    if let Some(ref bc) = boards_cfg {
        let active_board = bc.boards.iter().find(|b| b.id == bc.active_board_id).cloned();
        let override_board = board_id_override.as_deref().and_then(|bid| {
            bc.boards.iter().find(|b| b.id == bid).cloned()
        });
        let current_board = moving_task.board_id.as_deref().and_then(|bid| {
            bc.boards.iter().find(|b| b.id == bid).cloned()
        });

        let board_has_target = |b: &crate::config::BoardDef| -> bool {
            b.stages.iter().any(|c| c.id == target_stage_id || stage_id_matches(&c.id, target_stage_id))
        };

        // 1. Explicit override board requested by caller (e.g. active board during user move)
        if let Some(ref ob) = override_board {
            if board_has_target(ob) {
                effective_board = Some(ob.clone());
                moving_task.board_id = Some(ob.id.clone());
            }
        }

        // 2. Task's current board if it contains the target stage (preserves task on its board)
        if effective_board.is_none() {
            if let Some(ref cb) = current_board {
                if board_has_target(cb) {
                    effective_board = Some(cb.clone());
                    // Keep moving_task.board_id intact!
                }
            }
        }

        // 3. Active board if it contains the target stage
        if effective_board.is_none() {
            if let Some(ref ab) = active_board {
                if board_has_target(ab) {
                    effective_board = Some(ab.clone());
                    moving_task.board_id = Some(ab.id.clone());
                }
            }
        }

        // 4. Any board that defines the target stage (cross-board pipeline advance)
        if effective_board.is_none() {
            let any_with_target = bc.boards.iter().find(|b| board_has_target(b)).cloned();
            if let Some(ref b) = any_with_target {
                effective_board = Some(b.clone());
                moving_task.board_id = Some(b.id.clone());
            } else {
                effective_board = active_board.or_else(|| bc.boards.first().cloned());
                if let Some(ref b) = effective_board {
                    moving_task.board_id = Some(b.id.clone());
                } else {
                    moving_task.board_id = Some(bc.active_board_id.clone());
                }
            }
        }
    }

    // --- on_finish / on_leave hooks of source column ---
    let source_stage_id = moving_task.stage_id.clone();
    let source_col: Option<crate::config::PipelineStage> = {
        let from_board = effective_board.as_ref().and_then(|b| {
            b.stages.iter().find(|c| stage_id_matches(&c.id, &source_stage_id)).cloned()
        });
        match from_board {
            Some(c) => Some(c),
            None => load_pipeline_config(root)
                .await
                .ok()
                .and_then(|p| p.stages.into_iter().find(|c| stage_id_matches(&c.id, &source_stage_id))),
        }
    };

    let target_col = {
        let col_from_board = effective_board.as_ref().and_then(|b| {
            b.stages.iter().find(|c| stage_id_matches(&c.id, target_stage_id)).cloned()
        });
        match col_from_board {
            Some(c) => c,
            None => {
                let pipeline = load_pipeline_config(root)
                    .await
                    .unwrap_or_else(|_| crate::config::default_pipeline_config());
                let col_from_pipeline = pipeline
                    .stages
                    .iter()
                    .find(|c| stage_id_matches(&c.id, target_stage_id))
                    .cloned();

                match col_from_pipeline {
                    Some(c) => c,
                    None => {
                        let is_explicit_terminal_request = target_stage_id.trim().is_empty()
                            || stage_id_matches(target_stage_id, "ready")
                            || stage_id_matches(target_stage_id, "done")
                            || stage_id_matches(target_stage_id, "terminal");

                        let terminal_opt = if is_explicit_terminal_request {
                            effective_board.as_ref().and_then(|b| {
                                b.stages.iter().find(|c| c.terminal == Some(true) || stage_id_matches(&c.id, "ready") || stage_id_matches(&c.id, "done")).cloned()
                                    .or_else(|| b.stages.last().cloned())
                            }).or_else(|| {
                                pipeline.stages.iter().find(|c| {
                                    c.terminal == Some(true)
                                        || stage_id_matches(&c.id, "ready")
                                        || stage_id_matches(&c.id, "done")
                                        || stage_id_matches(&c.id, "terminal")
                                }).cloned()
                            })
                        } else {
                            None
                        };

                        if let Some(term) = terminal_opt {
                            term
                        } else {
                            let mut available_stages: Vec<String> = Vec::new();
                            if let Some(ref b) = effective_board {
                                for c in &b.stages {
                                    if !available_stages.contains(&c.id) {
                                        available_stages.push(c.id.clone());
                                    }
                                }
                            }
                            for c in &pipeline.stages {
                                if !available_stages.contains(&c.id) {
                                    available_stages.push(c.id.clone());
                                }
                            }
                            return Err(format!(
                                "Target stage '{}' not found in active board or pipeline columns. Available: {:?}",
                                target_stage_id,
                                available_stages
                            ));
                        }
                    }
                }
            }
        }
    };
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

    // --- LOST UPDATE FIX ---
    // Re-load the fresh task list from SQLite AFTER all async hooks complete so that
    // any concurrent updates to other tasks (or to this task's todos/metadata) are preserved.
    let mut tasks = load_tasks_from_disk(root).await;
    if !tasks.iter().any(|t| task_id_matches(&t.id, &moving_task.id)) {
        if let Some(app) = app_handle {
            if let Some(state) = app.try_state::<AppState>() {
                if let Ok(guard) = state.tasks.lock() {
                    for mem_t in guard.iter() {
                        if !tasks.iter().any(|t| task_id_matches(&t.id, &mem_t.id)) {
                            tasks.push(mem_t.clone());
                        }
                    }
                }
            }
        }
        if !tasks.iter().any(|t| task_id_matches(&t.id, &moving_task.id)) {
            tasks.push(moving_task.clone());
        }
    }

    let task_idx = tasks
        .iter()
        .position(|t| task_id_matches(&t.id, &moving_task.id))
        .ok_or_else(|| format!("Task {} not found after stage transition hooks", task_id))?;

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

    if let Err(e) = upsert_single_task(root, &mut tasks[task_idx]) {
        crate::logger::cvh_log(
            Some(root),
            "ERROR",
            "MOVE_STAGE",
            &format!("Failed to upsert moved task '{}' to SQLite: {}", task_id, e),
        );
        return Err(e);
    }
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
                let _ = upsert_single_task(root, t);
                mutated_ids.insert(t.id.clone());
            }
            if t.parent_id.as_deref() == Some(&cur_id) {
                t.status = Some("done".to_string());
                if t.completed_at.is_none() {
                    t.completed_at = Some(Utc::now().timestamp_millis());
                }
                t.updated_at = Utc::now().timestamp_millis();
                auto_complete_task_todos(root, t, app_handle);
                let _ = upsert_single_task(root, t);
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
                    let _ = upsert_single_task(root, &mut tasks[parent_idx]);
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
            if mutated_ids.contains(&t.id) {
                if t.status.as_deref() == Some("done") {
                    auto_complete_task_todos(root, t, app_handle);
                }
                let _ = upsert_single_task(root, t);
            }
        }
    }

    for ctask in &mut parent_created_tasks {
        let _ = upsert_single_task(root, ctask);
        mutated_ids.insert(ctask.id.clone());
    }

    // Update only the mutated/created tasks in AppState without overwriting concurrent updates to other tasks
    for t in &tasks {
        if mutated_ids.contains(&t.id) {
            sync_single_task_in_app_state(root, t, app_handle);
        }
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
        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks, "projectPath": &project_path_str }));
        let _ = app.emit(
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
        );
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
        if let Ok(map) = state.active_agents.lock() {
            let tid_trimmed = task_id.trim();
            map.keys().any(|k| {
                task_id_matches(k, tid_trimmed)
                    || k.to_ascii_lowercase().starts_with(&format!("{}.", tid_trimmed.to_ascii_lowercase()))
                    || k.to_ascii_lowercase().starts_with(&format!("{}:sub:", tid_trimmed.to_ascii_lowercase()))
                    || k.to_ascii_lowercase().starts_with(&format!("{}.sub-", tid_trimmed.to_ascii_lowercase()))
            })
        } else {
            false
        }
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

pub async fn reorder_task(
    task_id: String,
    target_stage_id: String,
    new_index: usize,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    let root = state.get_root();
    let mut tasks = load_tasks_from_disk(&root).await;

    let task_idx = tasks
        .iter()
        .position(|t| task_id_matches(&t.id, &task_id))
        .ok_or_else(|| format!("Task {} not found", task_id))?;

    let old_stage_id = tasks[task_idx].stage_id.clone();
    let is_stage_change = !stage_id_matches(&old_stage_id, &target_stage_id);

    if is_stage_change {
        let _ = internal_move_task_stage(
            &root,
            &task_id,
            &target_stage_id,
            None,
            None,
            None,
            Some(&app_handle),
            false,
            None,
        )
        .await?;
        tasks = load_tasks_from_disk(&root).await;
    }

    let mut stage_tuples: Vec<(String, f64, i64)> = tasks
        .iter()
        .filter(|t| stage_id_matches(&t.stage_id, &target_stage_id) && !task_id_matches(&t.id, &task_id))
        .map(|t| (t.id.clone(), t.position.unwrap_or(0.0), t.created_at))
        .collect();

    stage_tuples.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.2.cmp(&b.2))
    });

    let mut ordered_ids: Vec<String> = stage_tuples.into_iter().map(|(id, _, _)| id).collect();
    let insert_pos = new_index.min(ordered_ids.len());
    ordered_ids.insert(insert_pos, task_id.clone());

    let now = Utc::now().timestamp_millis();
    let mut updated_task = None;

    for (idx, tid) in ordered_ids.iter().enumerate() {
        let new_pos = (idx as f64) * 1000.0;
        if let Some(t) = tasks.iter_mut().find(|t| task_id_matches(&t.id, tid)) {
            t.position = Some(new_pos);
            t.updated_at = now;
            upsert_single_task(&root, t)?;
            sync_single_task_in_app_state(&root, t, Some(&app_handle));
            if task_id_matches(&t.id, &task_id) {
                updated_task = Some(t.clone());
            }
        }
    }

    let project_path_str = root.to_string_lossy().to_string();
    let _ = app_handle.emit("tasks:updated", serde_json::json!({ "tasks": &tasks, "projectPath": &project_path_str }));
    if let Some(ref ut) = updated_task {
        let _ = app_handle.emit("task:updated", serde_json::json!({ "taskId": ut.id, "task": ut, "projectPath": &project_path_str }));
    }

    updated_task.ok_or_else(|| format!("Failed to reorder task {}", task_id))
}

pub async fn internal_advance_task(
    root: &Path,
    task_id: &str,
    summary: &str,
    app_handle: Option<&AppHandle>,
) -> Result<String, String> {
    internal_advance_task_to_stage(root, task_id, None, summary, app_handle).await
}

pub async fn internal_advance_task_to_stage(
    root: &Path,
    task_id: &str,
    target_stage_override: Option<&str>,
    summary: &str,
    app_handle: Option<&AppHandle>,
) -> Result<String, String> {
    let root_resolved = resolve_actual_repo_root(root);
    let root = &root_resolved;
    let initial_tasks = load_tasks_from_disk(root).await;
    let mut current_task = initial_tasks
        .iter()
        .find(|t| task_id_matches(&t.id, task_id))
        .cloned()
        .ok_or_else(|| format!("Task {} not found", task_id))?;
    drop(initial_tasks);

    let boards_cfg = load_boards_config(root).await.ok();

    // Normal progression within same board or fallback boards
    let columns = {
        let from_board = boards_cfg.as_ref().and_then(|bc| {
            // 1. Try board referenced by current_task.board_id if it contains current stage
            if let Some(ref bid) = current_task.board_id {
                if let Some(b) = bc.boards.iter().find(|b| &b.id == bid) {
                    if b.stages.iter().any(|c| stage_id_matches(&c.id, &current_task.stage_id)) {
                        return Some(b.stages.clone());
                    }
                }
            }
            // 2. Try active board if it contains current stage
            if let Some(b) = bc.boards.iter().find(|b| b.id == bc.active_board_id) {
                if b.stages.iter().any(|c| stage_id_matches(&c.id, &current_task.stage_id)) {
                    return Some(b.stages.clone());
                }
            }
            // 3. Search any board in boards.json that contains current stage
            if let Some(b) = bc.boards.iter().find(|b| {
                b.stages.iter().any(|c| stage_id_matches(&c.id, &current_task.stage_id))
            }) {
                return Some(b.stages.clone());
            }
            // 4. Fallback to active board columns or first board
            bc.boards
                .iter()
                .find(|b| b.id == bc.active_board_id)
                .or_else(|| bc.boards.first())
                .map(|b| b.stages.clone())
        });
        match from_board {
            Some(cols) => {
                if cols.iter().any(|c| stage_id_matches(&c.id, &current_task.stage_id)) {
                    cols
                } else {
                    let pipeline = load_pipeline_config(root)
                        .await
                        .map_err(|e| e.to_string())?;
                    pipeline.stages
                }
            }
            None => {
                let pipeline = load_pipeline_config(root)
                    .await
                    .map_err(|e| e.to_string())?;
                pipeline.stages
            }
        }
    };

    let target_col = if let Some(target_stage_id) = target_stage_override {
        columns
            .iter()
            .find(|c| stage_id_matches(&c.id, target_stage_id))
            .cloned()
            .ok_or_else(|| {
                let available_stages: Vec<&str> = columns.iter().map(|c| c.id.as_str()).collect();
                format!(
                    "Target stage '{}' not found in columns. Available stages: {:?}",
                    target_stage_id,
                    available_stages
                )
            })?
    } else {
        let curr_idx = columns
            .iter()
            .position(|c| stage_id_matches(&c.id, &current_task.stage_id))
            .ok_or_else(|| {
                format!(
                    "Current column '{}' not found in pipeline",
                    current_task.stage_id
                )
            })?;

        if curr_idx + 1 >= columns.len() {
            return Err(format!(
                "Задача {} уже находится на последнем этапе пайплайна ({})",
                task_id, columns[curr_idx].title
            ));
        }

        columns[curr_idx + 1].clone()
    };

    // Universal Relay Progression (§ 2.2 & § 2.7):
    let is_terminal = target_col.terminal == Some(true)
        || stage_id_matches(&target_col.id, "ready")
        || stage_id_matches(&target_col.id, "done");

    let source_col = columns
        .iter()
        .find(|c| stage_id_matches(&c.id, &current_task.stage_id))
        .cloned();

    let is_source_intake = {
        let source_idx = columns
            .iter()
            .position(|c| stage_id_matches(&c.id, &current_task.stage_id));
        current_task.stage_id.is_empty()
            || source_idx == Some(0)
            || source_col
                .as_ref()
                .map(|c| c.is_intake == Some(true))
                .unwrap_or(false)
    };

    let is_review_stage = source_col
        .as_ref()
        .map(|c| c.persona.as_deref() == Some("reviewer") || stage_id_matches(&c.id, "review"))
        .unwrap_or(false);

    let effective_verdict = current_task.verdict.clone().or_else(|| {
        if is_terminal || is_review_stage {
            Some("APPROVED".to_string())
        } else {
            None
        }
    });

    if effective_verdict.is_some() && current_task.verdict.is_none() {
        current_task.verdict = effective_verdict.clone();
    }

    let pipeline = TaskTransitionPipeline::new(
        root,
        source_col.as_ref(),
        &target_col,
        is_source_intake,
        effective_verdict.clone(),
        false, // emit_task_moved
        app_handle,
    );
    pipeline.execute(&mut current_task).await?;
    let _ = upsert_single_task(root, &mut current_task);
    auto_complete_task_todos(root, &mut current_task, app_handle);

    // Legacy on_enter_script (col.on_enter_script) is a distinct field from
    // hooks.on_enter executed by the pipeline above — not a duplicate.
    // It is best-effort and must not block subtask creation.
    trigger_column_on_enter_script(
        &target_col,
        &mut current_task,
        &target_col.id,
        effective_verdict.as_deref(),
        root,
        app_handle,
    )
    .await;

    // Reload fresh tasks from SQLite after all async hooks complete
    let mut tasks = load_tasks_from_disk(root).await;

    if let Some(ref pid) = current_task.parent_id {
        // CASE A: The task calling advance_task is a child subtask (e.g. CVH-009.1)
        let subtask_id = current_task.id.clone();
        let parent_id = pid.clone();

        let parent_task = tasks
            .iter()
            .find(|t| task_id_matches(&t.id, &parent_id))
            .cloned()
            .ok_or_else(|| format!("Parent task {} not found", parent_id))?;

        if is_terminal {
            // 1. Mark this subtask as done
            if let Some(sub) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &subtask_id)) {
                sub.status = Some("done".to_string());
                if sub.verdict.is_none() && effective_verdict.is_some() {
                    sub.verdict = effective_verdict.clone();
                }
                if sub.completed_at.is_none() {
                    sub.completed_at = Some(Utc::now().timestamp_millis());
                }
                sub.updated_at = Utc::now().timestamp_millis();
                auto_complete_task_todos(root, sub, app_handle);
                let _ = upsert_single_task(root, sub);
                sync_single_task_in_app_state(root, sub, app_handle);
            }
            if let Some(app) = app_handle {
                let project_path_str = root.to_string_lossy().to_string();
                if let Some(sub) = tasks.iter().find(|t| task_id_matches(&t.id, &subtask_id)) {
                    let _ = app.emit("task:updated", serde_json::json!({ "task": sub, "projectPath": &project_path_str }));
                }
            }

            let sub_summary_content = format_stage_summary_activity(
                &current_task.stage_id,
                Some(&current_task.title),
                summary,
            );
            let _ = append_task_dialog_activity(root, &subtask_id, "summary", &sub_summary_content, app_handle).await;

            let updated_parent = internal_move_task_stage(
                root,
                &parent_id,
                &target_col.id,
                Some("APPROVED".to_string()),
                Some("done".to_string()),
                Some(summary.to_string()),
                app_handle,
                true,
                None,
            )
            .await?;

            let mut final_tasks = load_tasks_from_disk(root).await;
            for t in final_tasks.iter_mut() {
                if task_id_matches(&t.id, &parent_id) {
                    t.waiting_for_subtask_id = None;
                    t.status = Some("done".to_string());
                    if t.completed_at.is_none() {
                        t.completed_at = Some(Utc::now().timestamp_millis());
                    }
                    t.stage_id = target_col.id.clone();
                    if t.stage_entered_at.is_none() {
                        t.stage_entered_at = Some(Utc::now().timestamp_millis());
                    }
                    t.updated_at = Utc::now().timestamp_millis();
                    auto_complete_task_todos(root, t, app_handle);
                    let _ = upsert_single_task(root, t);
                    sync_single_task_in_app_state(root, t, app_handle);
                }
                if t.parent_id.as_deref() == Some(&parent_id) {
                    t.status = Some("done".to_string());
                    if t.completed_at.is_none() {
                        t.completed_at = Some(Utc::now().timestamp_millis());
                    }
                    t.updated_at = Utc::now().timestamp_millis();
                    auto_complete_task_todos(root, t, app_handle);
                    let _ = upsert_single_task(root, t);
                    sync_single_task_in_app_state(root, t, app_handle);
                }
            }
            if let Some(app) = app_handle {
                let project_path_str = root.to_string_lossy().to_string();
                if let Some(parent) = final_tasks.iter().find(|t| task_id_matches(&t.id, &parent_id)) {
                    let _ = app.emit("task:updated", serde_json::json!({ "task": parent, "projectPath": &project_path_str }));
                }
                let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &final_tasks, "projectPath": &project_path_str }));
                let _ = app.emit("task:advanced", serde_json::json!({
                    "taskId": &parent_id,
                    "sourceTaskId": &subtask_id,
                    "targetTaskId": &parent_id,
                    "sourceStageId": source_col.as_ref().map(|c| c.id.as_str()).unwrap_or(current_task.stage_id.as_str()),
                    "sourceStageTitle": source_col.as_ref().map(|c| c.title.as_str()).unwrap_or(current_task.stage_id.as_str()),
                    "targetStageId": &target_col.id,
                    "targetStageTitle": &target_col.title,
                    "isTerminal": true,
                    "summary": summary,
                    "projectPath": &project_path_str,
                }));
            }

            return Ok(format!(
                "УСПЕШНО: Задача {} завершена в терминальной колонке '{}' ({}), статус: {}",
                parent_id,
                target_col.id,
                target_col.title,
                updated_parent.status.as_deref().unwrap_or("done")
            ));
        } else {
            let existing_target_sub = tasks
                .iter()
                .filter(|t| {
                    t.parent_id.as_deref().is_some_and(|p| task_id_matches(p, &parent_id))
                        && stage_id_matches(&t.stage_id, &target_col.id)
                        && !task_id_matches(&t.id, &subtask_id)
                        && (t.verdict.as_deref() == Some("CHANGES_REQUESTED") || t.status.as_deref() != Some("done"))
                })
                .max_by_key(|t| (t.subtask_index.unwrap_or(0), t.created_at))
                .cloned();

            if let Some(existing_sub) = existing_target_sub {
                let target_has_autorun = target_col.persona.is_some() || target_col.auto_run == Some(true);
                let next_status = if target_has_autorun { "running" } else { "idle" };

                // 1. Mark current subtask as done
                if let Some(sub) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &subtask_id)) {
                    sub.status = Some("done".to_string());
                    if sub.completed_at.is_none() {
                        sub.completed_at = Some(Utc::now().timestamp_millis());
                    }
                    sub.updated_at = Utc::now().timestamp_millis();
                    auto_complete_task_todos(root, sub, app_handle);
                    let _ = upsert_single_task(root, sub);
                    sync_single_task_in_app_state(root, sub, app_handle);
                }

                // 2. Update parent task waiting_for_subtask_id
                if let Some(parent) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &parent_id)) {
                    parent.waiting_for_subtask_id = Some(existing_sub.id.clone());
                    parent.status = Some("done".to_string());
                    freeze_stage_duration(parent, Utc::now().timestamp_millis());
                    parent.updated_at = Utc::now().timestamp_millis();
                    let _ = upsert_single_task(root, parent);
                    sync_single_task_in_app_state(root, parent, app_handle);
                }

                // 3. Reactivate existing target subtask
                if let Some(target_sub) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &existing_sub.id)) {
                    target_sub.status = Some(next_status.to_string());
                    target_sub.verdict = None;
                    target_sub.completed_at = None;
                    target_sub.handover_summary = Some(summary.to_string());
                    target_sub.updated_at = Utc::now().timestamp_millis();
                    let _ = upsert_single_task(root, target_sub);
                    sync_single_task_in_app_state(root, target_sub, app_handle);
                }

                let sub_summary_content = format_stage_summary_activity(
                    &current_task.stage_id,
                    Some(&current_task.title),
                    summary,
                );
                let _ = append_task_dialog_activity(root, &subtask_id, "summary", &sub_summary_content, app_handle).await;

                let handover_prompt = format_handover_steer_prompt(summary);
                if !target_has_autorun {
                    let _ = append_task_dialog_activity(root, &existing_sub.id, "steer", &handover_prompt, app_handle).await;
                }

                if let Some(app) = app_handle {
                    let project_path_str = root.to_string_lossy().to_string();
                    if let Some(sub) = tasks.iter().find(|t| task_id_matches(&t.id, &subtask_id)) {
                        let _ = app.emit("task:updated", serde_json::json!({ "task": sub, "projectPath": &project_path_str }));
                    }
                    if let Some(parent) = tasks.iter().find(|t| task_id_matches(&t.id, &parent_id)) {
                        let _ = app.emit("task:updated", serde_json::json!({ "task": parent, "projectPath": &project_path_str }));
                    }
                    if let Some(target_sub) = tasks.iter().find(|t| task_id_matches(&t.id, &existing_sub.id)) {
                        let _ = app.emit("task:updated", serde_json::json!({ "task": target_sub, "projectPath": &project_path_str }));
                    }
                    let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks, "projectPath": &project_path_str }));
                    let _ = app.emit("task:advanced", serde_json::json!({
                        "taskId": &parent_id,
                        "sourceTaskId": &subtask_id,
                        "targetTaskId": &existing_sub.id,
                        "sourceStageId": source_col.as_ref().map(|c| c.id.as_str()).unwrap_or(current_task.stage_id.as_str()),
                        "sourceStageTitle": source_col.as_ref().map(|c| c.title.as_str()).unwrap_or(current_task.stage_id.as_str()),
                        "targetStageId": &target_col.id,
                        "targetStageTitle": &target_col.title,
                        "isTerminal": false,
                        "summary": summary,
                        "projectPath": &project_path_str,
                    }));

                    if target_has_autorun {
                        let _ = app.emit("agent:autorun", serde_json::json!({
                            "taskId": existing_sub.id,
                            "prompt": handover_prompt,
                            "projectPath": &project_path_str,
                        }));
                    }
                }

                return Ok(format!(
                    "УСПЕШНО: Возобновлена существующая подзадача {} ({}) для задачи {}",
                    existing_sub.id, target_col.title, parent_id
                ));
            } else {
                // 1. Mark this subtask as done
                if let Some(sub) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &subtask_id)) {
                    sub.status = Some("done".to_string());
                    if sub.completed_at.is_none() {
                        sub.completed_at = Some(Utc::now().timestamp_millis());
                    }
                    sub.updated_at = Utc::now().timestamp_millis();
                    auto_complete_task_todos(root, sub, app_handle);
                    let _ = upsert_single_task(root, sub);
                    sync_single_task_in_app_state(root, sub, app_handle);
                }
                if let Some(app) = app_handle {
                    let project_path_str = root.to_string_lossy().to_string();
                    if let Some(sub) = tasks.iter().find(|t| task_id_matches(&t.id, &subtask_id)) {
                        let _ = app.emit("task:updated", serde_json::json!({ "task": sub, "projectPath": &project_path_str }));
                    }
                }

                let sub_summary_content = format_stage_summary_activity(
                    &current_task.stage_id,
                    Some(&current_task.title),
                    summary,
                );
                let _ = append_task_dialog_activity(root, &subtask_id, "summary", &sub_summary_content, app_handle).await;

                let next_subtask = internal_create_subtask_with_hook_states(
                    root,
                    &parent_id,
                    Some(&target_col.id),
                    Some(&format!("{} ({})", parent_task.title, target_col.title)),
                    Some(summary),
                    app_handle,
                    current_task.hook_states.clone(),
                )
                .await?;

                let next_subtask_id = next_subtask.id.clone();

                if let Some(app) = app_handle {
                    let _ = app.emit("task:advanced", serde_json::json!({
                        "taskId": &parent_id,
                        "sourceTaskId": &subtask_id,
                        "targetTaskId": &next_subtask_id,
                        "sourceStageId": source_col.as_ref().map(|c| c.id.as_str()).unwrap_or(current_task.stage_id.as_str()),
                        "sourceStageTitle": source_col.as_ref().map(|c| c.title.as_str()).unwrap_or(current_task.stage_id.as_str()),
                        "targetStageId": &target_col.id,
                        "targetStageTitle": &target_col.title,
                        "isTerminal": false,
                        "summary": summary,
                    }));
                }

                return Ok(format!(
                    "УСПЕШНО: Создана подзадача {} ({}) для задачи {}",
                    next_subtask_id, target_col.title, parent_id
                ));
            }
        }
    }

    // CASE B: The task calling advance_task is a root parent task (e.g. CVH-009)
    if !is_terminal {
        // Strict guard against duplicate subtasks when subtask is already running:
        if let Some(ref existing_sub_id) = current_task.waiting_for_subtask_id {
            if let Some(existing_sub) = tasks.iter().find(|t| task_id_matches(&t.id, existing_sub_id)) {
                if existing_sub.status.as_deref() != Some("done")
                    && existing_sub.verdict.as_deref() != Some("CHANGES_REQUESTED")
                {
                    crate::logger::cvh_log(
                        Some(root),
                        "WARN",
                        "ADVANCE_TASK",
                        &format!(
                            "advance_task: Task '{}' already has active waiting_for_subtask_id '{}' (status: {:?})",
                            task_id, existing_sub_id, existing_sub.status
                        ),
                    );
                    let parent_summary_content = format_stage_summary_activity(
                        &current_task.stage_id,
                        Some(&current_task.title),
                        summary,
                    );
                    let _ = append_task_dialog_activity(root, task_id, "summary", &parent_summary_content, app_handle).await;

                    return Ok(format!(
                        "Подзадача {} уже активна для задачи {}",
                        existing_sub_id, task_id
                    ));
                }
            }
        }
        if let Some(existing_sub) = tasks.iter().find(|t| {
            t.parent_id.as_deref().is_some_and(|p| task_id_matches(p, task_id))
                && stage_id_matches(&t.stage_id, &target_col.id)
                && t.status.as_deref() != Some("done")
                && t.verdict.as_deref() != Some("CHANGES_REQUESTED")
        }) {
            crate::logger::cvh_log(
                Some(root),
                "WARN",
                "ADVANCE_TASK",
                &format!(
                    "advance_task: Task '{}' already has active subtask '{}' in column '{}' (status: {:?})",
                    task_id, existing_sub.id, target_col.title, existing_sub.status
                ),
            );
            return Ok(format!(
                "Подзадача {} уже выполняется в колонке {}",
                existing_sub.id, target_col.title
            ));
        }

        let existing_target_sub = tasks
            .iter()
            .filter(|t| {
                t.parent_id.as_deref().is_some_and(|p| task_id_matches(p, task_id))
                    && stage_id_matches(&t.stage_id, &target_col.id)
                    && (t.verdict.as_deref() == Some("CHANGES_REQUESTED") || t.status.as_deref() != Some("done"))
            })
            .max_by_key(|t| (t.subtask_index.unwrap_or(0), t.created_at))
            .cloned();

        if let Some(existing_sub) = existing_target_sub {
            let target_has_autorun = target_col.persona.is_some() || target_col.auto_run == Some(true);
            let next_status = if target_has_autorun { "running" } else { "idle" };

            if let Some(parent) = tasks.iter_mut().find(|t| task_id_matches(&t.id, task_id)) {
                parent.waiting_for_subtask_id = Some(existing_sub.id.clone());
                parent.status = Some("done".to_string());
                freeze_stage_duration(parent, Utc::now().timestamp_millis());
                parent.updated_at = Utc::now().timestamp_millis();
                auto_complete_task_todos(root, parent, app_handle);
                let _ = upsert_single_task(root, parent);
                sync_single_task_in_app_state(root, parent, app_handle);
            }

            if let Some(target_sub) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &existing_sub.id)) {
                target_sub.status = Some(next_status.to_string());
                target_sub.verdict = None;
                target_sub.handover_summary = Some(summary.to_string());
                target_sub.updated_at = Utc::now().timestamp_millis();
                let _ = upsert_single_task(root, target_sub);
                sync_single_task_in_app_state(root, target_sub, app_handle);
            }

            let parent_summary_content = format_stage_summary_activity(
                &current_task.stage_id,
                Some(&current_task.title),
                summary,
            );
            let _ = append_task_dialog_activity(root, task_id, "summary", &parent_summary_content, app_handle).await;

            let handover_prompt = format_handover_steer_prompt(summary);
            if !target_has_autorun {
                let _ = append_task_dialog_activity(root, &existing_sub.id, "steer", &handover_prompt, app_handle).await;
            }

            if let Some(app) = app_handle {
                let project_path_str = root.to_string_lossy().to_string();
                if let Some(parent) = tasks.iter().find(|t| task_id_matches(&t.id, task_id)) {
                    let _ = app.emit("task:updated", serde_json::json!({ "task": parent, "projectPath": &project_path_str }));
                }
                if let Some(target_sub) = tasks.iter().find(|t| task_id_matches(&t.id, &existing_sub.id)) {
                    let _ = app.emit("task:updated", serde_json::json!({ "task": target_sub, "projectPath": &project_path_str }));
                }
                let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks, "projectPath": &project_path_str }));
                let _ = app.emit("task:advanced", serde_json::json!({
                    "taskId": task_id,
                    "sourceTaskId": task_id,
                    "targetTaskId": &existing_sub.id,
                    "sourceStageId": source_col.as_ref().map(|c| c.id.as_str()).unwrap_or(current_task.stage_id.as_str()),
                    "sourceStageTitle": source_col.as_ref().map(|c| c.title.as_str()).unwrap_or(current_task.stage_id.as_str()),
                    "targetStageId": &target_col.id,
                    "targetStageTitle": &target_col.title,
                    "isTerminal": false,
                    "summary": summary,
                    "projectPath": &project_path_str,
                }));

                if target_has_autorun {
                    let _ = app.emit("agent:autorun", serde_json::json!({
                        "taskId": existing_sub.id,
                        "prompt": handover_prompt,
                    }));
                }
            }

            return Ok(format!(
                "УСПЕШНО: Возобновлена существующая подзадача {} ({}) для задачи {}",
                existing_sub.id, target_col.title, task_id
            ));
        }

        auto_complete_task_todos(root, &mut current_task, app_handle);
        let subtask = internal_create_subtask_with_hook_states(
            root,
            task_id,
            Some(&target_col.id),
            Some(&format!("{} ({})", current_task.title, target_col.title)),
            Some(summary),
            app_handle,
            current_task.hook_states.clone(),
        )
        .await?;

        let subtask_id = subtask.id.clone();

        let parent_summary_content = format_stage_summary_activity(
            &current_task.stage_id,
            Some(&current_task.title),
            summary,
        );
        let _ = append_task_dialog_activity(
            root,
            task_id,
            "summary",
            &parent_summary_content,
            app_handle,
        )
        .await;

        if let Some(app) = app_handle {
            let _ = app.emit("task:advanced", serde_json::json!({
                "taskId": task_id,
                "sourceTaskId": task_id,
                "targetTaskId": &subtask_id,
                "sourceStageId": source_col.as_ref().map(|c| c.id.as_str()).unwrap_or(current_task.stage_id.as_str()),
                "sourceStageTitle": source_col.as_ref().map(|c| c.title.as_str()).unwrap_or(current_task.stage_id.as_str()),
                "targetStageId": &target_col.id,
                "targetStageTitle": &target_col.title,
                "isTerminal": false,
                "summary": summary,
            }));
        }

        return Ok(format!(
            "УСПЕШНО: Создана подзадача {} ({}) для задачи {}",
            subtask_id, target_col.title, task_id
        ));
    }

    // Terminal column advance for parent task:
    let updated_task = internal_move_task_stage(
        root,
        task_id,
        &target_col.id,
        Some("APPROVED".to_string()),
        Some("done".to_string()),
        Some(summary.to_string()),
        app_handle,
        true, // is_autonomous_advance
        None,
    )
    .await?;

    let summary_content = format_stage_summary_activity(&target_col.title, None, summary);
    let _ = append_task_dialog_activity(
        root,
        task_id,
        "summary",
        &summary_content,
        app_handle,
    )
    .await;

    if let Some(app) = app_handle {
        let _ = app.emit("task:advanced", serde_json::json!({
            "taskId": task_id,
            "sourceTaskId": task_id,
            "targetTaskId": task_id,
            "sourceStageId": source_col.as_ref().map(|c| c.id.as_str()).unwrap_or(current_task.stage_id.as_str()),
            "sourceStageTitle": source_col.as_ref().map(|c| c.title.as_str()).unwrap_or(current_task.stage_id.as_str()),
            "targetStageId": &target_col.id,
            "targetStageTitle": &target_col.title,
            "isTerminal": true,
            "summary": summary,
        }));
    }

    Ok(format!(
        "УСПЕШНО: Задача {} завершена в терминальной колонке '{}' ({}), статус: {}",
        task_id,
        target_col.id,
        target_col.title,
        updated_task.status.as_deref().unwrap_or("done")
    ))
}

pub async fn advance_task(
    task_id: String,
    summary: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    let root = state.get_root();
    internal_advance_task(&root, &task_id, &summary, Some(&app)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_task(id: &str, stage_id: &str, status: &str) -> Task {
        Task {
            id: id.to_string(),
            title: format!("Title {}", id),
            description: format!("Desc {}", id),
            stage_id: stage_id.to_string(),
            board_id: Some("default".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some(status.to_string()),
            model: None,
            provider_id: None,
            reasoning_effort: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            handover_summary: None,
            cover_image: None,
            attachments: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: Some(true),
            created_at: 1000,
            updated_at: 1000,
            stage_entered_at: Some(1000),
            completed_at: None,
            position: Some(1000.0),
            hook_states: None,
            project_path: None,
            stage_durations: None,
            total_elapsed: None,
        }
    }

    #[tokio::test]
    async fn move_task_stage_preserves_concurrent_task_updates_during_on_enter_hook() {
        let root_buf = std::env::temp_dir().join(format!("cvh_transitions_lost_update_{}", uuid::Uuid::new_v4()));
        let root = root_buf.as_path();
        std::fs::create_dir_all(root.join(".cvh")).expect("create .cvh");

        // Create an on_enter script that concurrently updates CVH-002 in SQLite while CVH-001's hook is running
        #[cfg(windows)]
        let script_name = "concurrent_update.ps1";
        #[cfg(not(windows))]
        let script_name = "concurrent_update.sh";

        let script_path = root.join(".cvh").join(script_name);
        #[cfg(windows)]
        std::fs::write(
            &script_path,
            "Start-Sleep -Milliseconds 200\r\nWrite-Output 'hook-ran'\r\n",
        )
        .expect("write hook");
        #[cfg(not(windows))]
        std::fs::write(&script_path, "#!/bin/sh\nsleep 0.2\necho 'hook-ran'\n").expect("write hook");

        let pipeline_json = serde_json::json!({
            "stages": [
                { "id": "backlog", "title": "Backlog", "isIntake": true },
                { "id": "dev", "title": "Dev", "onEnterScript": format!(".cvh/{}", script_name) },
                { "id": "ready", "title": "Ready", "terminal": true }
            ]
        });
        std::fs::write(
            root.join(".cvh").join("pipeline.json"),
            serde_json::to_string_pretty(&pipeline_json).unwrap(),
        )
        .expect("write pipeline.json");

        let mut task1 = sample_task("CVH-001", "backlog", "idle");
        let mut task2 = sample_task("CVH-002", "dev", "running");
        upsert_single_task(root, &mut task1).expect("upsert task1");
        upsert_single_task(root, &mut task2).expect("upsert task2");

        let root_for_concurrent = root.to_path_buf();
        let concurrent_writer = async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            task2.status = Some("done".to_string());
            task2.stage_id = "ready".to_string();
            task2.handover_summary = Some("Concurrent completion summary".to_string());
            upsert_single_task(&root_for_concurrent, &mut task2).expect("concurrent upsert task2");
        };

        let (move_res, _) = tokio::join!(
            internal_move_task_stage(
                root,
                "CVH-001",
                "dev",
                None,
                None,
                None,
                None,
                false,
                None,
            ),
            concurrent_writer
        );

        let moved = move_res.expect("move CVH-001 to dev");

        assert_eq!(moved.stage_id, "dev");

        let final_tasks = load_tasks_from_disk(root).await;
        let t2 = final_tasks
            .iter()
            .find(|t| t.id == "CVH-002")
            .expect("CVH-002 present");
        assert_eq!(t2.stage_id, "ready");
        assert_eq!(t2.status.as_deref(), Some("done"));
        assert_eq!(
            t2.handover_summary.as_deref(),
            Some("Concurrent completion summary")
        );
    }
}

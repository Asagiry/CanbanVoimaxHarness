//! Task CRUD and Lifecycle Management Subsystem
//! Extracted from commands/tasks.rs per ub-mode Clean Code principles.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::agent::Task;
use crate::commands::dialogs::append_task_dialog_activity;
use crate::commands::git::apply_worktree_from_hook_stdout;
use crate::commands::state::AppState;
use crate::config::{load_boards_config, load_pipeline_config, resolve_actual_repo_root};
use crate::hooks::{execute_hook, HookContext};
use super::healing::{stage_id_matches, task_id_matches};
use super::storage::{
    get_next_subtask_index, load_tasks_from_disk,
    load_task_sequence, max_task_num_on_disk, resolve_or_allocate_task_id, save_tasks_to_disk,
    save_task_sequence,
};
use super::transitions::trigger_column_on_enter_script;

pub async fn list_tasks(state: State<'_, AppState>) -> Result<Vec<Task>, String> {
    let root = resolve_actual_repo_root(&state.get_root());
    let tasks = load_tasks_from_disk(&root).await;
    let ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    crate::logger::cvh_log_with_details(
        Some(&root),
        "INFO",
        "LIST_TASKS",
        &format!("list_tasks command: returning {} tasks for root '{}'", tasks.len(), root.display()),
        Some(&format!("Task IDs: [{}]", ids.join(", "))),
    );
    if let Ok(mut guard) = state.tasks.lock() {
        *guard = tasks.clone();
    }
    Ok(tasks)
}

pub async fn save_tasks(tasks: Vec<Task>, state: State<'_, AppState>) -> Result<(), String> {
    let root = resolve_actual_repo_root(&state.get_root());
    internal_save_tasks(&root, tasks, Some(&state.tasks)).await
}

pub async fn internal_save_tasks(
    root: &Path,
    tasks: Vec<Task>,
    state_tasks: Option<&Arc<Mutex<Vec<Task>>>>,
) -> Result<(), String> {
    let incoming_ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    crate::logger::cvh_log_with_details(
        Some(root),
        "INFO",
        "SAVE_TASKS",
        &format!("save_tasks command: received {} tasks from client for root '{}'", tasks.len(), root.display()),
        Some(&format!("Incoming IDs: [{}]", incoming_ids.join(", "))),
    );
    let actual_root = resolve_actual_repo_root(root);
    let foreign_incoming = super::storage::find_foreign_task_ids(&tasks, &actual_root);
    let mut valid_incoming = tasks;
    if !foreign_incoming.is_empty() {
        crate::logger::cvh_log(
            Some(&actual_root),
            "WARN",
            "SAVE_TASKS",
            &format!(
                "Dropping {} foreign tasks from incoming save_tasks payload for root '{}': [{}]",
                foreign_incoming.len(),
                actual_root.display(),
                foreign_incoming.iter().cloned().collect::<Vec<_>>().join(", ")
            ),
        );
        valid_incoming.retain(|t| !foreign_incoming.contains(&t.id));
    }

    let disk_tasks = load_tasks_from_disk(root).await;
    let boards_cfg = crate::config::load_boards_config(root).await.ok();

    // Defensive reconciliation: prevent stale frontend saves from destroying newly created
    // subtasks or wiping out active parent 'waiting_subtask' invariants (§ 2.2 & § 2.22).
    let mut merged_tasks = valid_incoming;

    // 1. Preserve existing subtasks or recent tasks from disk if missing from incoming tasks:
    // (delete_task has its own dedicated IPC command and is the ONLY authorized deletion path).
    for dt in &disk_tasks {
        if !merged_tasks.iter().any(|t| t.id == dt.id) {
            merged_tasks.push(dt.clone());
        }
    }

    // 2. Preserve waiting_for_subtask_id and waiting_subtask status if the subtask is still active
    let active_sub_ids: HashSet<String> = merged_tasks
        .iter()
        .filter(|sub| sub.status.as_deref() != Some("done"))
        .map(|sub| sub.id.clone())
        .collect();

    for t in &mut merged_tasks {
        if let Some(dt) = disk_tasks.iter().find(|d| d.id == t.id) {
            let is_term = stage_id_matches(&t.stage_id, "ready")
                || stage_id_matches(&t.stage_id, "done")
                || t.status.as_deref() == Some("done")
                || boards_cfg.as_ref().is_some_and(|cfg| {
                    cfg.boards.iter().any(|b| {
                        b.columns.iter().any(|c| stage_id_matches(&c.id, &t.stage_id) && c.terminal == Some(true))
                    })
                });

            if is_term {
                t.waiting_for_subtask_id = None;
                if t.status.as_deref() == Some("waiting_subtask") {
                    t.status = Some("done".to_string());
                }
            } else if dt.status.as_deref() == Some("waiting_subtask") && t.status.as_deref() != Some("waiting_subtask") {
                if let Some(ref sub_id) = dt.waiting_for_subtask_id {
                    if active_sub_ids.contains(sub_id) {
                        t.status = Some("waiting_subtask".to_string());
                        t.waiting_for_subtask_id = Some(sub_id.clone());
                    } else {
                        t.waiting_for_subtask_id = None;
                    }
                }
            } else if dt.waiting_for_subtask_id.is_some() && t.waiting_for_subtask_id.is_none() {
                if let Some(ref sub_id) = dt.waiting_for_subtask_id {
                    if active_sub_ids.contains(sub_id) {
                        t.waiting_for_subtask_id = Some(sub_id.clone());
                    }
                }
            }
            if t.attachments.is_none() && dt.attachments.is_some() {
                t.attachments = dt.attachments.clone();
            }
            if t.cover_image.is_none() && dt.cover_image.is_some() {
                t.cover_image = dt.cover_image.clone();
            }
            if t.handover_summary.is_none() && dt.handover_summary.is_some() {
                t.handover_summary = dt.handover_summary.clone();
            }
            if t.hook_states.is_none() && dt.hook_states.is_some() {
                t.hook_states = dt.hook_states.clone();
            }
            if t.worktree_path.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true) && dt.worktree_path.is_some() {
                t.worktree_path = dt.worktree_path.clone();
            }
            if t.branch.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true) && dt.branch.is_some() {
                t.branch = dt.branch.clone();
            }
            if t.parent_id.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true) && dt.parent_id.is_some() {
                t.parent_id = dt.parent_id.clone();
            }
            if t.stage_id == dt.stage_id {
                if dt.stage_entered_at.is_some() {
                    t.stage_entered_at = dt.stage_entered_at;
                }
            } else if t.stage_entered_at.is_none() {
                t.stage_entered_at = dt.stage_entered_at;
            }
            if t.stage_entered_at.is_none() {
                t.stage_entered_at = Some(t.created_at);
            }
        }
    }

    if let Some(st) = state_tasks {
        if let Ok(mut guard) = st.lock() {
            *guard = merged_tasks.clone();
        }
    }
    let res = save_tasks_to_disk(root, &merged_tasks).await;
    match &res {
        Ok(_) => {
            let merged_ids: Vec<&str> = merged_tasks.iter().map(|t| t.id.as_str()).collect();
            crate::logger::cvh_log_with_details(
                Some(root),
                "INFO",
                "SAVE_TASKS",
                &format!("save_tasks command: successfully persisted {} merged tasks to disk", merged_tasks.len()),
                Some(&format!("Merged IDs: [{}]", merged_ids.join(", "))),
            );
        }
        Err(e) => {
            crate::logger::cvh_log(
                Some(root),
                "ERROR",
                "SAVE_TASKS",
                &format!("save_tasks command: FAILED to persist {} tasks: {}", merged_tasks.len(), e),
            );
        }
    }
    res
}



#[allow(clippy::too_many_arguments)]
pub async fn create_task(
    title: String,
    description: String,
    stage_id: String,
    board_id: Option<String>,
    skip_worktree: Option<bool>,
    skip_backlog_hook: Option<bool>,
    model: Option<String>,
    task_id: Option<String>,
    attachments: Option<Vec<String>>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    let effective_skip = skip_backlog_hook.or(skip_worktree);
    let root = state.get_root();
    crate::logger::cvh_log_with_details(
        Some(&root),
        "INFO",
        "CREATE_TASK",
        &format!(
            "create_task IPC called: title='{}', stage_id='{}', board_id={:?}, task_id_param={:?}, skip_backlog_hook={:?}, model={:?}",
            title, stage_id, board_id, task_id, effective_skip, model
        ),
        Some(&format!("root='{}'", root.display())),
    );
    internal_create_task(
        &root,
        title,
        description,
        stage_id,
        board_id,
        effective_skip,
        model,
        task_id,
        attachments,
        Some(&app_handle),
        Some(state.inner()),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub fn record_task_hook_result(
    task: &mut Task,
    stage_id: &str,
    hook_type: &str,
    hook_name: &str,
    started_at: i64,
    duration_ms: u64,
    success: bool,
    exit_code: Option<i32>,
    stdout: &str,
    stderr: &str,
    error: Option<&str>,
) {
    let mut map: serde_json::Map<String, serde_json::Value> = task
        .hook_states
        .as_ref()
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();

    map.insert(
        hook_type.to_string(),
        serde_json::json!({
            "hookName": hook_name,
            "stageId": stage_id,
            "hookType": hook_type,
            "status": if success { "success" } else { "failed" },
            "startedAt": started_at,
            "durationMs": duration_ms,
            "exitCode": exit_code,
            "stdout": stdout,
            "stderr": stderr,
            "error": error,
        }),
    );

    task.hook_states = Some(serde_json::Value::Object(map));
}

#[allow(clippy::too_many_arguments)]
pub async fn internal_create_task(
    root: &Path,
    title: String,
    description: String,
    stage_id: String,
    board_id: Option<String>,
    skip_worktree: Option<bool>,
    model: Option<String>,
    task_id: Option<String>,
    attachments: Option<Vec<String>>,
    app_handle: Option<&AppHandle>,
    state: Option<&AppState>,
) -> Result<Task, String> {
    let actual_root = resolve_actual_repo_root(root);
    let root = &actual_root;
    crate::logger::cvh_log_with_details(
        Some(root),
        "INFO",
        "CREATE_TASK",
        &format!(
            "internal_create_task starting: title='{}', stage_id='{}', board_id={:?}, task_id_param={:?}",
            title, stage_id, board_id, task_id
        ),
        Some(&format!("actual_root='{}'", root.display())),
    );
    let mut tasks = load_tasks_from_disk(root).await;

    let effective_board_id = match board_id.as_deref() {
        Some(b) if !b.trim().is_empty() => b.trim().to_string(),
        _ => {
            load_boards_config(root)
                .await
                .map(|bc| bc.active_board_id)
                .unwrap_or_else(|_| "development".to_string())
        }
    };

    let clean_model = model
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    // Generate task ID — use explicit task_id (syncing sequence) or allocate monotonic ID
    let task_id = resolve_or_allocate_task_id(root, &tasks, task_id.as_deref()).await;
    crate::logger::cvh_log(
        Some(root),
        "INFO",
        "CREATE_TASK",
        &format!("Resolved task_id='{}' for task title='{}'", task_id, title),
    );
    let now = Utc::now().timestamp_millis();

    let next_pos = tasks
        .iter()
        .filter(|t| t.stage_id == stage_id)
        .filter_map(|t| t.position)
        .fold(0.0f64, f64::max) + 1000.0;

    let mut new_task = Task {
        id: task_id.clone(),
        title: title.clone(),
        description: description.clone(),
        stage_id: stage_id.clone(),
        board_id: Some(effective_board_id.clone()),
        worktree_path: None,
        branch: None,
        verdict: None,
        custom_flow: None,
        status: None,
        model: clean_model,
        parent_id: None,
        subtask_index: None,
        waiting_for_subtask_id: None,
        handover_summary: None,
        cover_image: None,
        attachments,
        todos: None,
        priority: None,
        tags: None,
        current_tokens: None,
        skip_worktree: skip_worktree.or(Some(false)),
        created_at: now,
        updated_at: now,
        stage_entered_at: Some(now),
        completed_at: None,
        position: Some(next_pos),
        hook_states: None,
    };

    // RACE CONDITION PREVENTION (AGENTS.md):
    // Atomically register the new task in in-memory state.tasks and save to disk IMMEDIATELY
    // before launching slow operations (worktree creation, hooks).
    // This guarantees that any concurrent move_task_stage or run_agent calls will find the task.
    if let Some(state_ref) = state {
        if let Ok(mut guard) = state_ref.tasks.lock() {
            if !guard.iter().any(|t| t.id == new_task.id) {
                guard.push(new_task.clone());
            }
            tasks = guard.clone();
        } else if !tasks.iter().any(|t| t.id == new_task.id) {
            tasks.push(new_task.clone());
        }
    } else if !tasks.iter().any(|t| t.id == new_task.id) {
        tasks.push(new_task.clone());
    }
    let _ = crate::db::db_upsert_task(root, &new_task);
    if let Err(e) = save_tasks_to_disk(root, &tasks).await {
        crate::logger::cvh_log(
            Some(root),
            "WARN",
            "CREATE_TASK",
            &format!("Initial save_tasks_to_disk warning for task '{}': {}. Task registered in state.", new_task.id, e),
        );
    }
    crate::logger::cvh_log(
        Some(root),
        "INFO",
        "CREATE_TASK",
        &format!("Initial task '{}' successfully registered in state and saved to disk (total: {})", new_task.id, tasks.len()),
    );
    if let Some(app) = app_handle {
        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
    }

    // Check pipeline config for stage hooks (board-aware: first check columns
    // of effective_board_id, then fallback to pipeline.json).
    // If skip_backlog_hook / skip_worktree is set, do NOT run on_enter hooks and do NOT provision builtin worktree.
    let skip = new_task.skip_worktree.unwrap_or(false);
    if !skip {
        let boards_cfg = load_boards_config(root).await.ok();
        let col_from_board = boards_cfg.as_ref().and_then(|bc| {
            let b = bc.boards.iter().find(|b| b.id == effective_board_id)
                .or_else(|| bc.boards.iter().find(|b| b.id == bc.active_board_id));
            b.and_then(|b| b.columns.iter().find(|c| c.id == stage_id).cloned())
        });
        let col_opt: Option<crate::config::PipelineColumn> = match col_from_board {
            Some(c) => Some(c),
            None => load_pipeline_config(root)
                .await
                .ok()
                .and_then(|p| p.columns.into_iter().find(|c| c.id == stage_id)),
        };
        if let Some(col) = col_opt {
            trigger_column_on_enter_script(&col, &new_task, &stage_id, None, root).await;
            if let Some(hook_cfg) = col.hooks.get("on_enter").cloned() {
                let hook_rel = hook_cfg.script().to_string();
                let hook_desc = hook_cfg.description().map(str::to_string);
                let hook_path = crate::config::resolve_hook_path(
                    root,
                    col.persona.as_deref(),
                    &hook_rel,
                );
                if hook_path.exists() {
                    if let Some(app) = app_handle {
                        let _ = app.emit("hook:start", serde_json::json!({
                            "taskId": task_id,
                            "stageId": stage_id,
                            "hookType": "on_enter",
                            "hookName": hook_rel,
                            "description": hook_desc,
                        }));
                    }
                    let hook_started_at = chrono::Utc::now().timestamp_millis();
                    let ctx = HookContext {
                        task_id: task_id.clone(),
                        task_title: title.clone(),
                        worktree_path: new_task.worktree_path.clone().unwrap_or_default(),
                        branch: new_task.branch.clone().unwrap_or_default(),
                        verdict: None,
                        stage_id: Some(stage_id.clone()),
                        board_id: Some(effective_board_id.clone()),
                        extra_env: HashMap::new(),
                    };
                    match execute_hook(&hook_path, root, &ctx, None).await {
                        Ok(res) => {
                            let duration_ms = (chrono::Utc::now().timestamp_millis() - hook_started_at).max(0) as u64;
                            if !res.success {
                                let err_details = if !res.stderr.trim().is_empty() {
                                    res.stderr.trim().to_string()
                                } else if !res.stdout.trim().is_empty() {
                                    res.stdout.trim().to_string()
                                } else {
                                    format!("Exit code {:?}", res.exit_code)
                                };
                                record_task_hook_result(
                                    &mut new_task,
                                    &stage_id,
                                    "on_enter",
                                    &hook_rel,
                                    hook_started_at,
                                    duration_ms,
                                    false,
                                    res.exit_code,
                                    &res.stdout,
                                    &res.stderr,
                                    Some(&err_details),
                                );
                                if let Some(app) = app_handle {
                                    let _ = app.emit("hook:finish", serde_json::json!({
                                        "taskId": task_id,
                                        "stageId": stage_id,
                                        "hookType": "on_enter",
                                        "success": false,
                                        "exitCode": res.exit_code,
                                        "stdout": res.stdout,
                                        "stderr": res.stderr,
                                        "error": err_details
                                    }));
                                }
                                let steer_content = format!(
                                    "### [ОТКЛОНЕНО ХУКОМ / HOOK REJECTED: {}]\n\nХук завершился с ошибкой (код {:?}).\n\n**Лог ошибки:**\n```\n{}\n```\n\nИсправь указанную ошибку.",
                                    hook_rel, res.exit_code, err_details
                                );
                                let _ = append_task_dialog_activity(
                                    root,
                                    &task_id,
                                    "steer",
                                    &steer_content,
                                    app_handle,
                                ).await;
                                new_task.verdict = Some("CHANGES_REQUESTED".to_string());
                            } else {
                                record_task_hook_result(
                                    &mut new_task,
                                    &stage_id,
                                    "on_enter",
                                    &hook_rel,
                                    hook_started_at,
                                    duration_ms,
                                    true,
                                    res.exit_code,
                                    &res.stdout,
                                    &res.stderr,
                                    None,
                                );
                                if let Some(app) = app_handle {
                                    let _ = app.emit("hook:finish", serde_json::json!({
                                        "taskId": task_id,
                                        "stageId": stage_id,
                                        "hookType": "on_enter",
                                        "success": true,
                                        "exitCode": res.exit_code,
                                        "stdout": res.stdout,
                                        "stderr": res.stderr
                                    }));
                                }
                                apply_worktree_from_hook_stdout(&mut new_task, &res.stdout);
                            }
                        }
                        Err(e) => {
                            let duration_ms = (chrono::Utc::now().timestamp_millis() - hook_started_at).max(0) as u64;
                            let err_details = e.to_string();
                            record_task_hook_result(
                                &mut new_task,
                                &stage_id,
                                "on_enter",
                                &hook_rel,
                                hook_started_at,
                                duration_ms,
                                false,
                                None,
                                "",
                                &err_details,
                                Some(&err_details),
                            );
                            if let Some(app) = app_handle {
                                let _ = app.emit("hook:finish", serde_json::json!({
                                    "taskId": task_id,
                                    "stageId": stage_id,
                                    "hookType": "on_enter",
                                    "success": false,
                                    "exitCode": None::<i32>,
                                    "stdout": "",
                                    "stderr": err_details,
                                    "error": err_details
                                }));
                            }
                            let steer_content = format!(
                                "### [ОТКЛОНЕНО ХУКОМ / HOOK REJECTED: {}]\n\nОшибка запуска хука: {}\n\nИсправь указанную ошибку.",
                                hook_rel, err_details
                            );
                            let _ = append_task_dialog_activity(
                                root,
                                &task_id,
                                "steer",
                                &steer_content,
                                app_handle,
                            ).await;
                            new_task.verdict = Some("CHANGES_REQUESTED".to_string());
                        }
                    }
                }
            }
        }
    }

    // After worktree creation and hooks, update worktree_path and branch fields.
    // If the task was moved or modified concurrently, preserve its latest stage_id and status.
    let mut updated_tasks = load_tasks_from_disk(root).await;
    if let Some(t) = updated_tasks.iter_mut().find(|t| t.id == new_task.id) {
        t.worktree_path = new_task.worktree_path.clone();
        t.branch = new_task.branch.clone();
        t.hook_states = new_task.hook_states.clone();
        t.updated_at = Utc::now().timestamp_millis();
        new_task = t.clone();
    } else {
        updated_tasks.push(new_task.clone());
    }

    if let Some(state_ref) = state {
        if let Ok(mut guard) = state_ref.tasks.lock() {
            if let Some(t) = guard.iter_mut().find(|t| t.id == new_task.id) {
                t.worktree_path = new_task.worktree_path.clone();
                t.branch = new_task.branch.clone();
                t.hook_states = new_task.hook_states.clone();
                t.updated_at = new_task.updated_at;
                new_task.stage_id = t.stage_id.clone();
                new_task.status = t.status.clone();
                new_task.board_id = t.board_id.clone();
            } else {
                guard.push(new_task.clone());
            }
            for mem_t in guard.iter() {
                if !updated_tasks.iter().any(|t| t.id == mem_t.id) {
                    updated_tasks.push(mem_t.clone());
                }
            }
        }
    }
    if let Some(t) = updated_tasks.iter_mut().find(|t| t.id == new_task.id) {
        t.stage_id = new_task.stage_id.clone();
        t.status = new_task.status.clone();
        t.board_id = new_task.board_id.clone();
    }
    if let Err(e) = save_tasks_to_disk(root, &updated_tasks).await {
        crate::logger::cvh_log(
            Some(root),
            "WARN",
            "CREATE_TASK",
            &format!("Warning: save_tasks_to_disk failed for task '{}': {}. In-memory state preserved.", new_task.id, e),
        );
    }
    crate::logger::cvh_log_with_details(
        Some(root),
        "INFO",
        "CREATE_TASK",
        &format!("Successfully created and persisted task '{}'", new_task.id),
        Some(&format!(
            "title='{}', stage='{}', board={:?}, worktree={:?}, branch={:?}",
            new_task.title, new_task.stage_id, new_task.board_id, new_task.worktree_path, new_task.branch
        )),
    );
    if let Some(app) = app_handle {
        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &updated_tasks }));
    }

    Ok(new_task)
}

pub fn is_review_column(col: &crate::config::PipelineColumn) -> bool {
    col.persona.as_deref() == Some("reviewer")
        || stage_id_matches(&col.id, "review")
        || col.id.to_lowercase().contains("review")
        || col.id.to_lowercase().contains("ревью")
}

#[allow(clippy::too_many_arguments)]
pub async fn internal_create_subtask(
    root: &Path,
    parent_id: &str,
    target_stage_id: Option<&str>,
    title: Option<&str>,
    handover_summary: Option<&str>,
    app_handle: Option<&AppHandle>,
) -> Result<Task, String> {
    let root_resolved = resolve_actual_repo_root(root);
    let root = &root_resolved;
    let mut tasks = load_tasks_from_disk(root).await;

    // Merge in-memory tasks if state is available in app_handle to prevent stale data
    if let Some(app) = app_handle {
        if let Some(state_ref) = app.try_state::<AppState>() {
            if let Ok(guard) = state_ref.tasks.lock() {
                for mem_t in guard.iter() {
                    if let Some(disk_t) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &mem_t.id)) {
                        if disk_t.worktree_path.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true) && mem_t.worktree_path.is_some() {
                            disk_t.worktree_path = mem_t.worktree_path.clone();
                        }
                        if disk_t.branch.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true) && mem_t.branch.is_some() {
                            disk_t.branch = mem_t.branch.clone();
                        }
                    }
                }
            }
        }
    }

    let parent_idx = tasks
        .iter()
        .position(|t| task_id_matches(&t.id, parent_id))
        .ok_or_else(|| format!("Parent task {} not found", parent_id))?;

    let boards_cfg = load_boards_config(root).await.ok();
    let effective_board = if let Some(ref bc) = boards_cfg {
        let parent_board_id = tasks[parent_idx].board_id.as_deref();
        bc.boards
            .iter()
            .find(|b| Some(b.id.as_str()) == parent_board_id)
            .cloned()
            .or_else(|| bc.boards.iter().find(|b| b.id == bc.active_board_id).cloned())
            .or_else(|| bc.boards.first().cloned())
    } else {
        None
    };

    let target_col = if let Some(target_id) = target_stage_id {
        let from_board = effective_board.as_ref().and_then(|b| {
            b.columns.iter().find(|c| stage_id_matches(&c.id, target_id)).cloned()
        });
        match from_board {
            Some(c) => c,
            None => {
                let pipeline = load_pipeline_config(root)
                    .await
                    .unwrap_or_else(|_| crate::config::default_pipeline_config());
                pipeline
                    .columns
                    .into_iter()
                    .find(|c| stage_id_matches(&c.id, target_id))
                    .ok_or_else(|| format!("Target column {} not found in pipeline", target_id))?
            }
        }
    } else {
        let cols = if let Some(ref b) = effective_board {
            b.columns.clone()
        } else {
            let pipeline = load_pipeline_config(root)
                .await
                .unwrap_or_else(|_| crate::config::default_pipeline_config());
            pipeline.columns
        };
        let parent_col_idx = cols
            .iter()
            .position(|c| stage_id_matches(&c.id, &tasks[parent_idx].stage_id))
            .ok_or_else(|| format!("Parent column {} not found in pipeline", tasks[parent_idx].stage_id))?;
        if parent_col_idx + 1 < cols.len() {
            cols[parent_col_idx + 1].clone()
        } else {
            return Err("Parent is already in the last pipeline stage".to_string());
        }
    };

    let next_subtask_idx = get_next_subtask_index(&tasks, parent_id);
    let subtask_id = format!("{}.{}", parent_id, next_subtask_idx);
    let now = Utc::now().timestamp_millis();

    let parent_title = tasks[parent_idx].title.clone();
    let mut parent_wt = tasks[parent_idx].worktree_path.clone();
    let mut parent_branch = tasks[parent_idx].branch.clone();
    let parent_model = tasks[parent_idx].model.clone();
    let parent_skip_wt = tasks[parent_idx].skip_worktree;

    // If parent worktree is missing or empty, check filesystem heuristics
    if parent_wt.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true) {
        let candidates = [
            format!("worktrees/{}", parent_id),
            format!("worktrees/{}", parent_id.to_lowercase()),
            format!(".cvh/worktrees/{}", parent_id),
            format!(".cvh/worktrees/{}", parent_id.to_lowercase()),
        ];
        for rel in &candidates {
            if root.join(rel).exists() {
                parent_wt = Some(rel.clone());
                tasks[parent_idx].worktree_path = Some(rel.clone());
                if parent_branch.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true) {
                    let fb = format!("feat/{}", parent_id);
                    parent_branch = Some(fb.clone());
                    tasks[parent_idx].branch = Some(fb);
                }
                let _ = crate::db::db_upsert_task(root, &tasks[parent_idx]);
                break;
            }
        }
    }

    // Update parent task: mark waiting for subtask, keep parent pinned in current stage (§ 2.2)
    tasks[parent_idx].waiting_for_subtask_id = Some(subtask_id.clone());
    tasks[parent_idx].status = Some("waiting_subtask".to_string());
    tasks[parent_idx].updated_at = now;

    let subtask_title = title
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{} ({})", parent_title, target_col.title));

    let subtask_desc = handover_summary
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!(
            "### Подзадача этапа: {}\nРодительская задача: {}\n\n{}",
            target_col.title, parent_id, tasks[parent_idx].description
        ));

    let subtask_status = if target_col.terminal == Some(true) {
        "done"
    } else if target_col.persona.is_some() || target_col.auto_run == Some(true) {
        "running"
    } else {
        "idle"
    };

    let next_subtask_pos = tasks
        .iter()
        .filter(|t| t.stage_id == target_col.id)
        .filter_map(|t| t.position)
        .fold(0.0f64, f64::max) + 1000.0;

    let new_subtask = Task {
        id: subtask_id.clone(),
        title: subtask_title,
        description: subtask_desc,
        stage_id: target_col.id.clone(),
        board_id: tasks[parent_idx].board_id.clone(),
        worktree_path: parent_wt,
        branch: parent_branch,
        verdict: None,
        custom_flow: None,
        status: Some(subtask_status.to_string()),
        model: parent_model.or_else(|| target_col.model_override.clone()),
        parent_id: Some(parent_id.to_string()),
        subtask_index: Some(next_subtask_idx),
        waiting_for_subtask_id: None,
        handover_summary: handover_summary.map(|s| s.to_string()),
        cover_image: None,
        attachments: tasks[parent_idx].attachments.clone(),
        todos: None,
        priority: tasks[parent_idx].priority.clone(),
        tags: None,
        current_tokens: None,
        skip_worktree: parent_skip_wt,
        created_at: now,
        updated_at: now,
        stage_entered_at: Some(now),
        completed_at: None,
        position: Some(next_subtask_pos),
        hook_states: tasks[parent_idx].hook_states.clone(),
    };

    let parent_task_clone = tasks[parent_idx].clone();
    tasks.push(new_subtask.clone());

    let _ = crate::db::db_upsert_task(root, &new_subtask);
    let _ = crate::db::db_upsert_task(root, &parent_task_clone);

    if let Some(app) = app_handle {
        if let Some(state_ref) = app.try_state::<AppState>() {
            if let Ok(mut guard) = state_ref.tasks.lock() {
                *guard = tasks.clone();
            }
        }
    }
    save_tasks_to_disk(root, &tasks).await?;

    if let Some(app) = app_handle {
        let _ = app.emit("task:created", serde_json::json!({ "task": &new_subtask }));
        let _ = app.emit("task:updated", serde_json::json!({ "task": &parent_task_clone }));
        let _ = app.emit(
            "task:moved",
            serde_json::json!({
                "taskId": parent_id,
                "status": "waiting_subtask",
                "waitingForSubtaskId": &subtask_id,
                "task": &parent_task_clone,
            }),
        );
        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));

        if target_col.auto_run == Some(true) || target_col.persona.is_some() {
            trigger_column_on_enter_script(
                &target_col,
                &new_subtask,
                &target_col.id,
                None,
                root,
            )
            .await;
            let _ = app.emit("agent:autorun", serde_json::json!({ "taskId": &subtask_id }));
        }
    }

    Ok(new_subtask)
}

pub async fn create_subtask(
    parent_id: String,
    target_stage_id: Option<String>,
    title: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    let root = state.get_root();
    internal_create_subtask(
        &root,
        &parent_id,
        target_stage_id.as_deref(),
        title.as_deref(),
        None,
        Some(&app_handle),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn update_task(
    task_id: String,
    model: Option<String>,
    title: Option<String>,
    description: Option<String>,
    status: Option<String>,
    verdict: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    let root = state.get_root();
    let mut tasks = load_tasks_from_disk(&root).await;
    let task_idx = tasks
        .iter()
        .position(|t| task_id_matches(&t.id, &task_id))
        .ok_or_else(|| format!("Task {} not found", task_id))?;

    let now = Utc::now().timestamp_millis();
    let task = &mut tasks[task_idx];

    if let Some(m) = model {
        let trimmed = m.trim().to_string();
        task.model = if trimmed.is_empty() { None } else { Some(trimmed) };
    }
    if let Some(t) = title {
        task.title = t;
    }
    if let Some(d) = description {
        task.description = d;
    }
    if let Some(s) = status {
        task.status = Some(s);
    }
    if let Some(v) = verdict {
        task.verdict = Some(v);
    }
    task.updated_at = now;

    let updated = task.clone();
    let _ = crate::db::db_upsert_task(&root, &updated);
    save_tasks_to_disk(&root, &tasks).await?;

    if let Ok(mut guard) = state.tasks.lock() {
        *guard = tasks.clone();
    }

    let _ = app_handle.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
    let _ = app_handle.emit("task:updated", serde_json::json!({ "task": &updated }));

    Ok(updated)
}

pub async fn delete_task(
    task_id: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let root = state.get_root();
    let mut tasks = load_tasks_from_disk(&root).await;

    // Collect target and any cascaded subtasks
    let mut to_delete = HashSet::new();
    to_delete.insert(task_id.clone());
    for t in &tasks {
        if task_id_matches(&t.id, &task_id) {
            to_delete.insert(t.id.clone());
        }
    }
    let mut grew = true;
    while grew {
        grew = false;
        for t in &tasks {
            if to_delete.iter().any(|id| task_id_matches(&t.id, id)) {
                continue;
            }
            if let Some(ref p) = t.parent_id {
                if to_delete.iter().any(|id| task_id_matches(p, id)) {
                    to_delete.insert(t.id.clone());
                    grew = true;
                    continue;
                }
            }
            if to_delete.iter().any(|id| t.id.to_ascii_lowercase().starts_with(&format!("{}.", id.trim().to_ascii_lowercase()))) {
                to_delete.insert(t.id.clone());
                grew = true;
            }
        }
    }

    // Monotonic sequence preservation: ensure sequence counter records max before deletion
    let pre_delete_max = max_task_num_on_disk(&tasks);
    let current_seq = load_task_sequence(&root).await;
    if pre_delete_max > current_seq {
        let _ = save_task_sequence(&root, pre_delete_max).await;
    }

    // Abort active agents for any deleted task/subtask (drop std::sync::MutexGuard before await)
    let handles: Vec<_> = {
        let mut entries = Vec::new();
        if let Ok(mut map) = state.active_agents.lock() {
            for id in &to_delete {
                if let Some(agent_entry) = map.remove(id) {
                    agent_entry.abort_flag.store(true, Ordering::SeqCst);
                    entries.push(agent_entry);
                }
            }
        }
        entries
    };

    for agent_entry in handles {
        let mut guard = agent_entry.join_handle.lock().await;
        if let Some(handle) = guard.take() {
            handle.abort();
        }
    }

    // Memory leak prevention: prune deleted task IDs and cascaded subtasks from agent_contexts
    if let Ok(mut ctx_map) = state.agent_contexts.lock() {
        for id in &to_delete {
            ctx_map.remove(id);
        }
    }

    // Collect branch info before removing from tasks vector
    let is_subtask = task_id.contains('.');
    let mut branches_to_delete = Vec::new();
    for t in &tasks {
        if to_delete.contains(&t.id) {
            if let Some(ref b) = t.branch {
                if !b.trim().is_empty() {
                    branches_to_delete.push(b.clone());
                }
            }
        }
    }

    // Delete from SQLite database tasks and todos
    for id in &to_delete {
        let _ = crate::db::db_delete_task(&root, id);
    }

    tasks.retain(|t| !to_delete.contains(&t.id));

    // Auto-unlock any parent that was waiting on a deleted subtask
    for t in &mut tasks {
        if let Some(ref waiting_id) = t.waiting_for_subtask_id {
            if to_delete.contains(waiting_id) {
                t.waiting_for_subtask_id = None;
                if t.status.as_deref() == Some("waiting_subtask") {
                    t.status = Some("idle".to_string());
                }
            }
        }
    }

    save_tasks_to_disk(&root, &tasks).await?;

    if let Ok(mut guard) = state.tasks.lock() {
        *guard = tasks.clone();
    }

    // Robust cleanup of git worktree and branch if top-level task
    if !is_subtask {
        let task_slug = task_id.to_lowercase();
        let _ = crate::git::remove_worktree(&root, &task_slug).await;
        let branch = format!("feat/{}", task_slug);
        let _ = crate::git::delete_branch(&root, &branch, true).await;
    }

    for branch in branches_to_delete {
        let _ = crate::git::delete_branch(&root, &branch, true).await;
    }

    // Defensively prune git worktree administrative records
    let _ = crate::git::prune_worktrees(&root).await;

    // Best-effort cleanup of SQLite dialogue items
    for id in &to_delete {
        let _ = crate::db::clear_dialog_items(&root, id);
    }

    // Broadcast tasks:updated and task:deleted events across application
    let _ = app_handle.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
    let _ = app_handle.emit(
        "task:deleted",
        serde_json::json!({
            "taskId": &task_id,
            "deletedIds": &to_delete.iter().cloned().collect::<Vec<_>>()
        }),
    );

    Ok(())
}

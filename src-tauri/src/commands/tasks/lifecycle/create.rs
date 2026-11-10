use std::collections::HashMap;
use std::path::Path;
use chrono::Utc;
use tauri::{AppHandle, Emitter, State};

use crate::agent::Task;
use crate::commands::dialogs::append_task_dialog_activity;
use crate::commands::git::apply_worktree_from_hook_stdout;
use crate::commands::state::AppState;
use crate::config::{load_boards_config, resolve_actual_repo_root};
use crate::hooks::{execute_hook, HookContext};
use crate::commands::healing::stage_id_matches;
use crate::commands::tasks::storage::{load_tasks_from_disk, resolve_or_allocate_task_id, save_tasks_to_disk};
use super::hook_records::record_task_hook_result;

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
        .map(str::to_string);

    let task_id = resolve_or_allocate_task_id(root, &tasks, task_id.as_deref()).await;

    let now = Utc::now().timestamp_millis();
    let max_pos = tasks
        .iter()
        .filter(|t| t.stage_id == stage_id)
        .filter_map(|t| t.position)
        .fold(0.0f64, f64::max);

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
        status: Some("idle".to_string()),
        model: clean_model,
        provider_id: None,
        reasoning_effort: None,
        parent_id: None,
        subtask_index: None,
        waiting_for_subtask_id: None,
        handover_summary: None,
        cover_image: None,
        attachments,
        todos: None,
        priority: Some("medium".to_string()),
        tags: None,
        current_tokens: None,
        skip_worktree,
        created_at: now,
        updated_at: now,
        stage_entered_at: Some(now),
        completed_at: None,
        position: Some(max_pos + 1000.0),
        hook_states: None,
        project_path: Some(root.to_string_lossy().to_string()),
        stage_durations: None,
        total_elapsed: None,
    };

    // Persist immediately to SQLite so hooks or subsequent transitions find the task
    let _ = crate::db::db_upsert_task(root, &new_task);

    tasks.push(new_task.clone());
    let _ = save_tasks_to_disk(root, &tasks).await;

    if let Some(state_ref) = state {
        if state_ref.get_root().as_path() == root {
            let mut guard = state_ref.tasks.lock().unwrap_or_else(|p| p.into_inner());
            guard.retain(|t| t.id != new_task.id);
            guard.push(new_task.clone());
        }
    }

    if let Some(app) = app_handle {
        let project_path_str = root.to_string_lossy().to_string();
        let _ = app.emit("task:created", serde_json::json!({ "task": &new_task, "projectPath": &project_path_str }));
        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks, "projectPath": &project_path_str }));
    }

    // Run custom on_enter hook for the creation column if configured
    let boards_cfg = load_boards_config(root).await.ok();
    let col_opt = if let Some(ref bc) = boards_cfg {
        bc.boards
            .iter()
            .find(|b| b.id == effective_board_id)
            .and_then(|b| b.stages.iter().find(|c| stage_id_matches(&c.id, &stage_id)).cloned())
    } else {
        None
    };

    if let Some(col) = col_opt {
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

    // After hooks, update worktree_path and branch fields.
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
        if state_ref.get_root().as_path() == root {
            let mut guard = state_ref.tasks.lock().unwrap_or_else(|p| p.into_inner());
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
        let project_path_str = root.to_string_lossy().to_string();
        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &updated_tasks, "projectPath": &project_path_str }));
    }

    Ok(new_task)
}

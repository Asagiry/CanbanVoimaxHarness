use std::path::Path;
use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::agent::Task;
use crate::commands::state::AppState;
use crate::config::{load_boards_config, load_pipeline_config, resolve_actual_repo_root};
use crate::commands::healing::{stage_id_matches, task_id_matches};
use crate::commands::tasks::storage::{get_next_subtask_index, load_tasks_from_disk, save_tasks_to_disk};
use crate::commands::tasks::timing::freeze_stage_duration;
use crate::commands::tasks::transitions::trigger_column_on_enter_script;

#[allow(clippy::too_many_arguments)]
pub async fn internal_create_subtask(
    root: &Path,
    parent_id: &str,
    target_stage_id: Option<&str>,
    title: Option<&str>,
    handover_summary: Option<&str>,
    app_handle: Option<&AppHandle>,
) -> Result<Task, String> {
    internal_create_subtask_with_hook_states(
        root,
        parent_id,
        target_stage_id,
        title,
        handover_summary,
        app_handle,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn internal_create_subtask_with_hook_states(
    root: &Path,
    parent_id: &str,
    target_stage_id: Option<&str>,
    title: Option<&str>,
    handover_summary: Option<&str>,
    app_handle: Option<&AppHandle>,
    hook_states_override: Option<serde_json::Value>,
) -> Result<Task, String> {
    let root_resolved = resolve_actual_repo_root(root);
    let root = &root_resolved;
    let mut tasks = load_tasks_from_disk(root).await;

    // Merge in-memory tasks if state is available in app_handle to prevent stale data
    if let Some(app) = app_handle {
        if let Some(state_ref) = app.try_state::<AppState>() {
            let guard = state_ref.tasks.lock().unwrap_or_else(|p| p.into_inner());
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

    let parent_idx = tasks
        .iter()
        .position(|t| task_id_matches(&t.id, parent_id))
        .ok_or_else(|| format!("Parent task {} not found", parent_id))?;

    let boards_cfg = load_boards_config(root).await.ok();
    let effective_board = if let Some(ref bc) = boards_cfg {
        let parent_board_id = tasks[parent_idx].board_id.as_deref();
        let board_has_target = |b: &crate::config::BoardDef| -> bool {
            target_stage_id.is_none_or(|tid| b.stages.iter().any(|c| stage_id_matches(&c.id, tid)))
        };
        bc.boards
            .iter()
            .find(|b| Some(b.id.as_str()) == parent_board_id && board_has_target(b))
            .cloned()
            .or_else(|| {
                bc.boards
                    .iter()
                    .find(|b| b.id == bc.active_board_id && board_has_target(b))
                    .cloned()
            })
            .or_else(|| bc.boards.iter().find(|b| board_has_target(b)).cloned())
            .or_else(|| {
                bc.boards
                    .iter()
                    .find(|b| Some(b.id.as_str()) == parent_board_id)
                    .cloned()
            })
            .or_else(|| bc.boards.iter().find(|b| b.id == bc.active_board_id).cloned())
            .or_else(|| bc.boards.first().cloned())
    } else {
        None
    };

    let target_col = if let Some(target_id) = target_stage_id {
        let from_board = effective_board.as_ref().and_then(|b| {
            b.stages.iter().find(|c| stage_id_matches(&c.id, target_id)).cloned()
        });
        match from_board {
            Some(c) => c,
            None => {
                let pipeline = load_pipeline_config(root)
                    .await
                    .unwrap_or_else(|_| crate::config::default_pipeline_config());
                let col = pipeline
                    .stages
                    .into_iter()
                    .find(|c| stage_id_matches(&c.id, target_id))
                    .ok_or_else(|| format!("Target column {} not found in pipeline", target_id))?;
                if stage_id_matches(&col.id, &tasks[parent_idx].stage_id)
                    && !stage_id_matches(&col.id, target_id)
                {
                    return Err(format!(
                        "Target column {} resolved to parent stage {} — refusing to create subtask in source column",
                        target_id, col.id
                    ));
                }
                col
            }
        }
    } else {
        let cols = if let Some(ref b) = effective_board {
            b.stages.clone()
        } else {
            let pipeline = load_pipeline_config(root)
                .await
                .unwrap_or_else(|_| crate::config::default_pipeline_config());
            pipeline.stages
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

    // Update parent task: mark waiting for subtask only when subtask is non-terminal
    // Do not point parent at a terminal subtask (defense in depth against stale badges)
    if target_col.terminal == Some(true) {
        tasks[parent_idx].waiting_for_subtask_id = None;
    } else {
        tasks[parent_idx].waiting_for_subtask_id = Some(subtask_id.clone());
    }
    tasks[parent_idx].status = Some("done".to_string());
    freeze_stage_duration(&mut tasks[parent_idx], now);
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

    if stage_id_matches(&target_col.id, &tasks[parent_idx].stage_id)
        && target_stage_id.is_some_and(|tid| !stage_id_matches(tid, &tasks[parent_idx].stage_id))
    {
        return Err(format!(
            "Target column {} resolved to parent stage {} — refusing to create subtask in source column",
            target_col.id, tasks[parent_idx].stage_id
        ));
    }

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
        model: parent_model.clone(),
        provider_id: tasks[parent_idx].provider_id.clone(),
        reasoning_effort: tasks[parent_idx].reasoning_effort.clone(),
        parent_id: Some(parent_id.to_string()),
        subtask_index: Some(next_subtask_idx),
        waiting_for_subtask_id: None,
        handover_summary: handover_summary.map(|s| s.to_string()),
        cover_image: None,
        attachments: None,
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
        hook_states: hook_states_override.clone().or_else(|| tasks[parent_idx].hook_states.clone()),
        project_path: Some(root.to_string_lossy().to_string()),
        stage_durations: None,
        total_elapsed: None,
    };

    let parent_task_clone = tasks[parent_idx].clone();
    tasks.push(new_subtask.clone());

    let _ = crate::db::db_upsert_task(root, &new_subtask);
    let _ = crate::db::db_upsert_task(root, &parent_task_clone);

    if let Some(app) = app_handle {
        if let Some(state_ref) = app.try_state::<AppState>() {
            if state_ref.get_root().as_path() == root {
                let mut guard = state_ref.tasks.lock().unwrap_or_else(|p| p.into_inner());
                *guard = tasks.clone();
            }
        }
    }
    save_tasks_to_disk(root, &tasks).await?;

    if let Some(app) = app_handle {
        let project_path_str = root.to_string_lossy().to_string();
        let _ = app.emit("task:created", serde_json::json!({ "task": &new_subtask, "projectPath": &project_path_str }));
        let _ = app.emit("task:updated", serde_json::json!({ "task": &parent_task_clone, "projectPath": &project_path_str }));
        let _ = app.emit(
            "task:moved",
            serde_json::json!({
                "taskId": parent_id,
                "status": "idle",
                "waitingForSubtaskId": &subtask_id,
                "task": &parent_task_clone,
                "projectPath": &project_path_str,
            }),
        );
        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks, "projectPath": &project_path_str }));

        if target_col.auto_run == Some(true) || target_col.persona.is_some() {
            // New subtask already carries parent hook_states (incl. any
            // `running` on_enter recorded by the preceding pipeline run)
            // for the target stage — re-emit it for the new id so
            // CardHookNode on the fresh card shows amber running instead
            // of idle. No-op when parent had no on_enter state.
            if let Some(hook_states) = new_subtask.hook_states.clone() {
                if let Some(enter) = hook_states.get("on_enter").cloned() {
                    match enter.get("status").and_then(|s| s.as_str()) {
                        Some("running") => {
                            let _ = app.emit("hook:start", serde_json::json!({
                                "taskId": subtask_id,
                                "stageId": target_col.id,
                                "hookType": "on_enter",
                                "hookName": enter.get("hookName").and_then(|v| v.as_str()).unwrap_or("on_enter"),
                                "description": enter.get("description"),
                            }));
                        }
                        Some("success") | Some("failed") => {
                            let success = enter.get("status").and_then(|s| s.as_str()) == Some("success");
                            let _ = app.emit("hook:finish", serde_json::json!({
                                "taskId": subtask_id,
                                "stageId": target_col.id,
                                "hookType": "on_enter",
                                "success": success,
                                "exitCode": enter.get("exitCode"),
                                "stdout": enter.get("stdout").and_then(|v| v.as_str()).unwrap_or(""),
                                "stderr": enter.get("stderr").and_then(|v| v.as_str()).unwrap_or(""),
                                "error": enter.get("error"),
                                "hookName": enter.get("hookName").and_then(|v| v.as_str()).unwrap_or("on_enter"),
                            }));
                        }
                        _ => {}
                    }
                }
            }
            let mut subtask_for_hook = new_subtask.clone();
            trigger_column_on_enter_script(
                &target_col,
                &mut subtask_for_hook,
                &target_col.id,
                None,
                root,
                app_handle,
            )
            .await;
            if subtask_for_hook.hook_states != new_subtask.hook_states {
                let _ = crate::db::db_upsert_task(root, &subtask_for_hook);
                if let Some(state_ref) = app.try_state::<AppState>() {
                    if state_ref.get_root().as_path() == root {
                        let mut guard = state_ref.tasks.lock().unwrap_or_else(|p| p.into_inner());
                        if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, &subtask_for_hook.id)) {
                            t.hook_states = subtask_for_hook.hook_states.clone();
                        }
                    }
                }
            }
            let _ = app.emit("agent:autorun", serde_json::json!({ "taskId": &subtask_id, "projectPath": &project_path_str }));
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

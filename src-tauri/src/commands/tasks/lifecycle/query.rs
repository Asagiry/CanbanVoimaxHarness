use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::State;

use crate::agent::Task;
use crate::commands::state::AppState;
use crate::config::resolve_actual_repo_root;
use crate::commands::healing::stage_id_matches;
use crate::commands::tasks::storage::{save_tasks_to_disk, try_load_tasks_from_disk};

pub async fn list_tasks(state: State<'_, AppState>) -> Result<Vec<Task>, String> {
    let root = resolve_actual_repo_root(&state.get_root());
    let mut tasks = try_load_tasks_from_disk(&root).await?;

    // Self-healing orphan running tasks: if a task is marked "running" in SQLite
    // but has no active Tokio execution in AppState (e.g. after desktop restart or crash),
    // reconcile it safely to "idle".
    let mut reconciled = false;
    for t in tasks.iter_mut() {
        if t.status.as_deref() == Some("running") && !state.is_agent_active(&root, &t.id) {
            crate::logger::cvh_log(
                Some(&root),
                "INFO",
                "LIST_TASKS",
                &format!("Reconciling orphan running task '{}' to idle (no active Tokio agent found)", t.id),
            );
            t.status = Some("idle".to_string());
            reconciled = true;
        }
    }
    if reconciled {
        let _ = save_tasks_to_disk(&root, &tasks).await;
    }

    let ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    crate::logger::cvh_log_with_details(
        Some(&root),
        "INFO",
        "LIST_TASKS",
        &format!("list_tasks command: returning {} tasks for root '{}'", tasks.len(), root.display()),
        Some(&format!("Task IDs: [{}]", ids.join(", "))),
    );
    let mut guard = state.tasks.lock().unwrap_or_else(|p| p.into_inner());
    *guard = tasks.clone();
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
    let foreign_incoming = crate::commands::tasks::storage::find_foreign_task_ids(&tasks, &actual_root);
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

    let disk_tasks = try_load_tasks_from_disk(root).await?;
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
                || boards_cfg.as_ref().is_some_and(|cfg| {
                    cfg.boards.iter().any(|b| {
                        b.stages.iter().any(|c| stage_id_matches(&c.id, &t.stage_id) && c.terminal == Some(true))
                    })
                });

            if is_term {
                t.waiting_for_subtask_id = None;
                if t.status.as_deref() == Some("waiting_subtask") {
                    t.status = Some("done".to_string());
                }
            } else {
                if t.status.as_deref() == Some("waiting_subtask") {
                    t.status = Some("idle".to_string());
                }
                if dt.waiting_for_subtask_id.is_some() && t.waiting_for_subtask_id.is_none() {
                    if let Some(ref sub_id) = dt.waiting_for_subtask_id {
                        if active_sub_ids.contains(sub_id) {
                            t.waiting_for_subtask_id = Some(sub_id.clone());
                        }
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
            if (t.todos.is_none() || t.todos.as_ref().map(|v| v.is_empty()).unwrap_or(false)) && dt.todos.is_some() {
                t.todos = dt.todos.clone();
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
        let mut guard = st.lock().unwrap_or_else(|p| p.into_inner());
        *guard = merged_tasks.clone();
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

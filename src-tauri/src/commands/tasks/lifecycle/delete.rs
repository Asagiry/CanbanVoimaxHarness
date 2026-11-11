use std::collections::HashSet;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, State};

use crate::agent::Task;
use crate::commands::state::AppState;
use crate::commands::healing::task_id_matches;
use crate::commands::tasks::storage::{
    load_task_sequence, max_task_num_on_disk, save_task_sequence, try_load_tasks_from_disk,
};

pub async fn delete_task(
    task_id: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let root = state.get_root();
    let mut tasks = try_load_tasks_from_disk(&root).await?;

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
        for id in &to_delete {
            if let Some(agent_entry) = state.remove_active_agent(&root, id) {
                agent_entry.abort_flag.store(true, Ordering::SeqCst);
                entries.push(agent_entry);
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
    for id in &to_delete {
        state.remove_agent_context(&root, id);
    }

    // Collect branch and worktree info before removing from tasks vector
    let is_subtask = task_id.contains('.');
    let mut branches_to_delete = Vec::new();
    let mut worktree_slugs_to_delete = Vec::new();
    for t in &tasks {
        if to_delete.iter().any(|d| task_id_matches(&t.id, d)) {
            if let Some(ref b) = t.branch {
                if !b.trim().is_empty() && !branches_to_delete.contains(b) {
                    branches_to_delete.push(b.clone());
                }
            }
            if !is_subtask {
                if let Some(ref wt) = t.worktree_path {
                    if let Some(name) = std::path::Path::new(wt.trim()).file_name().and_then(|n| n.to_str()) {
                        if !name.is_empty() && !worktree_slugs_to_delete.iter().any(|s: &String| s == name) {
                            worktree_slugs_to_delete.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    // Atomic cascade deletion in SQLite: deletes target tasks, subtasks, todos, telemetry, dialog_items
    // and unlocks waiting parents in a single TransactionBehavior::Immediate transaction.
    crate::db::db_delete_tasks_cascade(&root, &to_delete)?;

    // Synchronize in-memory cache directly without re-saving stale snapshots over concurrent updates.
    let remaining_tasks: Vec<Task> = if state.is_same_root(&root) {
        let mut guard = state.tasks.lock().unwrap_or_else(|p| p.into_inner());
        guard.retain(|t| !to_delete.iter().any(|d| task_id_matches(&t.id, d)));
        for t in guard.iter_mut() {
            if let Some(ref waiting_id) = t.waiting_for_subtask_id {
                if to_delete.iter().any(|d| task_id_matches(waiting_id, d)) {
                    t.waiting_for_subtask_id = None;
                    if t.status.as_deref() == Some("waiting_subtask") || t.status.as_deref() == Some("done") {
                        t.status = Some("idle".to_string());
                    }
                }
            }
        }
        guard.clone()
    } else {
        tasks.retain(|t| !to_delete.iter().any(|d| task_id_matches(&t.id, d)));
        for t in &mut tasks {
            if let Some(ref waiting_id) = t.waiting_for_subtask_id {
                if to_delete.iter().any(|d| task_id_matches(waiting_id, d)) {
                    t.waiting_for_subtask_id = None;
                    if t.status.as_deref() == Some("waiting_subtask") || t.status.as_deref() == Some("done") {
                        t.status = Some("idle".to_string());
                    }
                }
            }
        }
        tasks
    };

    // Robust cleanup of git worktree and branch if top-level task
    if !is_subtask {
        let raw_slug = task_id.trim();
        if !raw_slug.is_empty() && !worktree_slugs_to_delete.iter().any(|s| s == raw_slug) {
            worktree_slugs_to_delete.push(raw_slug.to_string());
        }
        let lower_slug = raw_slug.to_lowercase();
        if !lower_slug.is_empty() && !worktree_slugs_to_delete.iter().any(|s| s == &lower_slug) {
            worktree_slugs_to_delete.push(lower_slug);
        }
        for slug in &worktree_slugs_to_delete {
            crate::git::remove_worktree(&root, slug)
                .await
                .map_err(|e| format!("Failed to remove worktree '{}': {}", slug, e))?;
            let branch = format!("feat/{}", slug);
            if !branches_to_delete.contains(&branch) {
                branches_to_delete.push(branch);
            }
        }
    }

    for branch in branches_to_delete {
        if let Err(e) = crate::git::delete_branch(&root, &branch, true).await {
            let msg = e.to_string();
            if !msg.contains("not found") {
                eprintln!("CVH [delete_task]: Failed to delete branch '{}': {}", branch, msg);
            }
        }
    }

    // Defensively prune git worktree administrative records
    if let Err(e) = crate::git::prune_worktrees(&root).await {
        eprintln!("CVH [delete_task]: Failed to prune git worktrees: {}", e);
    }

    // Broadcast tasks:updated and task:deleted events across application
    let project_path_str = root.to_string_lossy().to_string();
    if let Err(e) = app_handle.emit("tasks:updated", serde_json::json!({ "tasks": &remaining_tasks, "projectPath": &project_path_str })) {
        eprintln!("CVH [WARN]: Failed to emit tasks:updated after delete: {}", e);
    }
    if let Err(e) = app_handle.emit(
        "task:deleted",
        serde_json::json!({
            "taskId": &task_id,
            "deletedIds": &to_delete.iter().cloned().collect::<Vec<_>>(),
            "projectPath": &project_path_str,
        }),
    ) {
        eprintln!("CVH [WARN]: Failed to emit task:deleted: {}", e);
    }

    Ok(())
}

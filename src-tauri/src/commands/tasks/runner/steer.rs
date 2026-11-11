use std::path::Path;
use std::sync::atomic::Ordering;
use chrono::Utc;
use tauri::{AppHandle, Emitter, State};

use crate::commands::state::{ActiveAgentHandle, AppState};
use crate::commands::healing::{stage_id_matches, task_id_matches};
use crate::commands::tasks::storage::{save_tasks_to_disk, try_load_tasks_from_disk};
use super::execution::run_agent_inner;

pub fn persist_or_queue_steer(
    root: &Path,
    task_id: &str,
    message: &str,
    images: Option<Vec<String>>,
    message_id: Option<&str>,
    active: Option<&ActiveAgentHandle>,
) -> Result<(bool, Option<crate::db::ActivityItem>), String> {
    if let Some(entry) = active {
        if let Some(steering) = &entry.steering {
            let gate = steering.lock().unwrap_or_else(|p| p.into_inner());
            let finished = entry.join_handle.try_lock().ok()
                .and_then(|handle| handle.as_ref().map(|handle| handle.is_finished())).unwrap_or(false);
            let queued = gate.accepting && !entry.abort_flag.load(Ordering::SeqCst) && !finished;
            let item = crate::agent::persist_steer_message(root, task_id, message, images, message_id)?;
            return Ok((queued, item));
        }
    }
    Ok((false, crate::agent::persist_steer_message(root, task_id, message, images, message_id)?))
}

pub async fn steer_agent(
    task_id: String,
    message: String,
    images: Option<Vec<String>>,
    message_id: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let root = state.get_root();

    let task_found = {
        let guard = state.tasks.lock().unwrap_or_else(|p| p.into_inner());
        guard.iter().any(|t| task_id_matches(&t.id, &task_id))
    };

    if !task_found {
        let tasks = try_load_tasks_from_disk(&root).await?;
        if !tasks.iter().any(|t| task_id_matches(&t.id, &task_id)) {
            return Err(format!("Task {} not found", task_id));
        }
    }

    let previous = state.get_active_agent(&root, &task_id);
    let was_aborted = previous.as_ref().is_some_and(|entry| entry.abort_flag.load(Ordering::SeqCst));
    let message_id = message_id.filter(|id| !id.trim().is_empty()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let (queued, item) = {
        let root_b = root.clone();
        let tid_b = task_id.clone();
        let msg_b = message.clone();
        let imgs_b = images.clone();
        let mid_b = message_id.clone();
        let prev_b = previous.clone();
        tokio::task::spawn_blocking(move || {
            persist_or_queue_steer(&root_b, &tid_b, &msg_b, imgs_b, Some(&mid_b), prev_b.as_ref())
        })
        .await
        .map_err(|e| format!("Steer persist join error: {}", e))??
    };
    let inserted = item.is_some();
    if let Some(item) = item {
        let payload = serde_json::json!({ "taskId": task_id, "item": item });
        let _ = app_handle.emit("task:activity", &payload);
        let _ = app_handle.emit("task:dialog", &payload);
    }

    let steer_payload = serde_json::json!({
        "taskId": task_id,
        "content": message,
        "type": "steer",
        "images": images,
    });
    let _ = app_handle.emit("agent:steer", steer_payload);

    if !inserted {
        let root_b = root.clone();
        let tid_b = task_id.clone();
        let mid_b = message_id.clone();
        let already_delivered = tokio::task::spawn_blocking(move || -> Result<bool, String> {
            let cursor = crate::db::load_conversation_context(&root_b, &tid_b)?
                .map(|context| context.dialog_cursor)
                .unwrap_or(0);
            let items = crate::db::load_dialog_items_after(&root_b, &tid_b, cursor)?;
            Ok(!items.iter().any(|item| item.id == mid_b))
        })
        .await
        .map_err(|e| format!("Steer check join error: {}", e))??;
        if already_delivered {
            return Ok(format!("Steer message {} was already delivered", message_id));
        }
    }
    if queued {
        return Ok(format!("Steer queued for task {} until the current tool batch completes", task_id));
    }
    if let Some(entry) = previous {
        let is_running = entry.join_handle.try_lock().ok()
            .and_then(|h| h.as_ref().map(|h| !h.is_finished()))
            .unwrap_or(false);
        if is_running && !entry.abort_flag.load(Ordering::SeqCst) {
            return Ok(format!("Steer queued for active task {}", task_id));
        }
        if !was_aborted && entry.abort_flag.load(Ordering::SeqCst) && !entry.advance_aborted.load(Ordering::SeqCst) {
            return Ok(format!("Steer saved for stopped task {}", task_id));
        }
        // Wait for any aborting/unwinding prior JoinHandle to exit cleanly before restarting.
        let maybe_handle = {
            let mut guard = entry.join_handle.lock().await;
            guard.take()
        };
        if let Some(handle) = maybe_handle {
            if !handle.is_finished()
                && tokio::time::timeout(std::time::Duration::from_secs(3), handle).await.is_err()
            {
                eprintln!("CVH [steer_agent]: Timed out waiting for prior agent JoinHandle for task {}", task_id);
            }
        }
        state.remove_active_agent_if_same(&root, &task_id, &entry.abort_flag);
    }

    let disk_tasks = try_load_tasks_from_disk(&root).await?;
    let current_task = disk_tasks.iter().find(|t| task_id_matches(&t.id, &task_id)).cloned();
    if let Some(ref t) = current_task {
        let is_terminal_stage = stage_id_matches(&t.stage_id, "ready") || stage_id_matches(&t.stage_id, "done");
        let is_terminal = t.completed_at.is_some() && is_terminal_stage;
        if is_terminal {
            return Ok(format!("Task completed; steer saved to dialog, reopen to continue {}", task_id));
        }
        let is_waiting_subtask = t.status.as_deref() == Some("waiting_subtask")
            || t.status.as_deref() == Some("waiting_subagent");
        if is_waiting_subtask {
            return Ok(format!("Steer queued for task {} waiting for subtask to finish", task_id));
        }
    }

    {
        let mut current_tasks = try_load_tasks_from_disk(&root).await?;
        if let Some(t) = current_tasks.iter_mut().find(|t| task_id_matches(&t.id, &task_id)) {
            t.updated_at = Utc::now().timestamp_millis();
            t.waiting_for_subtask_id = None;
            t.completed_at = None;
            save_tasks_to_disk(&root, &current_tasks).await?;
        }
        if state.is_same_root(&root) {
            let mut guard = state.tasks.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, &task_id)) {
                t.updated_at = Utc::now().timestamp_millis();
                t.waiting_for_subtask_id = None;
                t.completed_at = None;
            }
        }
    }

    run_agent_inner(task_id, Some(message), images, Some(root.to_string_lossy().to_string()), app_handle, false).await
}

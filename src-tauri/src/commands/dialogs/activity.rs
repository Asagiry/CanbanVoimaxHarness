use crate::commands::state::AppState;
pub use crate::db::ActivityItem;
use std::path::Path;
use tauri::{AppHandle, Emitter, State};

pub fn sanitize_task_id(task_id: &str) -> Result<String, String> {
    let trimmed = task_id.trim();
    if trimmed.is_empty() {
        return Err("Task ID cannot be empty".to_string());
    }
    if trimmed.contains("..") || trimmed.contains('/') || trimmed.contains('\\') {
        return Err(format!("Invalid task ID: '{}'", trimmed));
    }
    // Allow ':' strictly when part of the subagent delimiter ":sub:"
    let without_sub = trimmed.replace(":sub:", "");
    if without_sub.chars().any(|c| matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*')) {
        return Err(format!("Task ID contains invalid characters: '{}'", trimmed));
    }
    Ok(trimmed.to_string())
}

#[tauri::command]
pub async fn save_task_activity(
    task_id: String,
    items: Vec<ActivityItem>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let safe_id = sanitize_task_id(&task_id)?;
    let root = state.get_root();
    crate::db::append_dialog_items(&root, &safe_id, &items)?;
    eprintln!("CVH [save_task_activity]: saved {} items to SQLite for task {}", items.len(), safe_id);
    Ok(())
}

#[tauri::command]
pub async fn save_task_dialog(
    task_id: String,
    items: Vec<ActivityItem>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    save_task_activity(task_id, items, state).await
}

pub async fn append_task_activity(
    root: &Path,
    task_id: &str,
    item_type: &str,
    content: &str,
    app_handle: Option<&AppHandle>,
) -> Result<(), String> {
    let safe_id = sanitize_task_id(task_id)?;
    let new_item = ActivityItem {
        id: uuid::Uuid::new_v4().to_string(),
        item_type: item_type.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        content: Some(content.to_string()),
        tool: None,
        duration_ms: None,
        images: None,
    };

    crate::db::append_dialog_items(root, &safe_id, std::slice::from_ref(&new_item))?;

    if let Some(app) = app_handle {
        let payload = serde_json::json!({
            "taskId": task_id,
            "item": new_item,
        });
        let _ = app.emit("task:activity", &payload);
        let _ = app.emit("task:dialog", &payload);
    }

    Ok(())
}

pub async fn append_task_dialog_activity(
    root: &Path,
    task_id: &str,
    item_type: &str,
    content: &str,
    app_handle: Option<&AppHandle>,
) -> Result<(), String> {
    append_task_activity(root, task_id, item_type, content, app_handle).await
}

#[tauri::command]
pub async fn load_task_activity(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ActivityItem>, String> {
    let safe_id = sanitize_task_id(&task_id)?;
    let root = state.get_root();
    crate::db::load_dialog_items(&root, &safe_id)
}

#[tauri::command]
pub async fn load_task_dialog(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ActivityItem>, String> {
    load_task_activity(task_id, state).await
}

/// Receives renderer-side JS errors so white-screen crashes land in the backend log.
#[tauri::command]
pub async fn log_ui_error(message: String) -> Result<(), String> {
    let trimmed = message.chars().take(4000).collect::<String>();
    eprintln!("CVH [UI-ERROR]: {}", trimmed);
    Ok(())
}

#[tauri::command]
pub async fn delete_dialog_item(
    task_id: String,
    item_id: String,
    items: Option<Vec<ActivityItem>>,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let safe_id = sanitize_task_id(&task_id)?;
    let root = state.get_root();
    let safe_id = crate::db::resolve_dialog_item_task_id(&root, &safe_id, &item_id)?;
    let parent_id = safe_id.split(":sub:").next().unwrap_or(&safe_id);
    let sessions = crate::agent::SUBAGENT_SESSIONS.lock().await;
    for session in sessions.values() {
        let info = session.retry_info.try_lock().map_err(|_| "Agent session is busy; retry deletion after it stops")?;
        if AppState::normalize_root_key(&info.ctx.repo_root) == AppState::normalize_root_key(&root)
            && (info.parent_task_id == parent_id || info.subagent_id == safe_id)
            && info.status == "running"
            && !session.abort_handle.load(std::sync::atomic::Ordering::SeqCst)
        {
            return Err("Stop the agent before deleting messages".to_string());
        }
    }
    let active = state.active_agents.lock().map_err(|_| "Failed to lock active agents")?;
    if AppState::is_agent_active_in_map(&active, &root, parent_id) {
        return Err("Stop the agent before deleting messages".to_string());
    }
    crate::db::append_dialog_items(&root, &safe_id, items.as_deref().unwrap_or(&[]))?;
    let deleted_ids = crate::db::delete_dialog_item(&root, &safe_id, &item_id)?;
    let mut contexts = state.agent_contexts.lock().map_err(|_| "Failed to lock agent contexts")?;
    contexts.remove(&AppState::agent_key(&root, &safe_id));
    contexts.remove(&safe_id);
    drop(contexts);
    drop(active);
    drop(sessions);

    let _ = app_handle.emit("dialog:updated", serde_json::json!({ "taskId": &safe_id, "deletedItemId": &item_id }));
    Ok(deleted_ids)
}

#[tauri::command]
pub async fn clear_task_activity(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let safe_id = sanitize_task_id(&task_id)?;
    let root = state.get_root();
    crate::db::clear_dialog_items(&root, &safe_id)
}

#[tauri::command]
pub async fn clear_task_dialog(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    clear_task_activity(task_id, state).await
}

#[tauri::command]
pub async fn answer_agent_question(
    task_id: String,
    question_id: String,
    answer: String,
) -> Result<(), String> {
    let mut reg = crate::agent::QUESTION_REGISTRY.lock().await;
    if let Some(sender) = reg.remove(&question_id) {
        sender
            .send(answer)
            .map_err(|_| "Failed to send answer: agent question channel was dropped".to_string())?;
        Ok(())
    } else {
        Err(format!(
            "Question {} for task {} not found in registry (already answered or expired)",
            question_id, task_id
        ))
    }
}

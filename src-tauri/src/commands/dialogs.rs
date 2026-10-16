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
pub async fn save_task_dialog(
    task_id: String,
    items: Vec<ActivityItem>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let safe_id = sanitize_task_id(&task_id)?;
    let root = state.get_root();
    crate::db::append_dialog_items(&root, &safe_id, &items)?;
    eprintln!("CVH [save_task_dialog]: saved {} items to SQLite for task {}", items.len(), safe_id);
    Ok(())
}

pub async fn append_task_dialog_activity(
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
        let _ = app.emit("task:dialog", serde_json::json!({
            "taskId": task_id,
            "item": new_item,
        }));
    }

    Ok(())
}

#[tauri::command]
pub async fn load_task_dialog(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ActivityItem>, String> {
    let safe_id = sanitize_task_id(&task_id)?;
    let root = state.get_root();
    crate::db::load_dialog_items(&root, &safe_id)
}

#[tauri::command]
pub async fn load_subagent_dialog(
    parent_task_id: String,
    subagent_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ActivityItem>, String> {
    let sub_key = format!("{}:sub:{}", parent_task_id.trim(), subagent_id.trim());
    let safe_id = sanitize_task_id(&sub_key)?;
    let root = state.get_root();
    crate::db::load_dialog_items(&root, &safe_id)
}

#[tauri::command]
pub async fn retry_subagent(
    parent_task_id: String,
    subagent_id: String,
    message: Option<String>,
) -> Result<(), String> {
    let _ = parent_task_id;
    crate::agent::retry_subagent_execution(&subagent_id, message)
        .await
        .map_err(|e| e.to_string())
}

/// Receives renderer-side JS errors so white-screen crashes land in the backend log.
#[tauri::command]
pub async fn log_ui_error(message: String) -> Result<(), String> {
    let trimmed = message.chars().take(4000).collect::<String>();
    eprintln!("CVH [UI-ERROR]: {}", trimmed);
    Ok(())
}

#[tauri::command]
pub async fn clear_task_dialog(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let safe_id = sanitize_task_id(&task_id)?;
    let root = state.get_root();
    crate::db::clear_dialog_items(&root, &safe_id)
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

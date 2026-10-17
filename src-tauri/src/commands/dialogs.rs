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
    let root = state.get_root();
    let p_trim = parent_task_id.trim();
    let s_trim = subagent_id.trim();

    // 1. If subagent_id already contains ":sub:", use it directly.
    if s_trim.contains(":sub:") {
        if let Ok(safe_id) = sanitize_task_id(s_trim) {
            let items = crate::db::load_dialog_items(&root, &safe_id)?;
            if !items.is_empty() {
                return Ok(items);
            }
        }
    }

    // 2. Direct format: "{}:sub:{}"
    let direct_key = format!("{}:sub:{}", p_trim, s_trim);
    if let Ok(safe_id) = sanitize_task_id(&direct_key) {
        let items = crate::db::load_dialog_items(&root, &safe_id)?;
        if !items.is_empty() {
            return Ok(items);
        }
    }

    // 3. Hierarchical format: "{}:sub:{}.sub-{}" if s_trim does not already contain ".sub-"
    if !s_trim.contains(".sub-") && !p_trim.is_empty() {
        let nested_key = format!("{}:sub:{}.sub-{}", p_trim, p_trim, s_trim);
        if let Ok(safe_id) = sanitize_task_id(&nested_key) {
            let items = crate::db::load_dialog_items(&root, &safe_id)?;
            if !items.is_empty() {
                return Ok(items);
            }
        }
    }

    // 4. Check if subagent_id is a tool call ID (e.g. "call_...") or ends with tool call ID,
    // and search SQLite for keys ending with ".sub-{tool_call_id}".
    let tool_call_id = if let Some(pos) = s_trim.rfind(".sub-") {
        &s_trim[(pos + 5)..]
    } else {
        s_trim
    };

    if !tool_call_id.is_empty() {
        if let Ok(Some(matched_key)) = crate::db::find_subagent_dialog_key(&root, Some(p_trim), tool_call_id) {
            if let Ok(safe_id) = sanitize_task_id(&matched_key) {
                let items = crate::db::load_dialog_items(&root, &safe_id)?;
                if !items.is_empty() {
                    return Ok(items);
                }
            }
        }
    }

    // 5. Fallback: return result of direct key
    let fallback_key = if s_trim.contains(":sub:") {
        s_trim.to_string()
    } else {
        format!("{}:sub:{}", p_trim, s_trim)
    };
    let safe_id = sanitize_task_id(&fallback_key)?;
    crate::db::load_dialog_items(&root, &safe_id)
}

#[tauri::command]
pub async fn retry_subagent(
    parent_task_id: String,
    subagent_id: String,
    message: Option<String>,
) -> Result<(), String> {
    crate::agent::retry_subagent_execution(Some(&parent_task_id), &subagent_id, message)
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

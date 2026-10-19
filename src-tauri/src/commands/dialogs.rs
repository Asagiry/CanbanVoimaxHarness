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
    // Allow ':' strictly when part of the subagent delimiter ":sub:" or copilot namespace "copilot:"
    let without_sub = trimmed.replace(":sub:", "").replace("copilot:", "");
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentSummary {
    pub id: String,
    pub parent_task_id: String,
    pub tool_call_id: Option<String>,
    pub title: String,
    pub role: String,
    pub prompt: String,
    pub status: String,
    pub created_at: i64,
    pub duration_ms: Option<f64>,
    pub steps_count: usize,
    pub last_error: Option<String>,
    #[serde(default)]
    pub is_bg: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentDialogKeyInfo {
    pub key: String,
    pub is_bg: bool,
}

#[tauri::command]
pub async fn list_subagents(
    parent_task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<SubagentSummary>, String> {
    let root = state.get_root();
    let p_trim = parent_task_id.trim();
    if p_trim.is_empty() {
        return Ok(Vec::new());
    }

    let mut result_map: std::collections::HashMap<String, SubagentSummary> = std::collections::HashMap::new();

    // 1. In-memory sessions from SUBAGENT_SESSIONS
    {
        let sessions = crate::agent::SUBAGENT_SESSIONS.lock().await;
        for (sub_id, session) in sessions.iter() {
            let info = session.retry_info.lock().await;
            if info.parent_task_id == p_trim {
                let summary = SubagentSummary {
                    id: sub_id.clone(),
                    parent_task_id: p_trim.to_string(),
                    tool_call_id: info.tool_call_id.clone(),
                    title: info.title.clone(),
                    role: info.role.clone(),
                    prompt: info.prompt.clone(),
                    status: info.status.clone(),
                    created_at: chrono::Utc::now().timestamp_millis(),
                    duration_ms: info.duration_ms,
                    steps_count: 0,
                    last_error: info.last_error.clone(),
                    is_bg: info.is_bg,
                };
                result_map.insert(sub_id.clone(), summary);
            }
        }
    }

    // 2. Query parent task's dialog items to extract tool metadata (subagent/subagent_bg tool calls)
    struct ParentSubagentTool {
        call_id: Option<String>,
        title: String,
        role: String,
        prompt: String,
        duration: Option<f64>,
        status: String,
        is_bg: bool,
    }

    let mut parent_subagent_tools: Vec<ParentSubagentTool> = Vec::new();
    if let Ok(safe_pid) = sanitize_task_id(p_trim) {
        if let Ok(parent_items) = crate::db::load_dialog_items(&root, &safe_pid) {
            for it in parent_items {
                if it.item_type == "tool" {
                    if let Some(ref t) = it.tool {
                        let t_name = t.get("toolName").and_then(|v| v.as_str()).unwrap_or("");
                        if t_name == "subagent" || t_name == "subagent_bg" {
                            let params = t.get("params");
                            let title = params.and_then(|p| p.get("title")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let role = params.and_then(|p| p.get("role")).and_then(|v| v.as_str()).unwrap_or("general").to_string();
                            let prompt = params.and_then(|p| p.get("prompt")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let call_id = t.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
                            let duration = t.get("durationMs").and_then(|v| v.as_f64());
                            let status = t.get("status").and_then(|v| v.as_str()).unwrap_or("running").to_string();
                            let is_bg = t_name == "subagent_bg";
                            parent_subagent_tools.push(ParentSubagentTool {
                                call_id,
                                title,
                                role,
                                prompt,
                                duration,
                                status,
                                is_bg,
                            });
                        }
                    }
                }
            }
        }
    }

    // 3. Query SQLite for persisted subagent dialogues
    if let Ok(keys) = crate::db::list_subagent_dialog_keys(&root, p_trim) {
        for key in keys {
            let raw_sub_id = key.strip_prefix(&format!("{}:sub:", p_trim)).unwrap_or(&key).to_string();
            let items = crate::db::load_dialog_items(&root, &key).unwrap_or_default();
            let steps_count = items.len();

            let mut title = String::new();
            let mut prompt = String::new();
            let mut role = "general".to_string();
            let mut status = "finished".to_string();
            let mut duration_ms = 0.0f64;
            let mut last_error: Option<String> = None;

            for it in &items {
                if it.item_type == "prompt" {
                    if let Some(ref c) = it.content {
                        prompt = c.clone();
                    }
                }
                if it.item_type == "error" {
                    status = "error".to_string();
                    last_error = it.content.clone();
                }
                if let Some(dur) = it.duration_ms {
                    duration_ms += dur;
                }
            }

            let tool_call_id = raw_sub_id.rfind(".sub-").map(|pos| raw_sub_id[(pos + 5)..].to_string());

            let mut is_bg = false;
            // Correlate with parent_subagent_tools if possible
            if let Some(pt) = parent_subagent_tools.iter().find(|p| {
                (p.call_id.is_some() && tool_call_id.is_some() && p.call_id == tool_call_id)
                    || (!prompt.is_empty() && p.prompt == prompt)
            }) {
                is_bg = pt.is_bg;
                if !pt.title.is_empty() {
                    title = pt.title.clone();
                }
                if !pt.role.is_empty() {
                    role = pt.role.clone();
                }
                if prompt.is_empty() && !pt.prompt.is_empty() {
                    prompt = pt.prompt.clone();
                }
                if duration_ms == 0.0 {
                    if let Some(d) = pt.duration {
                        duration_ms = d;
                    }
                }
                if status != "error" {
                    if pt.status == "running" {
                        status = "running".to_string();
                    } else if pt.status == "failed" {
                        status = "error".to_string();
                    }
                }
            } else {
                let sessions = crate::agent::SUBAGENT_SESSIONS.lock().await;
                for (sid, s) in sessions.iter() {
                    if sid == &raw_sub_id || sid.ends_with(&raw_sub_id) {
                        let info = s.retry_info.lock().await;
                        is_bg = info.is_bg;
                        break;
                    }
                }
            }

            if title.is_empty() && !prompt.is_empty() {
                let first_line = prompt.lines().next().unwrap_or("").trim();
                let clean = first_line.trim_start_matches('#').trim();
                if !clean.is_empty() {
                    title = clean.chars().take(80).collect();
                }
            }
            if title.is_empty() {
                title = format!("Субагент {}", raw_sub_id);
            }

            if let Some(existing) = result_map.get_mut(&raw_sub_id) {
                existing.steps_count = steps_count;
                if existing.prompt.is_empty() {
                    existing.prompt = prompt;
                }
                if duration_ms > 0.0 {
                    existing.duration_ms = Some(duration_ms);
                }
                if is_bg {
                    existing.is_bg = true;
                }
            } else {
                let summary = SubagentSummary {
                    id: raw_sub_id.clone(),
                    parent_task_id: p_trim.to_string(),
                    tool_call_id,
                    title,
                    role,
                    prompt,
                    status,
                    created_at: chrono::Utc::now().timestamp_millis(),
                    duration_ms: if duration_ms > 0.0 { Some(duration_ms) } else { None },
                    steps_count,
                    last_error,
                    is_bg,
                };
                result_map.insert(raw_sub_id, summary);
            }
        }
    }

    let mut list: Vec<SubagentSummary> = result_map.into_values().collect();
    list.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(list)
}

#[tauri::command]
pub async fn list_subagent_dialog_keys(
    parent_task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<SubagentDialogKeyInfo>, String> {
    let root = state.get_root();
    let p_trim = parent_task_id.trim();
    if p_trim.is_empty() {
        return Ok(Vec::new());
    }
    let keys = crate::db::list_subagent_dialog_keys(&root, p_trim)?;
    let sessions = crate::agent::SUBAGENT_SESSIONS.lock().await;

    let mut result = Vec::new();
    for key in keys {
        let raw_sub_id = key.strip_prefix(&format!("{}:sub:", p_trim)).unwrap_or(&key).to_string();
        let mut is_bg = false;
        for (sid, sess) in sessions.iter() {
            if sid == &raw_sub_id || sid.ends_with(&raw_sub_id) {
                let info = sess.retry_info.lock().await;
                is_bg = info.is_bg;
                break;
            }
        }
        result.push(SubagentDialogKeyInfo { key, is_bg });
    }
    Ok(result)
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

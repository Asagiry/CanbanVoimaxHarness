use std::path::Path;
use chrono::Utc;
use tauri::{AppHandle, Emitter, State};

use crate::commands::state::AppState;
use crate::commands::healing::task_id_matches;
use crate::commands::tasks::storage::{load_tasks_from_disk, save_tasks_to_disk};

pub use crate::agent::session_registry::is_matching_subagent_key;

/// Agent & Child Subagent Abort:
pub async fn internal_abort_agent(
    root: &Path,
    task_id: &str,
    state: &AppState,
    app_handle: Option<&AppHandle>,
) -> Result<bool, String> {
    let p_trim = task_id.trim();
    if p_trim.is_empty() {
        return Ok(false);
    }

    state.remove_agent_context(root, p_trim);

    // 1. Abort subagent and parent sessions hierarchically in session_registry
    let outcome = crate::agent::session_registry::abort_hierarchical_sessions(p_trim, state).await?;

    // 2. Emit session status updates for any aborted sessions
    if let Some(app) = app_handle {
        for sid in &outcome.aborted_session_ids {
            if let Some(summary) = state.session_registry.get_session(sid) {
                let _ = app.emit("agent:session_status", &summary);
            }
        }
    }

    // 3. Mark mid-flight running tools as failed in SQLite & emit tool_result
    if let Ok(aborted_tools) = crate::db::mark_running_tools_as_aborted(root, p_trim, outcome.is_subagent) {
        if let Some(app) = app_handle {
            for (_item_id, item_task_id, tool_id, tool_name) in aborted_tools {
                let _ = app.emit(
                    "agent:tool_result",
                    serde_json::json!({
                        "taskId": item_task_id,
                        "id": tool_id,
                        "name": tool_name,
                        "result": "Прервано пользователем",
                        "success": false,
                    }),
                );
            }
        }
    }

    // 4. Update session statuses to Finished in session_registry
    for sid in &outcome.aborted_session_ids {
        state.session_registry.update_status(sid, crate::agent::session_registry::SessionStatus::Finished);
        if let Some(app) = app_handle {
            if let Some(summary) = state.session_registry.get_session(sid) {
                let _ = app.emit("agent:session_status", &summary);
            }
        }
    }

    // 5. Handle UI and task lifecycle: subagent vs parent task
    if outcome.is_subagent {
        if let Some(app) = app_handle {
            let _ = app.emit(
                "subagent:status",
                serde_json::json!({ "subagentId": p_trim, "status": "aborted" }),
            );
            let _ = app.emit(
                "subagent:aborted",
                serde_json::json!({ "subagentId": p_trim }),
            );
        }
        return Ok(outcome.had_entry);
    }

    // Parent task: update task status on disk & memory to "idle"
    let mut tasks = load_tasks_from_disk(root).await;
    {
        let guard = state.tasks.lock().unwrap_or_else(|p| p.into_inner());
        for mem_t in guard.iter() {
            if let Some(disk_t) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &mem_t.id)) {
                if mem_t.updated_at > disk_t.updated_at {
                    *disk_t = mem_t.clone();
                }
            } else {
                tasks.push(mem_t.clone());
            }
        }
    }
    let mut task_updated = false;
    let subtask_parent_id = tasks.iter().find(|t| task_id_matches(&t.id, p_trim)).and_then(|t| t.parent_id.clone());
    if let Some(parent) = tasks.iter_mut().find(|t| {
        t.waiting_for_subtask_id.as_deref().map(|s| s.eq_ignore_ascii_case(p_trim)).unwrap_or(false)
            || subtask_parent_id.as_deref().map(|pid| t.id.eq_ignore_ascii_case(pid) && t.waiting_for_subtask_id.is_some()).unwrap_or(false)
    }) {
        parent.waiting_for_subtask_id = None;
        if parent.status.as_deref() == Some("waiting_subtask") {
            parent.status = Some("idle".to_string());
        }
        parent.updated_at = Utc::now().timestamp_millis();
        let _ = crate::db::db_upsert_task(root, parent);
        task_updated = true;
    }
    if let Some(t) = tasks.iter_mut().find(|t| task_id_matches(&t.id, p_trim)) {
        if t.status.as_deref() != Some("idle") {
            t.status = Some("idle".to_string());
            t.updated_at = Utc::now().timestamp_millis();
            let _ = crate::db::db_upsert_task(root, t);
            task_updated = true;
        }
    }

    if task_updated {
        {
            let mut guard = state.tasks.lock().unwrap_or_else(|p| p.into_inner());
            *guard = tasks.clone();
        }
        let _ = save_tasks_to_disk(root, &tasks).await;
        if let Some(app) = app_handle {
            let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
        }
    }

    if let Some(app) = app_handle {
        let _ = app.emit(
            "agent:status",
            serde_json::json!({ "taskId": p_trim, "status": "idle" }),
        );
        let _ = app.emit(
            "subagent:aborted",
            serde_json::json!({ "parentTaskId": p_trim }),
        );
    }

    Ok(outcome.had_entry || task_updated)
}

pub async fn abort_agent(
    task_id: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let root = state.get_root();
    let p_trim = task_id.trim();
    state.remove_agent_context(&root, p_trim);
    let is_sub = crate::agent::session_registry::is_subagent_task_id(p_trim, Some(&state.active_agents)).await;
    let _ = internal_abort_agent(&root, &task_id, &state, Some(&app_handle)).await?;
    if !is_sub {
        let _ = app_handle.emit(
            "agent:status",
            serde_json::json!({ "taskId": &task_id, "status": "idle" }),
        );
    }
    Ok(())
}

pub async fn get_active_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<crate::agent::session_registry::AgentSessionSummary>, String> {
    Ok(state.session_registry.get_active_sessions())
}

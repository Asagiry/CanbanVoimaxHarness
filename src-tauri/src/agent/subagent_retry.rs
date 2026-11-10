use std::path::Path;
use std::sync::Arc;
use anyhow::{Context, Result};
use tauri::{Emitter, Manager};
use tokio::sync::oneshot;
use crate::agent::subagent_runtime::{find_subagent_session, SUBAGENT_RECEIVERS};
use crate::agent::subagent_exec::run_inprocess_subagent;
use crate::db::ActivityItem;

/// Updates an in-process subagent session's model, provider endpoint, or reasoning effort in place.
/// Returns `true` if a matching subagent session was found and updated.
pub async fn update_subagent_session_model_or_effort(
    repo_root: &Path,
    subagent_id: &str,
    model: Option<&str>,
    provider_id: Option<&str>,
    reasoning_effort: Option<&str>,
) -> bool {
    let parent_hint = subagent_id
        .split(".sub-")
        .next()
        .filter(|p| *p != subagent_id);
    let found = match find_subagent_session(None, subagent_id).await {
        Some(res) => Some(res),
        None => match parent_hint {
            Some(pid) => find_subagent_session(Some(pid), subagent_id).await,
            None => None,
        },
    };
    let Some((_, session)) = found else {
        return false;
    };

    let mut info = session.retry_info.lock().await;
    if let Some(m) = model.map(str::trim).filter(|s| !s.is_empty()) {
        info.ctx.model = Some(m.to_string());
    }
    if let Some(pid) = provider_id.map(str::trim).filter(|s| !s.is_empty()) {
        let settings = crate::config::load_llm_settings(repo_root)
            .await
            .unwrap_or_default();
        let model_id = info
            .ctx
            .model
            .clone()
            .unwrap_or_else(|| settings.model.clone());
        let (base_url, api_key) = settings.endpoint_for_pair(Some(pid), &model_id);
        if !base_url.is_empty() {
            info.ctx.base_url = Some(base_url);
        }
        if api_key.is_some() {
            info.ctx.api_key = api_key;
        }
    }
    if let Some(eff) = reasoning_effort.map(str::trim).filter(|s| !s.is_empty()) {
        info.ctx.reasoning_effort = crate::config::normalize_reasoning_effort(Some(eff));
    }
    true
}

pub async fn retry_subagent_execution(
    parent_task_id: Option<&str>,
    subagent_id: &str,
    additional_prompt: Option<String>,
    model: Option<String>,
    provider_id: Option<String>,
) -> Result<()> {
    let (resolved_id, session) = find_subagent_session(parent_task_id, subagent_id)
        .await
        .context(format!(
            "Subagent session not found in registry (parent: {:?}, id: '{}')",
            parent_task_id, subagent_id
        ))?;

    // Keep existing sender channel so parent agent awaiting rx is NEVER dropped!
    let maybe_new_rx = {
        let mut tx_guard = session.sender.lock().await;
        if tx_guard.is_none() {
            let (new_tx, new_rx) = oneshot::channel::<String>();
            *tx_guard = Some(new_tx);
            Some(new_rx)
        } else {
            None
        }
    };
    if let Some(new_rx) = maybe_new_rx {
        let mut receivers = SUBAGENT_RECEIVERS.lock().await;
        receivers.insert(resolved_id.clone(), new_rx);
    }

    let mut info = session.retry_info.lock().await.clone();

    // Honor a model/provider switch at retry time (e.g. after 429 pause)
    if let Some(m) = model.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        info.ctx.model = Some(m.to_string());
    }
    if let Some(pid) = provider_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let settings = crate::config::load_llm_settings(&info.ctx.repo_root).await.unwrap_or_default();
        let model_id = info.ctx.model.clone().unwrap_or_else(|| settings.model.clone());
        let (base_url, api_key) = settings.endpoint_for_pair(Some(pid), &model_id);
        if !base_url.is_empty() {
            info.ctx.base_url = Some(base_url);
        }
        if api_key.is_some() {
            info.ctx.api_key = api_key;
        }
    }

    if let Some(msg) = additional_prompt {
        let trimmed = msg.trim();
        if !trimmed.is_empty() {
            info.prompt = format!("{}\n\n[USER GUIDANCE]: {}", info.prompt, trimmed);
            let p_id = parent_task_id.unwrap_or(info.parent_task_id.as_str());
            let sub_dialog_id = if resolved_id.contains(":sub:") {
                resolved_id.clone()
            } else if !p_id.is_empty() {
                format!("{}:sub:{}", p_id, resolved_id)
            } else {
                resolved_id.clone()
            };
            let existing_user_item = crate::db::load_dialog_items(&info.ctx.repo_root, &sub_dialog_id)
                .ok()
                .and_then(|items| {
                    items.into_iter().rev().find(|it| {
                        (it.item_type == "user" || it.item_type == "steer")
                            && it.content.as_deref().map(str::trim) == Some(trimmed)
                    })
                });
            let user_item = if let Some(existing) = existing_user_item {
                existing
            } else {
                let item = ActivityItem {
                    id: format!(
                        "sub-user-{}-{}",
                        sub_dialog_id.replace(':', "-"),
                        chrono::Utc::now().timestamp_millis()
                    ),
                    item_type: "user".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    content: Some(trimmed.to_string()),
                    tool: None,
                    duration_ms: None,
                    images: None,
                };
                if let Err(e) = crate::db::append_dialog_items(
                    &info.ctx.repo_root,
                    &sub_dialog_id,
                    std::slice::from_ref(&item),
                ) {
                    eprintln!("CVH [WARN]: Failed to append subagent retry user dialog item: {}", e);
                }
                item
            };
            if let Some(ref app) = info.ctx.app_handle {
                let _ = app.emit(
                    "subagent:activity",
                    serde_json::json!({
                        "parentTaskId": p_id,
                        "subagentId": &resolved_id,
                        "activity": &user_item,
                    }),
                );
            }
        }
    }
    info.status = "running".to_string();
    info.last_error = None;
    info.report = None;

    // Revive the subagent: clear any stale abort so a previous manual Stop does not
    // immediately re-kill the relaunched run (D3).
    session.abort_handle.store(false, std::sync::atomic::Ordering::SeqCst);
    info.ctx.abort_handle.store(false, std::sync::atomic::Ordering::SeqCst);

    {
        let mut session_info = session.retry_info.lock().await;
        *session_info = info.clone();
    }

    let owned_ctx = info.ctx.clone();
    let title_s = info.title.clone();
    let role_s = info.role.clone();
    let prompt_s = info.prompt.clone();
    let requested_tools = info.requested_tools.clone();
    let sub_id = resolved_id.clone();

    // Re-register execution so Stop/Continue and parent waits keep working (D4).
    let join_h = tokio::spawn(async move {
        let _ = run_inprocess_subagent(
            owned_ctx,
            title_s,
            role_s,
            prompt_s,
            requested_tools,
            Some(sub_id),
        ).await;
    });
    *session.join_handle.lock().await = Some(join_h);

    if let Some(ref app) = info.ctx.app_handle {
        if let Some(state_ref) = app.try_state::<crate::commands::AppState>() {
            let root_str = info.ctx.repo_root.to_string_lossy().to_string();
            let sub_summary = crate::agent::session_registry::AgentSessionSummary {
                session_id: resolved_id.clone(),
                task_id: resolved_id.clone(),
                stage_id: None,
                started_at: chrono::Utc::now().timestamp_millis() as u64,
                status: crate::agent::session_registry::SessionStatus::Running,
                project_path: Some(root_str),
            };
            state_ref.session_registry.register(
                sub_summary.clone(),
                Arc::clone(&session.abort_handle),
                Arc::clone(&session.join_handle),
            );
            let _ = app.emit("agent:session_status", &sub_summary);
            state_ref.register_active_agent(
                &info.ctx.repo_root,
                &resolved_id,
                crate::commands::ActiveAgentHandle {
                    abort_flag: Arc::clone(&session.abort_handle),
                    join_handle: Arc::clone(&session.join_handle),
                    steering: None,
                    advance_aborted: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                },
            );
        }
        let _ = app.emit(
            "subagent:status",
            serde_json::json!({
                "parentTaskId": parent_task_id.unwrap_or(&info.parent_task_id),
                "subagentId": &resolved_id,
                "status": "running",
            }),
        );
    }

    Ok(())
}

use anyhow::{Context, Result};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::oneshot;

use crate::agent::{
    are_subagent_prompts_duplicate, prune_subagent_sessions_cache, run_inprocess_subagent,
    SubagentRetryInfo, SubagentSession, ToolContext, SUBAGENT_RECEIVERS, SUBAGENT_SESSIONS,
};

pub async fn execute_subagent_bg(
    ctx: &ToolContext<'_>,
    args: &serde_json::Value,
) -> Result<String> {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .context("Missing required parameter 'title'")?;
    let prompt = args
        .get("prompt")
        .and_then(|v| v.as_str())
        .context("Missing required parameter 'prompt'")?;
    let role = args
        .get("persona")
        .or_else(|| args.get("role"))
        .and_then(|v| v.as_str())
        .unwrap_or("general");
    let requested_tools: Option<Vec<String>> = args.get("tools").and_then(|v| v.as_array()).map(|arr| {
        arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()
    });

    let parent_id = ctx.task_id.unwrap_or("unknown");

    // Deduplication check across sessions: if an identical subagent is already actively running or finished, reuse it.
    {
        let candidates: Vec<(String, Arc<tokio::sync::Mutex<SubagentRetryInfo>>)> = {
            let sessions = SUBAGENT_SESSIONS.lock().await;
            sessions
                .iter()
                .map(|(sid, sess)| (sid.clone(), Arc::clone(&sess.retry_info)))
                .collect()
        };
        for (sid, retry_info_lock) in candidates {
            let (is_dup, status, role_name) = {
                let info = retry_info_lock.lock().await;
                if info.parent_task_id == parent_id
                    && (info.status == "running" || info.status == "finished")
                {
                    (
                        info.role.eq_ignore_ascii_case(role) && are_subagent_prompts_duplicate(&info.prompt, prompt),
                        info.status.clone(),
                        info.role.clone(),
                    )
                } else {
                    (false, String::new(), String::new())
                }
            };

            if is_dup {
                eprintln!("CVH [agent]: Reusing existing subagent {} (role: '{}') for duplicate subagent_bg request", sid, role_name);
                return Ok(serde_json::to_string(&serde_json::json!({
                    "subagent_id": sid,
                    "status": &status,
                    "note": format!("Reused existing {} subagent to prevent duplicate subagent_bg execution", role_name)
                }))?);
            }
        }
    }

    let subagent_suffix = match ctx.call_id {
        Some(cid) if !cid.trim().is_empty() => cid.trim().to_string(),
        _ => format!(
            "{}-{}",
            chrono::Utc::now().timestamp_millis(),
            uuid::Uuid::new_v4().to_string().chars().take(4).collect::<String>()
        ),
    };
    let subagent_id = format!("{}.sub-{}", parent_id, subagent_suffix);
    let tc_id_opt = ctx.call_id.map(|s| s.to_string());

    let sub_abort_handle = Arc::new(AtomicBool::new(false));
    let sub_join_slot = Arc::new(tokio::sync::Mutex::new(None));

    let (tx, rx) = oneshot::channel::<String>();
    let retry_info = SubagentRetryInfo {
        parent_task_id: parent_id.to_string(),
        subagent_id: subagent_id.clone(),
        tool_call_id: tc_id_opt,
        title: title.to_string(),
        role: role.to_string(),
        prompt: prompt.to_string(),
        requested_tools: requested_tools.clone(),
        ctx: ctx.to_owned(),
        status: "running".to_string(),
        created_at: chrono::Utc::now().timestamp_millis(),
        last_error: None,
        report: None,
        is_bg: true,
        duration_ms: None,
        steps: 0,
    };

    let session = SubagentSession {
        sender: Arc::new(tokio::sync::Mutex::new(Some(tx))),
        retry_info: Arc::new(tokio::sync::Mutex::new(retry_info)),
        abort_handle: Arc::clone(&sub_abort_handle),
        join_handle: Arc::clone(&sub_join_slot),
    };

    {
        let mut receivers = SUBAGENT_RECEIVERS.lock().await;
        receivers.insert(subagent_id.clone(), rx);
    }
    {
        let mut sessions = SUBAGENT_SESSIONS.lock().await;
        sessions.insert(subagent_id.clone(), session);
    }
    prune_subagent_sessions_cache().await;

    let mut owned_ctx = ctx.to_owned();
    owned_ctx.abort_handle = Arc::clone(&sub_abort_handle);
    let sub_id = subagent_id.clone();
    let title_s = title.to_string();
    let role_s = role.to_string();
    let prompt_s = prompt.to_string();

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
    *sub_join_slot.lock().await = Some(join_h);

    // Register into state.active_agents and session_registry
    if let Some(ref app) = ctx.app_handle {
        if let Some(state_ref) = app.try_state::<crate::commands::AppState>() {
            let root_str = ctx.repo_root.to_string_lossy().to_string();
            let sub_summary = crate::agent::session_registry::AgentSessionSummary {
                session_id: subagent_id.clone(),
                task_id: subagent_id.clone(),
                stage_id: None,
                started_at: chrono::Utc::now().timestamp_millis() as u64,
                status: crate::agent::session_registry::SessionStatus::Running,
                project_path: Some(root_str),
            };
            state_ref.session_registry.register(
                sub_summary.clone(),
                Arc::clone(&sub_abort_handle),
                Arc::clone(&sub_join_slot),
            );
            let _ = app.emit("agent:session_status", &sub_summary);
            state_ref.register_active_agent(
                ctx.repo_root,
                &subagent_id,
                crate::commands::ActiveAgentHandle {
                    abort_flag: Arc::clone(&sub_abort_handle),
                    join_handle: Arc::clone(&sub_join_slot),
                    steering: None,
                    advance_aborted: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                },
            );
        }
    }

    Ok(serde_json::to_string(&serde_json::json!({
        "subagent_id": subagent_id,
        "status": "running"
    }))?)
}

use std::sync::atomic::Ordering;
use anyhow::{bail, Result};
use tauri::{Emitter, Manager};
use tokio::sync::mpsc;
use crate::agent::compactor;
use crate::agent::react_engine::{self, ReActEngine, StreamConsumerOptions, SubagentDriver};
use crate::agent::subagent_runtime::{find_subagent_session, OwnedToolContext};
use crate::db::ActivityItem;
use crate::llm::{ChatMessage, LlmClient, OpenAIChatRequest, StreamChunk};

pub async fn run_subagent_synthesis(
    ctx: &OwnedToolContext,
    subagent_id: &str,
    parent_id: &str,
    subagent_suffix: &str,
    subagent_dialog_id: &str,
    resolved_model: &str,
    system_tokens: u64,
    temperature: Option<f32>,
    reasoning_effort: Option<String>,
    base_url: &str,
    api_key: Option<String>,
    llm_client: &LlmClient,
    subagent_engine: &ReActEngine<SubagentDriver>,
    sub_messages: &mut Vec<ChatMessage>,
    start_instant: std::time::Instant,
) -> Result<String> {
    sub_messages.push(ChatMessage::user(
        "Предоставь краткий и исчерпывающий финальный отчет о результатах выполненной работы и найденных данных (Provide a concise, comprehensive final report summarizing your findings and work done)."
    ));

    let model = resolved_model;
    let synth_system_tokens = system_tokens;
    let synth_opts = StreamConsumerOptions {
        model,
        turn: 999,
        system_tokens: synth_system_tokens,
        tool_tokens: 0,
        fallback_input_tokens: compactor::estimate_request_tokens(sub_messages, None) as u64,
        log_prefix: "subagent synthesis telemetry",
        subagent_id: Some(subagent_id),
        emit_thoughts: false,
    };

    let mut synth_retries = 0;
    let mut synth_content = String::new();
    let mut synth_error: Option<String> = None;
    loop {
        let (tx, mut rx) = mpsc::channel::<StreamChunk>(100);
        let cl = llm_client.clone();
        let b_url = base_url.to_string();
        let a_key = api_key.clone();
        let req_clone = OpenAIChatRequest {
            model: model.to_string(),
            messages: sub_messages.clone(),
            temperature,
            stream: true,
            tools: None,
            reasoning_effort: reasoning_effort.clone(),
            stream_options: Some(crate::llm::StreamOptions {
                include_usage: true,
            }),
        };
        tokio::spawn(async move {
            if let Err(e) = cl.stream_openai(&b_url, a_key.as_deref(), req_clone, tx.clone()).await {
                let _ = tx.send(StreamChunk::Error(format!("Subagent synthesis stream failed: {}", e))).await;
            }
        });

        let synth_res = subagent_engine.consume_turn_stream(&mut rx, &ctx.abort_handle, &synth_opts).await;
        if let Some(err) = synth_res.error {
            let retryable = react_engine::is_transient_llm_error(&err);
            if retryable && synth_retries < 2 && !ctx.abort_handle.load(Ordering::SeqCst) {
                synth_retries += 1;
                eprintln!(
                    "CVH [agent]: Subagent [{}] synthesis turn got transient stream error (retry {}/2): {}",
                    subagent_id, synth_retries, err
                );
                tokio::time::sleep(std::time::Duration::from_millis(2000 * synth_retries as u64)).await;
                continue;
            }
            synth_error = Some(err);
            break;
        }
        synth_content = synth_res.content;
        break;
    }
    if let Some(err_msg) = synth_error {
        let is_transient = react_engine::is_transient_llm_error(&err_msg);
        if is_transient {
            let duration_ms = start_instant.elapsed().as_millis() as f64;
            if let Some((_, session)) = find_subagent_session(Some(parent_id), subagent_id).await {
                let mut info = session.retry_info.lock().await;
                info.status = "paused".to_string();
                info.last_error = Some(err_msg.clone());
                info.duration_ms = Some(duration_ms);
                drop(session.sender.lock().await.take());
            }
            if let Some(ref app) = ctx.app_handle {
                if let Some(state_ref) = app.try_state::<crate::commands::AppState>() {
                    state_ref.session_registry.update_status(subagent_id, crate::agent::session_registry::SessionStatus::Paused);
                    if let Some(summary) = state_ref.session_registry.get_session(subagent_id) {
                        let _ = app.emit("agent:session_status", &summary);
                    }
                }
                let _ = app.emit(
                    "subagent:status",
                    serde_json::json!({
                        "parentTaskId": parent_id,
                        "subagentId": subagent_id,
                        "status": "paused",
                        "error": &err_msg,
                        "durationMs": duration_ms,
                    }),
                );
            }
            eprintln!(
                "CVH [agent]: Subagent [{}] paused on transient provider error (synthesis, awaiting user Continue/Stop): {}",
                subagent_id, err_msg
            );
            bail!("Subagent [{}] paused on transient provider error: {}", subagent_id, err_msg);
        } else {
            let duration_ms = start_instant.elapsed().as_millis() as f64;
            if let Some((_, session)) = find_subagent_session(Some(parent_id), subagent_id).await {
                let mut info = session.retry_info.lock().await;
                info.status = "error".to_string();
                info.last_error = Some(err_msg.clone());
                info.duration_ms = Some(duration_ms);
                if let Some(sender) = session.sender.lock().await.take() {
                    let _ = sender.send(format!("[Subagent synthesis error: {}]", err_msg));
                }
            }
            if let Some(ref app) = ctx.app_handle {
                if let Some(state_ref) = app.try_state::<crate::commands::AppState>() {
                    state_ref.session_registry.update_status(subagent_id, crate::agent::session_registry::SessionStatus::Finished);
                }
                let _ = app.emit("subagent:error", serde_json::json!({
                    "parentTaskId": parent_id,
                    "subagentId": subagent_id,
                    "error": &err_msg,
                    "durationMs": duration_ms,
                }));
            }
            bail!("Subagent [{}] synthesis error: {}", subagent_id, err_msg);
        }
    }

    let mut final_sub_response = synth_content.trim().to_string();
    if !final_sub_response.is_empty() {
        let out_item = ActivityItem {
            id: format!("sub-out-{}", subagent_suffix),
            item_type: "output".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            content: Some(final_sub_response.clone()),
            tool: None,
            duration_ms: None,
            images: None,
        };
        if let Err(e) = crate::db::append_dialog_items(&ctx.repo_root, subagent_dialog_id, std::slice::from_ref(&out_item)) {
            eprintln!("CVH [WARN]: Failed to append subagent final response dialog item: {}", e);
        }
        if let Some(ref app) = ctx.app_handle {
            let _ = app.emit("subagent:activity", serde_json::json!({
                "parentTaskId": parent_id,
                "subagentId": subagent_id,
                "activity": &out_item,
            }));
        }
    } else {
        // Fallback: if still empty, collect last tool results and assistant messages
        let mut collected = Vec::new();
        for msg in sub_messages.iter().rev() {
            if msg.role == crate::llm::Role::Tool || msg.role == crate::llm::Role::Assistant {
                if let Some(ref c) = msg.content {
                    let trimmed = c.trim();
                    if !trimmed.is_empty() {
                        collected.push(trimmed.to_string());
                        if collected.len() >= 3 {
                            break;
                        }
                    }
                }
            }
        }
        if !collected.is_empty() {
            collected.reverse();
            final_sub_response = collected.join("\n\n");
            let out_item = ActivityItem {
                id: format!("sub-out-{}", subagent_suffix),
                item_type: "output".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                content: Some(final_sub_response.clone()),
                tool: None,
                duration_ms: None,
                images: None,
            };
            if let Err(e) = crate::db::append_dialog_items(&ctx.repo_root, subagent_dialog_id, std::slice::from_ref(&out_item)) {
                eprintln!("CVH [WARN]: Failed to append subagent fallback response dialog item: {}", e);
            }
            if let Some(ref app) = ctx.app_handle {
                let _ = app.emit("subagent:activity", serde_json::json!({
                    "parentTaskId": parent_id,
                    "subagentId": subagent_id,
                    "activity": &out_item,
                }));
            }
        }
    }

    Ok(final_sub_response)
}

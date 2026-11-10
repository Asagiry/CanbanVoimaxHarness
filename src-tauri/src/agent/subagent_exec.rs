use std::sync::atomic::Ordering;
use anyhow::{bail, Result};
use tauri::{Emitter, Manager};
use tokio::sync::mpsc;
use crate::agent::compactor;
use crate::agent::react_engine::{self, maybe_compact_messages, ReActEngine, StreamConsumerOptions, SubagentDriver, SubagentDriverConfig};
use crate::agent::subagent_runtime::{
    find_subagent_session, prune_subagent_sessions_cache, OwnedToolContext,
};
use crate::agent::subagent_setup::prepare_subagent_execution;
use crate::agent::subagent_synth::run_subagent_synthesis;
use crate::agent::subagent_turn::execute_subagent_turn_tools;
use crate::agent::telemetry;
use crate::db::ActivityItem;
use crate::llm::{ChatMessage, LlmClient, OpenAIChatRequest, StreamChunk};

/// In-Process Subagent Execution Engine (§ 3 Architectural Invariants)
/// ZERO WORKTREES, ZERO HOOKS: Runs strictly in-process inside parent's active worktree_dir.
/// ZERO BOARD MOVEMENT: Subagents do not modify cards on the kanban board or move stages.
pub fn run_inprocess_subagent(
    ctx: OwnedToolContext,
    title: String,
    role: String,
    prompt: String,
    requested_tools: Option<Vec<String>>,
    explicit_subagent_id: Option<String>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>> {
    Box::pin(async move {
        let start_instant = std::time::Instant::now();
        let prep = prepare_subagent_execution(
            &ctx,
            &title,
            &role,
            &prompt,
            requested_tools.as_deref(),
            explicit_subagent_id.as_deref(),
        )
        .await;

        let parent_id = prep.parent_id;
        let subagent_suffix = prep.subagent_suffix;
        let subagent_id = prep.subagent_id;
        let subagent_dialog_id = prep.subagent_dialog_id;
        let tool_call_id = prep.tool_call_id;
        let effective_sub_tools = prep.effective_sub_tools;
        let active_sub_tool_defs = prep.active_sub_tool_defs;
        let subagent_setting = prep.subagent_setting;
        let mut sub_messages = prep.sub_messages;
        let mut resolved_model = prep.resolved_model;
        if resolved_model.trim().is_empty() {
            return Err(anyhow::anyhow!("Модель не выбрана: укажите модель в настройках задачи, персоны или Settings → Providers"));
        }
        let mut base_url = prep.base_url;
        let mut api_key = prep.api_key;
        let mut reasoning_effort = prep.reasoning_effort;
        let system_tokens = prep.system_tokens;
        let temperature = ctx.temperature.or(Some(0.2));

        let llm_client = LlmClient::for_conversation(&ctx.repo_root, &subagent_id);
        let mut sub_turn = 0;
        let mut final_sub_response = String::new();
        let mut sub_error: Option<String> = None;

        let subagent_driver = SubagentDriver::new(SubagentDriverConfig {
            parent_task_id: parent_id.clone(),
            subagent_id: subagent_id.clone(),
            subagent_dialog_id: subagent_dialog_id.clone(),
            tool_call_id: tool_call_id.clone(),
            repo_root: ctx.repo_root.to_path_buf(),
            worktree_dir: ctx.worktree_dir.to_path_buf(),
            model: resolved_model.clone(),
            app_handle: ctx.app_handle.clone(),
        });
        let subagent_engine = ReActEngine::new(subagent_driver);
        let mut stream_retries = 0;
        let mut recent_tool_calls: Vec<(String, String)> = Vec::new();
        let sub_context_window = subagent_setting.context_window.unwrap_or(128_000) as usize;

        loop {
            sub_turn += 1;
            if let Some((_, session)) = find_subagent_session(Some(&parent_id), &subagent_id).await {
                let mut info = session.retry_info.lock().await;
                info.steps = sub_turn;
                if let Some(ref m) = info.ctx.model {
                    if !m.trim().is_empty() {
                        resolved_model = m.trim().to_string();
                    }
                }
                if let Some(ref b) = info.ctx.base_url {
                    if !b.trim().is_empty() {
                        base_url = b.trim().to_string();
                    }
                }
                if info.ctx.api_key.is_some() {
                    api_key = info.ctx.api_key.clone();
                }
                if info.ctx.reasoning_effort.is_some() {
                    reasoning_effort = info.ctx.reasoning_effort.clone();
                }
            }
            let model = resolved_model.as_str();
            if ctx.abort_handle.load(Ordering::SeqCst) {
                break;
            }

            maybe_compact_messages(&mut sub_messages, sub_context_window);

            let req_tools = if active_sub_tool_defs.is_empty() {
                None
            } else {
                Some(active_sub_tool_defs.clone())
            };
            let tool_tokens = compactor::estimate_tools_tokens(&req_tools) as u64;

            let req = OpenAIChatRequest {
                model: model.to_string(),
                messages: sub_messages.clone(),
                temperature,
                stream: true,
                tools: req_tools.clone(),
                reasoning_effort: reasoning_effort.clone(),
                stream_options: Some(crate::llm::StreamOptions {
                    include_usage: true,
                }),
            };

            telemetry::log_turn_request(
                &ctx.repo_root,
                &subagent_id,
                sub_turn,
                true,
                model,
                &sub_messages,
                req_tools.as_deref(),
                &effective_sub_tools,
                None,
            )
            .await;

            let (tx, mut rx) = mpsc::channel::<StreamChunk>(100);
            let cl = llm_client.clone();
            let b_url = base_url.clone();
            let a_key = api_key.clone();

            tokio::spawn(async move {
                if let Err(e) = cl.stream_openai(&b_url, a_key.as_deref(), req, tx.clone()).await {
                    let _ = tx.send(StreamChunk::Error(format!("Subagent stream failed: {}", e))).await;
                }
            });

            let turn_thought_id = format!("sub-th-{}-turn-{}", subagent_suffix, sub_turn);
            subagent_engine.driver.reset_turn_thoughts(turn_thought_id);

            let stream_opts = StreamConsumerOptions {
                model,
                turn: sub_turn,
                system_tokens,
                tool_tokens,
                fallback_input_tokens: compactor::estimate_request_tokens(&sub_messages, req_tools.as_deref()) as u64,
                log_prefix: "subagent telemetry",
                subagent_id: Some(&subagent_id),
                emit_thoughts: true,
            };

            let stream_res = subagent_engine.consume_turn_stream(&mut rx, &ctx.abort_handle, &stream_opts).await;

            subagent_engine.driver.finalize_turn_thoughts();

            if let Some(err) = stream_res.error {
                let retryable = react_engine::is_transient_llm_error(&err);
                if retryable && stream_retries < 2 && !ctx.abort_handle.load(Ordering::SeqCst) {
                    stream_retries += 1;
                    eprintln!(
                        "CVH [agent]: Subagent [{}] turn {} got transient stream error (retry {}/2): {}",
                        subagent_id, sub_turn, stream_retries, err
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(2000 * stream_retries as u64)).await;
                    continue;
                }
                sub_error = Some(err);
                break;
            }

            if stream_res.aborted || ctx.abort_handle.load(Ordering::SeqCst) {
                break;
            }

            let turn_content = stream_res.content;
            let turn_thoughts = stream_res.thoughts;
            let turn_tool_calls = stream_res.tool_calls;

            telemetry::log_turn_response(
                &ctx.repo_root,
                &subagent_id,
                sub_turn,
                true,
                &turn_thoughts,
                &turn_content,
                &turn_tool_calls,
            )
            .await;

            if turn_tool_calls.is_empty() {
                final_sub_response = turn_content.clone();
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
                    if let Err(e) = crate::db::append_dialog_items(&ctx.repo_root, &subagent_dialog_id, std::slice::from_ref(&out_item)) {
                        eprintln!("CVH [WARN]: Failed to append subagent out dialog item: {}", e);
                    }
                    if let Some(ref app) = ctx.app_handle {
                        let _ = app.emit("subagent:activity", serde_json::json!({
                            "parentTaskId": parent_id,
                            "subagentId": &subagent_id,
                            "activity": &out_item,
                        }));
                    }
                }
                break;
            }

            sub_messages.push(ChatMessage::assistant(
                if turn_content.is_empty() { None } else { Some(turn_content.clone()) },
                Some(turn_tool_calls.clone()),
            ));

            if !turn_content.is_empty() {
                let out_item = ActivityItem {
                    id: format!("sub-msg-{}-{}", subagent_suffix, sub_turn),
                    item_type: "output".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    content: Some(turn_content),
                    tool: None,
                    duration_ms: None,
                    images: None,
                };
                if let Err(e) = crate::db::append_dialog_items(&ctx.repo_root, &subagent_dialog_id, std::slice::from_ref(&out_item)) {
                    eprintln!("CVH [WARN]: Failed to append subagent turn dialog item: {}", e);
                }
                if let Some(ref app) = ctx.app_handle {
                    let _ = app.emit("subagent:activity", serde_json::json!({
                        "parentTaskId": parent_id,
                        "subagentId": &subagent_id,
                        "activity": &out_item,
                    }));
                }
            }

            // Execute subagent tool calls via subagent_turn
            execute_subagent_turn_tools(
                &ctx,
                &role,
                &effective_sub_tools,
                &subagent_id,
                &parent_id,
                &subagent_dialog_id,
                sub_turn,
                &turn_tool_calls,
                &mut sub_messages,
                &mut recent_tool_calls,
                &base_url,
                api_key.as_deref(),
                model,
                reasoning_effort.as_deref(),
            ).await;
        }

        // Terminal error handling
        if let Some(err_msg) = sub_error {
            let duration_ms = start_instant.elapsed().as_millis() as f64;
            let err_item = ActivityItem {
                id: format!("sub-err-{}", uuid::Uuid::new_v4()),
                item_type: "error".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                content: Some(err_msg.clone()),
                tool: None,
                duration_ms: Some(duration_ms),
                images: None,
            };
            if let Err(e) = crate::db::append_dialog_items(&ctx.repo_root, &subagent_dialog_id, std::slice::from_ref(&err_item)) {
                eprintln!("CVH [WARN]: Failed to append subagent error dialog item: {}", e);
            }
            if let Some(ref app) = ctx.app_handle {
                let _ = app.emit("subagent:activity", serde_json::json!({
                    "parentTaskId": &parent_id,
                    "subagentId": &subagent_id,
                    "activity": &err_item,
                }));
            }

            let is_transient = react_engine::is_transient_llm_error(&err_msg);
            if is_transient {
                if let Some((_, session)) = find_subagent_session(Some(&parent_id), &subagent_id).await {
                    let mut info = session.retry_info.lock().await;
                    info.status = "paused".to_string();
                    info.last_error = Some(err_msg.clone());
                    info.duration_ms = Some(duration_ms);
                }

                if let Some(ref app) = ctx.app_handle {
                    if let Some(state_ref) = app.try_state::<crate::commands::AppState>() {
                        state_ref.session_registry.update_status(&subagent_id, crate::agent::session_registry::SessionStatus::Paused);
                        if let Some(summary) = state_ref.session_registry.get_session(&subagent_id) {
                            let _ = app.emit("agent:session_status", &summary);
                        }
                    }
                    let _ = app.emit(
                        "subagent:status",
                        serde_json::json!({
                            "parentTaskId": &parent_id,
                            "subagentId": &subagent_id,
                            "status": "paused",
                            "error": &err_msg,
                            "durationMs": duration_ms,
                        }),
                    );
                }

                eprintln!(
                    "CVH [agent]: Subagent [{}] paused on transient provider error (awaiting user Continue/Stop): {}",
                    subagent_id, err_msg
                );
                bail!("Subagent [{}] paused on transient provider error: {}", subagent_id, err_msg);
            }

            if let Some(ref app) = ctx.app_handle {
                let _ = app.emit("subagent:error", serde_json::json!({
                    "parentTaskId": &parent_id,
                    "subagentId": &subagent_id,
                    "error": &err_msg,
                    "durationMs": duration_ms,
                }));
            }

            if let Some((_, session)) = find_subagent_session(Some(&parent_id), &subagent_id).await {
                let mut info = session.retry_info.lock().await;
                info.status = "error".to_string();
                info.last_error = Some(err_msg.clone());
                info.duration_ms = Some(duration_ms);
                if let Some(sender) = session.sender.lock().await.take() {
                    let _ = sender.send(format!("[Subagent error: {}]", err_msg));
                }
            }

            if let Some(ref app) = ctx.app_handle {
                if let Some(state_ref) = app.try_state::<crate::commands::AppState>() {
                    state_ref.remove_active_agent(&ctx.repo_root, &subagent_id);
                    state_ref.session_registry.update_status(&subagent_id, crate::agent::session_registry::SessionStatus::Finished);
                    if let Some(summary) = state_ref.session_registry.get_session(&subagent_id) {
                        let _ = app.emit("agent:session_status", &summary);
                    }
                }
            }

            bail!("Subagent [{}] error: {}", subagent_id, err_msg);
        }

        // Synthesis turn if final response is empty
        if final_sub_response.trim().is_empty() && !ctx.abort_handle.load(Ordering::SeqCst) {
            final_sub_response = run_subagent_synthesis(
                &ctx,
                &subagent_id,
                &parent_id,
                &subagent_suffix,
                &subagent_dialog_id,
                &resolved_model,
                system_tokens,
                temperature,
                reasoning_effort,
                &base_url,
                api_key,
                &llm_client,
                &subagent_engine,
                &mut sub_messages,
                start_instant,
            ).await?;
        }

        let report = format!(
            "[Subagent ID: {}]\nSubagent [{}] '{}' report:\n\n{}",
            subagent_id,
            role,
            title,
            if final_sub_response.trim().is_empty() {
                "Subagent completed work."
            } else {
                final_sub_response.trim()
            }
        );

        if ctx.abort_handle.load(Ordering::SeqCst) {
            let duration_ms = start_instant.elapsed().as_millis() as f64;
            if let Some((_, session)) = find_subagent_session(Some(&parent_id), &subagent_id).await {
                let mut info = session.retry_info.lock().await;
                info.status = "aborted".to_string();
                info.last_error = Some("Прервано пользователем".to_string());
                info.duration_ms = Some(duration_ms);
                if let Some(sender) = session.sender.lock().await.take() {
                    let _ = sender.send("[Субагент прерван пользователем / Subagent aborted by user]".to_string());
                }
            }
            if let Some(ref app) = ctx.app_handle {
                if let Some(state_ref) = app.try_state::<crate::commands::AppState>() {
                    state_ref.remove_active_agent(&ctx.repo_root, &subagent_id);
                    state_ref.session_registry.update_status(&subagent_id, crate::agent::session_registry::SessionStatus::Finished);
                    if let Some(summary) = state_ref.session_registry.get_session(&subagent_id) {
                        let _ = app.emit("agent:session_status", &summary);
                    }
                }
                let _ = app.emit(
                    "subagent:aborted",
                    serde_json::json!({
                        "parentTaskId": &parent_id,
                        "subagentId": &subagent_id,
                        "durationMs": duration_ms,
                    }),
                );
            }
            bail!("Subagent [{}] was aborted by user", subagent_id);
        }

        let duration_ms = start_instant.elapsed().as_millis() as f64;

        // Subagent completed cleanly: deliver report to waiting parent channel!
        if let Some((_, session)) = find_subagent_session(Some(&parent_id), &subagent_id).await {
            let mut info = session.retry_info.lock().await;
            info.status = "finished".to_string();
            info.last_error = None;
            info.report = Some(report.clone());
            info.duration_ms = Some(duration_ms);
            if let Some(sender) = session.sender.lock().await.take() {
                let _ = sender.send(report.clone());
            }
        }
        prune_subagent_sessions_cache().await;

        if let Some(ref app) = ctx.app_handle {
            if let Some(state_ref) = app.try_state::<crate::commands::AppState>() {
                state_ref.remove_active_agent(&ctx.repo_root, &subagent_id);
                state_ref.session_registry.update_status(&subagent_id, crate::agent::session_registry::SessionStatus::Finished);
                if let Some(summary) = state_ref.session_registry.get_session(&subagent_id) {
                    let _ = app.emit("agent:session_status", &summary);
                }
            }
            let _ = app.emit(
                "subagent:finished",
                serde_json::json!({
                    "parentTaskId": &parent_id,
                    "subagentId": &subagent_id,
                    "response": &final_sub_response,
                    "durationMs": duration_ms,
                }),
            );
        }

        Ok(report)
    })
}

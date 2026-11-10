use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use anyhow::{bail, Result};
use tauri::Manager;
use tokio::sync::mpsc;
use crate::agent::compactor;
use crate::agent::engine_context::{maybe_compact_turn_context, resolve_initial_agent_context, sync_context_state};
use crate::agent::engine_tools::{execute_agent_turn_tools, TurnToolOutcome};
use crate::agent::prompts::{compile_primed_system_prompt_with_tools, is_tool_active_for_agent};
use crate::agent::react_engine::{AgentSessionDriver, MainAgentDriver, ReActEngine, StreamConsumerOptions};
use crate::agent::spill::get_all_tools_for_role;
use crate::agent::steering::{drain_pending_steers, AgentSessionConfig, SteeringLifetime};
use crate::agent::telemetry;
use crate::agent::types::AgentEvent;
use crate::config::{load_agent_setting, load_tool_policies};
use crate::llm::{ChatMessage, LlmClient, OpenAIChatRequest, Role, StreamChunk, ToolDefinition};

/// Executes the full ReAct loop for an agent session with deterministic prefix caching,
/// zero-trust tool policy filtering, and state-preserving context compaction.
pub async fn run_agent_loop<F>(
    config: AgentSessionConfig,
    abort_flag: Arc<AtomicBool>,
    event_sink: F,
) -> Result<String>
where
    F: Fn(AgentEvent) + Send + Sync + 'static,
{
    let _steering_lifetime = SteeringLifetime(Arc::clone(&config.steering));
    let driver = MainAgentDriver::new(
        config.task_id.clone(),
        config.worktree_path.clone(),
        config.repo_root.clone(),
        config.app_handle.clone(),
        event_sink,
    );
    let engine = ReActEngine::new(driver);
    let event_sink = |ev: AgentEvent| engine.driver.emit_event(ev);

    // Tool Policy Matrix (§ 2.16 & § 2.21):
    // tool-policy.json is the single authoritative source of truth for tool permissions.
    let tool_policy = load_tool_policies(&config.repo_root)
        .await
        .unwrap_or_default();

    // Best-effort per-agent overrides (missing/broken → defaults).
    let agent_setting = load_agent_setting(&config.repo_root, &config.role).await;
    let agent_allowed = agent_setting.allowed_tools.clone();

    let all_tools = get_all_tools_for_role(&config.repo_root, Some(&config.role)).await;

    let mut active_tools: Vec<ToolDefinition> = all_tools
        .into_iter()
        .filter(|t| {
            is_tool_active_for_agent(
                &t.function.name,
                &tool_policy,
                &config.role,
                Some(&config.stage_id),
                agent_allowed.as_deref(),
            )
        })
        .collect();

    // Deterministic sorting of tool definitions for prefix caching
    active_tools.sort_by(|a, b| a.function.name.cmp(&b.function.name));
    let active_tool_names: Vec<String> = active_tools.iter().map(|t| t.function.name.clone()).collect();

    // Deterministic static system prompt with sanitized frontmatter and dynamic tool mandates
    let system_prompt = compile_primed_system_prompt_with_tools(
        &config.repo_root,
        &config.role,
        Some(&active_tool_names),
    ).await?;

    let (starting_turn, mut messages, mut context_cursor, mut local_compaction_rev) =
        resolve_initial_agent_context(&config, &system_prompt).await?;

    let llm_client = LlmClient::for_conversation(&config.repo_root, &config.task_id);
    let mut context_revision = local_compaction_rev;
    const MAX_COMPLETION_NUDGES: usize = 3;
    let mut completion_nudges: usize = 0;
    let mut recent_tool_calls: Vec<(String, String)> = Vec::new();

    let mut turn: usize = starting_turn;
    loop {
        turn += 1;
        if abort_flag.load(Ordering::SeqCst) {
            sync_context_state(&config.app_handle, &config.task_id, turn, &mut messages, &mut local_compaction_rev);
            event_sink(AgentEvent::Aborted);
            bail!("Agent execution aborted by user");
        }

        event_sink(AgentEvent::TurnStart { turn });

        // Synchronize in-memory agent context state with AppState
        if let Some(ref app) = config.app_handle {
            if let Some(state) = app.try_state::<crate::commands::AppState>() {
                if let Ok(map) = state.agent_contexts.lock() {
                    if let Some(cs) = map.get(&config.task_id) {
                        if let Ok(guard) = cs.lock() {
                            if guard.compaction_revision > local_compaction_rev || (guard.messages.len() < messages.len() && !guard.messages.is_empty()) {
                                eprintln!(
                                    "CVH [compactor]: Adopting externally compacted messages (rev {} > {}, count {} -> {}) for task {}",
                                    guard.compaction_revision, local_compaction_rev, messages.len(), guard.messages.len(), config.task_id
                                );
                                local_compaction_rev = guard.compaction_revision;
                                messages = guard.messages.clone();
                            }
                        }
                    }
                }
            }
        }

        if drain_pending_steers(&config, &mut messages, &mut context_cursor, false)?.1 {
            completion_nudges = 0;
        }

        // State-Preserving Dynamic Context Compactor
        maybe_compact_turn_context(
            &config,
            &mut messages,
            &mut context_cursor,
            &mut local_compaction_rev,
            &mut completion_nudges,
            &event_sink,
        )?;

        // Safety guard: guarantee request never ends with an Assistant turn (Gemini HTTP 400).
        if messages.last().map(|m| m.role == Role::Assistant).unwrap_or(false) {
            let stage_name = if let Ok(pipe) = crate::config::load_pipeline_config(&config.repo_root).await {
                pipe.stages
                    .into_iter()
                    .find(|c| c.id == config.stage_id)
                    .map(|c| c.title)
                    .unwrap_or_else(|| config.stage_id.clone())
            } else {
                config.stage_id.clone()
            };
            let resume_msg = format!(
                "### [ВОЗОБНОВЛЕНИЕ ЗАДАЧИ / TASK RESUMED]\nЗадача на этапе '{}' (роль: {}). Продолжай автономное выполнение задачи. Используй доступные инструменты.",
                stage_name, config.role
            );
            messages.push(ChatMessage::user(resume_msg));
        }

        let tools: Option<Vec<crate::llm::ToolDefinition>> = if active_tools.is_empty() {
            None
        } else {
            Some(active_tools.clone())
        };

        // Pre-Flight Hard Ceiling Guard:
        let context_window = config.context_window.unwrap_or(128_000) as usize;
        let mut req_tokens = compactor::estimate_request_tokens(&messages, tools.as_deref());
        let model_ceiling = 1_000_000usize.min(context_window);

        if req_tokens >= model_ceiling {
            eprintln!(
                "CVH [agent]: Pre-flight token guard: req_tokens ({}) >= model_ceiling ({}). Triggering emergency multi-tier compaction.",
                req_tokens, model_ceiling
            );
            let emergency_target = (model_ceiling * 7) / 10;
            compactor::compact_conversation_to_target(&mut messages, emergency_target, None);
            req_tokens = compactor::estimate_request_tokens(&messages, tools.as_deref());

            if req_tokens >= model_ceiling {
                let err_msg = format!(
                    "Pre-flight token guard: context size ({} tokens) exceeds model ceiling ({} tokens). Request aborted to prevent HTTP 400.",
                    req_tokens, model_ceiling
                );
                eprintln!("CVH [agent]: {}", err_msg);
                event_sink(AgentEvent::Error(err_msg.clone()));
                bail!("{}", err_msg);
            }
        }

        let system_tokens = compactor::estimate_text_tokens(&system_prompt) as u64;
        if context_revision != local_compaction_rev {
            if drain_pending_steers(&config, &mut messages, &mut context_cursor, false)?.1 {
                completion_nudges = 0;
            }
            context_revision = local_compaction_rev;
        }
        crate::db::save_conversation_context(&config.repo_root, &config.task_id, turn, &messages, context_cursor, &abort_flag)
            .await.map_err(anyhow::Error::msg)?;
        let tool_tokens = compactor::estimate_tools_tokens(&tools) as u64;

        let req = OpenAIChatRequest {
            model: config.model.clone(),
            messages: messages.clone(),
            temperature: config.temperature,
            stream: true,
            tools: tools.clone(),
            reasoning_effort: config.reasoning_effort.clone(),
            stream_options: Some(crate::llm::StreamOptions { include_usage: true }),
        };

        telemetry::log_turn_request(
            &config.repo_root,
            &config.task_id,
            turn,
            false,
            &config.model,
            &messages,
            tools.as_deref(),
            &active_tool_names,
            Some(&config.base_url),
        )
        .await;

        let mut final_stream_res = None;
        let mut turn_stream_retry = 0;
        const MAX_TURN_STREAM_RETRIES: usize = 2;

        while turn_stream_retry <= MAX_TURN_STREAM_RETRIES {
            let (tx, mut rx) = mpsc::channel::<StreamChunk>(100);
            let client = llm_client.clone();
            let base_url = config.base_url.clone();
            let api_key = config.api_key.clone();
            let req_attempt = req.clone();

            tokio::spawn(async move {
                if let Err(e) = client.stream_openai(&base_url, api_key.as_deref(), req_attempt, tx.clone()).await {
                    let err_msg = format!("Failed to dispatch LLM request to {}: {}", base_url, e);
                    eprintln!("CVH [agent]: {}", err_msg);
                    let _ = tx.send(StreamChunk::Error(err_msg)).await;
                }
            });

            let stream_opts = StreamConsumerOptions {
                model: &config.model,
                turn,
                system_tokens,
                tool_tokens,
                fallback_input_tokens: compactor::estimate_request_tokens(&messages, tools.as_deref()) as u64,
                log_prefix: "telemetry",
                subagent_id: None,
                emit_thoughts: true,
            };

            let stream_res = engine.consume_turn_stream(&mut rx, &abort_flag, &stream_opts).await;

            if stream_res.aborted {
                final_stream_res = Some(stream_res);
                break;
            }

            let is_failure = stream_res.error.is_some() || (!stream_res.received_any_chunk && !abort_flag.load(Ordering::SeqCst));
            if is_failure && turn_stream_retry < MAX_TURN_STREAM_RETRIES && !abort_flag.load(Ordering::SeqCst) {
                turn_stream_retry += 1;
                eprintln!(
                    "CVH [agent]: Turn {} stream transient drop/error (retry {}/{}): {:?}. Retrying turn stream...",
                    turn, turn_stream_retry, MAX_TURN_STREAM_RETRIES, stream_res.error
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(500 * turn_stream_retry as u64)).await;
                continue;
            }

            final_stream_res = Some(stream_res);
            break;
        }

        let stream_res = final_stream_res.ok_or_else(|| anyhow::anyhow!("Stream result unexpectedly missing after LLM streaming loop"))?;

        if stream_res.aborted {
            sync_context_state(&config.app_handle, &config.task_id, turn, &mut messages, &mut local_compaction_rev);
            event_sink(AgentEvent::Aborted);
            bail!("Agent execution aborted by user");
        }

        if let Some(err) = stream_res.error {
            bail!("LLM streaming error: {}", err);
        }

        if !stream_res.received_any_chunk && !abort_flag.load(Ordering::SeqCst) {
            let err_msg = format!(
                "No stream chunks received from LLM endpoint ({}) - check proxy connection or port",
                config.base_url
            );
            eprintln!("CVH [agent]: {}", err_msg);
            event_sink(AgentEvent::Error(err_msg.clone()));
            bail!("{}", err_msg);
        }

        let turn_content = stream_res.content;
        let turn_thoughts = stream_res.thoughts;
        let turn_tool_calls = stream_res.tool_calls;

        telemetry::log_turn_response(
            &config.repo_root,
            &config.task_id,
            turn,
            false,
            &turn_thoughts,
            &turn_content,
            &turn_tool_calls,
        )
        .await;

        // If no tool calls, terminal response reached
        if turn_tool_calls.is_empty() {
            if !turn_content.is_empty() {
                messages.push(ChatMessage::assistant(Some(turn_content.clone()), None));
            }
            crate::db::save_conversation_context(&config.repo_root, &config.task_id, turn, &messages, context_cursor, &abort_flag)
                .await.map_err(anyhow::Error::msg)?;
            sync_context_state(&config.app_handle, &config.task_id, turn, &mut messages, &mut local_compaction_rev);
            let unfinished_todos = if active_tool_names.iter().any(|name| name == "todo_write") {
                crate::db::db_get_todos(&config.repo_root, &config.task_id)
                    .map_err(anyhow::Error::msg)?
            } else {
                Vec::new()
            };
            let unfinished = !unfinished_todos.is_empty();
            let (steer_appended, user_instruction) = drain_pending_steers(
                &config,
                &mut messages,
                &mut context_cursor,
                !unfinished || completion_nudges >= MAX_COMPLETION_NUDGES,
            )?;
            if steer_appended {
                if user_instruction {
                    completion_nudges = 0;
                }
                sync_context_state(&config.app_handle, &config.task_id, turn, &mut messages, &mut local_compaction_rev);
                crate::db::save_conversation_context(&config.repo_root, &config.task_id, turn, &messages, context_cursor, &abort_flag)
                    .await.map_err(anyhow::Error::msg)?;
                continue;
            }
            if unfinished {
                let items_list = unfinished_todos
                    .iter()
                    .map(|t| format!("- [{}] {}", t.status, t.content))
                    .collect::<Vec<_>>()
                    .join("\n");
                if completion_nudges < MAX_COMPLETION_NUDGES {
                    completion_nudges += 1;
                    let reminder = format!(
                        "[Runtime completion check] ({}/{})\n\
                         Your task checklist still has {} unfinished item(s):\n{}\n\
                         Final plain-text responses without tool calls are prohibited while items remain unfinished. \
                         Your next step MUST be a tool call: execute the remaining work, update `todo_write`, \
                         and call `advance_task` once all deliverables are verified. If blocked, use `question` or state the concrete blocker.",
                        completion_nudges,
                        MAX_COMPLETION_NUDGES,
                        unfinished_todos.len(),
                        items_list
                    );
                    messages.push(ChatMessage::user(reminder));
                    sync_context_state(&config.app_handle, &config.task_id, turn, &mut messages, &mut local_compaction_rev);
                    crate::db::save_conversation_context(&config.repo_root, &config.task_id, turn, &messages, context_cursor, &abort_flag)
                        .await.map_err(anyhow::Error::msg)?;
                    continue;
                }
                let warn_banner = format!(
                    "\n\n> ⚠️ **План не закрыт ({} пункт(ов) осталось незавершёнными):**\n{}",
                    unfinished_todos.len(),
                    unfinished_todos
                        .iter()
                        .map(|t| format!("> - `[{}]` {}", t.status, t.content))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
                let final_with_warning = if turn_content.trim().is_empty() {
                    warn_banner.trim_start().to_string()
                } else {
                    format!("{}{}", turn_content, warn_banner)
                };
                event_sink(AgentEvent::Finished {
                    final_response: final_with_warning.clone(),
                });
                return Ok(final_with_warning);
            }
            event_sink(AgentEvent::Finished {
                final_response: turn_content.clone(),
            });
            return Ok(turn_content);
        }

        // Append assistant turn message
        messages.push(ChatMessage::assistant(
            if turn_content.is_empty() { None } else { Some(turn_content) },
            Some(turn_tool_calls.clone()),
        ));

        // Execute turn tool calls via engine_tools
        let tool_outcome = execute_agent_turn_tools(
            &config,
            &tool_policy,
            agent_allowed.as_deref(),
            turn,
            &turn_tool_calls,
            &mut messages,
            &mut recent_tool_calls,
            &mut context_cursor,
            &mut local_compaction_rev,
            &mut completion_nudges,
            &abort_flag,
            &event_sink,
        ).await?;

        if let TurnToolOutcome::Terminated(res) = tool_outcome {
            return Ok(res);
        }

        // Sync messages to in-memory context state
        sync_context_state(&config.app_handle, &config.task_id, turn, &mut messages, &mut local_compaction_rev);
    }
}

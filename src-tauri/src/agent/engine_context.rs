use anyhow::Result;
use tauri::{AppHandle, Emitter, Manager};
use crate::agent::compactor;
use crate::agent::sanitizer::{append_messages_from_dialog, reconstruct_messages_from_dialog, sanitize_unclosed_tool_calls};
use crate::agent::steering::{persist_steer_message, AgentSessionConfig};
use crate::llm::{ChatMessage, Role};

pub fn sync_context_state(
    app_handle: &Option<AppHandle>,
    task_id: &str,
    turn: usize,
    messages: &mut Vec<ChatMessage>,
    local_compaction_rev: &mut u64,
) {
    if let Some(ref app) = app_handle {
        if let Some(state) = app.try_state::<crate::commands::AppState>() {
            if let Ok(map) = state.agent_contexts.lock() {
                if let Some(cs) = map.get(task_id) {
                    if let Ok(mut guard) = cs.lock() {
                        if guard.compaction_revision > *local_compaction_rev {
                            eprintln!(
                                "CVH [agent]: Detected external compaction (rev {} > {}) for task {}, adopting {} messages",
                                guard.compaction_revision, *local_compaction_rev, task_id, guard.messages.len()
                            );
                            *local_compaction_rev = guard.compaction_revision;
                            *messages = guard.messages.clone();
                        } else {
                            guard.current_turn = turn;
                            guard.messages = messages.clone();
                        }
                    }
                }
            }
        }
    }
}

pub async fn resolve_initial_agent_context(
    config: &AgentSessionConfig,
    system_prompt: &str,
) -> Result<(usize, Vec<ChatMessage>, i64, u64)> {
    // Zero-Turn Memory Priming Relay Handoff (§ 2.9):
    // If task has handover_summary, inject at the beginning of the prompt.
    let mut initial_user_prompt = String::new();
    if let Some(ref handover) = config.handover_summary {
        let trimmed = handover.trim();
        if !trimmed.is_empty() {
            initial_user_prompt.push_str(&format!(
                "### [ЭСТАФЕТА ПАЙПЛАЙНА / RELAY HANDOFF]\nПредыдущий этап пайплайна передал следующее саммари:\n{}\n\n",
                trimmed
            ));
        }
    }
    initial_user_prompt.push_str(&format!(
        "# Objective: {}\n\n{}",
        config.task_title,
        config.task_description.trim()
    ));

    if let Some(ref atts) = config.task_attachments {
        if !atts.is_empty() {
            initial_user_prompt.push_str(&format!(
                "\n\n[ВЛОЖЕНИЯ К ЗАДАЧЕ / ATTACHMENTS]: Прикреплено {} изображений/скриншотов. Внимательно изучи их при выполнении задачи.",
                atts.len()
            ));
        }
    }

    if let Some(steer) = config.steer_prompt.as_deref().map(str::trim).filter(|text| !text.is_empty()) {
        let already_persisted_id = crate::db::load_dialog_items(&config.repo_root, &config.task_id)
            .ok()
            .and_then(|items| {
                items.into_iter().rev().find(|it| {
                    it.item_type == "steer" && it.content.as_deref() == Some(steer)
                }).map(|it| it.id)
            });

        if let Some(existing_id) = already_persisted_id {
            eprintln!("CVH [agent]: Steer message already persisted as '{}', reusing.", existing_id);
        } else {
            let steer_message_id = if steer.contains("[ОТЧЕТ ОБ ИСПРАВЛЕНИЯХ / HANDOVER SUMMARY]") {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                use std::hash::{Hash, Hasher};
                steer.hash(&mut hasher);
                Some(format!("handover:{}:{:x}", config.task_id, hasher.finish()))
            } else {
                None
            };
            if let Some(item) = persist_steer_message(&config.repo_root, &config.task_id, steer, config.steer_images.clone(), steer_message_id.as_deref()).map_err(anyhow::Error::msg)? {
                if let Some(app) = config.app_handle.as_ref() {
                    let payload = serde_json::json!({ "taskId": config.task_id, "item": item });
                    let _ = app.emit("task:activity", &payload);
                    let _ = app.emit("task:dialog", &payload);
                }
            }
        }
    }
    let context_cursor = crate::db::context_dialog_cursor(&config.repo_root, &config.task_id)
        .map_err(anyhow::Error::msg)?;
    let saved_context = crate::db::load_conversation_context(&config.repo_root, &config.task_id)
        .map_err(anyhow::Error::msg)?;

    // 1. Check if an existing in-memory context already exists for this task_id
    let in_memory_context = if saved_context.is_some() || context_cursor > 0 || config.steer_prompt.is_some() {
        None
    } else if let Some(ref app) = config.app_handle {
        if let Some(state) = app.try_state::<crate::commands::AppState>() {
            state.agent_contexts.lock().ok().and_then(|map| {
                map.get(&config.task_id).and_then(|cs| {
                    let guard = cs.lock().ok()?;
                    // Discard stale in-memory context if SQLite was compacted and in-memory is not
                    let in_memory_has_compacted = guard.messages.iter().any(|m| {
                        m.content.as_deref().map(|c| c.contains("# Compacted Context Summary")).unwrap_or(false)
                    });
                    let sqlite_has_compacted = crate::db::load_dialog_items(&config.repo_root, &config.task_id)
                        .map(|items| items.iter().any(|it| it.item_type == "compacted"))
                        .unwrap_or(false);

                    if !in_memory_has_compacted && sqlite_has_compacted {
                        eprintln!("CVH [agent]: In-memory context is stale (missing compaction found in SQLite). Rebuilding from SQLite.");
                        return None;
                    }

                    if guard.messages.len() > 2 {
                        Some((guard.current_turn, guard.messages.clone()))
                    } else {
                        None
                    }
                })
            })
        } else {
            None
        }
    } else {
        None
    };

    // 2. Check SQLite history if in-memory context is absent or <= 2 messages
    let sqlite_context = if in_memory_context.is_none() {
        if let Some(mut saved) = saved_context {
            let pending = crate::db::load_dialog_items_between(&config.repo_root, &config.task_id, saved.dialog_cursor, context_cursor)
                .map_err(anyhow::Error::msg)?;
            let pending: Vec<_> = pending.into_iter().filter(|item| {
                crate::agent::sanitizer::is_dialog_item_replayable(item)
            }).collect();
            append_messages_from_dialog(&mut saved.messages, &initial_user_prompt, &pending);
            Some((saved.turn, saved.messages))
        } else if let Ok(dialog_items) = crate::db::load_dialog_items_between(&config.repo_root, &config.task_id, 0, context_cursor) {
            let reconstructed = reconstruct_messages_from_dialog(system_prompt, &initial_user_prompt, &dialog_items);
            if reconstructed.len() > 2 {
                Some((0, reconstructed))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // 3. Adopt existing messages or initialize fresh 2-message array
    let (starting_turn, mut messages) = if let Some((cur_turn, mut msgs)) = in_memory_context {
        eprintln!(
            "CVH [agent]: Adopting existing in-memory context ({} messages, turn {}) for task {}",
            msgs.len(), cur_turn, config.task_id
        );
        // Refresh system message so active tools, personas, and rules are always 100% in sync
        if !msgs.is_empty() && msgs[0].role == Role::System {
            msgs[0] = ChatMessage::system(system_prompt.to_string())
                .with_cache_control(crate::llm::CacheControl::ephemeral());
        } else {
            msgs.insert(
                0,
                ChatMessage::system(system_prompt.to_string())
                    .with_cache_control(crate::llm::CacheControl::ephemeral()),
            );
        }
        if let Some(ref atts) = config.task_attachments {
            if !atts.is_empty() && msgs.len() >= 2 && msgs[1].role == Role::User && !msgs[1].has_images() {
                let text = msgs[1].text_content().unwrap_or(&initial_user_prompt).to_string();
                msgs[1] = ChatMessage::user_with_images(text, atts.clone())
                    .with_cache_control(crate::llm::CacheControl::ephemeral());
            }
        }
        (cur_turn, msgs)
    } else if let Some((cur_turn, mut msgs)) = sqlite_context {
        eprintln!(
            "CVH [agent]: Adopting reconstructed SQLite dialogue context ({} messages) for task {}",
            msgs.len(), config.task_id
        );
        // Refresh system message so active tools, personas, and rules are always 100% in sync
        if !msgs.is_empty() && msgs[0].role == Role::System {
            msgs[0] = ChatMessage::system(system_prompt.to_string())
                .with_cache_control(crate::llm::CacheControl::ephemeral());
        } else {
            msgs.insert(
                0,
                ChatMessage::system(system_prompt.to_string())
                    .with_cache_control(crate::llm::CacheControl::ephemeral()),
            );
        }
        if let Some(ref atts) = config.task_attachments {
            if !atts.is_empty() && msgs.len() >= 2 && msgs[1].role == Role::User && !msgs[1].has_images() {
                let text = msgs[1].text_content().unwrap_or(&initial_user_prompt).to_string();
                msgs[1] = ChatMessage::user_with_images(text, atts.clone())
                    .with_cache_control(crate::llm::CacheControl::ephemeral());
            }
        }
        (cur_turn, msgs)
    } else {
        let initial_user_msg = if let Some(ref atts) = config.task_attachments {
            if !atts.is_empty() {
                ChatMessage::user_with_images(initial_user_prompt, atts.clone())
            } else {
                ChatMessage::user(initial_user_prompt)
            }
        } else {
            ChatMessage::user(initial_user_prompt)
        };
        (
            0,
            vec![
                ChatMessage::system(system_prompt.to_string()).with_cache_control(crate::llm::CacheControl::ephemeral()),
                initial_user_msg.with_cache_control(crate::llm::CacheControl::ephemeral()),
            ],
        )
    };

    // 4. Sanitize any unclosed tool calls (e.g. from an aborted turn mid-tool execution)
    sanitize_unclosed_tool_calls(&mut messages);

    // 5b. Guard against Gemini HTTP 400 "Requests ending with a model turn are not supported"
    // when resuming a task without a steer prompt.
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

    // 6. Synchronize active context state with AppState
    let mut local_compaction_rev: u64 = 0;
    if let Some(ref app) = config.app_handle {
        if let Some(state) = app.try_state::<crate::commands::AppState>() {
            if let Ok(mut map) = state.agent_contexts.lock() {
                let initial_rev = map.get(&config.task_id)
                    .and_then(|cs| cs.lock().ok().map(|g| g.compaction_revision))
                    .unwrap_or(0);
                local_compaction_rev = initial_rev;
                map.insert(
                    config.task_id.clone(),
                    std::sync::Arc::new(std::sync::Mutex::new(compactor::AgentContextState {
                        current_turn: starting_turn,
                        messages: messages.clone(),
                        compaction_revision: initial_rev,
                    })),
                );
            }
        }
    }

    Ok((starting_turn, messages, context_cursor, local_compaction_rev))
}

pub fn maybe_compact_turn_context<F>(
    config: &AgentSessionConfig,
    messages: &mut Vec<ChatMessage>,
    context_cursor: &mut i64,
    local_compaction_rev: &mut u64,
    completion_nudges: &mut usize,
    event_sink: &F,
) -> Result<()>
where
    F: Fn(crate::agent::types::AgentEvent),
{
    let context_window = config.context_window.unwrap_or(128_000) as usize;
    let compaction_threshold = context_window;
    let current_tokens = compactor::estimate_tokens(messages);

    let should_compact = current_tokens >= compaction_threshold;
    if should_compact {
        let target_window = (context_window * 3) / 4;
        let compaction_res = compactor::compact_conversation_to_target(messages, target_window, None);
        if compaction_res.saved_chars > 0 || compactor::estimate_tokens(messages) < current_tokens {
            let tokens_after = compactor::estimate_tokens(messages);
            let saved_tokens = compaction_res.estimated_saved_tokens;
            let stdout_msg = format!(
                "Контекст успешно сжат. Сохранено {} токенов. Контекст сокращен с {} до {} токенов.",
                saved_tokens, current_tokens, tokens_after
            );
            let tool_item_id = format!("compact_context-{}-{}", config.task_id, chrono::Utc::now().timestamp_millis());
            let tool_act_item = crate::db::ActivityItem {
                id: tool_item_id.clone(),
                item_type: "tool".to_string(),
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                content: None,
                tool: Some(serde_json::json!({
                    "id": &tool_item_id,
                    "toolName": "compact_context",
                    "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
                    "durationMs": 0,
                    "status": "success",
                    "params": { "taskId": &config.task_id },
                    "stdout": &stdout_msg,
                })),
                duration_ms: Some(0.0),
                images: None,
            };

            if let Some(ref app) = config.app_handle {
                let summary_text = if !compaction_res.preserved_state_summary.is_empty() {
                    compaction_res.preserved_state_summary.clone()
                } else {
                    "Dynamic context compaction: older turns condensed to preserve context window.".to_string()
                };
                let compacted_item = crate::db::ActivityItem {
                    id: format!("compacted-{}-{}", config.task_id, chrono::Utc::now().timestamp_millis()),
                    item_type: "compacted".to_string(),
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    content: Some(summary_text),
                    tool: Some(serde_json::json!({
                        "compactedCount": compaction_res.evaporated_tools_count,
                        "savedTokens": saved_tokens,
                        "tokensBefore": current_tokens,
                        "tokensAfter": tokens_after,
                        "autoCompacted": true,
                        "contextWindow": context_window,
                    })),
                    duration_ms: None,
                    images: None,
                };

                let existing_items = crate::db::load_dialog_items(&config.repo_root, &config.task_id).unwrap_or_default();
                let has_summary = messages.iter().any(|m| compactor::is_compacted_summary(&m.role, m.content.as_deref().unwrap_or("")));
                let non_recent_overhead = if has_summary { 3 } else { 2 };
                let recent_count = messages.len().saturating_sub(non_recent_overhead);
                let archive_count = existing_items.len().saturating_sub(recent_count);
                let ids_to_archive: Vec<String> = existing_items[..archive_count].iter().map(|it| it.id.clone()).collect();

                let _ = crate::db::persist_compacted_dialog(&config.repo_root, &config.task_id, &compacted_item, Some(&tool_act_item), &ids_to_archive);
                if crate::agent::steering::drain_pending_steers(config, messages, context_cursor, false)?.1 {
                    *completion_nudges = 0;
                }
                let _ = app.emit(
                    "agent:tool_call",
                    serde_json::json!({
                        "taskId": &config.task_id,
                        "id": &tool_item_id,
                        "name": "compact_context",
                        "arguments": serde_json::json!({ "taskId": &config.task_id }).to_string(),
                    }),
                );
                let _ = app.emit(
                    "agent:tool_result",
                    serde_json::json!({
                        "taskId": &config.task_id,
                        "id": &tool_item_id,
                        "name": "compact_context",
                        "result": &stdout_msg,
                        "success": true,
                    }),
                );
                let _ = app.emit("dialog:updated", serde_json::json!({ "taskId": config.task_id }));

                *local_compaction_rev += 1;
                if let Some(state) = app.try_state::<crate::commands::AppState>() {
                    if let Ok(map) = state.agent_contexts.lock() {
                        if let Some(cs) = map.get(&config.task_id) {
                            if let Ok(mut guard) = cs.lock() {
                                guard.compaction_revision = *local_compaction_rev;
                                guard.messages = messages.clone();
                            }
                        }
                    }
                }
            }

            event_sink(crate::agent::types::AgentEvent::Compacted {
                saved_chars: compaction_res.saved_chars,
                estimated_saved_tokens: compaction_res.estimated_saved_tokens,
                evaporated_count: compaction_res.evaporated_tools_count,
                tokens_before: current_tokens,
                tokens_after,
            });
        }
    }

    Ok(())
}

//! Compactor Orchestration & LLM Condensation (§ 2.22)

use std::sync::LazyLock;
use crate::llm::{ChatMessage, Role};
use tauri::{AppHandle, Emitter, State};
use crate::commands::state::AppState;
use crate::commands::tasks::storage::{load_tasks_from_disk, save_tasks_to_disk};
use crate::commands::tasks::runner::resolve_task_active_agent_model;
use super::tokens::{
    estimate_text_tokens, estimate_tokens, estimate_tools_tokens, resolve_compactor_model,
};
use super::extractor::{
    format_conversation_for_compaction, generate_fallback_compaction_report,
    COMPACTION_SYSTEM_PROMPT,
};
use super::evaporation::{
    evaporate_current_turn_tools, evaporate_tool_logs_older_than, CompactionReport,
};
use super::multi_tier::compact_conversation_to_target;

static COMPACTOR_HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .unwrap_or_default()
});

/// Executes LLM compaction using the configured model (or inherits active agent model).
/// Falls back safely to deterministic report if endpoint is unreachable.
pub async fn condense_context_with_llm(
    task_id: &str,
    messages_to_condense: &[ChatMessage],
    model: &str,
    base_url: &str,
    api_key: Option<&str>,
    task_title: Option<&str>,
    task_desc: Option<&str>,
) -> String {
    let mut msgs = messages_to_condense.to_vec();
    // Pre-evaporate bulky tool logs & thoughts before preparing transcript for LLM
    let _ = evaporate_tool_logs_older_than(&mut msgs, usize::MAX, 0);
    let _ = evaporate_current_turn_tools(&mut msgs, usize::MAX);

    let transcript = format_conversation_for_compaction(&msgs);
    if transcript.len() > 400_000 {
        eprintln!(
            "CVH [compactor]: Compaction transcript exceeds 400,000 chars ({} chars). Using deterministic fallback report to prevent HTTP 400.",
            transcript.len()
        );
        return generate_fallback_compaction_report(task_id, &msgs);
    }

    let trimmed_base = base_url.trim().trim_end_matches('/');
    let endpoint = if trimmed_base.ends_with("/chat/completions") {
        trimmed_base.to_string()
    } else {
        format!("{}/chat/completions", trimmed_base)
    };

    let client = &*COMPACTOR_HTTP_CLIENT;

    let mut headers = crate::llm::cache::opencode_headers(base_url, &format!("{task_id}:compactor"));
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        reqwest::header::HeaderValue::from_static("application/json"),
    );
    if let Some(key) = api_key {
        let trimmed_key = key.trim();
        if !trimmed_key.is_empty() {
            if let Ok(val) = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", trimmed_key)) {
                headers.insert(reqwest::header::AUTHORIZATION, val);
            }
        }
    }

    let title_str = task_title.unwrap_or("").trim();
    let desc_str = task_desc.unwrap_or("").trim();
    let desc_truncated: String = desc_str.chars().take(600).collect();
    let goal_clause = if !title_str.is_empty() {
        format!(
            "\n                     - In '## Primary Request and Intent': The immutable PRIMARY TASK GOAL is: \"{}\" (Description: {}).\n                       You MUST state this primary task goal. DO NOT replace or confuse it with intermediate user steer directives, feedback, questions, or side notes.",
            title_str, desc_truncated
        )
    } else {
        "\n                     - In '## Primary Request and Intent': State the original task goal. DO NOT replace or confuse it with intermediate user steer directives.".to_string()
    };

    let payload = serde_json::json!({
        "model": model,
        "temperature": 0.1,
        "max_tokens": 4096,
        "messages": [
            {
                "role": "system",
                "content": COMPACTION_SYSTEM_PROMPT
            },
            {
                "role": "user",
                "content": format!(
                    "Summarize and condense the following conversation history for task {} into the 7 structured sections.\n                     CRITICAL INSTRUCTIONS FOR STATE PRESERVATION:{}\n                     - In '## Files and Code': List every single file touched via edit_file/write_file/read_file with exact paths and what changed.\n                     - In '## Errors and Fixes': Detail all compiler errors, sensor results, and test outcomes (passed/failed).\n                     - In '## Current Work & Next Step': Preserve the active todos/checklist status and state the exact next technical action to take.\n\n                     Conversation Transcript:\n{}",
                    task_id, goal_clause, transcript
                )
            }
        ]
    });

    match client.post(&endpoint).headers(headers).json(&payload).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                if let Some(content) = body["choices"][0]["message"]["content"].as_str() {
                    let trimmed = content.trim();
                    if !trimmed.is_empty() {
                        return trimmed.to_string();
                    }
                }
            }
            generate_fallback_compaction_report(task_id, &msgs)
        }
        _ => generate_fallback_compaction_report(task_id, &msgs),
    }
}

/// Orchestrates manual context compaction for a given task:
/// 1. Emits running tool call event for UI.
/// 2. Resolves models and endpoints.
/// 3. Extracts older messages to condense while keeping recent dialog turns.
/// 4. Invokes LLM condensation with the structured 7-section prompt.
/// 5. Persists compacted dialog item to SQLite.
/// 6. Synchronizes token usage to disk and state memory.
/// 7. Updates in-memory active agent context if running.
/// 8. Emits tool completion, compaction, and telemetry events.
pub async fn compact_context_for_task(
    task_id: String,
    tool_id: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<CompactionReport, String> {
    let root = state.get_root();

    let tool_item_id = tool_id.unwrap_or_else(|| {
        format!(
            "compact_context-{}-{}",
            task_id,
            chrono::Utc::now().timestamp_millis()
        )
    });

    // 0. Emit agent:tool_call immediately so chat feed displays running compact_context card
    let _ = app_handle.emit(
        "agent:tool_call",
        serde_json::json!({
            "taskId": &task_id,
            "id": &tool_item_id,
            "name": "compact_context",
            "arguments": serde_json::json!({
                "taskId": &task_id,
                "title": "Сжатие контекста...",
            }).to_string(),
        }),
    );

    // 1. Load LLM configuration and resolve active agent model
    let llm_settings = crate::config::load_llm_settings(&root).await.unwrap_or_default();
    let tasks = load_tasks_from_disk(&root).await;
    let task = tasks.iter().find(|t| t.id == task_id).cloned();

    let active_agent_model = resolve_task_active_agent_model(&root, task.as_ref(), &llm_settings).await;

    // 2. Resolve compactor model (inheritance support)
    let compactor_model = resolve_compactor_model(
        &llm_settings.compactor_model,
        &active_agent_model,
    );

    let (provider_base_url, api_key) = llm_settings.endpoint_for_pair(
        task.as_ref().and_then(|t| t.provider_id.as_deref()),
        &compactor_model,
    );
    let base_url = if !provider_base_url.trim().is_empty() {
        provider_base_url
    } else {
        "https://opencode.ai/inference/go/openai/v1".to_string()
    };

    // 3. Load dialogue items from SQLite
    let root_for_load = root.clone();
    let tid_for_load = task_id.clone();
    let dialog_items = tokio::task::spawn_blocking(move || {
        crate::db::load_dialog_items(&root_for_load, &tid_for_load)
    })
    .await
    .map_err(|e| format!("Failed to join dialog load task: {e}"))?
    .unwrap_or_default();

    // 4. Determine items to condense before the last 4 active messages (2 full recent turns)
    let mut ids_to_archive: Vec<String> = Vec::new();
    let mut messages_to_condense: Vec<ChatMessage> = Vec::new();

    if dialog_items.len() > 4 {
        let split_idx = dialog_items.len().saturating_sub(4);
        let items_slice = &dialog_items[..split_idx];

        for it in items_slice {
            ids_to_archive.push(it.id.clone());
            if it.is_runtime_error() || it.item_type == "system" { continue; }
            match it.item_type.as_str() {
                "user" | "steer" => {
                    messages_to_condense.push(ChatMessage::user(
                        it.content.clone().unwrap_or_default(),
                    ));
                }
                "thought" => {
                    let mut msg = ChatMessage::assistant(
                        it.content.clone(),
                        None,
                    );
                    msg.name = Some("thought".to_string());
                    messages_to_condense.push(msg);
                }
                "tool" => {
                    if let Some(ref t) = it.tool {
                        let tool_id = t.get("id").and_then(|v| v.as_str()).unwrap_or("tool_call");
                        let stdout = t.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
                        let tool_name = t.get("toolName").and_then(|v| v.as_str()).unwrap_or("tool");
                        let params = t.get("params").cloned().unwrap_or(serde_json::Value::Null);

                        // Reconstruct Assistant ToolCall message so compactor extracts tool name and parameters
                        messages_to_condense.push(ChatMessage::assistant(
                            None,
                            Some(vec![crate::llm::ToolCall {
                                id: tool_id.to_string(),
                                call_type: "function".to_string(),
                                function: crate::llm::FunctionCall {
                                    name: tool_name.to_string(),
                                    arguments: params.to_string(),
                                },
                            }]),
                        ));
                        messages_to_condense.push(ChatMessage::tool_response(
                            tool_id,
                            stdout.to_string(),
                        ));
                    }
                }
                _ => {
                    if let Some(ref c) = it.content {
                        messages_to_condense.push(ChatMessage::assistant(
                            Some(c.clone()),
                            None,
                        ));
                    }
                }
            }
        }
    } else {
        // Fallback to in-memory active agent context if available, or task description
        let ctx_key = AppState::agent_key(&root, &task_id);
        let mem_msgs = {
            let map = state.agent_contexts.lock().unwrap_or_else(|p| p.into_inner());
            map.get(&ctx_key).or_else(|| map.get(task_id.trim())).map(|cs| {
                let g = cs.lock().unwrap_or_else(|p| p.into_inner());
                g.messages.clone()
            })
        };

        if let Some(msgs) = mem_msgs {
            if msgs.len() > 4 {
                let split_idx = msgs.len().saturating_sub(4);
                messages_to_condense.extend(msgs[..split_idx].iter().cloned());
            }
        }

        if messages_to_condense.is_empty() {
            if let Some(ref t) = task {
                messages_to_condense.push(ChatMessage::user(format!(
                    "Task {}: {}\n{}",
                    t.id, t.title, t.description
                )));
            } else {
                messages_to_condense.push(ChatMessage::user(format!(
                    "Task {}",
                    task_id
                )));
            }
        }
    }

    let tokens_before = estimate_tokens(&messages_to_condense);

    let (task_title, task_desc) = {
        let guard = state.tasks.lock().unwrap_or_else(|p| p.into_inner());
        guard.iter().find(|t| t.id == task_id)
            .map(|t| (t.title.clone(), t.description.clone()))
            .unwrap_or_default()
    };

    // 5. Run LLM condensation with the 7-section structured prompt
    let compacted_markdown = condense_context_with_llm(
        &task_id,
        &messages_to_condense,
        &compactor_model,
        &base_url,
        api_key.as_deref(),
        Some(&task_title),
        Some(&task_desc),
    )
    .await;

    let tokens_after = estimate_text_tokens(&compacted_markdown);
    let saved_tokens = tokens_before.saturating_sub(tokens_after);
    let compacted_count = if !ids_to_archive.is_empty() {
        ids_to_archive.len()
    } else {
        messages_to_condense.len().max(1)
    };

    // 6. Build tool execution item and compacted summary item for SQLite and in-chat display
    let stdout_msg = format!(
        "Контекст успешно сжат. Сохранено {} токенов. Контекст сокращен с {} до {} токенов.",
        saved_tokens, tokens_before, tokens_after
    );
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
            "params": {
                "taskId": &task_id,
                "title": format!("Сжато (-{} токенов)", saved_tokens),
                "savedTokens": saved_tokens,
            },
            "stdout": &stdout_msg,
        })),
        duration_ms: Some(0.0),
        images: None,
    };

    let compacted_item = crate::db::ActivityItem {
        id: format!("compacted-{}-{}", task_id, chrono::Utc::now().timestamp_millis()),
        item_type: "compacted".to_string(),
        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        content: Some(compacted_markdown.clone()),
        tool: Some(serde_json::json!({
            "compactedCount": compacted_count,
            "savedTokens": saved_tokens,
            "tokensBefore": tokens_before,
            "tokensAfter": tokens_after,
            "compactorModel": compactor_model,
        })),
        duration_ms: None,
        images: None,
    };

    let root_for_persist = root.clone();
    let tid_for_persist = task_id.clone();
    let compacted_for_persist = compacted_item.clone();
    let tool_for_persist = tool_act_item.clone();
    let archive_for_persist = ids_to_archive.clone();
    tokio::task::spawn_blocking(move || {
        crate::db::persist_compacted_dialog(
            &root_for_persist,
            &tid_for_persist,
            &compacted_for_persist,
            Some(&tool_for_persist),
            &archive_for_persist,
        )
    })
    .await
    .map_err(|e| format!("Failed to join compacted dialog task: {}", e))?
    .map_err(|e| format!("Failed to persist compacted dialog: {}", e))?;

    // 7. Store task.current_tokens in memory and persist to disk (Fixes 100% gauge on re-entering drawer)
    {
        let mut disk_tasks = load_tasks_from_disk(&root).await;
        if let Some(t) = disk_tasks.iter_mut().find(|t| t.id == task_id) {
            t.current_tokens = Some(tokens_after);
        }
        let _ = save_tasks_to_disk(&root, &disk_tasks).await;

        let mut guard = state.tasks.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(t) = guard.iter_mut().find(|t| t.id == task_id) {
            t.current_tokens = Some(tokens_after);
        }
    }

    // 8. Update in-memory active agent context if present (preserving Turn 0 anchors and last 4 messages)
    let ctx_key = AppState::agent_key(&root, &task_id);
    let context_state_opt = {
        let map = state.agent_contexts.lock().unwrap_or_else(|p| p.into_inner());
        map.get(&ctx_key).or_else(|| map.get(task_id.trim())).cloned()
    };

    let (sys_tokens, tool_tokens, total_tokens_after) = if let Some(cs) = context_state_opt {
        let mut guard = cs.lock().unwrap_or_else(|p| p.into_inner());
        let mut new_messages = Vec::new();
        if !guard.messages.is_empty() {
            new_messages.push(guard.messages[0].clone()); // System prompt
        }
        if guard.messages.len() > 1 && guard.messages[1].role == Role::User {
            new_messages.push(guard.messages[1].clone()); // Initial user spec
        }
        new_messages.push(ChatMessage::user(format!(
            "# Compacted Context Summary\n\n{}",
            compacted_markdown
        )));
        if guard.messages.len() > 4 {
            let tail_start = guard.messages.len().saturating_sub(4);
            let actual_tail = tail_start.max(2);
            for msg in &guard.messages[actual_tail..] {
                new_messages.push(msg.clone());
            }
        }
        // Enforce safe ceiling guarantee on in-memory context as well
        let _ = compact_conversation_to_target(&mut new_messages, 96_000, None);
        guard.messages = new_messages;
        guard.compaction_revision += 1;

        let sys = guard.messages.first()
            .and_then(|m| m.content.as_deref())
            .map(estimate_text_tokens)
            .unwrap_or(0);
        let tool = estimate_tools_tokens(&None);
        let tot = estimate_tokens(&guard.messages) + tool;
        (sys, tool, tot)
    } else {
        (0, 0, tokens_after)
    };

    let report = CompactionReport {
        task_id: task_id.clone(),
        tokens_before,
        tokens_after: total_tokens_after,
        evaporated_count: compacted_count,
        saved_tokens,
        compacted_messages_count: compacted_count,
        summary: Some(compacted_markdown),
    };

    // 9. Emit Tauri events for compact_context tool execution card, compaction report, and telemetry
    let _ = app_handle.emit(
        "agent:tool_result",
        serde_json::json!({
            "taskId": &task_id,
            "id": &tool_item_id,
            "name": "compact_context",
            "result": &stdout_msg,
            "success": true,
        }),
    );
    let _ = app_handle.emit("agent:compacted", &report);
    let _ = app_handle.emit(
        "agent:telemetry",
        serde_json::json!({
            "taskId": task_id,
            "inputTokens": total_tokens_after,
            "outputTokens": 0,
            "cacheReadTokens": 0,
            "cacheCreationTokens": 0,
            "cacheHitRate": 0.0,
            "systemTokens": sys_tokens,
            "toolTokens": tool_tokens,
        }),
    );
    let _ = app_handle.emit("dialog:updated", serde_json::json!({ "taskId": task_id }));

    Ok(report)
}

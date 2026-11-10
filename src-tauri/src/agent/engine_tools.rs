use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use anyhow::{bail, Result};
use crate::agent::engine_context::sync_context_state;
use crate::agent::react_engine::{detect_degenerate_command_or_loop, normalize_tool_call_for_circuit_breaker};
use crate::agent::sanitizer::sanitize_unclosed_tool_calls;
use crate::agent::steering::{drain_pending_steers, AgentSessionConfig};
use crate::agent::subagent_runtime::{
    are_questions_duplicate, are_subagent_prompts_duplicate, canonical_tool_name,
    should_abort_tool_batch, ExecutedSubagentCall, ToolContext,
};
use crate::agent::tool_dispatch::execute_tool;
use crate::agent::types::{AgentEvent, TodoItem};
use crate::llm::{ChatMessage, ToolCall};

pub enum TurnToolOutcome {
    Continue,
    Terminated(String),
}

pub async fn execute_agent_turn_tools<F>(
    config: &AgentSessionConfig,
    tool_policy: &crate::config::ToolPolicyConfig,
    agent_allowed: Option<&[String]>,
    turn: usize,
    turn_tool_calls: &[ToolCall],
    messages: &mut Vec<ChatMessage>,
    recent_tool_calls: &mut Vec<(String, String)>,
    context_cursor: &mut i64,
    local_compaction_rev: &mut u64,
    completion_nudges: &mut usize,
    abort_flag: &Arc<AtomicBool>,
    event_sink: &F,
) -> Result<TurnToolOutcome>
where
    F: Fn(AgentEvent) + Send + Sync,
{
    let tool_ctx = ToolContext {
        repo_root: &config.repo_root,
        worktree_dir: &config.worktree_path,
        stage_id: Some(&config.stage_id),
        role: Some(&config.role),
        abort_handle: Arc::clone(abort_flag),
        tool_policy,
        task_id: Some(&config.task_id),
        agent_allowed_tools: agent_allowed,
        app_handle: config.app_handle.clone(),
        base_url: Some(&config.base_url),
        api_key: config.api_key.as_deref(),
        model: Some(&config.model),
        temperature: config.temperature,
        reasoning_effort: config.reasoning_effort.as_deref(),
        call_id: None,
    };

    let mut executed_subagents_in_turn: Vec<ExecutedSubagentCall> = Vec::new();
    let mut executed_questions_in_turn: Vec<(String, String, String)> = Vec::new();
    let mut batch_aborted_by_error: Option<String> = None;
    let total_in_batch = turn_tool_calls.len();
    if total_in_batch > 1 {
        for (idx, tc) in turn_tool_calls.iter().enumerate() {
            let batch_info = Some(serde_json::json!({
                "index": idx + 1,
                "total": total_in_batch
            }));
            event_sink(AgentEvent::ToolQueued {
                id: tc.id.clone(),
                name: tc.function.name.clone(),
                arguments: tc.function.arguments.clone(),
                batch: batch_info,
            });
        }
    }

    // Execute each tool call and record response
    for (batch_idx, tc) in turn_tool_calls.iter().enumerate() {
        let batch_info = if total_in_batch > 1 {
            Some(serde_json::json!({
                "index": batch_idx + 1,
                "total": total_in_batch
            }))
        } else {
            None
        };

        if abort_flag.load(Ordering::SeqCst) {
            sanitize_unclosed_tool_calls(messages);
            sync_context_state(&config.app_handle, &config.task_id, turn, messages, local_compaction_rev);
            event_sink(AgentEvent::Aborted);
            bail!("Agent execution aborted by user");
        }

        if let Some(ref failed_tool) = batch_aborted_by_error {
            let skip_msg = format!("Skipped: previous tool '{}' in this batch failed", failed_tool);
            event_sink(AgentEvent::ToolCall {
                id: tc.id.clone(),
                name: tc.function.name.clone(),
                arguments: tc.function.arguments.clone(),
                batch: batch_info.clone(),
            });
            event_sink(AgentEvent::ToolResult {
                id: tc.id.clone(),
                name: tc.function.name.clone(),
                result: skip_msg.clone(),
                success: false,
                duration_ms: Some(0.0),
                batch: batch_info.clone(),
            });
            messages.push(ChatMessage::tool_response(tc.id.clone(), skip_msg));
            continue;
        }

        event_sink(AgentEvent::ToolCall {
            id: tc.id.clone(),
            name: tc.function.name.clone(),
            arguments: tc.function.arguments.clone(),
            batch: batch_info.clone(),
        });

        let parsed_args: serde_json::Value =
            serde_json::from_str(&tc.function.arguments).unwrap_or_else(|_| serde_json::json!({}));

        let norm_name = canonical_tool_name(&tc.function.name);
        let is_sub = norm_name == "subagent" || norm_name == "subagent_bg";

        if norm_name == "question" {
            let qtext = parsed_args
                .get("text")
                .or_else(|| parsed_args.get("question"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if !qtext.is_empty() {
                if let Some((prev_id, _, prev_result)) = executed_questions_in_turn
                    .iter()
                    .find(|(_, prev_text, _)| are_questions_duplicate(prev_text, qtext))
                {
                    eprintln!(
                        "CVH [agent]: Deduplicated question call '{}' in turn {} (matches earlier call '{}')",
                        tc.id, turn, prev_id
                    );
                    event_sink(AgentEvent::ToolResult {
                        id: tc.id.clone(),
                        name: tc.function.name.clone(),
                        result: prev_result.clone(),
                        success: true,
                        duration_ms: Some(0.0),
                        batch: batch_info.clone(),
                    });
                    messages.push(ChatMessage::tool_response(tc.id.clone(), prev_result.clone()));
                    continue;
                }
            }
        }

        if is_sub {
            let role = parsed_args
                .get("persona")
                .or_else(|| parsed_args.get("role"))
                .and_then(|v| v.as_str())
                .unwrap_or("general")
                .trim();
            let prompt = parsed_args
                .get("prompt")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();

            if let Some(prev) = executed_subagents_in_turn.iter().find(|p| {
                p.role.eq_ignore_ascii_case(role) && are_subagent_prompts_duplicate(&p.prompt, prompt)
            }) {
                eprintln!(
                    "CVH [agent]: Deduplicated subagent call '{}' (role: '{}') in turn {} (matches earlier call '{}' with role '{}')",
                    tc.id, role, turn, prev.id, prev.role
                );
                event_sink(AgentEvent::ToolResult {
                    id: tc.id.clone(),
                    name: tc.function.name.clone(),
                    result: prev.result.clone(),
                    success: true,
                    duration_ms: Some(0.0),
                    batch: batch_info.clone(),
                });
                messages.push(ChatMessage::tool_response(tc.id.clone(), prev.result.clone()));
                continue;
            }
        }

        let mut call_tool_ctx = tool_ctx.clone();
        call_tool_ctx.call_id = Some(&tc.id);

        let tool_start = std::time::Instant::now();
        let exec_result = if let Some(cb_msg) = detect_degenerate_command_or_loop(
            &tc.function.name,
            &tc.function.arguments,
            recent_tool_calls,
        ) {
            Err(anyhow::anyhow!("{}", cb_msg))
        } else {
            execute_tool(
                &call_tool_ctx,
                &tc.function.name,
                &parsed_args,
            )
            .await
        };
        recent_tool_calls.push(normalize_tool_call_for_circuit_breaker(
            &tc.function.name,
            &tc.function.arguments,
        ));
        if recent_tool_calls.len() > 16 {
            recent_tool_calls.remove(0);
        }
        let duration_ms = tool_start.elapsed().as_secs_f64() * 1000.0;

        if abort_flag.load(Ordering::SeqCst) {
            event_sink(AgentEvent::ToolResult {
                id: tc.id.clone(),
                name: tc.function.name.clone(),
                result: "Прервано пользователем".to_string(),
                success: false,
                duration_ms: Some(duration_ms),
                batch: batch_info.clone(),
            });
            sanitize_unclosed_tool_calls(messages);
            sync_context_state(&config.app_handle, &config.task_id, turn, messages, local_compaction_rev);
            event_sink(AgentEvent::Aborted);
            bail!("Agent execution aborted by user");
        }

        if tc.function.name == "todo_write" && exec_result.is_ok() {
            if let Some(raw_todos) = parsed_args.get("todos").and_then(|v| v.as_array()) {
                let mut todos_vec = Vec::new();
                for (idx, item) in raw_todos.iter().enumerate() {
                    let content = item
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    if content.is_empty() {
                        continue;
                    }
                    let raw_status = item
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("pending")
                        .trim()
                        .to_lowercase();
                    let status = match raw_status.as_str() {
                        "completed" | "done" => "completed",
                        "in_progress" | "running" => "in_progress",
                        _ => "pending",
                    }
                    .to_string();
                    let id = item
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("todo-{}", idx + 1));
                    todos_vec.push(TodoItem { id, content, status });
                }
                event_sink(AgentEvent::TaskUpdated {
                    task_id: config.task_id.clone(),
                    todos: todos_vec,
                });
            }
        }

        let (result_str, success) = match exec_result {
            Ok(out) => (out, true),
            Err(e) => (format!("Error executing {}: {:#}", tc.function.name, e), false),
        };

        if is_sub && success {
            let role = parsed_args
                .get("persona")
                .or_else(|| parsed_args.get("role"))
                .and_then(|v| v.as_str())
                .unwrap_or("general")
                .trim()
                .to_string();
            let prompt = parsed_args
                .get("prompt")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            executed_subagents_in_turn.push(ExecutedSubagentCall {
                id: tc.id.clone(),
                tool_name: tc.function.name.clone(),
                role,
                prompt,
                result: result_str.clone(),
            });
        }

        if norm_name == "question" && success {
            let qtext = parsed_args
                .get("text")
                .or_else(|| parsed_args.get("question"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if !qtext.is_empty() {
                executed_questions_in_turn.push((tc.id.clone(), qtext, result_str.clone()));
            }
        }

        event_sink(AgentEvent::ToolResult {
            id: tc.id.clone(),
            name: tc.function.name.clone(),
            result: result_str.clone(),
            success,
            duration_ms: Some(duration_ms),
            batch: batch_info.clone(),
        });

        messages.push(ChatMessage::tool_response(tc.id.clone(), result_str.clone()));
        crate::db::save_conversation_context(&config.repo_root, &config.task_id, turn, messages, *context_cursor, abort_flag)
            .await.map_err(anyhow::Error::msg)?;

        if should_abort_tool_batch(&tc.function.name, success, &result_str) {
            batch_aborted_by_error = Some(tc.function.name.clone());
        }

        let norm_name = tc.function.name.trim().to_lowercase();
        let is_stage_transition = matches!(
            norm_name.as_str(),
            "advance_task" | "move_task" | "reject_task" | "move_task_stage" | "advance_stage" | "next_stage"
        ) || canonical_tool_name(&norm_name) == "advance_task";

        if is_stage_transition && success {
            let (steer_appended, user_instruction) = drain_pending_steers(
                config,
                messages,
                context_cursor,
                false,
            )?;
            if steer_appended && user_instruction {
                *completion_nudges = 0;
                sync_context_state(&config.app_handle, &config.task_id, turn, messages, local_compaction_rev);
                crate::db::save_conversation_context(&config.repo_root, &config.task_id, turn, messages, *context_cursor, abort_flag)
                    .await.map_err(anyhow::Error::msg)?;
                continue;
            }
            if let Some(ref flag) = config.advance_aborted {
                flag.store(true, Ordering::SeqCst);
            }
            abort_flag.store(true, Ordering::SeqCst);
            sync_context_state(&config.app_handle, &config.task_id, turn, messages, local_compaction_rev);
            event_sink(AgentEvent::Finished {
                final_response: String::new(),
            });
            return Ok(TurnToolOutcome::Terminated(result_str));
        }
    }

    Ok(TurnToolOutcome::Continue)
}

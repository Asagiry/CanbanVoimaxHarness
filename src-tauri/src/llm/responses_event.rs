//! SSE event handler for OpenCode Responses-API streaming format.

use std::collections::HashMap;
use tokio::sync::mpsc::Sender;

use super::models::StreamChunk;
use super::usage::*;

#[allow(clippy::too_many_arguments)]
pub async fn handle_responses_sse_event(
    ev_type: &str,
    payload: &serde_json::Value,
    tx: &Sender<StreamChunk>,
    tool_calls_in_progress: &mut HashMap<usize, (String, String, String)>,
    next_tool_call_fallback: &mut usize,
    pending_usage: &mut Option<StreamChunk>,
    terminal_marker: &mut bool,
    stream_error: &mut Option<String>,
    streamed_reasoning_delta: &mut bool,
) {
    match ev_type {
        "response.output_text.delta" => {
            if let Some(delta) = payload.get("delta").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                let _ = tx.send(StreamChunk::Token(delta.to_string())).await;
            }
        }
        "response.reasoning_summary_text.delta" | "response.reasoning_text.delta" => {
            if let Some(delta) = payload.get("delta").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                *streamed_reasoning_delta = true;
                let _ = tx.send(StreamChunk::Thought(delta.to_string())).await;
            }
        }
        "response.output_item.added" => {
            if payload.pointer("/item/type").and_then(|v| v.as_str()) == Some("function_call") {
                let call_id = payload
                    .pointer("/item/call_id")
                    .or_else(|| payload.pointer("/item/id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let idx = resolve_tool_call_index(
                    tool_calls_in_progress,
                    payload.get("output_index").and_then(|v| v.as_u64()).map(|v| v as usize),
                    Some(call_id.as_str()),
                    next_tool_call_fallback,
                );
                let name = payload
                    .pointer("/item/name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let entry = tool_calls_in_progress
                    .entry(idx)
                    .or_insert_with(|| (String::new(), String::new(), String::new()));
                if !call_id.is_empty() {
                    entry.0 = call_id;
                }
                if !name.is_empty() {
                    entry.1 = name;
                }
            }
        }
        "response.function_call_arguments.delta" => {
            let idx = resolve_tool_call_index(
                tool_calls_in_progress,
                payload.get("output_index").and_then(|v| v.as_u64()).map(|v| v as usize),
                None,
                next_tool_call_fallback,
            );
            if let Some(delta) = payload.get("delta").and_then(|v| v.as_str()) {
                tool_calls_in_progress
                    .entry(idx)
                    .or_insert_with(|| (String::new(), String::new(), String::new()))
                    .2
                    .push_str(delta);
            }
        }
        "response.output_item.done" => {
            let item_type = payload.pointer("/item/type").and_then(|v| v.as_str());
            if item_type == Some("function_call") {
                let done_call_id = payload
                    .pointer("/item/call_id")
                    .or_else(|| payload.pointer("/item/id"))
                    .and_then(|v| v.as_str());
                let idx = resolve_tool_call_index(
                    tool_calls_in_progress,
                    payload.get("output_index").and_then(|v| v.as_u64()).map(|v| v as usize),
                    done_call_id,
                    next_tool_call_fallback,
                );
                let entry = tool_calls_in_progress
                    .entry(idx)
                    .or_insert_with(|| (String::new(), String::new(), String::new()));
                if entry.0.is_empty() {
                    if let Some(cid) = payload
                        .pointer("/item/call_id")
                        .or_else(|| payload.pointer("/item/id"))
                        .and_then(|v| v.as_str())
                    {
                        entry.0 = cid.to_string();
                    }
                }
                if entry.1.is_empty() {
                    if let Some(name) = payload.pointer("/item/name").and_then(|v| v.as_str()) {
                        entry.1 = name.to_string();
                    }
                }
                if entry.2.is_empty() {
                    if let Some(args) = payload.pointer("/item/arguments").and_then(|v| v.as_str()) {
                        entry.2 = args.to_string();
                    }
                }
            } else if item_type == Some("reasoning") && !*streamed_reasoning_delta {
                if let Some(summary) = payload.pointer("/item/summary").and_then(|v| v.as_array()) {
                    for part in summary {
                        if let Some(text) = part.get("text").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                            let _ = tx.send(StreamChunk::Thought(text.to_string())).await;
                        }
                    }
                }
            }
        }
        "response.completed" => {
            *terminal_marker = true;
            if let Some(usage) = payload.pointer("/response/usage") {
                let input_tokens = extract_openai_input_tokens(usage);
                let output_tokens = extract_openai_output_tokens(usage);
                let cache_read = extract_openai_cache_read(usage);
                let cache_creation = extract_openai_cache_creation(usage);
                *pending_usage = Some(StreamChunk::Usage {
                    input_tokens,
                    output_tokens,
                    cache_read_tokens: cache_read,
                    cache_creation_tokens: cache_creation,
                });
            }
        }
        "response.failed" | "response.incomplete" => {
            let msg = payload
                .pointer("/response/error/message")
                .or_else(|| payload.pointer("/response/incomplete_details/reason"))
                .and_then(|v| v.as_str())
                .unwrap_or(ev_type);
            *stream_error = Some(format!("LLM response error: {msg}"));
        }
        _ => {}
    }
}

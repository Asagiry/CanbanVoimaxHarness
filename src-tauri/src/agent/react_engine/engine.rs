use std::sync::atomic::AtomicBool;
use tokio::sync::mpsc;

use crate::agent::compactor;
use crate::llm::{ChatMessage, StreamChunk};
use super::drivers::AgentSessionDriver;
use super::stream_consumer::{StreamChunkConsumer, StreamConsumerOptions, StreamTurnResult};

/// Generic ReAct Engine parameterized by an `AgentSessionDriver`.
pub struct ReActEngine<D: AgentSessionDriver> {
    pub driver: D,
}

impl<D: AgentSessionDriver> ReActEngine<D> {
    pub fn new(driver: D) -> Self {
        Self { driver }
    }

    pub fn driver(&self) -> &D {
        &self.driver
    }

    pub fn driver_mut(&mut self) -> &mut D {
        &mut self.driver
    }

    pub async fn consume_turn_stream(
        &self,
        rx: &mut mpsc::Receiver<StreamChunk>,
        abort_flag: &AtomicBool,
        options: &StreamConsumerOptions<'_>,
    ) -> StreamTurnResult {
        StreamChunkConsumer::consume(rx, abort_flag, &self.driver, options).await
    }
}

/// Determines whether an LLM stream error is transient (rate limit 429, 5xx overload, or timeout)
/// and therefore eligible for automatic backoff retry or session pause.
pub fn is_transient_llm_error(err: &str) -> bool {
    let low = err.to_ascii_lowercase();
    low.contains("429")
        || low.contains("rate_limit")
        || low.contains("temporarily")
        || low.contains("overloaded")
        || low.contains("capacity")
        || low.contains("timeout")
        || low.contains("500")
        || low.contains("502")
        || low.contains("503")
        || low.contains("504")
}

/// Triggers multi-tier context compaction if `messages` reached or exceeded `context_window`.
pub fn maybe_compact_messages(
    messages: &mut Vec<ChatMessage>,
    context_window: usize,
) -> Option<compactor::CompactionResult> {
    let effective_window = if context_window == 0 { 128_000 } else { context_window };
    if compactor::estimate_tokens(messages) >= effective_window {
        let target_window = (effective_window * 3) / 4;
        Some(compactor::compact_conversation_to_target(messages, target_window, None))
    } else {
        None
    }
}

/// Normalizes a `(tool_name, args_str)` pair for repetitive loop detection.
/// Idempotent: works on both raw JSON arguments and already-normalized command strings.
pub fn normalize_tool_call_for_circuit_breaker(tool_name: &str, args_str: &str) -> (String, String) {
    let canon = crate::agent::canonical_tool_name(tool_name).to_string();
    if canon == "execute_command" {
        let raw_cmd = serde_json::from_str::<serde_json::Value>(args_str)
            .ok()
            .and_then(|v| {
                v.get("command")
                    .or_else(|| v.get("cmd"))
                    .and_then(|c| c.as_str())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| args_str.to_string());
        let stripped = crate::agent::tools::command::strip_any_leading_cd(&raw_cmd);
        let collapsed = stripped.split_whitespace().collect::<Vec<_>>().join(" ");
        (canon, collapsed)
    } else if let Ok(val) = serde_json::from_str::<serde_json::Value>(args_str) {
        (canon, val.to_string())
    } else {
        (canon, args_str.trim().to_string())
    }
}

/// Detects degenerate command patterns (`& echo A & echo B & ...`) or repetitive 3-turn tool loops.
pub fn detect_degenerate_command_or_loop(
    tool_name: &str,
    args_str: &str,
    recent_calls: &[(String, String)],
) -> Option<String> {
    let (canon_name, norm_args) = normalize_tool_call_for_circuit_breaker(tool_name, args_str);

    if canon_name == "execute_command" {
        let low = norm_args.to_ascii_lowercase();
        let echo_segments = low.matches("& echo ").count() + low.matches("&echo ").count();
        let starts_with_echo = low.trim_start().starts_with("echo ");
        if echo_segments >= 4 || (starts_with_echo && echo_segments >= 3) {
            return Some(
                "CircuitBreaker: Command contains degenerate '& echo' chaining. Use native 'grep', 'glob', or 'read_file' tools instead of batched shell findstr/echo chains.".to_string(),
            );
        }
    }

    if recent_calls.len() >= 2 {
        let (p1_raw_name, p1_raw_args) = &recent_calls[recent_calls.len() - 1];
        let (p2_raw_name, p2_raw_args) = &recent_calls[recent_calls.len() - 2];
        let (p1_name, p1_args) = normalize_tool_call_for_circuit_breaker(p1_raw_name, p1_raw_args);
        let (p2_name, p2_args) = normalize_tool_call_for_circuit_breaker(p2_raw_name, p2_raw_args);

        if canon_name == p1_name
            && p1_name == p2_name
            && !norm_args.is_empty()
            && norm_args == p1_args
            && p1_args == p2_args
        {
            return Some(
                "CircuitBreaker: Repetitive tool call detected across 3 consecutive turns. Stop repeating this command; use a different tool ('grep'/'read_file') or synthesize your final response now.".to_string(),
            );
        }

        if canon_name == "execute_command" && p1_name == "execute_command" && p2_name == "execute_command" {
            let first_45 = |s: &str| -> Option<String> {
                let p: String = s.chars().take(45).collect();
                if p.chars().count() >= 45 { Some(p) } else { None }
            };
            if let (Some(c_pref), Some(p1_pref), Some(p2_pref)) =
                (first_45(&norm_args), first_45(&p1_args), first_45(&p2_args))
            {
                if c_pref == p1_pref && p1_pref == p2_pref {
                    return Some(
                        "CircuitBreaker: Repetitive tool call detected across 3 consecutive turns. Stop repeating this command; use a different tool ('grep'/'read_file') or synthesize your final response now.".to_string(),
                    );
                }
            }
        }
    }

    None
}

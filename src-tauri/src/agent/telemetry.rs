//! Telemetry computation utilities for agent turns and subagents.
//! Adheres to ub-mode (Dryer & Zero-Risk Pure Extraction) and Telemetry Accuracy Invariants.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;
use crate::llm::{ChatMessage, Role, ToolCall, ToolDefinition};

/// Snapshot of turn telemetry metrics (cache hit rate and tokens per second).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TurnTelemetry {
    pub cache_hit_rate: f32,
    pub tps: f32,
}

/// Compute safe prompt cache hit rate percentage according to telemetry invariants.
///
/// Guaranteed to never produce NaN, infinite or negative values.
#[inline]
pub fn compute_cache_hit_rate(cache_read_tokens: u64, total_input: u64) -> f32 {
    if total_input > 0 {
        let rate = (cache_read_tokens as f32 / total_input as f32) * 100.0;
        if rate.is_nan() || rate.is_infinite() || rate < 0.0 {
            0.0
        } else {
            rate.min(100.0)
        }
    } else {
        0.0
    }
}

/// Compute active token generation throughput (tok/s).
///
/// Generation time must exceed minimum duration (50ms) and produce at least 1 token.
#[inline]
pub fn compute_tps(duration_secs: f64, output_tokens: u64) -> f32 {
    if duration_secs > 0.05 && output_tokens > 0 {
        let val = (output_tokens as f64 / duration_secs) as f32;
        if val.is_nan() || val.is_infinite() || val < 0.0 {
            0.0
        } else {
            val
        }
    } else {
        0.0
    }
}

/// Convenient helper for f32 duration_secs.
#[inline]
pub fn compute_tps_f32(duration_secs: f32, output_tokens: u64) -> f32 {
    compute_tps(duration_secs as f64, output_tokens)
}

/// Calculates timing metrics (duration_secs, duration_ms, ttft_ms) from turn instants.
#[inline]
pub fn compute_timing_metrics(
    turn_start_instant: Instant,
    first_token_instant: Option<Instant>,
    last_token_instant: Option<Instant>,
) -> (f64, f64, f64) {
    let start_time = first_token_instant.unwrap_or(turn_start_instant);
    let end_time = last_token_instant.unwrap_or_else(Instant::now);
    let duration_secs = end_time.duration_since(start_time).as_secs_f64();
    let duration_ms = duration_secs * 1000.0;
    let ttft_ms = first_token_instant
        .map(|t| t.duration_since(turn_start_instant).as_secs_f64() * 1000.0)
        .unwrap_or(0.0);
    (duration_secs, duration_ms, ttft_ms)
}

/// Unified telemetry calculation for an agent or subagent turn.
#[inline]
pub fn calculate_turn_telemetry(
    total_input: u64,
    cache_read_tokens: u64,
    output_tokens: u64,
    duration_secs: f64,
) -> TurnTelemetry {
    TurnTelemetry {
        cache_hit_rate: compute_cache_hit_rate(cache_read_tokens, total_input),
        tps: compute_tps(duration_secs, output_tokens),
    }
}

/// Appends raw model request and response telemetry to `.cvh/llm_debug.log`
/// Rotates the log to `.cvh/llm_debug.log.1` if size exceeds 10MB.
pub async fn append_llm_debug_log(repo_root: &Path, text: &str) {
    let cvh_dir = repo_root.join(".cvh");
    let _ = tokio::fs::create_dir_all(&cvh_dir).await;
    let log_path = cvh_dir.join("llm_debug.log");
    const MAX_BYTES: u64 = 10 * 1024 * 1024; // 10MB
    if let Ok(meta) = tokio::fs::metadata(&log_path).await {
        if meta.len() >= MAX_BYTES {
            let rot_path = cvh_dir.join("llm_debug.log.1");
            let _ = tokio::fs::remove_file(&rot_path).await;
            let _ = tokio::fs::rename(&log_path, &rot_path).await;
        }
    }
    if let Ok(mut file) = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .await
    {
        use tokio::io::AsyncWriteExt;
        let _ = file.write_all(text.as_bytes()).await;
    }
}

/// Appends formatted turn request metadata and system prompt to `.cvh/llm_debug.log`.
#[allow(clippy::too_many_arguments)]
pub async fn log_turn_request(
    repo_root: &Path,
    session_id: &str,
    turn: usize,
    is_subagent: bool,
    model: &str,
    messages: &[ChatMessage],
    tools: Option<&[ToolDefinition]>,
    tool_names: &[String],
    base_url: Option<&str>,
) {
    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    let req_tokens_est = crate::agent::compactor::estimate_request_tokens(messages, tools);
    let req_chars: usize = messages
        .iter()
        .map(|m| {
            m.content
                .as_ref()
                .map(|c| match c {
                    crate::llm::MessageContent::Text(t) => t.len(),
                    crate::llm::MessageContent::Parts(parts) => parts
                        .iter()
                        .map(|p| match p {
                            crate::llm::ContentPart::Text { text } => text.len(),
                            _ => 0,
                        })
                        .sum(),
                })
                .unwrap_or(0)
        })
        .sum();
    let label = if is_subagent { "Subagent Turn" } else { "Turn" };
    let mut req_log = format!(
        "[{}] [{}] {} {} Request:\nModel: {}\nMessages count: {}\nReq tokens (est): {}\nReq chars: {}\n",
        ts,
        session_id,
        label,
        turn,
        model,
        messages.len(),
        req_tokens_est,
        req_chars
    );
    if let Some(url) = base_url {
        let session_hdr = crate::llm::cache::opencode_session_header(
            url,
            &format!("{}:{}", repo_root.display(), session_id),
        )
        .unwrap_or_else(|| "n/a".to_string());
        req_log.push_str(&format!("Session: {}\nEndpoint: {}\n", session_hdr, url));
    }
    if let Some(first) = messages.first() {
        if first.role == Role::System {
            req_log.push_str(&format!(
                "System prompt:\n{}\n",
                first.content.as_deref().unwrap_or("")
            ));
        }
    }
    req_log.push_str(&format!(
        "Tool calls requested ({}): {:?}\n\n",
        tool_names.len(),
        tool_names
    ));
    append_llm_debug_log(repo_root, &req_log).await;
}

/// Appends formatted turn response (thoughts, content, tool calls) to `.cvh/llm_debug.log`.
pub async fn log_turn_response(
    repo_root: &Path,
    session_id: &str,
    turn: usize,
    is_subagent: bool,
    turn_thoughts: &str,
    turn_content: &str,
    turn_tool_calls: &[ToolCall],
) {
    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    let label = if is_subagent { "Subagent Turn" } else { "Turn" };
    let mut resp_log = format!("[{}] [{}] {} {} Response:\n", ts, session_id, label, turn);
    if !turn_thoughts.trim().is_empty() {
        resp_log.push_str(&format!("Thoughts:\n{}\n", turn_thoughts.trim()));
    }
    if !turn_content.trim().is_empty() {
        resp_log.push_str(&format!("Content:\n{}\n", turn_content.trim()));
    }
    if !turn_tool_calls.is_empty() {
        resp_log.push_str(&format!("Tool calls received ({}):\n", turn_tool_calls.len()));
        for tc in turn_tool_calls {
            resp_log.push_str(&format!(
                "  - {} (id: {}): {}\n",
                tc.function.name, tc.id, tc.function.arguments
            ));
        }
    } else if is_subagent {
        resp_log.push_str("Tool calls received: None (terminal response)\n");
    } else {
        resp_log.push_str("Tool calls received: None (terminal turn)\n");
    }
    resp_log.push_str("\n--------------------------------------------------------------------------------\n\n");
    append_llm_debug_log(repo_root, &resp_log).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_cache_hit_rate_zero() {
        assert_eq!(compute_cache_hit_rate(0, 0), 0.0);
        assert_eq!(compute_cache_hit_rate(100, 0), 0.0);
    }

    #[test]
    fn test_compute_cache_hit_rate_normal() {
        let rate = compute_cache_hit_rate(50, 100);
        assert!((rate - 50.0).abs() < 1e-4);

        let rate_full = compute_cache_hit_rate(100, 100);
        assert!((rate_full - 100.0).abs() < 1e-4);

        let rate_capped = compute_cache_hit_rate(150, 100);
        assert_eq!(rate_capped, 100.0);
    }

    #[test]
    fn test_compute_tps_thresholds() {
        assert_eq!(compute_tps(0.0, 10), 0.0);
        assert_eq!(compute_tps(0.05, 10), 0.0);
        assert_eq!(compute_tps(1.0, 0), 0.0);

        let tps = compute_tps(1.0, 50);
        assert!((tps - 50.0).abs() < 1e-4);

        let tps_fast = compute_tps(0.1, 10);
        assert!((tps_fast - 100.0).abs() < 1e-4);
    }

    #[test]
    fn test_calculate_turn_telemetry() {
        let telem = calculate_turn_telemetry(200, 50, 100, 2.0);
        assert!((telem.cache_hit_rate - 25.0).abs() < 1e-4);
        assert!((telem.tps - 50.0).abs() < 1e-4);
    }

    #[test]
    fn test_compute_timing_metrics() {
        let start = Instant::now();
        let (duration_secs, duration_ms, ttft_ms) = compute_timing_metrics(start, None, None);
        assert!(duration_secs >= 0.0);
        assert!(duration_ms >= 0.0);
        assert_eq!(ttft_ms, 0.0);
    }
}

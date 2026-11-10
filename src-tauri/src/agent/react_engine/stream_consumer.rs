use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;

use crate::agent::{compactor, AgentEvent, ThinkingStreamParser};
use crate::agent::telemetry::{
    calculate_turn_telemetry, compute_timing_metrics, compute_tps, TurnTelemetry,
};
use crate::llm::{StreamChunk, ToolCall};
use super::drivers::AgentSessionDriver;

/// Parameters for consuming a ReAct stream turn.
#[derive(Debug, Clone)]
pub struct StreamConsumerOptions<'a> {
    pub model: &'a str,
    pub turn: usize,
    pub system_tokens: u64,
    pub tool_tokens: u64,
    pub fallback_input_tokens: u64,
    pub log_prefix: &'a str,
    pub subagent_id: Option<&'a str>,
    pub emit_thoughts: bool,
}

impl<'a> StreamConsumerOptions<'a> {
    pub fn new(model: &'a str, turn: usize, fallback_input_tokens: u64) -> Self {
        Self {
            model,
            turn,
            system_tokens: 0,
            tool_tokens: 0,
            fallback_input_tokens,
            log_prefix: "telemetry",
            subagent_id: None,
            emit_thoughts: true,
        }
    }
}

/// Results of a consumed ReAct turn stream.
#[derive(Debug, Clone, Default)]
pub struct StreamTurnResult {
    pub content: String,
    pub thoughts: String,
    pub tool_calls: Vec<ToolCall>,
    pub received_any_chunk: bool,
    pub received_usage: bool,
    pub aborted: bool,
    pub duration_secs: f64,
    pub duration_ms: f64,
    pub ttft_ms: f64,
    pub tps: f32,
    pub cache_hit_rate: f32,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub error: Option<String>,
}

/// Generic consumer for streaming SSE chunks.
pub struct StreamChunkConsumer;

impl StreamChunkConsumer {
    /// Consume all SSE chunks from an active stream receiver, updating telemetry,
    /// parsing thoughts, capturing tool calls, and notifying the session driver.
    pub async fn consume<D: AgentSessionDriver + ?Sized>(
        rx: &mut mpsc::Receiver<StreamChunk>,
        abort_flag: &AtomicBool,
        driver: &D,
        options: &StreamConsumerOptions<'_>,
    ) -> StreamTurnResult {
        let mut turn_start_instant = std::time::Instant::now();
        let mut first_token_instant: Option<std::time::Instant> = None;
        let mut last_token_instant: Option<std::time::Instant> = None;
        let mut turn_streamed_chars: usize = 0;
        let mut received_any_chunk = false;
        let mut received_usage = false;
        let mut turn_content = String::new();
        let mut turn_thoughts = String::new();
        let mut turn_tool_calls: Vec<ToolCall> = Vec::new();
        let mut thinking_parser = ThinkingStreamParser::new();
        let mut turn_error: Option<String> = None;
        let mut is_aborted = false;

        let mut final_duration_secs = 0.0;
        let mut final_duration_ms = 0.0;
        let mut final_ttft_ms = 0.0;
        let mut final_tps = 0.0f32;
        let mut final_cache_hit_rate = 0.0f32;
        let mut final_input_tokens = 0u64;
        let mut final_output_tokens = 0u64;

        loop {
            if abort_flag.load(Ordering::SeqCst) {
                rx.close();
                is_aborted = true;
                break;
            }
            let chunk = tokio::select! {
                chunk = rx.recv() => match chunk {
                    Some(chunk) => chunk,
                    None => break,
                },
                _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => continue,
            };
            received_any_chunk = true;

            match chunk {
                StreamChunk::AttemptStart { id } => {
                    turn_start_instant = std::time::Instant::now();
                    first_token_instant = None;
                    last_token_instant = None;
                    turn_streamed_chars = 0;
                    received_any_chunk = false;
                    received_usage = false;
                    turn_content.clear();
                    turn_thoughts.clear();
                    turn_tool_calls.clear();
                    thinking_parser = ThinkingStreamParser::new();
                    final_duration_secs = 0.0;
                    final_duration_ms = 0.0;
                    final_ttft_ms = 0.0;
                    final_tps = 0.0;
                    final_cache_hit_rate = 0.0;
                    final_input_tokens = 0;
                    final_output_tokens = 0;
                    driver.emit_event(AgentEvent::StreamStart { id });
                }
                StreamChunk::Retry { id, delay_secs } => {
                    turn_content.clear();
                    turn_thoughts.clear();
                    turn_tool_calls.clear();
                    thinking_parser = ThinkingStreamParser::new();
                    let root = driver.repo_root().to_path_buf();
                    let dialog_key = driver.dialog_key().to_string();
                    let stream_id = id.clone();
                    let discarded = tokio::task::spawn_blocking(move || {
                        crate::db::discard_stream_attempt(&root, &dialog_key, &stream_id)
                    }).await.unwrap_or_else(|e| Err(e.to_string()));
                    if let Err(err) = discarded {
                        let err = format!("Failed to discard interrupted response: {err}");
                        driver.emit_event(AgentEvent::Error(err.clone()));
                        turn_error = Some(err);
                        rx.close();
                        break;
                    }
                    driver.emit_event(AgentEvent::StreamRetry { id, delay_secs });
                }
                StreamChunk::FirstChunk => {
                    if first_token_instant.is_none() {
                        first_token_instant = Some(std::time::Instant::now());
                    }
                    last_token_instant = Some(std::time::Instant::now());
                }
                StreamChunk::Token(token) => {
                    if first_token_instant.is_none() {
                        first_token_instant = Some(std::time::Instant::now());
                    }
                    last_token_instant = Some(std::time::Instant::now());
                    turn_streamed_chars += token.len();

                    let parsed = thinking_parser.push(&token);
                    for (is_thought, text) in parsed {
                        if is_thought {
                            turn_thoughts.push_str(&text);
                            if options.emit_thoughts {
                                driver.emit_event(AgentEvent::Thought(text));
                            }
                        } else {
                            turn_content.push_str(&text);
                            driver.emit_event(AgentEvent::Token(text));
                        }
                    }
                }
                StreamChunk::Output(output) => {
                    if first_token_instant.is_none() {
                        first_token_instant = Some(std::time::Instant::now());
                    }
                    last_token_instant = Some(std::time::Instant::now());
                    turn_streamed_chars += output.len();
                    turn_content.push_str(&output);
                    driver.emit_event(AgentEvent::Output(output));
                }
                StreamChunk::Thought(thought) => {
                    if first_token_instant.is_none() {
                        first_token_instant = Some(std::time::Instant::now());
                    }
                    last_token_instant = Some(std::time::Instant::now());
                    turn_streamed_chars += thought.len();
                    turn_thoughts.push_str(&thought);
                    if options.emit_thoughts {
                        driver.emit_event(AgentEvent::Thought(thought));
                    }
                }
                StreamChunk::ToolCall { id, name, arguments } => {
                    if first_token_instant.is_none() {
                        first_token_instant = Some(std::time::Instant::now());
                    }
                    last_token_instant = Some(std::time::Instant::now());
                    turn_streamed_chars += name.len() + arguments.len();
                    turn_tool_calls.push(ToolCall {
                        id,
                        call_type: "function".to_string(),
                        function: crate::llm::FunctionCall { name, arguments },
                    });
                }
                StreamChunk::Usage {
                    input_tokens,
                    output_tokens,
                    cache_read_tokens,
                    cache_creation_tokens,
                } => {
                    received_usage = true;
                    let total_input = input_tokens;
                    let (duration_secs, duration_ms, ttft_ms) =
                        compute_timing_metrics(turn_start_instant, first_token_instant, last_token_instant);
                    let TurnTelemetry { cache_hit_rate, tps } =
                        calculate_turn_telemetry(total_input, cache_read_tokens, output_tokens, duration_secs);

                    final_duration_secs = duration_secs;
                    final_duration_ms = duration_ms;
                    final_ttft_ms = ttft_ms;
                    final_tps = tps;
                    final_cache_hit_rate = cache_hit_rate;
                    final_input_tokens = input_tokens;
                    final_output_tokens = output_tokens;

                    if let Some(sid) = options.subagent_id {
                        eprintln!(
                            "CVH [{}] turn={} subagent={} input={} output={} cache_read={} hit_rate={:.1}% duration={:.2}s tps={:.1} ttft={:.1}ms",
                            options.log_prefix,
                            options.turn,
                            sid,
                            input_tokens,
                            output_tokens,
                            cache_read_tokens,
                            cache_hit_rate,
                            duration_secs,
                            tps,
                            ttft_ms
                        );
                    } else {
                        eprintln!(
                            "CVH [{}] turn={} model={} input={} output={} cache_read={} hit_rate={:.1}% duration={:.2}s tps={:.1} ttft={:.1}ms",
                            options.log_prefix,
                            options.turn,
                            options.model,
                            input_tokens,
                            output_tokens,
                            cache_read_tokens,
                            cache_hit_rate,
                            duration_secs,
                            tps,
                            ttft_ms
                        );
                    }

                    driver.emit_event(AgentEvent::Telemetry {
                        input_tokens,
                        output_tokens,
                        cache_read_tokens,
                        cache_creation_tokens,
                        cache_hit_rate,
                        duration_ms,
                        tps,
                        ttft_ms,
                        system_tokens: options.system_tokens,
                        tool_tokens: options.tool_tokens,
                    });
                }
                StreamChunk::Done => break,
                StreamChunk::Error(err) => {
                    eprintln!("CVH [{}]: LLM stream error: {}", options.log_prefix, err);
                    driver.emit_event(AgentEvent::Error(err.clone()));
                    turn_error = Some(err);
                    break;
                }
            }
        }

        // Flush any remaining characters from the thinking parser
        let flushed = thinking_parser.flush();
        for (is_thought, text) in flushed {
            if is_thought {
                turn_thoughts.push_str(&text);
                if options.emit_thoughts {
                    driver.emit_event(AgentEvent::Thought(text));
                }
            } else {
                turn_content.push_str(&text);
                driver.emit_event(AgentEvent::Token(text));
            }
        }

        // Fallback telemetry if provider did not emit a usage chunk:
        if !received_usage && received_any_chunk && !is_aborted && turn_error.is_none() {
            let (duration_secs, duration_ms, ttft_ms) =
                compute_timing_metrics(turn_start_instant, first_token_instant, last_token_instant);
            let output_tokens = (compactor::estimate_text_tokens(&turn_content) as u64)
                .max((turn_streamed_chars as f64 / 3.8).ceil() as u64);
            let input_tokens = options.fallback_input_tokens;
            let tps = compute_tps(duration_secs, output_tokens);

            final_duration_secs = duration_secs;
            final_duration_ms = duration_ms;
            final_ttft_ms = ttft_ms;
            final_tps = tps;
            final_cache_hit_rate = 0.0;
            final_input_tokens = input_tokens;
            final_output_tokens = output_tokens;

            if let Some(sid) = options.subagent_id {
                eprintln!(
                    "CVH [{}:fallback] turn={} subagent={} input={} output={} duration={:.2}s tps={:.1} ttft={:.1}ms",
                    options.log_prefix,
                    options.turn,
                    sid,
                    input_tokens,
                    output_tokens,
                    duration_secs,
                    tps,
                    ttft_ms
                );
            } else {
                eprintln!(
                    "CVH [{}:fallback] turn={} model={} input={} output={} duration={:.2}s tps={:.1} ttft={:.1}ms",
                    options.log_prefix,
                    options.turn,
                    options.model,
                    input_tokens,
                    output_tokens,
                    duration_secs,
                    tps,
                    ttft_ms
                );
            }

            driver.emit_event(AgentEvent::Telemetry {
                input_tokens,
                output_tokens,
                cache_read_tokens: 0,
                cache_creation_tokens: 0,
                cache_hit_rate: 0.0,
                duration_ms,
                tps,
                ttft_ms,
                system_tokens: options.system_tokens,
                tool_tokens: options.tool_tokens,
            });
        }

        StreamTurnResult {
            content: turn_content,
            thoughts: turn_thoughts,
            tool_calls: turn_tool_calls,
            received_any_chunk,
            received_usage,
            aborted: is_aborted,
            duration_secs: final_duration_secs,
            duration_ms: final_duration_ms,
            ttft_ms: final_ttft_ms,
            tps: final_tps,
            cache_hit_rate: final_cache_hit_rate,
            input_tokens: final_input_tokens,
            output_tokens: final_output_tokens,
            error: turn_error,
        }
    }
}

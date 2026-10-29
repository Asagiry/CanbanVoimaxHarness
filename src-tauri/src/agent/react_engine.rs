//! Generic ReAct Engine and Stream Chunk Consumer for agentic execution.
//!
//! Eliminates duplication between root agent loops and in-process subagents,
//! providing a unified SSE streaming pipeline, telemetry calculation (TTFT, TPS,
//! Cache Hit Rate), abort checking, and SQLite persistence.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::agent::{compactor, sync_context_state, AgentEvent, ThinkingStreamParser};
use crate::agent::telemetry::{
    calculate_turn_telemetry, compute_timing_metrics, compute_tps, TurnTelemetry,
};
use crate::db::ActivityItem;
use crate::llm::{ChatMessage, StreamChunk, ToolCall};

/// Session driver contract for agent execution environments.
pub trait AgentSessionDriver: Send + Sync {
    /// Emit an agent lifecycle event.
    fn emit_event(&self, event: AgentEvent);

    /// Dialogue key or SQLite session ID for persistence.
    fn dialog_key(&self) -> &str;

    /// Worktree directory root path.
    fn worktree_root(&self) -> &Path;

    /// Callback invoked when a turn has finished.
    fn on_turn_finished(&self, turn: usize, messages: &[ChatMessage]);

    /// Repository root path (defaults to worktree_root).
    fn repo_root(&self) -> &Path {
        self.worktree_root()
    }

    /// Task ID (defaults to dialog_key).
    fn task_id(&self) -> &str {
        self.dialog_key()
    }
}

/// Session driver for root agent loop (`run_agent_loop`).
pub struct MainAgentDriver {
    pub task_id: String,
    pub worktree_path: PathBuf,
    pub repo_root: PathBuf,
    pub app_handle: Option<AppHandle>,
    pub event_sink: Mutex<Box<dyn FnMut(AgentEvent) + Send>>,
}

impl MainAgentDriver {
    pub fn new<F>(
        task_id: String,
        worktree_path: PathBuf,
        repo_root: PathBuf,
        app_handle: Option<AppHandle>,
        event_sink: F,
    ) -> Self
    where
        F: FnMut(AgentEvent) + Send + 'static,
    {
        Self {
            task_id,
            worktree_path,
            repo_root,
            app_handle,
            event_sink: Mutex::new(Box::new(event_sink)),
        }
    }
}

impl AgentSessionDriver for MainAgentDriver {
    fn emit_event(&self, event: AgentEvent) {
        if let Ok(mut sink) = self.event_sink.lock() {
            sink(event);
        }
    }

    fn dialog_key(&self) -> &str {
        &self.task_id
    }

    fn worktree_root(&self) -> &Path {
        &self.worktree_path
    }

    fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    fn on_turn_finished(&self, turn: usize, messages: &[ChatMessage]) {
        sync_context_state(&self.app_handle, &self.task_id, turn, messages);
    }
}

/// Configuration parameters for constructing a `SubagentDriver`.
#[derive(Debug, Clone)]
pub struct SubagentDriverConfig {
    pub parent_task_id: String,
    pub subagent_id: String,
    pub subagent_dialog_id: String,
    pub tool_call_id: Option<String>,
    pub repo_root: PathBuf,
    pub worktree_dir: PathBuf,
    pub model: String,
    pub app_handle: Option<AppHandle>,
}

/// Session driver for in-process subagents (`run_inprocess_subagent`).
pub struct SubagentDriver {
    pub parent_task_id: String,
    pub subagent_id: String,
    pub subagent_dialog_id: String,
    pub tool_call_id: Option<String>,
    pub repo_root: PathBuf,
    pub worktree_dir: PathBuf,
    pub model: String,
    pub app_handle: Option<AppHandle>,
    pub turn_thought_id: Mutex<String>,
    pub thought_start: Mutex<Option<std::time::Instant>>,
    pub thought_end: Mutex<Option<std::time::Instant>>,
    pub accumulated_thoughts: Mutex<String>,
}

impl SubagentDriver {
    pub fn new(cfg: SubagentDriverConfig) -> Self {
        Self {
            parent_task_id: cfg.parent_task_id,
            subagent_id: cfg.subagent_id,
            subagent_dialog_id: cfg.subagent_dialog_id,
            tool_call_id: cfg.tool_call_id,
            repo_root: cfg.repo_root,
            worktree_dir: cfg.worktree_dir,
            model: cfg.model,
            app_handle: cfg.app_handle,
            turn_thought_id: Mutex::new(String::new()),
            thought_start: Mutex::new(None),
            thought_end: Mutex::new(None),
            accumulated_thoughts: Mutex::new(String::new()),
        }
    }

    pub fn reset_turn_thoughts(&self, thought_id: String) {
        *self.turn_thought_id.lock().unwrap() = thought_id;
        *self.thought_start.lock().unwrap() = None;
        *self.thought_end.lock().unwrap() = None;
        self.accumulated_thoughts.lock().unwrap().clear();
    }

    pub fn finalize_turn_thoughts(&self) {
        let thoughts = self.accumulated_thoughts.lock().unwrap().clone();
        if !thoughts.is_empty() {
            let start = *self.thought_start.lock().unwrap();
            let end = *self.thought_end.lock().unwrap();
            let duration_ms = start.zip(end).map(|(s, e)| e.duration_since(s).as_secs_f64() * 1000.0);
            let thought_id = self.turn_thought_id.lock().unwrap().clone();
            let th_item = ActivityItem {
                id: thought_id,
                item_type: "thought".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                content: Some(thoughts),
                tool: None,
                duration_ms,
                images: None,
            };
            let _ = crate::db::append_dialog_items(&self.repo_root, &self.subagent_dialog_id, std::slice::from_ref(&th_item));
            if let Some(ref app) = self.app_handle {
                let _ = app.emit("subagent:activity", serde_json::json!({
                    "parentTaskId": &self.parent_task_id,
                    "subagentId": &self.subagent_id,
                    "activity": &th_item,
                }));
            }
        }
    }
}

impl AgentSessionDriver for SubagentDriver {
    fn emit_event(&self, event: AgentEvent) {
        match event {
            AgentEvent::Thought(text) => {
                let mut start_guard = self.thought_start.lock().unwrap();
                if start_guard.is_none() {
                    *start_guard = Some(std::time::Instant::now());
                }
                drop(start_guard);
                *self.thought_end.lock().unwrap() = Some(std::time::Instant::now());

                let mut thoughts_guard = self.accumulated_thoughts.lock().unwrap();
                thoughts_guard.push_str(&text);
                let current_thoughts = thoughts_guard.clone();
                drop(thoughts_guard);

                let thought_id = self.turn_thought_id.lock().unwrap().clone();
                let th_item = ActivityItem {
                    id: thought_id,
                    item_type: "thought".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    content: Some(current_thoughts),
                    tool: None,
                    duration_ms: None,
                    images: None,
                };
                let _ = crate::db::append_dialog_items(&self.repo_root, &self.subagent_dialog_id, std::slice::from_ref(&th_item));
                if let Some(ref app) = self.app_handle {
                    let _ = app.emit("subagent:activity", serde_json::json!({
                        "parentTaskId": &self.parent_task_id,
                        "subagentId": &self.subagent_id,
                        "activity": &th_item,
                    }));
                }
            }
            AgentEvent::Telemetry {
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cache_creation_tokens,
                cache_hit_rate,
                duration_ms,
                tps,
                ttft_ms,
                system_tokens,
                tool_tokens,
            } => {
                if let Some(ref app) = self.app_handle {
                    let _ = app.emit(
                        "agent:telemetry",
                        serde_json::json!({
                            "taskId": &self.subagent_id,
                            "inputTokens": input_tokens,
                            "outputTokens": output_tokens,
                            "cacheReadTokens": cache_read_tokens,
                            "cacheCreationTokens": cache_creation_tokens,
                            "cacheHitRate": cache_hit_rate,
                            "durationMs": duration_ms,
                            "tps": tps,
                            "ttftMs": ttft_ms,
                            "systemTokens": system_tokens,
                            "toolTokens": tool_tokens,
                        }),
                    );
                    if let Some(ref cid) = self.tool_call_id {
                        if cid != &self.subagent_id {
                            let _ = app.emit(
                                "agent:telemetry",
                                serde_json::json!({
                                    "taskId": cid,
                                    "inputTokens": input_tokens,
                                    "outputTokens": output_tokens,
                                    "cacheReadTokens": cache_read_tokens,
                                    "cacheCreationTokens": cache_creation_tokens,
                                    "cacheHitRate": cache_hit_rate,
                                    "durationMs": duration_ms,
                                    "tps": tps,
                                    "ttftMs": ttft_ms,
                                    "systemTokens": system_tokens,
                                    "toolTokens": tool_tokens,
                                }),
                            );
                        }
                    }
                }
                let telem_record = crate::db::TelemetryRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    task_id: self.subagent_id.clone(),
                    model: Some(self.model.clone()),
                    input_tokens: input_tokens as i64,
                    output_tokens: output_tokens as i64,
                    cached_tokens: cache_read_tokens as i64,
                    tps: tps as f64,
                    ttft_ms,
                    duration_ms,
                    created_at: chrono::Utc::now().timestamp_millis(),
                };
                let _ = crate::db::db_record_telemetry(&self.repo_root, &telem_record);
            }
            AgentEvent::Error(ref err) => {
                eprintln!("CVH [subagent]: error: {}", err);
            }
            _ => {}
        }
    }

    fn dialog_key(&self) -> &str {
        &self.subagent_dialog_id
    }

    fn worktree_root(&self) -> &Path {
        &self.worktree_dir
    }

    fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    fn task_id(&self) -> &str {
        &self.subagent_id
    }

    fn on_turn_finished(&self, _turn: usize, _messages: &[ChatMessage]) {
        self.finalize_turn_thoughts();
    }
}

/// Parameters for consuming a ReAct stream turn.
#[derive(Debug, Clone)]
pub struct StreamConsumerOptions<'a> {
    pub model: &'a str,
    pub turn: usize,
    pub system_tokens: u64,
    pub tool_tokens: u64,
    pub fallback_input_tokens: u64,
    pub prefix_hash: Option<u64>,
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
            prefix_hash: None,
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
        let turn_start_instant = std::time::Instant::now();
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

        while let Some(chunk) = rx.recv().await {
            received_any_chunk = true;
            if abort_flag.load(Ordering::SeqCst) {
                is_aborted = true;
                break;
            }

            match chunk {
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
                    driver.emit_event(AgentEvent::ToolCall {
                        id: id.clone(),
                        name: name.clone(),
                        arguments: arguments.clone(),
                    });
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
                    let total_input = input_tokens + cache_read_tokens;
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
                    } else if let Some(prefix_hash) = options.prefix_hash {
                        eprintln!(
                            "CVH [{}] turn={} model={} input={} output={} cache_read={} cache_creation={} hit_rate={:.1}% duration={:.2}s tps={:.1} ttft={:.1}ms prefix_hash={:016x}",
                            options.log_prefix,
                            options.turn,
                            options.model,
                            input_tokens,
                            output_tokens,
                            cache_read_tokens,
                            cache_creation_tokens,
                            cache_hit_rate,
                            duration_secs,
                            tps,
                            ttft_ms,
                            prefix_hash
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    struct TestDriver {
        events: Mutex<Vec<AgentEvent>>,
        dialog_key: String,
        worktree_root: PathBuf,
    }

    impl TestDriver {
        fn new() -> Self {
            Self {
                events: Mutex::new(Vec::new()),
                dialog_key: "task-test-1".to_string(),
                worktree_root: PathBuf::from("/tmp/test"),
            }
        }
    }

    impl AgentSessionDriver for TestDriver {
        fn emit_event(&self, event: AgentEvent) {
            self.events.lock().unwrap().push(event);
        }

        fn dialog_key(&self) -> &str {
            &self.dialog_key
        }

        fn worktree_root(&self) -> &Path {
            &self.worktree_root
        }

        fn on_turn_finished(&self, _turn: usize, _messages: &[ChatMessage]) {}
    }

    #[tokio::test]
    async fn test_stream_consumer_output_and_thoughts() {
        let driver = TestDriver::new();
        let engine = ReActEngine::new(driver);
        let abort_flag = AtomicBool::new(false);
        let (tx, mut rx) = mpsc::channel(10);

        tokio::spawn(async move {
            let _ = tx.send(StreamChunk::FirstChunk).await;
            let _ = tx.send(StreamChunk::Token("<thinking>Thinking about it</thinking>Final answer".to_string())).await;
            let _ = tx.send(StreamChunk::Usage {
                input_tokens: 100,
                output_tokens: 20,
                cache_read_tokens: 40,
                cache_creation_tokens: 0,
            }).await;
            let _ = tx.send(StreamChunk::Done).await;
        });

        let opts = StreamConsumerOptions {
            model: "test-model",
            turn: 1,
            system_tokens: 50,
            tool_tokens: 10,
            fallback_input_tokens: 140,
            prefix_hash: None,
            log_prefix: "test",
            subagent_id: None,
            emit_thoughts: true,
        };

        let res = engine.consume_turn_stream(&mut rx, &abort_flag, &opts).await;

        assert_eq!(res.content, "Final answer");
        assert_eq!(res.thoughts, "Thinking about it");
        assert!(res.received_any_chunk);
        assert!(res.received_usage);
        assert!(!res.aborted);
        assert!(res.cache_hit_rate > 0.0);

        let events = engine.driver.events.lock().unwrap();
        assert!(events.iter().any(|e| matches!(e, AgentEvent::Thought(_))));
        assert!(events.iter().any(|e| matches!(e, AgentEvent::Token(_))));
        assert!(events.iter().any(|e| matches!(e, AgentEvent::Telemetry { .. })));
    }

    #[tokio::test]
    async fn test_stream_consumer_abort() {
        let driver = TestDriver::new();
        let abort_flag = AtomicBool::new(true);
        let (tx, mut rx) = mpsc::channel(10);

        tokio::spawn(async move {
            let _ = tx.send(StreamChunk::Token("Ignored".to_string())).await;
        });

        let opts = StreamConsumerOptions {
            model: "test-model",
            turn: 1,
            system_tokens: 50,
            tool_tokens: 10,
            fallback_input_tokens: 140,
            prefix_hash: None,
            log_prefix: "test",
            subagent_id: None,
            emit_thoughts: true,
        };

        let res = StreamChunkConsumer::consume(&mut rx, &abort_flag, &driver, &opts).await;
        assert!(res.aborted);
    }

    #[tokio::test]
    async fn test_stream_consumer_tool_calls() {
        let driver = TestDriver::new();
        let abort_flag = AtomicBool::new(false);
        let (tx, mut rx) = mpsc::channel(10);

        tokio::spawn(async move {
            let _ = tx.send(StreamChunk::ToolCall {
                id: "call-1".to_string(),
                name: "read_file".to_string(),
                arguments: "{\"file_path\": \"src/main.rs\"}".to_string(),
            }).await;
            let _ = tx.send(StreamChunk::Done).await;
        });

        let opts = StreamConsumerOptions {
            model: "test-model",
            turn: 1,
            system_tokens: 50,
            tool_tokens: 10,
            fallback_input_tokens: 140,
            prefix_hash: None,
            log_prefix: "test",
            subagent_id: None,
            emit_thoughts: true,
        };

        let res = StreamChunkConsumer::consume(&mut rx, &abort_flag, &driver, &opts).await;
        assert_eq!(res.tool_calls.len(), 1);
        assert_eq!(res.tool_calls[0].function.name, "read_file");
        assert_eq!(res.tool_calls[0].id, "call-1");
    }

    #[tokio::test]
    async fn test_main_agent_driver() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let ev_clone = Arc::clone(&events);
        let driver = MainAgentDriver::new(
            "task-42".to_string(),
            PathBuf::from("/worktree"),
            PathBuf::from("/repo"),
            None,
            move |ev| {
                ev_clone.lock().unwrap().push(ev);
            },
        );

        driver.emit_event(AgentEvent::Token("hello".to_string()));
        assert_eq!(events.lock().unwrap().len(), 1);
        assert_eq!(driver.task_id(), "task-42");
        assert_eq!(driver.worktree_root(), Path::new("/worktree"));
        assert_eq!(driver.repo_root(), Path::new("/repo"));
    }
}

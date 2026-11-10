use super::drivers::*;
use super::stream_consumer::*;
use super::engine::*;
use crate::agent::AgentEvent;
use crate::llm::{ChatMessage, StreamChunk};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

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
async fn retry_resets_partial_response_and_keeps_attempt_usage() {
    let driver = TestDriver {
        worktree_root: PathBuf::from("target").join(format!("cvh_consumer_retry_{}", uuid::Uuid::new_v4())),
        ..TestDriver::new()
    };
    let (tx, mut rx) = mpsc::channel(20);
    for chunk in [
        StreamChunk::AttemptStart { id: "failed".into() },
        StreamChunk::Token("discard this<think".into()),
        StreamChunk::Thought("discard reasoning".into()),
        StreamChunk::Usage { input_tokens: 100, output_tokens: 10, cache_read_tokens: 80, cache_creation_tokens: 0 },
        StreamChunk::Retry { id: "failed".into(), delay_secs: 5 },
        StreamChunk::AttemptStart { id: "success".into() },
        StreamChunk::Token("answer".into()),
        StreamChunk::ToolCall { id: "tool".into(), name: "read_file".into(), arguments: "{}".into() },
        StreamChunk::Usage { input_tokens: 100, output_tokens: 5, cache_read_tokens: 90, cache_creation_tokens: 0 },
        StreamChunk::Done,
    ] {
        tx.send(chunk).await.unwrap();
    }
    let result = StreamChunkConsumer::consume(&mut rx, &AtomicBool::new(false), &driver, &StreamConsumerOptions::new("test", 1, 100)).await;
    assert_eq!(result.content, "answer");
    assert!(result.thoughts.is_empty());
    assert_eq!(result.tool_calls.len(), 1);
    assert_eq!(result.output_tokens, 5);
    assert!(result.error.is_none());
    let events = driver.events.lock().unwrap();
    assert_eq!(events.iter().filter(|event| matches!(event, AgentEvent::Telemetry { .. })).count(), 2);
    assert!(events.iter().any(|event| matches!(event, AgentEvent::StreamRetry { id, .. } if id == "failed")));
    assert!(!events.iter().any(|event| matches!(event, AgentEvent::Error(_))));
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

#[test]
fn test_detect_degenerate_command_or_loop() {
    // 1. Degenerate echo chaining
    let res = detect_degenerate_command_or_loop(
        "execute_command",
        r#"{"command": "cd /d C:\\worktrees\\CVH-057 && findstr foo bar.rs & echo A & echo B & echo C & echo D"}"#,
        &[],
    );
    assert!(res.is_some());
    assert!(res.unwrap().contains("CircuitBreaker: Command contains degenerate '& echo' chaining"));

    // 2. Normal command is allowed
    assert!(detect_degenerate_command_or_loop(
        "execute_command",
        r#"{"command": "cargo check"}"#,
        &[],
    ).is_none());

    // 3. 3 identical tool calls in a row trigger circuit breaker
    let recent = vec![
        ("read_file".to_string(), r#"{"file_path":"src/main.rs"}"#.to_string()),
        ("read_file".to_string(), r#"{"file_path":"src/main.rs"}"#.to_string()),
    ];
    let res_rep = detect_degenerate_command_or_loop(
        "read_file",
        r#"{"file_path":"src/main.rs"}"#,
        &recent,
    );
    assert!(res_rep.is_some());
    assert!(res_rep.unwrap().contains("Repetitive tool call detected"));

    // 4. 3 execute_command calls sharing first 45 chars after stripping cd /d
    let cmd1 = r#"{"command": "cd /d C:\\wt && findstr /n /c:\"some_very_long_search_pattern_here_12345\" file1.rs"}"#;
    let cmd2 = r#"{"command": "cd /d C:\\wt && findstr /n /c:\"some_very_long_search_pattern_here_12345\" file2.rs"}"#;
    let cmd3 = r#"{"command": "cd /d C:\\wt && findstr /n /c:\"some_very_long_search_pattern_here_12345\" file3.rs"}"#;
    let recent_cmds = vec![
        normalize_tool_call_for_circuit_breaker("execute_command", cmd1),
        normalize_tool_call_for_circuit_breaker("execute_command", cmd2),
    ];
    let res_prefix = detect_degenerate_command_or_loop("execute_command", cmd3, &recent_cmds);
    assert!(res_prefix.is_some());
}

#[test]
fn test_is_transient_llm_error() {
    assert!(is_transient_llm_error("HTTP 429 rate_limit exceeded"));
    assert!(is_transient_llm_error("503 Service Unavailable - overloaded"));
    assert!(!is_transient_llm_error("HTTP 400 Invalid JSON schema"));
}

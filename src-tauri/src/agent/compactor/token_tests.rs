//! Token Estimation & Tiered Compactor Tests (§ 2.22)

use super::*;
use crate::llm::{ChatMessage, FunctionCall, OpenAITool, Role, ToolCall};

#[test]
fn test_multilingual_token_estimation() {
    assert_eq!(estimate_text_tokens(""), 0);

    // ASCII text: ~3.8 chars per token
    let ascii_text = "Hello world! This is a simple test.";
    let ascii_tokens = estimate_text_tokens(ascii_text);
    assert!((8..=12).contains(&ascii_tokens), "ascii_tokens={}", ascii_tokens);

    // Russian (Cyrillic) text: ~1.7 chars per token
    let cyrillic_text = "Привет мир! Это проверка токенизации.";
    let cyrillic_tokens = estimate_text_tokens(cyrillic_text);
    assert!((18..=25).contains(&cyrillic_tokens), "cyrillic_tokens={}", cyrillic_tokens);

    // Verify Cyrillic yields substantially more tokens than plain ASCII of same length
    let same_len_ascii = "a".repeat(cyrillic_text.chars().count());
    let ascii_same_tokens = estimate_text_tokens(&same_len_ascii);
    assert!(cyrillic_tokens > ascii_same_tokens * 3 / 2, "Cyrillic must produce ~1.5-2x more tokens than ASCII");
}

#[test]
fn test_estimate_request_tokens_includes_tools() {
    let messages = vec![
        ChatMessage::system("You are a helpful coding assistant."),
        ChatMessage::user("Привет, помоги починить баг!"),
    ];

    let msg_tokens = estimate_tokens(&messages);
    let request_tokens_no_tools = estimate_request_tokens(&messages, None);
    assert_eq!(msg_tokens, request_tokens_no_tools);

    // Create tool definitions
    let tools = vec![
        OpenAITool::function(
            "read_file",
            "Reads a UTF-8 text file from the workspace",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Relative path to file" },
                    "offset": { "type": "integer" },
                    "limit": { "type": "integer" }
                },
                "required": ["file_path"]
            }),
        ),
        OpenAITool::function(
            "execute_command",
            "Executes a terminal command and returns stdout/stderr",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Shell command to run" },
                    "timeout_seconds": { "type": "integer" }
                },
                "required": ["command"]
            }),
        ),
    ];

    let request_tokens_with_tools = estimate_request_tokens(&messages, Some(&tools));
    assert!(
        request_tokens_with_tools > msg_tokens + 100,
        "Request tokens must include tool schema sizes! got: {}, msg_tokens: {}",
        request_tokens_with_tools,
        msg_tokens
    );
}

#[test]
fn test_multi_tier_compaction_guarantees_target_window() {
    let system_prompt = "You are a senior systems engineer.".to_string();
    let initial_user_spec = "Fix the multi-tier compaction bug with hard ceiling guarantee.".to_string();

    let mut messages = vec![
        ChatMessage::system(system_prompt.clone()),
        ChatMessage::user(initial_user_spec.clone()),
    ];

    // Turn 1: 500,000 tokens (~1,900,000 chars)
    messages.push(ChatMessage::assistant(
        Some("Running extensive analysis".to_string()),
        Some(vec![ToolCall {
            id: "call_t1".to_string(),
            call_type: "function".to_string(),
            function: FunctionCall {
                name: "execute_command".to_string(),
                arguments: "{\"command\": \"cargo check --verbose\"}".to_string(),
            },
        }]),
    ));
    messages.push(ChatMessage::tool_response(
        "call_t1",
        format!("[Exit Code: 0]\n{}", "x".repeat(1_900_000)),
    ));

    // Turn 2: 700,000 tokens (~2,660,000 chars)
    messages.push(ChatMessage::assistant(
        Some("Reading gigantic log file".to_string()),
        Some(vec![ToolCall {
            id: "call_t2".to_string(),
            call_type: "function".to_string(),
            function: FunctionCall {
                name: "read_file".to_string(),
                arguments: "{\"file_path\": \"huge.log\"}".to_string(),
            },
        }]),
    ));
    messages.push(ChatMessage::tool_response(
        "call_t2",
        format!("[Exit Code: 0]\n{}", "y".repeat(2_660_000)),
    ));

    // Turn 3: 600,000 tokens (~2,280,000 chars in latest turn)
    messages.push(ChatMessage::assistant(
        Some("Fetching task history".to_string()),
        Some(vec![ToolCall {
            id: "call_t3".to_string(),
            call_type: "function".to_string(),
            function: FunctionCall {
                name: "get_task_history".to_string(),
                arguments: "{\"task_id\": \"CVH-001\"}".to_string(),
            },
        }]),
    ));
    messages.push(ChatMessage::tool_response(
        "call_t3",
        format!("[Exit Code: 0]\n{}", "z".repeat(2_280_000)),
    ));

    let tokens_start = estimate_tokens(&messages);
    assert!(
        tokens_start >= 1_700_000,
        "Initial tokens should be at least ~1.7M-1.8M, got {}",
        tokens_start
    );

    let target_tokens = 128_000;
    let res = compact_conversation_to_target(&mut messages, target_tokens, None);

    let tokens_end = estimate_tokens(&messages);
    assert!(
        tokens_end <= target_tokens,
        "Compacted tokens ({}) MUST be <= target_tokens ({})",
        tokens_end,
        target_tokens
    );

    assert_eq!(messages[0].role, Role::System);
    assert_eq!(messages[0].content.as_deref().unwrap(), system_prompt);
    assert_eq!(messages[1].role, Role::User);
    assert_eq!(messages[1].content.as_deref().unwrap(), initial_user_spec);

    assert!(res.evaporated_tools_count >= 3);
    assert!(res.estimated_saved_tokens > 1_500_000);

    let tool_evaporated_found = messages.iter().any(|m| {
        m.role == Role::Tool
            && m.content
                .as_deref()
                .map(|c| c.starts_with("[Output evaporated:") && c.contains("Exit code: 0"))
                .unwrap_or(false)
    });
    assert!(tool_evaporated_found, "Tool output must be replaced by evaporated placeholder");
}

#[test]
fn test_summary_normalization_reconstructed_from_sqlite() {
    let system_prompt = "You are a coding assistant.".to_string();
    let user_spec = "Initial user goal.".to_string();

    let mut messages = vec![
        ChatMessage::system(system_prompt.clone()),
        ChatMessage::user(user_spec.clone()),
        // Reconstructed SQLite summary with Russian header
        ChatMessage::system("### [СВЕРНУТЫЙ КОНТЕКСТ / COMPACTED CONTEXT]\n## Primary Request and Intent\n- Fix bug"),
        ChatMessage::assistant(Some("Turn 2 action".to_string()), None),
        ChatMessage::assistant(Some("Turn 3 action".to_string()), None),
        ChatMessage::assistant(Some("Turn 4 action".to_string()), None),
    ];

    let target_tokens = 50_000;
    let _ = compact_conversation_to_target(&mut messages, target_tokens, None);

    assert_eq!(messages[0].content.as_deref().unwrap(), system_prompt);
    assert_eq!(messages[1].content.as_deref().unwrap(), user_spec);

    let system_messages: Vec<_> = messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert_eq!(system_messages.len(), 1, "Only Turn 0 prompt may have Role::System");

    let summary_messages: Vec<_> = messages
        .iter()
        .filter(|m| is_compacted_summary(&m.role, m.content.as_deref().unwrap_or("")))
        .collect();

    assert_eq!(
        summary_messages.len(),
        1,
        "Must have exactly 1 compacted summary message"
    );
    assert_eq!(summary_messages[0].role, Role::User);
    assert!(
        summary_messages[0].content.as_deref().unwrap().starts_with("# Compacted Context Summary"),
        "Summary message must be normalized to standard header"
    );
}

#[test]
fn test_tier4_hard_ceiling_clamps_giant_user_message() {
    let system_prompt = "System prompt".to_string();
    let user_spec = "Initial user spec".to_string();

    let mut messages = vec![
        ChatMessage::system(system_prompt.clone()),
        ChatMessage::user(user_spec.clone()),
        ChatMessage::assistant(Some("Assistant reply".to_string()), None),
        // User pastes a gigantic 500,000 char log in Turn 2
        ChatMessage::user(format!("Here is a gigantic log:\n{}", "a".repeat(500_000))),
    ];

    let target_tokens = 10_000;
    let _ = compact_conversation_to_target(&mut messages, target_tokens, None);

    let final_tokens = estimate_tokens(&messages);
    assert!(
        final_tokens <= target_tokens,
        "Tier 4 clamping must enforce target_tokens (got {}, target {})",
        final_tokens,
        target_tokens
    );

    // Immutable anchors must stay intact
    assert_eq!(messages[0].content.as_deref().unwrap(), system_prompt);
    assert_eq!(messages[1].content.as_deref().unwrap(), user_spec);
}

#[test]
fn test_tool_call_summary_extractor_aliases_and_todos() {
    let json_str = r#"{
        "file_path": "src/components/Gauges.tsx",
        "command": "cargo check --tests",
        "query": "find_in_files",
        "todos": [
            {"id": "1", "content": "Refactor gauges", "status": "in_progress"},
            {"id": "2", "content": "Add tests", "status": "pending"}
        ]
    }"#;

    let extractor = ToolCallSummaryExtractor::from_raw_json(json_str);
    assert_eq!(extractor.file_path.as_deref(), Some("src/components/Gauges.tsx"));
    assert_eq!(extractor.command.as_deref(), Some("cargo check --tests"));

    let todos_val: serde_json::Value = serde_json::from_str(json_str).unwrap();
    let todos = ToolCallSummaryExtractor::extract_todos(&todos_val);
    assert_eq!(todos.len(), 2);
    assert_eq!(todos[0].0, "Refactor gauges");
    assert_eq!(todos[0].1, "in_progress");
    assert_eq!(todos[1].0, "Add tests");
    assert_eq!(todos[1].1, "pending");
}

#[test]
fn test_extract_touched_files_and_executed_commands() {
    let messages = vec![
        ChatMessage::assistant(
            None,
            Some(vec![
                ToolCall {
                    id: "call_1".to_string(),
                    call_type: "function".to_string(),
                    function: FunctionCall {
                        name: "edit_file".to_string(),
                        arguments: r#"{"file_path": "src/foo.rs"}"#.to_string(),
                    },
                },
                ToolCall {
                    id: "call_2".to_string(),
                    call_type: "function".to_string(),
                    function: FunctionCall {
                        name: "write_file".to_string(),
                        arguments: r#"{"path": "src/bar.rs"}"#.to_string(),
                    },
                },
                ToolCall {
                    id: "call_3".to_string(),
                    call_type: "function".to_string(),
                    function: FunctionCall {
                        name: "read_file".to_string(),
                        arguments: r#"{"file_path": "src/baz.rs"}"#.to_string(),
                    },
                },
                ToolCall {
                    id: "call_4".to_string(),
                    call_type: "function".to_string(),
                    function: FunctionCall {
                        name: "execute_command".to_string(),
                        arguments: r#"{"command": "cargo check"}"#.to_string(),
                    },
                },
            ]),
        ),
        ChatMessage::assistant(
            Some(r#"[Tool Call: execute_command with arguments: {"command": "cargo clippy"}]"#.to_string()),
            None,
        ),
    ];

    let (modified, inspected) = extract_touched_files(&messages);
    assert_eq!(modified.len(), 2);
    assert_eq!(modified[0].0, "src/foo.rs");
    assert_eq!(modified[1].0, "src/bar.rs");
    assert_eq!(inspected.len(), 1);
    assert_eq!(inspected[0], "src/baz.rs");

    let commands = extract_executed_commands(&messages);
    assert_eq!(commands.len(), 2);
    assert_eq!(commands[0].0, "cargo check");
    assert_eq!(commands[1].0, "cargo clippy");
}

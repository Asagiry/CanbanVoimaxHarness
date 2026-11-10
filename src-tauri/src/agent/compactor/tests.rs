//! Tests for Context Compactor & Token Estimation

use super::*;
use crate::llm::{ChatMessage, FunctionCall, Role, ToolCall};

#[test]
fn test_immutable_anchors_and_condensation() {
    let system_prompt = "# System: Persona + Rules + Skills".to_string();
    let initial_user_spec = "Implement Feature X. Specification details...".to_string();

    let mut messages = vec![
        ChatMessage::system(system_prompt.clone()),
        ChatMessage::user(initial_user_spec.clone()),
    ];

    // Turn 1: Bulky tool log (> 500 chars)
    let large_output = "x".repeat(2400);
    messages.push(ChatMessage::assistant(
        Some("Executing check".to_string()),
        Some(vec![ToolCall {
            id: "call_1".to_string(),
            call_type: "function".to_string(),
            function: FunctionCall {
                name: "execute_command".to_string(),
                arguments: "{\"command\": \"cargo check\"}".to_string(),
            },
        }]),
    ));
    messages.push(ChatMessage::tool_response(
        "call_1",
        format!("[Exit Code: 0]\n--- STDOUT ---\n{}", large_output),
    ));

    // Turn 2
    messages.push(ChatMessage::assistant(
        Some("Turn 2 assistant".to_string()),
        None,
    ));

    // Turn 3
    messages.push(ChatMessage::assistant(
        Some("Turn 3 assistant".to_string()),
        None,
    ));

    // Turn 4
    messages.push(ChatMessage::assistant(
        Some("Turn 4 assistant".to_string()),
        None,
    ));

    // Turn 5: Current turn is 5.
    let current_turn = 5;
    let report = compact_context_messages(&mut messages, current_turn);

    // Turn 0 Anchors MUST be untouched
    assert_eq!(messages[0].content.as_ref().unwrap(), &system_prompt);
    assert_eq!(messages[1].content.as_ref().unwrap(), &initial_user_spec);

    // Turn 1 & 2 condensed into crisp summary
    assert_eq!(report.evaporated_count, 1);
    assert!(report.compacted_messages_count > 0);
    assert!(report.saved_tokens > 200);
    assert!(report.tokens_after < report.tokens_before);

    // Message at index 2 should be the Compacted Context Summary
    let summary_msg = &messages[2];
    assert_eq!(summary_msg.role, Role::System);
    assert!(summary_msg.content.as_ref().unwrap().starts_with("# Compacted Context Summary"));

    // Recent turns (Turns 3 and 4) are preserved
    let recent_content = messages.last().unwrap().content.as_ref().unwrap();
    assert_eq!(recent_content, "Turn 4 assistant");
}

#[test]
fn test_evaporate_spilled_logs_and_large_thoughts() {
    let mut messages = vec![
        ChatMessage::system("system"),
        ChatMessage::user("spec"),
        // Turn 1: Spilled tool output (< 500 chars, but spilled format)
        ChatMessage::assistant(Some("action".to_string()), None),
        ChatMessage::tool_response(
            "call_spill",
            "[Output spilled to .cvh/spill/2026-03-30-build.log]\n--- HEAD ---\nline1\n--- TAIL ---\nline2",
        ),
        // Turn 2: Large assistant thought (> 500 chars)
        ChatMessage::assistant(
            Some(format!("<think>{}</think>\nResulting plan", "t".repeat(600))),
            None,
        ),
        // Turn 3 & 4 (recent turns)
        ChatMessage::assistant(Some("recent action 3".to_string()), None),
        ChatMessage::assistant(Some("recent action 4".to_string()), None),
    ];

    let current_turn = 5;
    let count = evaporate_tool_logs(&mut messages, current_turn);
    assert_eq!(count, 2);

    // Check spilled tool log evaporated
    let tool_content = messages[3].content.as_ref().unwrap();
    assert!(tool_content.starts_with("[Output evaporated:"));

    // Check thought evaporated
    let thought_content = messages[4].content.as_ref().unwrap();
    assert!(thought_content.contains("[Thought evaporated:"));
    assert!(thought_content.contains("Resulting plan"));
}

#[test]
fn test_recent_turns_and_small_logs_protected() {
    let mut messages = vec![
        ChatMessage::system("system"),
        ChatMessage::user("spec"),
        // Turn 1: Short output (< 500 chars)
        ChatMessage::assistant(Some("action".to_string()), None),
        ChatMessage::tool_response("call_short", "short output 42 chars"),
        // Turn 2: Recent turn (current_turn - 2 = 0)
        ChatMessage::assistant(Some("recent action".to_string()), None),
    ];

    let report = compact_context_messages(&mut messages, 2);
    assert_eq!(report.evaporated_count, 0);
    assert_eq!(messages[3].content.as_ref().unwrap(), "short output 42 chars");
}

#[test]
fn test_resolve_compactor_model_inheritance() {
    // "inherit" returns active_agent_model
    assert_eq!(resolve_compactor_model("inherit", "claude-3-5-sonnet"), "claude-3-5-sonnet");
    assert_eq!(resolve_compactor_model("INHERIT", "deepseek-chat"), "deepseek-chat");
    // Empty returns active_agent_model
    assert_eq!(resolve_compactor_model("", "gpt-4o"), "gpt-4o");
    assert_eq!(resolve_compactor_model("   ", "gemini-3.8-flash-high"), "gemini-3.8-flash-high");
    // Specific model override takes precedence
    assert_eq!(resolve_compactor_model("gemini-1.5-flash", "claude-3-5-sonnet"), "gemini-1.5-flash");
}

#[test]
fn test_multiple_compaction_rounds_no_duplicate_summary() {
    let mut messages = vec![
        ChatMessage::system("system"),
        ChatMessage::user("spec"),
        ChatMessage::assistant(Some("action 1".to_string()), None),
        ChatMessage::tool_response("call_1", "output 1"),
        ChatMessage::assistant(Some("action 2".to_string()), None),
        ChatMessage::assistant(Some("action 3".to_string()), None),
        ChatMessage::assistant(Some("action 4".to_string()), None),
    ];

    // Round 1 compaction at turn 5
    let report1 = compact_context_messages(&mut messages, 5);
    assert!(report1.compacted_messages_count > 0);

    // Count how many system summary messages exist
    let summaries_count_1 = messages
        .iter()
        .filter(|m| {
            m.role == Role::System
                && m.content
                    .as_ref()
                    .map(|c| c.starts_with("# Compacted Context Summary"))
                    .unwrap_or(false)
        })
        .count();
    assert_eq!(summaries_count_1, 1);

    // Add more turns
    messages.push(ChatMessage::assistant(Some("action 5".to_string()), None));
    messages.push(ChatMessage::assistant(Some("action 6".to_string()), None));
    messages.push(ChatMessage::assistant(Some("action 7".to_string()), None));

    // Round 2 compaction at turn 8
    let report2 = compact_context_messages(&mut messages, 8);
    assert!(report2.compacted_messages_count > 0);

    let summaries_count_2 = messages
        .iter()
        .filter(|m| {
            m.role == Role::System
                && m.content
                    .as_ref()
                    .map(|c| c.starts_with("# Compacted Context Summary"))
                    .unwrap_or(false)
        })
        .count();
    assert_eq!(summaries_count_2, 1, "Should never duplicate summary blocks!");
}

#[test]
fn test_fallback_compaction_extracts_tools_todos_and_tests() {
    let messages = vec![
        ChatMessage::system("You are an agent"),
        ChatMessage::user("Fix context compactor memory loss and gauge bugs"),
        // Assistant tool call: todo_write
        ChatMessage::assistant(
            Some("Creating plan".to_string()),
            Some(vec![ToolCall {
                id: "call_todo".to_string(),
                call_type: "function".to_string(),
                function: FunctionCall {
                    name: "todo_write".to_string(),
                    arguments: serde_json::json!({
                        "todos": [
                            { "content": "Update compactor.rs", "status": "completed" },
                            { "content": "Fix gauge bug in ContextGauge.tsx", "status": "in_progress" },
                            { "content": "Run cargo test", "status": "pending" }
                        ]
                    }).to_string(),
                },
            }]),
        ),
        ChatMessage::tool_response("call_todo", "Todos recorded successfully"),
        // Assistant tool call: edit_file
        ChatMessage::assistant(
            Some("Editing compactor".to_string()),
            Some(vec![ToolCall {
                id: "call_edit".to_string(),
                call_type: "function".to_string(),
                function: FunctionCall {
                    name: "edit_file".to_string(),
                    arguments: serde_json::json!({
                        "file_path": "src-tauri/src/agent/compactor.rs",
                        "old_string": "foo",
                        "new_string": "bar"
                    }).to_string(),
                },
            }]),
        ),
        ChatMessage::tool_response("call_edit", "File edited successfully"),
        // Assistant tool call: execute_command
        ChatMessage::assistant(
            Some("Running cargo test".to_string()),
            Some(vec![ToolCall {
                id: "call_exec".to_string(),
                call_type: "function".to_string(),
                function: FunctionCall {
                    name: "execute_command".to_string(),
                    arguments: serde_json::json!({
                        "command": "cargo test"
                    }).to_string(),
                },
            }]),
        ),
        ChatMessage::tool_response("call_exec", "[Exit Code: 0]\ntest result: ok. 14 passed; 0 failed"),
    ];

    let report = generate_fallback_compaction_report("CVH-TEST", &messages);

    // Verify no generic stubs
    assert!(!report.contains("Inspected workspace configuration and task requirements."));

    // Verify edited file extracted
    assert!(report.contains("src-tauri/src/agent/compactor.rs"));
    assert!(report.contains("Modified via edit_file"));

    // Verify command executed
    assert!(report.contains("cargo test"));

    // Verify test passed result
    assert!(report.contains("[TEST PASSED]"));
    assert!(report.contains("test result: ok"));

    // Verify checklist items
    assert!(report.contains("- [x] Update compactor.rs"));
    assert!(report.contains("- [/] (IN PROGRESS) Fix gauge bug in ContextGauge.tsx"));
    assert!(report.contains("- [ ] Run cargo test"));

    // Verify current focus
    assert!(report.contains("Fix gauge bug in ContextGauge.tsx"));
}

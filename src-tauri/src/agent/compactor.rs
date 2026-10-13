use crate::llm::{ChatMessage, Role};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Official compaction report detailing token space reclaimed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompactionReport {
    pub tokens_before: usize,
    pub tokens_after: usize,
    pub evaporated_count: usize,
    pub saved_tokens: usize,
}

/// Backwards-compatible result struct for existing callers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionResult {
    pub original_chars: usize,
    pub compacted_chars: usize,
    pub saved_chars: usize,
    pub estimated_saved_tokens: usize,
    pub evaporated_tools_count: usize,
    pub preserved_state_summary: String,
}

/// Structured state snapshot preserving crucial facts across turns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStateSnapshot {
    pub modified_files: Vec<String>,
    pub sensor_status: String,
    pub next_step: String,
    pub verdict: String,
}

impl ExecutionStateSnapshot {
    pub fn to_markdown(&self) -> String {
        let files_str = if self.modified_files.is_empty() {
            "None detected".to_string()
        } else {
            self.modified_files
                .iter()
                .map(|f| format!("`{}`", f))
                .collect::<Vec<_>>()
                .join(", ")
        };

        format!(
            "- **Modified Files**: {}\n- **Sensor Status**: {}\n- **Exact Next Step**: {}\n- **Active Blockers / Verdict**: {}",
            files_str, self.sensor_status, self.next_step, self.verdict
        )
    }
}

/// In-memory/persisted representation of an active agent's conversational context.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentContextState {
    pub current_turn: usize,
    pub messages: Vec<ChatMessage>,
}

/// Estimates tokens for a given text (~4 chars per token).
pub fn estimate_text_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    text.chars().count().div_ceil(4)
}

/// Estimates the total token count of a sequence of ChatMessages.
pub fn estimate_tokens(messages: &[ChatMessage]) -> usize {
    let mut total = 0;
    for msg in messages {
        total += 4; // base message framing

        if let Some(ref content) = msg.content {
            total += estimate_text_tokens(content);
        }
        if let Some(ref name) = msg.name {
            total += estimate_text_tokens(name);
        }
        if let Some(ref tool_id) = msg.tool_call_id {
            total += estimate_text_tokens(tool_id);
        }
        if let Some(ref tool_calls) = msg.tool_calls {
            for tc in tool_calls {
                total += 4;
                total += estimate_text_tokens(&tc.function.name);
                total += estimate_text_tokens(&tc.function.arguments);
            }
        }
    }
    total
}

/// Maps each message in the conversation to its corresponding turn index.
/// Turn 0: System Prompt and Initial User Specification (Immutable Anchors).
/// Turn 1..N: Subsequent Assistant responses and associated Tool executions.
pub fn compute_message_turns(messages: &[ChatMessage]) -> Vec<usize> {
    let mut turns = Vec::with_capacity(messages.len());
    let mut current_turn_idx = 0;
    let mut seen_first_user = false;

    for (i, msg) in messages.iter().enumerate() {
        if i == 0 && msg.role == Role::System {
            turns.push(0);
            continue;
        }
        if !seen_first_user && msg.role == Role::User {
            seen_first_user = true;
            turns.push(0);
            continue;
        }

        match msg.role {
            Role::System => {
                turns.push(0);
            }
            Role::User => {
                current_turn_idx += 1;
                turns.push(current_turn_idx);
            }
            Role::Assistant => {
                current_turn_idx += 1;
                turns.push(current_turn_idx);
            }
            Role::Tool => {
                turns.push(current_turn_idx);
            }
        }
    }
    turns
}

/// Extracts a mapping from tool_call_id to tool name by scanning Assistant tool_calls.
pub fn build_tool_name_map(messages: &[ChatMessage]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for msg in messages {
        if let Some(ref tool_calls) = msg.tool_calls {
            for tc in tool_calls {
                map.insert(tc.id.clone(), tc.function.name.clone());
            }
        }
    }
    map
}

/// Parses the exit code from tool output or returns a default.
fn extract_exit_code(content: &str) -> i32 {
    if let Some(idx) = content.find("[Exit Code: ") {
        let remainder = &content[idx + 12..];
        if let Some(end) = remainder.find(']') {
            if let Ok(code) = remainder[..end].trim().parse::<i32>() {
                return code;
            }
        }
    }
    if content.starts_with("Error executing") || content.contains("[CRITICAL]") {
        1
    } else {
        0
    }
}

/// Zero-Cost Log Evaporation:
/// Iterates through messages and replaces bulky tool execution outputs (> 500 characters)
/// where `current_turn - turn_index > 3` with a lightweight, deterministic placeholder:
/// `[Output evaporated: {len} chars truncated. Exit code: {code}. Tool: {name}]`
/// Reclaims up to 80% of token space instantly WITHOUT ANY LLM CALL.
#[allow(clippy::ptr_arg)]
pub fn evaporate_tool_logs(messages: &mut Vec<ChatMessage>, current_turn: usize) -> usize {
    let turns = compute_message_turns(messages);
    let tool_names = build_tool_name_map(messages);
    let mut evaporated_count = 0;

    for i in 0..messages.len() {
        let turn_index = turns[i];

        // Immutable Anchors: Turn 0 (System + Initial User) is never modified
        if turn_index == 0 {
            continue;
        }

        // Only evaporate older turns where current_turn - turn_index > 3
        if current_turn.saturating_sub(turn_index) <= 3 {
            continue;
        }

        let msg = &mut messages[i];
        if msg.role != Role::Tool {
            continue;
        }

        if let Some(ref content) = msg.content {
            let char_len = content.chars().count();
            if char_len > 500 {
                let code = extract_exit_code(content);
                let tool_name = msg
                    .name
                    .clone()
                    .or_else(|| {
                        msg.tool_call_id
                            .as_ref()
                            .and_then(|id| tool_names.get(id).cloned())
                    })
                    .unwrap_or_else(|| "tool".to_string());

                let replacement = format!(
                    "[Output evaporated: {} chars truncated. Exit code: {}. Tool: {}]",
                    char_len, code, tool_name
                );

                msg.content = Some(replacement);
                evaporated_count += 1;
            }
        }
    }

    evaporated_count
}

/// Compresses intermediate turns without injecting synthetic state clutter into messages.
#[allow(clippy::ptr_arg)]
pub fn compress_intermediate_turns(
    _messages: &mut Vec<ChatMessage>,
    _current_turn: usize,
) -> usize {
    // Clutter removal: do not inject synthetic 'Preserved Execution State' into chat messages array
    0
}

/// Full State-Preserving Context Compactor:
/// 1. Enforces Immutable Anchors (Turn 0: System + User spec, and 2 most recent turns).
/// 2. Executes Zero-Cost Log Evaporation on tool results older than 3 turns (> 500 chars).
/// 3. Generates and returns a comprehensive `CompactionReport`.
pub fn compact_context_messages(
    messages: &mut Vec<ChatMessage>,
    current_turn: usize,
) -> CompactionReport {
    let tokens_before = estimate_tokens(messages);

    // 1. Evaporate bulky tool logs (> 500 chars, older than 3 turns)
    let evaporated_count = evaporate_tool_logs(messages, current_turn);

    // 2. Do not inject clutter into messages
    let _ = compress_intermediate_turns(messages, current_turn);

    let tokens_after = estimate_tokens(messages);
    let saved_tokens = tokens_before.saturating_sub(tokens_after);

    CompactionReport {
        tokens_before,
        tokens_after,
        evaporated_count,
        saved_tokens,
    }
}

/// Backwards-compatible function for existing callers
pub fn compact_conversation(
    messages: &mut Vec<ChatMessage>,
    current_snapshot: Option<ExecutionStateSnapshot>,
) -> CompactionResult {
    let original_chars: usize = messages
        .iter()
        .map(|m| m.content.as_ref().map(|c| c.len()).unwrap_or(0))
        .sum();

    let turns = compute_message_turns(messages);
    let max_turn = turns.iter().copied().max().unwrap_or(0);

    let report = compact_context_messages(messages, max_turn);

    let snapshot_summary = if let Some(s) = current_snapshot {
        s.to_markdown()
    } else {
        String::new()
    };

    let compacted_chars: usize = messages
        .iter()
        .map(|m| m.content.as_ref().map(|c| c.len()).unwrap_or(0))
        .sum();

    CompactionResult {
        original_chars,
        compacted_chars,
        saved_chars: original_chars.saturating_sub(compacted_chars),
        estimated_saved_tokens: report.saved_tokens,
        evaporated_tools_count: report.evaporated_count,
        preserved_state_summary: snapshot_summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{FunctionCall, ToolCall};

    #[test]
    fn test_immutable_anchors_and_log_evaporation() {
        let system_prompt = "# System: Persona + Rules + Skills".to_string();
        let initial_user_spec = "Implement Feature X. Specification details...".to_string();

        let mut messages = vec![
            ChatMessage::system(system_prompt.clone()),
            ChatMessage::user(initial_user_spec.clone()),
        ];

        // Turn 1: Bulky tool log (> 500 chars)
        let large_output = "x".repeat(1200);
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
        // Turn 1 is 4 turns ago (5 - 1 = 4 > 3). It should be evaporated!
        let current_turn = 5;
        let report = compact_context_messages(&mut messages, current_turn);

        // Turn 0 Anchors MUST be untouched
        assert_eq!(messages[0].content.as_ref().unwrap(), &system_prompt);
        assert_eq!(messages[1].content.as_ref().unwrap(), &initial_user_spec);

        // Tool output at Turn 1 should be evaporated
        assert_eq!(report.evaporated_count, 1);
        let tool_content = messages[3].content.as_ref().unwrap();
        assert!(tool_content.contains("[Output evaporated:"));
        assert!(tool_content.contains("Exit code: 0"));
        assert!(tool_content.contains("Tool: execute_command"));
        assert!(report.saved_tokens > 200);
        assert!(report.tokens_after < report.tokens_before);
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
}

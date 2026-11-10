//! Zero-Cost Log Evaporation & Message Clamping (§ 2.22)

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::llm::{ChatMessage, Role};

/// Official compaction report detailing token space reclaimed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompactionReport {
    pub tokens_before: usize,
    pub tokens_after: usize,
    pub evaporated_count: usize,
    pub saved_tokens: usize,
    #[serde(default)]
    pub compacted_messages_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
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
                if !seen_first_user {
                    turns.push(0);
                } else {
                    turns.push(current_turn_idx);
                }
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
pub fn extract_exit_code(content: &str) -> i32 {
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

/// Internal helper to evaporate bulky tool execution outputs and large assistant thoughts
/// matching a turn predicate. Preserves Turn 0 immutable anchors.
pub fn evaporate_messages_by_turn_predicate<P>(
    messages: &mut [ChatMessage],
    should_evaporate_turn: P,
) -> usize
where
    P: Fn(usize) -> bool,
{
    let turns = compute_message_turns(messages);
    let tool_names = build_tool_name_map(messages);
    let mut evaporated_count = 0;

    for i in 0..messages.len() {
        let turn_index = turns[i];

        // Immutable Anchors: Turn 0 (System + Initial User) is never modified
        if turn_index == 0 {
            continue;
        }

        if !should_evaporate_turn(turn_index) {
            continue;
        }

        let msg = &mut messages[i];

        // 1. Evaporate Tool Results
        if msg.role == Role::Tool {
            if let Some(ref content) = msg.content {
                if content.starts_with("[Output evaporated:") {
                    continue;
                }

                let char_len = content.chars().count();
                let is_spilled = content.starts_with("[Output spilled to")
                    || (content.starts_with("[Output truncated:") && content.contains("spilled to"))
                    || content.contains(".cvh/spill/");
                let is_bulky = char_len > 500;

                if is_spilled || is_bulky {
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

                    msg.content = Some(replacement.into());
                    evaporated_count += 1;
                }
            }
        } else if msg.role == Role::Assistant {
            // 2. Evaporate Large Assistant Thought Messages
            if let Some(ref content) = msg.content {
                if content.starts_with("[Thought evaporated:") {
                    continue;
                }

                let is_explicit_thought = msg.name.as_deref() == Some("thought")
                    || msg.name.as_deref() == Some("thinking")
                    || content.starts_with("[Thought")
                    || content.starts_with("<think>")
                    || content.starts_with("<thinking>");

                let has_embedded_think = (content.contains("<think>") && content.contains("</think>"))
                    || (content.contains("<thinking>") && content.contains("</thinking>"));

                let char_len = content.chars().count();

                if has_embedded_think {
                    let mut new_content = content.to_string();
                    let mut modified = false;

                    while let Some(start_idx) = new_content.find("<think>") {
                        if let Some(end_offset) = new_content[start_idx..].find("</think>") {
                            let end_idx = start_idx + end_offset + "</think>".len();
                            let inner_len = end_idx - start_idx;
                            if inner_len > 300 {
                                let placeholder = format!("[Thought evaporated: {} chars truncated]", inner_len);
                                new_content.replace_range(start_idx..end_idx, &placeholder);
                                modified = true;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    while let Some(start_idx) = new_content.find("<thinking>") {
                        if let Some(end_offset) = new_content[start_idx..].find("</thinking>") {
                            let end_idx = start_idx + end_offset + "</thinking>".len();
                            let inner_len = end_idx - start_idx;
                            if inner_len > 300 {
                                let placeholder = format!("[Thought evaporated: {} chars truncated]", inner_len);
                                new_content.replace_range(start_idx..end_idx, &placeholder);
                                modified = true;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    if modified {
                        msg.content = Some(new_content.into());
                        evaporated_count += 1;
                    }
                } else if is_explicit_thought && char_len > 300 {
                    let replacement = format!("[Thought evaporated: {} chars truncated]", char_len);
                    msg.content = Some(replacement.into());
                    evaporated_count += 1;
                }
            }
        }
    }

    evaporated_count
}

pub fn evaporate_tool_logs_older_than(
    messages: &mut [ChatMessage],
    current_turn: usize,
    keep_recent_turns: usize,
) -> usize {
    evaporate_messages_by_turn_predicate(messages, |turn| {
        current_turn.saturating_sub(turn) > keep_recent_turns
    })
}

pub fn evaporate_current_turn_tools(
    messages: &mut [ChatMessage],
    current_turn: usize,
) -> usize {
    evaporate_messages_by_turn_predicate(messages, |turn| {
        current_turn == usize::MAX || turn == current_turn
    })
}

/// Zero-Cost Log Evaporation:
/// Preserves Immutable Anchors (Turn 0) and the 2 most recent turns.
/// Reclaims up to 80% of token space instantly WITHOUT ANY LLM CALL.
#[allow(clippy::ptr_arg)]
pub fn evaporate_tool_logs(messages: &mut Vec<ChatMessage>, current_turn: usize) -> usize {
    evaporate_tool_logs_older_than(messages, current_turn, 2)
}

/// Clamps oversized messages and tool call arguments exceeding `max_chars`.
/// Strictly preserves Turn 0 immutable anchors.
pub fn clamp_oversized_messages(
    messages: &mut [ChatMessage],
    max_chars: usize,
) -> usize {
    let turns = compute_message_turns(messages);
    let mut clamped_count = 0;

    for (i, msg) in messages.iter_mut().enumerate() {
        let turn = turns.get(i).copied().unwrap_or(0);
        if turn == 0 {
            continue;
        }

        if let Some(ref content) = msg.content {
            let char_count = content.chars().count();
            if char_count > max_chars {
                let truncated: String = content.chars().take(max_chars).collect();
                msg.content = Some(format!(
                    "{}... [Truncated by Context Compactor to enforce window ceiling]",
                    truncated
                ).into());
                clamped_count += 1;
            }
        }

        if let Some(ref mut tool_calls) = msg.tool_calls {
            for tc in tool_calls.iter_mut() {
                let arg_chars = tc.function.arguments.chars().count();
                if arg_chars > max_chars {
                    let truncated: String = tc.function.arguments.chars().take(max_chars).collect();
                    tc.function.arguments = format!(
                        "{}... [Truncated by Context Compactor to enforce window ceiling]",
                        truncated
                    );
                    clamped_count += 1;
                }
            }
        }
    }

    clamped_count
}

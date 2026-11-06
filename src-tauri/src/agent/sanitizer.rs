use crate::db::ActivityItem;
use crate::llm::{CacheControl, ChatMessage, FunctionCall, Role, ToolCall};
use std::collections::HashSet;

/// Scans the conversation for any assistant messages with unclosed tool calls
/// (e.g. when execution was aborted mid-tool) and appends synthetic tool responses
/// so that the transcript remains strictly valid according to LLM API schemas.
pub fn sanitize_unclosed_tool_calls(messages: &mut Vec<ChatMessage>) {
    let mut i = 0;
    while i < messages.len() {
        if messages[i].role == Role::Assistant {
            if let Some(tool_calls) = messages[i].tool_calls.clone() {
                if !tool_calls.is_empty() {
                    // Collect tool_call_ids answered immediately following this assistant message
                    let mut answered_ids = HashSet::new();
                    let mut j = i + 1;
                    while j < messages.len() && messages[j].role == Role::Tool {
                        if let Some(ref tid) = messages[j].tool_call_id {
                            answered_ids.insert(tid.clone());
                        }
                        j += 1;
                    }

                    // For any tool call that was not answered, synthesize a tool response
                    for tc in tool_calls {
                        if !answered_ids.contains(&tc.id) {
                            eprintln!(
                                "CVH [agent:sanitize]: Synthesizing tool response for unclosed tool call {} ({})",
                                tc.id, tc.function.name
                            );
                            messages.insert(
                                j,
                                ChatMessage::tool_response(
                                    tc.id.clone(),
                                    "[Инструмент прерван указанием пользователя / Tool execution interrupted by user steer]",
                                ),
                            );
                            j += 1;
                        }
                    }
                    i = j;
                    continue;
                }
            }
        }
        i += 1;
    }
}

/// Reconstructs a list of ChatMessages from persisted SQLite dialog items
/// when in-memory context has been lost (e.g. after process restart).
pub fn reconstruct_messages_from_dialog(
    system_prompt: &str,
    initial_user_prompt: &str,
    items: &[ActivityItem],
) -> Vec<ChatMessage> {
    let mut messages = vec![
        ChatMessage::system(system_prompt).with_cache_control(CacheControl::ephemeral()),
        ChatMessage::user(initial_user_prompt).with_cache_control(CacheControl::ephemeral()),
    ];

    append_messages_from_dialog(&mut messages, initial_user_prompt, items);
    messages
}

/// Whether a persisted dialog item should be replayed into the LLM transcript
/// when a snapshot is resumed or steers are drained. Tool results, assistant
/// outputs and compacted summaries must survive restarts — otherwise the
/// conversation restarts from ~2 messages and the prompt stops growing
/// (Req chars plateau), which mimics a provider-side cache cap.
pub fn is_dialog_item_replayable(item: &ActivityItem) -> bool {
    matches!(
        item.item_type.as_str(),
        "user" | "steer" | "agent" | "summary" | "tool" | "output" | "assistant" | "compacted"
    )
}

pub fn append_messages_from_dialog(
    messages: &mut Vec<ChatMessage>,
    initial_user_prompt: &str,
    items: &[ActivityItem],
) {
    // Deduplicate tool pairs reconstructed from SQLite: a "tool" ActivityItem
    // already encodes both the assistant tool_call and the tool_response
    // (see the "tool" arm below). When a snapshot resume replays items that
    // are already present in saved.messages (e.g. after a crash between
    // save_conversation_context and the dialog_cursor advance), skipping an
    // exact (tool_call_id, result) pair that was just appended keeps the
    // transcript strictly valid for the LLM API.
    let mut appended_tool_ids: std::collections::HashSet<String> = messages
        .iter()
        .filter_map(|m| m.tool_call_id.clone())
        .collect();

    for item in items {
        if item.is_runtime_error() { continue; }
        match item.item_type.as_str() {
            "summary" => {
                if let Some(ref c) = item.content {
                    let trimmed = c.trim();
                    if !trimmed.is_empty() {
                        messages.push(ChatMessage::user(trimmed));
                    }
                }
            }
            "steer" | "agent" => {
                if let Some(ref c) = item.content {
                    let trimmed = c.trim();
                    if !trimmed.is_empty() {
                        let formatted = if item.item_type == "agent" && !trimmed.starts_with("### [") && !trimmed.starts_with("### ") {
                            format!("### [СООБЩЕНИЕ АГЕНТА / AGENT MESSAGE]\n{}", trimmed)
                        } else if trimmed.starts_with("### [") || trimmed.starts_with("### Задача возвращена") {
                            trimmed.to_string()
                        } else {
                            format!(
                                "### [УКАЗАНИЕ ПОЛЬЗОВАТЕЛЯ / USER STEER]\n{}",
                                trimmed
                            )
                        };
                        if let Some(ref imgs) = item.images {
                            if !imgs.is_empty() {
                                messages.push(ChatMessage::user_with_images(formatted, imgs.clone()));
                            } else {
                                messages.push(ChatMessage::user(formatted));
                            }
                        } else {
                            messages.push(ChatMessage::user(formatted));
                        }
                    }
                }
            }
            "user" => {
                if let Some(ref c) = item.content {
                    let trimmed = c.trim();
                    if !trimmed.is_empty() {
                        if trimmed == initial_user_prompt.trim() {
                            if let Some(ref imgs) = item.images {
                                if !imgs.is_empty() && messages.len() >= 2 && messages[1].role == Role::User {
                                    messages[1] = ChatMessage::user_with_images(initial_user_prompt, imgs.clone())
                                        .with_cache_control(CacheControl::ephemeral());
                                }
                            }
                        } else {
                            if let Some(ref imgs) = item.images {
                                if !imgs.is_empty() {
                                    messages.push(ChatMessage::user_with_images(trimmed, imgs.clone()));
                                } else {
                                    messages.push(ChatMessage::user(trimmed));
                                }
                            } else {
                                messages.push(ChatMessage::user(trimmed));
                            }
                        }
                    }
                }
            }
            "output" | "assistant" => {
                if let Some(ref c) = item.content {
                    let trimmed = c.trim();
                    if !trimmed.is_empty() {
                        messages.push(ChatMessage::assistant(Some(trimmed.to_string()), None));
                    }
                }
            }
            "tool" => {
                if let Some(ref tool_val) = item.tool {
                    let call_id = tool_val.get("id").and_then(|v| v.as_str()).unwrap_or(&item.id);

                    // Interest-only / optimistic cards carry no real result yet
                    // ("queued"/"running" without a result payload). Replaying
                    // them would fabricate an assistant tool_call that the model
                    // never made. Only reconstruct pairs that carry non-empty
                    // result content (or an explicit stdout/output/result).
                    let has_result = tool_val
                        .get("result")
                        .or_else(|| tool_val.get("stdout"))
                        .or_else(|| tool_val.get("output"))
                        .and_then(|v| v.as_str())
                        .map(|s| !s.trim().is_empty())
                        .unwrap_or(false)
                        || item.content.as_deref().is_some_and(|c| !c.trim().is_empty());
                    let status = tool_val.get("status").and_then(|v| v.as_str()).unwrap_or("");
                    if !has_result && matches!(status, "queued" | "running") {
                        continue;
                    }
                    if appended_tool_ids.contains(call_id) {
                        continue;
                    }

                    // 1. Имя инструмента: проверяем 'name', затем 'toolName', затем алиасы
                    let name = tool_val
                        .get("name")
                        .or_else(|| tool_val.get("toolName"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown_tool");

                    // 2. Аргументы: поддерживаем и строку 'arguments', и объект 'params'
                    let args = if let Some(args_str) = tool_val.get("arguments").and_then(|v| v.as_str()) {
                        args_str.to_string()
                    } else if let Some(params_val) = tool_val.get("params") {
                        if params_val.is_string() {
                            params_val.as_str().unwrap().to_string()
                        } else {
                            serde_json::to_string(params_val).unwrap_or_else(|_| "{}".to_string())
                        }
                    } else {
                        "{}".to_string()
                    };

                    // 3. Результат: проверяем 'result', 'stdout', 'output' или item.content
                    let result = tool_val
                        .get("result")
                        .or_else(|| tool_val.get("stdout"))
                        .or_else(|| tool_val.get("output"))
                        .and_then(|v| v.as_str())
                        .or(item.content.as_deref())
                        .unwrap_or("");

                    let tc = ToolCall {
                        id: call_id.to_string(),
                        call_type: "function".to_string(),
                        function: FunctionCall {
                            name: name.to_string(),
                            arguments: args,
                        },
                    };
                    messages.push(ChatMessage::assistant(None, Some(vec![tc])));
                    messages.push(ChatMessage::tool_response(call_id, result));
                    appended_tool_ids.insert(call_id.to_string());
                }
            }
            "compacted" => {
                if let Some(ref c) = item.content {
                    let trimmed = c.trim();
                    if !trimmed.is_empty() {
                        let formatted = if trimmed.starts_with("# Compacted Context Summary") {
                            trimmed.to_string()
                        } else {
                            format!("# Compacted Context Summary\n\n{}", trimmed)
                        };
                        messages.push(ChatMessage::system(formatted));
                    }
                }
            }
            _ => {}
        }
    }

    sanitize_unclosed_tool_calls(messages);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{FunctionCall, ToolCall};

    #[test]
    fn test_sanitize_unclosed_tool_calls() {
        let mut messages = vec![
            ChatMessage::assistant(
                None,
                Some(vec![ToolCall {
                    id: "call_abc123".to_string(),
                    call_type: "function".to_string(),
                    function: FunctionCall {
                        name: "execute_command".to_string(),
                        arguments: r#"{"command":"dir"}"#.to_string(),
                    },
                }]),
            ),
        ];

        sanitize_unclosed_tool_calls(&mut messages);

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1].role, Role::Tool);
        assert_eq!(messages[1].tool_call_id.as_deref(), Some("call_abc123"));
        assert!(messages[1].content.as_deref().unwrap().contains("прерван"));
    }

    #[test]
    fn test_reconstruct_messages_from_dialog_empty() {
        let messages = reconstruct_messages_from_dialog("System instructions", "User prompt", &[]);

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::System);
        assert_eq!(messages[0].content.as_deref(), Some("System instructions"));
        assert_eq!(messages[1].role, Role::User);
        assert_eq!(messages[1].content.as_deref(), Some("User prompt"));
    }
}

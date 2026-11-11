//! Token Estimation, Model Resolution & Context State (§ 2.22)

use crate::llm::{ChatMessage, OpenAITool, Role};
use serde::{Deserialize, Serialize};

/// Resolves the compactor model: if "inherit" or empty, uses the running agent's model.
pub fn resolve_compactor_model(compactor_setting: &str, active_agent_model: &str) -> String {
    let trimmed = compactor_setting.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("inherit") {
        active_agent_model.to_string()
    } else {
        trimmed.to_string()
    }
}

/// In-memory/persisted representation of an active agent's conversational context.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentContextState {
    pub current_turn: usize,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub compaction_revision: u64,
}

/// Estimates tokens for a given text using multilingual-aware BPE heuristics:
/// - Non-ASCII / Cyrillic UTF-8 characters: ~1.4 chars per token.
/// - ASCII and other characters: ~3.8 chars per token.
pub fn estimate_text_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    let mut non_ascii = 0usize;
    let mut ascii = 0usize;

    for c in text.chars() {
        if !c.is_ascii() {
            non_ascii += 1;
        } else {
            ascii += 1;
        }
    }

    let estimated = (non_ascii as f64 / 1.4) + (ascii as f64 / 3.8);
    (estimated.ceil() as usize).max(1)
}

/// Estimates the total token count of a sequence of ChatMessages.
pub fn estimate_tokens(messages: &[ChatMessage]) -> usize {
    let mut total = 0;
    for msg in messages {
        total += 4; // base message framing

        if let Some(ref content) = msg.content {
            match content {
                crate::llm::MessageContent::Text(t) => total += estimate_text_tokens(t),
                crate::llm::MessageContent::Parts(parts) => {
                    for p in parts {
                        match p {
                            crate::llm::ContentPart::Text { text } => total += estimate_text_tokens(text),
                            crate::llm::ContentPart::ImageUrl { .. } => total += 1600,
                        }
                    }
                }
            }
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

/// Estimates the token count of a set of tool definitions schemas.
pub fn estimate_tools_tokens(tools: &Option<Vec<OpenAITool>>) -> usize {
    let mut total = 0;
    if let Some(tool_list) = tools {
        if !tool_list.is_empty() {
            total += 12; // base tool definitions framing overhead
            for tool in tool_list {
                total += 12; // individual tool framing
                total += estimate_text_tokens(&tool.tool_type);
                total += estimate_text_tokens(&tool.function.name);
                total += estimate_text_tokens(&tool.function.description);
                let params_str = serde_json::to_string(&tool.function.parameters).unwrap_or_default();
                if !params_str.is_empty() {
                    total += estimate_text_tokens(&params_str);
                }
            }
        }
    }
    total
}

/// Estimates the total token count of an entire LLM request,
/// accounting for conversation messages and the full JSON schemas of tools definitions.
pub fn estimate_request_tokens(
    messages: &[ChatMessage],
    tools: Option<&[OpenAITool]>,
) -> usize {
    let mut total = estimate_tokens(messages);

    if let Some(tool_list) = tools {
        if !tool_list.is_empty() {
            total += 12; // base tool definitions framing overhead
            for tool in tool_list {
                total += 12; // individual tool framing
                total += estimate_text_tokens(&tool.tool_type);
                total += estimate_text_tokens(&tool.function.name);
                total += estimate_text_tokens(&tool.function.description);
                let params_str = serde_json::to_string(&tool.function.parameters).unwrap_or_default();
                if !params_str.is_empty() {
                    total += estimate_text_tokens(&params_str);
                }
            }
        }
    }

    total
}

/// Returns true if the message is a compacted context summary from earlier compaction.
pub fn is_compacted_summary(role: &Role, content: &str) -> bool {
    if !matches!(role, Role::System | Role::User) {
        return false;
    }
    let trimmed = content.trim_start();
    trimmed.starts_with("# Compacted Context Summary")
        || trimmed.starts_with("### [СВЕРНУТЫЙ КОНТЕКСТ")
        || trimmed.starts_with("### [COMPACTED CONTEXT]")
        || trimmed.starts_with("# Context Summary")
        || trimmed.starts_with("### Compacted Context Summary")
}

/// Strips leading summary headers from existing compacted context summaries.
pub fn strip_summary_header(content: &str) -> &str {
    let trimmed = content.trim();
    if let Some(rest) = trimmed.strip_prefix("# Compacted Context Summary") {
        rest.trim_start()
    } else if let Some(rest) = trimmed.strip_prefix("### [СВЕРНУТЫЙ КОНТЕКСТ / COMPACTED CONTEXT]") {
        rest.trim_start()
    } else if let Some(rest) = trimmed.strip_prefix("### [СВЕРНУТЫЙ КОНТЕКСТ") {
        if let Some(idx) = rest.find(']') {
            rest[idx + 1..].trim_start()
        } else {
            rest.trim_start()
        }
    } else if let Some(rest) = trimmed.strip_prefix("### [COMPACTED CONTEXT]") {
        rest.trim_start()
    } else if let Some(rest) = trimmed.strip_prefix("# Context Summary") {
        rest.trim_start()
    } else if let Some(rest) = trimmed.strip_prefix("### Compacted Context Summary") {
        rest.trim_start()
    } else {
        trimmed
    }
}

/// Computes the split index in persisted SQLite `ActivityItem`s matching the
/// recent messages retained after compaction, so older items are archived accurately.
pub fn find_archive_split_index(
    existing_items: &[crate::db::ActivityItem],
    messages: &[ChatMessage],
) -> usize {
    let has_summary = messages.iter().any(|m| {
        is_compacted_summary(&m.role, m.content.as_deref().unwrap_or(""))
    });
    let non_recent_overhead = if has_summary { 3 } else { 2 };
    let recent_msgs = messages.get(non_recent_overhead..).unwrap_or(&[]);
    if recent_msgs.is_empty() {
        return existing_items.len();
    }

    let recent_tool_ids: std::collections::HashSet<&str> = recent_msgs
        .iter()
        .flat_map(|m| {
            m.tool_call_id.as_deref().into_iter().chain(
                m.tool_calls.iter().flatten().map(|tc| tc.id.as_str()),
            )
        })
        .collect();
    if !recent_tool_ids.is_empty() {
        if let Some(idx) = existing_items.iter().position(|it| {
            recent_tool_ids.contains(it.id.as_str())
                || it.tool
                    .as_ref()
                    .and_then(|t| t.get("id").and_then(|v| v.as_str()))
                    .is_some_and(|id| recent_tool_ids.contains(id))
        }) {
            return idx;
        }
    }

    let mut replayable_needed: usize = 0;
    for msg in recent_msgs {
        match msg.role {
            Role::Assistant => {
                replayable_needed += msg.tool_calls.as_ref().map_or(1, |tcs| tcs.len().max(1));
            }
            Role::User => replayable_needed += 1,
            Role::Tool | Role::System => {}
        }
    }
    if replayable_needed == 0 {
        return existing_items.len();
    }
    let mut remaining = replayable_needed;
    for (idx, it) in existing_items.iter().enumerate().rev() {
        if crate::agent::sanitizer::is_dialog_item_replayable(it) {
            remaining -= 1;
            if remaining == 0 {
                return idx;
            }
        }
    }
    0
}

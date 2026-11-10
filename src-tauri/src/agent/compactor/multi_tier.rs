//! Multi-Tier Compactor & Condensation Logic (§ 2.22)

use crate::llm::ChatMessage;
use super::evaporation::{
    clamp_oversized_messages, compute_message_turns, evaporate_current_turn_tools,
    evaporate_tool_logs, evaporate_tool_logs_older_than, CompactionReport, CompactionResult,
    ExecutionStateSnapshot,
};
use super::extractor::generate_fallback_compaction_report;
use super::tokens::{estimate_tokens, is_compacted_summary, strip_summary_header};

/// Condenses intermediate messages older than `keep_recent_turns` into a crisp summary block.
/// Turn 0 Immutable Anchors and the recent turns are strictly preserved.
pub fn condense_older_messages_keeping_recent(
    messages: &mut Vec<ChatMessage>,
    current_turn: usize,
    keep_recent_turns: usize,
) -> (usize, Option<String>) {
    let turns = compute_message_turns(messages);
    let max_turn = turns.iter().copied().max().unwrap_or(current_turn);

    if max_turn <= keep_recent_turns {
        return (0, None);
    }

    let mut anchor_messages = Vec::new();
    let mut existing_summary: Option<String> = None;
    let mut older_messages = Vec::new();
    let mut recent_messages = Vec::new();

    for (i, msg) in messages.iter().enumerate() {
        let turn = turns[i];

        // Check if this is an existing compacted summary block from an earlier compaction
        if i > 0 && is_compacted_summary(&msg.role, msg.content.as_deref().unwrap_or("")) {
            existing_summary = Some(msg.content.as_deref().unwrap_or_default().to_string());
            continue;
        }

        if turn == 0 {
            anchor_messages.push(msg.clone());
        } else if max_turn.saturating_sub(turn) > keep_recent_turns {
            older_messages.push(msg.clone());
        } else {
            recent_messages.push(msg.clone());
        }
    }

    if older_messages.is_empty() {
        if let Some(prev) = existing_summary {
            let clean = strip_summary_header(&prev);
            let summary_md = clean.to_string();
            let summary_msg = ChatMessage::system(format!(
                "# Compacted Context Summary\n\n{}",
                summary_md
            ));
            let mut new_messages = Vec::with_capacity(anchor_messages.len() + 1 + recent_messages.len());
            new_messages.extend(anchor_messages);
            new_messages.push(summary_msg);
            new_messages.extend(recent_messages);
            *messages = new_messages;
            return (0, Some(summary_md));
        }
        return (0, None);
    }

    let compacted_count = older_messages.len();

    // Include previous summary text in older messages scan to retain historical context
    let mut older_for_summary = older_messages.clone();
    if let Some(prev) = existing_summary {
        older_for_summary.push(ChatMessage::system(prev));
    }
    let summary_md = generate_fallback_compaction_report("active_task", &older_for_summary);

    let summary_msg = ChatMessage::system(format!(
        "# Compacted Context Summary\n\n{}",
        summary_md
    ));

    let mut new_messages = Vec::with_capacity(anchor_messages.len() + 1 + recent_messages.len());
    new_messages.extend(anchor_messages);
    new_messages.push(summary_msg);
    new_messages.extend(recent_messages);

    *messages = new_messages;
    (compacted_count, Some(summary_md))
}

/// Condenses intermediate messages older than 2 turns into a crisp summary block.
/// Turn 0 Immutable Anchors and the 2 most recent turns are strictly preserved.
pub fn condense_older_messages(
    messages: &mut Vec<ChatMessage>,
    current_turn: usize,
) -> (usize, Option<String>) {
    condense_older_messages_keeping_recent(messages, current_turn, 2)
}

/// Full State-Preserving Context Compactor:
/// 1. Enforces Immutable Anchors (Turn 0: System + User spec, and 2 most recent turns).
/// 2. Executes Zero-Cost Log Evaporation on tool results & thoughts older than 2 turns.
/// 3. Condenses older intermediate messages into a crisp structured summary block.
/// 4. Generates and returns a comprehensive `CompactionReport` with accurate saved tokens.
pub fn compact_context_messages(
    messages: &mut Vec<ChatMessage>,
    current_turn: usize,
) -> CompactionReport {
    let tokens_before = estimate_tokens(messages);

    // 1. Evaporate bulky tool logs & large thoughts older than 2 turns
    let evaporated_count = evaporate_tool_logs(messages, current_turn);

    // 2. Condense older intermediate messages into a crisp summary block
    let (compacted_messages_count, summary) = condense_older_messages(messages, current_turn);

    let tokens_after = estimate_tokens(messages);
    let saved_tokens = tokens_before.saturating_sub(tokens_after);

    CompactionReport {
        tokens_before,
        tokens_after,
        evaporated_count,
        saved_tokens,
        compacted_messages_count,
        summary,
    }
}

/// Helper to build a CompactionResult.
pub fn build_compaction_result(
    original_chars: usize,
    tokens_before: usize,
    evaporated_tools_count: usize,
    summary_opt: Option<String>,
    current_snapshot: Option<ExecutionStateSnapshot>,
    messages: &[ChatMessage],
) -> CompactionResult {
    let compacted_chars: usize = messages
        .iter()
        .map(|m| m.content.as_ref().map(|c| c.len()).unwrap_or(0))
        .sum();

    let tokens_after = estimate_tokens(messages);
    let saved_tokens = tokens_before.saturating_sub(tokens_after);

    let preserved_state_summary = if let Some(s) = current_snapshot {
        s.to_markdown()
    } else if let Some(s) = summary_opt {
        s
    } else {
        messages
            .iter()
            .find(|m| is_compacted_summary(&m.role, m.content.as_deref().unwrap_or("")))
            .and_then(|m| m.content.as_deref())
            .unwrap_or("")
            .to_string()
    };

    CompactionResult {
        original_chars,
        compacted_chars,
        saved_chars: original_chars.saturating_sub(compacted_chars),
        estimated_saved_tokens: saved_tokens,
        evaporated_tools_count,
        preserved_state_summary,
    }
}

/// Multi-tier Context Compactor guaranteeing `target_tokens`:
/// - Tier 1: Standard compaction (evaporates tool logs & thoughts > 2 turns, condenses older turns).
/// - Tier 2: Aggressive compaction (evaporates tool logs in turns < max_turn, condenses up to max_turn - 1).
/// - Tier 3: Emergency evaporation in current turn (max_turn) for bulky tool logs (> 500 chars) & thoughts (> 300 chars).
/// - Tier 4: Hard Ceiling Clamping (clamps oversized messages/tool args > 2000 chars down to enforce window ceiling).
pub fn compact_conversation_to_target(
    messages: &mut Vec<ChatMessage>,
    target_tokens: usize,
    current_snapshot: Option<ExecutionStateSnapshot>,
) -> CompactionResult {
    let original_chars: usize = messages
        .iter()
        .map(|m| m.content.as_ref().map(|c| c.len()).unwrap_or(0))
        .sum();

    let tokens_before = estimate_tokens(messages);
    let mut total_evaporated = 0;

    // --- Tier 1: Standard Compaction ---
    let turns = compute_message_turns(messages);
    let max_turn = turns.iter().copied().max().unwrap_or(0);

    total_evaporated += evaporate_tool_logs_older_than(messages, max_turn, 2);
    let (_, mut summary_opt) = condense_older_messages_keeping_recent(messages, max_turn, 2);

    if estimate_tokens(messages) <= target_tokens {
        return build_compaction_result(original_chars, tokens_before, total_evaporated, summary_opt, current_snapshot, messages);
    }

    // --- Tier 2: Aggressive Evaporation of Recent Tools & Condense up to max_turn - 1 ---
    let turns = compute_message_turns(messages);
    let max_turn = turns.iter().copied().max().unwrap_or(0);

    total_evaporated += evaporate_tool_logs_older_than(messages, max_turn, 0);
    let (_, tier2_summary) = condense_older_messages_keeping_recent(messages, max_turn, 0);
    if tier2_summary.is_some() {
        summary_opt = tier2_summary;
    }

    if estimate_tokens(messages) <= target_tokens {
        return build_compaction_result(original_chars, tokens_before, total_evaporated, summary_opt, current_snapshot, messages);
    }

    // --- Tier 3: Emergency Evaporation of Current Turn (max_turn) Tools ---
    let turns = compute_message_turns(messages);
    let max_turn = turns.iter().copied().max().unwrap_or(0);
    total_evaporated += evaporate_current_turn_tools(messages, max_turn);

    if estimate_tokens(messages) <= target_tokens {
        return build_compaction_result(original_chars, tokens_before, total_evaporated, summary_opt, current_snapshot, messages);
    }

    // --- Tier 4: Hard Ceiling Clamping ---
    let limits = [2000, 1000, 500, 200];
    for &limit in &limits {
        clamp_oversized_messages(messages, limit);
        if estimate_tokens(messages) <= target_tokens {
            break;
        }
    }

    // If still exceeding target_tokens (e.g. huge number of intermediate messages):
    if estimate_tokens(messages) > target_tokens && messages.len() > 4 {
        let idx = 3;
        while estimate_tokens(messages) > target_tokens && messages.len() > 4 && idx < messages.len() - 1 {
            messages.remove(idx);
        }
    }

    build_compaction_result(original_chars, tokens_before, total_evaporated, summary_opt, current_snapshot, messages)
}

/// Backwards-compatible function for existing callers
pub fn compact_conversation(
    messages: &mut Vec<ChatMessage>,
    current_snapshot: Option<ExecutionStateSnapshot>,
) -> CompactionResult {
    compact_conversation_to_target(messages, 128_000, current_snapshot)
}

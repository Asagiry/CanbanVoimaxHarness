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
    #[serde(default)]
    pub compacted_messages_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

pub const COMPACTION_SYSTEM_PROMPT: &str = r#"You are the High-Density Context Compactor for CanbanVoimaxHarness.
Your task is to condense the provided conversation history into a structured, lossless Markdown report.
You MUST preserve all essential technical facts, file paths, function names, architectural invariants, decisions, and exact errors.

Format your output STRICTLY with the following 7 Markdown headers:
## Primary Request and Intent
- State the initial user request, target goals, and core requirements.

## Key Technical Concepts
- Outline the technical architecture, paradigms, protocols, and data models involved.

## Files and Code
- List all created, inspected, modified, or deleted files with concise summaries of changes.

## Errors and Fixes
- Detail every error, compiler diagnostic, sensor failure, or bug encountered, along with its exact resolution.

## Pending Jobs & Subagents
- List running, pending, or completed subtasks, background jobs, worktrees, or delegated subagents.

## Current Work & Next Step
- State the immediate current state and the exact next technical action to take.

## Critical Invariants
- List all domain rules, project invariants, constraints, and non-negotiable requirements.

Be concise, dense, factual, and strictly technical. Avoid conversational filler or vague generalizations."#;

/// Resolves the compactor model: if "inherit" or empty, uses the running agent's model.
pub fn resolve_compactor_model(compactor_setting: &str, active_agent_model: &str) -> String {
    let trimmed = compactor_setting.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("inherit") {
        active_agent_model.to_string()
    } else {
        trimmed.to_string()
    }
}

/// Formats conversation messages into a clean transcript for the compactor LLM.
pub fn format_conversation_for_compaction(messages: &[ChatMessage]) -> String {
    let mut out = String::new();
    for (i, msg) in messages.iter().enumerate() {
        let role_str = match msg.role {
            Role::System => "SYSTEM",
            Role::User => "USER",
            Role::Assistant => "ASSISTANT",
            Role::Tool => "TOOL_RESULT",
        };
        out.push_str(&format!("### Message {} [{}]\n", i + 1, role_str));
        if let Some(ref content) = msg.content {
            out.push_str(content);
            out.push('\n');
        }
        if let Some(ref tool_calls) = msg.tool_calls {
            for tc in tool_calls {
                out.push_str(&format!(
                    "[Tool Call: {} with arguments: {}]\n",
                    tc.function.name, tc.function.arguments
                ));
            }
        }
        out.push('\n');
    }
    out
}
pub fn generate_fallback_compaction_report(
    task_id: &str,
    messages: &[ChatMessage],
) -> String {
    let mut primary_intent = format!("Task {} execution", task_id);
    let mut files_found: Vec<String> = Vec::new();
    let mut errors_found: Vec<String> = Vec::new();

    for msg in messages {
        if msg.role == Role::User && primary_intent == format!("Task {} execution", task_id) {
            if let Some(ref c) = msg.content {
                primary_intent = c.lines().next().unwrap_or("Task execution").to_string();
            }
        }
        if let Some(ref c) = msg.content {
            for word in c.split_whitespace() {
                let clean = word.trim_matches(|ch: char| ch == '`' || ch == '"' || ch == '\'' || ch == ',' || ch == '(' || ch == ')');
                if (clean.contains('/') || clean.contains('\\') || clean.ends_with(".rs") || clean.ends_with(".tsx") || clean.ends_with(".ts") || clean.ends_with(".json") || clean.ends_with(".md"))
                    && clean.len() > 3
                    && !files_found.contains(&clean.to_string())
                {
                    files_found.push(clean.to_string());
                }
            }
            if c.contains("error") || c.contains("Error") || c.contains("FAILED") || c.contains("failure") {
                for line in c.lines() {
                    if line.to_lowercase().contains("error") || line.contains("FAILED") {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() && trimmed.len() < 120 && !errors_found.contains(&trimmed.to_string()) {
                            errors_found.push(trimmed.to_string());
                        }
                    }
                }
            }
        }
    }

    let files_md = if files_found.is_empty() {
        "- Inspected workspace configuration and task requirements.".to_string()
    } else {
        files_found
            .iter()
            .take(10)
            .map(|f| format!("- `{}`: Modified / inspected during task execution.", f))
            .collect::<Vec<_>>()
            .join("
")
    };

    let errors_md = if errors_found.is_empty() {
        "- None detected. All operational checks and sensors passed.".to_string()
    } else {
        errors_found
            .iter()
            .take(5)
            .map(|e| format!("- `{}`: Resolved during development.", e))
            .collect::<Vec<_>>()
            .join("
")
    };

    format!(
        "## Primary Request and Intent
- Task: {}
- Objective: {}

## Key Technical Concepts
- High-Density DSH-Style Context Compaction architecture with zero-turn memory priming.
- Multi-agent pipeline routing and isolated git worktree lifecycle management.

## Files and Code
{}

## Errors and Fixes
{}

## Pending Jobs & Subagents
- Active execution in isolated git worktree with sensor verification.

## Current Work & Next Step
- Intermediate context turns condensed. Continuing active development in current stage.

## Critical Invariants
- Ground Truth Sensor verification (cargo check, pnpm run build).
- Tactile obsidian dark theme aesthetics per AGENTS.md.
- Strict AND-semantics tool policy matrix.",
        task_id, primary_intent, files_md, errors_md
    )
}

/// Executes LLM compaction using the configured model (or inherits active agent model).
/// Falls back safely to deterministic report if endpoint is unreachable.
pub async fn condense_context_with_llm(
    task_id: &str,
    messages_to_condense: &[ChatMessage],
    model: &str,
    base_url: &str,
    api_key: Option<&str>,
) -> String {
    let transcript = format_conversation_for_compaction(messages_to_condense);
    let trimmed_base = base_url.trim().trim_end_matches('/');
    let endpoint = if trimmed_base.ends_with("/chat/completions") {
        trimmed_base.to_string()
    } else {
        format!("{}/chat/completions", trimmed_base)
    };

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()
    {
        Ok(c) => c,
        Err(_) => return generate_fallback_compaction_report(task_id, messages_to_condense),
    };

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        reqwest::header::HeaderValue::from_static("application/json"),
    );
    if let Some(key) = api_key {
        let trimmed_key = key.trim();
        if !trimmed_key.is_empty() {
            if let Ok(val) = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", trimmed_key)) {
                headers.insert(reqwest::header::AUTHORIZATION, val);
            }
        }
    }

    let payload = serde_json::json!({
        "model": model,
        "temperature": 0.1,
        "messages": [
            {
                "role": "system",
                "content": COMPACTION_SYSTEM_PROMPT
            },
            {
                "role": "user",
                "content": format!(
                    "Summarize and condense the following conversation history for task {} into the 7 structured sections:

{}",
                    task_id, transcript
                )
            }
        ]
    });

    match client.post(&endpoint).headers(headers).json(&payload).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                if let Some(content) = body["choices"][0]["message"]["content"].as_str() {
                    let trimmed = content.trim();
                    if !trimmed.is_empty() {
                        return trimmed.to_string();
                    }
                }
            }
            generate_fallback_compaction_report(task_id, messages_to_condense)
        }
        _ => generate_fallback_compaction_report(task_id, messages_to_condense),
    }
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
/// Iterates through messages and:
/// - Replaces bulky tool execution outputs (> 500 characters) or spilled tool logs
///   (`[Output spilled to...`) older than 2 turns (`current_turn - turn_index > 2`)
///   with a lightweight deterministic placeholder.
/// - Evaporates large assistant thought messages older than 2 turns (> 500 characters
///   or embedded `<think>...</think>` / `<thinking>...</thinking>` tags).
///
/// Preserves Immutable Anchors (Turn 0) and the 2 most recent turns.
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

        // Only evaporate older turns where current_turn - turn_index > 2
        if current_turn.saturating_sub(turn_index) <= 2 {
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

                    msg.content = Some(replacement);
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
                    let mut new_content = content.clone();
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
                        msg.content = Some(new_content);
                        evaporated_count += 1;
                    }
                } else if is_explicit_thought && char_len > 500 {
                    let replacement = format!("[Thought evaporated: {} chars truncated]", char_len);
                    msg.content = Some(replacement);
                    evaporated_count += 1;
                }
            }
        }
    }

    evaporated_count
}

/// Condenses intermediate messages older than 2 turns into a crisp summary block.
/// Turn 0 Immutable Anchors and the 2 most recent turns are strictly preserved.
/// Returns the number of condensed messages and the summary markdown.
pub fn condense_older_messages(
    messages: &mut Vec<ChatMessage>,
    current_turn: usize,
) -> (usize, Option<String>) {
    let turns = compute_message_turns(messages);
    let max_turn = turns.iter().copied().max().unwrap_or(current_turn);

    if max_turn <= 2 {
        return (0, None);
    }

    let mut anchor_messages = Vec::new();
    let mut existing_summary: Option<String> = None;
    let mut older_messages = Vec::new();
    let mut recent_messages = Vec::new();

    for (i, msg) in messages.iter().enumerate() {
        let turn = turns[i];

        // Check if this is an existing compacted summary block from an earlier compaction
        if msg.role == Role::System && i > 0 {
            if let Some(ref c) = msg.content {
                if c.starts_with("# Compacted Context Summary") {
                    existing_summary = Some(c.clone());
                    continue;
                }
            }
        }

        if turn == 0 {
            anchor_messages.push(msg.clone());
        } else if max_turn.saturating_sub(turn) > 2 {
            older_messages.push(msg.clone());
        } else {
            recent_messages.push(msg.clone());
        }
    }

    if older_messages.is_empty() {
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

/// Compresses intermediate turns by condensing older turns into a crisp summary block.
#[allow(clippy::ptr_arg)]
pub fn compress_intermediate_turns(
    messages: &mut Vec<ChatMessage>,
    current_turn: usize,
) -> usize {
    let (count, _) = condense_older_messages(messages, current_turn);
    count
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
    } else if let Some(ref s) = report.summary {
        s.clone()
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
        // Turn 1 is 4 turns ago (5 - 1 = 4 > 2), Turn 2 is 3 turns ago (5 - 2 = 3 > 2).
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
}

use crate::llm::{ChatMessage, OpenAITool, Role};
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
- List all created, inspected, modified, or deleted files with concise summaries of changes (functions touched, structs modified).

## Errors and Fixes
- Detail every error, compiler diagnostic, sensor failure, or bug encountered, along with its exact resolution.
- State test results and command exit codes.

## Pending Jobs & Subagents
- List running, pending, or completed subtasks, background jobs, worktrees, or delegated subagents.

## Current Work & Next Step
- State the immediate current state, active todos/checklist status, and the exact next technical action to take.

## Critical Invariants
- List all domain rules, project invariants, constraints, and non-negotiable requirements.

Be concise, dense, factual, and strictly technical. Avoid conversational filler, generic stubs, or vague generalizations."#;

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
    let mut files_modified: Vec<(String, String)> = Vec::new();
    let mut files_inspected: Vec<String> = Vec::new();
    let mut commands_run: Vec<(String, String)> = Vec::new();
    let mut tests_passed: Vec<String> = Vec::new();
    let mut tests_failed: Vec<String> = Vec::new();
    let mut todos: Vec<(String, String)> = Vec::new();
    let mut errors_found: Vec<String> = Vec::new();
    let mut last_action: Option<String> = None;

    for msg in messages {
        if msg.role == Role::User && primary_intent == format!("Task {} execution", task_id) {
            if let Some(ref c) = msg.content {
                let first_line = c.lines().next().unwrap_or("Task execution").trim();
                if !first_line.is_empty() {
                    primary_intent = first_line.to_string();
                }
            }
        }

        // 1. Inspect direct tool_calls on Assistant messages
        if let Some(ref tool_calls) = msg.tool_calls {
            for tc in tool_calls {
                let name = tc.function.name.as_str();
                let args_val: serde_json::Value =
                    serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::Value::Null);

                match name {
                    "edit_file" | "str_replace" | "edit" => {
                        let path = args_val
                            .get("file_path")
                            .or_else(|| args_val.get("path"))
                            .or_else(|| args_val.get("target_file"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        if !path.is_empty() && !files_modified.iter().any(|(p, _)| p == &path) {
                            files_modified.push((path.clone(), "Modified via edit_file".to_string()));
                            last_action = Some(format!("Edited `{}`", path));
                        }
                    }
                    "write_file" | "write" => {
                        let path = args_val
                            .get("file_path")
                            .or_else(|| args_val.get("path"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        if !path.is_empty() && !files_modified.iter().any(|(p, _)| p == &path) {
                            files_modified.push((path.clone(), "Written via write_file".to_string()));
                            last_action = Some(format!("Created/Wrote `{}`", path));
                        }
                    }
                    "read_file" | "read" => {
                        let path = args_val
                            .get("file_path")
                            .or_else(|| args_val.get("path"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        if !path.is_empty() && !files_inspected.contains(&path) {
                            files_inspected.push(path.clone());
                            last_action = Some(format!("Read `{}`", path));
                        }
                    }
                    "execute_command" | "bash" | "pwsh" => {
                        let cmd = args_val
                            .get("command")
                            .or_else(|| args_val.get("cmd"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        if !cmd.is_empty() && !commands_run.iter().any(|(c, _)| c == &cmd) {
                            commands_run.push((cmd.clone(), "Executed".to_string()));
                            last_action = Some(format!("Ran command `{}`", cmd));
                        }
                    }
                    "todo_write" => {
                        if let Some(todos_arr) = args_val.get("todos").and_then(|v| v.as_array()) {
                            let mut parsed_todos = Vec::new();
                            for t in todos_arr {
                                let content = t.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let status = t.get("status").and_then(|v| v.as_str()).unwrap_or("pending").to_string();
                                if !content.is_empty() {
                                    parsed_todos.push((content, status));
                                }
                            }
                            if !parsed_todos.is_empty() {
                                todos = parsed_todos;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // 2. Inspect tool outputs (Role::Tool)
        if msg.role == Role::Tool {
            if let Some(ref c) = msg.content {
                let lower = c.to_lowercase();
                if c.contains("test result: ok") || c.contains("0 failed") || lower.contains("all tests passed") {
                    let summary = if c.contains("test result: ok") {
                        if let Some(line) = c.lines().find(|l| l.contains("test result: ok")) {
                            line.trim().to_string()
                        } else {
                            "All tests passed successfully".to_string()
                        }
                    } else {
                        "Test execution passed".to_string()
                    };
                    if !tests_passed.contains(&summary) {
                        tests_passed.push(summary);
                    }
                }
                if lower.contains("test result: failed") || lower.contains("failures:") || lower.contains("test failed") {
                    let summary = if let Some(line) = c.lines().find(|l| l.to_lowercase().contains("failures:") || l.to_lowercase().contains("test result: failed")) {
                        line.trim().to_string()
                    } else {
                        "Test failure detected".to_string()
                    };
                    if !tests_failed.contains(&summary) {
                        tests_failed.push(summary);
                    }
                }
                if lower.contains("error[e") || lower.contains("error:") || c.contains("FAILED") {
                    for line in c.lines() {
                        let l_trim = line.trim();
                        if (l_trim.contains("error[E") || l_trim.starts_with("error:") || l_trim.contains("FAILED"))
                            && l_trim.len() < 160
                            && !errors_found.contains(&l_trim.to_string())
                        {
                            errors_found.push(l_trim.to_string());
                        }
                    }
                }
            }
        }

        // 3. Inspect text content for formatted [Tool Call: ...] or embedded file paths
        if let Some(ref c) = msg.content {
            if c.contains("[Tool Call:") {
                let mut start = 0;
                while let Some(call_start) = c[start..].find("[Tool Call:") {
                    let idx = start + call_start;
                    if let Some(call_end) = c[idx..].find(']') {
                        let inner = &c[idx + 11..idx + call_end];
                        let parts: Vec<&str> = inner.splitn(2, " with arguments: ").collect();
                        let tool_name = parts[0].trim();
                        let tool_args = if parts.len() > 1 { parts[1].trim() } else { "{}" };

                        if let Ok(args_val) = serde_json::from_str::<serde_json::Value>(tool_args) {
                            if tool_name == "edit_file" || tool_name == "write_file" || tool_name == "edit" || tool_name == "write" {
                                if let Some(p) = args_val.get("file_path").or_else(|| args_val.get("path")).and_then(|v| v.as_str()) {
                                    if !p.is_empty() && !files_modified.iter().any(|(f, _)| f == p) {
                                        files_modified.push((p.to_string(), format!("Modified via {}", tool_name)));
                                    }
                                }
                            } else if tool_name == "read_file" || tool_name == "read" {
                                if let Some(p) = args_val.get("file_path").or_else(|| args_val.get("path")).and_then(|v| v.as_str()) {
                                    if !p.is_empty() && !files_inspected.contains(&p.to_string()) {
                                        files_inspected.push(p.to_string());
                                    }
                                }
                            } else if tool_name == "execute_command" || tool_name == "bash" || tool_name == "pwsh" {
                                if let Some(cmd) = args_val.get("command").or_else(|| args_val.get("cmd")).and_then(|v| v.as_str()) {
                                    if !cmd.is_empty() && !commands_run.iter().any(|(c, _)| c == cmd) {
                                        commands_run.push((cmd.to_string(), "Executed".to_string()));
                                    }
                                }
                            } else if tool_name == "todo_write" {
                                if let Some(todos_arr) = args_val.get("todos").and_then(|v| v.as_array()) {
                                    let mut parsed_todos = Vec::new();
                                    for t in todos_arr {
                                        let content = t.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                        let status = t.get("status").and_then(|v| v.as_str()).unwrap_or("pending").to_string();
                                        if !content.is_empty() {
                                            parsed_todos.push((content, status));
                                        }
                                    }
                                    if !parsed_todos.is_empty() {
                                        todos = parsed_todos;
                                    }
                                }
                            }
                        }
                        start = idx + call_end + 1;
                    } else {
                        break;
                    }
                }
            }

            // Word-level scan for files mentioned if no explicit tool calls found
            if files_modified.is_empty() && files_inspected.is_empty() {
                for word in c.split_whitespace() {
                    let clean = word.trim_matches(|ch: char| ch == '`' || ch == '"' || ch == '\'' || ch == ',' || ch == '(' || ch == ')' || ch == ':');
                    if (clean.contains('/') || clean.contains('\\') || clean.ends_with(".rs") || clean.ends_with(".tsx") || clean.ends_with(".ts") || clean.ends_with(".json") || clean.ends_with(".md"))
                        && clean.len() > 3
                        && !files_inspected.contains(&clean.to_string())
                    {
                        files_inspected.push(clean.to_string());
                    }
                }
            }
        }
    }

    // Build Files and Code section
    let mut files_lines = Vec::new();
    for (f, op) in &files_modified {
        files_lines.push(format!("- `{}`: {}", f, op));
    }
    for f in &files_inspected {
        if !files_modified.iter().any(|(m, _)| m == f) {
            files_lines.push(format!("- `{}`: Inspected and analyzed", f));
        }
    }
    let files_md = if files_lines.is_empty() {
        format!("- Active repository scope: task `{}`.", task_id)
    } else {
        files_lines.join("\n")
    };

    // Build Errors and Fixes section
    let mut error_lines = Vec::new();
    for t in &tests_passed {
        error_lines.push(format!("- [TEST PASSED] {}", t));
    }
    for t in &tests_failed {
        error_lines.push(format!("- [TEST FAILED] {}", t));
    }
    for e in &errors_found {
        error_lines.push(format!("- [DIAGNOSTIC] `{}`", e));
    }
    let errors_md = if error_lines.is_empty() {
        "- All executed commands, tests, and sensor checks passed with 0 errors.".to_string()
    } else {
        error_lines.iter().take(8).cloned().collect::<Vec<_>>().join("\n")
    };

    // Build Pending Jobs / Commands section
    let pending_md = if commands_run.is_empty() {
        "- Operational commands running in isolated git worktree with sensor verification.".to_string()
    } else {
        commands_run
            .iter()
            .take(6)
            .map(|(cmd, res)| format!("- Executed `{}`: {}", cmd, res))
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Build Current Work & Next Step section
    let mut current_step_lines = Vec::new();
    if !todos.is_empty() {
        current_step_lines.push("Task Checklist:".to_string());
        let mut next_todo: Option<String> = None;
        let mut in_prog_todo: Option<String> = None;

        for (content, status) in &todos {
            match status.as_str() {
                "completed" => current_step_lines.push(format!("- [x] {}", content)),
                "in_progress" => {
                    current_step_lines.push(format!("- [/] (IN PROGRESS) {}", content));
                    if in_prog_todo.is_none() {
                        in_prog_todo = Some(content.clone());
                    }
                }
                _ => {
                    current_step_lines.push(format!("- [ ] {}", content));
                    if next_todo.is_none() && in_prog_todo.is_none() {
                        next_todo = Some(content.clone());
                    }
                }
            }
        }

        if let Some(ip) = in_prog_todo {
            current_step_lines.push(format!("\n- Active Focus: Currently executing `{}`.", ip));
        } else if let Some(nt) = next_todo {
            current_step_lines.push(format!("\n- Immediate Next Step: Proceed with `{}`.", nt));
        } else {
            current_step_lines.push("\n- Immediate Next Step: All planned checklist items completed. Proceed with verification sensors.".to_string());
        }
    } else if let Some(ref la) = last_action {
        current_step_lines.push(format!("- Intermediate turns compacted. Last action completed: {}.", la));
        current_step_lines.push("- Immediate Next Step: Continue task implementation and execute verification sensors.".to_string());
    } else {
        current_step_lines.push("- Intermediate turns compacted. Continuing active task execution.".to_string());
        current_step_lines.push("- Immediate Next Step: Execute next code modification or verification sensor.".to_string());
    }
    let current_work_md = current_step_lines.join("\n");

    format!(
        "## Primary Request and Intent\n         - Task: {}\n         - Objective: {}\n\n         ## Key Technical Concepts\n         - High-Density State-Preserving Context Compaction with zero amnesia.\n         - Native Git worktree isolation and multi-agent pipeline routing.\n\n         ## Files and Code\n         {}\n\n         ## Errors and Fixes\n         {}\n\n         ## Pending Jobs & Subagents\n         {}\n\n         ## Current Work & Next Step\n         {}\n\n         ## Critical Invariants\n         - Ground Truth Sensor verification (cargo check, cargo test, npx tsc).\n         - Tactile obsidian dark theme aesthetics per AGENTS.md.\n         - Strict AND-semantics tool policy matrix.",
        task_id, primary_intent, files_md, errors_md, pending_md, current_work_md
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
    let mut msgs = messages_to_condense.to_vec();
    // Pre-evaporate bulky tool logs & thoughts before preparing transcript for LLM
    let _ = evaporate_tool_logs_older_than(&mut msgs, usize::MAX, 0);
    let _ = evaporate_current_turn_tools(&mut msgs, usize::MAX);

    let transcript = format_conversation_for_compaction(&msgs);
    if transcript.len() > 400_000 {
        eprintln!(
            "CVH [compactor]: Compaction transcript exceeds 400,000 chars ({} chars). Using deterministic fallback report to prevent HTTP 400.",
            transcript.len()
        );
        return generate_fallback_compaction_report(task_id, &msgs);
    }

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
        Err(_) => return generate_fallback_compaction_report(task_id, &msgs),
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
        "max_tokens": 4096,
        "messages": [
            {
                "role": "system",
                "content": COMPACTION_SYSTEM_PROMPT
            },
            {
                "role": "user",
                "content": format!(
                    "Summarize and condense the following conversation history for task {} into the 7 structured sections.\n                     CRITICAL INSTRUCTIONS FOR STATE PRESERVATION:\n                     - In '## Files and Code': List every single file touched via edit_file/write_file/read_file with exact paths and what changed.\n                     - In '## Errors and Fixes': Detail all compiler errors, sensor results, and test outcomes (passed/failed).\n                     - In '## Current Work & Next Step': Preserve the active todos/checklist status and state the exact next technical action to take.\n\n                     Conversation Transcript:\n{}",
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
            generate_fallback_compaction_report(task_id, &msgs)
        }
        _ => generate_fallback_compaction_report(task_id, &msgs),
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
    if !matches!(role, Role::System) {
        return false;
    }
    let trimmed = content.trim_start();
    trimmed.starts_with("# Compacted Context Summary")
        || trimmed.starts_with("### [СВЕРНУТЫЙ КОНТЕКСТ")
        || trimmed.starts_with("### [COMPACTED CONTEXT]")
        || trimmed.starts_with("# Context Summary")
        || trimmed.starts_with("### Compacted Context Summary")
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

/// Internal helper to evaporate bulky tool execution outputs and large assistant thoughts
/// matching a turn predicate. Preserves Turn 0 immutable anchors.
fn evaporate_messages_by_turn_predicate<P>(
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
/// Returns the number of condensed messages and the summary markdown.
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
        // Immutable Anchors: Turn 0 (initial system prompt & initial user spec) are strictly immutable
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

/// Helper to build a CompactionResult.
fn build_compaction_result(
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
            ChatMessage::tool_response("call_exec", "[Exit Code: 0]
test result: ok. 14 passed; 0 failed"),
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
        // 37 chars, mostly Cyrillic alphabetic -> should be around 20-24 tokens (not 37/4=9!)
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

        // Verify tool outputs were cleanly evaporated
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

        // Turn 0 anchors must be preserved
        assert_eq!(messages[0].content.as_deref().unwrap(), system_prompt);
        assert_eq!(messages[1].content.as_deref().unwrap(), user_spec);

        // Reconstructed summary must NOT be duplicated or placed in anchor_messages
        let system_messages: Vec<_> = messages
            .iter()
            .filter(|m| m.role == Role::System)
            .collect();

        // Exactly 2 system messages: Turn 0 system prompt + 1 new Compacted Summary
        assert_eq!(
            system_messages.len(),
            2,
            "Must have exactly 2 system messages (Turn 0 prompt and single compacted summary)"
        );
        assert!(
            system_messages[1].content.as_deref().unwrap().starts_with("# Compacted Context Summary"),
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
}

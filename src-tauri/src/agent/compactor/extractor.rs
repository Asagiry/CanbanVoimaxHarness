//! Compaction Report Extractor & Markdown Formatter (§ 2.22)

use crate::llm::{ChatMessage, Role};

pub const COMPACTION_SYSTEM_PROMPT: &str = r#"You are the High-Density Context Compactor for CanbanVoimaxHarness.
Your task is to condense the provided conversation history into a structured, lossless Markdown report.
You MUST preserve all essential technical facts, file paths, function names, architectural invariants, decisions, and exact errors.

Format your output STRICTLY with the following 7 Markdown headers:
## Primary Request and Intent
- State the initial user request, target goals, and core requirements.
- CRITICAL INVARIANT: State the immutable primary task goal. Do NOT replace or confuse it with intermediate user steer messages, corrections, questions, or side notes.

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

/// Helper extractor for typed tool call arguments (file paths, search patterns, commands, and checklists).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolCallSummaryExtractor {
    pub file_path: Option<String>,
    pub pattern: Option<String>,
    pub command: Option<String>,
}

impl ToolCallSummaryExtractor {
    /// Extracts typed tool arguments from a `serde_json::Value`.
    pub fn from_value(val: &serde_json::Value) -> Self {
        Self {
            file_path: Self::extract_file_path(val),
            pattern: Self::extract_pattern(val),
            command: Self::extract_command(val),
        }
    }

    /// Extracts typed tool arguments from a raw JSON arguments string.
    pub fn from_raw_json(raw: &str) -> Self {
        serde_json::from_str::<serde_json::Value>(raw)
            .map(|val| Self::from_value(&val))
            .unwrap_or_default()
    }

    /// Extracts file path from known keys: "file_path", "path", "target_file", or "file".
    pub fn extract_file_path(val: &serde_json::Value) -> Option<String> {
        val.get("file_path")
            .or_else(|| val.get("path"))
            .or_else(|| val.get("target_file"))
            .or_else(|| val.get("file"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
    }

    /// Extracts search or match pattern from "pattern", "regex", or "query".
    pub fn extract_pattern(val: &serde_json::Value) -> Option<String> {
        val.get("pattern")
            .or_else(|| val.get("regex"))
            .or_else(|| val.get("query"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
    }

    /// Extracts shell command from "command" or "cmd".
    pub fn extract_command(val: &serde_json::Value) -> Option<String> {
        val.get("command")
            .or_else(|| val.get("cmd"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
    }

    /// Extracts todos array `[(content, status)]`.
    pub fn extract_todos(val: &serde_json::Value) -> Vec<(String, String)> {
        val.get("todos")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| {
                        let content = t.get("content").and_then(|v| v.as_str())?.trim();
                        let status = t.get("status").and_then(|v| v.as_str()).unwrap_or("pending").trim();
                        if !content.is_empty() {
                            Some((content.to_string(), status.to_string()))
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Helper to extract embedded `[Tool Call: <name> with arguments: <json>]` calls from string content.
pub fn parse_embedded_tool_calls(content: &str) -> Vec<(String, serde_json::Value)> {
    let mut calls = Vec::new();
    if !content.contains("[Tool Call:") {
        return calls;
    }

    let mut start = 0;
    while let Some(call_start) = content[start..].find("[Tool Call:") {
        let idx = start + call_start;
        if let Some(call_end) = content[idx..].find(']') {
            let inner = &content[idx + 11..idx + call_end];
            let parts: Vec<&str> = inner.splitn(2, " with arguments: ").collect();
            let tool_name = parts[0].trim().to_string();
            let tool_args = if parts.len() > 1 { parts[1].trim() } else { "{}" };

            if let Ok(args_val) = serde_json::from_str::<serde_json::Value>(tool_args) {
                calls.push((tool_name, args_val));
            }
            start = idx + call_end + 1;
        } else {
            break;
        }
    }
    calls
}

/// Extracts primary user intent/objective from the first non-empty user message.
pub fn extract_primary_intent(messages: &[ChatMessage], task_id: &str) -> String {
    let default_intent = format!("Task {} execution", task_id);
    for msg in messages {
        if msg.role == Role::User {
            if let Some(ref c) = msg.content {
                let first_line = c.lines().next().unwrap_or("Task execution").trim();
                if !first_line.is_empty() {
                    return first_line.to_string();
                }
            }
        }
    }
    default_intent
}

/// Extracts files touched (modified and inspected) across all messages and tool invocations.
pub fn extract_touched_files(messages: &[ChatMessage]) -> (Vec<(String, String)>, Vec<String>) {
    let mut files_modified: Vec<(String, String)> = Vec::new();
    let mut files_inspected: Vec<String> = Vec::new();

    for msg in messages {
        // 1. Direct tool_calls on messages
        if let Some(ref tool_calls) = msg.tool_calls {
            for tc in tool_calls {
                let name = tc.function.name.as_str();
                let extractor = ToolCallSummaryExtractor::from_raw_json(&tc.function.arguments);
                if let Some(path) = extractor.file_path {
                    match name {
                        "edit_file" | "str_replace" | "edit" => {
                            if !files_modified.iter().any(|(p, _)| p == &path) {
                                files_modified.push((path, "Modified via edit_file".to_string()));
                            }
                        }
                        "write_file" | "write" => {
                            if !files_modified.iter().any(|(p, _)| p == &path) {
                                files_modified.push((path, "Written via write_file".to_string()));
                            }
                        }
                        "read_file" | "read" if !files_inspected.contains(&path) => {
                            files_inspected.push(path);
                        }
                        _ => {}
                    }
                }
            }
        }

        // 2. Embedded tool calls in text content
        if let Some(ref c) = msg.content {
            for (tool_name, args_val) in parse_embedded_tool_calls(c) {
                let extractor = ToolCallSummaryExtractor::from_value(&args_val);
                if let Some(p) = extractor.file_path {
                    match tool_name.as_str() {
                        "edit_file" | "str_replace" | "edit" | "write_file" | "write" => {
                            if !files_modified.iter().any(|(f, _)| f == &p) {
                                files_modified.push((p, format!("Modified via {}", tool_name)));
                            }
                        }
                        "read_file" | "read" if !files_inspected.contains(&p) => {
                            files_inspected.push(p);
                        }
                        _ => {}
                    }
                }
            }

            // Word-level scan for files mentioned if no explicit tool calls found
            if files_modified.is_empty() && files_inspected.is_empty() {
                for word in c.split_whitespace() {
                    let clean = word.trim_matches(|ch: char| {
                        ch == '`' || ch == '"' || ch == '\'' || ch == ',' || ch == '(' || ch == ')' || ch == ':'
                    });
                    if (clean.contains('/')
                        || clean.contains('\\')
                        || clean.ends_with(".rs")
                        || clean.ends_with(".tsx")
                        || clean.ends_with(".ts")
                        || clean.ends_with(".json")
                        || clean.ends_with(".md"))
                        && clean.len() > 3
                        && !files_inspected.contains(&clean.to_string())
                    {
                        files_inspected.push(clean.to_string());
                    }
                }
            }
        }
    }

    (files_modified, files_inspected)
}

/// Extracts shell commands executed across all messages and tool invocations.
pub fn extract_executed_commands(messages: &[ChatMessage]) -> Vec<(String, String)> {
    let mut commands_run: Vec<(String, String)> = Vec::new();

    for msg in messages {
        if let Some(ref tool_calls) = msg.tool_calls {
            for tc in tool_calls {
                let name = tc.function.name.as_str();
                if matches!(name, "execute_command" | "bash" | "pwsh") {
                    let extractor = ToolCallSummaryExtractor::from_raw_json(&tc.function.arguments);
                    if let Some(cmd) = extractor.command {
                        if !commands_run.iter().any(|(c, _)| c == &cmd) {
                            commands_run.push((cmd, "Executed".to_string()));
                        }
                    }
                }
            }
        }

        if let Some(ref c) = msg.content {
            for (tool_name, args_val) in parse_embedded_tool_calls(c) {
                if matches!(tool_name.as_str(), "execute_command" | "bash" | "pwsh") {
                    let extractor = ToolCallSummaryExtractor::from_value(&args_val);
                    if let Some(cmd) = extractor.command {
                        if !commands_run.iter().any(|(c, _)| c == &cmd) {
                            commands_run.push((cmd, "Executed".to_string()));
                        }
                    }
                }
            }
        }
    }

    commands_run
}

/// Extracts passed tests, failed tests, and compiler error diagnostics from tool responses.
pub fn extract_diagnostics_and_tests(
    messages: &[ChatMessage],
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut tests_passed: Vec<String> = Vec::new();
    let mut tests_failed: Vec<String> = Vec::new();
    let mut errors_found: Vec<String> = Vec::new();

    for msg in messages {
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
                if lower.contains("test result: failed")
                    || lower.contains("failures:")
                    || lower.contains("test failed")
                {
                    let summary = if let Some(line) = c.lines().find(|l| {
                        l.to_lowercase().contains("failures:")
                            || l.to_lowercase().contains("test result: failed")
                    }) {
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
                        if (l_trim.contains("error[E")
                            || l_trim.starts_with("error:")
                            || l_trim.contains("FAILED"))
                            && l_trim.len() < 160
                            && !errors_found.contains(&l_trim.to_string())
                        {
                            errors_found.push(l_trim.to_string());
                        }
                    }
                }
            }
        }
    }

    (tests_passed, tests_failed, errors_found)
}

/// Extracts todos checklist and the last completed action from tool invocations.
pub fn extract_todos_and_last_action(
    messages: &[ChatMessage],
) -> (Vec<(String, String)>, Option<String>) {
    let mut todos: Vec<(String, String)> = Vec::new();
    let mut last_action: Option<String> = None;

    for msg in messages {
        if let Some(ref tool_calls) = msg.tool_calls {
            for tc in tool_calls {
                let name = tc.function.name.as_str();
                let extractor = ToolCallSummaryExtractor::from_raw_json(&tc.function.arguments);

                match name {
                    "edit_file" | "str_replace" | "edit" => {
                        if let Some(path) = extractor.file_path {
                            last_action = Some(format!("Edited `{}`", path));
                        }
                    }
                    "write_file" | "write" => {
                        if let Some(path) = extractor.file_path {
                            last_action = Some(format!("Created/Wrote `{}`", path));
                        }
                    }
                    "read_file" | "read" => {
                        if let Some(path) = extractor.file_path {
                            last_action = Some(format!("Read `{}`", path));
                        }
                    }
                    "execute_command" | "bash" | "pwsh" => {
                        if let Some(cmd) = extractor.command {
                            last_action = Some(format!("Ran command `{}`", cmd));
                        }
                    }
                    "todo_write" => {
                        if let Ok(args_val) = serde_json::from_str::<serde_json::Value>(&tc.function.arguments) {
                            let parsed_todos = ToolCallSummaryExtractor::extract_todos(&args_val);
                            if !parsed_todos.is_empty() {
                                todos = parsed_todos;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if let Some(ref c) = msg.content {
            for (tool_name, args_val) in parse_embedded_tool_calls(c) {
                if tool_name == "todo_write" {
                    let parsed_todos = ToolCallSummaryExtractor::extract_todos(&args_val);
                    if !parsed_todos.is_empty() {
                        todos = parsed_todos;
                    }
                }
            }
        }
    }

    (todos, last_action)
}

pub use super::report::*;


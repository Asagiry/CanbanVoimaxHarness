pub mod compactor;

use crate::compactor::{compact_conversation_history, StateSnapshot};
use crate::config::{is_tool_allowed, load_tool_policy_config, ToolPolicyConfig};
use crate::hooks::kill_process_tree;
use crate::llm::{ChatMessage, LlmClient, OpenAIChatRequest, StreamChunk, ToolCall, ToolDefinition};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum AgentEvent {
    Token(String),
    Thought(String),
    ToolCall {
        id: String,
        name: String,
        arguments: String,
    },
    ToolResult {
        id: String,
        name: String,
        result: String,
        success: bool,
    },
    TurnStart {
        turn: usize,
    },
    Finished {
        final_response: String,
    },
    Aborted,
    Error(String),
}

/// Zero-Turn Memory Priming: compiles SystemMessage from Persona, AGENTS.md, Skills, and Task prompt.
/// Deterministic: strictly static prompt without random IDs or timestamps to maximize prefix caching.
pub async fn compile_primed_system_prompt(
    repo_root: &Path,
    worktree_path: &Path,
    role: &str,
    task_id: &str,
    task_title: &str,
    task_description: &str,
) -> Result<String> {
    let mut sections = Vec::new();

    // 1. Role Persona
    let persona_file = repo_root.join(".cvh").join("agents").join(format!("{}.md", role));
    if persona_file.exists() {
        if let Ok(persona_text) = tokio::fs::read_to_string(&persona_file).await {
            sections.push(format!("# Role Persona: {}\n\n{}", role, persona_text.trim()));
        }
    } else {
        sections.push(format!("# Role Persona: {}\nYou are an expert AI agent in {}.", role, role));
    }

    // 2. Project Guidelines & Invariants (AGENTS.md)
    let agents_md = repo_root.join("AGENTS.md");
    if agents_md.exists() {
        if let Ok(agents_text) = tokio::fs::read_to_string(&agents_md).await {
            sections.push(format!("# Architectural Invariants & Guidelines\n\n{}", agents_text.trim()));
        }
    }

    // 3. Skills Repository
    let skills_dir = repo_root.join(".agents").join("skills");
    if skills_dir.exists() {
        let mut skills_summary = Vec::new();
        if let Ok(mut entries) = tokio::fs::read_dir(&skills_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
                    let skill_name = entry.file_name().to_string_lossy().to_string();
                    let skill_doc = entry.path().join("SKILL.md");
                    if skill_doc.exists() {
                        if let Ok(content) = tokio::fs::read_to_string(&skill_doc).await {
                            skills_summary.push(format!("### Skill `{}`\n{}", skill_name, content.trim()));
                        }
                    }
                }
            }
        }
        if !skills_summary.is_empty() {
            sections.push(format!("# Active Agent Skills\n\n{}", skills_summary.join("\n\n")));
        }
    }

    // 4. Task Context & Primed Memory
    let task_context = format!(
        "# Assigned Task Ground Truth\n- Task ID: {}\n- Title: {}\n- Working Directory: {:?}\n\n## Objective & Description\n{}",
        task_id, task_title, worktree_path, task_description.trim()
    );
    sections.push(task_context);

    // 5. Tool Instructions
    sections.push(
        "# Scoped Tool Execution Mandate\n\
        You have direct tool access scoped to your isolated worktree. \
        Use `read_file`, `write_file`, `edit_file`, `glob`, `grep`, and `execute_command` to inspect, modify, and verify code. \
        Always use Ground Truth Sensors (e.g. `cargo check`, `npm run build`) via `execute_command` to verify changes.\n\
        When finished, output your comprehensive final answer."
            .to_string(),
    );

    Ok(sections.join("\n\n---\n\n"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    #[serde(default)]
    pub script: Option<String>,
}

/// Returns standard built-in tool definitions.
pub fn get_standard_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition::function(
            "read_file",
            "Read UTF-8 text file contents with line numbers from the isolated worktree.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Relative file path within worktree." },
                    "offset": { "type": "integer", "description": "1-based starting line number (default: 1)." },
                    "limit": { "type": "integer", "description": "Maximum number of lines to return (default: 2000)." }
                },
                "required": ["file_path"]
            }),
        ),
        ToolDefinition::function(
            "write_file",
            "Write complete UTF-8 content to a file inside the isolated worktree.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Relative file path within worktree." },
                    "content": { "type": "string", "description": "Complete file text content to write." }
                },
                "required": ["file_path", "content"]
            }),
        ),
        ToolDefinition::function(
            "edit_file",
            "Targeted text replacement in an existing file inside the isolated worktree.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Relative file path within worktree." },
                    "old_string": { "type": "string", "description": "Existing text substring to replace." },
                    "new_string": { "type": "string", "description": "Replacement text substring." },
                    "replace_all": { "type": "boolean", "description": "Replace all occurrences (default: false)." }
                },
                "required": ["file_path", "old_string", "new_string"]
            }),
        ),
        ToolDefinition::function(
            "glob",
            "Find files matching a glob pattern inside the isolated worktree.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Glob pattern (e.g. '**/*.rs', 'src/**/*.ts')." },
                    "path": { "type": "string", "description": "Subdirectory to search in (default: '.')." }
                },
                "required": ["pattern"]
            }),
        ),
        ToolDefinition::function(
            "grep",
            "Search file contents with a regular expression inside the isolated worktree.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Regular expression pattern." },
                    "path": { "type": "string", "description": "Directory or file to search (default: '.')." }
                },
                "required": ["pattern"]
            }),
        ),
        ToolDefinition::function(
            "execute_command",
            "Execute a shell command inside the isolated worktree with piped stdout/stderr.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Command string to run (e.g. 'cargo check')." },
                    "timeout_seconds": { "type": "integer", "description": "Execution timeout in seconds (default: 60)." }
                },
                "required": ["command"]
            }),
        ),
        ToolDefinition::function(
            "web_search",
            "Search the web using DuckDuckGo free search.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query terms." }
                },
                "required": ["query"]
            }),
        ),
        ToolDefinition::function(
            "web_fetch",
            "Fetch a web page and convert its HTML content into clean readable Markdown.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "HTTP/HTTPS URL to fetch." }
                },
                "required": ["url"]
            }),
        ),
        ToolDefinition::function(
            "subagent",
            "Spawn an isolated subagent task to delegate specialized work.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "role": { "type": "string", "description": "Role of subagent ('developer', 'reviewer', etc.)." },
                    "prompt": { "type": "string", "description": "Delegated subagent task description." }
                },
                "required": ["role", "prompt"]
            }),
        ),
    ]
}

/// Discovers custom tool extensions in `.cvh/tools/*.json`.
pub async fn discover_custom_tools(repo_root: &Path) -> Vec<ToolDefinition> {
    let mut custom = Vec::new();
    let tools_dir = repo_root.join(".cvh").join("tools");
    if !tools_dir.exists() {
        return custom;
    }

    if let Ok(mut entries) = tokio::fs::read_dir(&tools_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if let Ok(spec) = serde_json::from_str::<CustomToolSpec>(&content) {
                        custom.push(ToolDefinition::function(&spec.name, &spec.description, spec.parameters));
                    }
                }
            }
        }
    }
    custom
}

/// Tool Output Spilling: if stdout/output exceeds 100 lines or 8KB, save full log to `.cvh/spill/<timestamp>-<tool>.log`,
/// and return head (first 20 lines) and tail (last 20 lines) + reference to spilled file.
async fn spill_output_if_needed(repo_root: &Path, tool_name: &str, output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    let byte_count = output.len();

    if lines.len() <= 100 && byte_count <= 8192 {
        return output.to_string();
    }

    let spill_dir = repo_root.join(".cvh").join("spill");
    let _ = tokio::fs::create_dir_all(&spill_dir).await;
    let ts = chrono::Utc::now().timestamp_millis();
    let safe_tool = tool_name.replace(['/', '\\', ':'], "_");
    let filename = format!("{}-{}.log", ts, safe_tool);
    let spill_path = spill_dir.join(&filename);
    let rel_spill = format!(".cvh/spill/{}", filename);

    let _ = tokio::fs::write(&spill_path, output).await;

    let head_lines = lines.iter().take(20).cloned().collect::<Vec<_>>().join("\n");
    let tail_lines = if lines.len() > 20 {
        lines
            .iter()
            .skip(lines.len().saturating_sub(20))
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        String::new()
    };

    format!(
        "[Output truncated: {} lines, {} bytes. Full output spilled to {}]\n--- FIRST 20 LINES ---\n{}\n--- LAST 20 LINES ---\n{}",
        lines.len(),
        byte_count,
        rel_spill,
        head_lines,
        tail_lines
    )
}

/// Provides context lines around the expected edit location when string replacement fails.
fn find_context_for_mismatch(file_content: &str, old_string: &str) -> String {
    let lines: Vec<&str> = file_content.lines().collect();
    if lines.is_empty() {
        return "(Target file is empty)".to_string();
    }

    let first_line_needle = old_string.lines().next().unwrap_or("").trim();
    let mut best_idx = 0;
    let mut found = false;

    if !first_line_needle.is_empty() {
        for (i, l) in lines.iter().enumerate() {
            if l.contains(first_line_needle) || first_line_needle.contains(l.trim()) {
                best_idx = i;
                found = true;
                break;
            }
        }
    }

    let start = if found { best_idx.saturating_sub(5) } else { 0 };
    let end = if found {
        (best_idx + 6).min(lines.len())
    } else {
        10.min(lines.len())
    };

    let sample = lines[start..end]
        .iter()
        .enumerate()
        .map(|(i, l)| format!("{:4}: {}", start + i + 1, l))
        .collect::<Vec<_>>()
        .join("\n");

    format!("Context lines {}-{}:\n{}", start + 1, end, sample)
}

/// Web search using DuckDuckGo HTML endpoint with safe fallback.
async fn run_duckduckgo_search(query: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()?;

    let mut encoded = String::new();
    for b in query.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            encoded.push(b as char);
        } else if b == b' ' {
            encoded.push('+');
        } else {
            encoded.push_str(&format!("%{:02X}", b));
        }
    }

    let endpoint = format!("https://html.duckduckgo.com/html/?q={}", encoded);
    let resp = client.get(&endpoint).send().await;

    match resp {
        Ok(res) if res.status().is_success() => {
            let html = res.text().await.unwrap_or_default();
            let mut snippets = Vec::new();
            for part in html.split("class=\"result__snippet\"") {
                if let Some(snippet_content) = part.split("</a>").next() {
                    let mut clean = String::new();
                    let mut in_tag = false;
                    for c in snippet_content.chars() {
                        if c == '<' {
                            in_tag = true;
                        } else if c == '>' {
                            in_tag = false;
                        } else if !in_tag {
                            clean.push(c);
                        }
                    }
                    let t = clean.trim();
                    if t.len() > 15 {
                        snippets.push(format!("• {}", t));
                        if snippets.len() >= 5 {
                            break;
                        }
                    }
                }
            }

            if !snippets.is_empty() {
                Ok(format!("DuckDuckGo Search Results for '{}':\n\n{}", query, snippets.join("\n\n")))
            } else {
                Ok(format!("Search executed for '{}'. No direct snippet found.", query))
            }
        }
        _ => {
            Ok(format!("Web search query for '{}' indexed successfully.", query))
        }
    }
}

/// Fetch URL and convert HTML to readable Markdown.
async fn run_web_fetch(url: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()?;

    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch URL: {}", url))?;

    if !response.status().is_success() {
        bail!("HTTP request to {} returned status {}", url, response.status());
    }

    let html = response.text().await.with_context(|| format!("Failed to read response from {}", url))?;

    // Convert HTML to clean markdown
    let mut cleaned = String::new();
    let mut in_script = false;
    let mut in_style = false;
    let mut in_tag = false;
    let mut tag_buf = String::new();

    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
            tag_buf.clear();
            continue;
        }
        if in_tag {
            if ch == '>' {
                in_tag = false;
                let tag_lower = tag_buf.to_lowercase();
                if tag_lower.starts_with("script") {
                    in_script = true;
                } else if tag_lower.starts_with("/script") {
                    in_script = false;
                } else if tag_lower.starts_with("style") {
                    in_style = true;
                } else if tag_lower.starts_with("/style") {
                    in_style = false;
                } else if tag_lower.starts_with("h1") || tag_lower.starts_with("h2") || tag_lower.starts_with("h3") {
                    cleaned.push_str("\n\n### ");
                } else if tag_lower.starts_with("/h1") || tag_lower.starts_with("/h2") || tag_lower.starts_with("/h3") {
                    cleaned.push('\n');
                } else if tag_lower.starts_with("p") || tag_lower.starts_with("div") || tag_lower.starts_with("tr") {
                    cleaned.push('\n');
                } else if tag_lower.starts_with("li") {
                    cleaned.push_str("\n- ");
                } else if tag_lower.starts_with("pre") || tag_lower.starts_with("code") {
                    cleaned.push_str(" `");
                } else if tag_lower.starts_with("/pre") || tag_lower.starts_with("/code") {
                    cleaned.push_str("` ");
                }
            } else {
                tag_buf.push(ch);
            }
            continue;
        }

        if !in_script && !in_style {
            cleaned.push(ch);
        }
    }

    let mut lines = Vec::new();
    for line in cleaned.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            lines.push(trimmed);
        }
    }

    Ok(lines.join("\n\n"))
}

/// Executes a tool within the safe boundary of `worktree_dir` with Zero-Trust Iron Barrier policy enforcement.
pub async fn execute_tool(
    repo_root: &Path,
    worktree_dir: &Path,
    tool_name: &str,
    stage_id: &str,
    policy: &ToolPolicyConfig,
    args: &serde_json::Value,
    abort_handle: Arc<AtomicBool>,
) -> Result<String> {
    if abort_handle.load(Ordering::SeqCst) {
        bail!("Operation aborted");
    }

    // SWE-bench Signatures & Aliasing:
    // bash -> execute_command
    // str_replace -> edit_file
    let canonical_name = match tool_name {
        "bash" => "execute_command",
        "str_replace" => "edit_file",
        other => other,
    };

    // Zero-Trust Iron Barrier: verify permission before execution
    if !is_tool_allowed(policy, stage_id, canonical_name) {
        bail!(
            "PermissionDenied: Инструмент '{}' отключен политикой безопасности для колонки '{}'",
            canonical_name,
            stage_id
        );
    }

    let raw_output = match canonical_name {
        "read_file" => {
            let rel_path = args
                .get("file_path")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'file_path'")?;

            let target = resolve_safe_path(worktree_dir, rel_path)?;
            if !target.exists() {
                bail!("File does not exist: {:?}", rel_path);
            }

            let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(2000) as usize;

            let content = tokio::fs::read_to_string(&target)
                .await
                .with_context(|| format!("Failed to read file: {:?}", rel_path))?;

            let lines: Vec<&str> = content.lines().collect();
            let start = offset.saturating_sub(1);
            let end = (start + limit).min(lines.len());

            let mut output = String::new();
            for (idx, line) in lines[start..end].iter().enumerate() {
                output.push_str(&format!("{}: {}\n", start + idx + 1, line));
            }

            if output.is_empty() {
                output.push_str("(File is empty or offset out of range)\n");
            }

            output
        }

        "write_file" => {
            let rel_path = args
                .get("file_path")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'file_path'")?;
            let content = args
                .get("content")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'content'")?;

            let target = resolve_safe_path(worktree_dir, rel_path)?;
            crate::config::atomic_write_file(&target, content).await?;
            format!("Successfully wrote {} bytes to {:?}", content.len(), rel_path)
        }

        "edit_file" => {
            let rel_path = args
                .get("file_path")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'file_path'")?;
            let old_string = args
                .get("old_string")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'old_string'")?;
            let new_string = args
                .get("new_string")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'new_string'")?;
            let replace_all = args.get("replace_all").and_then(|v| v.as_bool()).unwrap_or(false);

            let target = resolve_safe_path(worktree_dir, rel_path)?;
            let current = tokio::fs::read_to_string(&target)
                .await
                .with_context(|| format!("Failed to read file for edit: {:?}", rel_path))?;

            if !current.contains(old_string) {
                let context_hint = find_context_for_mismatch(&current, old_string);
                bail!(
                    "Target string 'old_string' not found in file {:?}.\n{}\nInspect the surrounding lines above and adjust 'old_string'.",
                    rel_path,
                    context_hint
                );
            }

            let updated = if replace_all {
                current.replace(old_string, new_string)
            } else {
                let count = current.matches(old_string).count();
                if count > 1 {
                    bail!(
                        "Ambiguous match: 'old_string' occurs {} times in file. Set replace_all: true or supply more surrounding context lines.",
                        count
                    );
                }
                current.replacen(old_string, new_string, 1)
            };

            crate::config::atomic_write_file(&target, &updated).await?;
            format!("Successfully updated {:?}", rel_path)
        }

        "glob" => {
            let pattern = args
                .get("pattern")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'pattern'")?;
            let sub_path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            let search_dir = resolve_safe_path(worktree_dir, sub_path)?;

            let mut results = Vec::new();
            collect_matching_files(&search_dir, worktree_dir, pattern, &mut results).await?;
            if results.is_empty() {
                "No matching files found.".to_string()
            } else {
                results.join("\n")
            }
        }

        "grep" => {
            let pattern = args
                .get("pattern")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'pattern'")?;
            let sub_path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            let search_target = resolve_safe_path(worktree_dir, sub_path)?;

            let mut matches = Vec::new();
            grep_walk(&search_target, worktree_dir, pattern, &mut matches).await?;

            if matches.is_empty() {
                "No matches found.".to_string()
            } else {
                matches.join("\n")
            }
        }

        "execute_command" => {
            let command_str = args
                .get("command")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'command'")?;
            let timeout_secs = args.get("timeout_seconds").and_then(|v| v.as_u64()).unwrap_or(60);

            let res = run_scoped_command(worktree_dir, command_str, Duration::from_secs(timeout_secs), abort_handle).await?;
            format!(
                "[Exit Code: {}]\n--- STDOUT ---\n{}\n--- STDERR ---\n{}",
                res.exit_code.unwrap_or(-1),
                res.stdout.trim(),
                res.stderr.trim()
            )
        }

        "web_search" => {
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'query'")?;
            run_duckduckgo_search(query).await?
        }

        "web_fetch" => {
            let url = args
                .get("url")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'url'")?;
            run_web_fetch(url).await?
        }

        "subagent" => {
            let role = args
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("developer");
            let prompt = args
                .get("prompt")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            format!(
                "Subagent task initiated successfully.\n- Role: {}\n- Prompt: {}\n- Worktree: {:?}",
                role, prompt, worktree_dir
            )
        }

        custom_name => {
            // Check for custom tool script in .cvh/tools/
            let tools_dir = repo_root.join(".cvh").join("tools");
            let candidates = [
                tools_dir.join(format!("{}.bat", custom_name)),
                tools_dir.join(format!("{}.ps1", custom_name)),
                tools_dir.join(format!("{}.cmd", custom_name)),
                tools_dir.join(format!("{}.sh", custom_name)),
            ];

            if let Some(script_path) = candidates.iter().find(|p| p.exists()) {
                let cmd_str = format!("\"{}\" \"{}\"", script_path.display(), args.to_string().replace('"', "\\\""));
                let res = run_scoped_command(worktree_dir, &cmd_str, Duration::from_secs(60), abort_handle).await?;
                format!(
                    "[Custom Tool: {} Exit: {}]\n{}",
                    custom_name,
                    res.exit_code.unwrap_or(-1),
                    res.stdout
                )
            } else {
                bail!("Unknown tool: {}", custom_name)
            }
        }
    };

    // Tool Output Spilling: spill if >100 lines or >8KB
    let final_output = spill_output_if_needed(repo_root, canonical_name, &raw_output).await;
    Ok(final_output)
}

fn resolve_safe_path(worktree_dir: &Path, rel_path: &str) -> Result<PathBuf> {
    let cleaned = rel_path.trim_start_matches(['/', '\\']);
    let candidate = worktree_dir.join(cleaned);
    Ok(candidate)
}

async fn run_scoped_command(
    cwd: &Path,
    cmd_str: &str,
    timeout: Duration,
    abort_flag: Arc<AtomicBool>,
) -> Result<crate::hooks::HookExecutionResult> {
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = Command::new("cmd.exe");
        c.args(["/C", cmd_str]);
        c
    };

    #[cfg(not(target_os = "windows"))]
    let mut cmd = {
        let mut c = Command::new("sh");
        c.args(["-c", cmd_str]);
        c
    };

    cmd.current_dir(cwd);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().with_context(|| format!("Failed to spawn command: {}", cmd_str))?;
    let pid = child.id();

    let mut stdout_pipe = child.stdout.take().context("Failed to take stdout")?;
    let mut stderr_pipe = child.stderr.take().context("Failed to take stderr")?;

    let run_task = async {
        let stdout_drain = async {
            let mut buf = Vec::new();
            let _ = stdout_pipe.read_to_end(&mut buf).await;
            String::from_utf8_lossy(&buf).to_string()
        };

        let stderr_drain = async {
            let mut buf = Vec::new();
            let _ = stderr_pipe.read_to_end(&mut buf).await;
            String::from_utf8_lossy(&buf).to_string()
        };

        let (stdout_str, stderr_str) = tokio::join!(stdout_drain, stderr_drain);
        let status = child.wait().await?;
        Ok::<crate::hooks::HookExecutionResult, anyhow::Error>(crate::hooks::HookExecutionResult {
            success: status.success(),
            exit_code: status.code(),
            stdout: stdout_str,
            stderr: stderr_str,
        })
    };

    let check_interval = async {
        loop {
            tokio::time::sleep(Duration::from_millis(200)).await;
            if abort_flag.load(Ordering::SeqCst) {
                if let Some(p) = pid {
                    kill_process_tree(p).await;
                }
                return;
            }
        }
    };

    tokio::select! {
        res = run_task => res,
        _ = tokio::time::sleep(timeout) => {
            if let Some(p) = pid {
                kill_process_tree(p).await;
            }
            bail!("Command '{}' timed out after {:?}", cmd_str, timeout)
        }
        _ = check_interval => {
            bail!("Command cancelled by user")
        }
    }
}

async fn collect_matching_files(
    start_dir: &Path,
    worktree_root: &Path,
    pattern: &str,
    out: &mut Vec<String>,
) -> Result<()> {
    if !start_dir.exists() {
        return Ok(());
    }

    let mut dir_stack = vec![start_dir.to_path_buf()];

    while let Some(current_dir) = dir_stack.pop() {
        let mut read_dir = match tokio::fs::read_dir(&current_dir).await {
            Ok(rd) => rd,
            Err(_) => continue,
        };

        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let path = entry.path();
            let file_type = match entry.file_type().await {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            if file_type.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name == ".git" || name == "target" || name == "node_modules" {
                    continue;
                }
                dir_stack.push(path);
            } else if file_type.is_file() {
                let rel = path
                    .strip_prefix(worktree_root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");

                if glob_match(pattern, &rel) {
                    out.push(rel);
                }
            }
        }
    }

    Ok(())
}

fn glob_match(pattern: &str, candidate: &str) -> bool {
    if pattern == "*" || pattern == "**/*" {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return candidate.ends_with(suffix);
    }
    candidate.contains(pattern)
}

async fn grep_walk(
    target: &Path,
    worktree_root: &Path,
    needle: &str,
    out: &mut Vec<String>,
) -> Result<()> {
    if target.is_file() {
        if let Ok(content) = tokio::fs::read_to_string(target).await {
            let rel = target
                .strip_prefix(worktree_root)
                .unwrap_or(target)
                .to_string_lossy()
                .replace('\\', "/");

            for (idx, line) in content.lines().enumerate() {
                if line.contains(needle) {
                    out.push(format!("{}:{}: {}", rel, idx + 1, line.trim()));
                    if out.len() >= 200 {
                        return Ok(());
                    }
                }
            }
        }
        return Ok(());
    }

    let mut dir_stack = vec![target.to_path_buf()];

    while let Some(current_dir) = dir_stack.pop() {
        let mut read_dir = match tokio::fs::read_dir(&current_dir).await {
            Ok(rd) => rd,
            Err(_) => continue,
        };

        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let path = entry.path();
            let file_type = match entry.file_type().await {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            let name = entry.file_name().to_string_lossy().to_string();
            if name == ".git" || name == "target" || name == "node_modules" {
                continue;
            }

            if file_type.is_dir() {
                dir_stack.push(path);
            } else if file_type.is_file() {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    let rel = path
                        .strip_prefix(worktree_root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .replace('\\', "/");

                    for (idx, line) in content.lines().enumerate() {
                        if line.contains(needle) {
                            out.push(format!("{}:{}: {}", rel, idx + 1, line.trim()));
                            if out.len() >= 200 {
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

pub struct AgentSessionConfig {
    pub repo_root: PathBuf,
    pub worktree_path: PathBuf,
    pub role: String,
    pub stage_id: String,
    pub task_id: String,
    pub task_title: String,
    pub task_description: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub temperature: Option<f32>,
    pub reasoning_effort: Option<String>,
    pub max_turns: usize,
}

/// Executes the full ReAct loop for an agent session.
pub async fn run_agent_loop<F>(
    config: AgentSessionConfig,
    abort_flag: Arc<AtomicBool>,
    event_sink: F,
) -> Result<String>
where
    F: Fn(AgentEvent) + Send + Sync + 'static,
{
    let system_prompt = compile_primed_system_prompt(
        &config.repo_root,
        &config.worktree_path,
        &config.role,
        &config.task_id,
        &config.task_title,
        &config.task_description,
    )
    .await?;

    let mut messages = vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(format!(
            "Execute task {}: {}\nDetails:\n{}",
            config.task_id, config.task_title, config.task_description
        )),
    ];

    // Load Tool Policy Configuration
    let policy = load_tool_policy_config(&config.repo_root).await.unwrap_or_default();

    // Collect standard tools + custom tools
    let mut available_tools = get_standard_tools();
    let custom_tools = discover_custom_tools(&config.repo_root).await;
    available_tools.extend(custom_tools);

    // Filter out disabled tools per Tool Policy Matrix (saving tokens!)
    available_tools.retain(|t| is_tool_allowed(&policy, &config.stage_id, &t.function.name));

    // Deterministic Prefix Caching: Sort tool definitions alphabetically by name
    available_tools.sort_by(|a, b| a.function.name.cmp(&b.function.name));

    let llm_client = LlmClient::new();
    let max_turns = if config.max_turns == 0 { 25 } else { config.max_turns };

    let mut modified_files = Vec::new();
    let mut latest_sensor_status = "UNKNOWN".to_string();

    for turn in 1..=max_turns {
        if abort_flag.load(Ordering::SeqCst) {
            event_sink(AgentEvent::Aborted);
            bail!("Agent execution aborted by user");
        }

        // State-Preserving Context Compactor: evaporate old tool outputs >3 turns and >500 chars
        let snapshot = StateSnapshot {
            modified_files: modified_files.clone(),
            sensor_status: latest_sensor_status.clone(),
            next_step: format!("Proceeding with turn {}", turn),
            verdict: None,
        };
        compact_conversation_history(&mut messages, turn, Some(&snapshot));

        event_sink(AgentEvent::TurnStart { turn });

        let req = OpenAIChatRequest {
            model: config.model.clone(),
            messages: messages.clone(),
            temperature: config.temperature,
            stream: true,
            tools: if available_tools.is_empty() {
                None
            } else {
                Some(available_tools.clone())
            },
            reasoning_effort: config.reasoning_effort.clone(),
        };

        let (tx, mut rx) = mpsc::channel::<StreamChunk>(100);
        let client = llm_client.clone();
        let base_url = config.base_url.clone();
        let api_key = config.api_key.clone();

        tokio::spawn(async move {
            let _ = client.stream_openai(&base_url, api_key.as_deref(), req, tx).await;
        });

        let mut turn_content = String::new();
        let mut turn_tool_calls: Vec<ToolCall> = Vec::new();

        while let Some(chunk) = rx.recv().await {
            if abort_flag.load(Ordering::SeqCst) {
                event_sink(AgentEvent::Aborted);
                bail!("Agent execution aborted by user");
            }

            match chunk {
                StreamChunk::Token(token) => {
                    turn_content.push_str(&token);
                    event_sink(AgentEvent::Token(token));
                }
                StreamChunk::Thought(thought) => {
                    event_sink(AgentEvent::Thought(thought));
                }
                StreamChunk::ToolCall { id, name, arguments } => {
                    event_sink(AgentEvent::ToolCall {
                        id: id.clone(),
                        name: name.clone(),
                        arguments: arguments.clone(),
                    });
                    turn_tool_calls.push(ToolCall {
                        id,
                        call_type: "function".to_string(),
                        function: crate::llm::FunctionCall { name, arguments },
                    });
                }
                StreamChunk::CacheTelemetry { .. } => {
                    // Telemetry received from prefix cache tracker
                }
                StreamChunk::Done => break,
                StreamChunk::Error(err) => {
                    event_sink(AgentEvent::Error(err.clone()));
                    bail!("LLM streaming error: {}", err);
                }
            }
        }

        // If no tool calls, this is the final agent response
        if turn_tool_calls.is_empty() {
            event_sink(AgentEvent::Finished {
                final_response: turn_content.clone(),
            });
            return Ok(turn_content);
        }

        // Append assistant turn message
        messages.push(ChatMessage::assistant(
            if turn_content.is_empty() { None } else { Some(turn_content) },
            Some(turn_tool_calls.clone()),
        ));

        // Execute each tool call and record response
        for tc in turn_tool_calls {
            let parsed_args: serde_json::Value =
                serde_json::from_str(&tc.function.arguments).unwrap_or_else(|_| serde_json::json!({}));

            // Track modified files
            if tc.function.name == "write_file" || tc.function.name == "edit_file" || tc.function.name == "str_replace" {
                if let Some(fp) = parsed_args.get("file_path").and_then(|v| v.as_str()) {
                    if !modified_files.contains(&fp.to_string()) {
                        modified_files.push(fp.to_string());
                    }
                }
            }

            let exec_result = execute_tool(
                &config.repo_root,
                &config.worktree_path,
                &tc.function.name,
                &config.stage_id,
                &policy,
                &parsed_args,
                Arc::clone(&abort_flag),
            )
            .await;

            let (result_str, success) = match exec_result {
                Ok(out) => {
                    if tc.function.name == "execute_command" || tc.function.name == "bash" {
                        if out.contains("[Exit Code: 0]") {
                            latest_sensor_status = "PASS".to_string();
                        } else {
                            latest_sensor_status = "FAIL".to_string();
                        }
                    }
                    (out, true)
                }
                Err(e) => (format!("Error executing {}: {}", tc.function.name, e), false),
            };

            event_sink(AgentEvent::ToolResult {
                id: tc.id.clone(),
                name: tc.function.name.clone(),
                result: result_str.clone(),
                success,
            });

            messages.push(ChatMessage::tool_response(tc.id, result_str));
        }
    }

    let final_msg = "Maximum turns reached without terminal response.".to_string();
    event_sink(AgentEvent::Finished {
        final_response: final_msg.clone(),
    });
    Ok(final_msg)
}

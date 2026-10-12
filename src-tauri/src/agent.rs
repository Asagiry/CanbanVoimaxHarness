pub mod compactor;

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
            "Search web via DuckDuckGo (free, zero API key required).",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query keywords." }
                },
                "required": ["query"]
            }),
        ),
        ToolDefinition::function(
            "web_fetch",
            "Fetch URL content and extract clean text/markdown.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "HTTP or HTTPS URL to fetch." }
                },
                "required": ["url"]
            }),
        ),
    ]
}

/// Tool Policy Matrix Zero-Trust Iron Barrier: checks column permissions.
pub fn check_stage_tool_permission(stage_id: &str, tool_name: &str) -> Result<()> {
    let allowed = match stage_id {
        "review" => {
            // Review Gate: Read-only auditing. Code modification is strictly forbidden!
            !matches!(tool_name, "edit_file" | "write_file")
        }
        "backlog" => {
            // Backlog: Planning and exploration. Execution and modification forbidden.
            !matches!(tool_name, "execute_command" | "edit_file" | "write_file")
        }
        "retro" => {
            // Retro: Rules and ledger updates only. Arbitrary execution forbidden.
            !matches!(tool_name, "execute_command" | "write_file")
        }
        "ready" => {
            // Ready: Read-only.
            matches!(tool_name, "read_file" | "glob" | "grep")
        }
        _ => {
            // dev and other stages: default permitted
            true
        }
    };

    if !allowed {
        bail!(
            "PermissionDenied: Инструмент '{}' отключен политикой безопасности для колонки '{}'",
            tool_name,
            stage_id
        );
    }
    Ok(())
}

/// Tool Output Spilling: If output > 100 lines or > 8KB, save full log to `.cvh/spill/<timestamp>-<tool>.log` and return head/tail + path.
pub async fn spill_output_if_large(
    repo_root: &Path,
    tool_name: &str,
    output: &str,
) -> Result<String> {
    let line_count = output.lines().count();
    let byte_count = output.len();

    if line_count > 100 || byte_count > 8192 {
        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S-%3f");
        let spill_dir = repo_root.join(".cvh").join("spill");
        tokio::fs::create_dir_all(&spill_dir).await?;

        let file_name = format!("{}-{}.log", timestamp, tool_name);
        let file_path = spill_dir.join(&file_name);
        tokio::fs::write(&file_path, output).await?;

        let rel_path = format!(".cvh/spill/{}", file_name);

        let lines: Vec<&str> = output.lines().collect();
        let head_lines = lines.iter().take(30).copied().collect::<Vec<_>>().join("\n");
        let tail_lines = if lines.len() > 30 {
            lines
                .iter()
                .skip(lines.len().saturating_sub(30))
                .copied()
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            String::new()
        };

        let truncated_count = lines.len().saturating_sub(60);

        return Ok(format!(
            "[Output spilled to {} ({} bytes, {} lines)]\n\
            --- HEAD (first 30 lines) ---\n\
            {}\n\
            ... [truncated {} lines; full log saved to {}] ...\n\
            --- TAIL (last 30 lines) ---\n\
            {}",
            rel_path, byte_count, line_count, head_lines, truncated_count, rel_path, tail_lines
        ));
    }

    Ok(output.to_string())
}

/// Executes a tool within the safe boundary of `worktree_dir` with Zero-Trust barrier and output spilling.
pub async fn execute_tool(
    repo_root: &Path,
    worktree_dir: &Path,
    stage_id: &str,
    tool_name: &str,
    args: &serde_json::Value,
    abort_handle: Arc<AtomicBool>,
) -> Result<String> {
    if abort_handle.load(Ordering::SeqCst) {
        bail!("Operation aborted");
    }

    // Zero-Trust Iron Barrier
    check_stage_tool_permission(stage_id, tool_name)?;

    let raw_output = match tool_name {
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
                bail!("Target string 'old_string' not found in file {:?}", rel_path);
            }

            let updated = if replace_all {
                current.replace(old_string, new_string)
            } else {
                let count = current.matches(old_string).count();
                if count > 1 {
                    bail!(
                        "Ambiguous match: 'old_string' occurs {} times in file. Set replace_all: true or supply more context.",
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

            let re = regex_lite_match(pattern)?;
            let mut matches = Vec::new();
            grep_walk(&search_target, worktree_dir, &re, &mut matches).await?;

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
            perform_duckduckgo_search(query).await?
        }

        "web_fetch" => {
            let url = args
                .get("url")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'url'")?;
            perform_web_fetch(url).await?
        }

        _ => bail!("Unknown tool: {}", tool_name),
    };

    // Apply Tool Output Spilling
    spill_output_if_large(repo_root, tool_name, &raw_output).await
}

async fn perform_duckduckgo_search(query: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()?;

    let encoded_query = urlencoding_simple(query);
    let url = format!("https://html.duckduckgo.com/html/?q={}", encoded_query);

    let res = client
        .post(&url)
        .form(&[("q", query)])
        .send()
        .await
        .with_context(|| format!("Failed to fetch DuckDuckGo search for '{}'", query))?;

    let html = res.text().await.unwrap_or_default();
    let results = parse_duckduckgo_html(&html);

    if results.is_empty() {
        Ok(format!("No web search results found for query: '{}'.", query))
    } else {
        Ok(results.join("\n\n"))
    }
}

fn parse_duckduckgo_html(html: &str) -> Vec<String> {
    let mut results = Vec::new();
    let parts: Vec<&str> = html.split("class=\"result__body\"").collect();
    for part in parts.iter().skip(1).take(5) {
        let snippet = extract_tag_text(part, "class=\"result__snippet\"");
        let title = extract_tag_text(part, "class=\"result__title\"");
        let url = extract_url(part);
        if !title.is_empty() || !snippet.is_empty() {
            results.push(format!(
                "**Title**: {}\n**URL**: {}\n**Snippet**: {}",
                title, url, snippet
            ));
        }
    }
    results
}

fn extract_tag_text(content: &str, marker: &str) -> String {
    if let Some(pos) = content.find(marker) {
        let rest = &content[pos..];
        if let Some(tag_end) = rest.find('>') {
            let inner = &rest[tag_end + 1..];
            if let Some(close) = inner.find("</a>") {
                return clean_html_tags(&inner[..close]);
            }
        }
    }
    String::new()
}

fn extract_url(content: &str) -> String {
    if let Some(pos) = content.find("class=\"result__url\"") {
        let rest = &content[pos..];
        if let Some(tag_end) = rest.find('>') {
            let inner = &rest[tag_end + 1..];
            if let Some(close) = inner.find("</a>") {
                return clean_html_tags(&inner[..close]).trim().to_string();
            }
        }
    }
    String::new()
}

fn clean_html_tags(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    let mut in_tag = false;
    for c in raw.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }
    result
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .trim()
        .to_string()
}

fn urlencoding_simple(s: &str) -> String {
    let mut encoded = String::new();
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{:02X}", b)),
        }
    }
    encoded
}

async fn perform_web_fetch(url: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()?;

    let res = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch URL '{}'", url))?;

    let body = res.text().await.unwrap_or_default();
    let cleaned = clean_html_to_markdown(&body);
    Ok(format!("# Fetched URL: {}\n\n{}", url, cleaned))
}

fn clean_html_to_markdown(html: &str) -> String {
    let without_scripts = strip_between_tags(html, "<script", "</script>");
    let without_styles = strip_between_tags(&without_scripts, "<style", "</style>");
    let cleaned = clean_html_tags(&without_styles);

    let mut lines = Vec::new();
    for line in cleaned.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            lines.push(trimmed);
        }
    }
    lines.join("\n\n")
}

fn strip_between_tags(input: &str, open_tag: &str, close_tag: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;

    while let Some(start) = input[cursor..].find(open_tag) {
        let abs_start = cursor + start;
        output.push_str(&input[cursor..abs_start]);
        if let Some(end) = input[abs_start..].find(close_tag) {
            cursor = abs_start + end + close_tag.len();
        } else {
            return output;
        }
    }
    output.push_str(&input[cursor..]);
    output
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

fn regex_lite_match(pattern: &str) -> Result<String> {
    Ok(pattern.to_string())
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
    pub stage_id: String,
    pub role: String,
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

    let tools = get_standard_tools();
    let llm_client = LlmClient::new();
    let max_turns = if config.max_turns == 0 { 25 } else { config.max_turns };

    for turn in 1..=max_turns {
        if abort_flag.load(Ordering::SeqCst) {
            event_sink(AgentEvent::Aborted);
            bail!("Agent execution aborted by user");
        }

        // State-Preserving Context Compactor: truncate tool outputs older than 3 turns (> 500 chars)
        if turn > 2 {
            let evaporated_bytes = compactor::ContextCompactor::evaporate_tool_logs(&mut messages);
            if evaporated_bytes > 0 {
                event_sink(AgentEvent::Thought(format!(
                    "[Compactor] Evaporated {} bytes of historical tool logs.",
                    evaporated_bytes
                )));
            }
        }

        event_sink(AgentEvent::TurnStart { turn });

        let req = OpenAIChatRequest {
            model: config.model.clone(),
            messages: messages.clone(),
            temperature: config.temperature,
            stream: true,
            tools: Some(tools.clone()),
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

            let exec_result = execute_tool(
                &config.repo_root,
                &config.worktree_path,
                &config.stage_id,
                &tc.function.name,
                &parsed_args,
                Arc::clone(&abort_flag),
            )
            .await;

            let (result_str, success) = match exec_result {
                Ok(out) => (out, true),
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

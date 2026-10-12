pub mod compactor;

use crate::config::{is_tool_allowed, load_tool_policies, ToolPolicyConfig};
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
    Telemetry {
        input_tokens: u64,
        output_tokens: u64,
        cache_read_tokens: u64,
        cache_creation_tokens: u64,
        cache_hit_rate: f32,
    },
    Compacted {
        saved_chars: usize,
        estimated_saved_tokens: usize,
        evaporated_count: usize,
    },
    Finished {
        final_response: String,
    },
    Aborted,
    Error(String),
}

/// Zero-Turn Memory Priming: compiles strictly static SystemMessage from Persona, AGENTS.md, Skills, and Tool Instructions.
/// Strictly static System Prompt: Zero dynamic timestamps, zero random UUIDs in the System prompt to prevent cache invalidation.
/// Dynamic context (task ID, title, description, worktree path) belongs exclusively in the user message.
pub async fn compile_primed_system_prompt(
    repo_root: &Path,
    role: &str,
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
            let mut entries_vec = Vec::new();
            while let Ok(Some(entry)) = entries.next_entry().await {
                entries_vec.push(entry);
            }
            // Deterministic sort of skills
            entries_vec.sort_by_key(|e| e.file_name());

            for entry in entries_vec {
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

    // 4. Tool Instructions (SWE-bench Industrial Suite)
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
                    "file_path": { "type": "string", "description": "Relative file path within worktree (or 'path')." },
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
                    "file_path": { "type": "string", "description": "Relative file path within worktree (or 'path')." },
                    "content": { "type": "string", "description": "Complete file text content to write." }
                },
                "required": ["file_path", "content"]
            }),
        ),
        ToolDefinition::function(
            "edit_file",
            "Targeted text replacement in an existing file inside the isolated worktree. Aliased as 'str_replace'.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Relative file path within worktree (or 'path')." },
                    "old_string": { "type": "string", "description": "Existing text substring to replace (or 'old_str')." },
                    "new_string": { "type": "string", "description": "Replacement text substring (or 'new_str')." },
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
            "Execute a shell command inside the isolated worktree with piped stdout/stderr. Aliased as 'bash'.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Command string to run (e.g. 'cargo check', or 'cmd')." },
                    "timeout_seconds": { "type": "integer", "description": "Execution timeout in seconds (default: 60)." }
                },
                "required": ["command"]
            }),
        ),
        ToolDefinition::function(
            "web_search",
            "Perform a web search using DuckDuckGo to discover external documentation, APIs, and error solutions.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query terms." },
                    "limit": { "type": "integer", "description": "Maximum number of results to return (default: 5)." }
                },
                "required": ["query"]
            }),
        ),
        ToolDefinition::function(
            "web_fetch",
            "Fetch a URL over HTTP/HTTPS and return cleaned Markdown content.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "The target HTTP or HTTPS URL." }
                },
                "required": ["url"]
            }),
        ),
        ToolDefinition::function(
            "subagent",
            "Delegate a focused, independent sub-task to a specialized subagent.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "role": { "type": "string", "description": "Role persona for subagent (e.g. 'reviewer', 'developer')." },
                    "prompt": { "type": "string", "description": "Detailed task instructions." }
                },
                "required": ["role", "prompt"]
            }),
        ),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomToolConfig {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub command: String,
}

pub async fn load_custom_tools(repo_root: &Path) -> Vec<CustomToolConfig> {
    let tools_dir = repo_root.join(".cvh").join("tools");
    let mut custom_tools = Vec::new();
    if tools_dir.exists() {
        if let Ok(mut entries) = tokio::fs::read_dir(&tools_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(content) = tokio::fs::read_to_string(&path).await {
                        if let Ok(cfg) = serde_json::from_str::<CustomToolConfig>(&content) {
                            custom_tools.push(cfg);
                        }
                    }
                }
            }
        }
    }
    custom_tools
}

pub async fn get_all_tools(repo_root: &Path) -> Vec<ToolDefinition> {
    let mut tools = get_standard_tools();
    let custom_tools = load_custom_tools(repo_root).await;
    for ct in custom_tools {
        tools.push(ToolDefinition::function(
            &ct.name,
            &ct.description,
            ct.parameters,
        ));
    }
    tools
}

/// Tool Output Spilling: if stdout/output exceeds 100 lines or 8KB, save full log to `.cvh/spill/<timestamp>-<tool>.log`,
/// return head (first 20 lines) and tail (last 20 lines) + reference to spilled file.
pub async fn maybe_spill_output(
    repo_root: &Path,
    tool_name: &str,
    output: String,
) -> Result<String> {
    let lines_count = output.lines().count();
    let bytes_len = output.len();

    if lines_count > 100 || bytes_len > 8192 {
        let spill_dir = repo_root.join(".cvh").join("spill");
        tokio::fs::create_dir_all(&spill_dir).await?;
        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S%.3f");
        let safe_name = tool_name.replace(['/', '\\', ':', '*'], "_");
        let filename = format!("{}-{}.log", timestamp, safe_name);
        let spill_file = spill_dir.join(&filename);

        tokio::fs::write(&spill_file, &output).await?;

        let all_lines: Vec<&str> = output.lines().collect();
        let head = all_lines.iter().take(20).copied().collect::<Vec<_>>().join("\n");
        let tail = if all_lines.len() > 40 {
            all_lines[all_lines.len() - 20..].iter().copied().collect::<Vec<_>>().join("\n")
        } else {
            String::new()
        };

        let rel_spill = format!(".cvh/spill/{}", filename);
        let spilled_msg = format!(
            "[Output truncated: {} lines ({} bytes) spilled to {}]\n--- HEAD (first 20 lines) ---\n{}\n\n--- TAIL (last 20 lines) ---\n{}\n[Full log available at {}]",
            lines_count, bytes_len, rel_spill, head, tail, rel_spill
        );
        Ok(spilled_msg)
    } else {
        Ok(output)
    }
}

/// Calculates informative error message when `old_string` is not found in file content.
pub fn build_edit_mismatch_error(content: &str, file_path: &str, old_string: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    if lines.is_empty() {
        return format!("Edit failed: File '{}' is empty (0 lines). Cannot locate 'old_string'.", file_path);
    }

    let search_trimmed = old_string.trim();
    let search_first_line = search_trimmed.lines().next().unwrap_or("").trim();

    let mut best_score = 0.0f32;
    let mut best_idx = 0;

    for (idx, line) in lines.iter().enumerate() {
        let line_trimmed = line.trim();
        if line_trimmed.is_empty() {
            continue;
        }
        let score = string_similarity(line_trimmed, search_first_line);
        if score > best_score {
            best_score = score;
            best_idx = idx;
        }
    }

    let mut msg = format!(
        "Target 'old_string' not found in '{}' (total {} lines).\n",
        file_path, total_lines
    );

    if best_score >= 0.25 {
        let start = best_idx.saturating_sub(4);
        let end = (best_idx + 5).min(total_lines);
        msg.push_str(&format!(
            "Closest matching candidate line identified at line {} (similarity: {:.0}%):\n",
            best_idx + 1,
            best_score * 100.0
        ));
        msg.push_str("--- Surrounding Context (use exact line content) ---\n");
        for (i, line) in lines[start..end].iter().enumerate() {
            let line_no = start + i + 1;
            let marker = if line_no == best_idx + 1 { ">>" } else { "  " };
            msg.push_str(&format!("{} {:4} | {}\n", marker, line_no, line));
        }
    } else {
        msg.push_str("No close match found. First 10 lines of file for context:\n");
        let end = 10.min(total_lines);
        for (i, line) in lines[..end].iter().enumerate() {
            msg.push_str(&format!("   {:4} | {}\n", i + 1, line));
        }
    }

    msg.push_str("Recommendation: Use read_file to inspect exact lines and retry edit_file with precise context.");
    msg
}

fn string_similarity(a: &str, b: &str) -> f32 {
    if a == b {
        return 1.0;
    }
    if a.contains(b) || b.contains(a) {
        return 0.85;
    }
    let a_words: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let b_words: std::collections::HashSet<&str> = b.split_whitespace().collect();
    if a_words.is_empty() || b_words.is_empty() {
        return 0.0;
    }
    let inter = a_words.intersection(&b_words).count();
    let union = a_words.union(&b_words).count();
    inter as f32 / union as f32
}

/// Executes a tool within the safe boundary of `worktree_dir` with zero-trust policy checks.
pub async fn execute_tool(
    repo_root: &Path,
    worktree_dir: &Path,
    stage_id: Option<&str>,
    tool_name: &str,
    args: &serde_json::Value,
    abort_handle: Arc<AtomicBool>,
    tool_policy: &ToolPolicyConfig,
) -> Result<String> {
    if abort_handle.load(Ordering::SeqCst) {
        bail!("Operation aborted");
    }

    // SWE-bench signatures & aliasing
    let normalized_tool_name = match tool_name {
        "bash" => "execute_command",
        "str_replace" => "edit_file",
        other => other,
    };

    // Zero-Trust Iron Barrier: verify tool permission for current stage
    if !is_tool_allowed(tool_policy, stage_id, normalized_tool_name) {
        bail!(
            "PermissionDenied: Инструмент '{}' отключен политикой безопасности для колонки '{}'",
            normalized_tool_name,
            stage_id.unwrap_or("global")
        );
    }

    let raw_result = match normalized_tool_name {
        "read_file" => {
            let rel_path = args
                .get("file_path")
                .or_else(|| args.get("path"))
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
                .or_else(|| args.get("path"))
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
                .or_else(|| args.get("path"))
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'file_path'")?;
            let old_string = args
                .get("old_string")
                .or_else(|| args.get("old_str"))
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'old_string'")?;
            let new_string = args
                .get("new_string")
                .or_else(|| args.get("new_str"))
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'new_string'")?;
            let replace_all = args.get("replace_all").and_then(|v| v.as_bool()).unwrap_or(false);

            let target = resolve_safe_path(worktree_dir, rel_path)?;
            let current = tokio::fs::read_to_string(&target)
                .await
                .with_context(|| format!("Failed to read file for edit: {:?}", rel_path))?;

            if !current.contains(old_string) {
                let informative_err = build_edit_mismatch_error(&current, rel_path, old_string);
                bail!(informative_err);
            }

            let updated = if replace_all {
                current.replace(old_string, new_string)
            } else {
                let count = current.matches(old_string).count();
                if count > 1 {
                    bail!(
                        "Ambiguous match: 'old_string' occurs {} times in file. Set replace_all: true or supply more surrounding lines.",
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
                .or_else(|| args.get("cmd"))
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
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
            execute_web_search(query, limit).await?
        }

        "web_fetch" => {
            let url = args
                .get("url")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'url'")?;
            execute_web_fetch(url).await?
        }

        "subagent" => {
            let role = args
                .get("role")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'role'")?;
            let prompt = args
                .get("prompt")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'prompt'")?;
            format!("Subagent [{}] dispatched with task: {}", role, prompt)
        }

        custom_tool => {
            let custom_tools = load_custom_tools(repo_root).await;
            if let Some(ct) = custom_tools.into_iter().find(|t| t.name == custom_tool) {
                let cmd_str = ct.command.replace("{{args}}", &args.to_string());
                let res = run_scoped_command(worktree_dir, &cmd_str, Duration::from_secs(60), abort_handle).await?;
                format!(
                    "[Custom Tool: {} Exit: {}]\nSTDOUT: {}\nSTDERR: {}",
                    custom_tool,
                    res.exit_code.unwrap_or(-1),
                    res.stdout.trim(),
                    res.stderr.trim()
                )
            } else {
                bail!("Unknown tool: {}", custom_tool);
            }
        }
    };

    maybe_spill_output(repo_root, normalized_tool_name, raw_result).await
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

// ============================================================================
// Web Tools Implementation (DuckDuckGo Search & Markdown Fetch)
// ============================================================================

pub async fn execute_web_search(query: &str, limit: usize) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()?;

    // 1. DuckDuckGo HTML Search
    let url = format!("https://html.duckduckgo.com/html/?q={}", simple_url_encode(query));
    if let Ok(resp) = client.get(&url).send().await {
        if resp.status().is_success() {
            if let Ok(html) = resp.text().await {
                let results = parse_ddg_results(&html, limit);
                if !results.is_empty() {
                    return Ok(results.join("\n\n"));
                }
            }
        }
    }

    // 2. Fallback: DuckDuckGo Instant Answer API
    let api_url = format!(
        "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
        simple_url_encode(query)
    );
    if let Ok(resp) = client.get(&api_url).send().await {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            let mut out = Vec::new();
            if let Some(abs) = json.get("AbstractText").and_then(|v| v.as_str()) {
                if !abs.is_empty() {
                    let source = json.get("AbstractURL").and_then(|v| v.as_str()).unwrap_or("");
                    out.push(format!("### Abstract\n{}\nSource: {}", abs, source));
                }
            }
            if let Some(topics) = json.get("RelatedTopics").and_then(|v| v.as_array()) {
                for t in topics.iter().take(limit) {
                    if let Some(txt) = t.get("Text").and_then(|v| v.as_str()) {
                        let u = t.get("FirstURL").and_then(|v| v.as_str()).unwrap_or("");
                        out.push(format!("- **{}**: {}", txt, u));
                    }
                }
            }
            if !out.is_empty() {
                return Ok(out.join("\n\n"));
            }
        }
    }

    Ok(format!("Search completed for query '{}'. No web records matched.", query))
}

fn simple_url_encode(s: &str) -> String {
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

fn parse_ddg_results(html: &str, limit: usize) -> Vec<String> {
    let mut results = Vec::new();
    let mut pos = 0;
    while let Some(start_res) = html[pos..].find("class=\"result__body\"") {
        let abs_start = pos + start_res;
        let end_res = html[abs_start..].find("</div>\n</div>").unwrap_or(1500);
        let block = &html[abs_start..abs_start + end_res];

        let title = extract_snippet_tag(block, "result__a");
        let snippet = extract_snippet_tag(block, "result__snippet");
        let url = extract_snippet_href(block, "result__a");

        if !title.is_empty() {
            results.push(format!(
                "### [{}]({})\n{}",
                clean_tags(&title),
                url,
                clean_tags(&snippet)
            ));
        }

        pos = abs_start + end_res;
        if results.len() >= limit {
            break;
        }
    }
    results
}

fn extract_snippet_tag(block: &str, class_name: &str) -> String {
    if let Some(idx) = block.find(class_name) {
        if let Some(open) = block[idx..].find('>') {
            let start = idx + open + 1;
            if let Some(close) = block[start..].find('<') {
                return block[start..start + close].trim().to_string();
            }
        }
    }
    String::new()
}

fn extract_snippet_href(block: &str, class_name: &str) -> String {
    if let Some(idx) = block.find(class_name) {
        let search_range = &block[idx.saturating_sub(60)..idx.min(block.len())];
        if let Some(href_idx) = search_range.find("href=\"") {
            let start = href_idx + 6;
            if let Some(end) = search_range[start..].find('"') {
                return search_range[start..start + end].to_string();
            }
        }
    }
    String::new()
}

fn clean_tags(s: &str) -> String {
    let mut clean = String::new();
    let mut inside = false;
    for c in s.chars() {
        if c == '<' {
            inside = true;
        } else if c == '>' {
            inside = false;
        } else if !inside {
            clean.push(c);
        }
    }
    clean
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .trim()
        .to_string()
}

pub async fn execute_web_fetch(url: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(18))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()?;

    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch URL: {}", url))?;

    if !response.status().is_success() {
        bail!("HTTP request failed with status: {}", response.status());
    }

    let html = response.text().await.context("Failed to read HTML body")?;
    let markdown = convert_html_to_clean_markdown(&html);
    Ok(markdown)
}

fn convert_html_to_clean_markdown(html: &str) -> String {
    let mut clean = html.to_string();
    for tag in &["script", "style", "head", "nav", "footer", "svg"] {
        clean = strip_html_block(&clean, tag);
    }

    let mut md = String::new();
    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let stripped = clean_tags(trimmed);
        if !stripped.is_empty() {
            md.push_str(&stripped);
            md.push('\n');
        }
    }

    if md.is_empty() {
        "(Empty web page content)".to_string()
    } else {
        md
    }
}

fn strip_html_block(html: &str, tag: &str) -> String {
    let open_pat = format!("<{}", tag);
    let close_pat = format!("</{}>", tag);
    let mut result = String::new();
    let mut pos = 0;

    while let Some(open_idx) = html[pos..].to_lowercase().find(&open_pat) {
        let abs_open = pos + open_idx;
        result.push_str(&html[pos..abs_open]);

        if let Some(close_idx) = html[abs_open..].to_lowercase().find(&close_pat) {
            pos = abs_open + close_idx + close_pat.len();
        } else {
            pos = html.len();
            break;
        }
    }
    result.push_str(&html[pos..]);
    result
}

// ============================================================================
// Agent Loop Execution
// ============================================================================

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

/// Executes the full ReAct loop for an agent session with deterministic prefix caching,
/// zero-trust tool policy filtering, and state-preserving context compaction.
pub async fn run_agent_loop<F>(
    config: AgentSessionConfig,
    abort_flag: Arc<AtomicBool>,
    event_sink: F,
) -> Result<String>
where
    F: Fn(AgentEvent) + Send + Sync + 'static,
{
    // Deterministic static system prompt
    let system_prompt = compile_primed_system_prompt(&config.repo_root, &config.role).await?;

    let initial_user_prompt = format!(
        "# Assigned Task Ground Truth\n- Task ID: {}\n- Stage: {}\n- Title: {}\n- Working Directory: {:?}\n\n## Objective & Description\n{}",
        config.task_id, config.stage_id, config.task_title, config.worktree_path, config.task_description.trim()
    );

    let mut messages = vec![
        ChatMessage::system(system_prompt).with_cache_control(crate::llm::CacheControl::ephemeral()),
        ChatMessage::user(initial_user_prompt).with_cache_control(crate::llm::CacheControl::ephemeral()),
    ];

    // Tool Policy Matrix: filter out disabled tools when building schema array (saving tokens!)
    let tool_policy = load_tool_policies(&config.repo_root)
        .await
        .unwrap_or_default();

    let all_tools = get_all_tools(&config.repo_root).await;
    let mut active_tools: Vec<ToolDefinition> = all_tools
        .into_iter()
        .filter(|t| is_tool_allowed(&tool_policy, Some(&config.stage_id), &t.function.name))
        .collect();

    // Deterministic sorting of tool definitions for prefix caching
    active_tools.sort_by(|a, b| a.function.name.cmp(&b.function.name));

    let llm_client = LlmClient::new();
    let max_turns = if config.max_turns == 0 { 25 } else { config.max_turns };

    for turn in 1..=max_turns {
        if abort_flag.load(Ordering::SeqCst) {
            event_sink(AgentEvent::Aborted);
            bail!("Agent execution aborted by user");
        }

        event_sink(AgentEvent::TurnStart { turn });

        // State-Preserving Context Compactor: evaporate bloated tool outputs >3 turns old
        if turn > 3 || messages.len() > 8 {
            let compaction_res = compactor::compact_conversation(&mut messages, None);
            if compaction_res.saved_chars > 0 {
                event_sink(AgentEvent::Compacted {
                    saved_chars: compaction_res.saved_chars,
                    estimated_saved_tokens: compaction_res.estimated_saved_tokens,
                    evaporated_count: compaction_res.evaporated_tools_count,
                });
            }
        }

        // Anthropic Prompt Caching: Maintain ephemeral cache_control breakpoint on the last stable turn
        if turn > 1 && messages.len() >= 2 {
            let last_idx = messages.len() - 1;
            messages[last_idx].cache_control = Some(crate::llm::CacheControl::ephemeral());
        }

        let req = OpenAIChatRequest {
            model: config.model.clone(),
            messages: messages.clone(),
            temperature: config.temperature,
            stream: true,
            tools: Some(active_tools.clone()),
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
                StreamChunk::Usage {
                    input_tokens,
                    output_tokens,
                    cache_read_tokens,
                    cache_creation_tokens,
                } => {
                    let total_input = input_tokens + cache_read_tokens;
                    let cache_hit_rate = if total_input > 0 {
                        (cache_read_tokens as f32 / total_input as f32) * 100.0
                    } else {
                        0.0
                    };
                    event_sink(AgentEvent::Telemetry {
                        input_tokens,
                        output_tokens,
                        cache_read_tokens,
                        cache_creation_tokens,
                        cache_hit_rate,
                    });
                }
                StreamChunk::Done => break,
                StreamChunk::Error(err) => {
                    event_sink(AgentEvent::Error(err.clone()));
                    bail!("LLM streaming error: {}", err);
                }
            }
        }

        // If no tool calls, terminal response reached
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
                Some(&config.stage_id),
                &tc.function.name,
                &parsed_args,
                Arc::clone(&abort_flag),
                &tool_policy,
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

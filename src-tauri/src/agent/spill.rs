use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::llm::ToolDefinition;
use crate::agent::tool_schemas::{get_load_rule_tool, get_standard_tools};
use crate::agent::subagent_runtime::canonical_tool_name;

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
    get_all_tools_for_role(repo_root, None).await
}

pub async fn get_all_tools_for_role(repo_root: &Path, role: Option<&str>) -> Vec<ToolDefinition> {
    let mut tools = get_standard_tools();
    if let Some(pos) = tools.iter().position(|t| t.function.name == "load_rule") {
        tools[pos] = get_load_rule_tool(repo_root, role);
    }
    let mut custom_tools = load_custom_tools(repo_root).await;
    // Deterministic order: filesystem read_dir order is OS-dependent.
    // Sort custom tools by name so merged list is stable (final active list
    // is additionally sorted alphabetically in run_agent_loop for prefix caching).
    custom_tools.sort_by(|a, b| a.name.cmp(&b.name));
    for ct in custom_tools {
        tools.push(ToolDefinition::function(
            &ct.name,
            &ct.description,
            ct.parameters,
        ));
    }
    if let Ok(tool_policy) = crate::config::load_tool_policies(repo_root).await {
        for tool in &mut tools {
            if let Some(custom_desc) = tool_policy.tool_descriptions.get(&tool.function.name) {
                tool.function.description = custom_desc.clone();
            }
            let custom_schema = tool_policy
                .tool_custom_schemas
                .get(&tool.function.name)
                .or_else(|| {
                    if tool.function.name == "subagent" {
                        tool_policy.tool_custom_schemas.get("subagent_sync")
                    } else {
                        None
                    }
                });
            if let Some(custom_val) = custom_schema {
                if let Ok(custom_def) = serde_json::from_value::<ToolDefinition>(custom_val.clone()) {
                    *tool = custom_def;
                }
            }
            if tool.function.name == "execute_command" && tool.function.aliases.is_empty() {
                if let Some(shell) = tool_policy
                    .tool_settings
                    .get("execute_command")
                    .and_then(|v| v.get("shell"))
                    .and_then(|v| v.as_str())
                {
                    match shell.to_lowercase().as_str() {
                        "pwsh" | "powershell" => {
                            tool.function.aliases = vec!["pwsh".to_string(), "powershell".to_string()];
                        }
                        "cmd" => {
                            tool.function.aliases = vec!["cmd".to_string()];
                        }
                        _ => {
                            tool.function.aliases = vec!["bash".to_string()];
                        }
                    }
                }
            }
        }
    }
    tools
}

/// Prunes the .cvh/spill directory if file count exceeds 50 or total size exceeds 20MB,
/// removing the oldest files first based on modification time.
pub async fn prune_spill_dir(spill_dir: &Path) -> Result<()> {
    let mut entries = match tokio::fs::read_dir(spill_dir).await {
        Ok(rd) => rd,
        Err(_) => return Ok(()),
    };

    struct SpillFileInfo {
        path: PathBuf,
        mtime: std::time::SystemTime,
        size: u64,
    }

    let mut files: Vec<SpillFileInfo> = Vec::new();
    let mut total_bytes = 0u64;

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.is_file() {
            if let Ok(meta) = entry.metadata().await {
                let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let size = meta.len();
                total_bytes += size;
                files.push(SpillFileInfo { path, mtime, size });
            }
        }
    }

    const MAX_FILES: usize = 50;
    const MAX_BYTES: u64 = 20 * 1024 * 1024; // 20 MB

    if files.len() > MAX_FILES || total_bytes > MAX_BYTES {
        files.sort_by_key(|f| f.mtime);
        let mut file_count = files.len();
        for file in files {
            if file_count <= MAX_FILES && total_bytes <= MAX_BYTES {
                break;
            }
            if tokio::fs::remove_file(&file.path).await.is_ok() {
                total_bytes = total_bytes.saturating_sub(file.size);
                file_count = file_count.saturating_sub(1);
            }
        }
    }

    Ok(())
}

/// Tool Output Spilling: if stdout/output exceeds thresholds, save full log to `.cvh/spill/<timestamp>-<tool>.log`,
/// return head and tail + reference to spilled file.
/// Subagent and task history tools have a generous limit (5000 lines / 256KB), but are still safely spilled
/// if they exceed that absolute ceiling to prevent context explosion.
pub async fn maybe_spill_output(
    repo_root: &Path,
    worktree_dir: Option<&Path>,
    tool_name: &str,
    output: String,
) -> Result<String> {
    let canonical = canonical_tool_name(tool_name);
    let is_exempt = matches!(
        canonical,
        "subagent"
            | "subagent_bg"
            | "collect_subagent"
            | "subagent_list"
            | "load_rule"
            | "get_task_history"
    );

    let lines_count = output.lines().count();
    let bytes_len = output.len();

    let (max_lines, max_bytes) = if is_exempt {
        (5000, 256 * 1024)
    } else if canonical == "read_file" {
        (2000, 128 * 1024)
    } else {
        (500, 64 * 1024)
    };

    if lines_count > max_lines || bytes_len > max_bytes {
        let spill_dir = repo_root.join(".cvh").join("spill");
        tokio::fs::create_dir_all(&spill_dir).await?;
        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S%.3f");
        let safe_name = tool_name.replace(['/', '\\', ':', '*'], "_");
        let filename = format!("{}-{}-{}.log", timestamp, safe_name, uuid::Uuid::new_v4());
        let spill_file = spill_dir.join(&filename);

        tokio::fs::write(&spill_file, &output).await?;
        let _ = prune_spill_dir(&spill_dir).await;

        // Mirror to worktree if isolated execution (§ 2.8 worktree safety)
        if let Some(wt) = worktree_dir {
            if wt != repo_root {
                let wt_spill = wt.join(".cvh").join("spill");
                if tokio::fs::create_dir_all(&wt_spill).await.is_ok() {
                    let _ = tokio::fs::write(wt_spill.join(&filename), &output).await;
                    let _ = prune_spill_dir(&wt_spill).await;
                }
            }
        }

        let all_lines: Vec<&str> = output.lines().collect();
        let head_count = if is_exempt { 50 } else { 20 };
        let tail_count = if is_exempt { 50 } else { 20 };
        let mut head = all_lines[..head_count.min(all_lines.len())].join("\n");
        let mut tail = if all_lines.len() > head_count + tail_count {
            all_lines[all_lines.len() - tail_count..].join("\n")
        } else {
            String::new()
        };
        let preview_bytes = if is_exempt { 8192 } else { 4096 };
        if head.len() > preview_bytes {
            let mut end = preview_bytes;
            while !head.is_char_boundary(end) { end -= 1; }
            head.truncate(end);
            head.push_str("\n[Head preview truncated]");
            if tail.is_empty() {
                tail = output.clone();
            }
        }
        if tail.len() > preview_bytes {
            let mut start = tail.len() - preview_bytes;
            while !tail.is_char_boundary(start) { start += 1; }
            tail = format!("[Tail preview truncated]\n{}", &tail[start..]);
        }

        let rel_spill = format!(".cvh/spill/{}", filename);
        let spilled_msg = format!(
            "[Output truncated: {} lines ({} bytes) spilled to {}]\n--- HEAD (first {} lines) ---\n{}\n\n--- TAIL (last {} lines) ---\n{}\n[Full log available at {}]",
            lines_count, bytes_len, rel_spill, head_count, head, tail_count, tail, rel_spill
        );
        Ok(spilled_msg)
    } else {
        Ok(output)
    }
}

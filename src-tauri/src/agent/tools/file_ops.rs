//! File Operations Tool Handlers (read_file, write_file, edit_file, glob, grep)
//! Extracted from agent.rs per ub-mode Clean Code & Command Pattern principles.

use std::path::{Path, PathBuf};
use anyhow::{bail, Context, Result};
use tauri::Emitter;

use crate::agent::text_match::{build_edit_mismatch_error, replace_text_normalized};
use crate::agent::ToolContext;

pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for comp in path.components() {
        match comp {
            std::path::Component::Prefix(p) => components.push(std::path::Component::Prefix(p)),
            std::path::Component::RootDir => components.push(std::path::Component::RootDir),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::Normal(c) => components.push(std::path::Component::Normal(c)),
        }
    }
    components.into_iter().collect()
}

pub fn resolve_safe_path(worktree_dir: &Path, rel_path: &str) -> Result<PathBuf> {
    let cleaned = rel_path.trim_start_matches(['/', '\\']);
    if cleaned.contains(':') {
        bail!("Absolute path with drive letter denied: {:?}", rel_path);
    }

    let candidate = worktree_dir.join(cleaned);
    let norm_worktree = normalize_path(worktree_dir);
    let norm_candidate = normalize_path(&candidate);

    if !norm_candidate.starts_with(&norm_worktree) {
        bail!(
            "Path traversal denied: {:?} attempts to escape worktree boundary {:?}",
            rel_path,
            worktree_dir
        );
    }

    if worktree_dir.exists() {
        let canon_root = std::fs::canonicalize(worktree_dir)
            .with_context(|| format!("Failed to canonicalize worktree {:?}", worktree_dir))?;

        if std::fs::symlink_metadata(&norm_candidate).is_ok() {
            let canon_target = std::fs::canonicalize(&norm_candidate).with_context(|| {
                format!("Denied dangling or unreadable path/symlink: {:?}", rel_path)
            })?;
            if !canon_target.starts_with(&canon_root) {
                bail!(
                    "Path traversal denied: symlink or junction {:?} escapes worktree boundary",
                    rel_path
                );
            }
        } else {
            let mut ancestor_opt = norm_candidate.parent();
            while let Some(anc) = ancestor_opt {
                if std::fs::symlink_metadata(anc).is_ok() {
                    let canon_anc = std::fs::canonicalize(anc).with_context(|| {
                        format!("Denied dangling or unreadable ancestor for {:?}", rel_path)
                    })?;
                    let remainder = norm_candidate.strip_prefix(anc).unwrap_or(Path::new(""));
                    let resolved_check = canon_anc.join(remainder);
                    if !resolved_check.starts_with(&canon_root) {
                        bail!(
                            "Path traversal denied: ancestor symlink/junction {:?} escapes worktree boundary",
                            rel_path
                        );
                    }
                    break;
                }
                if anc == norm_worktree {
                    break;
                }
                ancestor_opt = anc.parent();
            }
        }
    }

    Ok(norm_candidate)
}

pub async fn emit_file_change_events(ctx: &ToolContext<'_>, rel_path: &str) {
    if let Some(app) = &ctx.app_handle {
        let norm = rel_path.replace('\\', "/");
        if norm.ends_with("boards.json") {
            if let Ok(bc) = crate::config::load_boards_config(ctx.repo_root).await {
                let _ = app.emit(
                    "boards:updated",
                    serde_json::json!({ "boards": &bc.boards, "activeBoardId": &bc.active_board_id }),
                );
                let mut tasks = crate::commands::tasks::load_tasks_from_disk(ctx.repo_root).await;
                let modified = crate::commands::tasks::heal_tasks(&mut tasks, &bc);
                if modified {
                    let _ = crate::commands::tasks::save_tasks_to_disk(ctx.repo_root, &tasks).await;
                }
                let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
            }
            if let Ok(pipe) = crate::config::load_pipeline_config(ctx.repo_root).await {
                let _ = app.emit("pipeline:updated", serde_json::json!({ "pipeline": &pipe }));
            }
        } else if norm.contains(".cvh/agents") || norm.contains("/agents/") {
            let list = crate::commands::list_agent_personas_in(ctx.repo_root).await;
            let _ = app.emit("agent:personas_updated", &list);
        } else if norm.contains(".cvh/rules") || norm.contains("/rules/") {
            let _ = app.emit("cortex:rules_updated", serde_json::json!({ "path": norm }));
        }
    }
}

pub async fn collect_matching_files(
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

            if file_type.is_symlink() {
                continue;
            }
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

pub fn glob_match(pattern: &str, candidate: &str) -> bool {
    let p = pattern.replace('\\', "/");
    let c = candidate.replace('\\', "/");
    glob_match_slices(p.as_bytes(), c.as_bytes())
}

enum GlobTok {
    DoubleStar,
    Star,
    Any,
    Lit(u8),
}

pub fn glob_match_slices(pattern: &[u8], candidate: &[u8]) -> bool {
    let mut toks = Vec::with_capacity(pattern.len());
    let mut idx = 0;
    while idx < pattern.len() {
        if pattern[idx..].starts_with(b"**/*") {
            toks.push(GlobTok::DoubleStar);
            idx += 4;
        } else if pattern[idx..].starts_with(b"**/") {
            toks.push(GlobTok::DoubleStar);
            idx += 3;
        } else if pattern[idx..].starts_with(b"**") {
            toks.push(GlobTok::DoubleStar);
            idx += 2;
        } else if pattern[idx] == b'*' {
            toks.push(GlobTok::Star);
            idx += 1;
        } else if pattern[idx] == b'?' {
            toks.push(GlobTok::Any);
            idx += 1;
        } else {
            toks.push(GlobTok::Lit(pattern[idx]));
            idx += 1;
        }
    }

    let n = candidate.len();
    let mut dp = vec![false; n + 1];
    let mut next = vec![false; n + 1];
    dp[0] = true;

    for tok in &toks {
        next.fill(false);
        match tok {
            GlobTok::DoubleStar => {
                if let Some(first) = dp.iter().position(|&b| b) {
                    next[first..].fill(true);
                }
            }
            GlobTok::Star => {
                let mut active = false;
                for j in 0..=n {
                    if dp[j] {
                        active = true;
                    }
                    if active {
                        next[j] = true;
                    }
                    if j < n && candidate[j] == b'/' {
                        active = false;
                    }
                }
            }
            GlobTok::Any => {
                for j in 0..n {
                    if dp[j] {
                        next[j + 1] = true;
                    }
                }
            }
            GlobTok::Lit(b) => {
                for j in 0..n {
                    if dp[j] && candidate[j] == *b {
                        next[j + 1] = true;
                    }
                }
            }
        }
        std::mem::swap(&mut dp, &mut next);
    }

    dp[n]
}

const MAX_READ_FILE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_GREP_FILE_BYTES: u64 = 2 * 1024 * 1024;

async fn grep_single_file(
    path: &Path,
    worktree_root: &Path,
    pattern: &regex::Regex,
    max_bytes: u64,
    out: &mut Vec<String>,
) -> bool {
    if !tokio::fs::metadata(path).await.is_ok_and(|m| m.len() <= max_bytes) {
        return false;
    }
    if let Ok(content) = tokio::fs::read_to_string(path).await {
        let rel = path
            .strip_prefix(worktree_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        for (idx, line) in content.lines().enumerate() {
            if pattern.is_match(line) {
                out.push(format!("{}:{}: {}", rel, idx + 1, line.trim()));
                if out.len() >= 200 {
                    return true;
                }
            }
        }
    }
    false
}

pub async fn grep_walk(
    target: &Path,
    worktree_root: &Path,
    pattern: &regex::Regex,
    out: &mut Vec<String>,
) -> Result<()> {
    if target.is_file() {
        let _ = grep_single_file(target, worktree_root, pattern, MAX_READ_FILE_BYTES, out).await;
        return Ok(());
    }

    let mut dir_stack = vec![target.to_path_buf()];
    while let Some(current_dir) = dir_stack.pop() {
        let Ok(mut read_dir) = tokio::fs::read_dir(&current_dir).await else {
            continue;
        };
        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let path = entry.path();
            let Ok(file_type) = entry.file_type().await else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name == ".git" || name == "target" || name == "node_modules" {
                continue;
            }
            if file_type.is_dir() {
                dir_stack.push(path);
            } else if file_type.is_file()
                && grep_single_file(&path, worktree_root, pattern, MAX_GREP_FILE_BYTES, out).await
            {
                return Ok(());
            }
        }
    }
    Ok(())
}

pub async fn execute_read_file(ctx: &ToolContext<'_>, args: &serde_json::Value) -> Result<String> {
    let rel_path = args
        .get("file_path")
        .or_else(|| args.get("path"))
        .and_then(|v| v.as_str())
        .context("Missing required parameter 'file_path'")?;

    let mut target = resolve_safe_path(ctx.worktree_dir, rel_path)?;
    if !target.exists() && ctx.worktree_dir != ctx.repo_root {
        // Fallback for shared .cvh/ assets (e.g. .cvh/spill/ logs or shared .cvh/ configs)
        let norm = rel_path.trim_start_matches(['/', '\\']).replace('\\', "/");
        if norm.starts_with(".cvh/") {
            if let Ok(repo_target) = resolve_safe_path(ctx.repo_root, rel_path) {
                if repo_target.exists() {
                    target = repo_target;
                }
            }
        }
    }
    if !target.exists() {
        bail!("File does not exist: {:?}", rel_path);
    }

    let default_limit = ctx
        .tool_policy
        .tool_settings
        .get("read_file")
        .and_then(|v| v.get("max_lines"))
        .and_then(|v| v.as_u64())
        .unwrap_or(200) as usize;
    let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(default_limit);

    let meta = tokio::fs::metadata(&target)
        .await
        .with_context(|| format!("Failed to stat file: {:?}", rel_path))?;
    if meta.len() > MAX_READ_FILE_BYTES {
        bail!(
            "File {:?} exceeds maximum readable size ({} bytes > {} bytes)",
            rel_path,
            meta.len(),
            MAX_READ_FILE_BYTES
        );
    }

    let content = tokio::fs::read_to_string(&target)
        .await
        .with_context(|| format!("Failed to read file: {:?}", rel_path))?;

    let lines: Vec<&str> = content.lines().collect();
    let start = offset.saturating_sub(1);
    if start >= lines.len() {
        return Ok("(File is empty or offset out of range)\n".to_string());
    }
    let end = start.saturating_add(limit).min(lines.len());
    if start > end {
        return Ok("(File is empty or offset out of range)\n".to_string());
    }

    let mut output = String::new();
    for (idx, line) in lines[start..end].iter().enumerate() {
        output.push_str(&format!("{}: {}\n", start + idx + 1, line));
    }

    if output.is_empty() {
        output.push_str("(File is empty or offset out of range)\n");
    } else if end < lines.len() {
        output.push_str(&format!("[Showing lines {}-{} of {}. Use offset={} to continue.]\n", start + 1, end, lines.len(), end + 1));
    }

    Ok(output)
}

pub async fn execute_write_file(ctx: &ToolContext<'_>, args: &serde_json::Value) -> Result<String> {
    let rel_path = args
        .get("file_path")
        .or_else(|| args.get("path"))
        .and_then(|v| v.as_str())
        .context("Missing required parameter 'file_path'")?;
    let content = args
        .get("content")
        .and_then(|v| v.as_str())
        .context("Missing required parameter 'content'")?;

    let target = resolve_safe_path(ctx.worktree_dir, rel_path)?;
    crate::config::atomic_write_file(&target, content).await?;
    if ctx.worktree_dir == ctx.repo_root {
        emit_file_change_events(ctx, rel_path).await;
    }
    Ok(format!("Successfully wrote {} bytes to {:?}", content.len(), rel_path))
}

pub async fn execute_edit_file(ctx: &ToolContext<'_>, args: &serde_json::Value) -> Result<String> {
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

    let target = resolve_safe_path(ctx.worktree_dir, rel_path)?;
    let current = tokio::fs::read_to_string(&target)
        .await
        .with_context(|| format!("Failed to read file for edit: {:?}", rel_path))?;

    let updated = match replace_text_normalized(&current, old_string, new_string, replace_all) {
        Ok(u) => u,
        Err(err) if err == "NOT_FOUND" => {
            let informative_err = build_edit_mismatch_error(&current, rel_path, old_string);
            bail!(informative_err);
        }
        Err(err) => {
            bail!(err);
        }
    };

    crate::config::atomic_write_file(&target, &updated).await?;
    if ctx.worktree_dir == ctx.repo_root {
        emit_file_change_events(ctx, rel_path).await;
    }
    Ok(format!("Successfully updated {:?}", rel_path))
}

pub async fn execute_glob(ctx: &ToolContext<'_>, args: &serde_json::Value) -> Result<String> {
    let pattern = args
        .get("pattern")
        .and_then(|v| v.as_str())
        .context("Missing required parameter 'pattern'")?;
    let sub_path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let search_dir = resolve_safe_path(ctx.worktree_dir, sub_path)?;

    let mut results = Vec::new();
    collect_matching_files(&search_dir, ctx.worktree_dir, pattern, &mut results).await?;
    if results.is_empty() {
        Ok("No matching files found.".to_string())
    } else {
        Ok(results.join("\n"))
    }
}

pub async fn execute_grep(ctx: &ToolContext<'_>, args: &serde_json::Value) -> Result<String> {
    let pattern = args
        .get("pattern")
        .and_then(|v| v.as_str())
        .context("Missing required parameter 'pattern'")?;
    let sub_path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let search_target = resolve_safe_path(ctx.worktree_dir, sub_path)?;
    let pattern = regex::Regex::new(pattern)
        .map_err(|err| anyhow::anyhow!("Invalid regular expression: {err}"))?;

    let mut matches = Vec::new();
    grep_walk(&search_target, ctx.worktree_dir, &pattern, &mut matches).await?;

    if matches.is_empty() {
        Ok("No matches found.".to_string())
    } else {
        Ok(matches.join("\n"))
    }
}

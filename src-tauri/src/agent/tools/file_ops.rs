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

    Ok(norm_candidate)
}

pub async fn emit_file_change_events(ctx: &ToolContext<'_>, rel_path: &str) {
    if let Some(app) = &ctx.app_handle {
        let norm = rel_path.replace('\\', "/");
        if norm.ends_with("boards.json") || norm.ends_with("pipeline.json") {
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

pub fn glob_match_slices(pattern: &[u8], candidate: &[u8]) -> bool {
    if pattern.is_empty() {
        return candidate.is_empty();
    }
    if pattern.starts_with(b"**") {
        let rest_pat = if pattern.starts_with(b"**/*") {
            &pattern[4..]
        } else if pattern.starts_with(b"**/") {
            &pattern[3..]
        } else {
            &pattern[2..]
        };
        for i in 0..=candidate.len() {
            if glob_match_slices(rest_pat, &candidate[i..]) {
                return true;
            }
        }
        return false;
    }
    if pattern[0] == b'*' {
        let rest_pat = &pattern[1..];
        for i in 0..=candidate.len() {
            if i > 0 && candidate[i - 1] == b'/' {
                break;
            }
            if glob_match_slices(rest_pat, &candidate[i..]) {
                return true;
            }
        }
        return false;
    }
    if !candidate.is_empty() && (pattern[0] == b'?' || pattern[0] == candidate[0]) {
        return glob_match_slices(&pattern[1..], &candidate[1..]);
    }
    false
}

pub async fn grep_walk(
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

pub async fn execute_read_file(ctx: &ToolContext<'_>, args: &serde_json::Value) -> Result<String> {
    let rel_path = args
        .get("file_path")
        .or_else(|| args.get("path"))
        .and_then(|v| v.as_str())
        .context("Missing required parameter 'file_path'")?;

    let target = resolve_safe_path(ctx.worktree_dir, rel_path)?;
    if !target.exists() {
        bail!("File does not exist: {:?}", rel_path);
    }

    let default_limit = ctx
        .tool_policy
        .tool_settings
        .get("read_file")
        .and_then(|v| v.get("max_lines"))
        .and_then(|v| v.as_u64())
        .unwrap_or(2000) as usize;
    let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(default_limit);

    let content = tokio::fs::read_to_string(&target)
        .await
        .with_context(|| format!("Failed to read file: {:?}", rel_path))?;

    let lines: Vec<&str> = content.lines().collect();
    let start = offset.saturating_sub(1);
    if start >= lines.len() {
        return Ok("(File is empty or offset out of range)\n".to_string());
    }
    let end = (start + limit).min(lines.len());
    if start > end {
        return Ok("(File is empty or offset out of range)\n".to_string());
    }

    let mut output = String::new();
    for (idx, line) in lines[start..end].iter().enumerate() {
        output.push_str(&format!("{}: {}\n", start + idx + 1, line));
    }

    if output.is_empty() {
        output.push_str("(File is empty or offset out of range)\n");
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
    emit_file_change_events(ctx, rel_path).await;
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
    emit_file_change_events(ctx, rel_path).await;
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

    let mut matches = Vec::new();
    grep_walk(&search_target, ctx.worktree_dir, pattern, &mut matches).await?;

    if matches.is_empty() {
        Ok("No matches found.".to_string())
    } else {
        Ok(matches.join("\n"))
    }
}

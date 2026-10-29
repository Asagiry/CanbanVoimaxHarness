//! Rule Loading Tool Handler (load_rule)
//! Extracted from agent.rs per ub-mode Clean Code & Command Pattern principles.

use anyhow::{bail, Context, Result};
use crate::agent::ToolContext;

pub async fn execute_load_rule(
    ctx: &ToolContext<'_>,
    args: &serde_json::Value,
) -> Result<String> {
    let repo_root = ctx.repo_root;
    let worktree_dir = ctx.worktree_dir;

    let rule_raw = args
        .get("rule")
        .or_else(|| args.get("name"))
        .and_then(|v| v.as_str())
        .context("Missing required parameter 'rule'")?;

    if rule_raw.contains("..") || rule_raw.contains(':') || rule_raw.starts_with('/') || rule_raw.starts_with('\\') {
        bail!("Invalid rule name '{}': path traversal characters are forbidden", rule_raw);
    }

    let rule_clean = rule_raw.strip_suffix(".md").unwrap_or(rule_raw);
    let role = ctx.role.unwrap_or("general");

    // Look for rule in prioritized order:
    // 1) repo_root/.cvh/agents/<role>/rules/<rule>.md
    // 2) repo_root/.cvh/rules/<rule>.md
    // 3) repo_root/.cvh/agents/<role>/rules/<rule>
    let mut candidates = vec![
        repo_root.join(".cvh").join("agents").join(role).join("rules").join(format!("{}.md", rule_clean)),
        repo_root.join(".cvh").join("rules").join(format!("{}.md", rule_clean)),
        repo_root.join(".cvh").join("agents").join(role).join("rules").join(rule_raw),
    ];

    // If running in an isolated worktree or test dir:
    if worktree_dir != repo_root {
        candidates.push(worktree_dir.join(".cvh").join("agents").join(role).join("rules").join(format!("{}.md", rule_clean)));
        candidates.push(worktree_dir.join(".cvh").join("rules").join(format!("{}.md", rule_clean)));
        candidates.push(worktree_dir.join(".cvh").join("agents").join(role).join("rules").join(rule_raw));
    }

    // Fallback: search across any agents' rules directory
    let agents_base = repo_root.join(".cvh").join("agents");
    if agents_base.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&agents_base) {
            for entry in entries.flatten() {
                let path = entry.path().join("rules").join(format!("{}.md", rule_clean));
                if !candidates.contains(&path) {
                    candidates.push(path);
                }
            }
        }
    }

    let mut content = None;
    for cand in candidates {
        if cand.is_file() {
            match tokio::fs::read_to_string(&cand).await {
                Ok(text) => {
                    content = Some(text);
                    break;
                }
                Err(e) => {
                    bail!("Failed to read rule file at {:?}: {}", cand, e);
                }
            }
        }
    }

    if let Some(text) = content {
        Ok(text)
    } else {
        bail!(
            "Rule '{}' not found in .cvh/agents/{}/rules or .cvh/rules",
            rule_raw,
            role
        )
    }
}

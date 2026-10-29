//! Command Execution Tool Handlers (execute_command, custom tools)
//! Extracted from agent.rs per ub-mode Clean Code & Command Pattern principles.

use std::sync::Arc;
use std::time::Duration;
use anyhow::{bail, Context, Result};

use crate::agent::{load_custom_tools, run_scoped_command, ToolContext};

pub async fn execute_command(
    ctx: &ToolContext<'_>,
    args: &serde_json::Value,
) -> Result<String> {
    let repo_root = ctx.repo_root;
    let worktree_dir = ctx.worktree_dir;
    let abort_handle = &ctx.abort_handle;

    let command_str = args
        .get("command")
        .or_else(|| args.get("cmd"))
        .and_then(|v| v.as_str())
        .context("Missing required parameter 'command'")?;
    let default_timeout = ctx
        .tool_policy
        .tool_settings
        .get("execute_command")
        .and_then(|v| v.get("timeout_sec"))
        .and_then(|v| v.as_u64())
        .unwrap_or(180);
    let timeout_secs = args
        .get("timeout_seconds")
        .and_then(|v| v.as_u64())
        .unwrap_or(default_timeout);

    let res = run_scoped_command(
        worktree_dir,
        command_str,
        Duration::from_secs(timeout_secs),
        Arc::clone(abort_handle),
        Some(repo_root),
    ).await?;

    Ok(format!(
        "[Exit Code: {}]
--- STDOUT ---
{}
--- STDERR ---
{}",
        res.exit_code.unwrap_or(-1),
        res.stdout,
        res.stderr
    ))
}

pub async fn execute_custom_tool(
    ctx: &ToolContext<'_>,
    custom_tool: &str,
    args: &serde_json::Value,
) -> Result<String> {
    let repo_root = ctx.repo_root;
    let worktree_dir = ctx.worktree_dir;
    let abort_handle = &ctx.abort_handle;

    let custom_tools = load_custom_tools(repo_root).await;
    if let Some(ct) = custom_tools.into_iter().find(|t| t.name == custom_tool) {
        let cmd_str = ct.command.replace("{{args}}", &args.to_string());
        let res = run_scoped_command(
            worktree_dir,
            &cmd_str,
            Duration::from_secs(60),
            Arc::clone(abort_handle),
            Some(repo_root),
        ).await?;
        Ok(format!(
            "[Custom Tool: {} Exit: {}]
STDOUT: {}
STDERR: {}",
            custom_tool,
            res.exit_code.unwrap_or(-1),
            res.stdout.trim(),
            res.stderr.trim()
        ))
    } else {
        bail!("Unknown tool: {}", custom_tool);
    }
}

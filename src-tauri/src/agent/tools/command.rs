//! Command Execution Tool Handlers (execute_command, custom tools)
//! Extracted from agent.rs per ub-mode Clean Code & Command Pattern principles.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use anyhow::{bail, Context, Result};

use crate::agent::{load_custom_tools, run_scoped_command, ToolContext};

/// Parses a leading `cd [/d] <path> && <rest>` or `cd [/d] <path> & <rest>` segment,
/// returning `Some((target_path, rest_command))`.
pub fn parse_leading_cd(cmd: &str) -> Option<(&str, &str)> {
    let trimmed = cmd.trim_start();
    if trimmed.len() < 3 || !trimmed.get(..2).is_some_and(|p| p.eq_ignore_ascii_case("cd")) {
        return None;
    }
    let after_cd = trimmed.get(2..)?;
    if !after_cd.starts_with([' ', '\t']) {
        return None;
    }
    let mut s = after_cd.trim_start();

    // Optional /d or -d switch
    if let Some(flag) = s.get(..2) {
        if flag.eq_ignore_ascii_case("/d") || flag.eq_ignore_ascii_case("-d") {
            if let Some(after_flag) = s.get(2..) {
                if after_flag.starts_with([' ', '\t', '"', '\'']) {
                    s = after_flag.trim_start();
                }
            }
        }
    }

    if s.is_empty() {
        return None;
    }

    let (target, after_target) = if let Some(rest_quoted) = s.strip_prefix('"') {
        let end_quote = rest_quoted.find('"')?;
        (&rest_quoted[..end_quote], &rest_quoted[end_quote + 1..])
    } else if let Some(rest_quoted) = s.strip_prefix('\'') {
        let end_quote = rest_quoted.find('\'')?;
        (&rest_quoted[..end_quote], &rest_quoted[end_quote + 1..])
    } else {
        let end_idx = s.find([' ', '\t', '&'])?;
        (&s[..end_idx], &s[end_idx..])
    };

    let after_ws = after_target.trim_start();
    let after_sep = if let Some(r) = after_ws.strip_prefix("&&") {
        r
    } else {
        after_ws.strip_prefix('&')?
    };

    let rest = after_sep.trim_start();
    if rest.is_empty() {
        return None;
    }

    Some((target.trim(), rest))
}

/// Strips any leading `cd [/d] <path> &&` prefix regardless of target directory.
pub fn strip_any_leading_cd(cmd: &str) -> &str {
    let mut current = cmd.trim_start();
    while let Some((_, rest)) = parse_leading_cd(current) {
        current = rest;
    }
    current
}

fn normalize_path_for_cmp(p: &str) -> String {
    let s = p.trim().trim_matches(['"', '\'']).replace('\\', "/");
    let s = s.strip_prefix("//?/").unwrap_or(&s);
    s.trim_end_matches('/').to_ascii_lowercase()
}

/// Strips a redundant leading `cd /d "<path>" && `, `cd /d <path> && `, `cd "<path>" && `,
/// or `cd <path> && ` when `<path>` points to `worktree_dir`.
pub fn strip_redundant_worktree_cd<'a>(cmd: &'a str, worktree_dir: &Path) -> &'a str {
    let norm_wt = normalize_path_for_cmp(&worktree_dir.to_string_lossy());
    let wt_marker = norm_wt.rfind("/worktrees/").map(|idx| &norm_wt[idx..]);
    let mut current = cmd;

    while let Some((target, rest)) = parse_leading_cd(current) {
        let norm_target = normalize_path_for_cmp(target);
        let is_same_worktree = norm_target == "."
            || norm_target == "./"
            || (!norm_wt.is_empty() && norm_target == norm_wt)
            || match wt_marker {
                Some(marker) if marker.len() > "/worktrees/".len() => norm_target.ends_with(marker),
                _ => false,
            };
        if is_same_worktree {
            current = rest;
        } else {
            break;
        }
    }

    current
}

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
    let cleaned_command = strip_redundant_worktree_cd(command_str, worktree_dir);
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

    let configured_shell = ctx
        .tool_policy
        .tool_settings
        .get("execute_command")
        .and_then(|v| v.get("shell"))
        .and_then(|v| v.as_str());

    let res = run_scoped_command(
        worktree_dir,
        cleaned_command,
        Duration::from_secs(timeout_secs),
        Arc::clone(abort_handle),
        Some(repo_root),
        configured_shell,
    ).await?;

    let output = format!(
        "--- STDOUT ---\n{}\n--- STDERR ---\n{}",
        res.stdout,
        res.stderr
    );

    if res.exit_code != Some(0) {
        bail!("[Exit Code: {}]\n{}", res.exit_code.unwrap_or(-1), output);
    }

    Ok(format!("[Exit Code: 0]\n{}", output))
}

pub fn format_custom_tool_command(template: &str, args: &serde_json::Value) -> Result<String> {
    if !template.contains("{{args}}") {
        return Ok(template.to_string());
    }
    let args_json = serde_json::to_string(args)?;
    if args_json.contains('\0') {
        bail!("Custom tool arguments contain disallowed NUL byte");
    }

    #[cfg(target_os = "windows")]
    let uses_cmd_exe = crate::agent::command_runner::find_git_bash().is_none();
    #[cfg(not(target_os = "windows"))]
    let uses_cmd_exe = false;

    let safe_arg = if uses_cmd_exe {
        if args_json.chars().any(|c| {
            matches!(c, '&' | '|' | '<' | '>' | '^' | '%' | '!' | '`' | '$' | '(' | ')' | ';' | '\r' | '\n')
        }) {
            bail!("Custom tool arguments contain unsafe cmd.exe shell metacharacters");
        }
        crate::hooks::quote_windows_arg(&args_json)
    } else {
        format!("'{}'", args_json.replace('\'', "'\\''"))
    };

    Ok(template
        .replace("'{{args}}'", &safe_arg)
        .replace("\"{{args}}\"", &safe_arg)
        .replace("{{args}}", &safe_arg))
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
        let cmd_str = format_custom_tool_command(&ct.command, args)?;
        let res = run_scoped_command(
            worktree_dir,
            &cmd_str,
            Duration::from_secs(60),
            Arc::clone(abort_handle),
            Some(repo_root),
            None,
        ).await?;
        Ok(format!(
            "[Custom Tool: {} Exit: {}]\nSTDOUT: {}\nSTDERR: {}",
            custom_tool,
            res.exit_code.unwrap_or(-1),
            res.stdout.trim(),
            res.stderr.trim()
        ))
    } else {
        bail!("Unknown tool: {}", custom_tool);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_strip_redundant_worktree_cd_variants() {
        let wt = Path::new(r"C:\Users\VibeCode\CanbanVoimaxHarness\worktrees\CVH-057");

        assert_eq!(
            strip_redundant_worktree_cd(
                r#"cd /d "C:\Users\VibeCode\CanbanVoimaxHarness\worktrees\CVH-057" && findstr /n "foo" bar.rs"#,
                wt
            ),
            r#"findstr /n "foo" bar.rs"#
        );

        assert_eq!(
            strip_redundant_worktree_cd(
                r"cd /d C:\Users\VibeCode\CanbanVoimaxHarness\worktrees\CVH-057 && cargo check",
                wt
            ),
            "cargo check"
        );

        assert_eq!(
            strip_redundant_worktree_cd(
                "cd \"c:/users/vibecode/canbanvoimaxharness/worktrees/cvh-057/\" && pnpm test",
                wt
            ),
            "pnpm test"
        );

        assert_eq!(
            strip_redundant_worktree_cd("cd src-tauri && cargo check", wt),
            "cd src-tauri && cargo check"
        );
    }
}

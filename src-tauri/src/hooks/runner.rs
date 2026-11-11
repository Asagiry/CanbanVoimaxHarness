use super::process::{
    build_windows_cmd_raw_arg, kill_process_tree, quote_windows_path, read_pipe_capped,
    resolve_powershell_executable, ProcessTreeGuard, MAX_PIPE_CAPTURE_BYTES,
};
#[cfg(windows)]
use super::process::CREATE_NO_WINDOW;
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HookContext {
    pub task_id: String,
    pub task_title: String,
    pub worktree_path: String,
    pub branch: String,
    #[serde(default)]
    pub verdict: Option<String>,
    #[serde(default)]
    pub stage_id: Option<String>,
    #[serde(default)]
    pub board_id: Option<String>,
    #[serde(default)]
    pub extra_env: HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HookExecutionResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Executes a hook script (.bat, .cmd, .ps1, .sh) with environment variables injected.
pub async fn execute_hook(
    script_path: impl AsRef<Path>,
    working_dir: impl AsRef<Path>,
    context: &HookContext,
    timeout: Option<Duration>,
) -> Result<HookExecutionResult> {
    let script = script_path.as_ref();
    let cwd = working_dir.as_ref();

    if !script.exists() {
        bail!("Hook script does not exist at {:?}", script);
    }

    let extension = script
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut cmd = match extension.as_str() {
        "bat" | "cmd" => {
            #[cfg(target_os = "windows")]
            {
                let mut c = Command::new("cmd.exe");
                let quoted_script = quote_windows_path(script);
                let call_cmd = format!("call {}", quoted_script);
                c.raw_arg("/D");
                c.raw_arg("/S");
                c.raw_arg("/C");
                c.raw_arg(build_windows_cmd_raw_arg(&call_cmd)?);
                c
            }
            #[cfg(not(target_os = "windows"))]
            {
                let mut c = Command::new("sh");
                c.arg(script);
                c
            }
        }
        "ps1" => {
            let exe = resolve_powershell_executable();
            let mut c = Command::new(exe);
            c.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
                .arg(script);
            c
        }
        "py" => {
            let mut c = Command::new("python");
            c.arg(script);
            c
        }
        "sh" => {
            let mut c = Command::new("sh");
            c.arg(script);
            c
        }
        _ => {
            // Default direct invocation
            Command::new(script)
        }
    };

    cmd.current_dir(cwd);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    // Inject CVH environment variables
    cmd.env("CVH_TASK_ID", &context.task_id);
    cmd.env("CVH_TASK_TITLE", &context.task_title);
    cmd.env("CVH_WORKTREE_PATH", &context.worktree_path);
    cmd.env("CVH_BRANCH", &context.branch);
    if let Some(v) = &context.verdict {
        cmd.env("CVH_VERDICT", v);
    }
    if let Some(stage) = &context.stage_id {
        cmd.env("CVH_STAGE_ID", stage);
    }
    if let Some(board) = &context.board_id {
        cmd.env("CVH_BOARD_ID", board);
    }
    for (k, v) in &context.extra_env {
        cmd.env(k, v);
    }

    // Inject shared CARGO_TARGET_DIR to avoid cold compilations in worktrees
    if !context.extra_env.contains_key("CARGO_TARGET_DIR") {
        if let Ok(existing) = std::env::var("CARGO_TARGET_DIR") {
            if !existing.trim().is_empty() {
                cmd.env("CARGO_TARGET_DIR", existing);
            }
        } else {
            let abs_script = if script.is_absolute() {
                script.to_path_buf()
            } else {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(script)
            };
            let mut resolved_target = None;
            for ancestor in abs_script.ancestors() {
                if ancestor.file_name().and_then(|n| n.to_str()) == Some(".cvh") {
                    if let Some(repo_root) = ancestor.parent() {
                        resolved_target = Some(repo_root.join("src-tauri").join("target"));
                        break;
                    }
                }
            }
            if resolved_target.is_none() {
                let abs_cwd = if cwd.is_absolute() {
                    cwd.to_path_buf()
                } else {
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(cwd)
                };
                for ancestor in abs_cwd.ancestors() {
                    if ancestor.file_name().and_then(|n| n.to_str()) == Some("worktrees") {
                        if let Some(repo_root) = ancestor.parent() {
                            resolved_target = Some(repo_root.join("src-tauri").join("target"));
                            break;
                        }
                    }
                }
            }
            if let Some(target) = resolved_target {
                cmd.env("CARGO_TARGET_DIR", target);
            }
        }
    }

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    #[cfg(unix)]
    cmd.process_group(0);
    cmd.kill_on_drop(true);

    let mut child = cmd.spawn().with_context(|| format!("Failed to spawn hook script {:?}", script))?;
    let pid = child.id();
    let mut tree_guard = ProcessTreeGuard::new(pid);

    let stdout_pipe = child.stdout.take().context("Failed to open child stdout")?;
    let stderr_pipe = child.stderr.take().context("Failed to open child stderr")?;

    let timeout_duration = timeout.unwrap_or(Duration::from_secs(180));

    let run_task = async {
        let stdout_drain = async {
            let (buf, truncated) = read_pipe_capped(stdout_pipe, MAX_PIPE_CAPTURE_BYTES).await;
            let mut s = crate::agent::decode_output_bytes(&buf);
            if truncated {
                s.push_str("\n... [Output truncated: exceeded 4 MiB capture limit]");
            }
            s
        };

        let stderr_drain = async {
            let (buf, truncated) = read_pipe_capped(stderr_pipe, MAX_PIPE_CAPTURE_BYTES).await;
            let mut s = crate::agent::decode_output_bytes(&buf);
            if truncated {
                s.push_str("\n... [Output truncated: exceeded 4 MiB capture limit]");
            }
            s
        };

        let (stdout_str, stderr_str) = tokio::join!(stdout_drain, stderr_drain);
        let status = child.wait().await?;
        tree_guard.disarm();
        Ok::<HookExecutionResult, anyhow::Error>(HookExecutionResult {
            success: status.success(),
            exit_code: status.code(),
            stdout: stdout_str,
            stderr: stderr_str,
        })
    };

    tokio::select! {
        res = run_task => res,
        _ = tokio::time::sleep(timeout_duration) => {
            if let Some(p) = pid {
                kill_process_tree(p).await;
                tree_guard.disarm();
            }
            let _ = child.kill().await;
            bail!("Hook script {:?} timed out after {:?}", script, timeout_duration)
        }
    }
}

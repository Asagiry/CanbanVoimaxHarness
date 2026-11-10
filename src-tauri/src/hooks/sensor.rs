use super::process::{kill_process_tree, ProcessTreeGuard};
#[cfg(windows)]
use super::process::CREATE_NO_WINDOW;
use super::runner::HookExecutionResult;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

#[cfg(target_os = "windows")]
pub fn parse_command_token(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    if let Some(stripped) = s.strip_prefix('"') {
        if let Some(end) = stripped.find('"') {
            return (&stripped[..end], stripped[end + 1..].trim_start());
        }
    } else if let Some(stripped) = s.strip_prefix('\'') {
        if let Some(end) = stripped.find('\'') {
            return (&stripped[..end], stripped[end + 1..].trim_start());
        }
    }
    match s.find(|c: char| c.is_whitespace()) {
        Some(idx) => (&s[..idx], s[idx..].trim_start()),
        None => (s, ""),
    }
}

/// Executes an arbitrary gatekeeper sensor command (e.g. "cargo check" or "npm test")
pub async fn execute_sensor_command(
    command_str: &str,
    working_dir: impl AsRef<Path>,
    timeout: Option<Duration>,
) -> Result<HookExecutionResult> {
    let cwd_ref = working_dir.as_ref();
    let cwd_path = if cwd_ref.exists() {
        cwd_ref.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = Command::new("cmd.exe");
        c.raw_arg("/D");
        c.raw_arg("/S");
        c.raw_arg("/C");
        let effective_cmd = {
            let trimmed = command_str.trim();
            let (is_call, token_slice, rest) = if trimmed.len() >= 5 && trimmed[..5].eq_ignore_ascii_case("call ") {
                let after_call = trimmed[5..].trim_start();
                let (tok, r) = parse_command_token(after_call);
                (true, tok, r)
            } else {
                let (tok, r) = parse_command_token(trimmed);
                (false, tok, r)
            };

            let unquoted = token_slice.trim_matches('"').trim_matches('\'');
            let lower = unquoted.to_ascii_lowercase();
            if lower.ends_with(".bat") || lower.ends_with(".cmd") {
                let normalized = unquoted.replace('/', "\\");
                if rest.is_empty() {
                    format!("call \"{}\"", normalized)
                } else {
                    format!("call \"{}\" {}", normalized, rest)
                }
            } else if is_call {
                trimmed.to_string()
            } else {
                command_str.to_string()
            }
        };
        c.raw_arg(format!("\"chcp 65001 >nul && {}\"", effective_cmd));
        c
    };

    #[cfg(not(target_os = "windows"))]
    let mut cmd = {
        let mut c = Command::new("sh");
        c.args(["-c", command_str]);
        c
    };

    cmd.current_dir(&cwd_path);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    // Inject shared CARGO_TARGET_DIR to avoid cold compilations in worktrees
    if let Ok(existing) = std::env::var("CARGO_TARGET_DIR") {
        if !existing.trim().is_empty() {
            cmd.env("CARGO_TARGET_DIR", existing);
        }
    } else {
        let abs_cwd = if cwd_path.is_absolute() {
            cwd_path.clone()
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(&cwd_path)
        };
        for ancestor in abs_cwd.ancestors() {
            if ancestor.file_name().and_then(|n| n.to_str()) == Some("worktrees") {
                if let Some(repo_root) = ancestor.parent() {
                    cmd.env("CARGO_TARGET_DIR", repo_root.join("src-tauri").join("target"));
                    break;
                }
            }
        }
    }

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.kill_on_drop(true);

    let mut child = cmd.spawn().with_context(|| format!("Failed to spawn sensor command: {}", command_str))?;
    let pid = child.id();
    let mut tree_guard = ProcessTreeGuard::new(pid);

    let mut stdout_pipe = child.stdout.take().context("Failed to open child stdout")?;
    let mut stderr_pipe = child.stderr.take().context("Failed to open child stderr")?;

    let timeout_duration = timeout.unwrap_or(Duration::from_secs(300));

    let run_task = async {
        let stdout_drain = async {
            let mut buf = Vec::new();
            let _ = stdout_pipe.read_to_end(&mut buf).await;
            crate::agent::decode_output_bytes(&buf)
        };

        let stderr_drain = async {
            let mut buf = Vec::new();
            let _ = stderr_pipe.read_to_end(&mut buf).await;
            crate::agent::decode_output_bytes(&buf)
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
            }
            let _ = child.kill().await;
            bail!("Sensor command '{}' timed out after {:?}", command_str, timeout_duration)
        }
    }
}

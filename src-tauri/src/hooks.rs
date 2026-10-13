use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
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

pub async fn kill_process_tree(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .output()
            .await;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output()
            .await;
    }
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
            let mut c = Command::new("cmd.exe");
            c.arg("/C").arg(script);
            c
        }
        "ps1" => {
            let mut c = Command::new("powershell.exe");
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

    let mut child = cmd.spawn().with_context(|| format!("Failed to spawn hook script {:?}", script))?;
    let pid = child.id();

    let mut stdout_pipe = child.stdout.take().context("Failed to open child stdout")?;
    let mut stderr_pipe = child.stderr.take().context("Failed to open child stderr")?;

    let timeout_duration = timeout.unwrap_or(Duration::from_secs(180));

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
            bail!("Hook script {:?} timed out after {:?}", script, timeout_duration)
        }
    }
}

/// Executes an arbitrary gatekeeper sensor command (e.g. "cargo check" or "npm test")
pub async fn execute_sensor_command(
    command_str: &str,
    working_dir: impl AsRef<Path>,
    timeout: Option<Duration>,
) -> Result<HookExecutionResult> {
    let cwd = working_dir.as_ref();

    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = Command::new("cmd.exe");
        c.args(["/C", command_str]);
        c
    };

    #[cfg(not(target_os = "windows"))]
    let mut cmd = {
        let mut c = Command::new("sh");
        c.args(["-c", command_str]);
        c
    };

    cmd.current_dir(cwd);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().with_context(|| format!("Failed to spawn sensor command: {}", command_str))?;
    let pid = child.id();

    let mut stdout_pipe = child.stdout.take().context("Failed to open child stdout")?;
    let mut stderr_pipe = child.stderr.take().context("Failed to open child stderr")?;

    let timeout_duration = timeout.unwrap_or(Duration::from_secs(300));

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

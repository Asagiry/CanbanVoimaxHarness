use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

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

/// Properly quotes an argument for Windows command execution.
/// Guarantees that spaces, parentheses `()`, ampersands `&`, carets `^`, and non-ASCII paths
/// are safely escaped and enclosed in double quotes.
pub fn quote_windows_arg(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".to_string();
    }
    if arg.starts_with('"') && arg.ends_with('"') && arg.len() >= 2 && !arg[1..arg.len() - 1].contains('"') {
        return arg.to_string();
    }
    let needs_quotes = arg.chars().any(|c| {
        c.is_whitespace() || matches!(c, '(' | ')' | '&' | '^' | '|' | '<' | '>' | '%' | '!' | '"' | ',')
    });

    if needs_quotes {
        let mut quoted = String::with_capacity(arg.len() + 4);
        quoted.push('"');
        for c in arg.chars() {
            if c == '"' {
                quoted.push('\\');
            }
            quoted.push(c);
        }
        quoted.push('"');
        quoted
    } else {
        arg.to_string()
    }
}

/// Properly quotes a Windows path for use in shell commands.
/// If the path contains spaces, parentheses, '&', '^', '%', or other special characters,
/// it is wrapped in double quotes with internal quotes escaped.
pub fn quote_windows_path(path: impl AsRef<Path>) -> String {
    let raw = path.as_ref().to_string_lossy();
    quote_windows_arg(&raw)
}

/// Formats a directory change command `cd /d "<path>"` with robust Windows quotation handling.
/// Correctly handles paths with spaces, non-ASCII characters, and parentheses.
pub fn format_windows_cd_command(dir: impl AsRef<Path>, cmd: &str) -> String {
    let quoted_dir = quote_windows_path(dir);
    format!("cd /d {} && {}", quoted_dir, cmd)
}

/// Wraps a command string for execution under Windows `cmd.exe`.
/// Ensures:
/// 1. `/D` ignores AutoRun registry scripts.
/// 2. `/S` preserves inner quotations and prevents quote-stripping with parentheses/spaces.
/// 3. `chcp 65001 >nul` forces UTF-8 encoding across all child processes and outputs.
pub fn build_windows_cmd_args(raw_cmd: &str) -> Vec<String> {
    vec![
        "/D".to_string(),
        "/S".to_string(),
        "/C".to_string(),
        format!("chcp 65001 >nul && {}", raw_cmd),
    ]
}

pub async fn kill_process_tree(pid: u32) {
    if pid == 0 {
        return;
    }
    #[cfg(target_os = "windows")]
    {
        let taskkill_bin = std::env::var("SystemRoot")
            .map(|sr| std::path::PathBuf::from(sr).join("System32").join("taskkill.exe"))
            .ok()
            .filter(|p| p.exists())
            .unwrap_or_else(|| std::path::PathBuf::from("taskkill.exe"));

        let mut cmd = Command::new(taskkill_bin);
        let pid_str = pid.to_string();
        cmd.args(["/F", "/T", "/PID", &pid_str]);
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);
        let _ = cmd.output().await;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let pid_str = pid.to_string();
        let _ = Command::new("kill")
            .args(["-9", &pid_str])
            .output()
            .await;
    }
}

/// Checks if `pwsh` (PowerShell Core / 7+) is available on PATH.
pub fn is_pwsh_available() -> bool {
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            #[cfg(target_os = "windows")]
            {
                if dir.join("pwsh.exe").is_file()
                    || dir.join("pwsh.cmd").is_file()
                    || dir.join("pwsh.bat").is_file()
                {
                    return true;
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                if dir.join("pwsh").is_file() {
                    return true;
                }
            }
        }
    }
    false
}

/// Resolves the preferred PowerShell executable: prefers modern `pwsh` if available,
/// falling back to `powershell.exe` on Windows (or `pwsh` on Unix).
pub fn resolve_powershell_executable() -> &'static str {
    if is_pwsh_available() {
        "pwsh"
    } else {
        #[cfg(target_os = "windows")]
        {
            "powershell.exe"
        }
        #[cfg(not(target_os = "windows"))]
        {
            "pwsh"
        }
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
            #[cfg(target_os = "windows")]
            {
                let mut c = Command::new("cmd.exe");
                let quoted_script = quote_windows_path(script);
                let call_cmd = format!("call {}", quoted_script);
                c.raw_arg("/D");
                c.raw_arg("/S");
                c.raw_arg("/C");
                c.raw_arg(format!("\"chcp 65001 >nul && {}\"", call_cmd));
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

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

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
        c.raw_arg("/D");
        c.raw_arg("/S");
        c.raw_arg("/C");
        c.raw_arg(format!("\"chcp 65001 >nul && {}\"", command_str));
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

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_quote_windows_arg_cases() {
        assert_eq!(quote_windows_arg(""), r#""""#);
        assert_eq!(quote_windows_arg("cargo"), "cargo");
        assert_eq!(quote_windows_arg("hello world"), r#""hello world""#);
        assert_eq!(quote_windows_arg("my_func(arg)"), r#""my_func(arg)""#);
        assert_eq!(quote_windows_arg("путь (тест)"), r#""путь (тест)""#);
        assert_eq!(quote_windows_arg("a&b"), r#""a&b""#);
        assert_eq!(quote_windows_arg("a^b"), r#""a^b""#);
        assert_eq!(quote_windows_arg(r#""already_quoted""#), r#""already_quoted""#);
    }

    #[test]
    fn test_quote_windows_path() {
        let path = PathBuf::from(r#"C:\Program Files (x86)\My App\hook.bat"#);
        assert_eq!(
            quote_windows_path(&path),
            r#""C:\Program Files (x86)\My App\hook.bat""#
        );

        let cyrillic_path = PathBuf::from(r#"C:\Проекты (Рабочие)\тест.cmd"#);
        assert_eq!(
            quote_windows_path(&cyrillic_path),
            r#""C:\Проекты (Рабочие)\тест.cmd""#
        );
    }

    #[test]
    fn test_format_windows_cd_command() {
        let dir = PathBuf::from(r#"D:\Workspace (Beta)\repo"#);
        let formatted = format_windows_cd_command(&dir, "cargo test");
        assert_eq!(
            formatted,
            r#"cd /d "D:\Workspace (Beta)\repo" && cargo test"#
        );
    }

    #[test]
    fn test_build_windows_cmd_args() {
        let args = build_windows_cmd_args("cargo check --tests");
        assert_eq!(args.len(), 4);
        assert_eq!(args[0], "/D");
        assert_eq!(args[1], "/S");
        assert_eq!(args[2], "/C");
        assert_eq!(args[3], "chcp 65001 >nul && cargo check --tests");
    }

    #[tokio::test]
    async fn test_kill_process_tree_zero_pid_is_noop() {
        // PID 0 must immediately return without killing system idle process or spawning taskkill
        kill_process_tree(0).await;
    }

    #[test]
    fn test_resolve_powershell_executable() {
        let exe = resolve_powershell_executable();
        #[cfg(target_os = "windows")]
        {
            assert!(exe == "pwsh" || exe == "powershell.exe");
        }
        #[cfg(not(target_os = "windows"))]
        {
            assert_eq!(exe, "pwsh");
        }
    }

    #[tokio::test]
    async fn test_execute_sensor_command_windows_quotes() {
        let res = execute_sensor_command(r#"python -c "print('sensor_ok')""#, ".", Some(Duration::from_secs(10)))
            .await
            .expect("sensor command execution");
        assert!(res.success, "sensor command should succeed, stderr: {}", res.stderr);
        assert!(res.stdout.contains("sensor_ok"), "got stdout: {}", res.stdout);
    }
}

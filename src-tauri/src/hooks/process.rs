use std::path::Path;
use tokio::process::Command;

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

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

/// Synchronous, detached process tree killer for RAII and drop guards.
pub fn kill_process_tree_sync(pid: u32) {
    if pid == 0 {
        return;
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let taskkill_bin = std::env::var("SystemRoot")
            .map(|sr| std::path::PathBuf::from(sr).join("System32").join("taskkill.exe"))
            .ok()
            .filter(|p| p.exists())
            .unwrap_or_else(|| std::path::PathBuf::from("taskkill.exe"));

        let mut cmd = std::process::Command::new(taskkill_bin);
        let pid_str = pid.to_string();
        cmd.args(["/F", "/T", "/PID", &pid_str]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        if let Ok(mut child) = cmd.spawn() {
            let _ = child.wait();
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let pid_str = pid.to_string();
        if let Ok(mut child) = std::process::Command::new("kill")
            .args(["-9", &pid_str])
            .spawn()
        {
            let _ = child.wait();
        }
    }
}

/// RAII Guard that guarantees `kill_process_tree_sync` is invoked if dropped while armed.
pub struct ProcessTreeGuard {
    pub pid: Option<u32>,
}

impl ProcessTreeGuard {
    pub fn new(pid: Option<u32>) -> Self {
        Self { pid }
    }

    pub fn disarm(&mut self) {
        self.pid = None;
    }
}

impl Drop for ProcessTreeGuard {
    fn drop(&mut self) {
        if let Some(pid) = self.pid {
            kill_process_tree_sync(pid);
        }
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

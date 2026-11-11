use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use crate::hooks::{
    build_windows_cmd_raw_arg, kill_process_tree, read_pipe_capped, MAX_PIPE_CAPTURE_BYTES,
};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Mapping table for CP866 (IBM866 OEM Cyrillic) bytes 0x80..=0xFF to Unicode char.
pub const CP866_TABLE: [char; 128] = [
    'А', 'Б', 'В', 'Г', 'Д', 'Е', 'Ж', 'З', 'И', 'Й', 'К', 'Л', 'М', 'Н', 'О', 'П',
    'Р', 'С', 'Т', 'У', 'Ф', 'Х', 'Ц', 'Ч', 'Ш', 'Щ', 'Ъ', 'Ы', 'Ь', 'Э', 'Ю', 'Я',
    'а', 'б', 'в', 'г', 'д', 'е', 'ж', 'з', 'и', 'й', 'к', 'л', 'м', 'н', 'о', 'п',
    '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕', '╣', '║', '╗', '╝', '╜', '╛', '┐',
    '└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦', '╠', '═', '╬', '╧',
    '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐', '▀',
    'р', 'с', 'т', 'у', 'ф', 'х', 'ц', 'ч', 'ш', 'щ', 'ъ', 'ы', 'ь', 'э', 'ю', 'я',
    'Ё', 'ё', 'Є', 'є', 'Ї', 'ї', 'Ў', 'ў', '°', '∙', '·', '√', '№', '¤', '■', '\u{00A0}',
];

/// Mapping table for Windows-1251 (CP1251 ANSI Cyrillic) bytes 0x80..=0xFF to Unicode char.
pub const CP1251_TABLE: [char; 128] = [
    'Ђ', 'Ѓ', '‚', 'ѓ', '„', '…', '†', '‡', '€', '‰', 'Љ', '‹', 'Њ', 'Ќ', 'Ћ', 'Џ',
    'ђ', '‘', '’', '“', '”', '•', '–', '—', '\u{FFFD}', '™', 'љ', '›', 'њ', 'ќ', 'ћ', 'џ',
    '\u{00A0}', 'Ў', 'ў', 'Ј', '¤', 'Ґ', '¦', '§', 'Ё', '©', 'Є', '«', '¬', '\u{00AD}', '®', 'Ї',
    '°', '±', 'І', 'і', 'ґ', 'µ', '¶', '·', 'ё', '№', 'є', '»', 'ј', 'Ѕ', 'ѕ', 'ї',
    'А', 'Б', 'В', 'Г', 'Д', 'Е', 'Ж', 'З', 'И', 'Й', 'К', 'Л', 'М', 'Н', 'О', 'П',
    'Р', 'С', 'Т', 'У', 'Ф', 'Х', 'Ц', 'Ч', 'Ш', 'Щ', 'Ъ', 'Ы', 'Ь', 'Э', 'Ю', 'Я',
    'а', 'б', 'в', 'г', 'д', 'е', 'ж', 'з', 'и', 'й', 'к', 'л', 'м', 'н', 'о', 'п',
    'р', 'с', 'т', 'у', 'ф', 'х', 'ц', 'ч', 'ш', 'щ', 'ъ', 'ы', 'ь', 'э', 'ю', 'я',
];

fn decode_with_table(bytes: &[u8], table: &[char; 128]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        if b < 0x80 {
            out.push(b as char);
        } else {
            out.push(table[(b - 0x80) as usize]);
        }
    }
    out
}

/// Decodes CP866 (OEM code page 866 used by Russian Windows cmd.exe) bytes into valid UTF-8.
pub fn decode_cp866(bytes: &[u8]) -> String {
    decode_with_table(bytes, &CP866_TABLE)
}

/// Decodes Windows-1251 (CP1251 ANSI Cyrillic) bytes into valid UTF-8.
pub fn decode_cp1251(bytes: &[u8]) -> String {
    decode_with_table(bytes, &CP1251_TABLE)
}

/// Decodes command output bytes (stdout/stderr) with resilient fallback for CP866 / CP1251 on Russian Windows.
pub fn decode_output_bytes(buf: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(buf) {
        if !s.contains('\u{FFFD}') {
            return s.to_string();
        }
    }
    // Heuristic: CP866 Russian lowercase/uppercase spans 0x80..=0xAF + 0xE0..=0xEF,
    // whereas CP1251 spans 0xC0..=0xFF (where 0xB0..=0xDF in CP866 are box-drawing glyphs).
    let mut cp866_alpha = 0usize;
    let mut cp1251_alpha = 0usize;
    for &b in buf {
        if matches!(b, 0x80..=0xAF | 0xE0..=0xEF) {
            cp866_alpha += 1;
        }
        if b >= 0xC0 {
            cp1251_alpha += 1;
        }
    }
    if cp1251_alpha > cp866_alpha {
        decode_cp1251(buf)
    } else {
        decode_cp866(buf)
    }
}

/// Resolves standard runtime locations and enriches Windows PATH if missing.
pub fn build_enriched_windows_paths(current_path: &std::ffi::OsStr) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::env::split_paths(current_path).collect();
    let mut candidates = Vec::new();

    if let Ok(appdata) = std::env::var("APPDATA") {
        if !appdata.trim().is_empty() {
            candidates.push(PathBuf::from(appdata).join("npm"));
        }
    }
    if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
        if !localappdata.trim().is_empty() {
            candidates.push(PathBuf::from(localappdata).join("pnpm"));
        }
    }
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        if !userprofile.trim().is_empty() {
            candidates.push(PathBuf::from(userprofile).join(".cargo").join("bin"));
        }
    }
    let program_files = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string());
    if !program_files.trim().is_empty() {
        candidates.push(PathBuf::from(&program_files).join("nodejs"));
        candidates.push(PathBuf::from(&program_files).join("Git").join("bin"));
        candidates.push(PathBuf::from(&program_files).join("Git").join("usr").join("bin"));
    }
    if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
        if !pf86.trim().is_empty() {
            candidates.push(PathBuf::from(&pf86).join("Git").join("bin"));
            candidates.push(PathBuf::from(&pf86).join("Git").join("usr").join("bin"));
        }
    }
    if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
        if !localappdata.trim().is_empty() {
            candidates.push(PathBuf::from(&localappdata).join("Programs").join("Git").join("bin"));
            candidates.push(PathBuf::from(&localappdata).join("Programs").join("Git").join("usr").join("bin"));
        }
    }

    for candidate in candidates {
        let candidate_str = candidate.to_string_lossy();
        let already_present = paths.iter().any(|p| {
            p.to_string_lossy().eq_ignore_ascii_case(&candidate_str)
        });
        if !already_present {
            paths.push(candidate);
        }
    }
    paths
}

#[cfg(target_os = "windows")]
pub fn find_git_bash() -> Option<PathBuf> {
    let pf = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string());
    let standard = PathBuf::from(&pf).join("Git").join("bin").join("bash.exe");
    if standard.exists() {
        return Some(standard);
    }
    let standard_usr = PathBuf::from(&pf).join("Git").join("usr").join("bin").join("bash.exe");
    if standard_usr.exists() {
        return Some(standard_usr);
    }
    if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
        let candidate = PathBuf::from(&pf86).join("Git").join("bin").join("bash.exe");
        if candidate.exists() {
            return Some(candidate);
        }
        let candidate_usr = PathBuf::from(&pf86).join("Git").join("usr").join("bin").join("bash.exe");
        if candidate_usr.exists() {
            return Some(candidate_usr);
        }
    }
    if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
        let candidate = PathBuf::from(localappdata).join("Programs").join("Git").join("bin").join("bash.exe");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    if let Ok(path_var) = std::env::var("PATH") {
        for p in std::env::split_paths(&path_var) {
            let git_exe = p.join("git.exe");
            if git_exe.exists() {
                if let Some(parent) = p.parent() {
                    let git_bash = parent.join("bin").join("bash.exe");
                    if git_bash.exists() {
                        return Some(git_bash);
                    }
                    let git_usr_bash = parent.join("usr").join("bin").join("bash.exe");
                    if git_usr_bash.exists() {
                        return Some(git_usr_bash);
                    }
                }
            }
            let candidate = p.join("bash.exe");
            let norm = candidate.to_string_lossy().to_ascii_lowercase();
            if !norm.contains("system32") && !norm.contains("windowsapps") && candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
pub fn enrich_windows_path(cmd: &mut Command) {
    let current_path = std::env::var_os("PATH").unwrap_or_default();
    let paths = build_enriched_windows_paths(&current_path);
    if let Ok(new_path) = std::env::join_paths(paths) {
        cmd.env("PATH", new_path);
    }
}

pub async fn run_scoped_command(
    cwd: &Path,
    cmd_str: &str,
    timeout: Duration,
    abort_flag: Arc<AtomicBool>,
    repo_root: Option<&Path>,
    shell: Option<&str>,
) -> Result<crate::hooks::HookExecutionResult> {
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let req_shell = shell.unwrap_or("bash").to_lowercase();
        let mut c = match req_shell.as_str() {
            "pwsh" | "powershell" => {
                let mut c = Command::new("powershell.exe");
                let utf8_cmd = format!(
                    "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $OutputEncoding = [System.Text.Encoding]::UTF8; {}",
                    cmd_str
                );
                c.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &utf8_cmd]);
                c
            }
            "cmd" => {
                let mut c = Command::new("cmd.exe");
                c.raw_arg("/D");
                c.raw_arg("/S");
                c.raw_arg("/C");
                c.raw_arg(build_windows_cmd_raw_arg(cmd_str)?);
                c
            }
            _ => {
                if let Some(bash_path) = find_git_bash() {
                    let mut c = Command::new(bash_path);
                    c.args(["-c", cmd_str]);
                    c
                } else {
                    let mut c = Command::new("cmd.exe");
                    c.raw_arg("/D");
                    c.raw_arg("/S");
                    c.raw_arg("/C");
                    c.raw_arg(build_windows_cmd_raw_arg(cmd_str)?);
                    c
                }
            }
        };
        enrich_windows_path(&mut c);
        c.env("PYTHONIOENCODING", "utf-8");
        c.env("PYTHONUTF8", "1");
        c
    };

    #[cfg(not(target_os = "windows"))]
    let mut cmd = {
        let req_shell = shell.unwrap_or("sh").to_lowercase();
        let sh_bin = if req_shell == "bash" { "bash" } else { "sh" };
        let mut c = Command::new(sh_bin);
        c.args(["-c", cmd_str]);
        c
    };

    cmd.current_dir(cwd);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    // Inject shared CARGO_TARGET_DIR to prevent redundant cold compilations in worktrees
    if let Ok(existing_target) = std::env::var("CARGO_TARGET_DIR") {
        if !existing_target.trim().is_empty() {
            cmd.env("CARGO_TARGET_DIR", existing_target);
        }
    } else if let Some(root) = repo_root {
        cmd.env("CARGO_TARGET_DIR", root.join("src-tauri").join("target"));
    } else {
        let abs_cwd = if cwd.is_absolute() {
            cwd.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(cwd)
        };
        for ancestor in abs_cwd.ancestors() {
            if ancestor.file_name().and_then(|n| n.to_str()) == Some("worktrees") {
                if let Some(root) = ancestor.parent() {
                    cmd.env("CARGO_TARGET_DIR", root.join("src-tauri").join("target"));
                    break;
                }
            }
        }
    }

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    #[cfg(unix)]
    cmd.process_group(0);
    cmd.kill_on_drop(true);

    let mut child = cmd.spawn().with_context(|| format!("Failed to spawn command: {}", cmd_str))?;
    let pid = child.id();
    let mut tree_guard = crate::hooks::ProcessTreeGuard::new(pid);

    let stdout_pipe = child.stdout.take().context("Failed to take stdout")?;
    let stderr_pipe = child.stderr.take().context("Failed to take stderr")?;

    let run_task = async {
        let stdout_drain = async {
            let (buf, truncated) = read_pipe_capped(stdout_pipe, MAX_PIPE_CAPTURE_BYTES).await;
            let mut s = decode_output_bytes(&buf);
            if truncated {
                s.push_str("\n... [Output truncated: exceeded 4 MiB capture limit]");
            }
            s
        };

        let stderr_drain = async {
            let (buf, truncated) = read_pipe_capped(stderr_pipe, MAX_PIPE_CAPTURE_BYTES).await;
            let mut s = decode_output_bytes(&buf);
            if truncated {
                s.push_str("\n... [Output truncated: exceeded 4 MiB capture limit]");
            }
            s
        };
        let (stdout_str, stderr_str) = tokio::join!(stdout_drain, stderr_drain);
        let status = child.wait().await?;
        Ok::<crate::hooks::HookExecutionResult, anyhow::Error>(crate::hooks::HookExecutionResult {
            success: status.success(),
            exit_code: status.code(),
            stdout: stdout_str,
            stderr: stderr_str,
        })
    };

    let check_interval = async {
        loop {
            tokio::time::sleep(Duration::from_millis(200)).await;
            if abort_flag.load(Ordering::SeqCst) {
                if let Some(p) = pid {
                    kill_process_tree(p).await;
                }
                return;
            }
        }
    };

    let res = tokio::select! {
        res = run_task => res,
        _ = tokio::time::sleep(timeout) => {
            if let Some(p) = pid {
                kill_process_tree(p).await;
            }
            Err(anyhow::anyhow!("Command '{}' timed out after {:?}", cmd_str, timeout))
        }
        _ = check_interval => {
            Err(anyhow::anyhow!("Command cancelled by user"))
        }
    };
    tree_guard.disarm();
    res
}

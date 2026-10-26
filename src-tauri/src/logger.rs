use std::fs::{create_dir_all, remove_file, rename, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use chrono::Local;
use tauri::State;
use crate::commands::state::AppState;
use crate::config::{get_app_config_dir, resolve_actual_repo_root};

static LOG_MUTEX: Mutex<()> = Mutex::new(());

const MAX_LOG_SIZE_BYTES: u64 = 10 * 1024 * 1024; // 10 MB per file

/// Determines the primary log directory and file path for the given repo root.
fn get_primary_log_path(repo_root: &Path) -> PathBuf {
    if repo_root.as_os_str().is_empty() {
        return get_fallback_log_path();
    }
    let actual_root = resolve_actual_repo_root(repo_root);
    let target = if !actual_root.as_os_str().is_empty() {
        actual_root
    } else {
        repo_root.to_path_buf()
    };
    if target.as_os_str().is_empty() || crate::config::is_binary_or_target_dir(&target) || crate::config::is_app_own_directory(&target) {
        get_fallback_log_path()
    } else {
        target.join(".cvh").join("logs").join("cvh.log")
    }
}

/// Fallback log path in %LOCALAPPDATA%/com.voimax.cvh/logs/cvh.log
fn get_fallback_log_path() -> PathBuf {
    get_app_config_dir().join("logs").join("cvh.log")
}

/// Performs simple 3-generation log rotation if the file exceeds MAX_LOG_SIZE_BYTES.
fn rotate_log_if_needed(log_path: &Path) {
    if let Ok(metadata) = std::fs::metadata(log_path) {
        if metadata.len() >= MAX_LOG_SIZE_BYTES {
            let p3 = log_path.with_extension("log.3");
            let p2 = log_path.with_extension("log.2");
            let p1 = log_path.with_extension("log.1");

            let _ = remove_file(&p3);
            let _ = rename(&p2, &p3);
            let _ = rename(&p1, &p2);
            let _ = rename(log_path, &p1);
        }
    }
}

/// Appends a log line to the specified path, performing rotation if needed.
fn append_to_file(log_path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = log_path.parent() {
        create_dir_all(parent)?;
    }
    rotate_log_if_needed(log_path);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    file.write_all(content.as_bytes())?;
    file.flush()?;
    Ok(())
}

/// Core function to write an atomic log entry with rotation and fallback.
/// Formats as: `[YYYY-MM-DD HH:mm:ss.SSS] [LEVEL] [TAG] message`
pub fn cvh_log_with_details(
    repo_root: Option<&Path>,
    level: &str,
    tag: &str,
    message: &str,
    details: Option<&str>,
) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    let level_upper = level.trim().to_uppercase();
    let tag_upper = tag.trim().to_uppercase();
    let clean_msg = message.trim();

    let formatted_entry = match details.map(str::trim).filter(|s| !s.is_empty()) {
        Some(d) => format!(
            "[{}] [{}] [{}] {}\n  Details: {}\n",
            timestamp, level_upper, tag_upper, clean_msg, d
        ),
        None => format!(
            "[{}] [{}] [{}] {}\n",
            timestamp, level_upper, tag_upper, clean_msg
        ),
    };

    // Print to stderr for immediate terminal visibility
    eprintln!("[{}] [{}] [{}] {}", timestamp, level_upper, tag_upper, clean_msg);
    if let Some(d) = details.map(str::trim).filter(|s| !s.is_empty()) {
        eprintln!("  Details: {}", d);
    }

    // Thread-safe atomic file append
    let _guard = LOG_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

    let mut success = false;
    if let Some(root) = repo_root {
        let primary_path = get_primary_log_path(root);
        if append_to_file(&primary_path, &formatted_entry).is_ok() {
            success = true;
        }
    }

    if !success {
        let fallback_path = get_fallback_log_path();
        let _ = append_to_file(&fallback_path, &formatted_entry);
    }
}

pub fn cvh_log(repo_root: Option<&Path>, level: &str, tag: &str, message: &str) {
    cvh_log_with_details(repo_root, level, tag, message, None);
}

/// Tauri command to allow frontend to emit logs into cvh.log
#[tauri::command]
pub async fn log_event(
    level: String,
    tag: String,
    message: String,
    details: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let root = state.get_root();
    cvh_log_with_details(Some(&root), &level, &tag, &message, details.as_deref());
    Ok(())
}

#[macro_export]
macro_rules! cvh_log_info {
    ($root:expr, $tag:expr, $($arg:tt)*) => {
        $crate::logger::cvh_log(Some($root), "INFO", $tag, &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! cvh_log_warn {
    ($root:expr, $tag:expr, $($arg:tt)*) => {
        $crate::logger::cvh_log(Some($root), "WARN", $tag, &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! cvh_log_error {
    ($root:expr, $tag:expr, $($arg:tt)*) => {
        $crate::logger::cvh_log(Some($root), "ERROR", $tag, &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! cvh_log_debug {
    ($root:expr, $tag:expr, $($arg:tt)*) => {
        $crate::logger::cvh_log(Some($root), "DEBUG", $tag, &format!($($arg)*))
    };
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cvh_log_format_and_file_write() {
        let temp_dir = std::path::PathBuf::from("target").join(format!("cvh_log_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let _ = std::fs::create_dir_all(&temp_dir);

        cvh_log(Some(&temp_dir), "INFO", "TEST", "Hello logging world!");
        cvh_log_with_details(Some(&temp_dir), "ERROR", "TEST_ERR", "Something failed", Some("Call stack details"));

        let log_file = temp_dir.join(".cvh").join("logs").join("cvh.log");
        assert!(log_file.exists(), "Log file must exist at {:?}", log_file);

        let content = std::fs::read_to_string(&log_file).unwrap();
        assert!(content.contains("[INFO] [TEST] Hello logging world!"));
        assert!(content.contains("[ERROR] [TEST_ERR] Something failed"));
        assert!(content.contains("Details: Call stack details"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}


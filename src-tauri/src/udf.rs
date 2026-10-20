use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static PROBE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Inspects candidate WebView2 User Data Folders (UDF) for writability, stale locks,
/// or active locks held by another process (which causes 0x800700AA ERROR_BUSY).
///
/// If a candidate directory has stale locks from a crashed process, they are safely cleaned up.
/// If a candidate directory is currently locked by another process (e.g., an elevated instance
/// or hung msedgewebview2.exe) or unwritable, it is rejected and an alternative fallback folder
/// is selected.
///
/// Returns the path to the verified healthy and usable UDF directory.
pub fn resolve_and_prepare_webview2_udf() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        // 1. If an explicit environment variable was already provided and is usable, honor it.
        if let Ok(explicit) = std::env::var("WEBVIEW2_USER_DATA_FOLDER") {
            let p = PathBuf::from(explicit);
            if is_udf_usable(&p) {
                clean_unheld_stale_locks(&p);
                return p;
            }
        }

        // 2. Build candidate list in LocalAppData
        let mut candidates = Vec::new();
        if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
            let base = PathBuf::from(&localappdata).join("com.voimax.cvh");
            // Primary default folder
            candidates.push(base.join("webview_data"));
            // Dedicated user fallback
            candidates.push(base.join("webview_data_user"));
            // Alternate secondary fallback
            candidates.push(base.join("webview_data_alt"));
            // Isolated PID fallback
            candidates.push(base.join(format!("webview_data_p{}", std::process::id())));
        }

        // 3. Fallbacks in TEMP in case LocalAppData has restrictive ACLs or is busy
        if let Ok(temp) = std::env::var("TEMP") {
            let temp_base = PathBuf::from(&temp).join("com.voimax.cvh");
            candidates.push(temp_base.join("webview_data"));
            candidates.push(temp_base.join(format!("webview_data_p{}", std::process::id())));
        }

        // 4. Fallback in std::env::temp_dir()
        let t_base = std::env::temp_dir().join("com.voimax.cvh");
        candidates.push(t_base.join("webview_data"));
        candidates.push(t_base.join(format!("webview_data_p{}", std::process::id())));

        // 5. Test candidates in order
        for candidate in &candidates {
            if is_udf_usable(candidate) {
                clean_unheld_stale_locks(candidate);
                // Opportunistically clean unheld old pid-based fallback directories
                if let Some(parent) = candidate.parent() {
                    cleanup_stale_fallback_dirs(parent);
                }
                return candidate.clone();
            }
        }

        // 6. Ultimate fallback: local project folder if all else fails
        let local_fallback = PathBuf::from(".cvh").join("webview_data");
        if is_udf_usable(&local_fallback) {
            clean_unheld_stale_locks(&local_fallback);
            return local_fallback;
        }

        PathBuf::from("webview_data")
    }

    #[cfg(not(target_os = "windows"))]
    {
        PathBuf::from("webview_data")
    }
}

/// Checks whether a candidate UDF directory is writable and not locked by another process.
pub fn is_udf_usable(candidate: &Path) -> bool {
    // 1. Check directory writability
    if std::fs::create_dir_all(candidate).is_err() {
        return false;
    }

    let probe_idx = PROBE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let probe_file = candidate.join(format!(".cvh_probe_{}_{}.tmp", std::process::id(), probe_idx));
    match std::fs::write(&probe_file, b"ok") {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe_file);
        }
        Err(_) => return false,
    }

    // 2. Check EBWebView\lockfile
    let lockfile = candidate.join("EBWebView").join("lockfile");
    if lockfile.exists() && is_file_locked_by_other_process(&lockfile) {
        return false;
    }

    // 3. Check EBWebView\Default\LOCK
    let default_lock = candidate.join("EBWebView").join("Default").join("LOCK");
    if default_lock.exists() && is_file_locked_by_other_process(&default_lock) {
        return false;
    }

    // 4. Check EBWebView\Default\Preferences (locked if browser is active)
    let prefs_lock = candidate.join("EBWebView").join("Default").join("Preferences");
    if prefs_lock.exists() && is_file_locked_by_other_process(&prefs_lock) {
        return false;
    }

    true
}

/// Tests if a specific file is locked by another process on Windows,
/// or if it is unheld and can be safely removed/reused.
pub fn is_file_locked_by_other_process(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }

    // Attempt to open with read-write access without sharing.
    // On Windows, if another process has an active handle, this will fail
    // with ERROR_SHARING_VIOLATION (32) or ERROR_ACCESS_DENIED (5).
    match std::fs::OpenOptions::new().read(true).write(true).open(path) {
        Ok(file) => {
            // Explicitly drop file handle before removing the file
            drop(file);
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name == "LOCK" || name == "lockfile" || name.ends_with(".lock") {
                    let _ = std::fs::remove_file(path);
                }
            }
            false
        }
        Err(_) => true,
    }
}

/// Recursively removes unheld stale lock files (LOCK, lockfile, *.lock).
pub fn clean_unheld_stale_locks(dir: &Path) {
    if !dir.exists() {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                clean_unheld_stale_locks(&path);
            } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name == "LOCK" || name == "lockfile" || name.ends_with(".lock") {
                    // Try to open with write access to verify it's unheld, then delete
                    if let Ok(file) = std::fs::OpenOptions::new().read(true).write(true).open(&path) {
                        drop(file);
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
    }
}

/// Opportunistically cleans up old PID-based fallback directories whose processes have exited.
pub fn cleanup_stale_fallback_dirs(parent: &Path) {
    if !parent.exists() {
        return;
    }
    let current_pid = std::process::id();
    if let Ok(entries) = std::fs::read_dir(parent) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if let Some(pid_str) = name.strip_prefix("webview_data_p") {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        if pid == current_pid {
                            continue;
                        }
                        // Check if lockfile is present and locked
                        let lockfile = path.join("EBWebView").join("lockfile");
                        if !lockfile.exists() || !is_file_locked_by_other_process(&lockfile) {
                            // Directory is unheld! Safely remove it to save disk space
                            let _ = std::fs::remove_dir_all(&path);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(100);

    fn get_test_dir() -> PathBuf {
        let idx = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test_tmp")
            .join(format!("cvh_udf_{}_{}", std::process::id(), idx));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn test_clean_unheld_stale_locks() {
        let temp_dir = get_test_dir();
        let eb_dir = temp_dir.join("EBWebView").join("Default");
        std::fs::create_dir_all(&eb_dir).unwrap();

        let lock1 = temp_dir.join("EBWebView").join("lockfile");
        let lock2 = eb_dir.join("LOCK");
        let normal_file = eb_dir.join("data.txt");

        std::fs::write(&lock1, b"").unwrap();
        std::fs::write(&lock2, b"").unwrap();
        std::fs::write(&normal_file, b"keep me").unwrap();

        clean_unheld_stale_locks(&temp_dir);

        assert!(!lock1.exists(), "lockfile should have been removed");
        assert!(!lock2.exists(), "LOCK should have been removed");
        assert!(normal_file.exists(), "normal file should NOT be removed");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_is_udf_usable_on_fresh_dir() {
        let temp_dir = get_test_dir();
        let usable = is_udf_usable(&temp_dir);
        assert!(usable, "Fresh test dir should be usable");
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_is_udf_unusable_when_locked() {
        let temp_dir = get_test_dir();
        let eb_dir = temp_dir.join("EBWebView");
        std::fs::create_dir_all(&eb_dir).unwrap();
        let lockfile = eb_dir.join("lockfile");
        std::fs::write(&lockfile, b"lock content").unwrap();

        // Open with exclusive write access (no sharing on Windows)
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::fs::OpenOptionsExt;
            // dwShareMode = 0 -> exclusive lock
            let _file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .share_mode(0)
                .open(&lockfile)
                .unwrap();

            assert!(!is_udf_usable(&temp_dir), "Dir with locked file should NOT be usable");
            // _file is dropped here, releasing lock
        }

        // Now that the lock is released, it should be usable and clean the stale lock
        assert!(is_udf_usable(&temp_dir), "Dir should become usable after lock release");
        assert!(!lockfile.exists(), "Stale lockfile should be cleaned");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_resolve_and_prepare_webview2_udf_with_explicit_env() {
        let temp_dir = get_test_dir();
        std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", temp_dir.to_str().unwrap());

        let resolved = resolve_and_prepare_webview2_udf();
        assert_eq!(resolved, temp_dir);
        assert!(is_udf_usable(&resolved));

        std::env::remove_var("WEBVIEW2_USER_DATA_FOLDER");
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

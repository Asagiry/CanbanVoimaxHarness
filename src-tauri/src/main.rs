// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Mitigate msedgewebview2.exe STATUS_BREAKPOINT (0x80000003) crashes:
    // third-party DLL injectors (Camo, AnyDesk, Aurora SDK) trip Chromium's
    // RendererCodeIntegrity guard, and GPU-process faults kill the renderer.
    // The WebView2 loader appends this env var to its own browser arguments.
    #[cfg(target_os = "windows")]
    if std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").is_err() {
        std::env::set_var(
            "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
            "--disable-features=RendererCodeIntegrity",
        );
    }

    // Install file-based panic hook so panics are never lost silently
    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        let msg = format!(
            "CanbanVoimaxHarness Crash Panic:\n{info}\n\nBacktrace:\n{backtrace}\n"
        );
        let _ = std::fs::write("panic.log", &msg);
        if let Ok(temp) = std::env::var("TEMP") {
            let _ = std::fs::write(std::path::Path::new(&temp).join("cvh_panic.log"), &msg);
        }
        eprintln!("{msg}");
    }));

    // WebView2 user-data folder (UDF) resilience & non-admin permission inheritance:
    // If com.voimax.cvh was created/touched during an elevated (admin) run, normal user tokens
    // hit 0x80070005 (Access Denied). Stale lockfiles also trigger 0x800700AA (ERROR_BUSY).
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
            let base = std::path::PathBuf::from(&localappdata).join("com.voimax.cvh");
            let _ = std::fs::create_dir_all(&base);

            // Recursively ensure current user and AppContainer have Full Control
            if let Some(path_str) = base.to_str() {
                let username = std::env::var("USERNAME").unwrap_or_else(|_| "Users".to_string());
                let _ = std::process::Command::new("icacls")
                    .args([
                        path_str,
                        "/grant",
                        &format!("{username}:(OI)(CI)F"),
                        "/grant",
                        "*S-1-15-2-1:(OI)(CI)F",
                        "/T",
                        "/Q",
                        "/C",
                    ])
                    .creation_flags(CREATE_NO_WINDOW)
                    .output();
            }

            // Stale lock recovery: If EBWebView\Default\LOCK or EBWebView\EBWebView.lock exist
            // from a previous ungraceful exit or crash, attempt to remove them so WebView2 doesn't hit 0x800700AA
            let eb_dir = base.join("EBWebView");
            if eb_dir.exists() {
                let lock_paths = [
                    eb_dir.join("EBWebView.lock"),
                    eb_dir.join("Default").join("LOCK"),
                    eb_dir.join("Default").join("Local Storage").join("leveldb").join("LOCK"),
                    eb_dir.join("Default").join("Session Storage").join("LOCK"),
                ];
                for lock in &lock_paths {
                    if lock.exists() {
                        let _ = std::fs::remove_file(lock);
                    }
                }
            }
        }
    }

    canban_voimax_harness_lib::run();
}

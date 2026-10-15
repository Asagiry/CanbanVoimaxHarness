// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Mitigate msedgewebview2.exe crashes:
    // third-party DLL injectors (antivirus, recording tools, proxies) trip Chromium's
    // RendererCodeIntegrity guard. Disabling RendererCodeIntegrity prevents the crash.
    #[cfg(target_os = "windows")]
    {
        // Workaround for Microsoft Edge WebView2 regression in build 155.0.4283.45 (which asserts and crashes in msedge_elf.dll at offset 0x281a5b):
        // If the stable 154 runtime (154.0.4258.62) is installed on the machine, bind directly to it via WEBVIEW2_BROWSER_EXECUTABLE_FOLDER.
        if std::env::var("WEBVIEW2_BROWSER_EXECUTABLE_FOLDER").is_err() {
            let app_dir = std::path::Path::new(r"C:\Program Files (x86)\Microsoft\EdgeWebView\Application");
            if app_dir.exists() {
                let candidate_154 = app_dir.join("154.0.4258.62");
                if candidate_154.exists() {
                    std::env::set_var(
                        "WEBVIEW2_BROWSER_EXECUTABLE_FOLDER",
                        candidate_154.to_string_lossy().as_ref(),
                    );
                }
            }
        }

        // Dedicated, unpolluted WebView2 User Data Folder (prevents legacy 0x800700AA / 0x8000FFFF UDF corruption):
        if std::env::var("WEBVIEW2_USER_DATA_FOLDER").is_err() {
            if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
                let udf = std::path::PathBuf::from(&localappdata)
                    .join("com.voimax.cvh")
                    .join("webview_data");
                let _ = std::fs::create_dir_all(&udf);
                std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", udf.to_string_lossy().as_ref());
            }
        }

        let required_flags = "--disable-features=RendererCodeIntegrity";
        match std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS") {
            Ok(existing) => {
                if !existing.contains("RendererCodeIntegrity") {
                    std::env::set_var(
                        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
                        format!("{existing} {required_flags}"),
                    );
                }
            }
            Err(_) => {
                std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", required_flags);
            }
        }
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

    // WebView2 user-data folder (UDF) resilience:
    // Stale lock recovery: Recursively search and remove all unheld LevelDB and UDF LOCK files.
    // If the app was closed abruptly or crashed, stale locks cause WebView2 0x800700AA (ERROR_BUSY).
    #[cfg(target_os = "windows")]
    {
        if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
            let base = std::path::PathBuf::from(&localappdata).join("com.voimax.cvh");
            let _ = std::fs::create_dir_all(&base);

            fn remove_stale_locks_recursive(dir: &std::path::Path) {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            remove_stale_locks_recursive(&path);
                        } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if name == "LOCK" || name.ends_with(".lock") {
                                let _ = std::fs::remove_file(&path);
                            }
                        }
                    }
                }
            }

            let eb_dir = base.join("EBWebView");
            if eb_dir.exists() {
                remove_stale_locks_recursive(&eb_dir);
            }
            let webview_data = base.join("webview_data");
            if webview_data.exists() {
                remove_stale_locks_recursive(&webview_data);
            }
        }
    }

    canban_voimax_harness_lib::run();
}

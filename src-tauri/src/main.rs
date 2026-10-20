// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Mitigate msedgewebview2.exe crashes:
    // third-party DLL injectors (antivirus, recording tools, proxies) trip Chromium's
    // RendererCodeIntegrity guard. Disabling RendererCodeIntegrity prevents the crash.
    #[cfg(target_os = "windows")]
    {
        // Dedicated, unpolluted, verified available WebView2 User Data Folder.
        // Sanitizes unheld stale locks from previous crashed processes and automatically
        // falls back to alternative isolated UDF directories if the primary folder is in use
        // by an elevated/another process or unwritable, preventing 0x800700AA (ERROR_BUSY) crashes.
        let udf = canban_voimax_harness_lib::udf::resolve_and_prepare_webview2_udf();
        std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", udf.to_string_lossy().as_ref());

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
        let msg = format!("CanbanVoimaxHarness Crash Panic:\n{info}\n\nBacktrace:\n{backtrace}\n");
        let _ = std::fs::write("panic.log", &msg);
        if let Ok(temp) = std::env::var("TEMP") {
            let _ = std::fs::write(std::path::Path::new(&temp).join("cvh_panic.log"), &msg);
        }
        if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
            let _ = std::fs::write(
                std::path::PathBuf::from(localappdata)
                    .join("com.voimax.cvh")
                    .join("panic.log"),
                &msg,
            );
        }
        eprintln!("{msg}");
    }));

    canban_voimax_harness_lib::run();
}

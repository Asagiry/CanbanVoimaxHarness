// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Mitigate msedgewebview2.exe crashes:
    // third-party DLL injectors (antivirus, recording tools, proxies) trip Chromium's
    // RendererCodeIntegrity guard. Disabling RendererCodeIntegrity prevents the crash.
    #[cfg(target_os = "windows")]
    {
        let flags_to_add = [
            "--disable-features=RendererCodeIntegrity",
            "--js-flags=--max-old-space-size=4096",
        ];
        let mut current_flags = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS")
            .unwrap_or_default();

        for flag in flags_to_add {
            if !current_flags.contains(flag) {
                if current_flags.is_empty() {
                    current_flags = flag.to_string();
                } else {
                    current_flags = format!("{current_flags} {flag}");
                }
            }
        }
        std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", current_flags);
    }

    // Install file-based panic hook so panics are written to app/temp dir (never CWD)
    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        let msg = format!("CanbanVoimaxHarness Crash Panic:\n{info}\n\nBacktrace:\n{backtrace}\n");
        let _ = std::fs::write(std::env::temp_dir().join("cvh_panic.log"), &msg);
        if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
            let app_dir = std::path::PathBuf::from(localappdata).join("com.voimax.cvh");
            if std::fs::create_dir_all(&app_dir).is_ok() {
                let _ = std::fs::write(app_dir.join("panic.log"), &msg);
            }
        }
        eprintln!("{msg}");
    }));

    canban_voimax_harness_lib::run();
}

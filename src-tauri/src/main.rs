// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "windows")]
    {
        // Configure WebView2 arguments to prevent 0x8000FFFF (E_UNEXPECTED) crashes on Windows
        if std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").is_err() {
            std::env::set_var(
                "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
                "--no-sandbox --disable-gpu",
            );
        }
    }

    // Install file-based panic hook so panics are never lost silently
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("CanbanVoimaxHarness Crash Panic:\n{info}\n");
        let _ = std::fs::write("panic.log", &msg);
        if let Ok(temp) = std::env::var("TEMP") {
            let _ = std::fs::write(std::path::Path::new(&temp).join("cvh_panic.log"), &msg);
        }
        eprintln!("{msg}");
    }));

    canban_voimax_harness_lib::run();
}

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
            "--disable-features=RendererCodeIntegrity --disable-gpu",
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

    // WebView2 user-data folder (UDF) resilience: the browser process runs in an
    // AppContainer and aborts startup with std::io Error 5 (Access Denied) when
    // its profile folder is missing or lacks an ACL entry for the sandbox SID.
    // Best-effort: ensure the folder exists, then grant ALL APPLICATION PACKAGES
    // (*S-1-15-2-1) full recursive access. All failures are non-fatal.
    #[cfg(target_os = "windows")]
    if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
        let udf = std::path::PathBuf::from(localappdata).join("com.voimax.cvh");
        let _ = std::fs::create_dir_all(&udf);
        let _ = std::process::Command::new("icacls")
            .arg(&udf)
            .args(["/grant", "*S-1-15-2-1:(OI)(CI)F", "/Q"])
            .output();
    }

    canban_voimax_harness_lib::run();
}

fn main() {
    let windows_attrs = tauri_build::WindowsAttributes::new().static_vc_runtime(true);
    let attrs = tauri_build::Attributes::new().windows_attributes(windows_attrs);
    if let Err(e) = tauri_build::try_build(attrs) {
        eprintln!("tauri_build: {e}");
    }
}

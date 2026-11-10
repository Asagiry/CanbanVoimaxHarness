//! Notification Subsystem Unit Tests

use super::*;
use std::path::Path;

#[test]
fn test_escape_xml() {
    let raw = "<hello & 'world' \"test\">";
    assert_eq!(escape_xml(raw), "&lt;hello &amp; &apos;world&apos; &quot;test&quot;&gt;");
}

#[test]
fn test_encode_powershell_script() {
    let script = "Write-Host 'Hello'";
    let encoded = encode_powershell_script(script);
    let decoded = base64::engine::general_purpose::STANDARD.decode(&encoded).expect("Valid base64");
    let u16_chars: Vec<u16> = decoded.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    assert_eq!(String::from_utf16(&u16_chars).expect("Valid UTF-16"), script);
}

#[test]
fn test_build_windows_toast_script_contains_aumid_and_styles() {
    let icon = Path::new("C:\\temp\\icon.png");
    let s = build_windows_toast_script("Task CVH-042", Some("Review & test"), Some(icon), Some("CVH-042"));
    assert!(s.contains("com.voimax.cvh") && s.contains("Windows.SystemToast.BackgroundAccess"));
    assert!(s.contains("Microsoft.Windows.Explorer") && s.contains("{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}"));
    assert!(s.contains("&amp;") && s.contains("<text hint-style=\"title\">Task CVH-042</text>"));
    assert!(s.contains("<text hint-style=\"body\" hint-wrap=\"true\">Review &amp; test</text>"));
    assert!(s.contains("<text placement=\"attribution\">CanbanVoimaxHarness</text>"));
    assert!(s.contains("appLogoOverride") && s.contains("Открыть задачу") && s.contains("open:CVH-042"));
    assert!(s.contains("Start-Sleep -Milliseconds 1500") && s.contains("System.Windows.Forms.NotifyIcon"));
}

#[test]
fn test_build_windows_toast_script_empty_body_and_no_action() {
    let script = build_windows_toast_script("Alert Title", None, None, None);
    assert!(script.contains("<text hint-style=\"title\">Alert Title</text>"));
    assert!(!script.contains("appLogoOverride"));
    assert!(!script.contains("<actions>"));
    assert!(script.contains("System.Windows.Forms.NotifyIcon"));
}

#[test]
fn test_ensure_cached_app_icon() {
    let p = ensure_cached_app_icon().expect("Cached icon path");
    assert!(p.exists());
    assert_eq!(p.file_name().and_then(|n| n.to_str()), Some("app_icon.png"));
}

#[test]
fn test_ensure_cached_app_ico() {
    let p = ensure_cached_app_ico().expect("Cached ico path");
    assert!(p.exists());
    assert_eq!(p.file_name().and_then(|n| n.to_str()), Some("app_icon.ico"));
}

#[test]
fn test_resolve_toast_icon_returns_app_icon() {
    let dev = resolve_toast_icon(Some("dev")).expect("Dev icon path");
    assert!(dev.exists());
    assert_eq!(dev.file_name().and_then(|n| n.to_str()), Some("app_icon.png"));

    let none = resolve_toast_icon(None).expect("Default icon path");
    assert!(none.exists());
    assert_eq!(none.file_name().and_then(|n| n.to_str()), Some("app_icon.png"));
}

#[test]
fn test_build_windows_shortcut_script() {
    let script = build_windows_shortcut_script("C:\\test.lnk", "C:\\test.exe", "C:\\", "C:\\icon.ico");
    assert!(script.contains("CvhShortcutManager"));
    assert!(script.contains("com.voimax.cvh"));
    assert!(script.contains("9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3"));
    assert!(script.contains("WScript.Shell"));
}

#[test]
fn test_ensure_cached_stage_icon() {
    let dev_path = ensure_cached_stage_icon("dev").expect("Cached dev svg");
    assert!(dev_path.exists());
    let dev_svg = std::fs::read_to_string(dev_path).expect("Read SVG");
    assert!(dev_svg.contains("<svg") && dev_svg.contains("#3b82f6"));

    let rev_path = ensure_cached_stage_icon("review").expect("Cached review svg");
    assert!(rev_path.exists());
    let rev_svg = std::fs::read_to_string(rev_path).expect("Read SVG");
    assert!(rev_svg.contains("#a855f7"));
}

use std::path::{Path, PathBuf};
use tauri_plugin_notification::NotificationExt;

const APP_ICON_PNG: &[u8] = include_bytes!("../../icons/128x128.png");

/// Escapes XML special characters for safe inclusion in WinRT Toast XML templates.
pub fn escape_xml(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

/// Encodes a UTF-8 script string into UTF-16LE Base64 format required by PowerShell's `-EncodedCommand`.
pub fn encode_powershell_script(script: &str) -> String {
    use base64::Engine;
    let utf16_bytes: Vec<u8> = script
        .encode_utf16()
        .flat_map(|u| u.to_le_bytes())
        .collect();
    base64::engine::general_purpose::STANDARD.encode(&utf16_bytes)
}

/// Spawns a hidden PowerShell process with `-EncodedCommand` on Windows.
#[cfg(target_os = "windows")]
fn run_encoded_powershell(encoded: &str) -> std::io::Result<std::process::Output> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", encoded])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
}

/// Resolves the dedicated local cache directory for CVH notification assets.
pub fn get_cache_directory() -> Option<PathBuf> {
    let base = std::env::var("LOCALAPPDATA")
        .ok()
        .or_else(|| std::env::var("APPDATA").ok())
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join("com.voimax.cvh");
    let _ = std::fs::create_dir_all(&dir);
    Some(dir)
}

/// Ensures the application icon (128x128.png) is extracted and cached in the user's LocalAppData folder.
pub fn ensure_cached_app_icon() -> Option<PathBuf> {
    let dir = get_cache_directory()?;
    let icon_path = dir.join("app_icon.png");
    let needs_write = match std::fs::metadata(&icon_path) {
        Ok(meta) => meta.len() as usize != APP_ICON_PNG.len(),
        Err(_) => true,
    };
    if needs_write {
        if let Err(e) = std::fs::write(&icon_path, APP_ICON_PNG) {
            eprintln!("[Notification] Failed to cache app_icon.png: {}", e);
            return None;
        }
    }
    Some(icon_path)
}

/// Returns the theme color and SVG inner paths for canonical board column icons.
fn get_stage_svg_content(icon_name: &str) -> Option<(&'static str, &'static str)> {
    match icon_name.to_ascii_lowercase().as_str() {
        "dev" | "code" | "code2" => Some(("#3b82f6", r#"<path d="m18 16 4-4-4-4"/><path d="m6 8-4 4 4 4"/><path d="m14.5 4-5 16"/>"#)),
        "review" | "shieldcheck" | "eye" => Some(("#a855f7", r#"<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/><path d="m9 12 2 2 4-4"/>"#)),
        "ready" | "checkcircle2" | "check" => Some(("#10b981", r#"<circle cx="12" cy="12" r="10"/><path d="m9 12 2 2 4-4"/>"#)),
        "question" | "helpcircle" => Some(("#38bdf8", r#"<circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><path d="M12 17h.01"/>"#)),
        "reject" | "rotateccw" => Some(("#f43f5e", r#"<path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/>"#)),
        "plan" | "compass" => Some(("#0284c7", r#"<circle cx="12" cy="12" r="10"/><polygon points="16.24 7.76 14.12 14.12 7.76 16.24 9.88 9.88 16.24 7.76"/>"#)),
        "backlog" | "inbox" => Some(("#a1a1aa", r#"<polyline points="22 12 16 12 14 15 10 15 8 12 2 12"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/>"#)),
        "retro" | "history" => Some(("#eab308", r#"<path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M12 7v5l4 2"/>"#)),
        _ => None,
    }
}

/// Ensures the requested board column SVG icon is cached to disk in LocalAppData.
pub fn ensure_cached_stage_icon(icon_name: &str) -> Option<PathBuf> {
    let (color, glyph) = get_stage_svg_content(icon_name)?;
    let dir = get_cache_directory()?.join("icons");
    let _ = std::fs::create_dir_all(&dir);
    let icon_path = dir.join(format!("{}.svg", icon_name.to_ascii_lowercase()));
    if !icon_path.exists() {
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64"><circle cx="32" cy="32" r="30" fill="#171717" stroke="{color}" stroke-width="2"/><g transform="translate(16, 16) scale(1.33)" fill="none" stroke="{color}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{glyph}</g></svg>"##
        );
        let _ = std::fs::write(&icon_path, svg);
    }
    Some(icon_path)
}

/// Resolves the optimal icon path: stage SVG if matched, otherwise cached app icon PNG.
pub fn resolve_toast_icon(icon_name: Option<&str>) -> Option<PathBuf> {
    if let Some(name) = icon_name {
        if let Some(path) = ensure_cached_stage_icon(name) {
            return Some(path);
        }
    }
    ensure_cached_app_icon()
}

/// Builds PowerShell WinRT Toast script with adaptive elements:
/// circular column SVG / app icon, wrapped typography, attribution, and action button.
pub fn build_windows_toast_script(
    title: &str,
    body: Option<&str>,
    icon_path: Option<&Path>,
    task_id: Option<&str>,
) -> String {
    let xml_title = escape_xml(title).replace("'@", "' @");
    let xml_body = escape_xml(body.unwrap_or_default()).replace("'@", "' @");
    let raw_title = title.replace("'@", "' @");
    let unescaped_body = body.unwrap_or_default().replace("'@", "' @");
    let balloon_body = if unescaped_body.trim().is_empty() { " " } else { &unescaped_body };

    let image_el = match icon_path {
        Some(p) => {
            let p_esc = escape_xml(&p.to_string_lossy()).replace("'@", "' @");
            format!(r#"      <image placement="appLogoOverride" src="{}" hint-crop="circle" />"#, p_esc)
        }
        None => String::new(),
    };

    let actions_el = match task_id.filter(|t| !t.trim().is_empty()) {
        Some(tid) => {
            let tid_esc = escape_xml(tid).replace("'@", "' @");
            format!(
                r#"  <actions>
    <action content="Открыть задачу" arguments="open:{}" activationType="foreground"/>
  </actions>"#,
                tid_esc
            )
        }
        None => String::new(),
    };

    format!(
r#"$delivered = $false
try {{
    [Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
    [Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null
    $xml = New-Object Windows.Data.Xml.Dom.XmlDocument
    $template = @'
<toast>
  <visual>
    <binding template="ToastGeneric">
{0}
      <text hint-style="title">{1}</text>
      <text hint-style="body" hint-wrap="true">{2}</text>
      <text placement="attribution">CanbanVoimaxHarness</text>
    </binding>
  </visual>
{3}
</toast>
'@
    $xml.LoadXml($template)
    $toast = New-Object Windows.UI.Notifications.ToastNotification $xml
    $aumids = @("com.voimax.cvh", "Windows.SystemToast.BackgroundAccess", "Microsoft.Windows.Explorer", "{{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}}\WindowsPowerShell\v1.0\powershell.exe")
    foreach ($appId in $aumids) {{
        try {{
            [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier($appId).Show($toast)
            $delivered = $true
            break
        }} catch {{}}
    }}
}} catch {{}}

if ($delivered) {{
    Start-Sleep -Milliseconds 1500
}} else {{
    try {{
        Add-Type -AssemblyName System.Windows.Forms
        Add-Type -AssemblyName System.Drawing
        $balloon = New-Object System.Windows.Forms.NotifyIcon
        $balloon.Icon = [System.Drawing.SystemIcons]::Information
        $balloon.BalloonTipTitle = @'
{4}
'@
        $balloon.BalloonTipText = @'
{5}
'@
        $balloon.Visible = $true
        $balloon.ShowBalloonTip(3000)
        Start-Sleep -Seconds 2
        $balloon.Dispose()
    }} catch {{}}
}}"#,
        image_el, xml_title, xml_body, actions_el, raw_title, balloon_body
    )
}

/// Tier 2 Windows Fallback: Displays toast notification via headless PowerShell process.
#[cfg(target_os = "windows")]
pub fn show_windows_toast_fallback(
    title: &str,
    body: Option<&str>,
    icon_path: Option<&Path>,
    task_id: Option<&str>,
) -> Result<(), String> {
    let script = build_windows_toast_script(title, body, icon_path, task_id);
    let encoded = encode_powershell_script(&script);
    let output = run_encoded_powershell(&encoded)
        .map_err(|e| format!("Failed to spawn PowerShell toast process: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("PowerShell toast execution error: {}", stderr));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn show_windows_toast_fallback(
    _title: &str,
    _body: Option<&str>,
    _icon_path: Option<&Path>,
    _task_id: Option<&str>,
) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn set_registry_val(key: &str, name: &str, reg_type: &str, val: &str) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let _ = std::process::Command::new("reg")
        .args(["add", key, "/v", name, "/t", reg_type, "/d", val, "/f"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
}

/// Registers AppUserModelID (`com.voimax.cvh`) in the Windows Current User Registry
/// with cached icon URI so WinRT ToastNotificationManager can authorize desktop toast notifications.
#[cfg(target_os = "windows")]
pub fn register_windows_aumid() {
    let icon_uri = ensure_cached_app_icon()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default());
    let key = r"HKCU\Software\Classes\AppUserModelId\com.voimax.cvh";
    set_registry_val(key, "DisplayName", "REG_SZ", "CanbanVoimaxHarness");
    set_registry_val(key, "IconUri", "REG_SZ", &icon_uri);
    set_registry_val(key, "ShowInSettings", "REG_DWORD", "1");
}

#[cfg(not(target_os = "windows"))]
pub fn register_windows_aumid() {}

/// Ensures a Start Menu shortcut exists on Windows, assigning the application icon location.
#[cfg(target_os = "windows")]
pub fn ensure_windows_start_menu_shortcut() {
    register_windows_aumid();
    let _ = std::thread::spawn(|| {
        let app_data = match std::env::var("APPDATA") {
            Ok(v) => v,
            Err(_) => return,
        };
        let shortcut_path = PathBuf::from(app_data)
            .join(r"Microsoft\Windows\Start Menu\Programs\CanbanVoimaxHarness.lnk");
        if shortcut_path.exists() {
            return;
        }

        let exe_path = match std::env::current_exe() {
            Ok(p) => p,
            Err(_) => return,
        };

        let exe_str = exe_path.to_string_lossy().to_string();
        let shortcut_str = shortcut_path.to_string_lossy().to_string();
        let parent_str = exe_path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
        let icon_str = ensure_cached_app_icon()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| exe_str.clone());

        let ps_script = format!(
            "$ws = New-Object -ComObject WScript.Shell; \
             $s = $ws.CreateShortcut('{}'); \
             $s.TargetPath = '{}'; \
             $s.WorkingDirectory = '{}'; \
             $s.IconLocation = '{}'; \
             $s.Save();",
            shortcut_str.replace('\'', "''"),
            exe_str.replace('\'', "''"),
            parent_str.replace('\'', "''"),
            icon_str.replace('\'', "''"),
        );

        let encoded = encode_powershell_script(&ps_script);
        let _ = run_encoded_powershell(&encoded);
    });
}

#[cfg(not(target_os = "windows"))]
pub fn ensure_windows_start_menu_shortcut() {}

/// Dispatches desktop notification: Tier 1 native plugin, Tier 2 PowerShell WinRT fallback.
pub fn send_system_notification_internal(
    app: &tauri::AppHandle,
    title: &str,
    body: Option<&str>,
    task_id: Option<&str>,
    icon: Option<&str>,
) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    register_windows_aumid();

    let icon_path = resolve_toast_icon(icon);

    let mut builder = app.notification().builder().title(title);
    if let Some(b) = body {
        builder = builder.body(b);
    }
    if let Some(ref p) = icon_path {
        if let Some(p_str) = p.to_str() {
            builder = builder.icon(p_str);
        }
    }

    let native_res = builder.show();
    match native_res {
        Ok(()) => {
            println!("[Notification] Native sent (task_id: {:?}, title: {:?})", task_id, title);
            Ok(())
        }
        Err(err) => {
            println!("[Notification] Native failed: {:?}. Invoking fallback...", err);
            #[cfg(target_os = "windows")]
            {
                show_windows_toast_fallback(title, body, icon_path.as_deref(), task_id)
            }
            #[cfg(not(target_os = "windows"))]
            {
                Err(format!("Notification delivery failed: {}", err))
            }
        }
    }
}

/// Tauri IPC command to display desktop notification with task jumping action and column SVG badge.
#[tauri::command]
pub async fn show_system_notification(
    title: String,
    body: Option<String>,
    task_id: Option<String>,
    icon: Option<String>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        send_system_notification_internal(
            &app,
            &title,
            body.as_deref(),
            task_id.as_deref(),
            icon.as_deref(),
        )
    })
    .await
    .map_err(|e| format!("Notification task execution failed: {}", e))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_xml() {
        let raw = "<hello & 'world' \"test\">";
        assert_eq!(escape_xml(raw), "&lt;hello &amp; &apos;world&apos; &quot;test&quot;&gt;");
    }

    #[test]
    fn test_encode_powershell_script() {
        let script = "Write-Host 'Hello'";
        let encoded = encode_powershell_script(script);
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD.decode(&encoded).expect("Valid base64");
        let u16_chars: Vec<u16> = decoded
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        assert_eq!(String::from_utf16(&u16_chars).expect("Valid UTF-16"), script);
    }

    #[test]
    fn test_build_windows_toast_script_contains_aumid_and_styles() {
        let icon = Path::new("C:\\temp\\icon.png");
        let script = build_windows_toast_script("Task CVH-042", Some("Review & test"), Some(icon), Some("CVH-042"));
        assert!(script.contains("com.voimax.cvh"));
        assert!(script.contains("Windows.SystemToast.BackgroundAccess"));
        assert!(script.contains("Microsoft.Windows.Explorer"));
        assert!(script.contains("{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}"));
        assert!(script.contains("&amp;"));
        assert!(script.contains("<text hint-style=\"title\">Task CVH-042</text>"));
        assert!(script.contains("<text hint-style=\"body\" hint-wrap=\"true\">Review &amp; test</text>"));
        assert!(script.contains("<text placement=\"attribution\">CanbanVoimaxHarness</text>"));
        assert!(script.contains("appLogoOverride"));
        assert!(script.contains("Открыть задачу"));
        assert!(script.contains("open:CVH-042"));
        assert!(script.contains("Start-Sleep -Milliseconds 1500"));
        assert!(script.contains("System.Windows.Forms.NotifyIcon"));
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
    fn test_ensure_cached_stage_icon() {
        let dev_path = ensure_cached_stage_icon("dev").expect("Cached dev svg");
        assert!(dev_path.exists());
        let dev_svg = std::fs::read_to_string(dev_path).expect("Read SVG");
        assert!(dev_svg.contains("<svg"));
        assert!(dev_svg.contains("#3b82f6"));

        let rev_path = ensure_cached_stage_icon("review").expect("Cached review svg");
        assert!(rev_path.exists());
        let rev_svg = std::fs::read_to_string(rev_path).expect("Read SVG");
        assert!(rev_svg.contains("#a855f7"));
    }
}

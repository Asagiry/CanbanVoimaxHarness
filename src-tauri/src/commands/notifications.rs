use std::path::{Path, PathBuf};
use base64::Engine;
use tauri_plugin_notification::NotificationExt;

const APP_ICON_PNG: &[u8] = include_bytes!("../../icons/128x128@2x.png");
const APP_ICON_ICO: &[u8] = include_bytes!("../../icons/icon.ico");

/// Escapes XML special characters for safe inclusion in WinRT Toast XML templates.
pub fn escape_xml(input: &str) -> String {
    input.chars().fold(String::with_capacity(input.len()), |mut acc, c| {
        match c {
            '&' => acc.push_str("&amp;"),
            '<' => acc.push_str("&lt;"),
            '>' => acc.push_str("&gt;"),
            '"' => acc.push_str("&quot;"),
            '\'' => acc.push_str("&apos;"),
            _ => acc.push(c),
        }
        acc
    })
}

/// Encodes a UTF-8 script string into UTF-16LE Base64 format required by PowerShell's `-EncodedCommand`.
pub fn encode_powershell_script(script: &str) -> String {
    let utf16: Vec<u8> = script.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    base64::engine::general_purpose::STANDARD.encode(&utf16)
}

/// Spawns a hidden PowerShell process with `-EncodedCommand` on Windows.
#[cfg(target_os = "windows")]
fn run_encoded_powershell(encoded: &str) -> std::io::Result<std::process::Output> {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", encoded])
        .creation_flags(0x08000000)
        .output()
}

/// Resolves the dedicated local cache directory for CVH notification assets.
pub fn get_cache_directory() -> Option<PathBuf> {
    #[cfg(test)]
    {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join("notification_cache");
        let _ = std::fs::create_dir_all(&dir);
        Some(dir)
    }
    #[cfg(not(test))]
    {
        let base = std::env::var("LOCALAPPDATA").or_else(|_| std::env::var("APPDATA"))
            .map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
        let dir = base.join("com.voimax.cvh");
        let _ = std::fs::create_dir_all(&dir);
        Some(dir)
    }
}

fn ensure_cached_asset(name: &str, bytes: &[u8]) -> Option<PathBuf> {
    let dir = get_cache_directory()?;
    let icon_path = dir.join(name);
    let needs_write = match std::fs::metadata(&icon_path) {
        Ok(meta) => meta.len() as usize != bytes.len(),
        Err(_) => true,
    };
    if needs_write {
        if let Err(e) = std::fs::write(&icon_path, bytes) {
            eprintln!("[Notification] Failed to cache {} at {:?}: {}", name, icon_path, e);
            // Fallback to temp_dir if LocalAppData is not writable (e.g. sandbox/permissions)
            let fallback_dir = std::env::temp_dir().join("com.voimax.cvh");
            let _ = std::fs::create_dir_all(&fallback_dir);
            let fallback_path = fallback_dir.join(name);
            if let Err(e2) = std::fs::write(&fallback_path, bytes) {
                eprintln!("[Notification] Fallback cache failed for {} at {:?}: {}", name, fallback_path, e2);
                return None;
            }
            return Some(fallback_path);
        }
    }
    Some(icon_path)
}

/// Ensures the application icon (128x128@2x.png) is extracted and cached in the user's LocalAppData folder.
pub fn ensure_cached_app_icon() -> Option<PathBuf> {
    ensure_cached_asset("app_icon.png", APP_ICON_PNG)
}

/// Ensures the application Windows icon (icon.ico) is extracted and cached in the user's LocalAppData folder.
pub fn ensure_cached_app_ico() -> Option<PathBuf> {
    ensure_cached_asset("app_icon.ico", APP_ICON_ICO)
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

/// Resolves the optimal icon path: returns cached app icon PNG (raster format supported by WinRT Toast engine).
pub fn resolve_toast_icon(_icon_name: Option<&str>) -> Option<PathBuf> {
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
        Some(p) => format!(
            r#"      <image placement="appLogoOverride" src="{}" hint-crop="circle" />"#,
            escape_xml(&p.to_string_lossy()).replace("'@", "' @")
        ),
        None => String::new(),
    };

    let actions_el = match task_id.filter(|t| !t.trim().is_empty()) {
        Some(tid) => format!(
            r#"  <actions>
    <action content="Открыть задачу" arguments="open:{}" activationType="foreground"/>
  </actions>"#,
            escape_xml(tid).replace("'@", "' @")
        ),
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
        Add-Type -AssemblyName System.Windows.Forms, System.Drawing
        $balloon = New-Object System.Windows.Forms.NotifyIcon
        $balloon.Icon = [System.Drawing.SystemIcons]::Information
        $balloon.BalloonTipTitle = @'
{4}
'@
        $balloon.BalloonTipText = @'
{5}
'@
        $balloon.Visible = $true; $balloon.ShowBalloonTip(3000)
        Start-Sleep -Seconds 2; $balloon.Dispose()
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
        let stderr = crate::agent::decode_output_bytes(&output.stderr);
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
/// with cached icon URI and notification settings so WinRT ToastNotificationManager authorizes desktop toasts.
#[cfg(target_os = "windows")]
pub fn register_windows_aumid() {
    let icon_uri = ensure_cached_app_ico()
        .or_else(ensure_cached_app_icon)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default());
    let key = r"HKCU\Software\Classes\AppUserModelId\com.voimax.cvh";
    set_registry_val(key, "DisplayName", "REG_SZ", "CanbanVoimaxHarness");
    set_registry_val(key, "IconUri", "REG_SZ", &icon_uri);
    set_registry_val(key, "ShowInSettings", "REG_DWORD", "1");

    let notif_key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings\com.voimax.cvh";
    set_registry_val(notif_key, "ShowInSettings", "REG_DWORD", "1");
    set_registry_val(notif_key, "Enabled", "REG_DWORD", "1");
}

#[cfg(not(target_os = "windows"))]
pub fn register_windows_aumid() {}

/// Builds PowerShell script to create or update Start Menu shortcut with System.AppUserModel.ID property.
pub fn build_windows_shortcut_script(
    shortcut_path: &str,
    target_path: &str,
    work_dir: &str,
    icon_path: &str,
) -> String {
    let sc = shortcut_path.replace('\'', "''");
    let tg = target_path.replace('\'', "''");
    let wd = work_dir.replace('\'', "''");
    let ic = icon_path.replace('\'', "''");

    format!(
r#"$sc = '{0}'; $tg = '{1}'; $wd = '{2}'; $ic = '{3}'
try {{
    Add-Type -TypeDefinition @'
using System; using System.Runtime.InteropServices; using System.Runtime.InteropServices.ComTypes;
[ComImport, Guid("00021401-0000-0000-C000-000000000046")] class ShellLink {{}}
[ComImport, InterfaceType(ComInterfaceType.InterfaceIsIUnknown), Guid("000214F9-0000-0000-C000-000000000046")]
interface IShellLinkW {{
    void GetPath([Out, MarshalAs(UnmanagedType.LPWStr)] System.Text.StringBuilder f, int c, IntPtr p, uint fl);
    void GetIDList(out IntPtr p); void SetIDList(IntPtr p);
    void GetDescription([Out, MarshalAs(UnmanagedType.LPWStr)] System.Text.StringBuilder d, int c); void SetDescription([MarshalAs(UnmanagedType.LPWStr)] string d);
    void GetWorkingDirectory([Out, MarshalAs(UnmanagedType.LPWStr)] System.Text.StringBuilder d, int c); void SetWorkingDirectory([MarshalAs(UnmanagedType.LPWStr)] string d);
    void GetArguments([Out, MarshalAs(UnmanagedType.LPWStr)] System.Text.StringBuilder a, int c); void SetArguments([MarshalAs(UnmanagedType.LPWStr)] string a);
    void GetHotkey(out short k); void SetHotkey(short k); void GetShowCmd(out int s); void SetShowCmd(int s);
    void GetIconLocation([Out, MarshalAs(UnmanagedType.LPWStr)] System.Text.StringBuilder p, int c, out int i); void SetIconLocation([MarshalAs(UnmanagedType.LPWStr)] string p, int i);
    void SetRelativePath([MarshalAs(UnmanagedType.LPWStr)] string p, uint r); void Resolve(IntPtr h, uint f); void SetPath([MarshalAs(UnmanagedType.LPWStr)] string p);
}}
[ComImport, InterfaceType(ComInterfaceType.InterfaceIsIUnknown), Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99")]
interface IPropertyStore {{
    void GetCount(out uint c); void GetAt(uint i, IntPtr k);
    void GetValue(ref PROPERTYKEY k, [In, Out] PROPVARIANT pv);
    void SetValue(ref PROPERTYKEY k, ref PROPVARIANT pv); void Commit();
}}
[StructLayout(LayoutKind.Sequential, Pack = 4)] struct PROPERTYKEY {{ public Guid fmtid; public uint pid; }}
[StructLayout(LayoutKind.Explicit)] struct PROPVARIANT {{ [FieldOffset(0)] public ushort vt; [FieldOffset(8)] public IntPtr pwszVal; }}
public class CvhShortcutManager {{
    public static void CreateShortcut(string p, string t, string w, string ic, string a) {{
        var l = (IShellLinkW)new ShellLink(); l.SetPath(t); l.SetWorkingDirectory(w);
        if (!string.IsNullOrEmpty(ic)) l.SetIconLocation(ic, 0);
        var s = (IPropertyStore)l;
        var k = new PROPERTYKEY {{ fmtid = new Guid("9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3"), pid = 5 }};
        var v = new PROPVARIANT {{ vt = 31, pwszVal = Marshal.StringToCoTaskMemUni(a) }};
        try {{ s.SetValue(ref k, ref v); s.Commit(); }} finally {{ Marshal.FreeCoTaskMem(v.pwszVal); }}
        ((IPersistFile)l).Save(p, true);
    }}
}}
'@
    [CvhShortcutManager]::CreateShortcut($sc, $tg, $wd, $ic, 'com.voimax.cvh')
}} catch {{
    $ws = New-Object -ComObject WScript.Shell
    $s = $ws.CreateShortcut($sc)
    $s.TargetPath = $tg; $s.WorkingDirectory = $wd; $s.IconLocation = $ic
    $s.Save()
}}"#,
        sc, tg, wd, ic
    )
}

/// Ensures a Start Menu shortcut exists on Windows, assigning the application icon location and AUMID.
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

        let exe_path = match std::env::current_exe() {
            Ok(p) => p,
            Err(_) => return,
        };

        let exe_str = exe_path.to_string_lossy().to_string();
        let shortcut_str = shortcut_path.to_string_lossy().to_string();
        let parent_str = exe_path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
        let icon_str = ensure_cached_app_ico()
            .or_else(ensure_cached_app_icon)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| exe_str.clone());

        let ps_script = build_windows_shortcut_script(&shortcut_str, &exe_str, &parent_str, &icon_str);
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
}

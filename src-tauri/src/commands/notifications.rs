use tauri_plugin_notification::NotificationExt;

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

/// Builds PowerShell WinRT Toast script with cascade AUMID strategy and NotifyIcon Tier 3 fallback:
/// Tier 2.1: `com.voimax.cvh` (primary AUMID)
/// Tier 2.2: `Windows.SystemToast.BackgroundAccess` (system background toast AUMID)
/// Tier 2.3: `Microsoft.Windows.Explorer` (explorer toast AUMID)
/// Tier 2.4: Built-in PowerShell AUMID
/// Tier 3: System.Windows.Forms.NotifyIcon BalloonTip fallback
pub fn build_windows_toast_script(title: &str, body: Option<&str>) -> String {
    let xml_title = escape_xml(title).replace("'@", "' @");
    let xml_body = escape_xml(body.unwrap_or_default()).replace("'@", "' @");
    let raw_title = title.replace("'@", "' @");
    let unescaped_body = body.unwrap_or_default().replace("'@", "' @");
    let balloon_body = if unescaped_body.trim().is_empty() {
        " "
    } else {
        &unescaped_body
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
      <text>{0}</text>
      <text>{1}</text>
    </binding>
  </visual>
</toast>
'@
    $xml.LoadXml($template)
    $toast = New-Object Windows.UI.Notifications.ToastNotification $xml

    $aumids = @(
        "com.voimax.cvh",
        "Windows.SystemToast.BackgroundAccess",
        "Microsoft.Windows.Explorer",
        "{{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}}\WindowsPowerShell\v1.0\powershell.exe"
    )

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
{2}
'@
        $balloon.BalloonTipText = @'
{3}
'@
        $balloon.Visible = $true
        $balloon.ShowBalloonTip(3000)
        Start-Sleep -Seconds 2
        $balloon.Dispose()
    }} catch {{}}
}}"#,
        xml_title, xml_body, raw_title, balloon_body
    )
}

/// Tier 2 Windows Fallback: Displays toast notification via headless PowerShell process.
#[cfg(target_os = "windows")]
pub fn show_windows_toast_fallback(title: &str, body: Option<&str>) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    let script = build_windows_toast_script(title, body);
    let encoded = encode_powershell_script(&script);

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-EncodedCommand",
            &encoded,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to spawn PowerShell toast process: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("PowerShell toast execution error: {}", stderr));
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn show_windows_toast_fallback(_title: &str, _body: Option<&str>) -> Result<(), String> {
    Ok(())
}

/// Registers AppUserModelID (`com.voimax.cvh`) in the Windows Current User Registry
/// so that WinRT ToastNotificationManager can locate and authorize desktop toast notifications.
#[cfg(target_os = "windows")]
pub fn register_windows_aumid() {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let exe_path = match std::env::current_exe() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => return,
    };

    let key = r"HKCU\Software\Classes\AppUserModelId\com.voimax.cvh";

    let _ = std::process::Command::new("reg")
        .args([
            "add",
            key,
            "/v",
            "DisplayName",
            "/t",
            "REG_SZ",
            "/d",
            "CanbanVoimaxHarness",
            "/f",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    let _ = std::process::Command::new("reg")
        .args([
            "add",
            key,
            "/v",
            "IconUri",
            "/t",
            "REG_SZ",
            "/d",
            &exe_path,
            "/f",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    let _ = std::process::Command::new("reg")
        .args([
            "add",
            key,
            "/v",
            "ShowInSettings",
            "/t",
            "REG_DWORD",
            "/d",
            "1",
            "/f",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
}

#[cfg(not(target_os = "windows"))]
pub fn register_windows_aumid() {
    // No-op on non-Windows platforms
}

/// Ensures a Start Menu shortcut exists for the application on Windows,
/// allowing WinRT ToastNotificationManager to resolve the AppUserModelId (`com.voimax.cvh`).
#[cfg(target_os = "windows")]
pub fn ensure_windows_start_menu_shortcut() {
    register_windows_aumid();
    let _ = std::thread::spawn(|| {
        let app_data = match std::env::var("APPDATA") {
            Ok(v) => v,
            Err(_) => return,
        };
        let shortcut_path = std::path::PathBuf::from(app_data)
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
        let parent_str = exe_path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let ps_script = format!(
            "$ws = New-Object -ComObject WScript.Shell; \
             $s = $ws.CreateShortcut('{}'); \
             $s.TargetPath = '{}'; \
             $s.WorkingDirectory = '{}'; \
             $s.Save();",
            shortcut_str.replace('\'', "''"),
            exe_str.replace('\'', "''"),
            parent_str.replace('\'', "''"),
        );

        let encoded = encode_powershell_script(&ps_script);
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let _ = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-EncodedCommand",
                &encoded,
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    });
}

#[cfg(not(target_os = "windows"))]
pub fn ensure_windows_start_menu_shortcut() {
    // No-op on non-Windows platforms
}

/// Dispatches a desktop notification with multi-tier fallback:
/// Tier 1: Native Tauri notification plugin (`NotificationExt`).
/// Tier 2: PowerShell WinRT fallback toast on Windows.
pub fn send_system_notification_internal(
    app: &tauri::AppHandle,
    title: &str,
    body: Option<&str>,
    task_id: Option<&str>,
) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    register_windows_aumid();

    let mut builder = app.notification().builder().title(title);
    if let Some(b) = body {
        builder = builder.body(b);
    }

    let native_res = builder.show();
    match native_res {
        Ok(()) => {
            println!(
                "[Notification] Native notification sent (task_id: {:?}, title: {:?})",
                task_id, title
            );
            Ok(())
        }
        Err(err) => {
            println!(
                "[Notification] Native notification failed: {:?}. Invoking fallback...",
                err
            );
            #[cfg(target_os = "windows")]
            {
                show_windows_toast_fallback(title, body)
            }
            #[cfg(not(target_os = "windows"))]
            {
                Err(format!("Notification delivery failed: {}", err))
            }
        }
    }
}

/// Tauri IPC command to display a desktop notification for background task transitions and events.
#[tauri::command]
pub async fn show_system_notification(
    title: String,
    body: Option<String>,
    task_id: Option<String>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        send_system_notification_internal(&app, &title, body.as_deref(), task_id.as_deref())
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
        let escaped = escape_xml(raw);
        assert_eq!(escaped, "&lt;hello &amp; &apos;world&apos; &quot;test&quot;&gt;");
    }

    #[test]
    fn test_encode_powershell_script() {
        let script = "Write-Host 'Hello'";
        let encoded = encode_powershell_script(script);
        // UTF-16LE of "Write-Host 'Hello'"
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&encoded)
            .expect("Valid base64");
        let u16_chars: Vec<u16> = decoded
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        let decoded_str = String::from_utf16(&u16_chars).expect("Valid UTF-16");
        assert_eq!(decoded_str, script);
    }

    #[test]
    fn test_build_windows_toast_script_contains_aumid() {
        let script = build_windows_toast_script("Task CVH-042", Some("Review & test"));
        assert!(script.contains("com.voimax.cvh"));
        assert!(script.contains("Windows.SystemToast.BackgroundAccess"));
        assert!(script.contains("Microsoft.Windows.Explorer"));
        assert!(script.contains("{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}"));
        assert!(script.contains("&amp;"));
        assert!(script.contains("<text>Task CVH-042</text>"));
        assert!(script.contains("Start-Sleep -Milliseconds 1500"));
        assert!(script.contains("System.Windows.Forms.NotifyIcon"));
    }

    #[test]
    fn test_build_windows_toast_script_empty_body() {
        let script = build_windows_toast_script("Alert Title", None);
        assert!(script.contains("<text>Alert Title</text>"));
        assert!(script.contains("System.Windows.Forms.NotifyIcon"));
    }
}

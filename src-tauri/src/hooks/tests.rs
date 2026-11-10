use super::*;
use std::path::PathBuf;
use std::time::Duration;

#[test]
fn test_quote_windows_arg_cases() {
    assert_eq!(quote_windows_arg(""), r#""""#);
    assert_eq!(quote_windows_arg("cargo"), "cargo");
    assert_eq!(quote_windows_arg("hello world"), r#""hello world""#);
    assert_eq!(quote_windows_arg("my_func(arg)"), r#""my_func(arg)""#);
    assert_eq!(quote_windows_arg("путь (тест)"), r#""путь (тест)""#);
    assert_eq!(quote_windows_arg("a&b"), r#""a&b""#);
    assert_eq!(quote_windows_arg("a^b"), r#""a^b""#);
    assert_eq!(quote_windows_arg(r#""already_quoted""#), r#""already_quoted""#);
}

#[test]
fn test_quote_windows_path() {
    let path = PathBuf::from(r#"C:\Program Files (x86)\My App\hook.bat"#);
    assert_eq!(
        quote_windows_path(&path),
        r#""C:\Program Files (x86)\My App\hook.bat""#
    );

    let cyrillic_path = PathBuf::from(r#"C:\Проекты (Рабочие)\тест.cmd"#);
    assert_eq!(
        quote_windows_path(&cyrillic_path),
        r#""C:\Проекты (Рабочие)\тест.cmd""#
    );
}

#[test]
fn test_format_windows_cd_command() {
    let dir = PathBuf::from(r#"D:\Workspace (Beta)\repo"#);
    let formatted = format_windows_cd_command(&dir, "cargo test");
    assert_eq!(
        formatted,
        r#"cd /d "D:\Workspace (Beta)\repo" && cargo test"#
    );
}

#[test]
fn test_build_windows_cmd_args() {
    let args = build_windows_cmd_args("cargo check --tests");
    assert_eq!(args.len(), 4);
    assert_eq!(args[0], "/D");
    assert_eq!(args[1], "/S");
    assert_eq!(args[2], "/C");
    assert_eq!(args[3], "chcp 65001 >nul && cargo check --tests");
}

#[tokio::test]
async fn test_kill_process_tree_zero_pid_is_noop() {
    // PID 0 must immediately return without killing system idle process or spawning taskkill
    kill_process_tree(0).await;
}

#[test]
fn test_resolve_powershell_executable() {
    let exe = resolve_powershell_executable();
    #[cfg(target_os = "windows")]
    {
        assert!(exe == "pwsh" || exe == "powershell.exe");
    }
    #[cfg(not(target_os = "windows"))]
    {
        assert_eq!(exe, "pwsh");
    }
}

#[tokio::test]
#[cfg(windows)]
async fn test_execute_sensor_command_windows_quotes() {
    let res = execute_sensor_command(r#"python -c "print('sensor_ok')""#, ".", Some(Duration::from_secs(10)))
        .await
        .expect("sensor command execution");
    assert!(res.success, "sensor command should succeed, stderr: {}", res.stderr);
    assert!(res.stdout.contains("sensor_ok"), "got stdout: {}", res.stdout);
}

#[tokio::test]
#[cfg(windows)]
async fn test_execute_sensor_command_bat_forward_slashes() {
    let tmp = std::path::PathBuf::from("target").join(format!("cvh_sensor_test_{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(tmp.join("sub")).await.expect("test dir");
    let bat_path = tmp.join("sub").join("sensor.bat");
    tokio::fs::write(&bat_path, "@echo off\r\necho sensor_batch_ok\r\nexit /b 0\r\n")
        .await
        .expect("write test bat");

    let rel_cmd = "sub/sensor.bat";
    let res = execute_sensor_command(rel_cmd, &tmp, Some(Duration::from_secs(10)))
        .await
        .expect("sensor bat execution");
    assert!(res.success, "sensor bat should succeed, stderr: {}", res.stderr);
    assert!(res.stdout.contains("sensor_batch_ok"), "got stdout: {}", res.stdout);

    let _ = tokio::fs::remove_dir_all(&tmp).await;
}

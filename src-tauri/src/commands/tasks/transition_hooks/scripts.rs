use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

use crate::agent::Task;
use crate::hooks::{execute_hook, HookContext};
use crate::commands::tasks::lifecycle::record_task_hook_result;
use super::helpers::upsert_single_task;

#[allow(clippy::too_many_arguments)]
pub async fn trigger_column_on_enter_script(
    col: &crate::config::PipelineColumn,
    task: &mut Task,
    target_stage_id: &str,
    verdict: Option<&str>,
    root: &Path,
    app_handle: Option<&AppHandle>,
) {
    if let Some(ref script_str) = col.on_enter_script {
        let trimmed = script_str.trim();
        if trimmed.is_empty() {
            return;
        }

        let script_buf = PathBuf::from(trimmed);
        let script_path = if script_buf.is_absolute() {
            script_buf
        } else {
            root.join(&script_buf)
        };

        if !script_path.exists() {
            eprintln!(
                "CVH [on_enter_script]: Hook script does not exist at {:?}",
                script_path
            );
            if let Some(app) = app_handle {
                let _ = app.emit("hook:finish", serde_json::json!({
                    "taskId": task.id,
                    "stageId": target_stage_id,
                    "hookType": "on_enter",
                    "success": false,
                    "exitCode": None::<i32>,
                    "stdout": "",
                    "stderr": format!("Script not found: {}", script_path.display()),
                    "error": format!("Script not found: {}", script_path.display()),
                    "hookName": trimmed,
                }));
            }
            return;
        }

        let wt_path = task
            .worktree_path
            .as_deref()
            .map(PathBuf::from)
            .filter(|p| p.exists())
            .unwrap_or_else(|| root.to_path_buf());

        let ctx = HookContext {
            task_id: task.id.clone(),
            task_title: task.title.clone(),
            worktree_path: task.worktree_path.clone().unwrap_or_default(),
            branch: task.branch.clone().unwrap_or_default(),
            verdict: verdict.map(|v| v.to_string()).or_else(|| task.verdict.clone()),
            stage_id: Some(target_stage_id.to_string()),
            board_id: task.board_id.clone(),
            extra_env: HashMap::new(),
        };

        println!(
            "CVH [on_enter_script]: Executing hook {:?} for task {} entering stage {}",
            script_path, task.id, target_stage_id
        );

        if let Some(app) = app_handle {
            let _ = app.emit("hook:start", serde_json::json!({
                "taskId": task.id,
                "stageId": target_stage_id,
                "hookType": "on_enter",
                "hookName": trimmed,
                "description": "on_enter_script hook",
            }));
        }
        let hook_started_at = chrono::Utc::now().timestamp_millis();

        match execute_hook(&script_path, &wt_path, &ctx, None).await {
            Ok(res) => {
                let duration_ms = (chrono::Utc::now().timestamp_millis() - hook_started_at).max(0) as u64;
                record_task_hook_result(
                    task,
                    target_stage_id,
                    "on_enter",
                    trimmed,
                    hook_started_at,
                    duration_ms,
                    res.success,
                    res.exit_code,
                    &res.stdout,
                    &res.stderr,
                    if res.success { None } else { Some(res.stderr.as_str()) },
                );
                if let Some(app) = app_handle {
                    let _ = app.emit("hook:finish", serde_json::json!({
                        "taskId": task.id,
                        "stageId": target_stage_id,
                        "hookType": "on_enter",
                        "success": res.success,
                        "exitCode": res.exit_code,
                        "stdout": res.stdout,
                        "stderr": res.stderr,
                    }));
                    let _ = upsert_single_task(root, task);
                }
                if res.success {
                    println!(
                        "CVH [on_enter_script]: Hook {:?} succeeded for task {} in stage {} (exit code {:?}):\nSTDOUT: {}",
                        script_path, task.id, target_stage_id, res.exit_code, res.stdout
                    );
                } else {
                    eprintln!(
                        "CVH [on_enter_script]: Hook {:?} failed for task {} in stage {} (exit code {:?}):\nSTDOUT: {}\nSTDERR: {}",
                        script_path, task.id, target_stage_id, res.exit_code, res.stdout, res.stderr
                    );
                }
            }
            Err(e) => {
                let duration_ms = (chrono::Utc::now().timestamp_millis() - hook_started_at).max(0) as u64;
                let err = e.to_string();
                record_task_hook_result(
                    task,
                    target_stage_id,
                    "on_enter",
                    trimmed,
                    hook_started_at,
                    duration_ms,
                    false,
                    None,
                    "",
                    &err,
                    Some(&err),
                );
                if let Some(app) = app_handle {
                    let _ = app.emit("hook:finish", serde_json::json!({
                        "taskId": task.id,
                        "stageId": target_stage_id,
                        "hookType": "on_enter",
                        "success": false,
                        "exitCode": None::<i32>,
                        "stdout": "",
                        "stderr": &err,
                        "error": &err,
                    }));
                    let _ = upsert_single_task(root, task);
                }
                eprintln!(
                    "CVH [on_enter_script]: Error executing hook {:?} for task {}: {}",
                    script_path, task.id, e
                );
            }
        }
    }
}

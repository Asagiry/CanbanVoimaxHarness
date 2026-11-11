use std::path::{Path, PathBuf};
use chrono::Utc;
use tauri::{AppHandle, Emitter};

use crate::agent::Task;
use crate::commands::dialogs::append_task_dialog_activity;
use crate::commands::tasks::lifecycle::record_task_hook_result;
use crate::commands::tasks::storage::load_tasks_from_disk;
use super::helpers::{sync_single_task_in_app_state, upsert_single_task};

/// Unifies stage transition lifecycle execution (§ 2.2, § 2.6, § 2.7, § 2.27):
/// 1. `on_leave` / `on_exit` hook of the source column.
/// 2. `on_enter` hook of the target column.
/// 3. Unified rejection handling and event broadcasting.
pub struct TaskTransitionPipeline<'a> {
    pub root: &'a Path,
    pub source_col: Option<&'a crate::config::PipelineColumn>,
    pub target_col: &'a crate::config::PipelineColumn,
    pub is_source_intake: bool,
    pub verdict: Option<String>,
    pub emit_task_moved: bool,
    pub app_handle: Option<&'a AppHandle>,
}

impl<'a> TaskTransitionPipeline<'a> {
    pub fn new(
        root: &'a Path,
        source_col: Option<&'a crate::config::PipelineColumn>,
        target_col: &'a crate::config::PipelineColumn,
        is_source_intake: bool,
        verdict: Option<String>,
        emit_task_moved: bool,
        app_handle: Option<&'a AppHandle>,
    ) -> Self {
        Self {
            root,
            source_col,
            target_col,
            is_source_intake,
            verdict,
            emit_task_moved,
            app_handle,
        }
    }

    /// Step: Rejection and failure handling with state persistence and event broadcast.
    /// Upserts ONLY the rejected task in SQLite to avoid overwriting concurrent updates to other tasks.
    pub async fn handle_rejection(
        &self,
        task: &mut Task,
        steer_content: &str,
        error_msg: String,
        reason_details: &str,
    ) -> Result<(), String> {
        let _ = append_task_dialog_activity(
            self.root,
            &task.id,
            "steer",
            steer_content,
            self.app_handle,
        )
        .await;

        let scol_has_autorun = self
            .source_col
            .map(|c| c.persona.is_some() || c.auto_run == Some(true))
            .unwrap_or(false);

        task.verdict = Some("CHANGES_REQUESTED".to_string());
        task.status = if scol_has_autorun {
            Some("running".to_string())
        } else {
            Some("idle".to_string())
        };
        task.updated_at = Utc::now().timestamp_millis();

        let _ = upsert_single_task(self.root, task);
        let failed_task = task.clone();
        sync_single_task_in_app_state(self.root, &failed_task, self.app_handle);

        let tasks = load_tasks_from_disk(self.root).await;

        if let Some(app) = self.app_handle {
            let project_path_str = self.root.to_string_lossy().to_string();
            if self.emit_task_moved {
                let _ = app.emit("task:moved", serde_json::json!({
                    "task": &failed_task,
                    "rejected": true,
                    "reason": reason_details,
                    "projectPath": &project_path_str,
                }));
            }
            let _ = app.emit("task:updated", serde_json::json!({
                "taskId": failed_task.id,
                "task": &failed_task,
                "projectPath": &project_path_str,
            }));
            let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks, "projectPath": &project_path_str }));

            if scol_has_autorun {
                let _ = app.emit("agent:autorun", serde_json::json!({
                    "taskId": failed_task.id,
                    "prompt": steer_content,
                    "projectPath": &project_path_str,
                }));
            }
        }

        Err(error_msg)
    }

    /// Step: Execute source column `gatekeeper_sensor` command if configured.
    /// Skipped when rejecting backwards with verdict `CHANGES_REQUESTED`.
    pub async fn run_gatekeeper_sensor(&self, task: &mut Task) -> Result<(), String> {
        let Some(sc) = self.source_col else {
            return Ok(());
        };
        if self.verdict.as_deref() == Some("CHANGES_REQUESTED") {
            return Ok(());
        }
        let Some(sensor_cmd) = sc
            .gatekeeper_sensor
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            return Ok(());
        };

        let working_dir = task
            .worktree_path
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|wt| {
                let p = PathBuf::from(wt);
                if p.is_absolute() { p } else { self.root.join(p) }
            })
            .filter(|p| p.exists())
            .unwrap_or_else(|| self.root.to_path_buf());

        if let Some(app) = self.app_handle {
            let _ = app.emit("hook:start", serde_json::json!({
                "taskId": task.id,
                "stageId": sc.id,
                "hookType": "gatekeeper_sensor",
                "hookName": sensor_cmd,
                "description": "Gatekeeper sensor check",
            }));
        }

        let started_at = Utc::now().timestamp_millis();
        let sensor_res = crate::hooks::execute_sensor_command(sensor_cmd, &working_dir, None).await;
        let duration_ms = (Utc::now().timestamp_millis() - started_at).max(0) as u64;

        match sensor_res {
            Ok(res) if res.success => {
                record_task_hook_result(
                    task,
                    &sc.id,
                    "gatekeeper_sensor",
                    sensor_cmd,
                    started_at,
                    duration_ms,
                    true,
                    res.exit_code,
                    &res.stdout,
                    &res.stderr,
                    None,
                );
                if let Some(app) = self.app_handle {
                    let _ = app.emit("hook:finish", serde_json::json!({
                        "taskId": task.id,
                        "stageId": sc.id,
                        "hookType": "gatekeeper_sensor",
                        "success": true,
                        "exitCode": res.exit_code,
                        "stdout": res.stdout,
                        "stderr": res.stderr,
                    }));
                }
                upsert_single_task(self.root, task)
            }
            Ok(res) => {
                let err_details = if !res.stderr.trim().is_empty() {
                    res.stderr.trim().to_string()
                } else if !res.stdout.trim().is_empty() {
                    res.stdout.trim().to_string()
                } else {
                    format!("Exit code {:?}", res.exit_code)
                };
                record_task_hook_result(
                    task,
                    &sc.id,
                    "gatekeeper_sensor",
                    sensor_cmd,
                    started_at,
                    duration_ms,
                    false,
                    res.exit_code,
                    &res.stdout,
                    &res.stderr,
                    Some(&err_details),
                );
                if let Some(app) = self.app_handle {
                    let _ = app.emit("hook:finish", serde_json::json!({
                        "taskId": task.id,
                        "stageId": sc.id,
                        "hookType": "gatekeeper_sensor",
                        "success": false,
                        "exitCode": res.exit_code,
                        "stdout": res.stdout,
                        "stderr": res.stderr,
                        "error": &err_details,
                    }));
                }
                let steer_content = format!(
                    "### [ОТКЛОНЕНО СЕНСОРОМ / GATEKEEPER FAILED: {}]\n\nСенсор этапа '{}' завершился с ошибкой (код {:?}).\n\n**Вывод:**\n```\n{}\n```\n\nИсправь указанную ошибку.",
                    sensor_cmd, sc.id, res.exit_code, err_details
                );
                let err_msg = format!(
                    "Gatekeeper sensor '{}' этапа '{}' отклонил переход (Exit code: {:?}):\n{}",
                    sensor_cmd, sc.id, res.exit_code, err_details
                );
                self.handle_rejection(task, &steer_content, err_msg, &err_details).await
            }
            Err(e) => {
                let err_details = e.to_string();
                record_task_hook_result(
                    task,
                    &sc.id,
                    "gatekeeper_sensor",
                    sensor_cmd,
                    started_at,
                    duration_ms,
                    false,
                    None,
                    "",
                    &err_details,
                    Some(&err_details),
                );
                if let Some(app) = self.app_handle {
                    let _ = app.emit("hook:finish", serde_json::json!({
                        "taskId": task.id,
                        "stageId": sc.id,
                        "hookType": "gatekeeper_sensor",
                        "success": false,
                        "exitCode": None::<i32>,
                        "stdout": "",
                        "stderr": &err_details,
                        "error": &err_details,
                    }));
                }
                let steer_content = format!(
                    "### [ОТКЛОНЕНО СЕНСОРОМ / GATEKEEPER ERROR: {}]\n\nОшибка запуска сенсора этапа '{}': {}\n\nИсправь указанную ошибку.",
                    sensor_cmd, sc.id, err_details
                );
                let err_msg = format!(
                    "Gatekeeper sensor '{}' этапа '{}' ошибка запуска: {}",
                    sensor_cmd, sc.id, err_details
                );
                self.handle_rejection(task, &steer_content, err_msg, &err_details).await
            }
        }
    }

    /// Master execution: run `gatekeeper_sensor` -> `on_leave` -> `on_enter`.
    pub async fn execute(&self, task: &mut Task) -> Result<(), String> {
        self.run_gatekeeper_sensor(task).await?;
        self.run_on_leave(task).await?;
        self.run_on_enter(task).await?;
        Ok(())
    }
}

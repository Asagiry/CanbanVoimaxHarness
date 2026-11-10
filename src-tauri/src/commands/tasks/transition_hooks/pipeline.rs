use std::path::Path;
use chrono::Utc;
use tauri::{AppHandle, Emitter};

use crate::agent::Task;
use crate::commands::dialogs::append_task_dialog_activity;
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

    /// Master execution: run `on_leave` -> `on_enter`.
    pub async fn execute(&self, task: &mut Task) -> Result<(), String> {
        self.run_on_leave(task).await?;
        self.run_on_enter(task).await?;
        Ok(())
    }
}

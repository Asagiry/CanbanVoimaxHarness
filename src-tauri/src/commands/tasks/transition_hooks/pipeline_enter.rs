use std::collections::HashMap;
use tauri::{Emitter, Manager};

use crate::agent::Task;
use crate::commands::git::apply_worktree_from_hook_stdout;
use crate::commands::healing::task_id_matches;
use crate::commands::state::AppState;
use crate::hooks::{execute_hook, HookContext};
use crate::commands::tasks::lifecycle::record_task_hook_result;
use super::helpers::upsert_single_task;
use super::pipeline::TaskTransitionPipeline;

impl<'a> TaskTransitionPipeline<'a> {
    /// Step: Execute target column `on_enter` custom hook script if configured.
    pub async fn run_on_enter(&self, task: &mut Task) -> Result<(), String> {
        let hook_cfg = match self.target_col.hooks.get("on_enter").cloned() {
            Some(cfg) => cfg,
            None => return Ok(()),
        };

        let hook_rel = hook_cfg.script().to_string();
        let hook_desc = hook_cfg.description().map(str::to_string);
        let hook_path = crate::config::resolve_hook_path(
            self.root,
            self.target_col.persona.as_deref(),
            &hook_rel,
        );
        if !hook_path.exists() {
            if let Some(app) = self.app_handle {
                let _ = app.emit("hook:finish", serde_json::json!({
                    "taskId": task.id,
                    "stageId": self.target_col.id,
                    "hookType": "on_enter",
                    "success": false,
                    "exitCode": None::<i32>,
                    "stdout": "",
                    "stderr": format!("Script not found: {}", hook_path.display()),
                    "error": format!("Script not found: {}", hook_path.display()),
                    "hookName": &hook_rel,
                }));
            }
            return Ok(());
        }

        if let Some(app) = self.app_handle {
            let _ = app.emit("hook:start", serde_json::json!({
                "taskId": task.id,
                "stageId": self.target_col.id,
                "hookType": "on_enter",
                "hookName": &hook_rel,
                "description": hook_desc,
            }));
        }

        let hook_started_at = chrono::Utc::now().timestamp_millis();
        let ctx = HookContext {
            task_id: task.id.clone(),
            task_title: task.title.clone(),
            worktree_path: task.worktree_path.clone().unwrap_or_default(),
            branch: task.branch.clone().unwrap_or_default(),
            verdict: self.verdict.clone().or_else(|| task.verdict.clone()),
            stage_id: Some(self.target_col.id.clone()),
            board_id: task.board_id.clone(),
            extra_env: HashMap::new(),
        };

        let hook_res = execute_hook(&hook_path, self.root, &ctx, None).await;
        let duration_ms = (chrono::Utc::now().timestamp_millis() - hook_started_at).max(0) as u64;

        match hook_res {
            Ok(res) => {
                if !res.success {
                    let err_details = if !res.stderr.trim().is_empty() {
                        res.stderr.trim().to_string()
                    } else if !res.stdout.trim().is_empty() {
                        res.stdout.trim().to_string()
                    } else {
                        format!("Exit code {:?}", res.exit_code)
                    };

                    record_task_hook_result(
                        task,
                        &self.target_col.id,
                        "on_enter",
                        &hook_rel,
                        hook_started_at,
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
                            "stageId": self.target_col.id,
                            "hookType": "on_enter",
                            "success": false,
                            "exitCode": res.exit_code,
                            "stdout": res.stdout,
                            "stderr": res.stderr,
                            "error": &err_details
                        }));
                    }

                    let steer_content = format!(
                        "### [ОТКЛОНЕНО ХУКОМ / HOOK REJECTED: {}]\n\nХук на входе в колонку '{}' завершился с ошибкой (код {:?}).\n\n**Лог ошибки:**\n```\n{}\n```\n\nИсправь указанную ошибку.",
                        hook_rel, self.target_col.id, res.exit_code, err_details
                    );
                    let err_msg = format!(
                        "Хук on_enter '{}' целевой колонки '{}' отклонил переход (Exit code: {:?}):\n{}",
                        hook_rel, self.target_col.id, res.exit_code, err_details
                    );
                    return self.handle_rejection(task, &steer_content, err_msg, &err_details).await;
                }

                record_task_hook_result(
                    task,
                    &self.target_col.id,
                    "on_enter",
                    &hook_rel,
                    hook_started_at,
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
                        "stageId": self.target_col.id,
                        "hookType": "on_enter",
                        "success": true,
                        "exitCode": res.exit_code,
                        "stdout": res.stdout,
                        "stderr": res.stderr
                    }));
                }

                apply_worktree_from_hook_stdout(task, &res.stdout);
                let _ = upsert_single_task(self.root, task);
                if let Some(app) = self.app_handle {
                    if let Some(state_ref) = app.try_state::<AppState>() {
                        if state_ref.get_root() == self.root {
                            let mut guard = state_ref.tasks.lock().unwrap_or_else(|p| p.into_inner());
                            if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, &task.id)) {
                                t.worktree_path = task.worktree_path.clone();
                                t.branch = task.branch.clone();
                                t.hook_states = task.hook_states.clone();
                            }
                        }
                    }
                }
                Ok(())
            }
            Err(e) => {
                let err_details = e.to_string();
                record_task_hook_result(
                    task,
                    &self.target_col.id,
                    "on_enter",
                    &hook_rel,
                    hook_started_at,
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
                        "stageId": self.target_col.id,
                        "hookType": "on_enter",
                        "success": false,
                        "exitCode": None::<i32>,
                        "stdout": "",
                        "stderr": &err_details,
                        "error": &err_details
                    }));
                }

                let steer_content = format!(
                    "### [ОТКЛОНЕНО ХУКОМ / HOOK REJECTED: {}]\n\nОшибка запуска хука на входе в колонку '{}': {}\n\nИсправь указанную ошибку.",
                    hook_rel, self.target_col.id, err_details
                );
                let err_msg = format!(
                    "Хук on_enter '{}' целевой колонки '{}' ошибка исполнения: {}",
                    hook_rel, self.target_col.id, err_details
                );
                self.handle_rejection(task, &steer_content, err_msg, &err_details).await
            }
        }
    }
}

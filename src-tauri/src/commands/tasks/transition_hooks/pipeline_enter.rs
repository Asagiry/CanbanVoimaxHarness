use std::collections::HashMap;
use tauri::{Emitter, Manager};

use crate::agent::Task;
use crate::commands::git::apply_worktree_from_hook_stdout;
use crate::commands::state::AppState;
use crate::hooks::{execute_hook, HookContext};
use crate::commands::healing::{stage_id_matches, task_id_matches};
use crate::commands::tasks::lifecycle::record_task_hook_result;
use super::helpers::{ensure_task_worktree, upsert_single_task};
use super::pipeline::TaskTransitionPipeline;

impl<'a> TaskTransitionPipeline<'a> {
    /// Step: Execute target column `on_enter` hook, or builtin:create_worktree, or backlog worktree allocation.
    pub async fn run_on_enter(&self, task: &mut Task) -> Result<(), String> {
        let hook_cfg_opt = self.target_col.hooks.get("on_enter").cloned();
        let is_builtin = hook_cfg_opt.as_ref().map(|cfg| {
            let s = cfg.script();
            s == "builtin:create_worktree" || s.starts_with("builtin:create_worktree")
        }).unwrap_or(false);

        let is_backlog_col = self.target_col.is_intake == Some(true)
            || stage_id_matches(&self.target_col.id, "backlog");

        // Worktree allocation: if target stage has builtin:create_worktree or backlog entry (§ 3.1 & § 3.2)
        if is_builtin || is_backlog_col {
            let wt_empty = task.worktree_path.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true);
            if wt_empty && !task.skip_worktree.unwrap_or(false) {
                if let Some(app) = self.app_handle {
                    let _ = app.emit("hook:start", serde_json::json!({
                        "taskId": task.id,
                        "stageId": self.target_col.id,
                        "hookType": "on_enter",
                        "hookName": "builtin:create_worktree",
                        "description": "Allocate isolated git worktree",
                    }));
                }
                let started_at = chrono::Utc::now().timestamp_millis();
                let wt_res = ensure_task_worktree(self.root, task).await;
                let duration_ms = (chrono::Utc::now().timestamp_millis() - started_at).max(0) as u64;
                let success = wt_res.is_ok();
                let wt_stdout = task.worktree_path.clone().unwrap_or_default();
                let wt_stderr = if let Err(ref e) = wt_res { e.clone() } else { String::new() };
                record_task_hook_result(
                    task,
                    &self.target_col.id,
                    "on_enter",
                    "builtin:create_worktree",
                    started_at,
                    duration_ms,
                    success,
                    if success { Some(0) } else { Some(1) },
                    &wt_stdout,
                    &wt_stderr,
                    if !wt_stderr.is_empty() { Some(&wt_stderr) } else { None },
                );
                if let Some(app) = self.app_handle {
                    let _ = app.emit("hook:finish", serde_json::json!({
                        "taskId": task.id,
                        "stageId": self.target_col.id,
                        "hookType": "on_enter",
                        "success": success,
                        "exitCode": if success { Some(0) } else { Some(1) },
                        "stdout": task.worktree_path.as_deref().unwrap_or(""),
                        "stderr": if let Err(ref e) = wt_res { e.as_str() } else { "" },
                    }));
                }
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
            }
            if is_builtin {
                return Ok(());
            }
        }

        let hook_cfg = match hook_cfg_opt {
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

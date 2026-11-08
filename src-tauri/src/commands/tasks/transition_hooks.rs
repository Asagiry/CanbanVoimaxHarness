//! Stage Transition Hooks, Worktree Lifecycle, and Handover Formatting Helpers
//! Extracted from `commands/tasks/transitions.rs` for modularity and single-task persistence safety.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager};

use crate::agent::Task;
use crate::commands::dialogs::append_task_dialog_activity;
use crate::commands::git::apply_worktree_from_hook_stdout;
use crate::commands::state::AppState;
use crate::git::create_worktree;
use crate::hooks::{execute_hook, HookContext};
use super::super::healing::{stage_id_matches, task_id_matches};
use super::super::lifecycle::{normalize_hook_type, record_task_hook_result};
use super::super::storage::load_tasks_from_disk;

/// Upserts a single mutated task into SQLite (ensuring `project_path` is set) without
/// overwriting any other concurrent tasks on the board.
pub fn upsert_single_task(root: &Path, task: &mut Task) -> Result<(), String> {
    if task.project_path.is_none() && !root.as_os_str().is_empty() {
        task.project_path = Some(root.to_string_lossy().to_string());
    }
    crate::db::db_upsert_task(root, task)
        .map_err(|e| format!("Failed to upsert task '{}' in SQLite: {}", task.id, e))
}

/// Synchronizes a single mutated or newly created task into in-memory `AppState`
/// without overwriting concurrent updates to other tasks.
pub fn sync_single_task_in_app_state(root: &Path, task: &Task, app_handle: Option<&AppHandle>) {
    if let Some(app) = app_handle {
        if let Some(state_ref) = app.try_state::<AppState>() {
            if state_ref.get_root().as_path() == root {
                if let Ok(mut guard) = state_ref.tasks.lock() {
                    if let Some(existing) = guard.iter_mut().find(|t| task_id_matches(&t.id, &task.id)) {
                        *existing = task.clone();
                    } else {
                        guard.push(task.clone());
                    }
                }
            }
        }
    }
}

/// Formats a stage completion summary activity entry for the task dialog log.
pub fn format_stage_summary_activity(
    stage_label: &str,
    task_title: Option<&str>,
    summary: &str,
) -> String {
    match task_title {
        Some(title) => format!("### Саммари этапа {}: {}\n\n{}", stage_label, title, summary),
        None => format!("### Саммари этапа: {}\n\n{}", stage_label, summary),
    }
}

/// Formats a handover steer prompt when reactivating a previously rejected subtask.
pub fn format_handover_steer_prompt(summary: &str) -> String {
    format!(
        "### [ОТЧЕТ ОБ ИСПРАВЛЕНИЯХ / HANDOVER SUMMARY]\n\nЗамечания исправлены. Отчет исполнителя:\n{}",
        summary
    )
}

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

/// Allocates an isolated git worktree for a task if not already provisioned (§ 3.1 & § 3.2).
pub async fn ensure_task_worktree(root: &Path, task: &mut Task) -> Result<(), String> {
    if task.skip_worktree.unwrap_or(false) {
        return Ok(());
    }
    if let Some(ref wt) = task.worktree_path {
        let trimmed = wt.trim();
        if !trimmed.is_empty() {
            let p = Path::new(trimmed);
            let abs_p = if p.is_absolute() { p.to_path_buf() } else { root.join(p) };
            if abs_p.exists() {
                return Ok(());
            }
        }
    }
    let slug = task.id.to_lowercase();
    match create_worktree(root, &slug, "HEAD").await {
        Ok(wt_abs) => {
            let rel = wt_abs.strip_prefix(root).unwrap_or(&wt_abs);
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            task.worktree_path = Some(rel_str);
            if task.branch.as_deref().map(|b| b.trim().is_empty()).unwrap_or(true) {
                task.branch = Some(format!("feat/{}", slug));
            }
            crate::logger::cvh_log(
                Some(root),
                "INFO",
                "WORKTREE",
                &format!(
                    "Allocated worktree {:?} on branch {:?} for task '{}'",
                    task.worktree_path, task.branch, task.id
                ),
            );
            Ok(())
        }
        Err(e) => {
            crate::logger::cvh_log(
                Some(root),
                "WARN",
                "WORKTREE",
                &format!("Failed to allocate worktree for task '{}': {}", task.id, e),
            );
            Ok(())
        }
    }
}

#[allow(clippy::too_many_arguments)]
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

    /// Step: Execute source column `on_finish` / `on_leave` / `on_exit` hook.
    pub async fn run_on_leave(&self, task: &mut Task) -> Result<(), String> {
        let sc = match self.source_col {
            Some(c) => c,
            None => return Ok(()),
        };
        let (hook_key, hook_cfg) = if let Some(cfg) = sc.hooks.get("on_finish") {
            ("on_finish", cfg)
        } else if let Some(cfg) = sc.hooks.get("on_leave") {
            ("on_leave", cfg)
        } else if let Some(cfg) = sc.hooks.get("on_exit") {
            ("on_exit", cfg)
        } else {
            return Ok(());
        };

        let emit_hook_type = normalize_hook_type(hook_key);
        let hook_rel = hook_cfg.script();
        let hook_desc = hook_cfg.description();
        let hook_path = crate::config::resolve_hook_path(self.root, sc.persona.as_deref(), hook_rel);
        if !hook_path.exists() {
            if let Some(app) = self.app_handle {
                let _ = app.emit("hook:finish", serde_json::json!({
                    "taskId": task.id,
                    "stageId": sc.id,
                    "hookType": emit_hook_type,
                    "success": false,
                    "exitCode": None::<i32>,
                    "stdout": "",
                    "stderr": format!("Script not found: {}", hook_path.display()),
                    "error": format!("Script not found: {}", hook_path.display()),
                    "hookName": hook_rel,
                }));
            }
            return Ok(());
        }

        if let Some(app) = self.app_handle {
            let _ = app.emit("hook:start", serde_json::json!({
                "taskId": task.id,
                "stageId": sc.id,
                "hookType": emit_hook_type,
                "hookName": hook_rel,
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
            stage_id: Some(sc.id.clone()),
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
                        &sc.id,
                        hook_key,
                        hook_rel,
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
                            "stageId": sc.id,
                            "hookType": emit_hook_type,
                            "success": false,
                            "exitCode": res.exit_code,
                            "stdout": res.stdout,
                            "stderr": res.stderr,
                            "error": &err_details
                        }));
                    }

                    // on_finish is best-effort (GUIDE.md § 3.2): failure does NOT block transition
                    if hook_key == "on_finish" {
                        crate::logger::cvh_log(
                            Some(self.root),
                            "WARN",
                            "HOOK",
                            &format!("Best-effort on_finish hook '{}' failed for task '{}' (code {:?}): {}", hook_rel, task.id, res.exit_code, err_details),
                        );
                        apply_worktree_from_hook_stdout(task, &res.stdout);
                        let _ = upsert_single_task(self.root, task);
                        return Ok(());
                    }

                    let steer_content = format!(
                        "### [ОТКЛОНЕНО ХУКОМ / HOOK REJECTED: {}]\n\nХук на выходе из колонки '{}' завершился с ошибкой (код {:?}).\n\n**Лог ошибки:**\n```\n{}\n```\n\nИсправь указанную ошибку.",
                        hook_rel, sc.id, res.exit_code, err_details
                    );
                    let err_msg = format!(
                        "Хук {} '{}' отклонил переход (Exit code: {:?}):\n{}",
                        hook_key, hook_rel, res.exit_code, err_details
                    );
                    return self.handle_rejection(task, &steer_content, err_msg, &err_details).await;
                }

                record_task_hook_result(
                    task,
                    &sc.id,
                    hook_key,
                    hook_rel,
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
                        "stageId": sc.id,
                        "hookType": emit_hook_type,
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
                            if let Ok(mut guard) = state_ref.tasks.lock() {
                                if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, &task.id)) {
                                    t.worktree_path = task.worktree_path.clone();
                                    t.branch = task.branch.clone();
                                    t.hook_states = task.hook_states.clone();
                                }
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
                    &sc.id,
                    hook_key,
                    hook_rel,
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
                        "stageId": sc.id,
                        "hookType": emit_hook_type,
                        "success": false,
                        "exitCode": None::<i32>,
                        "stdout": "",
                        "stderr": &err_details,
                        "error": &err_details
                    }));
                }

                if hook_key == "on_finish" {
                    crate::logger::cvh_log(
                        Some(self.root),
                        "WARN",
                        "HOOK",
                        &format!("Best-effort on_finish hook '{}' error for task '{}': {}", hook_rel, task.id, err_details),
                    );
                    return Ok(());
                }

                let steer_content = format!(
                    "### [ОТКЛОНЕНО ХУКОМ / HOOK REJECTED: {}]\n\nОшибка запуска хука на выходе из колонки '{}': {}\n\nИсправь указанную ошибку.",
                    hook_rel, sc.id, err_details
                );
                let err_msg = format!("Хук {} '{}' ошибка исполнения: {}", hook_key, hook_rel, err_details);
                self.handle_rejection(task, &steer_content, err_msg, &err_details).await
            }
        }
    }

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
                            if let Ok(mut guard) = state_ref.tasks.lock() {
                                if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, &task.id)) {
                                    t.worktree_path = task.worktree_path.clone();
                                    t.branch = task.branch.clone();
                                    t.hook_states = task.hook_states.clone();
                                }
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
                            if let Ok(mut guard) = state_ref.tasks.lock() {
                                if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, &task.id)) {
                                    t.worktree_path = task.worktree_path.clone();
                                    t.branch = task.branch.clone();
                                    t.hook_states = task.hook_states.clone();
                                }
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

    /// Master execution: run `on_leave` -> `on_enter`.
    pub async fn execute(&self, task: &mut Task) -> Result<(), String> {
        self.run_on_leave(task).await?;
        self.run_on_enter(task).await?;
        Ok(())
    }
}

/// Fallback: auto-completes any unfinished task todos (status != 'completed')
/// when advancing or completing a task, updating SQLite, memory, and emitting UI events.
/// Returns true if any todos were modified/completed, false if already completed or absent.
pub fn auto_complete_task_todos(
    root: &Path,
    task: &mut Task,
    app_handle: Option<&AppHandle>,
) -> bool {
    let memory_had_unfinished = task.todos.as_ref().is_some_and(|tds| {
        tds.iter().any(|td| td.status != "completed")
    });
    let db_had_unfinished = if memory_had_unfinished {
        true
    } else {
        crate::db::db_has_unfinished_todos(root, &task.id).unwrap_or(true)
    };

    if !memory_had_unfinished && !db_had_unfinished {
        return false;
    }

    let mut completed_count = 0usize;
    if let Some(ref mut todos) = task.todos {
        for td in todos.iter_mut() {
            if td.status != "completed" {
                td.status = "completed".to_string();
                completed_count += 1;
            }
        }
    }
    if completed_count > 0 {
        crate::logger::cvh_log(
            Some(root),
            "INFO",
            "AUTO_COMPLETE_TODOS",
            &format!("Auto-completed {} unfinished TODO item(s) for task {}", completed_count, task.id),
        );
    }

    match crate::db::db_complete_all_todos(root, &task.id) {
        Ok(Some(db_todos)) => {
            if task.todos.is_none() {
                task.todos = Some(db_todos);
            }
        }
        Ok(None) => {}
        Err(e) => {
            crate::logger::cvh_log(
                Some(root),
                "WARN",
                "AUTO_COMPLETE_TODOS",
                &format!("db_complete_all_todos error for task {}: {}", task.id, e),
            );
        }
    }

    task.updated_at = Utc::now().timestamp_millis();
    let _ = upsert_single_task(root, task);

    if let Some(app) = app_handle {
        if let Some(state_ref) = app.try_state::<AppState>() {
            if state_ref.get_root().as_path() == root {
                if let Ok(mut guard) = state_ref.tasks.lock() {
                    if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, &task.id)) {
                        t.todos = task.todos.clone();
                        t.updated_at = task.updated_at;
                    }
                }
            }
        }
        let project_path_str = root.to_string_lossy().to_string();
        let _ = app.emit("task:updated", serde_json::json!({
            "taskId": &task.id,
            "todos": &task.todos,
            "task": &task,
            "projectPath": &project_path_str,
        }));
    }

    true
}

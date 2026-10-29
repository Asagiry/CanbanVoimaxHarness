//! Task Pipeline Transitions & Stage Progression Subsystem (§ 2.2, § 2.6, § 2.7, § 2.27)
//! Extracted from commands/tasks.rs per ub-mode Clean Code principles.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::agent::Task;
use crate::commands::dialogs::append_task_dialog_activity;
use crate::commands::git::apply_worktree_from_hook_stdout;
use crate::commands::state::AppState;
use crate::config::{load_boards_config, load_pipeline_config, resolve_actual_repo_root};
use crate::hooks::{execute_hook, execute_sensor_command, HookContext};
use super::healing::{stage_id_matches, task_id_matches};
use super::lifecycle::{
    internal_create_subtask, record_task_hook_result,
};
use super::runner::internal_abort_agent;
use super::storage::{
    get_next_subtask_index, load_tasks_from_disk, save_tasks_to_disk,
};

pub async fn trigger_column_on_enter_script(
    col: &crate::config::PipelineColumn,
    task: &Task,
    target_stage_id: &str,
    verdict: Option<&str>,
    root: &Path,
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

        match execute_hook(&script_path, &wt_path, &ctx, None).await {
            Ok(res) => {
                if res.success {
                    println!(
                        "CVH [on_enter_script]: Hook {:?} succeeded for task {} in stage {} (exit code {:?}):
STDOUT: {}",
                        script_path, task.id, target_stage_id, res.exit_code, res.stdout
                    );
                } else {
                    eprintln!(
                        "CVH [on_enter_script]: Hook {:?} failed for task {} in stage {} (exit code {:?}):
STDOUT: {}
STDERR: {}",
                        script_path, task.id, target_stage_id, res.exit_code, res.stdout, res.stderr
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "CVH [on_enter_script]: Error executing hook {:?} for task {}: {}",
                    script_path, task.id, e
                );
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct ParentAdvanceResult {
    pub moved_events: Vec<serde_json::Value>,
    pub created_tasks: Vec<Task>,
    pub autorun_task_ids: Vec<String>,
}

/// Propagates advancement of completed/terminal subtasks up the hierarchy to parent tasks (§ 2.2 & § 2.7).
/// Universal relay progression: if next stage is non-terminal, parent spawns next isolated child subtask;
/// if next stage is terminal (ready/done), parent moves to ready with status done.
pub fn propagate_parent_subtask_advance(
    tasks: &mut Vec<Task>,
    completed_subtask_id: &str,
    subtask_source_stage: &str,
    verdict: Option<&str>,
    boards_cfg: Option<&crate::config::BoardsConfig>,
    pipeline: Option<&crate::config::PipelineConfig>,
) -> ParentAdvanceResult {
    let mut moved_events = Vec::new();
    let mut created_tasks = Vec::new();
    let mut autorun_task_ids = Vec::new();

    let mut current_child_id = completed_subtask_id.to_string();
    let mut current_source_stage = subtask_source_stage.to_string();

    // 1. Ensure the completed subtask has status "done"
    if let Some(sub_idx) = tasks.iter().position(|t| task_id_matches(&t.id, completed_subtask_id)) {
        tasks[sub_idx].status = Some("done".to_string());
        if tasks[sub_idx].completed_at.is_none() {
            tasks[sub_idx].completed_at = Some(Utc::now().timestamp_millis());
        }
        tasks[sub_idx].updated_at = Utc::now().timestamp_millis();
    }

    while let Some(parent_idx) = tasks.iter().position(|t| {
        t.waiting_for_subtask_id.as_deref() == Some(&current_child_id)
            || (t.status.as_deref() == Some("waiting_subtask")
                && tasks
                    .iter()
                    .find(|sub| sub.id == current_child_id)
                    .and_then(|sub| sub.parent_id.as_deref())
                    == Some(&t.id))
            || tasks
                .iter()
                .find(|sub| sub.id == current_child_id)
                .and_then(|sub| sub.parent_id.as_deref())
                == Some(&t.id)
    }) {
        let parent_id = tasks[parent_idx].id.clone();
        let old_parent_stage = tasks[parent_idx].stage_id.clone();
        let parent_board_id = tasks[parent_idx].board_id.clone();
        let parent_title = tasks[parent_idx].title.clone();

        // Resolve columns for parent task's board
        let cols = {
            let from_boards = boards_cfg.and_then(|bc| {
                if let Some(ref bid) = parent_board_id {
                    if let Some(b) = bc.boards.iter().find(|b| &b.id == bid) {
                        return Some(b.columns.clone());
                    }
                }
                if let Some(b) = bc.boards.iter().find(|b| b.id == bc.active_board_id) {
                    return Some(b.columns.clone());
                }
                bc.boards.first().map(|b| b.columns.clone())
            });
            from_boards.or_else(|| pipeline.map(|p| p.columns.clone())).unwrap_or_default()
        };

        // Find the next stage for the parent task:
        // Look up index of the subtask's source stage (e.g. "dev" or "review"):
        let sub_stage_idx = cols
            .iter()
            .position(|c| stage_id_matches(&c.id, &current_source_stage))
            .or_else(|| cols.iter().position(|c| stage_id_matches(&c.id, &old_parent_stage)));

        let next_col_opt = if let Some(idx) = sub_stage_idx {
            if idx + 1 < cols.len() {
                Some(cols[idx + 1].clone())
            } else {
                cols.last().cloned()
            }
        } else {
            cols.iter()
                .find(|c| c.terminal == Some(true) || stage_id_matches(&c.id, "ready") || stage_id_matches(&c.id, "done"))
                .cloned()
                .or_else(|| cols.last().cloned())
        };

        let next_col = match next_col_opt {
            Some(c) => c,
            None => break,
        };

        let is_terminal = next_col.terminal == Some(true)
            || stage_id_matches(&next_col.id, "ready")
            || stage_id_matches(&next_col.id, "done");

        if is_terminal {
            tasks[parent_idx].waiting_for_subtask_id = None;
            tasks[parent_idx].stage_id = next_col.id.clone();
            tasks[parent_idx].verdict = Some(verdict.unwrap_or("APPROVED").to_string());
            tasks[parent_idx].status = Some("done".to_string());
            if tasks[parent_idx].completed_at.is_none() {
                tasks[parent_idx].completed_at = Some(Utc::now().timestamp_millis());
            }
            tasks[parent_idx].updated_at = Utc::now().timestamp_millis();

            // Also mark any child subtasks of this parent as done so they collapse into parent (§ 3)
            for sub in tasks.iter_mut() {
                if sub.parent_id.as_deref() == Some(&parent_id) {
                    sub.status = Some("done".to_string());
                    if sub.completed_at.is_none() {
                        sub.completed_at = Some(Utc::now().timestamp_millis());
                    }
                    sub.updated_at = Utc::now().timestamp_millis();
                }
            }

            let parent_clone = tasks[parent_idx].clone();
            moved_events.push(serde_json::json!({
                "taskId": parent_id,
                "fromStage": old_parent_stage,
                "toStage": &next_col.id,
                "stageId": &next_col.id,
                "status": parent_clone.status,
                "verdict": parent_clone.verdict,
                "task": &parent_clone
            }));

            // If parent task also advanced to a terminal column, check if grandparent was waiting on it!
            current_child_id = parent_id;
            current_source_stage = next_col.id;
        } else {
            // Next stage is not terminal -> universal progression spawns next isolated child subtask!
            let next_subtask_idx = get_next_subtask_index(tasks, &parent_id);
            let new_subtask_id = format!("{}.{}", parent_id, next_subtask_idx);
            let now = Utc::now().timestamp_millis();

            tasks[parent_idx].waiting_for_subtask_id = Some(new_subtask_id.clone());
            tasks[parent_idx].status = Some("waiting_subtask".to_string());
            tasks[parent_idx].updated_at = now;

            let has_autorun = next_col.persona.is_some() || next_col.auto_run == Some(true);
            let subtask_status = if has_autorun { "running" } else { "idle" };

            let parent_wt = tasks[parent_idx].worktree_path.clone();
            let parent_branch = tasks[parent_idx].branch.clone();
            let parent_model = tasks[parent_idx].model.clone();
            let parent_skip_wt = tasks[parent_idx].skip_worktree;
            let parent_priority = tasks[parent_idx].priority.clone();

            let next_subtask_pos = tasks
                .iter()
                .filter(|t| t.stage_id == next_col.id)
                .filter_map(|t| t.position)
                .fold(0.0f64, f64::max) + 1000.0;

            let new_subtask = Task {
                id: new_subtask_id.clone(),
                title: format!("{} ({})", parent_title, next_col.title),
                description: format!(
                    "### Подзадача этапа: {}
Родительская задача: {}

{}",
                    next_col.title, parent_id, tasks[parent_idx].description
                ),
                stage_id: next_col.id.clone(),
                board_id: tasks[parent_idx].board_id.clone(),
                worktree_path: parent_wt,
                branch: parent_branch,
                verdict: None,
                custom_flow: None,
                status: Some(subtask_status.to_string()),
                model: parent_model.or_else(|| next_col.model_override.clone()),
                parent_id: Some(parent_id.clone()),
                subtask_index: Some(next_subtask_idx),
                waiting_for_subtask_id: None,
                handover_summary: None,
                cover_image: None,
                attachments: tasks[parent_idx].attachments.clone(),
                todos: None,
                priority: parent_priority,
                tags: None,
                current_tokens: None,
                skip_worktree: parent_skip_wt,
                created_at: now,
                updated_at: now,
                stage_entered_at: Some(now),
                completed_at: None,
                position: Some(next_subtask_pos),
                hook_states: tasks[parent_idx].hook_states.clone(),
            };

            if has_autorun {
                autorun_task_ids.push(new_subtask_id.clone());
            }

            let parent_clone = tasks[parent_idx].clone();
            moved_events.push(serde_json::json!({
                "taskId": parent_id,
                "fromStage": &old_parent_stage,
                "toStage": &old_parent_stage,
                "stageId": &old_parent_stage,
                "status": "waiting_subtask",
                "waitingForSubtaskId": &new_subtask_id,
                "verdict": parent_clone.verdict,
                "task": &parent_clone
            }));

            created_tasks.push(new_subtask.clone());
            tasks.push(new_subtask);

            // Parent is now waiting on the newly created subtask; break recursion
            break;
        }
    }

    ParentAdvanceResult {
        moved_events,
        created_tasks,
        autorun_task_ids,
    }
}

#[allow(clippy::too_many_arguments)]
/// Unifies stage transition lifecycle execution (§ 2.2, § 2.6, § 2.7, § 2.27):
/// 1. `on_leave` / `on_exit` hook of the source column.
/// 2. `gatekeeper_sensor` validation of the target column.
/// 3. `on_enter` hook of the target column.
/// 4. Unified rejection handling and event broadcasting.
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

        let failed_task = task.clone();
        let _ = crate::db::db_upsert_task(self.root, &failed_task);

        let mut tasks = load_tasks_from_disk(self.root).await;
        if let Some(t) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &failed_task.id)) {
            t.verdict = failed_task.verdict.clone();
            t.status = failed_task.status.clone();
            t.updated_at = failed_task.updated_at;
        }
        let _ = save_tasks_to_disk(self.root, &tasks).await;

        if let Some(app) = self.app_handle {
            if let Some(state_ref) = app.try_state::<AppState>() {
                if let Ok(mut guard) = state_ref.tasks.lock() {
                    if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, &failed_task.id)) {
                        t.verdict = failed_task.verdict.clone();
                        t.status = failed_task.status.clone();
                        t.updated_at = failed_task.updated_at;
                    }
                }
            }
            if self.emit_task_moved {
                let _ = app.emit("task:moved", serde_json::json!({
                    "task": &failed_task,
                    "rejected": true,
                    "reason": reason_details
                }));
            }
            let _ = app.emit("task:updated", serde_json::json!({
                "taskId": failed_task.id,
                "task": &failed_task
            }));
            let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));

            if scol_has_autorun {
                let _ = app.emit("agent:autorun", serde_json::json!({
                    "taskId": failed_task.id,
                    "prompt": steer_content
                }));
            }
        }

        Err(error_msg)
    }

    /// Step: Execute source column `on_leave` / `on_exit` hook.
    pub async fn run_on_leave(&self, task: &mut Task) -> Result<(), String> {
        let sc = match self.source_col {
            Some(c) => c,
            None => return Ok(()),
        };
        let hook_cfg = match sc.hooks.get("on_leave").or_else(|| sc.hooks.get("on_exit")) {
            Some(cfg) => cfg,
            None => return Ok(()),
        };

        let hook_rel = hook_cfg.script();
        let hook_desc = hook_cfg.description();
        let hook_path = crate::config::resolve_hook_path(self.root, sc.persona.as_deref(), hook_rel);
        if !hook_path.exists() {
            return Ok(());
        }

        if let Some(app) = self.app_handle {
            let _ = app.emit("hook:start", serde_json::json!({
                "taskId": task.id,
                "stageId": sc.id,
                "hookType": "on_leave",
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
                        "on_leave",
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
                            "hookType": "on_leave",
                            "success": false,
                            "exitCode": res.exit_code,
                            "stdout": res.stdout,
                            "stderr": res.stderr,
                            "error": &err_details
                        }));
                    }

                    let steer_content = format!(
                        "### [ОТКЛОНЕНО ХУКОМ / HOOK REJECTED: {}]\n\nХук на выходе из колонки '{}' завершился с ошибкой (код {:?}).\n\n**Лог ошибки:**\n```\n{}\n```\n\nИсправь указанную ошибку.",
                        hook_rel, sc.id, res.exit_code, err_details
                    );
                    let err_msg = format!(
                        "Хук on_leave '{}' отклонил переход (Exit code: {:?}):\n{}",
                        hook_rel, res.exit_code, err_details
                    );
                    return self.handle_rejection(task, &steer_content, err_msg, &err_details).await;
                }

                record_task_hook_result(
                    task,
                    &sc.id,
                    "on_leave",
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
                        "hookType": "on_leave",
                        "success": true,
                        "exitCode": res.exit_code,
                        "stdout": res.stdout,
                        "stderr": res.stderr
                    }));
                }

                apply_worktree_from_hook_stdout(task, &res.stdout);
                let _ = crate::db::db_upsert_task(self.root, task);
                if let Some(app) = self.app_handle {
                    if let Some(state_ref) = app.try_state::<AppState>() {
                        if let Ok(mut guard) = state_ref.tasks.lock() {
                            if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, &task.id)) {
                                t.worktree_path = task.worktree_path.clone();
                                t.branch = task.branch.clone();
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
                    "on_leave",
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
                        "hookType": "on_leave",
                        "success": false,
                        "exitCode": None::<i32>,
                        "stdout": "",
                        "stderr": &err_details,
                        "error": &err_details
                    }));
                }

                let steer_content = format!(
                    "### [ОТКЛОНЕНО ХУКОМ / HOOK REJECTED: {}]\n\nОшибка запуска хука на выходе из колонки '{}': {}\n\nИсправь указанную ошибку.",
                    hook_rel, sc.id, err_details
                );
                let err_msg = format!("Хук on_leave '{}' ошибка исполнения: {}", hook_rel, err_details);
                self.handle_rejection(task, &steer_content, err_msg, &err_details).await
            }
        }
    }

    /// Step: Validate gatekeeper sensor of target column.
    pub async fn validate_gatekeeper(&self, task: &mut Task) -> Result<(), String> {
        let sensor_cmd = match &self.target_col.gatekeeper_sensor {
            Some(cmd) => cmd,
            None => return Ok(()),
        };

        let abs_wt_path = match task.worktree_path.as_deref() {
            Some(p) => {
                let pb = PathBuf::from(p);
                if pb.is_absolute() { pb } else { self.root.join(pb) }
            }
            None => self.root.to_path_buf(),
        };
        let effective_wt_path = if abs_wt_path.exists() {
            abs_wt_path.clone()
        } else {
            self.root.to_path_buf()
        };

        let wt_missing_or_empty = task.worktree_path.is_none()
            || !abs_wt_path.exists()
            || abs_wt_path.read_dir().map(|mut it| it.next().is_none()).unwrap_or(true);

        if self.is_source_intake {
            crate::logger::cvh_log(
                Some(self.root),
                "INFO",
                "GATEKEEPER",
                &format!(
                    "Skipping gatekeeper sensor '{}' for initial intake transition of task {} from '{}' to '{}'",
                    sensor_cmd, task.id, task.stage_id, self.target_col.id
                ),
            );
            return Ok(());
        }

        if wt_missing_or_empty {
            crate::logger::cvh_log(
                Some(self.root),
                "INFO",
                "GATEKEEPER",
                &format!(
                    "Skipping gatekeeper sensor '{}' for task {} because worktree does not exist or is empty",
                    sensor_cmd, task.id
                ),
            );
            return Ok(());
        }

        crate::logger::cvh_log(
            Some(self.root),
            "INFO",
            "GATEKEEPER",
            &format!("Executing gatekeeper sensor '{}' for task '{}' in {:?}", sensor_cmd, task.id, effective_wt_path),
        );

        let sensor_res = execute_sensor_command(sensor_cmd, &effective_wt_path, None).await;
        match sensor_res {
            Ok(res) => {
                if !res.success {
                    let stdout_trim = res.stdout.trim();
                    let stderr_trim = res.stderr.trim();
                    let mut details = String::new();
                    if !stdout_trim.is_empty() {
                        details.push_str(&format!("\n--- STDOUT ---\n{}", stdout_trim));
                    }
                    if !stderr_trim.is_empty() {
                        details.push_str(&format!("\n--- STDERR ---\n{}", stderr_trim));
                    }
                    if details.is_empty() {
                        details = "\n(No stdout/stderr produced by sensor command)".to_string();
                    }
                    crate::logger::cvh_log_with_details(
                        Some(self.root),
                        "WARN",
                        "GATEKEEPER",
                        &format!("Gatekeeper sensor '{}' REJECTED transition for task '{}' (exit code: {:?})", sensor_cmd, task.id, res.exit_code),
                        Some(&details),
                    );

                    let steer_content = format!(
                        "### [ОТКЛОНЕНО СЕНСОРОМ / SENSOR REJECTED: {}]\n\nGatekeeper-сенсор целевой колонки '{}' завершился с ошибкой (код {:?}).\n\n**Лог ошибки:**\n```\n{}\n```\n\nИсправь указанную ошибку.",
                        sensor_cmd, self.target_col.id, res.exit_code, details
                    );
                    let err_msg = format!(
                        "Gatekeeper Sensor '{}' rejected transition (Exit code: {:?}):{}",
                        sensor_cmd, res.exit_code, details
                    );
                    return self.handle_rejection(task, &steer_content, err_msg, &details).await;
                }
                crate::logger::cvh_log(
                    Some(self.root),
                    "INFO",
                    "GATEKEEPER",
                    &format!("Gatekeeper sensor '{}' PASSED for task '{}'", sensor_cmd, task.id),
                );
                Ok(())
            }
            Err(e) => {
                let err_details = e.to_string();
                crate::logger::cvh_log_with_details(
                    Some(self.root),
                    "WARN",
                    "GATEKEEPER",
                    &format!("Gatekeeper sensor '{}' EXECUTION FAILED for task '{}': {}", sensor_cmd, task.id, err_details),
                    Some(&err_details),
                );
                let steer_content = format!(
                    "### [ОТКЛОНЕНО СЕНСОРОМ / SENSOR REJECTED: {}]\n\nОшибка исполнения gatekeeper-сенсора целевой колонки '{}': {}\n\nИсправь указанную ошибку.",
                    sensor_cmd, self.target_col.id, err_details
                );
                let err_msg = format!("Gatekeeper Sensor '{}' execution failed: {}", sensor_cmd, err_details);
                self.handle_rejection(task, &steer_content, err_msg, &err_details).await
            }
        }
    }

    /// Step: Execute target column `on_enter` hook.
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
            return Ok(());
        }

        if let Some(app) = self.app_handle {
            let _ = app.emit("hook:start", serde_json::json!({
                "taskId": task.id,
                "stageId": self.target_col.id,
                "hookType": "on_enter",
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
                let _ = crate::db::db_upsert_task(self.root, task);
                if let Some(app) = self.app_handle {
                    if let Some(state_ref) = app.try_state::<AppState>() {
                        if let Ok(mut guard) = state_ref.tasks.lock() {
                            if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, &task.id)) {
                                t.worktree_path = task.worktree_path.clone();
                                t.branch = task.branch.clone();
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

    /// Master execution: run `on_leave` -> `validate_gatekeeper` -> `on_enter`.
    pub async fn execute(&self, task: &mut Task) -> Result<(), String> {
        self.run_on_leave(task).await?;
        self.validate_gatekeeper(task).await?;
        self.run_on_enter(task).await?;
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn internal_move_task_stage(
    root: &Path,
    task_id: &str,
    target_stage_id: &str,
    verdict: Option<String>,
    status_override: Option<String>,
    handover_summary: Option<String>,
    app_handle: Option<&AppHandle>,
    is_autonomous_advance: bool,
    board_id_override: Option<String>,
) -> Result<Task, String> {
    let actual_root = resolve_actual_repo_root(root);
    let root = &actual_root;
    crate::logger::cvh_log_with_details(
        Some(root),
        "INFO",
        "MOVE_STAGE",
        &format!(
            "internal_move_task_stage: task_id='{}', target_stage='{}', board_override={:?}, verdict={:?}, auto_adv={}",
            task_id, target_stage_id, board_id_override, verdict, is_autonomous_advance
        ),
        Some(&format!("actual_root='{}'", root.display())),
    );
    let mut tasks = load_tasks_from_disk(root).await;

    // In-memory fallback: if task was just created and not yet on disk,
    // or if memory has tasks not yet on disk, retrieve from AppState (§ 2.22).
    if !tasks.iter().any(|t| task_id_matches(&t.id, task_id)) {
        crate::logger::cvh_log(
            Some(root),
            "WARN",
            "MOVE_STAGE",
            &format!("Task '{}' not found in {} disk tasks. Checking in-memory AppState tasks...", task_id, tasks.len()),
        );
        if let Some(app) = app_handle {
            if let Some(state) = app.try_state::<AppState>() {
                if let Ok(guard) = state.tasks.lock() {
                    for mem_t in guard.iter() {
                        if !tasks.iter().any(|t| task_id_matches(&t.id, &mem_t.id)) {
                            tasks.push(mem_t.clone());
                        }
                    }
                }
            }
        }
    }

    let task_idx = match tasks.iter().position(|t| task_id_matches(&t.id, task_id)) {
        Some(idx) => idx,
        None => {
            let disk_ids: Vec<String> = tasks.iter().map(|t| t.id.clone()).collect();
            let err_msg = format!("Task {} not found", task_id);
            crate::logger::cvh_log_with_details(
                Some(root),
                "ERROR",
                "MOVE_STAGE",
                &format!("REJECTED move_task_stage: {}", err_msg),
                Some(&format!("Available tasks on disk/memory (count: {}): [{}]", disk_ids.len(), disk_ids.join(", "))),
            );
            return Err(err_msg);
        }
    };

    let current_task = &mut tasks[task_idx];

    // --- Board resolution & fallback (support alias and fallback when board_id missing/stale) ---
    // ZERO INVISIBLE TASKS (§ 2.18):
    // 1. If explicit board_id_override is provided and contains target stage, stay on it.
    // 2. Otherwise if current_task.board_id contains target stage, preserve it (NO accidental board hopping!).
    // 3. Otherwise if active_board contains target stage, use active_board.
    // 4. Only if no priority board contains the stage, search any other board with matching target stage.
    let boards_cfg = load_boards_config(root).await.ok();
    let mut effective_board: Option<crate::config::BoardDef> = None;

    if let Some(ref bc) = boards_cfg {
        let active_board = bc.boards.iter().find(|b| b.id == bc.active_board_id).cloned();
        let override_board = board_id_override.as_deref().and_then(|bid| {
            bc.boards.iter().find(|b| b.id == bid).cloned()
        });
        let current_board = current_task.board_id.as_deref().and_then(|bid| {
            bc.boards.iter().find(|b| b.id == bid).cloned()
        });

        let board_has_target = |b: &crate::config::BoardDef| -> bool {
            b.columns.iter().any(|c| c.id == target_stage_id || stage_id_matches(&c.id, target_stage_id))
        };

        // 1. Explicit override board requested by caller (e.g. active board during user move)
        if let Some(ref ob) = override_board {
            if board_has_target(ob) {
                effective_board = Some(ob.clone());
                current_task.board_id = Some(ob.id.clone());
            }
        }

        // 2. Task's current board if it contains the target stage (preserves task on its board)
        if effective_board.is_none() {
            if let Some(ref cb) = current_board {
                if board_has_target(cb) {
                    effective_board = Some(cb.clone());
                    // Keep current_task.board_id intact!
                }
            }
        }

        // 3. Active board if it contains the target stage
        if effective_board.is_none() {
            if let Some(ref ab) = active_board {
                if board_has_target(ab) {
                    effective_board = Some(ab.clone());
                    current_task.board_id = Some(ab.id.clone());
                }
            }
        }

        // 4. Any board that defines the target stage (cross-board pipeline advance)
        if effective_board.is_none() {
            let any_with_target = bc.boards.iter().find(|b| board_has_target(b)).cloned();
            if let Some(ref b) = any_with_target {
                effective_board = Some(b.clone());
                current_task.board_id = Some(b.id.clone());
            } else {
                effective_board = active_board.or_else(|| bc.boards.first().cloned());
                if let Some(ref b) = effective_board {
                    current_task.board_id = Some(b.id.clone());
                } else {
                    current_task.board_id = Some(bc.active_board_id.clone());
                }
            }
        }
    }

    // --- on_finish / on_leave hooks of source column ---
    let source_stage_id = current_task.stage_id.clone();
    let source_col: Option<crate::config::PipelineColumn> = {
        let from_board = effective_board.as_ref().and_then(|b| {
            b.columns.iter().find(|c| stage_id_matches(&c.id, &source_stage_id)).cloned()
        });
        match from_board {
            Some(c) => Some(c),
            None => load_pipeline_config(root)
                .await
                .ok()
                .and_then(|p| p.columns.into_iter().find(|c| stage_id_matches(&c.id, &source_stage_id))),
        }
    };

    let target_col = {
        let col_from_board = effective_board.as_ref().and_then(|b| {
            b.columns.iter().find(|c| stage_id_matches(&c.id, target_stage_id)).cloned()
        });
        match col_from_board {
            Some(c) => c,
            None => {
                let pipeline = load_pipeline_config(root)
                    .await
                    .unwrap_or_else(|_| crate::config::default_pipeline_config());
                let col_from_pipeline = pipeline
                    .columns
                    .iter()
                    .find(|c| stage_id_matches(&c.id, target_stage_id))
                    .cloned();

                match col_from_pipeline {
                    Some(c) => c,
                    None => {
                        let is_explicit_terminal_request = target_stage_id.trim().is_empty()
                            || stage_id_matches(target_stage_id, "ready")
                            || stage_id_matches(target_stage_id, "done")
                            || stage_id_matches(target_stage_id, "terminal");

                        let terminal_opt = if is_explicit_terminal_request {
                            effective_board.as_ref().and_then(|b| {
                                b.columns.iter().find(|c| c.terminal == Some(true) || stage_id_matches(&c.id, "ready") || stage_id_matches(&c.id, "done")).cloned()
                                    .or_else(|| b.columns.last().cloned())
                            }).or_else(|| {
                                pipeline.columns.iter().find(|c| {
                                    c.terminal == Some(true)
                                        || stage_id_matches(&c.id, "ready")
                                        || stage_id_matches(&c.id, "done")
                                }).cloned()
                            })
                        } else {
                            None
                        };

                        if let Some(term) = terminal_opt {
                            term
                        } else {
                            let mut available_stages: Vec<String> = Vec::new();
                            if let Some(ref b) = effective_board {
                                for c in &b.columns {
                                    if !available_stages.contains(&c.id) {
                                        available_stages.push(c.id.clone());
                                    }
                                }
                            }
                            for c in &pipeline.columns {
                                if !available_stages.contains(&c.id) {
                                    available_stages.push(c.id.clone());
                                }
                            }
                            return Err(format!(
                                "Target stage '{}' not found in active board or pipeline columns. Available: {:?}",
                                target_stage_id,
                                available_stages
                            ));
                        }
                    }
                }
            }
        }
    };
    let canonical_target_stage_id = target_col.id.clone();
    let is_backlog = target_col.is_intake == Some(true)
        || stage_id_matches(&canonical_target_stage_id, "backlog");
    let target_has_autorun = !is_backlog && (target_col.persona.is_some() || target_col.auto_run == Some(true));
    let was_running = current_task.status.as_deref() == Some("running");

    // Immediate Cancellation (§ 2 specifications):
    if !is_autonomous_advance && (is_backlog || !target_has_autorun || was_running) {
        if let Some(app) = app_handle {
            if let Some(state_ref) = app.try_state::<AppState>() {
                let _ = internal_abort_agent(root, task_id, state_ref.inner(), Some(app)).await;
            }
        }
    }

    let is_source_intake = {
        let cols = if let Some(b) = &effective_board {
            b.columns.clone()
        } else if let Some(bc) = &boards_cfg {
            bc.boards.iter().find(|b| b.id == bc.active_board_id).map(|b| b.columns.clone()).unwrap_or_default()
        } else if let Ok(pipe) = load_pipeline_config(root).await {
            pipe.columns
        } else {
            Vec::new()
        };
        source_stage_id.is_empty()
            || cols
                .iter()
                .position(|c| stage_id_matches(&c.id, &source_stage_id))
                .map(|idx| idx == 0 || cols[idx].is_intake == Some(true))
                .unwrap_or(false)
    };

    let pipeline = TaskTransitionPipeline::new(
        root,
        source_col.as_ref(),
        &target_col,
        is_source_intake,
        verdict.clone(),
        true, // emit_task_moved
        app_handle,
    );
    pipeline.execute(current_task).await?;

    // Trigger on_enter_script if configured on target column
    trigger_column_on_enter_script(
        &target_col,
        current_task,
        &canonical_target_stage_id,
        verdict.as_deref(),
        root,
    )
    .await;

    // Update task
    let old_stage = current_task.stage_id.clone();
    current_task.stage_id = canonical_target_stage_id.clone();
    if verdict.is_some() {
        current_task.verdict = verdict.clone();
    }
    if let Some(summary) = handover_summary {
        current_task.handover_summary = Some(summary);
    }
    let is_terminal_col = target_col.terminal == Some(true)
        || stage_id_matches(&canonical_target_stage_id, "ready")
        || stage_id_matches(&canonical_target_stage_id, "done");
    if is_backlog {
        current_task.status = Some("idle".to_string());
    } else if let Some(st) = status_override {
        current_task.status = Some(st);
    } else if is_terminal_col {
        current_task.status = Some("done".to_string());
    } else if target_has_autorun {
        current_task.status = Some("running".to_string());
    } else {
        current_task.status = Some("idle".to_string());
    }
    if is_terminal_col || current_task.status.as_deref() == Some("done") {
        if current_task.completed_at.is_none() {
            current_task.completed_at = Some(Utc::now().timestamp_millis());
        }
        current_task.waiting_for_subtask_id = None;
        auto_complete_task_todos(root, current_task, app_handle);
    } else {
        current_task.completed_at = None;
    }
    current_task.updated_at = Utc::now().timestamp_millis();
    if old_stage != canonical_target_stage_id || current_task.stage_entered_at.is_none() {
        current_task.stage_entered_at = Some(Utc::now().timestamp_millis());
    }

    if old_stage != canonical_target_stage_id {
        let cur_id = tasks[task_idx].id.clone();
        let max_pos = tasks
            .iter()
            .filter(|t| !task_id_matches(&t.id, &cur_id) && stage_id_matches(&t.stage_id, &canonical_target_stage_id))
            .filter_map(|t| t.position)
            .fold(0.0f64, f64::max);
        tasks[task_idx].position = Some(max_pos + 1000.0);
    } else if tasks[task_idx].position.is_none() {
        tasks[task_idx].position = Some(0.0);
    }

    let updated = tasks[task_idx].clone();
    let _ = crate::db::db_upsert_task(root, &updated);
    let parent_id_opt = updated.parent_id.clone();

    // If a top-level parent task moves to terminal, ensure all child subtasks are also marked done (§ 3)
    if is_terminal_col && parent_id_opt.is_none() {
        let cur_id = updated.id.clone();
        for t in tasks.iter_mut() {
            if task_id_matches(&t.id, &cur_id) {
                t.waiting_for_subtask_id = None;
                t.status = Some("done".to_string());
                if t.completed_at.is_none() {
                    t.completed_at = Some(Utc::now().timestamp_millis());
                }
                t.updated_at = Utc::now().timestamp_millis();
                auto_complete_task_todos(root, t, app_handle);
            }
            if t.parent_id.as_deref() == Some(&cur_id) {
                t.status = Some("done".to_string());
                if t.completed_at.is_none() {
                    t.completed_at = Some(Utc::now().timestamp_millis());
                }
                t.updated_at = Utc::now().timestamp_millis();
                auto_complete_task_todos(root, t, app_handle);
            }
        }
    }

    // Relational symmetry (§ 2.7 & § 2.22):
    let mut parent_moved_events: Vec<serde_json::Value> = Vec::new();
    let mut parent_created_tasks: Vec<Task> = Vec::new();
    let mut parent_autorun_ids: Vec<String> = Vec::new();

    if verdict.as_deref() == Some("CHANGES_REQUESTED") {
        // 1. When CHANGES_REQUESTED is emitted by a subtask, unlock the parent task in its development stage:
        if let Some(ref pid) = parent_id_opt {
            if let Some(parent_idx) = tasks.iter().position(|t| &t.id == pid) {
                if tasks[parent_idx].status.as_deref() == Some("waiting_subtask")
                    || tasks[parent_idx].waiting_for_subtask_id.as_deref() == Some(task_id)
                {
                    tasks[parent_idx].status = Some("idle".to_string());
                    tasks[parent_idx].verdict = Some("CHANGES_REQUESTED".to_string());
                    tasks[parent_idx].waiting_for_subtask_id = None;
                    tasks[parent_idx].updated_at = Utc::now().timestamp_millis();
                    let parent_clone = tasks[parent_idx].clone();
                    parent_moved_events.push(serde_json::json!({
                        "taskId": pid,
                        "fromStage": &parent_clone.stage_id,
                        "toStage": &parent_clone.stage_id,
                        "stageId": &parent_clone.stage_id,
                        "status": parent_clone.status,
                        "verdict": parent_clone.verdict,
                        "task": &parent_clone
                    }));
                }
            }
        }
    } else if is_terminal_col {
        // 2. POSITIVE APPROVAL / TERMINAL ADVANCE:
        // When a child subtask advances to a terminal column (ready/done),
        // propagate advancement to the parent task (and grandparents recursively)
        // advancing them past the subtask column into the next pipeline stage (§ 2.2, § 2.7 & § 2.22).
        let pipeline_opt = load_pipeline_config(root).await.ok();
        let parent_res = propagate_parent_subtask_advance(
            &mut tasks,
            task_id,
            &old_stage,
            verdict.as_deref(),
            boards_cfg.as_ref(),
            pipeline_opt.as_ref(),
        );
        parent_moved_events = parent_res.moved_events;
        parent_created_tasks = parent_res.created_tasks;
        parent_autorun_ids = parent_res.autorun_task_ids;

        for t in tasks.iter_mut() {
            if t.status.as_deref() == Some("done") {
                auto_complete_task_todos(root, t, app_handle);
            }
        }
    }

    if let Some(app) = app_handle {
        if let Some(state_ref) = app.try_state::<AppState>() {
            if let Ok(mut guard) = state_ref.tasks.lock() {
                for mem_t in guard.iter() {
                    if !tasks.iter().any(|t| task_id_matches(&t.id, &mem_t.id)) {
                        tasks.push(mem_t.clone());
                    }
                }
                *guard = tasks.clone();
            }
        }
    }
    for ctask in &parent_created_tasks {
        let _ = crate::db::db_upsert_task(root, ctask);
    }

    if let Err(e) = save_tasks_to_disk(root, &tasks).await {
        crate::logger::cvh_log(
            Some(root),
            "ERROR",
            "MOVE_STAGE",
            &format!("Failed to save tasks to disk after moving task '{}': {}", task_id, e),
        );
        return Err(e);
    }
    crate::logger::cvh_log_with_details(
        Some(root),
        "INFO",
        "MOVE_STAGE",
        &format!(
            "Task '{}' stage successfully updated from '{}' to '{}' (status: {:?}, board: {:?})",
            task_id, old_stage, canonical_target_stage_id, updated.status, updated.board_id
        ),
        Some(&format!("worktree={:?}, branch={:?}", updated.worktree_path, updated.branch)),
    );

    if let Some(app) = app_handle {
        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
        let _ = app.emit(
            "task:moved",
            serde_json::json!({
                "taskId": task_id,
                "fromStage": old_stage,
                "toStage": &canonical_target_stage_id,
                "stageId": &canonical_target_stage_id,
                "status": updated.status,
                "verdict": updated.verdict,
                "task": &updated
            }),
        );
        for ctask in &parent_created_tasks {
            let _ = app.emit("task:created", serde_json::json!({ "task": ctask }));
            let _ = app.emit("task:updated", serde_json::json!({ "task": ctask }));
        }
        for p_event in &parent_moved_events {
            let _ = app.emit("task:moved", p_event);
            if let Some(ptask) = p_event.get("task") {
                let _ = app.emit("task:updated", serde_json::json!({ "task": ptask }));
            }
        }
        if target_has_autorun {
            let _ = app.emit("agent:autorun", serde_json::json!({ "taskId": task_id }));
        }
        for auto_pid in parent_autorun_ids {
            let _ = app.emit("agent:autorun", serde_json::json!({ "taskId": auto_pid }));
        }
    }

    Ok(updated)
}

pub async fn move_task_stage(
    task_id: String,
    target_stage_id: String,
    verdict: Option<String>,
    handover_summary: Option<String>,
    board_id: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    let root = state.get_root();
    crate::logger::cvh_log_with_details(
        Some(&root),
        "INFO",
        "MOVE_STAGE",
        &format!(
            "move_task_stage IPC called: task_id='{}', target_stage='{}', board_id={:?}, verdict={:?}",
            task_id, target_stage_id, board_id, verdict
        ),
        Some(&format!("root='{}'", root.display())),
    );
    let is_intake = stage_id_matches(&target_stage_id, "backlog") || {
        let boards_cfg = crate::config::load_boards_config(&root).await.ok();
        boards_cfg.as_ref().is_some_and(|cfg| {
            let active = cfg.boards.iter().find(|b| b.id == cfg.active_board_id);
            active.is_some_and(|b| {
                b.columns.iter().find(|c| stage_id_matches(&c.id, &target_stage_id)).is_some_and(|c| c.is_intake == Some(true))
            })
        })
    };
    let is_running_or_active = {
        if let Ok(map) = state.active_agents.lock() {
            let tid_trimmed = task_id.trim();
            map.keys().any(|k| {
                task_id_matches(k, tid_trimmed)
                    || k.to_ascii_lowercase().starts_with(&format!("{}.", tid_trimmed.to_ascii_lowercase()))
                    || k.to_ascii_lowercase().starts_with(&format!("{}:sub:", tid_trimmed.to_ascii_lowercase()))
                    || k.to_ascii_lowercase().starts_with(&format!("{}.sub-", tid_trimmed.to_ascii_lowercase()))
            })
        } else {
            false
        }
    };

    if is_intake || is_running_or_active {
        let _ = internal_abort_agent(&root, &task_id, &state, Some(&app_handle)).await;
    }

    internal_move_task_stage(
        &root,
        &task_id,
        &target_stage_id,
        verdict,
        None,
        handover_summary,
        Some(&app_handle),
        false,
        board_id,
    )
    .await
}

pub async fn reorder_task(
    task_id: String,
    target_stage_id: String,
    new_index: usize,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    let root = state.get_root();
    let mut tasks = load_tasks_from_disk(&root).await;

    let task_idx = tasks
        .iter()
        .position(|t| task_id_matches(&t.id, &task_id))
        .ok_or_else(|| format!("Task {} not found", task_id))?;

    let old_stage_id = tasks[task_idx].stage_id.clone();
    let is_stage_change = !stage_id_matches(&old_stage_id, &target_stage_id);

    if is_stage_change {
        let _ = internal_move_task_stage(
            &root,
            &task_id,
            &target_stage_id,
            None,
            None,
            None,
            Some(&app_handle),
            false,
            None,
        )
        .await?;
        tasks = load_tasks_from_disk(&root).await;
    }

    let mut stage_tuples: Vec<(String, f64, i64)> = tasks
        .iter()
        .filter(|t| stage_id_matches(&t.stage_id, &target_stage_id) && !task_id_matches(&t.id, &task_id))
        .map(|t| (t.id.clone(), t.position.unwrap_or(0.0), t.created_at))
        .collect();

    stage_tuples.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.2.cmp(&b.2))
    });

    let mut ordered_ids: Vec<String> = stage_tuples.into_iter().map(|(id, _, _)| id).collect();
    let insert_pos = new_index.min(ordered_ids.len());
    ordered_ids.insert(insert_pos, task_id.clone());

    let now = Utc::now().timestamp_millis();
    let mut updated_task = None;

    for (idx, tid) in ordered_ids.iter().enumerate() {
        let new_pos = (idx as f64) * 1000.0;
        if let Some(t) = tasks.iter_mut().find(|t| task_id_matches(&t.id, tid)) {
            t.position = Some(new_pos);
            t.updated_at = now;
            let _ = crate::db::db_upsert_task(&root, t);
            if task_id_matches(&t.id, &task_id) {
                updated_task = Some(t.clone());
            }
        }
    }

    save_tasks_to_disk(&root, &tasks).await?;

    if let Ok(mut guard) = state.tasks.lock() {
        *guard = tasks.clone();
    }

    let _ = app_handle.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
    if let Some(ref ut) = updated_task {
        let _ = app_handle.emit("task:updated", serde_json::json!({ "taskId": ut.id, "task": ut }));
    }

    updated_task.ok_or_else(|| format!("Failed to reorder task {}", task_id))
}

pub async fn internal_advance_task(
    root: &Path,
    task_id: &str,
    summary: &str,
    app_handle: Option<&AppHandle>,
) -> Result<String, String> {
    internal_advance_task_to_stage(root, task_id, None, summary, app_handle).await
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
        crate::db::db_has_unfinished_todos(root, &task.id)
    };

    if !memory_had_unfinished && !db_had_unfinished {
        return false;
    }

    if let Some(ref mut todos) = task.todos {
        for td in todos.iter_mut() {
            if td.status != "completed" {
                td.status = "completed".to_string();
            }
        }
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
    let _ = crate::db::db_upsert_task(root, task);

    if let Some(app) = app_handle {
        if let Some(state_ref) = app.try_state::<AppState>() {
            if let Ok(mut guard) = state_ref.tasks.lock() {
                if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, &task.id)) {
                    t.todos = task.todos.clone();
                    t.updated_at = task.updated_at;
                }
            }
        }
        let _ = app.emit("task:updated", serde_json::json!({
            "taskId": &task.id,
            "todos": &task.todos,
            "task": &task,
        }));
    }

    true
}

pub async fn internal_advance_task_to_stage(
    root: &Path,
    task_id: &str,
    target_stage_override: Option<&str>,
    summary: &str,
    app_handle: Option<&AppHandle>,
) -> Result<String, String> {
    let root_resolved = resolve_actual_repo_root(root);
    let root = &root_resolved;
    let tasks = load_tasks_from_disk(root).await;
    let mut current_task = tasks
        .iter()
        .find(|t| task_id_matches(&t.id, task_id))
        .cloned()
        .ok_or_else(|| format!("Task {} not found", task_id))?;

    let boards_cfg = load_boards_config(root).await.ok();

    // Normal progression within same board or fallback boards
    let columns = {
        let from_board = boards_cfg.as_ref().and_then(|bc| {
            // 1. Try board referenced by current_task.board_id if it contains current stage
            if let Some(ref bid) = current_task.board_id {
                if let Some(b) = bc.boards.iter().find(|b| &b.id == bid) {
                    if b.columns.iter().any(|c| stage_id_matches(&c.id, &current_task.stage_id)) {
                        return Some(b.columns.clone());
                    }
                }
            }
            // 2. Try active board if it contains current stage
            if let Some(b) = bc.boards.iter().find(|b| b.id == bc.active_board_id) {
                if b.columns.iter().any(|c| stage_id_matches(&c.id, &current_task.stage_id)) {
                    return Some(b.columns.clone());
                }
            }
            // 3. Search any board in boards.json that contains current stage
            if let Some(b) = bc.boards.iter().find(|b| {
                b.columns.iter().any(|c| stage_id_matches(&c.id, &current_task.stage_id))
            }) {
                return Some(b.columns.clone());
            }
            // 4. Fallback to active board columns or first board
            bc.boards
                .iter()
                .find(|b| b.id == bc.active_board_id)
                .or_else(|| bc.boards.first())
                .map(|b| b.columns.clone())
        });
        match from_board {
            Some(cols) => {
                if cols.iter().any(|c| stage_id_matches(&c.id, &current_task.stage_id)) {
                    cols
                } else {
                    let pipeline = load_pipeline_config(root)
                        .await
                        .map_err(|e| e.to_string())?;
                    pipeline.columns
                }
            }
            None => {
                let pipeline = load_pipeline_config(root)
                    .await
                    .map_err(|e| e.to_string())?;
                pipeline.columns
            }
        }
    };

    let target_col = if let Some(target_stage_id) = target_stage_override {
        columns
            .iter()
            .find(|c| stage_id_matches(&c.id, target_stage_id))
            .cloned()
            .ok_or_else(|| {
                let available_stages: Vec<&str> = columns.iter().map(|c| c.id.as_str()).collect();
                format!(
                    "Target stage '{}' not found in columns. Available stages: {:?}",
                    target_stage_id,
                    available_stages
                )
            })?
    } else {
        let curr_idx = columns
            .iter()
            .position(|c| stage_id_matches(&c.id, &current_task.stage_id))
            .ok_or_else(|| {
                format!(
                    "Current column '{}' not found in pipeline",
                    current_task.stage_id
                )
            })?;

        if curr_idx + 1 >= columns.len() {
            return Err(format!(
                "Задача {} уже находится на последнем этапе пайплайна ({})",
                task_id, columns[curr_idx].title
            ));
        }

        columns[curr_idx + 1].clone()
    };

    // Universal Relay Progression (§ 2.2 & § 2.7):
    let is_terminal = target_col.terminal == Some(true)
        || stage_id_matches(&target_col.id, "ready")
        || stage_id_matches(&target_col.id, "done");

    let source_col = columns
        .iter()
        .find(|c| stage_id_matches(&c.id, &current_task.stage_id))
        .cloned();

    let is_source_intake = {
        let source_idx = columns
            .iter()
            .position(|c| stage_id_matches(&c.id, &current_task.stage_id));
        current_task.stage_id.is_empty()
            || source_idx == Some(0)
            || source_col
                .as_ref()
                .map(|c| c.is_intake == Some(true))
                .unwrap_or(false)
    };

    let pipeline = TaskTransitionPipeline::new(
        root,
        source_col.as_ref(),
        &target_col,
        is_source_intake,
        current_task.verdict.clone(),
        false, // emit_task_moved
        app_handle,
    );
    pipeline.execute(&mut current_task).await?;

    if let Some(ref pid) = current_task.parent_id {
        // CASE A: The task calling advance_task is a child subtask (e.g. CVH-009.1)
        let subtask_id = current_task.id.clone();
        let parent_id = pid.clone();

        let parent_task = tasks
            .iter()
            .find(|t| task_id_matches(&t.id, &parent_id))
            .cloned()
            .ok_or_else(|| format!("Parent task {} not found", parent_id))?;

        if is_terminal {
            // 1. Mark this subtask as done
            let mut tasks = load_tasks_from_disk(root).await;
            if let Some(sub) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &subtask_id)) {
                sub.status = Some("done".to_string());
                if sub.completed_at.is_none() {
                    sub.completed_at = Some(Utc::now().timestamp_millis());
                }
                sub.updated_at = Utc::now().timestamp_millis();
                auto_complete_task_todos(root, sub, app_handle);
                let _ = crate::db::db_upsert_task(root, sub);
            }
            if let Some(app) = app_handle {
                if let Some(state_ref) = app.try_state::<AppState>() {
                    if let Ok(mut guard) = state_ref.tasks.lock() {
                        if let Some(sub) = guard.iter_mut().find(|t| task_id_matches(&t.id, &subtask_id)) {
                            sub.status = Some("done".to_string());
                            if sub.completed_at.is_none() {
                                sub.completed_at = Some(Utc::now().timestamp_millis());
                            }
                            sub.updated_at = Utc::now().timestamp_millis();
                        }
                    }
                }
            }
            let _ = save_tasks_to_disk(root, &tasks).await;
            if let Some(app) = app_handle {
                if let Some(sub) = tasks.iter().find(|t| task_id_matches(&t.id, &subtask_id)) {
                    let _ = app.emit("task:updated", serde_json::json!({ "task": sub }));
                    let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
                }
            }

            let sub_summary_content = format!(
                "### Саммари этапа {}: {}\n\n{}",
                current_task.stage_id, current_task.title, summary
            );
            let _ = append_task_dialog_activity(root, &subtask_id, "summary", &sub_summary_content, app_handle).await;

            let updated_parent = internal_move_task_stage(
                root,
                &parent_id,
                &target_col.id,
                Some("APPROVED".to_string()),
                Some("done".to_string()),
                Some(summary.to_string()),
                app_handle,
                true,
                None,
            )
            .await?;

            let mut final_tasks = load_tasks_from_disk(root).await;
            for t in final_tasks.iter_mut() {
                if task_id_matches(&t.id, &parent_id) {
                    t.waiting_for_subtask_id = None;
                    t.status = Some("done".to_string());
                    if t.completed_at.is_none() {
                        t.completed_at = Some(Utc::now().timestamp_millis());
                    }
                    t.stage_id = target_col.id.clone();
                    if t.stage_entered_at.is_none() {
                        t.stage_entered_at = Some(Utc::now().timestamp_millis());
                    }
                    t.updated_at = Utc::now().timestamp_millis();
                    auto_complete_task_todos(root, t, app_handle);
                    let _ = crate::db::db_upsert_task(root, t);
                }
                if t.parent_id.as_deref() == Some(&parent_id) {
                    t.status = Some("done".to_string());
                    if t.completed_at.is_none() {
                        t.completed_at = Some(Utc::now().timestamp_millis());
                    }
                    t.updated_at = Utc::now().timestamp_millis();
                    auto_complete_task_todos(root, t, app_handle);
                    let _ = crate::db::db_upsert_task(root, t);
                }
            }
            let _ = save_tasks_to_disk(root, &final_tasks).await;
            if let Some(app) = app_handle {
                if let Some(state_ref) = app.try_state::<AppState>() {
                    if let Ok(mut guard) = state_ref.tasks.lock() {
                        *guard = final_tasks.clone();
                    }
                }
                if let Some(parent) = final_tasks.iter().find(|t| task_id_matches(&t.id, &parent_id)) {
                    let _ = app.emit("task:updated", serde_json::json!({ "task": parent }));
                }
                let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &final_tasks }));
                let _ = app.emit("task:advanced", serde_json::json!({
                    "taskId": &parent_id,
                    "sourceTaskId": &subtask_id,
                    "targetTaskId": &parent_id,
                    "sourceStageId": source_col.as_ref().map(|c| c.id.as_str()).unwrap_or(current_task.stage_id.as_str()),
                    "sourceStageTitle": source_col.as_ref().map(|c| c.title.as_str()).unwrap_or(current_task.stage_id.as_str()),
                    "targetStageId": &target_col.id,
                    "targetStageTitle": &target_col.title,
                    "isTerminal": true,
                    "summary": summary,
                }));
            }

            let parent_summary_content = format!(
                "### Саммари финализации задачи {}: {}\n\n{}",
                parent_id, target_col.title, summary
            );
            let _ = append_task_dialog_activity(root, &parent_id, "summary", &parent_summary_content, app_handle).await;

            return Ok(format!(
                "УСПЕШНО: Задача {} завершена в терминальной колонке '{}' ({}), статус: {}",
                parent_id,
                target_col.id,
                target_col.title,
                updated_parent.status.as_deref().unwrap_or("done")
            ));
        } else {
            let mut tasks = load_tasks_from_disk(root).await;
            let existing_target_sub = tasks
                .iter()
                .filter(|t| {
                    t.parent_id.as_deref().is_some_and(|p| task_id_matches(p, &parent_id))
                        && stage_id_matches(&t.stage_id, &target_col.id)
                        && !task_id_matches(&t.id, &subtask_id)
                        && (t.verdict.as_deref() == Some("CHANGES_REQUESTED") || t.status.as_deref() != Some("done"))
                })
                .max_by_key(|t| (t.subtask_index.unwrap_or(0), t.created_at))
                .cloned();

            if let Some(existing_sub) = existing_target_sub {
                let target_has_autorun = target_col.persona.is_some() || target_col.auto_run == Some(true);
                let next_status = if target_has_autorun { "running" } else { "idle" };

                // 1. Mark current subtask as done
                if let Some(sub) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &subtask_id)) {
                    sub.status = Some("done".to_string());
                    if sub.completed_at.is_none() {
                        sub.completed_at = Some(Utc::now().timestamp_millis());
                    }
                    sub.updated_at = Utc::now().timestamp_millis();
                    let _ = crate::db::db_upsert_task(root, sub);
                }

                // 2. Update parent task waiting_for_subtask_id
                if let Some(parent) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &parent_id)) {
                    parent.waiting_for_subtask_id = Some(existing_sub.id.clone());
                    parent.status = Some("waiting_subtask".to_string());
                    parent.updated_at = Utc::now().timestamp_millis();
                    let _ = crate::db::db_upsert_task(root, parent);
                }

                // 3. Reactivate existing target subtask
                if let Some(target_sub) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &existing_sub.id)) {
                    target_sub.status = Some(next_status.to_string());
                    target_sub.verdict = None;
                    target_sub.completed_at = None;
                    target_sub.handover_summary = Some(summary.to_string());
                    target_sub.updated_at = Utc::now().timestamp_millis();
                    let _ = crate::db::db_upsert_task(root, target_sub);
                }

                let _ = save_tasks_to_disk(root, &tasks).await;
                if let Some(app) = app_handle {
                    if let Some(state_ref) = app.try_state::<AppState>() {
                        if let Ok(mut guard) = state_ref.tasks.lock() {
                            *guard = tasks.clone();
                        }
                    }
                }

                let sub_summary_content = format!(
                    "### Саммари этапа {}: {}\n\n{}",
                    current_task.stage_id, current_task.title, summary
                );
                let _ = append_task_dialog_activity(root, &subtask_id, "summary", &sub_summary_content, app_handle).await;

                let parent_summary_content = format!(
                    "### Саммари этапа (передано на {} {}):\n\n{}",
                    existing_sub.id, target_col.title, summary
                );
                let _ = append_task_dialog_activity(root, &parent_id, "summary", &parent_summary_content, app_handle).await;

                let handover_prompt = format!(
                    "### [ОТЧЕТ ОБ ИСПРАВЛЕНИЯХ / HANDOVER SUMMARY]\n\nЗамечания исправлены. Отчет исполнителя:\n{}",
                    summary
                );
                let _ = append_task_dialog_activity(root, &existing_sub.id, "steer", &handover_prompt, app_handle).await;

                if let Some(app) = app_handle {
                    if let Some(sub) = tasks.iter().find(|t| task_id_matches(&t.id, &subtask_id)) {
                        let _ = app.emit("task:updated", serde_json::json!({ "task": sub }));
                    }
                    if let Some(parent) = tasks.iter().find(|t| task_id_matches(&t.id, &parent_id)) {
                        let _ = app.emit("task:updated", serde_json::json!({ "task": parent }));
                    }
                    if let Some(target_sub) = tasks.iter().find(|t| task_id_matches(&t.id, &existing_sub.id)) {
                        let _ = app.emit("task:updated", serde_json::json!({ "task": target_sub }));
                    }
                    let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
                    let _ = app.emit("task:advanced", serde_json::json!({
                        "taskId": &parent_id,
                        "sourceTaskId": &subtask_id,
                        "targetTaskId": &existing_sub.id,
                        "sourceStageId": source_col.as_ref().map(|c| c.id.as_str()).unwrap_or(current_task.stage_id.as_str()),
                        "sourceStageTitle": source_col.as_ref().map(|c| c.title.as_str()).unwrap_or(current_task.stage_id.as_str()),
                        "targetStageId": &target_col.id,
                        "targetStageTitle": &target_col.title,
                        "isTerminal": false,
                        "summary": summary,
                    }));

                    if target_has_autorun {
                        let _ = app.emit("agent:autorun", serde_json::json!({
                            "taskId": existing_sub.id,
                            "prompt": handover_prompt,
                        }));
                    }
                }

                return Ok(format!(
                    "УСПЕШНО: Возобновлена существующая подзадача {} ({}) для задачи {}",
                    existing_sub.id, target_col.title, parent_id
                ));
            } else {
                // 1. Mark this subtask as done
                if let Some(sub) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &subtask_id)) {
                    sub.status = Some("done".to_string());
                    if sub.completed_at.is_none() {
                        sub.completed_at = Some(Utc::now().timestamp_millis());
                    }
                    sub.updated_at = Utc::now().timestamp_millis();
                    let _ = crate::db::db_upsert_task(root, sub);
                }
                if let Some(app) = app_handle {
                    if let Some(state_ref) = app.try_state::<AppState>() {
                        if let Ok(mut guard) = state_ref.tasks.lock() {
                            if let Some(sub) = guard.iter_mut().find(|t| task_id_matches(&t.id, &subtask_id)) {
                                sub.status = Some("done".to_string());
                                if sub.completed_at.is_none() {
                                    sub.completed_at = Some(Utc::now().timestamp_millis());
                                }
                                sub.updated_at = Utc::now().timestamp_millis();
                            }
                        }
                    }
                }
                let _ = save_tasks_to_disk(root, &tasks).await;
                if let Some(app) = app_handle {
                    if let Some(sub) = tasks.iter().find(|t| task_id_matches(&t.id, &subtask_id)) {
                        let _ = app.emit("task:updated", serde_json::json!({ "task": sub }));
                        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
                    }
                }

                let sub_summary_content = format!(
                    "### Саммари этапа {}: {}\n\n{}",
                    current_task.stage_id, current_task.title, summary
                );
                let _ = append_task_dialog_activity(root, &subtask_id, "summary", &sub_summary_content, app_handle).await;

                let next_subtask = internal_create_subtask(
                    root,
                    &parent_id,
                    Some(&target_col.id),
                    Some(&format!("{} ({})", parent_task.title, target_col.title)),
                    Some(summary),
                    app_handle,
                )
                .await?;

                let next_subtask_id = next_subtask.id.clone();
                let parent_summary_content = format!(
                    "### Саммари этапа (передано на {} {}):\n\n{}",
                    next_subtask_id, target_col.title, summary
                );
                let _ = append_task_dialog_activity(root, &parent_id, "summary", &parent_summary_content, app_handle).await;

                if let Some(app) = app_handle {
                    let _ = app.emit("task:advanced", serde_json::json!({
                        "taskId": &parent_id,
                        "sourceTaskId": &subtask_id,
                        "targetTaskId": &next_subtask_id,
                        "sourceStageId": source_col.as_ref().map(|c| c.id.as_str()).unwrap_or(current_task.stage_id.as_str()),
                        "sourceStageTitle": source_col.as_ref().map(|c| c.title.as_str()).unwrap_or(current_task.stage_id.as_str()),
                        "targetStageId": &target_col.id,
                        "targetStageTitle": &target_col.title,
                        "isTerminal": false,
                        "summary": summary,
                    }));
                }

                return Ok(format!(
                    "УСПЕШНО: Создана подзадача {} ({}) для задачи {}",
                    next_subtask_id, target_col.title, parent_id
                ));
            }
        }
    }

    // CASE B: The task calling advance_task is a root parent task (e.g. CVH-009)
    if !is_terminal {
        // Strict guard against duplicate subtasks when subtask is already running:
        if let Some(ref existing_sub_id) = current_task.waiting_for_subtask_id {
            if let Some(existing_sub) = tasks.iter().find(|t| task_id_matches(&t.id, existing_sub_id)) {
                if existing_sub.status.as_deref() != Some("done")
                    && existing_sub.verdict.as_deref() != Some("CHANGES_REQUESTED")
                {
                    crate::logger::cvh_log(
                        Some(root),
                        "WARN",
                        "ADVANCE_TASK",
                        &format!(
                            "advance_task: Task '{}' already has active waiting_for_subtask_id '{}' (status: {:?})",
                            task_id, existing_sub_id, existing_sub.status
                        ),
                    );
                    return Ok(format!(
                        "Подзадача {} уже активна для задачи {}",
                        existing_sub_id, task_id
                    ));
                }
            }
        }
        if let Some(existing_sub) = tasks.iter().find(|t| {
            t.parent_id.as_deref().is_some_and(|p| task_id_matches(p, task_id))
                && stage_id_matches(&t.stage_id, &target_col.id)
                && t.status.as_deref() != Some("done")
                && t.verdict.as_deref() != Some("CHANGES_REQUESTED")
        }) {
            crate::logger::cvh_log(
                Some(root),
                "WARN",
                "ADVANCE_TASK",
                &format!(
                    "advance_task: Task '{}' already has active subtask '{}' in column '{}' (status: {:?})",
                    task_id, existing_sub.id, target_col.title, existing_sub.status
                ),
            );
            return Ok(format!(
                "Подзадача {} уже выполняется в колонке {}",
                existing_sub.id, target_col.title
            ));
        }

        let existing_target_sub = tasks
            .iter()
            .filter(|t| {
                t.parent_id.as_deref().is_some_and(|p| task_id_matches(p, task_id))
                    && stage_id_matches(&t.stage_id, &target_col.id)
                    && (t.verdict.as_deref() == Some("CHANGES_REQUESTED") || t.status.as_deref() != Some("done"))
            })
            .max_by_key(|t| (t.subtask_index.unwrap_or(0), t.created_at))
            .cloned();

        if let Some(existing_sub) = existing_target_sub {
            let target_has_autorun = target_col.persona.is_some() || target_col.auto_run == Some(true);
            let next_status = if target_has_autorun { "running" } else { "idle" };

            let mut tasks = load_tasks_from_disk(root).await;
            if let Some(parent) = tasks.iter_mut().find(|t| task_id_matches(&t.id, task_id)) {
                parent.waiting_for_subtask_id = Some(existing_sub.id.clone());
                parent.status = Some("waiting_subtask".to_string());
                parent.updated_at = Utc::now().timestamp_millis();
                let _ = crate::db::db_upsert_task(root, parent);
            }

            if let Some(target_sub) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &existing_sub.id)) {
                target_sub.status = Some(next_status.to_string());
                target_sub.verdict = None;
                target_sub.handover_summary = Some(summary.to_string());
                target_sub.updated_at = Utc::now().timestamp_millis();
                let _ = crate::db::db_upsert_task(root, target_sub);
            }

            let _ = save_tasks_to_disk(root, &tasks).await;
            if let Some(app) = app_handle {
                if let Some(state_ref) = app.try_state::<AppState>() {
                    if let Ok(mut guard) = state_ref.tasks.lock() {
                        *guard = tasks.clone();
                    }
                }
            }

            let parent_summary_content = format!(
                "### Саммари этапа (передано на {} {}):\n\n{}",
                existing_sub.id, target_col.title, summary
            );
            let _ = append_task_dialog_activity(root, task_id, "summary", &parent_summary_content, app_handle).await;

            let handover_prompt = format!(
                "### [ОТЧЕТ ОБ ИСПРАВЛЕНИЯХ / HANDOVER SUMMARY]\n\nЗамечания исправлены. Отчет исполнителя:\n{}",
                summary
            );
            let _ = append_task_dialog_activity(root, &existing_sub.id, "steer", &handover_prompt, app_handle).await;

            if let Some(app) = app_handle {
                if let Some(parent) = tasks.iter().find(|t| task_id_matches(&t.id, task_id)) {
                    let _ = app.emit("task:updated", serde_json::json!({ "task": parent }));
                }
                if let Some(target_sub) = tasks.iter().find(|t| task_id_matches(&t.id, &existing_sub.id)) {
                    let _ = app.emit("task:updated", serde_json::json!({ "task": target_sub }));
                }
                let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
                let _ = app.emit("task:advanced", serde_json::json!({
                    "taskId": task_id,
                    "sourceTaskId": task_id,
                    "targetTaskId": &existing_sub.id,
                    "sourceStageId": source_col.as_ref().map(|c| c.id.as_str()).unwrap_or(current_task.stage_id.as_str()),
                    "sourceStageTitle": source_col.as_ref().map(|c| c.title.as_str()).unwrap_or(current_task.stage_id.as_str()),
                    "targetStageId": &target_col.id,
                    "targetStageTitle": &target_col.title,
                    "isTerminal": false,
                    "summary": summary,
                }));

                if target_has_autorun {
                    let _ = app.emit("agent:autorun", serde_json::json!({
                        "taskId": existing_sub.id,
                        "prompt": handover_prompt,
                    }));
                }
            }

            return Ok(format!(
                "УСПЕШНО: Возобновлена существующая подзадача {} ({}) для задачи {}",
                existing_sub.id, target_col.title, task_id
            ));
        }

        let subtask = internal_create_subtask(
            root,
            task_id,
            Some(&target_col.id),
            Some(&format!("{} ({})", current_task.title, target_col.title)),
            Some(summary),
            app_handle,
        )
        .await?;

        let subtask_id = subtask.id.clone();

        let parent_summary_content = format!(
            "### Саммари этапа (передано на {} {}):\n\n{}",
            subtask_id, target_col.title, summary
        );
        let _ = append_task_dialog_activity(
            root,
            task_id,
            "summary",
            &parent_summary_content,
            app_handle,
        )
        .await;

        if let Some(app) = app_handle {
            let _ = app.emit("task:advanced", serde_json::json!({
                "taskId": task_id,
                "sourceTaskId": task_id,
                "targetTaskId": &subtask_id,
                "sourceStageId": source_col.as_ref().map(|c| c.id.as_str()).unwrap_or(current_task.stage_id.as_str()),
                "sourceStageTitle": source_col.as_ref().map(|c| c.title.as_str()).unwrap_or(current_task.stage_id.as_str()),
                "targetStageId": &target_col.id,
                "targetStageTitle": &target_col.title,
                "isTerminal": false,
                "summary": summary,
            }));
        }

        return Ok(format!(
            "УСПЕШНО: Создана подзадача {} ({}) для задачи {}",
            subtask_id, target_col.title, task_id
        ));
    }

    // Terminal column advance for parent task:
    let updated_task = internal_move_task_stage(
        root,
        task_id,
        &target_col.id,
        Some("APPROVED".to_string()),
        Some("done".to_string()),
        Some(summary.to_string()),
        app_handle,
        true, // is_autonomous_advance
        None,
    )
    .await?;

    let summary_content = format!("### Саммари этапа: {}\n\n{}", target_col.title, summary);
    let _ = append_task_dialog_activity(
        root,
        task_id,
        "summary",
        &summary_content,
        app_handle,
    )
    .await;

    if let Some(app) = app_handle {
        let _ = app.emit("task:advanced", serde_json::json!({
            "taskId": task_id,
            "sourceTaskId": task_id,
            "targetTaskId": task_id,
            "sourceStageId": source_col.as_ref().map(|c| c.id.as_str()).unwrap_or(current_task.stage_id.as_str()),
            "sourceStageTitle": source_col.as_ref().map(|c| c.title.as_str()).unwrap_or(current_task.stage_id.as_str()),
            "targetStageId": &target_col.id,
            "targetStageTitle": &target_col.title,
            "isTerminal": true,
            "summary": summary,
        }));
    }

    Ok(format!(
        "УСПЕШНО: Задача {} завершена в терминальной колонке '{}' ({}), статус: {}",
        task_id,
        target_col.id,
        target_col.title,
        updated_task.status.as_deref().unwrap_or("done")
    ))}

pub async fn advance_task(
    task_id: String,
    summary: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    let root = state.get_root();
    internal_advance_task(&root, &task_id, &summary, Some(&app)).await
}

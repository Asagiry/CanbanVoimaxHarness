//! Task Advancement & Autonomous Relay Progression (§ 2.2, § 2.6, § 2.7)

use std::path::Path;
use chrono::Utc;
use tauri::{AppHandle, Emitter, State};

use crate::commands::dialogs::append_task_dialog_activity;
use crate::commands::state::AppState;
use crate::config::{load_boards_config, resolve_actual_repo_root};
use crate::commands::tasks::healing::{stage_id_matches, task_id_matches};
use crate::commands::tasks::lifecycle::internal_create_subtask_with_hook_states;
use crate::commands::tasks::storage::try_load_tasks_from_disk;
use crate::commands::tasks::timing::freeze_stage_duration;
use super::transition_hooks::{
    auto_complete_task_todos, format_handover_steer_prompt, format_stage_summary_activity,
    sync_single_task_in_app_state, trigger_column_on_enter_script, upsert_single_task,
    TaskTransitionPipeline,
};
use super::advance_child::handle_child_subtask_advance;
use super::move_stage::internal_move_task_stage;
use super::resolve::resolve_advance_columns;

pub async fn internal_advance_task(
    root: &Path,
    task_id: &str,
    summary: &str,
    app_handle: Option<&AppHandle>,
) -> Result<String, String> {
    internal_advance_task_to_stage(root, task_id, None, summary, app_handle).await
}

pub async fn internal_advance_task_to_stage(
    root: &Path,
    task_id: &str,
    target_stage_override: Option<&str>,
    summary: &str,
    app_handle: Option<&AppHandle>,
) -> Result<String, String> {
    internal_advance_task_with_verdict(root, task_id, target_stage_override, None, summary, app_handle).await
}

pub async fn internal_advance_task_with_verdict(
    root: &Path,
    task_id: &str,
    target_stage_override: Option<&str>,
    verdict_override: Option<&str>,
    summary: &str,
    app_handle: Option<&AppHandle>,
) -> Result<String, String> {
    let root_resolved = resolve_actual_repo_root(root);
    let root = &root_resolved;
    let initial_tasks = try_load_tasks_from_disk(root).await?;
    let mut current_task = initial_tasks
        .iter()
        .find(|t| task_id_matches(&t.id, task_id))
        .cloned()
        .ok_or_else(|| format!("Task {} not found", task_id))?;
    drop(initial_tasks);

    let boards_cfg = load_boards_config(root).await.ok();
    let columns = resolve_advance_columns(root, &current_task, boards_cfg.as_ref()).await?;

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

    let is_review_stage = source_col
        .as_ref()
        .map(|c| c.persona.as_deref() == Some("reviewer") || stage_id_matches(&c.id, "review"))
        .unwrap_or(false);

    let effective_verdict = verdict_override
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| current_task.verdict.clone())
        .or_else(|| {
            if is_terminal || is_review_stage {
                Some("APPROVED".to_string())
            } else {
                None
            }
        });

    let pipeline = TaskTransitionPipeline::new(
        root,
        source_col.as_ref(),
        &target_col,
        is_source_intake,
        effective_verdict.clone(),
        false, // emit_task_moved
        app_handle,
    );
    pipeline.execute(&mut current_task).await?;
    if effective_verdict.is_some() {
        current_task.verdict = effective_verdict.clone();
    }
    upsert_single_task(root, &mut current_task)?;
    auto_complete_task_todos(root, &mut current_task, app_handle);

    // Legacy on_enter_script (col.on_enter_script) is a distinct field from
    // hooks.on_enter executed by the pipeline above — not a duplicate.
    trigger_column_on_enter_script(
        &target_col,
        &mut current_task,
        &target_col.id,
        effective_verdict.as_deref(),
        root,
        app_handle,
    )
    .await;

    // Reload fresh tasks from SQLite after all async hooks complete
    let mut tasks = try_load_tasks_from_disk(root).await?;

    if let Some(ref pid) = current_task.parent_id {
        return handle_child_subtask_advance(
            root,
            &current_task,
            &target_col,
            source_col.as_ref(),
            is_terminal,
            summary,
            effective_verdict.as_deref(),
            app_handle,
            &mut tasks,
            pid,
        )
        .await;
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
                    let parent_summary_content = format_stage_summary_activity(
                        &current_task.stage_id,
                        Some(&current_task.title),
                        summary,
                    );
                    let _ = append_task_dialog_activity(root, task_id, "summary", &parent_summary_content, app_handle).await;

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

            if let Some(parent) = tasks.iter_mut().find(|t| task_id_matches(&t.id, task_id)) {
                parent.waiting_for_subtask_id = Some(existing_sub.id.clone());
                parent.status = Some("done".to_string());
                freeze_stage_duration(parent, Utc::now().timestamp_millis());
                parent.updated_at = Utc::now().timestamp_millis();
                auto_complete_task_todos(root, parent, app_handle);
                let _ = upsert_single_task(root, parent);
                sync_single_task_in_app_state(root, parent, app_handle);
            }

            if let Some(target_sub) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &existing_sub.id)) {
                target_sub.status = Some(next_status.to_string());
                target_sub.verdict = None;
                target_sub.handover_summary = Some(summary.to_string());
                target_sub.updated_at = Utc::now().timestamp_millis();
                let _ = upsert_single_task(root, target_sub);
                sync_single_task_in_app_state(root, target_sub, app_handle);
            }

            let parent_summary_content = format_stage_summary_activity(
                &current_task.stage_id,
                Some(&current_task.title),
                summary,
            );
            let _ = append_task_dialog_activity(root, task_id, "summary", &parent_summary_content, app_handle).await;

            let handover_prompt = format_handover_steer_prompt(summary);
            if !target_has_autorun {
                let _ = append_task_dialog_activity(root, &existing_sub.id, "steer", &handover_prompt, app_handle).await;
            }

            if let Some(app) = app_handle {
                let project_path_str = root.to_string_lossy().to_string();
                if let Some(parent) = tasks.iter().find(|t| task_id_matches(&t.id, task_id)) {
                    let _ = app.emit("task:updated", serde_json::json!({ "task": parent, "projectPath": &project_path_str }));
                }
                if let Some(target_sub) = tasks.iter().find(|t| task_id_matches(&t.id, &existing_sub.id)) {
                    let _ = app.emit("task:updated", serde_json::json!({ "task": target_sub, "projectPath": &project_path_str }));
                }
                let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks, "projectPath": &project_path_str }));
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
                    "projectPath": &project_path_str,
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

        auto_complete_task_todos(root, &mut current_task, app_handle);
        let subtask = internal_create_subtask_with_hook_states(
            root,
            task_id,
            Some(&target_col.id),
            Some(&format!("{} ({})", current_task.title, target_col.title)),
            Some(summary),
            app_handle,
            current_task.hook_states.clone(),
        )
        .await?;

        let subtask_id = subtask.id.clone();

        let parent_summary_content = format_stage_summary_activity(
            &current_task.stage_id,
            Some(&current_task.title),
            summary,
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

    let summary_content = format_stage_summary_activity(&target_col.title, None, summary);
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
    ))
}

pub async fn advance_task(
    task_id: String,
    summary: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    let root = state.get_root();
    internal_advance_task(&root, &task_id, &summary, Some(&app)).await
}

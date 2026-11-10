//! Child Subtask Stage Advancement Logic (Case A: § 2.2 & § 2.7)

use std::path::Path;
use chrono::Utc;
use tauri::{AppHandle, Emitter};

use crate::agent::Task;
use crate::config::PipelineStage;
use crate::commands::dialogs::append_task_dialog_activity;
use crate::commands::tasks::healing::{stage_id_matches, task_id_matches};
use crate::commands::tasks::lifecycle::internal_create_subtask_with_hook_states;
use crate::commands::tasks::storage::load_tasks_from_disk;
use crate::commands::tasks::timing::freeze_stage_duration;
use super::transition_hooks::{
    auto_complete_task_todos, format_handover_steer_prompt, format_stage_summary_activity,
    sync_single_task_in_app_state, upsert_single_task,
};
use super::move_stage::internal_move_task_stage;

#[allow(clippy::too_many_arguments)]
pub async fn handle_child_subtask_advance(
    root: &Path,
    current_task: &Task,
    target_col: &PipelineStage,
    source_col: Option<&PipelineStage>,
    is_terminal: bool,
    summary: &str,
    effective_verdict: Option<&str>,
    app_handle: Option<&AppHandle>,
    tasks: &mut Vec<Task>,
    parent_id: &str,
) -> Result<String, String> {
    let subtask_id = current_task.id.clone();

    let parent_task = tasks
        .iter()
        .find(|t| task_id_matches(&t.id, parent_id))
        .cloned()
        .ok_or_else(|| format!("Parent task {} not found", parent_id))?;

    if is_terminal {
        // 1. Mark this subtask as done
        if let Some(sub) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &subtask_id)) {
            sub.status = Some("done".to_string());
            if sub.verdict.is_none() && effective_verdict.is_some() {
                sub.verdict = effective_verdict.map(|s| s.to_string());
            }
            if sub.completed_at.is_none() {
                sub.completed_at = Some(Utc::now().timestamp_millis());
            }
            sub.updated_at = Utc::now().timestamp_millis();
            auto_complete_task_todos(root, sub, app_handle);
            let _ = upsert_single_task(root, sub);
            sync_single_task_in_app_state(root, sub, app_handle);
        }
        if let Some(app) = app_handle {
            let project_path_str = root.to_string_lossy().to_string();
            if let Some(sub) = tasks.iter().find(|t| task_id_matches(&t.id, &subtask_id)) {
                let _ = app.emit("task:updated", serde_json::json!({ "task": sub, "projectPath": &project_path_str }));
            }
        }

        let sub_summary_content = format_stage_summary_activity(
            &current_task.stage_id,
            Some(&current_task.title),
            summary,
        );
        let _ = append_task_dialog_activity(root, &subtask_id, "summary", &sub_summary_content, app_handle).await;

        let updated_parent = internal_move_task_stage(
            root,
            parent_id,
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
            if task_id_matches(&t.id, parent_id) {
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
                let _ = upsert_single_task(root, t);
                sync_single_task_in_app_state(root, t, app_handle);
            }
            if t.parent_id.as_deref() == Some(parent_id) {
                t.status = Some("done".to_string());
                if t.completed_at.is_none() {
                    t.completed_at = Some(Utc::now().timestamp_millis());
                }
                t.updated_at = Utc::now().timestamp_millis();
                auto_complete_task_todos(root, t, app_handle);
                let _ = upsert_single_task(root, t);
                sync_single_task_in_app_state(root, t, app_handle);
            }
        }
        if let Some(app) = app_handle {
            let project_path_str = root.to_string_lossy().to_string();
            if let Some(parent) = final_tasks.iter().find(|t| task_id_matches(&t.id, parent_id)) {
                let _ = app.emit("task:updated", serde_json::json!({ "task": parent, "projectPath": &project_path_str }));
            }
            let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &final_tasks, "projectPath": &project_path_str }));
            let _ = app.emit("task:advanced", serde_json::json!({
                "taskId": parent_id,
                "sourceTaskId": &subtask_id,
                "targetTaskId": parent_id,
                "sourceStageId": source_col.as_ref().map(|c| c.id.as_str()).unwrap_or(current_task.stage_id.as_str()),
                "sourceStageTitle": source_col.as_ref().map(|c| c.title.as_str()).unwrap_or(current_task.stage_id.as_str()),
                "targetStageId": &target_col.id,
                "targetStageTitle": &target_col.title,
                "isTerminal": true,
                "summary": summary,
                "projectPath": &project_path_str,
            }));
        }

        return Ok(format!(
            "УСПЕШНО: Задача {} завершена в терминальной колонке '{}' ({}), статус: {}",
            parent_id,
            target_col.id,
            target_col.title,
            updated_parent.status.as_deref().unwrap_or("done")
        ));
    }

    let existing_target_sub = tasks
        .iter()
        .filter(|t| {
            t.parent_id.as_deref().is_some_and(|p| task_id_matches(p, parent_id))
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
            auto_complete_task_todos(root, sub, app_handle);
            let _ = upsert_single_task(root, sub);
            sync_single_task_in_app_state(root, sub, app_handle);
        }

        // 2. Update parent task waiting_for_subtask_id
        if let Some(parent) = tasks.iter_mut().find(|t| task_id_matches(&t.id, parent_id)) {
            parent.waiting_for_subtask_id = Some(existing_sub.id.clone());
            parent.status = Some("done".to_string());
            freeze_stage_duration(parent, Utc::now().timestamp_millis());
            parent.updated_at = Utc::now().timestamp_millis();
            let _ = upsert_single_task(root, parent);
            sync_single_task_in_app_state(root, parent, app_handle);
        }

        // 3. Reactivate existing target subtask
        if let Some(target_sub) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &existing_sub.id)) {
            target_sub.status = Some(next_status.to_string());
            target_sub.verdict = None;
            target_sub.completed_at = None;
            target_sub.handover_summary = Some(summary.to_string());
            target_sub.updated_at = Utc::now().timestamp_millis();
            let _ = upsert_single_task(root, target_sub);
            sync_single_task_in_app_state(root, target_sub, app_handle);
        }

        let sub_summary_content = format_stage_summary_activity(
            &current_task.stage_id,
            Some(&current_task.title),
            summary,
        );
        let _ = append_task_dialog_activity(root, &subtask_id, "summary", &sub_summary_content, app_handle).await;

        let handover_prompt = format_handover_steer_prompt(summary);
        if !target_has_autorun {
            let _ = append_task_dialog_activity(root, &existing_sub.id, "steer", &handover_prompt, app_handle).await;
        }

        if let Some(app) = app_handle {
            let project_path_str = root.to_string_lossy().to_string();
            if let Some(sub) = tasks.iter().find(|t| task_id_matches(&t.id, &subtask_id)) {
                let _ = app.emit("task:updated", serde_json::json!({ "task": sub, "projectPath": &project_path_str }));
            }
            if let Some(parent) = tasks.iter().find(|t| task_id_matches(&t.id, parent_id)) {
                let _ = app.emit("task:updated", serde_json::json!({ "task": parent, "projectPath": &project_path_str }));
            }
            if let Some(target_sub) = tasks.iter().find(|t| task_id_matches(&t.id, &existing_sub.id)) {
                let _ = app.emit("task:updated", serde_json::json!({ "task": target_sub, "projectPath": &project_path_str }));
            }
            let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks, "projectPath": &project_path_str }));
            let _ = app.emit("task:advanced", serde_json::json!({
                "taskId": parent_id,
                "sourceTaskId": &subtask_id,
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
                    "projectPath": &project_path_str,
                }));
            }
        }

        Ok(format!(
            "УСПЕШНО: Возобновлена существующая подзадача {} ({}) для задачи {}",
            existing_sub.id, target_col.title, parent_id
        ))
    } else {
        // 1. Mark this subtask as done
        if let Some(sub) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &subtask_id)) {
            sub.status = Some("done".to_string());
            if sub.completed_at.is_none() {
                sub.completed_at = Some(Utc::now().timestamp_millis());
            }
            sub.updated_at = Utc::now().timestamp_millis();
            auto_complete_task_todos(root, sub, app_handle);
            let _ = upsert_single_task(root, sub);
            sync_single_task_in_app_state(root, sub, app_handle);
        }
        if let Some(app) = app_handle {
            let project_path_str = root.to_string_lossy().to_string();
            if let Some(sub) = tasks.iter().find(|t| task_id_matches(&t.id, &subtask_id)) {
                let _ = app.emit("task:updated", serde_json::json!({ "task": sub, "projectPath": &project_path_str }));
            }
        }

        let sub_summary_content = format_stage_summary_activity(
            &current_task.stage_id,
            Some(&current_task.title),
            summary,
        );
        let _ = append_task_dialog_activity(root, &subtask_id, "summary", &sub_summary_content, app_handle).await;

        let next_subtask = internal_create_subtask_with_hook_states(
            root,
            parent_id,
            Some(&target_col.id),
            Some(&format!("{} ({})", parent_task.title, target_col.title)),
            Some(summary),
            app_handle,
            current_task.hook_states.clone(),
        )
        .await?;

        let next_subtask_id = next_subtask.id.clone();

        if let Some(app) = app_handle {
            let _ = app.emit("task:advanced", serde_json::json!({
                "taskId": parent_id,
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

        Ok(format!(
            "УСПЕШНО: Создана подзадача {} ({}) для задачи {}",
            next_subtask_id, target_col.title, parent_id
        ))
    }
}

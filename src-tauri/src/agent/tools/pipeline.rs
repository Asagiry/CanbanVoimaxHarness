//! Pipeline Operations Tool Handlers (advance_task, move_task, reject_task)
//! Extracted from agent.rs per ub-mode Clean Code & Command Pattern principles.

use anyhow::{bail, Context, Result};
use tauri::{Emitter, Manager};

use crate::agent::ToolContext;

pub async fn execute_advance_task(
    ctx: &ToolContext<'_>,
    args: &serde_json::Value,
) -> Result<String> {
    let repo_root = ctx.repo_root;
    let task_id = ctx.task_id;

    let summary = args
        .get("summary")
        .or_else(|| args.get("handover_summary"))
        .or_else(|| args.get("description"))
        .or_else(|| args.get("comment"))
        .or_else(|| args.get("message"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("Задача выполнена и передана на следующий этап");

    let target_stage = args
        .get("stage_id")
        .or_else(|| args.get("target_stage"))
        .or_else(|| args.get("column_id"))
        .or_else(|| args.get("target_stage_id"))
        .or_else(|| args.get("to_stage"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    let task_id = args
        .get("task_id")
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .or(task_id)
        .context("No active task ID for advance_task / move_task")?;

    crate::commands::internal_advance_task_to_stage(
        repo_root,
        task_id,
        target_stage,
        summary,
        ctx.app_handle.as_ref(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("{}", e))
}

pub async fn execute_reject_task(
    ctx: &ToolContext<'_>,
    args: &serde_json::Value,
) -> Result<String> {
    let repo_root = ctx.repo_root;
    let task_id = ctx.task_id;

    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .context("Missing required parameter 'reason'")?;

    let task_id = args
        .get("task_id")
        .or_else(|| args.get("taskId"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .or(task_id)
        .context("No active task ID for reject_task")?;

    // 1. Determine active board and find current column
    let tasks = crate::commands::load_tasks_from_disk(repo_root).await;
    let current_task = tasks
        .iter()
        .find(|t| crate::commands::task_id_matches(&t.id, task_id))
        .ok_or_else(|| anyhow::anyhow!("Task {} not found", task_id))?;

    let boards_cfg = crate::config::load_boards_config(repo_root).await.ok();
    let columns = {
        let from_board = boards_cfg.as_ref().and_then(|bc| {
            let b = if let Some(ref bid) = current_task.board_id {
                bc.boards.iter().find(|b| &b.id == bid)
            } else {
                bc.boards.iter().find(|b| b.id == bc.active_board_id)
            };
            b.map(|b| b.columns.clone())
        });
        match from_board {
            Some(cols) => cols,
            None => {
                let pipeline = crate::config::load_pipeline_config(repo_root)
                    .await
                    .context("Failed to load pipeline configuration")?;
                pipeline.columns
            }
        }
    };

    let curr_idx = columns
        .iter()
        .position(|c| c.id == current_task.stage_id)
        .ok_or_else(|| anyhow::anyhow!("Current column '{}' not found in pipeline", current_task.stage_id))?;

    if curr_idx == 0 {
        bail!("Задача {} уже находится в начальной колонке ({})", task_id, columns[curr_idx].title);
    }

    let prev_col = &columns[curr_idx - 1];

    // ЖЕЛЕЗНЫЙ ИНВАРИАНТ: Если предыдущая колонка — колонка приёма/бэклог (индекс 0 или is_intake),
    // шаг назад в бэклог СТРОГО ЗАПРЕЩЁН! Возвращай ошибку:
    // "Запрещено возвращать задачу из первой рабочей стадии назад в бэклог. Задача должна быть доработана или закрыта."
    let is_prev_backlog = prev_col.is_intake == Some(true)
        || curr_idx == 1
        || prev_col.id == "backlog"
        || prev_col.title.to_lowercase().contains("backlog")
        || prev_col.title.to_lowercase().contains("бэклог");

    if is_prev_backlog {
        bail!("Запрещено возвращать задачу из первой рабочей стадии назад в бэклог. Задача должна быть доработана или закрыта.");
    }

    let reject_content = format!(
        "### Задача возвращена на этап: {}\n\n**Причина отклонения:**\n{}",
        prev_col.title,
        reason
    );

    // If current task is a subtask, find the previous subtask in prev_col and reactivate it
    let subtask_rejection = if let Some(ref pid) = current_task.parent_id {
        let prev_subtask_opt = tasks
            .iter()
            .filter(|t| {
                t.parent_id.as_deref().is_some_and(|p| crate::commands::task_id_matches(p, pid))
                    && crate::commands::stage_id_matches(&t.stage_id, &prev_col.id)
                    && !crate::commands::task_id_matches(&t.id, task_id)
            })
            .max_by_key(|t| (t.subtask_index.unwrap_or(0), t.created_at))
            .cloned();

        if let Some(prev_sub) = prev_subtask_opt {
            let mut mut_tasks = tasks.clone();
            let prev_col_has_autorun = prev_col.persona.is_some() || prev_col.auto_run == Some(true);
            let prev_status = if prev_col_has_autorun { "running" } else { "idle" };

            // 1. Reactivate prev_subtask in prev_col
            if let Some(ps) = mut_tasks.iter_mut().find(|t| crate::commands::task_id_matches(&t.id, &prev_sub.id)) {
                ps.status = Some(prev_status.to_string());
                ps.verdict = Some("CHANGES_REQUESTED".to_string());
                ps.handover_summary = Some(format!("ОТКЛОНЕНО: {}", reason));
                ps.updated_at = chrono::Utc::now().timestamp_millis();
                let _ = crate::db::db_upsert_task(repo_root, ps);
            }

            // 2. Keep rejecting subtask in its stage with status idle and verdict CHANGES_REQUESTED
            if let Some(cs) = mut_tasks.iter_mut().find(|t| crate::commands::task_id_matches(&t.id, task_id)) {
                cs.status = Some("idle".to_string());
                cs.verdict = Some("CHANGES_REQUESTED".to_string());
                cs.updated_at = chrono::Utc::now().timestamp_millis();
                let _ = crate::db::db_upsert_task(repo_root, cs);
            }

            // 3. Update parent task waiting_for_subtask_id
            if let Some(p) = mut_tasks.iter_mut().find(|t| crate::commands::task_id_matches(&t.id, pid)) {
                p.waiting_for_subtask_id = Some(prev_sub.id.clone());
                p.status = Some("waiting_subtask".to_string());
                p.updated_at = chrono::Utc::now().timestamp_millis();
                let _ = crate::db::db_upsert_task(repo_root, p);
            }

            // Save to disk and AppState
            let _ = crate::commands::save_tasks_to_disk(repo_root, &mut_tasks).await;
            if let Some(app) = ctx.app_handle.as_ref() {
                if let Some(state_ref) = app.try_state::<crate::commands::AppState>() {
                    if let Ok(mut guard) = state_ref.tasks.lock() {
                        *guard = mut_tasks.clone();
                    }
                }
            }

            // Append dialog activities
            let _ = crate::commands::append_task_dialog_activity(
                repo_root,
                task_id,
                "reject",
                &reject_content,
                ctx.app_handle.as_ref(),
            )
            .await;

            let _ = crate::commands::append_task_dialog_activity(
                repo_root,
                &prev_sub.id,
                "steer",
                &reject_content,
                ctx.app_handle.as_ref(),
            )
            .await;

            // Emit events
            if let Some(app) = ctx.app_handle.as_ref() {
                if let Some(cs) = mut_tasks.iter().find(|t| crate::commands::task_id_matches(&t.id, task_id)) {
                    let _ = app.emit("task:updated", serde_json::json!({ "task": cs }));
                }
                if let Some(ps) = mut_tasks.iter().find(|t| crate::commands::task_id_matches(&t.id, &prev_sub.id)) {
                    let _ = app.emit("task:updated", serde_json::json!({ "task": ps }));
                    let _ = app.emit("task:moved", serde_json::json!({
                        "taskId": ps.id,
                        "toStage": prev_col.id,
                        "stageId": prev_col.id,
                        "status": prev_status,
                        "rejected": true,
                        "reason": reason,
                        "verdict": "CHANGES_REQUESTED",
                        "task": ps,
                    }));
                }
                if let Some(p) = mut_tasks.iter().find(|t| crate::commands::task_id_matches(&t.id, pid)) {
                    let _ = app.emit("task:updated", serde_json::json!({ "task": p }));
                }
                let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &mut_tasks }));

                if prev_col_has_autorun {
                    let _ = app.emit("agent:autorun", serde_json::json!({
                        "taskId": prev_sub.id,
                        "prompt": reject_content,
                    }));
                }
            }

            Some(format!(
                "ОТКЛОНЕНО: Задача {} возвращена на этап '{}' ({}) к подзадаче {} по причине: {}",
                task_id,
                prev_col.title,
                prev_col.id,
                prev_sub.id,
                reason
            ))
        } else {
            None
        }
    } else {
        None
    };

    if let Some(res_str) = subtask_rejection {
        Ok(res_str)
    } else {
        // Fallback for single tasks or subtasks without previous subtask
        let rejection_summary = format!("ОТКЛОНЕНО РЕВЬЮ: {}", reason);
        let updated_task = crate::commands::internal_move_task_stage(
            repo_root,
            task_id,
            &prev_col.id,
            Some("CHANGES_REQUESTED".to_string()),
            Some("idle".to_string()),
            Some(rejection_summary),
            ctx.app_handle.as_ref(),
            true, // is_autonomous_advance
            None,
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

        let _ = crate::commands::append_task_dialog_activity(
            repo_root,
            task_id,
            "reject",
            &reject_content,
            ctx.app_handle.as_ref(),
        )
        .await;

        // Emit task:moved with reject marker and reason
        if let Some(app) = ctx.app_handle.as_ref() {
            let _ = app.emit("task:moved", serde_json::json!({
                "taskId": task_id,
                "toStage": prev_col.id,
                "stageId": prev_col.id,
                "status": "idle",
                "rejected": true,
                "reason": reason,
                "verdict": "CHANGES_REQUESTED",
                "task": &updated_task,
            }));
            let prev_has_autorun = prev_col.persona.is_some() || prev_col.auto_run == Some(true);
            if prev_has_autorun {
                let _ = app.emit("agent:autorun", serde_json::json!({
                    "taskId": task_id,
                    "prompt": reject_content,
                }));
            }
        }

        Ok(format!(
            "ОТКЛОНЕНО: Задача {} возвращена на этап '{}' ({}) по причине: {}",
            task_id,
            prev_col.title,
            prev_col.id,
            reason
        ))
    }
}

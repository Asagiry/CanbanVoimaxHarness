use anyhow::{Context, Result};
use tauri::{Emitter, Manager};

use super::params::*;
use crate::agent::ToolContext;

pub async fn execute_complete_task(
    ctx: &ToolContext<'_>,
    args: &serde_json::Value,
) -> Result<String> {
    let repo_root = ctx.repo_root;
    let task_id = ctx.task_id;

    let summary = args
        .get("summary")
        .and_then(|v| v.as_str())
        .context("Missing required parameter 'summary'")?;

    let current_task_id = get_str_param(args, TASK_ID_KEYS)
        .or(task_id)
        .context("No active task ID for complete_task")?;

    let mut tasks = crate::commands::load_tasks_from_disk(repo_root).await;
    let current_task_idx = tasks
        .iter()
        .position(|t| crate::commands::task_id_matches(&t.id, current_task_id))
        .ok_or_else(|| anyhow::anyhow!("Task {} not found", current_task_id))?;

    let current_task = &tasks[current_task_idx];

    let boards_cfg = crate::config::load_boards_config(repo_root).await.ok();
    let board_cols = boards_cfg.as_ref().and_then(|bc| {
        let b = if let Some(ref bid) = current_task.board_id {
            bc.boards.iter().find(|b| &b.id == bid)
        } else {
            bc.boards.iter().find(|b| b.id == bc.active_board_id)
        };
        b.map(|b| b.stages.clone())
    });

    // Find terminal column (ready / done)
    let columns = match board_cols {
        Some(cols) => cols,
        None => {
            let pipeline = crate::config::load_pipeline_config(repo_root).await
                .context("Failed to load pipeline configuration")?;
            pipeline.stages
        }
    };

    let terminal_col = columns
        .iter()
        .find(|c| {
            c.terminal == Some(true)
                || crate::commands::tasks::stage_id_matches(&c.id, "ready")
                || crate::commands::tasks::stage_id_matches(&c.id, "done")
        })
        .or_else(|| columns.last())
        .ok_or_else(|| anyhow::anyhow!("No terminal column found in board/pipeline"))?;
    let terminal_col_id = terminal_col.id.clone();
    let old_stage = tasks[current_task_idx].stage_id.clone();

    tasks[current_task_idx].stage_id = terminal_col_id.clone();
    tasks[current_task_idx].status = Some("done".to_string());
    tasks[current_task_idx].verdict = Some("APPROVED".to_string());
    tasks[current_task_idx].handover_summary = Some(summary.to_string());
    tasks[current_task_idx].updated_at = chrono::Utc::now().timestamp_millis();
    let updated_task = tasks[current_task_idx].clone();

    // In-memory state sync
    if let Some(app) = ctx.app_handle.as_ref() {
        if let Some(state_ref) = app.try_state::<crate::commands::state::AppState>() {
            if let Ok(mut guard) = state_ref.tasks.lock() {
                *guard = tasks.clone();
            }
        }
    }

    // Persist to disk
    crate::commands::tasks::save_tasks_to_disk(repo_root, &tasks)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Emit events
    if let Some(app) = ctx.app_handle.as_ref() {
        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
        let _ = app.emit(
            "task:moved",
            serde_json::json!({
                "taskId": current_task_id,
                "fromStage": old_stage,
                "toStage": &terminal_col_id,
                "stageId": &terminal_col_id,
                "status": "done",
                "verdict": "APPROVED",
                "task": &updated_task
            }),
        );
    }

    // Append dialog activity in SQLite
    let _ = crate::commands::dialogs::append_task_dialog_activity(
        repo_root,
        current_task_id,
        "orchestration_complete",
        &format!("### Оркестратор завершил задачу\n\n**Итоговое саммари декомпозиции:**\n{}", summary),
        ctx.app_handle.as_ref(),
    ).await;

    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "task_id": current_task_id,
        "title": updated_task.title,
        "column_id": terminal_col_id,
        "status": "done",
        "handover_summary": summary
    }))?)
}

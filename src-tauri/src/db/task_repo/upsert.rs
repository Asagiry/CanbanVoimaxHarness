use crate::agent::Task;
use crate::db::get_db_conn;
use rusqlite::{params, TransactionBehavior};
use std::path::Path;

/// Atomically upserts a single task and its checklist items into SQLite.
pub fn db_upsert_task(root: &Path, task: &Task) -> Result<(), String> {
    let mut conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    tx.execute_batch("PRAGMA defer_foreign_keys = ON;")
        .map_err(|e| format!("Failed to defer foreign keys: {}", e))?;

    let custom_flow_json = task.custom_flow.as_ref().and_then(|v| serde_json::to_string(v).ok());
    let attachments_json = task.attachments.as_ref().and_then(|v| serde_json::to_string(v).ok());
    let tags_json = task.tags.as_ref().and_then(|v| serde_json::to_string(v).ok());
    let hook_states_json = task.hook_states.as_ref().and_then(|v| serde_json::to_string(v).ok());
    let stage_durations_json = task.stage_durations.as_ref().and_then(|v| serde_json::to_string(v).ok());
    let skip_wt = if task.skip_worktree.unwrap_or(false) { 1i64 } else { 0i64 };
    let cur_tokens = task.current_tokens.map(|v| v as i64);
    let sub_idx = task.subtask_index.map(|v| v as i64);
    let pos_val = task.position.unwrap_or(0.0);

    tx.execute(
        "INSERT INTO tasks (
            id, title, description, stage_id, board_id, worktree_path, branch, verdict,
            custom_flow, status, model, provider_id, reasoning_effort, parent_id, subtask_index, waiting_for_subtask_id,
            handover_summary, cover_image, priority, current_tokens, skip_worktree,
            created_at, updated_at, completed_at, attachments, tags, position, hook_states,
            stage_entered_at, stage_durations, total_elapsed
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?30, ?31, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, COALESCE(?29, 0))
        ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            description = excluded.description,
            stage_id = excluded.stage_id,
            board_id = excluded.board_id,
            worktree_path = excluded.worktree_path,
            branch = excluded.branch,
            verdict = excluded.verdict,
            custom_flow = excluded.custom_flow,
            status = excluded.status,
            model = excluded.model,
            provider_id = excluded.provider_id,
            reasoning_effort = excluded.reasoning_effort,
            parent_id = excluded.parent_id,
            subtask_index = excluded.subtask_index,
            waiting_for_subtask_id = excluded.waiting_for_subtask_id,
            handover_summary = excluded.handover_summary,
            cover_image = excluded.cover_image,
            priority = excluded.priority,
            current_tokens = excluded.current_tokens,
            skip_worktree = excluded.skip_worktree,
            updated_at = excluded.updated_at,
            completed_at = excluded.completed_at,
            attachments = excluded.attachments,
            tags = excluded.tags,
            position = excluded.position,
            hook_states = excluded.hook_states,
            stage_entered_at = COALESCE(excluded.stage_entered_at, tasks.stage_entered_at, excluded.created_at),
            stage_durations = COALESCE(excluded.stage_durations, tasks.stage_durations),
            total_elapsed = COALESCE(excluded.total_elapsed, tasks.total_elapsed)",
        params![
            task.id,
            task.title,
            task.description,
            task.stage_id,
            task.board_id.as_deref().unwrap_or("development"),
            task.worktree_path,
            task.branch,
            task.verdict,
            custom_flow_json,
            task.status.as_deref().unwrap_or("idle"),
            task.model,
            task.parent_id,
            sub_idx,
            task.waiting_for_subtask_id,
            task.handover_summary,
            task.cover_image,
            task.priority,
            cur_tokens,
            skip_wt,
            task.created_at,
            task.updated_at,
            task.completed_at,
            attachments_json,
            tags_json,
            pos_val,
            hook_states_json,
            task.stage_entered_at,
            stage_durations_json,
            task.total_elapsed,
            task.provider_id,
            task.reasoning_effort,
        ],
    )
    .map_err(|e| format!("Failed to upsert task {}: {}", task.id, e))?;

    if let Some(ref todos) = task.todos {
        tx.execute("DELETE FROM task_todos WHERE task_id = ?1", params![task.id])
            .map_err(|e| format!("Failed to delete old todos for task {}: {}", task.id, e))?;
        for (idx, td) in todos.iter().enumerate() {
            tx.execute(
                "INSERT INTO task_todos (id, task_id, content, status, order_idx)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![td.id, task.id, td.content, td.status, idx as i64],
            )
            .map_err(|e| format!("Failed to insert todo {}: {}", td.id, e))?;
        }
    }

    tx.commit()
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;
    Ok(())
}

/// Atomically upserts a batch of tasks into SQLite in a single transaction.
pub fn db_upsert_all_tasks(root: &Path, tasks: &[Task]) -> Result<(), String> {
    let mut conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    tx.execute_batch("PRAGMA defer_foreign_keys = ON;")
        .map_err(|e| format!("Failed to defer foreign keys: {}", e))?;

    for task in tasks {
        let custom_flow_json = task.custom_flow.as_ref().and_then(|v| serde_json::to_string(v).ok());
        let attachments_json = task.attachments.as_ref().and_then(|v| serde_json::to_string(v).ok());
        let tags_json = task.tags.as_ref().and_then(|v| serde_json::to_string(v).ok());
        let hook_states_json = task.hook_states.as_ref().and_then(|v| serde_json::to_string(v).ok());
        let stage_durations_json = task.stage_durations.as_ref().and_then(|v| serde_json::to_string(v).ok());
        let skip_wt = if task.skip_worktree.unwrap_or(false) { 1i64 } else { 0i64 };
        let cur_tokens = task.current_tokens.map(|v| v as i64);
        let sub_idx = task.subtask_index.map(|v| v as i64);
        let pos_val = task.position.unwrap_or(0.0);

        tx.execute(
            "INSERT INTO tasks (
                id, title, description, stage_id, board_id, worktree_path, branch, verdict,
                custom_flow, status, model, provider_id, reasoning_effort, parent_id, subtask_index, waiting_for_subtask_id,
                handover_summary, cover_image, priority, current_tokens, skip_worktree,
                created_at, updated_at, completed_at, attachments, tags, position, hook_states,
                stage_entered_at, stage_durations, total_elapsed
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?30, ?31, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, COALESCE(?29, 0))
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                description = excluded.description,
                stage_id = excluded.stage_id,
                board_id = excluded.board_id,
                worktree_path = excluded.worktree_path,
                branch = excluded.branch,
                verdict = excluded.verdict,
                custom_flow = excluded.custom_flow,
                status = excluded.status,
                model = excluded.model,
                provider_id = excluded.provider_id,
                reasoning_effort = excluded.reasoning_effort,
                parent_id = excluded.parent_id,
                subtask_index = excluded.subtask_index,
                waiting_for_subtask_id = excluded.waiting_for_subtask_id,
                handover_summary = excluded.handover_summary,
                cover_image = excluded.cover_image,
                priority = excluded.priority,
                current_tokens = excluded.current_tokens,
                skip_worktree = excluded.skip_worktree,
                updated_at = excluded.updated_at,
                completed_at = excluded.completed_at,
                attachments = excluded.attachments,
                tags = excluded.tags,
                position = excluded.position,
                hook_states = excluded.hook_states,
                stage_entered_at = COALESCE(excluded.stage_entered_at, tasks.stage_entered_at, excluded.created_at),
                stage_durations = COALESCE(excluded.stage_durations, tasks.stage_durations),
                total_elapsed = COALESCE(excluded.total_elapsed, tasks.total_elapsed)",
            params![
                task.id,
                task.title,
                task.description,
                task.stage_id,
                task.board_id.as_deref().unwrap_or("development"),
                task.worktree_path,
                task.branch,
                task.verdict,
                custom_flow_json,
                task.status.as_deref().unwrap_or("idle"),
                task.model,
                task.parent_id,
                sub_idx,
                task.waiting_for_subtask_id,
                task.handover_summary,
                task.cover_image,
                task.priority,
                cur_tokens,
                skip_wt,
                task.created_at,
                task.updated_at,
                task.completed_at,
                attachments_json,
                tags_json,
                pos_val,
                hook_states_json,
                task.stage_entered_at,
                stage_durations_json,
                task.total_elapsed,
                task.provider_id,
                task.reasoning_effort,
            ],
        )
        .map_err(|e| format!("Failed to upsert task {}: {}", task.id, e))?;

        if let Some(ref todos) = task.todos {
            tx.execute("DELETE FROM task_todos WHERE task_id = ?1", params![task.id])
                .map_err(|e| format!("Failed to clean todos for task {}: {}", task.id, e))?;
            for (idx, td) in todos.iter().enumerate() {
                tx.execute(
                    "INSERT INTO task_todos (id, task_id, content, status, order_idx)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![td.id, task.id, td.content, td.status, idx as i64],
                )
                .map_err(|e| format!("Failed to insert todo {}: {}", td.id, e))?;
            }
        }
    }

    tx.commit()
        .map_err(|e| format!("Failed to commit batch task upsert: {}", e))?;
    Ok(())
}

/// Atomically upserts multiple tasks and their todos into SQLite within a single immediate transaction.
pub fn db_upsert_tasks_batch(root: &Path, tasks: &[Task]) -> Result<(), String> {
    if tasks.is_empty() {
        return Ok(());
    }
    let mut conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    for task in tasks {
        let custom_flow_json = task.custom_flow.as_ref().and_then(|v| serde_json::to_string(v).ok());
        let attachments_json = task.attachments.as_ref().and_then(|v| serde_json::to_string(v).ok());
        let tags_json = task.tags.as_ref().and_then(|v| serde_json::to_string(v).ok());
        let hook_states_json = task.hook_states.as_ref().and_then(|v| serde_json::to_string(v).ok());
        let stage_durations_json = task.stage_durations.as_ref().and_then(|v| serde_json::to_string(v).ok());
        let skip_wt = if task.skip_worktree.unwrap_or(false) { 1i64 } else { 0i64 };
        let cur_tokens = task.current_tokens.map(|v| v as i64);
        let sub_idx = task.subtask_index.map(|v| v as i64);
        let pos_val = task.position.unwrap_or(0.0);

        tx.execute(
            "INSERT OR REPLACE INTO tasks (
                id, title, description, stage_id, board_id, worktree_path, branch, verdict,
                custom_flow, status, model, parent_id, subtask_index, waiting_for_subtask_id,
                handover_summary, cover_image, priority, current_tokens, skip_worktree,
                created_at, updated_at, completed_at, attachments, tags, position, hook_states,
                stage_entered_at, stage_durations, total_elapsed, provider_id, reasoning_effort
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31)",
            params![
                task.id,
                task.title,
                task.description,
                task.stage_id,
                task.board_id.as_deref().unwrap_or("development"),
                task.worktree_path,
                task.branch,
                task.verdict,
                custom_flow_json,
                task.status.as_deref().unwrap_or("idle"),
                task.model,
                task.parent_id,
                sub_idx,
                task.waiting_for_subtask_id,
                task.handover_summary,
                task.cover_image,
                task.priority,
                cur_tokens,
                skip_wt,
                task.created_at,
                task.updated_at,
                task.completed_at,
                attachments_json,
                tags_json,
                pos_val,
                hook_states_json,
                task.stage_entered_at,
                stage_durations_json,
                task.total_elapsed,
                task.provider_id,
                task.reasoning_effort,
            ],
        )
        .map_err(|e| format!("Failed to upsert task {}: {}", task.id, e))?;

        if let Some(ref todos) = task.todos {
            tx.execute("DELETE FROM task_todos WHERE task_id = ?1", params![task.id])
                .map_err(|e| format!("Failed to clean todos for task {}: {}", task.id, e))?;
            for (idx, td) in todos.iter().enumerate() {
                tx.execute(
                    "INSERT INTO task_todos (id, task_id, content, status, order_idx)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![td.id, task.id, td.content, td.status, idx as i64],
                )
                .map_err(|e| format!("Failed to insert todo {}: {}", td.id, e))?;
            }
        }
    }

    tx.commit()
        .map_err(|e| format!("Failed to commit batch task upsert: {}", e))?;
    Ok(())
}

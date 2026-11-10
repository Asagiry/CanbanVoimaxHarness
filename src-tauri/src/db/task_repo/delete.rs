use crate::db::get_db_conn;
use rusqlite::{params, TransactionBehavior};
use std::path::Path;

/// Deletes a task and all associated cascaded subtasks and todos from SQLite.
pub fn db_delete_task(root: &Path, task_id: &str) -> Result<(), String> {
    let mut conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    // 1. Auto-unlock any parent waiting on this task
    tx.execute(
        "UPDATE tasks
         SET waiting_for_subtask_id = NULL,
             status = CASE WHEN status = 'waiting_subtask' THEN 'idle' ELSE status END
         WHERE waiting_for_subtask_id = ?1 OR LOWER(waiting_for_subtask_id) = LOWER(?1)",
        params![task_id],
    )
    .map_err(|e| format!("Failed to unlock parent for task {}: {}", task_id, e))?;

    // 2. Cascade delete subtasks (dialogs, telemetry, todos, tasks)
    let subtask_pattern = format!("{}.%", task_id);
    tx.execute(
        "DELETE FROM dialog_items WHERE task_id IN (
            SELECT id FROM tasks WHERE parent_id = ?1 OR LOWER(parent_id) = LOWER(?1) OR id LIKE ?2
        )",
        params![task_id, subtask_pattern],
    )
    .map_err(|e| format!("Failed to cascade delete dialog_items for subtasks of {}: {}", task_id, e))?;
    tx.execute(
        "DELETE FROM telemetry WHERE task_id IN (
            SELECT id FROM tasks WHERE parent_id = ?1 OR LOWER(parent_id) = LOWER(?1) OR id LIKE ?2
        )",
        params![task_id, subtask_pattern],
    )
    .map_err(|e| format!("Failed to cascade delete telemetry for subtasks of {}: {}", task_id, e))?;
    tx.execute(
        "DELETE FROM task_todos WHERE task_id IN (
            SELECT id FROM tasks WHERE parent_id = ?1 OR LOWER(parent_id) = LOWER(?1) OR id LIKE ?2
        )",
        params![task_id, subtask_pattern],
    )
    .map_err(|e| format!("Failed to cascade delete todos for subtasks of {}: {}", task_id, e))?;
    tx.execute(
        "DELETE FROM tasks WHERE parent_id = ?1 OR LOWER(parent_id) = LOWER(?1) OR id LIKE ?2",
        params![task_id, subtask_pattern],
    )
    .map_err(|e| format!("Failed to cascade delete subtasks of {}: {}", task_id, e))?;

    // 3. Delete target task and its dialogs, telemetry, and todos
    tx.execute("DELETE FROM dialog_items WHERE task_id = ?1 OR LOWER(task_id) = LOWER(?1)", params![task_id])
        .map_err(|e| format!("Failed to delete dialog_items for task {}: {}", task_id, e))?;
    tx.execute("DELETE FROM telemetry WHERE task_id = ?1 OR LOWER(task_id) = LOWER(?1)", params![task_id])
        .map_err(|e| format!("Failed to delete telemetry for task {}: {}", task_id, e))?;
    tx.execute("DELETE FROM task_todos WHERE task_id = ?1 OR LOWER(task_id) = LOWER(?1)", params![task_id])
        .map_err(|e| format!("Failed to delete task_todos for task {}: {}", task_id, e))?;
    tx.execute("DELETE FROM tasks WHERE id = ?1 OR LOWER(id) = LOWER(?1)", params![task_id])
        .map_err(|e| format!("Failed to delete task {}: {}", task_id, e))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit delete transaction: {}", e))?;
    Ok(())
}

/// Atomically cascade-deletes an exact set of task IDs and unlinks waiting parents in one immediate transaction.
pub fn db_delete_tasks_cascade(root: &Path, to_delete: &std::collections::HashSet<String>) -> Result<(), String> {
    if to_delete.is_empty() {
        return Ok(());
    }
    let mut conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    for task_id in to_delete {
        tx.execute(
            "UPDATE tasks
             SET waiting_for_subtask_id = NULL,
                 status = CASE WHEN status = 'waiting_subtask' THEN 'idle' ELSE status END
             WHERE waiting_for_subtask_id = ?1 OR LOWER(waiting_for_subtask_id) = LOWER(?1)",
            params![task_id],
        )
        .map_err(|e| format!("Failed to unlock parent waiting for task {}: {}", task_id, e))?;

        tx.execute("DELETE FROM dialog_items WHERE task_id = ?1 OR LOWER(task_id) = LOWER(?1)", params![task_id])
            .map_err(|e| format!("Failed to delete dialog_items for task {}: {}", task_id, e))?;
        tx.execute("DELETE FROM telemetry WHERE task_id = ?1 OR LOWER(task_id) = LOWER(?1)", params![task_id])
            .map_err(|e| format!("Failed to delete telemetry for task {}: {}", task_id, e))?;
        tx.execute("DELETE FROM task_todos WHERE task_id = ?1 OR LOWER(task_id) = LOWER(?1)", params![task_id])
            .map_err(|e| format!("Failed to delete task_todos for task {}: {}", task_id, e))?;
        tx.execute("DELETE FROM tasks WHERE id = ?1 OR LOWER(id) = LOWER(?1)", params![task_id])
            .map_err(|e| format!("Failed to delete task {}: {}", task_id, e))?;
    }

    tx.commit()
        .map_err(|e| format!("Failed to commit cascade delete transaction: {}", e))?;
    Ok(())
}

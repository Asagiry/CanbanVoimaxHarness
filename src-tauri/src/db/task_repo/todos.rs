use crate::agent::TodoItem;
use crate::db::get_db_conn;
use rusqlite::{params, TransactionBehavior};
use std::path::Path;

/// Atomically saves the checklist (todos) for a specific task.
pub fn db_save_todos(root: &Path, task_id: &str, todos: &[TodoItem]) -> Result<(), String> {
    let mut conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    tx.execute(
        "DELETE FROM task_todos WHERE task_id = ?1 OR LOWER(task_id) = LOWER(?1)",
        params![task_id],
    )
    .map_err(|e| format!("Failed to clear existing todos: {}", e))?;

    for (idx, td) in todos.iter().enumerate() {
        tx.execute(
            "INSERT INTO task_todos (id, task_id, content, status, order_idx)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![td.id, task_id, td.content, td.status, idx as i64],
        )
        .map_err(|e| format!("Failed to insert todo {}: {}", td.id, e))?;
    }

    let now = chrono::Utc::now().timestamp_millis();
    tx.execute(
        "UPDATE tasks SET updated_at = ?1 WHERE id = ?2 OR LOWER(id) = LOWER(?2)",
        params![now, task_id],
    )
    .map_err(|e| format!("Failed to touch task updated_at in db_save_todos: {}", e))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit save todos transaction: {}", e))?;
    Ok(())
}

/// Atomically marks all unfinished todos (status != 'completed') as 'completed' for a specific task.
/// Returns the updated list of todos if any exist, or Ok(None) if there were no todos.
pub fn db_complete_all_todos(root: &Path, task_id: &str) -> Result<Option<Vec<TodoItem>>, String> {
    let mut conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    let changed = tx
        .execute(
            "UPDATE task_todos SET status = 'completed' WHERE (task_id = ?1 OR LOWER(task_id) = LOWER(?1)) AND status != 'completed'",
            params![task_id],
        )
        .map_err(|e| format!("Failed to update task_todos: {}", e))?;

    let now = chrono::Utc::now().timestamp_millis();
    if changed > 0 {
        tx.execute(
            "UPDATE tasks SET updated_at = ?1 WHERE id = ?2 OR LOWER(id) = LOWER(?2)",
            params![now, task_id],
        )
        .map_err(|e| format!("Failed to touch task updated_at in db_complete_all_todos: {}", e))?;
    }

    let todos: Vec<TodoItem> = {
        let mut stmt = tx
            .prepare("SELECT id, content, status FROM task_todos WHERE task_id = ?1 OR LOWER(task_id) = LOWER(?1) ORDER BY order_idx ASC")
            .map_err(|e| format!("Failed to prepare select todos: {}", e))?;

        let rows = stmt
            .query_map(params![task_id], |r| {
                Ok(TodoItem {
                    id: r.get(0)?,
                    content: r.get(1)?,
                    status: r.get(2)?,
                })
            })
            .map_err(|e| format!("Failed to query task_todos: {}", e))?;

        rows.flatten().collect()
    };

    tx.commit()
        .map_err(|e| format!("Failed to commit complete todos transaction: {}", e))?;

    if todos.is_empty() {
        Ok(None)
    } else {
        Ok(Some(todos))
    }
}

/// Returns all unfinished todos (`status != 'completed'`) for a task ordered by `order_idx`.
pub fn db_get_todos(root: &Path, task_id: &str) -> Result<Vec<TodoItem>, String> {
    let conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, content, status FROM task_todos WHERE (task_id = ?1 OR LOWER(task_id) = LOWER(?1)) AND status != 'completed' ORDER BY order_idx ASC",
        )
        .map_err(|e| format!("Failed to prepare select todos: {}", e))?;
    let rows = stmt
        .query_map(params![task_id], |r| {
            Ok(TodoItem {
                id: r.get(0)?,
                content: r.get(1)?,
                status: r.get(2)?,
            })
        })
        .map_err(|e| format!("Failed to query task_todos: {}", e))?;
    Ok(rows.flatten().collect())
}

/// Checks whether a task has any unfinished todos in SQLite.
pub fn db_has_unfinished_todos(root: &Path, task_id: &str) -> Result<bool, String> {
    let conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM task_todos WHERE (task_id = ?1 OR LOWER(task_id) = LOWER(?1)) AND status != 'completed'",
            params![task_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("Failed to read task todos: {}", e))?;
    Ok(count > 0)
}

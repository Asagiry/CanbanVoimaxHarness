use crate::db::get_db_conn;
use rusqlite::{params, TransactionBehavior};
use std::path::Path;

/// Allocates the next monotonic task number from SQLite sequences table.
pub fn db_allocate_next_task_num(root: &Path) -> Result<u32, String> {
    let mut conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    let current_val: u32 = tx
        .query_row(
            "SELECT current_val FROM sequences WHERE name = 'task'",
            [],
            |r| r.get::<_, i64>(0).map(|v| v.max(0) as u32),
        )
        .unwrap_or(0);

    let mut max_in_db: u32 = 0;
    {
        let mut stmt = tx
            .prepare("SELECT id FROM tasks")
            .map_err(|e| format!("Prepare query failed: {}", e))?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| format!("Query failed: {}", e))?;
        for id in rows.flatten() {
            let base = id.split('.').next().unwrap_or(&id);
            let digits: String = base.chars().filter(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = digits.parse::<u32>() {
                if n > max_in_db {
                    max_in_db = n;
                }
            }
        }
    }

    let mut next_num = current_val.max(max_in_db).saturating_add(1).max(1);

    loop {
        let candidate_id = format!("CVH-{:03}", next_num);
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?1 OR LOWER(id) = LOWER(?1))",
                params![candidate_id],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if !exists {
            break;
        }
        next_num = next_num.saturating_add(1);
    }

    tx.execute(
        "INSERT INTO sequences (name, current_val) VALUES ('task', ?1)
         ON CONFLICT(name) DO UPDATE SET current_val = excluded.current_val",
        params![next_num as i64],
    )
    .map_err(|e| format!("Failed to update sequence: {}", e))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit sequence allocation: {}", e))?;

    // Mirror to .cvh/settings/sequence.json and .cvh/task-sequence.json for external tools
    let settings_dir = root.join(".cvh").join("settings");
    let _ = std::fs::create_dir_all(&settings_dir);
    let seq_path = settings_dir.join("sequence.json");
    if let Ok(content) = serde_json::to_string_pretty(&serde_json::json!({
        "last_task_num": next_num
    })) {
        let _ = std::fs::write(&seq_path, &content);
        let alt_path = root.join(".cvh").join("task-sequence.json");
        let _ = std::fs::write(&alt_path, &content);
    }

    Ok(next_num)
}

/// Explicitly updates or sets the sequence counter for task IDs in SQLite and disk files.
pub fn db_update_task_sequence(root: &Path, val: u32) -> Result<(), String> {
    let conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    conn.execute(
        "INSERT INTO sequences (name, current_val) VALUES ('task', ?1)
         ON CONFLICT(name) DO UPDATE SET current_val = MAX(current_val, excluded.current_val)",
        params![val as i64],
    )
    .map_err(|e| format!("Failed to update sequence: {}", e))?;

    let settings_dir = root.join(".cvh").join("settings");
    let _ = std::fs::create_dir_all(&settings_dir);
    let seq_path = settings_dir.join("sequence.json");
    if let Ok(content) = serde_json::to_string_pretty(&serde_json::json!({
        "last_task_num": val
    })) {
        let _ = std::fs::write(&seq_path, &content);
        let alt_path = root.join(".cvh").join("task-sequence.json");
        let _ = std::fs::write(&alt_path, &content);
    }
    Ok(())
}

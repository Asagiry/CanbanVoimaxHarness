use crate::db::{get_db_conn, write_file_atomic_sync};
use rusqlite::{params, TransactionBehavior};
use std::path::Path;
use std::sync::{LazyLock, Mutex};

static SEQUENCE_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn write_sequence_mirrors(root: &Path, val: u32) -> Result<(), String> {
    let settings_dir = root.join(".cvh").join("settings");
    std::fs::create_dir_all(&settings_dir)
        .map_err(|e| format!("Failed to create settings directory {:?}: {}", settings_dir, e))?;
    let content = serde_json::to_string_pretty(&serde_json::json!({
        "last_task_num": val
    }))
    .map_err(|e| format!("Failed to serialize task sequence: {}", e))?;

    let seq_path = settings_dir.join("sequence.json");
    write_file_atomic_sync(&seq_path, &content)?;
    let alt_path = root.join(".cvh").join("task-sequence.json");
    write_file_atomic_sync(&alt_path, &content)?;
    Ok(())
}

/// Allocates the next monotonic task number from SQLite sequences table.
pub fn db_allocate_next_task_num(root: &Path) -> Result<u32, String> {
    let _seq_guard = SEQUENCE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mut conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    let current_val: u32 = match tx.query_row(
        "SELECT current_val FROM sequences WHERE name = 'task'",
        [],
        |r| r.get::<_, i64>(0).map(|v| v.max(0) as u32),
    ) {
        Ok(v) => v,
        Err(rusqlite::Error::QueryReturnedNoRows) => 0,
        Err(e) => return Err(format!("Failed to read task sequence: {}", e)),
    };

    let mut max_in_db: u32 = 0;
    {
        let mut stmt = tx
            .prepare("SELECT id FROM tasks")
            .map_err(|e| format!("Prepare query failed: {}", e))?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| format!("Query failed: {}", e))?;
        for row_res in rows {
            let id = row_res.map_err(|e| format!("Failed to read task id: {}", e))?;
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
            .map_err(|e| format!("Failed to check task ID uniqueness: {}", e))?;
        if !exists {
            break;
        }
        next_num = next_num.saturating_add(1);
    }

    tx.execute(
        "INSERT INTO sequences (name, current_val) VALUES ('task', ?1)
         ON CONFLICT(name) DO UPDATE SET current_val = MAX(current_val, excluded.current_val)",
        params![next_num as i64],
    )
    .map_err(|e| format!("Failed to update sequence: {}", e))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit sequence allocation: {}", e))?;

    if let Err(e) = write_sequence_mirrors(root, next_num) {
        eprintln!("CVH [db_allocate_next_task_num]: Failed to mirror sequence files: {}", e);
    }

    Ok(next_num)
}

/// Explicitly updates or sets the sequence counter for task IDs in SQLite and disk files.
pub fn db_update_task_sequence(root: &Path, val: u32) -> Result<(), String> {
    let _seq_guard = SEQUENCE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    conn.execute(
        "INSERT INTO sequences (name, current_val) VALUES ('task', ?1)
         ON CONFLICT(name) DO UPDATE SET current_val = MAX(current_val, excluded.current_val)",
        params![val as i64],
    )
    .map_err(|e| format!("Failed to update sequence: {}", e))?;

    let effective_val: u32 = conn
        .query_row(
            "SELECT current_val FROM sequences WHERE name = 'task'",
            [],
            |r| r.get::<_, i64>(0).map(|v| v.max(0) as u32),
        )
        .unwrap_or(val);

    write_sequence_mirrors(root, effective_val)
}

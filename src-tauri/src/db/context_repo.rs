use crate::llm::ChatMessage;
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

#[derive(Serialize, Deserialize)]
pub struct ConversationContext {
    pub turn: usize,
    pub messages: Vec<ChatMessage>,
    pub dialog_cursor: i64,
}

pub fn context_file_path(root: &Path, task_id: &str) -> PathBuf {
    let safe_task_id: String = task_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    root.join(".cvh").join("context").join(format!("{}.json", safe_task_id))
}

fn write_context_file_atomic(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let tmp_path = path.with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));
    if let Err(e) = std::fs::write(&tmp_path, content) {
        return Err(format!("Failed to write temporary context file: {e}"));
    }
    if let Err(err) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        std::fs::write(path, content)
            .map_err(|e| format!("Failed to write context file directly after rename error ({err}): {e}"))?;
    }
    Ok(())
}

pub fn context_dialog_cursor(root: &Path, task_id: &str) -> Result<i64, String> {
    let conn = super::get_db_conn(root).map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT COALESCE(MAX(rowid), 0) FROM dialog_items WHERE task_id = ?1 AND type != 'context_snapshot'",
        params![task_id], |row| row.get(0),
    ).map_err(|e| e.to_string())
}

pub fn load_conversation_context(root: &Path, task_id: &str) -> Result<Option<ConversationContext>, String> {
    let conn = super::get_db_conn(root).map_err(|e| e.to_string())?;
    let db_content: Option<String> = conn.query_row(
        "SELECT content FROM dialog_items WHERE task_id = ?1 AND type = 'context_snapshot'",
        params![task_id], |row| row.get(0),
    ).optional().map_err(|e| e.to_string())?;

    let text = match db_content {
        None => return Ok(None),
        Some(s) if s.trim().is_empty() => return Ok(None),
        Some(s) => s,
    };

    // 1. Primary path: Read from dedicated context file (.cvh/context/{task_id}.json)
    let file_path = context_file_path(root, task_id);
    if file_path.exists() {
        if let Ok(file_data) = std::fs::read_to_string(&file_path) {
            let trimmed = file_data.trim();
            if !trimmed.is_empty() {
                if let Ok(ctx) = serde_json::from_str::<ConversationContext>(trimmed) {
                    return Ok(Some(ctx));
                }
            }
        }
    }

    // 2. Backward compatibility fallback: read from dialog_items.content if legacy JSON exists
    if text.trim().starts_with('{') {
        return serde_json::from_str(&text).map(Some).map_err(|e| e.to_string());
    }

    Ok(None)
}

pub async fn save_conversation_context(
    root: &Path,
    task_id: &str,
    turn: usize,
    messages: &[ChatMessage],
    dialog_cursor: i64,
    abort_flag: &Arc<AtomicBool>,
) -> Result<(), String> {
    let root = root.to_path_buf();
    let task_id = task_id.to_string();
    let context = ConversationContext { turn, messages: messages.to_vec(), dialog_cursor };
    let abort_flag = Arc::clone(abort_flag);
    tokio::task::spawn_blocking(move || {
        let json = serde_json::to_string(&context).map_err(|e| e.to_string())?;
        let mut conn = super::get_db_conn(&root).map_err(|e| e.to_string())?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
        if abort_flag.load(Ordering::SeqCst) {
            return Ok(());
        }
        let rows_affected = tx.execute(
            "INSERT INTO dialog_items (id, task_id, type, content, created_at)
             SELECT ?1, ?2, 'context_snapshot', ?3, ?4
             WHERE NOT EXISTS (
                 SELECT 1 FROM dialog_items WHERE task_id = ?2 AND type = 'compacted' AND rowid > ?5
             )
             ON CONFLICT(id) DO UPDATE SET content = excluded.content, created_at = excluded.created_at",
            params![format!("context:{task_id}"), task_id, "file", chrono::Utc::now().timestamp_millis(), dialog_cursor],
        ).map_err(|e| e.to_string())?;

        if rows_affected > 0 {
            let file_path = context_file_path(&root, &task_id);
            write_context_file_atomic(&file_path, &json)?;
        }
        tx.commit().map_err(|e| e.to_string())
    }).await.map_err(|e| e.to_string())?
}

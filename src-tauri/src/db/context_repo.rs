use crate::llm::ChatMessage;
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

#[derive(Serialize, Deserialize)]
pub struct ConversationContext {
    pub turn: usize,
    pub messages: Vec<ChatMessage>,
    pub dialog_cursor: i64,
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
    let json: Option<String> = conn.query_row(
        "SELECT content FROM dialog_items WHERE task_id = ?1 AND type = 'context_snapshot'",
        params![task_id], |row| row.get(0),
    ).optional().map_err(|e| e.to_string())?;
    json.map(|text| serde_json::from_str(&text).map_err(|e| e.to_string())).transpose()
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
        tx.execute(
            "INSERT INTO dialog_items (id, task_id, type, content, created_at)
             SELECT ?1, ?2, 'context_snapshot', ?3, ?4
             WHERE NOT EXISTS (
                 SELECT 1 FROM dialog_items WHERE task_id = ?2 AND type = 'compacted' AND rowid > ?5
             )
             ON CONFLICT(id) DO UPDATE SET content = excluded.content, created_at = excluded.created_at",
            params![format!("context:{task_id}"), task_id, json, chrono::Utc::now().timestamp_millis(), dialog_cursor],
        ).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }).await.map_err(|e| e.to_string())?
}

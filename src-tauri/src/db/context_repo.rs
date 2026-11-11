use crate::llm::ChatMessage;
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{atomic::{AtomicBool, Ordering}, Arc, LazyLock, Mutex};

static CONTEXT_IO_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub(crate) fn escape_like(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        if matches!(c, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[derive(Serialize, Deserialize)]
pub struct ConversationContext {
    pub turn: usize,
    pub messages: Vec<ChatMessage>,
    pub dialog_cursor: i64,
}

/// Maps `task_id` injectively into a safe cross-platform filename without collisions.
pub fn context_file_path(root: &Path, task_id: &str) -> PathBuf {
    let mut encoded = String::with_capacity(task_id.len());
    for b in task_id.as_bytes() {
        let c = *b as char;
        if c.is_ascii_alphanumeric() || c == '-' {
            encoded.push(c);
        } else {
            encoded.push_str(&format!("_{:02x}", b));
        }
    }
    let filename = if encoded.len() > 160 {
        let mut hasher = DefaultHasher::new();
        task_id.hash(&mut hasher);
        format!("{}_{:016x}.json", &encoded[..140], hasher.finish())
    } else {
        format!("{}.json", encoded)
    };
    root.join(".cvh").join("context").join(filename)
}

pub(crate) fn write_file_atomic_sync(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory {:?}: {e}", parent))?;
    }
    let tmp_path = path.with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));
    std::fs::write(&tmp_path, content)
        .map_err(|e| format!("Failed to write temporary file {:?}: {e}", tmp_path))?;

    let mut last_err = None;
    for attempt in 0..5u64 {
        match std::fs::rename(&tmp_path, path) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                if attempt < 4 {
                    std::thread::sleep(std::time::Duration::from_millis(15 * (attempt + 1)));
                }
            }
        }
    }
    if let Err(rm_err) = std::fs::remove_file(&tmp_path) {
        if rm_err.kind() != std::io::ErrorKind::NotFound {
            eprintln!("CVH [db]: Failed to clean temp file {:?}: {}", tmp_path, rm_err);
        }
    }
    Err(format!(
        "Failed to atomically rename {:?} -> {:?}: {}",
        tmp_path,
        path,
        last_err.map(|e| e.to_string()).unwrap_or_else(|| "unknown error".to_string())
    ))
}

fn write_context_temp_file(path: &Path, content: &str) -> Result<PathBuf, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create context directory {:?}: {e}", parent))?;
    }
    let tmp_path = path.with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));
    std::fs::write(&tmp_path, content)
        .map_err(|e| format!("Failed to write temporary context file {:?}: {e}", tmp_path))?;
    Ok(tmp_path)
}

fn promote_temp_file_atomic(tmp_path: &Path, target: &Path) -> Result<(), String> {
    let mut last_err = None;
    for attempt in 0..5u64 {
        match std::fs::rename(tmp_path, target) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                if attempt < 4 {
                    std::thread::sleep(std::time::Duration::from_millis(15 * (attempt + 1)));
                }
            }
        }
    }
    if let Err(rm_err) = std::fs::remove_file(tmp_path) {
        if rm_err.kind() != std::io::ErrorKind::NotFound {
            eprintln!("CVH [db]: Failed to clean temp context file {:?}: {}", tmp_path, rm_err);
        }
    }
    Err(format!(
        "Failed to atomically replace context file {:?}: {}",
        target,
        last_err.map(|e| e.to_string()).unwrap_or_else(|| "unknown error".to_string())
    ))
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

    let trimmed_db = text.trim();
    if trimmed_db == "file" {
        let file_path = context_file_path(root, task_id);
        let file_data = std::fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read context file {:?}: {}", file_path, e))?;
        let trimmed = file_data.trim();
        if trimmed.is_empty() {
            return Err(format!("Context file {:?} is empty", file_path));
        }
        return serde_json::from_str::<ConversationContext>(trimmed)
            .map(Some)
            .map_err(|e| format!("Corrupt context file {:?}: {}", file_path, e));
    }

    // Backward compatibility fallback: read from dialog_items.content if legacy JSON exists
    if trimmed_db.starts_with('{') {
        return serde_json::from_str(trimmed_db).map(Some).map_err(|e| e.to_string());
    }

    Err(format!("Unsupported context_snapshot marker for task {}", task_id))
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
        if abort_flag.load(Ordering::SeqCst) {
            return Ok(());
        }
        let json = serde_json::to_string(&context).map_err(|e| e.to_string())?;
        let file_path = context_file_path(&root, &task_id);
        // Perform disk write into temporary file BEFORE opening SQLite write transaction.
        let tmp_path = write_context_temp_file(&file_path, &json)?;

        let _io_guard = CONTEXT_IO_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        if abort_flag.load(Ordering::SeqCst) {
            let _ = std::fs::remove_file(&tmp_path);
            return Ok(());
        }
        let rows_affected = {
            let mut conn = super::get_db_conn(&root).map_err(|e| e.to_string())?;
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
            if abort_flag.load(Ordering::SeqCst) {
                drop(tx);
                let _ = std::fs::remove_file(&tmp_path);
                return Ok(());
            }
            let affected = tx.execute(
                "INSERT INTO dialog_items (id, task_id, type, content, created_at)
                 SELECT ?1, ?2, 'context_snapshot', ?3, ?4
                 WHERE NOT EXISTS (
                     SELECT 1 FROM dialog_items WHERE task_id = ?2 AND type = 'compacted' AND rowid > ?5
                 )
                 ON CONFLICT(id) DO UPDATE SET content = excluded.content, created_at = excluded.created_at",
                params![format!("context:{task_id}"), task_id, "file", chrono::Utc::now().timestamp_millis(), dialog_cursor],
            ).map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
            affected
        };

        if rows_affected > 0 {
            promote_temp_file_atomic(&tmp_path, &file_path)
        } else {
            if let Err(rm_err) = std::fs::remove_file(&tmp_path) {
                if rm_err.kind() != std::io::ErrorKind::NotFound {
                    eprintln!("CVH [db]: Failed to clean unused temp context file {:?}: {}", tmp_path, rm_err);
                }
            }
            Ok(())
        }
    }).await.map_err(|e| e.to_string())?
}

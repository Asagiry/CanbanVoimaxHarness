use crate::db::get_db_conn;
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use std::path::Path;

use super::dialog_query::ActivityItem;

pub fn insert_steer_message(root: &Path, task_id: &str, item: &ActivityItem) -> Result<bool, String> {
    let images = item.images.as_ref().filter(|images| !images.is_empty())
        .map(serde_json::to_string).transpose().map_err(|e| e.to_string())?;
    let mut conn = get_db_conn(root).map_err(|e| e.to_string())?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
    let inserted = tx.execute(
        "INSERT INTO dialog_items (id, task_id, type, content, images_json, timestamp, created_at)
         VALUES (?1, ?2, 'steer', ?3, ?4, ?5, ?6) ON CONFLICT(id) DO NOTHING",
        params![item.id, task_id, item.content, images, item.timestamp, chrono::Utc::now().timestamp_millis()],
    ).map_err(|e| e.to_string())? > 0;
    if !inserted {
        let existing: (String, String, Option<String>, Option<String>) = tx.query_row(
            "SELECT task_id, type, content, images_json FROM dialog_items WHERE id = ?1",
            params![item.id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        ).map_err(|e| e.to_string())?;
        if existing.0 != task_id || !matches!(existing.1.as_str(), "user" | "steer")
            || existing.2 != item.content || existing.3 != images {
            return Err("Message ID already belongs to a different message".into());
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(inserted)
}

/// Persists/appends dialog items for a task into SQLite atomically.
pub fn append_dialog_items(root: &Path, task_id: &str, items: &[ActivityItem]) -> Result<(), String> {
    if items.is_empty() {
        return Ok(());
    }

    let mut conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    let now_base = chrono::Utc::now().timestamp_millis();

    {
        let mut stmt = tx
            .prepare_cached(
                "INSERT INTO dialog_items (
                    id, task_id, type, content, images_json, timestamp, created_at, tool_json, duration_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(id) DO UPDATE SET
                    type = excluded.type,
                    content = excluded.content,
                    images_json = excluded.images_json,
                    timestamp = excluded.timestamp,
                    created_at = excluded.created_at,
                    tool_json = excluded.tool_json,
                    duration_ms = excluded.duration_ms
                WHERE dialog_items.type != 'deleted'",
            )
            .map_err(|e| format!("Failed to prepare insert statement: {}", e))?;

        for (idx, item) in items.iter().enumerate() {
            // Deduplicate matching output items to eliminate duplicate final response cards
            if item.item_type == "output" && !item.id.ends_with(":output") {
                if let Some(ref text) = item.content {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        tx.execute(
                            "DELETE FROM dialog_items WHERE task_id = ?1 AND type = 'output' AND TRIM(content) = ?2 AND id != ?3
                             AND NOT EXISTS (SELECT 1 FROM dialog_items WHERE id = ?3 AND type = 'deleted')",
                            params![task_id, trimmed, item.id],
                        )
                        .map_err(|e| format!("Failed to deduplicate output dialog item: {}", e))?;
                    }
                }
            }

            if matches!(item.item_type.as_str(), "steer" | "summary") {
                if let Some(ref text) = item.content {
                    let trimmed = text.trim();
                    let is_handover = trimmed.contains("[ОТЧЕТ ОБ ИСПРАВЛЕНИЯХ / HANDOVER SUMMARY]");
                    if !trimmed.is_empty() && (item.item_type == "summary" || is_handover) {
                        tx.execute(
                            "DELETE FROM dialog_items WHERE task_id = ?1 AND type = ?2 AND TRIM(content) = ?3 AND id != ?4
                             AND NOT EXISTS (SELECT 1 FROM dialog_items WHERE id = ?4 AND type = 'deleted')",
                            params![task_id, item.item_type, trimmed, item.id],
                        )
                        .map_err(|e| format!("Failed to deduplicate steer/summary dialog item: {}", e))?;
                    }
                }
            }

            let images_json = match &item.images {
                Some(imgs) if !imgs.is_empty() => {
                    Some(serde_json::to_string(imgs).map_err(|e| e.to_string())?)
                }
                _ => None,
            };

            let tool_json = match &item.tool {
                Some(val) => Some(serde_json::to_string(val).map_err(|e| e.to_string())?),
                None => None,
            };

            let created_at = now_base + (idx as i64);

            stmt.execute(params![
                item.id,
                task_id,
                if item.is_runtime_error() { "error" } else { &item.item_type },
                item.content,
                images_json,
                item.timestamp,
                created_at,
                tool_json,
                item.duration_ms,
            ])
            .map_err(|e| format!("Failed to insert dialog item {}: {}", item.id, e))?;
        }
    }

    tx.commit()
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;

    Ok(())
}

pub fn discard_stream_attempt(root: &Path, task_id: &str, stream_id: &str) -> Result<(), String> {
    let mut conn = get_db_conn(root).map_err(|e| e.to_string())?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now();
    for kind in ["output", "thought"] {
        tx.execute(
            "INSERT INTO dialog_items (id, task_id, type, timestamp, created_at)
             VALUES (?1, ?2, 'deleted', ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET type = 'deleted', content = NULL,
                 images_json = NULL, tool_json = NULL, duration_ms = NULL
             WHERE dialog_items.task_id = excluded.task_id",
            params![format!("{stream_id}:{kind}"), task_id, now.to_rfc3339(), now.timestamp_millis()],
        ).map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}

/// Persists a compacted summary item and an optional tool execution item,
/// and marks old intermediate dialog items as 'compacted_archived'.
pub fn persist_compacted_dialog(
    root: &Path,
    task_id: &str,
    compacted_item: &ActivityItem,
    tool_item: Option<&ActivityItem>,
    ids_to_archive: &[String],
) -> Result<(), String> {
    let mut conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    let source_ids = serde_json::to_string(ids_to_archive).map_err(|e| e.to_string())?;
    let source_deleted: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM dialog_items WHERE task_id = ?1 AND type = 'deleted'
         AND (id IN (SELECT value FROM json_each(?2)) OR id = ?3))",
        params![task_id, source_ids, tool_item.map(|item| &item.id)], |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    if source_deleted {
        return Err("Dialogue changed during compaction; retry using the remaining messages".to_string());
    }

    // 1. Mark intermediate items as compacted_archived
    tx.execute("DELETE FROM dialog_items WHERE task_id = ?1 AND type = 'context_snapshot'", params![task_id])
        .map_err(|e| format!("Failed to invalidate conversation context: {e}"))?;
    for id in ids_to_archive {
        tx.execute(
            "UPDATE dialog_items SET type = 'compacted_archived' WHERE task_id = ?1 AND id = ?2 AND type != 'deleted'",
            params![task_id, id],
        )
        .map_err(|e| format!("Failed to archive dialog item {}: {}", id, e))?;
    }

    let now_millis = chrono::Utc::now().timestamp_millis();

    // 2. Optionally insert the tool execution card for compact_context
    if let Some(tool_act) = tool_item {
        let tool_json = match &tool_act.tool {
            Some(val) => Some(serde_json::to_string(val).map_err(|e| e.to_string())?),
            None => None,
        };
        tx.execute(
            "INSERT INTO dialog_items (
                id, task_id, type, content, images_json, timestamp, created_at, tool_json, duration_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(id) DO UPDATE SET
                type = excluded.type,
                content = excluded.content,
                images_json = excluded.images_json,
                timestamp = excluded.timestamp,
                tool_json = excluded.tool_json,
                duration_ms = excluded.duration_ms
             WHERE dialog_items.type != 'deleted'",
            params![
                tool_act.id,
                task_id,
                tool_act.item_type,
                tool_act.content,
                None::<String>,
                tool_act.timestamp,
                now_millis,
                tool_json,
                tool_act.duration_ms,
            ],
        )
        .map_err(|e| format!("Failed to insert compact tool item: {}", e))?;
    }

    // 3. Insert compacted summary accordion item (1ms later for deterministic ordering)
    let tool_json = match &compacted_item.tool {
        Some(val) => Some(serde_json::to_string(val).map_err(|e| e.to_string())?),
        None => None,
    };

    tx.execute(
        "INSERT INTO dialog_items (
            id, task_id, type, content, images_json, timestamp, created_at, tool_json, duration_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ON CONFLICT(id) DO UPDATE SET
            type = excluded.type,
            content = excluded.content,
            images_json = excluded.images_json,
            timestamp = excluded.timestamp,
            tool_json = excluded.tool_json,
            duration_ms = excluded.duration_ms
         WHERE dialog_items.type != 'deleted'",
        params![
            compacted_item.id,
            task_id,
            compacted_item.item_type,
            compacted_item.content,
            None::<String>,
            compacted_item.timestamp,
            now_millis + 1,
            tool_json,
            compacted_item.duration_ms,
        ],
    )
    .map_err(|e| format!("Failed to insert compacted item: {}", e))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;

    Ok(())
}

pub fn resolve_dialog_item_task_id(root: &Path, task_id: &str, item_id: &str) -> Result<String, String> {
    let conn = get_db_conn(root).map_err(|e| e.to_string())?;
    let owner: Option<String> = conn.query_row(
        "SELECT task_id FROM dialog_items WHERE id = ?1", params![item_id], |row| row.get(0),
    ).optional().map_err(|e| e.to_string())?;
    let Some(owner) = owner else { return Ok(task_id.to_string()); };
    if owner.eq_ignore_ascii_case(task_id) || task_id.split_once(":sub:").is_some_and(|(parent, _)| {
        owner.to_lowercase().starts_with(&format!("{}:sub:", parent.to_lowercase()))
    }) {
        return Ok(owner);
    }
    Err("Dialog item belongs to another task".to_string())
}

/// Deletes a specific dialog item for a task by ID.
pub fn delete_dialog_item(root: &Path, task_id: &str, item_id: &str) -> Result<Vec<String>, String> {
    if item_id.trim().is_empty() || item_id.starts_with("context:") {
        return Err("Invalid dialog item ID".to_string());
    }
    let mut conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
    let tool_json: Option<String> = tx.query_row(
        "SELECT tool_json FROM dialog_items WHERE LOWER(task_id) = LOWER(?1) AND id = ?2 AND type IN ('tool', 'question')",
        params![task_id, item_id], |row| row.get(0),
    ).optional().map_err(|e| e.to_string())?.flatten();
    let mut ids = vec![item_id.to_string()];
    if let Some(json) = tool_json.filter(|json| !json.trim().is_empty()) {
        let tool: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        for path in ["/id", "/params/callId", "/params/questionId"] {
            if let Some(id) = tool.pointer(path).and_then(|value| value.as_str()).filter(|id| !id.is_empty()) {
                ids.push(id.to_string());
            }
        }
        let identities = serde_json::to_string(&ids).map_err(|e| e.to_string())?;
        let mut stmt = tx.prepare(
            "SELECT id FROM dialog_items WHERE LOWER(task_id) = LOWER(?1) AND type IN ('tool', 'question') AND (
                id IN (SELECT value FROM json_each(?2)) OR
                json_extract(tool_json, '$.id') IN (SELECT value FROM json_each(?2)) OR
                json_extract(tool_json, '$.params.callId') IN (SELECT value FROM json_each(?2)) OR
                json_extract(tool_json, '$.params.questionId') IN (SELECT value FROM json_each(?2)))",
        ).map_err(|e| e.to_string())?;
        let related = stmt.query_map(params![task_id, identities], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        for id in related { ids.push(id.map_err(|e| e.to_string())?); }
    }
    ids.sort();
    ids.dedup();
    for id in &ids {
        tx.execute(
            "INSERT INTO dialog_items (id, task_id, type, created_at) VALUES (?1, ?2, 'deleted', ?3)
             ON CONFLICT(id) DO UPDATE SET type = 'deleted', content = NULL, images_json = NULL, tool_json = NULL, duration_ms = NULL
             WHERE LOWER(dialog_items.task_id) = LOWER(?2)",
            params![id, task_id, chrono::Utc::now().timestamp_millis()],
        ).map_err(|e| format!("Failed to delete dialog item: {e}"))?;
    }
    tx.execute("DELETE FROM dialog_items WHERE LOWER(task_id) = LOWER(?1) AND type = 'context_snapshot'", params![task_id])
        .map_err(|e| format!("Failed to invalidate context: {e}"))?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(ids)
}

/// Deletes all dialog items for a specific task and its cascaded subagents,
/// including any context snapshot files on disk.
pub fn clear_dialog_items(root: &Path, task_id: &str) -> Result<(), String> {
    let mut conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let escaped = crate::db::escape_like(task_id);
    let sub_colon = format!("{}:sub:%", escaped);
    let sub_dot = format!("{}.sub-%", escaped);

    let mut tids_to_clean: Vec<String> = {
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT task_id FROM dialog_items
                 WHERE task_id = ?1 OR task_id LIKE ?2 ESCAPE '\\' OR task_id LIKE ?3 ESCAPE '\\'",
            )
            .map_err(|e| format!("Failed to query dialog task IDs: {}", e))?;
        let rows = stmt
            .query_map(params![task_id, sub_colon, sub_dot], |row| row.get::<_, String>(0))
            .map_err(|e| format!("Failed to list dialog task IDs: {}", e))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| format!("Failed to read dialog task ID: {}", e))?);
        }
        out
    };
    if !tids_to_clean.iter().any(|id| id == task_id) {
        tids_to_clean.push(task_id.to_string());
    }

    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("Failed to start transaction: {}", e))?;
    tx.execute(
        "DELETE FROM dialog_items WHERE task_id = ?1 OR task_id LIKE ?2 ESCAPE '\\' OR task_id LIKE ?3 ESCAPE '\\'",
        params![task_id, sub_colon, sub_dot],
    )
    .map_err(|e| format!("Failed to delete dialog items: {}", e))?;
    tx.commit()
        .map_err(|e| format!("Failed to commit clear_dialog_items: {}", e))?;

    for tid in tids_to_clean {
        let ctx_path = crate::db::context_file_path(root, &tid);
        if let Err(e) = std::fs::remove_file(&ctx_path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(format!("Failed to remove context file {:?}: {}", ctx_path, e));
            }
        }
    }
    Ok(())
}


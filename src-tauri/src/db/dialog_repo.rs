use crate::db::get_db_conn;
use rusqlite::{params, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActivityItem {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub timestamp: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
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
                    content = excluded.content,
                    images_json = excluded.images_json,
                    timestamp = excluded.timestamp,
                    tool_json = excluded.tool_json,
                    duration_ms = excluded.duration_ms",
            )
            .map_err(|e| format!("Failed to prepare insert statement: {}", e))?;

        for (idx, item) in items.iter().enumerate() {
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
                item.item_type,
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

/// Loads all dialog items for a task ordered chronologically by created_at,
/// excluding compacted archived intermediate messages.
pub fn load_dialog_items(root: &Path, task_id: &str) -> Result<Vec<ActivityItem>, String> {
    let conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, type, content, images_json, timestamp, tool_json, duration_ms
             FROM dialog_items
             WHERE task_id = ?1 AND type != 'compacted_archived'
             ORDER BY created_at ASC",
        )
        .map_err(|e| format!("Failed to prepare select statement: {}", e))?;

    let rows = stmt
        .query_map(params![task_id], |row| {
            let id: String = row.get(0)?;
            let item_type: String = row.get(1)?;
            let content: Option<String> = row.get(2)?;
            let images_json: Option<String> = row.get(3)?;
            let timestamp: String = row.get(4)?;
            let tool_json: Option<String> = row.get(5)?;
            let duration_ms: Option<f64> = row.get(6)?;

            Ok((id, item_type, content, images_json, timestamp, tool_json, duration_ms))
        })
        .map_err(|e| format!("Query failed: {}", e))?;

    let mut items = Vec::new();
    for row_res in rows {
        let (id, item_type, content, images_json, timestamp, tool_json, duration_ms) =
            row_res.map_err(|e| format!("Row extraction failed: {}", e))?;

        let images = images_json.and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok());
        let tool = tool_json.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());

        items.push(ActivityItem {
            id,
            item_type,
            timestamp,
            content,
            tool,
            duration_ms,
            images,
        });
    }

    Ok(items)
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

    // 1. Mark intermediate items as compacted_archived
    for id in ids_to_archive {
        let _ = tx.execute(
            "UPDATE dialog_items SET type = 'compacted_archived' WHERE task_id = ?1 AND id = ?2",
            params![task_id, id],
        );
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
                duration_ms = excluded.duration_ms",
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
            duration_ms = excluded.duration_ms",
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

/// Deletes all dialog items for a specific task and its cascaded subagents.
pub fn clear_dialog_items(root: &Path, task_id: &str) -> Result<(), String> {
    let conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let sub_colon = format!("{}:sub:%", task_id);
    let sub_dot = format!("{}.sub-%", task_id);
    conn.execute(
        "DELETE FROM dialog_items WHERE task_id = ?1 OR task_id LIKE ?2 OR task_id LIKE ?3",
        params![task_id, sub_colon, sub_dot],
    )
    .map_err(|e| format!("Failed to delete dialog items: {}", e))?;
    Ok(())
}

/// Finds all running tools in dialog_items for a task (or specific subagent)
/// and updates them to status 'failed' with stderr 'Прервано пользователем'.
/// Returns a list of (item_id, item_task_id, tool_id, tool_name) that were updated.
pub fn mark_running_tools_as_aborted(
    root: &Path,
    task_id: &str,
    is_subagent: bool,
) -> Result<Vec<(String, String, String, String)>, String> {
    let mut conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let p_clean = task_id.trim();
    if p_clean.is_empty() {
        return Ok(Vec::new());
    }

    let rows = if is_subagent {
        let sub_core = p_clean.rsplit(":sub:").next().unwrap_or(p_clean);
        let pattern1 = format!("%:sub:{}", sub_core);
        let pattern2 = format!("%:sub:%{}", sub_core);
        let mut stmt = conn
            .prepare(
                r#"SELECT id, task_id, content, tool_json FROM dialog_items
                 WHERE (task_id = ?1 OR task_id LIKE ?2 OR task_id LIKE ?3)
                   AND tool_json IS NOT NULL
                   AND tool_json LIKE '%"status"%"running"%'"#,
            )
            .map_err(|e| e.to_string())?;

        let res = stmt
            .query_map(params![p_clean, pattern1, pattern2], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();
        res
    } else {
        let pattern_dot = format!("{}.sub-%", p_clean);
        let pattern_colon = format!("{}:sub:%", p_clean);
        let mut stmt = conn
            .prepare(
                r#"SELECT id, task_id, content, tool_json FROM dialog_items
                 WHERE (task_id = ?1 OR task_id LIKE ?2 OR task_id LIKE ?3)
                   AND tool_json IS NOT NULL
                   AND tool_json LIKE '%"status"%"running"%'"#,
            )
            .map_err(|e| e.to_string())?;

        let res = stmt
            .query_map(params![p_clean, pattern_dot, pattern_colon], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();
        res
    };

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    let mut updated_tools = Vec::new();

    {
        let mut update_stmt = tx
            .prepare_cached(
                "UPDATE dialog_items
                 SET content = ?1, tool_json = ?2, timestamp = ?3
                 WHERE id = ?4",
            )
            .map_err(|e| e.to_string())?;

        let now_iso = chrono::Utc::now().to_rfc3339();

        for (item_id, item_task_id, content_opt, raw_tool_json) in rows {
            if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&raw_tool_json) {
                let status = val.get("status").and_then(|s| s.as_str()).unwrap_or("");
                if status == "running" {
                    let tool_id = val
                        .get("id")
                        .and_then(|s| s.as_str())
                        .unwrap_or(&item_id)
                        .to_string();
                    let tool_name = val
                        .get("toolName")
                        .and_then(|s| s.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    val["status"] = serde_json::json!("failed");
                    val["stderr"] = serde_json::json!("Прервано пользователем");
                    val["stdout"] = serde_json::json!("[Прервано пользователем / Aborted by user]");
                    val["timestamp"] = serde_json::json!(&now_iso);

                    let new_tool_json = serde_json::to_string(&val).unwrap_or(raw_tool_json);
                    let new_content = match content_opt {
                        Some(c) if !c.trim().is_empty() && c != "running" => Some(c),
                        _ => Some("Прервано пользователем".to_string()),
                    };

                    update_stmt
                        .execute(params![new_content, new_tool_json, now_iso, item_id])
                        .map_err(|e| e.to_string())?;

                    updated_tools.push((item_id, item_task_id, tool_id, tool_name));
                }
            }
        }
    }

    tx.commit()
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;

    Ok(updated_tools)
}

/// Searches SQLite for a subagent dialog key matching the given parent_task_id and/or subagent/tool_call_id.
pub fn find_subagent_dialog_key(
    root: &Path,
    parent_task_id: Option<&str>,
    tool_call_id: &str,
) -> Result<Option<String>, String> {
    let conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tc_clean = tool_call_id.trim();
    if tc_clean.is_empty() {
        return Ok(None);
    }

    // 1. Try parent-scoped suffix match
    if let Some(p) = parent_task_id {
        let p_clean = p.trim();
        if !p_clean.is_empty() {
            let pattern = format!("{}:sub:%.sub-{}", p_clean, tc_clean);
            let mut stmt = conn
                .prepare("SELECT task_id FROM dialog_items WHERE task_id LIKE ?1 ORDER BY created_at DESC LIMIT 1")
                .map_err(|e| format!("Prepare failed: {}", e))?;
            let mut rows = stmt
                .query(params![pattern])
                .map_err(|e| format!("Query failed: {}", e))?;
            if let Some(row) = rows.next().map_err(|e| format!("Next row failed: {}", e))? {
                let matched: String = row.get(0).map_err(|e| format!("Get col failed: {}", e))?;
                return Ok(Some(matched));
            }
        }
    }

    // 2. Try general suffix match
    let pattern_suffix = format!("%.sub-{}", tc_clean);
    let mut stmt = conn
        .prepare("SELECT task_id FROM dialog_items WHERE task_id LIKE ?1 ORDER BY created_at DESC LIMIT 1")
        .map_err(|e| format!("Prepare failed: {}", e))?;
    let mut rows = stmt
        .query(params![pattern_suffix])
        .map_err(|e| format!("Query failed: {}", e))?;
    if let Some(row) = rows.next().map_err(|e| format!("Next row failed: {}", e))? {
        let matched: String = row.get(0).map_err(|e| format!("Get col failed: {}", e))?;
        return Ok(Some(matched));
    }

    // 3. Try exact suffix match if tool_call_id is already prefixed with .sub-
    if tc_clean.contains(".sub-") {
        let pattern_any = format!("%{}", tc_clean);
        let mut stmt = conn
            .prepare("SELECT task_id FROM dialog_items WHERE task_id LIKE ?1 ORDER BY created_at DESC LIMIT 1")
            .map_err(|e| format!("Prepare failed: {}", e))?;
        let mut rows = stmt
            .query(params![pattern_any])
            .map_err(|e| format!("Query failed: {}", e))?;
        if let Some(row) = rows.next().map_err(|e| format!("Next row failed: {}", e))? {
            let matched: String = row.get(0).map_err(|e| format!("Get col failed: {}", e))?;
            return Ok(Some(matched));
        }
    }

    Ok(None)
}

/// Returns all distinct subagent dialog keys for a given parent task id.
pub fn list_subagent_dialog_keys(
    root: &Path,
    parent_task_id: &str,
) -> Result<Vec<String>, String> {
    let conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let p_clean = parent_task_id.trim();
    if p_clean.is_empty() {
        return Ok(Vec::new());
    }
    let pattern = format!("{}:sub:%", p_clean);
    let mut stmt = conn
        .prepare("SELECT DISTINCT task_id FROM dialog_items WHERE task_id LIKE ?1 ORDER BY created_at ASC")
        .map_err(|e| format!("Prepare failed: {}", e))?;
    let rows = stmt
        .query_map(params![pattern], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Query failed: {}", e))?;
    let keys = rows.flatten().collect();
    Ok(keys)
}

// Aliases for convenience
pub use append_dialog_items as db_append_dialog_item;
pub use load_dialog_items as db_load_dialog_items;

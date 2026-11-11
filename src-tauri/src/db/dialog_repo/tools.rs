use crate::db::get_db_conn;
use rusqlite::{params, TransactionBehavior};
use std::path::Path;

use super::mutation::append_dialog_items;
use super::dialog_query::ActivityItem;

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
        let sub_core = crate::db::escape_like(p_clean.rsplit(":sub:").next().unwrap_or(p_clean));
        let pattern1 = format!("%:sub:{}", sub_core);
        let pattern2 = format!("%:sub:%{}", sub_core);
        let mut stmt = conn
            .prepare(
                r#"SELECT id, task_id, content, tool_json FROM dialog_items
                 WHERE (task_id = ?1 OR task_id LIKE ?2 ESCAPE '\' OR task_id LIKE ?3 ESCAPE '\')
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
        let escaped = crate::db::escape_like(p_clean);
        let pattern_dot = format!("{}.sub-%", escaped);
        let pattern_colon = format!("{}:sub:%", escaped);
        let mut stmt = conn
            .prepare(
                r#"SELECT id, task_id, content, tool_json FROM dialog_items
                 WHERE (task_id = ?1 OR task_id LIKE ?2 ESCAPE '\' OR task_id LIKE ?3 ESCAPE '\')
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

/// Targeted update of a single tool execution result without deserializing or rewriting the entire dialog history.
pub fn db_update_dialog_tool_result(
    root: &Path,
    task_id: &str,
    tool_id: &str,
    tool_json: &str,
    duration_ms: Option<f64>,
) -> Result<(), String> {
    let conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;

    let rows = conn
        .execute(
            "UPDATE dialog_items
             SET tool_json = ?1, duration_ms = COALESCE(?2, duration_ms)
             WHERE task_id = ?3
               AND (id = ?4 OR (json_valid(tool_json) AND (
                   json_extract(tool_json, '$.id') = ?4
                   OR json_extract(tool_json, '$.params.callId') = ?4
               )))",
            params![tool_json, duration_ms, task_id, tool_id],
        )
        .map_err(|e| format!("Failed to update dialog tool result: {}", e))?;

    if rows == 0 {
        let new_item = ActivityItem {
            id: tool_id.to_string(),
            item_type: "tool".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            content: None,
            tool: serde_json::from_str(tool_json).ok(),
            duration_ms,
            images: None,
        };
        append_dialog_items(root, task_id, std::slice::from_ref(&new_item))?;
    }
    Ok(())
}

/// Pool-based variant for targeted tool result updates.
pub fn db_update_dialog_tool_result_pool(
    pool: &r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
    task_id: &str,
    tool_id: &str,
    tool_json: &str,
    duration_ms: f64,
) -> anyhow::Result<()> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!("Failed to acquire connection: {}", e))?;
    conn.execute(
        "UPDATE dialog_items
         SET tool_json = ?1, duration_ms = ?2
         WHERE task_id = ?3
           AND (id = ?4 OR (json_valid(tool_json) AND (
               json_extract(tool_json, '$.id') = ?4
               OR json_extract(tool_json, '$.params.callId') = ?4
           )))",
        params![tool_json, duration_ms, task_id, tool_id],
    )?;
    Ok(())
}

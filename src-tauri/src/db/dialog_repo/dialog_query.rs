use crate::db::get_db_conn;
use rusqlite::params;
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
    #[serde(default, alias = "duration_ms", skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
}

impl ActivityItem {
    pub fn is_runtime_error(&self) -> bool {
        self.item_type == "error" || (matches!(self.item_type.as_str(), "output" | "assistant")
            && self.content.as_deref().is_some_and(|text| text.trim_start().starts_with("[LLM Error:")))
    }
}

/// Loads all dialog items for a task ordered chronologically by created_at,
/// excluding compacted archived intermediate messages.
pub fn load_dialog_items(root: &Path, task_id: &str) -> Result<Vec<ActivityItem>, String> {
    load_dialog_items_after(root, task_id, 0)
}

pub fn load_dialog_items_after(root: &Path, task_id: &str, cursor: i64) -> Result<Vec<ActivityItem>, String> {
    load_dialog_items_between(root, task_id, cursor, i64::MAX)
}

pub fn load_dialog_items_between(root: &Path, task_id: &str, cursor: i64, end: i64) -> Result<Vec<ActivityItem>, String> {
    let conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, type, content, images_json, timestamp, tool_json, duration_ms
             FROM dialog_items
             WHERE task_id = ?1 AND type NOT IN ('compacted_archived', 'context_snapshot', 'deleted') AND rowid > ?2 AND rowid <= ?3
             ORDER BY created_at ASC, rowid ASC",
        )
        .map_err(|e| format!("Failed to prepare select statement: {}", e))?;

    let rows = stmt
        .query_map(params![task_id, cursor, end], |row| {
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

        let mut item = ActivityItem {
            id,
            item_type,
            timestamp,
            content,
            tool,
            duration_ms,
            images,
        };
        if item.is_runtime_error() { item.item_type = "error".to_string(); }
        items.push(item);
    }

    Ok(items)
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

/// Retrieves the tool JSON value for a specific tool ID in a task dialogue.
pub fn db_get_dialog_tool_json(root: &Path, task_id: &str, tool_id: &str) -> Option<serde_json::Value> {
    let conn = get_db_conn(root).ok()?;
    let like_pattern = format!("%{}%", tool_id);
    let raw: Option<String> = conn
        .query_row(
            "SELECT tool_json FROM dialog_items WHERE task_id = ?1 AND (id = ?2 OR tool_json LIKE ?3) LIMIT 1",
            params![task_id, tool_id, like_pattern],
            |row| row.get(0),
        )
        .ok()?;
    raw.and_then(|s| serde_json::from_str(&s).ok())
}

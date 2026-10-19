use rusqlite::{params, Connection};
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



/// Initializes SQLite database at <root>/.cvh/harness.db and creates tables and indexes.
pub fn init_db(root: &Path) -> Result<Connection, rusqlite::Error> {
    let db_dir = root.join(".cvh");
    if !db_dir.exists() {
        let _ = std::fs::create_dir_all(&db_dir);
    }
    let db_path = db_dir.join("harness.db");
    let conn = Connection::open(db_path)?;

    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS dialog_items (
             id TEXT PRIMARY KEY,
             task_id TEXT NOT NULL,
             type TEXT NOT NULL,
             content TEXT,
             images_json TEXT,
             timestamp TEXT,
             created_at INTEGER NOT NULL,
             tool_json TEXT,
             duration_ms REAL
         );
         CREATE INDEX IF NOT EXISTS idx_dialog_task ON dialog_items(task_id, created_at);",
    )?;

    // Ensure optional columns exist if table was initialized by an earlier schema
    let _ = conn.execute("ALTER TABLE dialog_items ADD COLUMN tool_json TEXT", []);
    let _ = conn.execute("ALTER TABLE dialog_items ADD COLUMN duration_ms REAL", []);

    Ok(conn)
}

/// Persists/appends dialog items for a task into SQLite atomically.
pub fn append_dialog_items(root: &Path, task_id: &str, items: &[ActivityItem]) -> Result<(), String> {
    if items.is_empty() {
        return Ok(());
    }

    let mut conn = init_db(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction()
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
    let conn = init_db(root).map_err(|e| format!("Failed to open database: {}", e))?;

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
    let mut conn = init_db(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction()
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

/// Deletes all dialog items for a specific task.
pub fn clear_dialog_items(root: &Path, task_id: &str) -> Result<(), String> {
    let conn = init_db(root).map_err(|e| format!("Failed to open database: {}", e))?;
    conn.execute(
        "DELETE FROM dialog_items WHERE task_id = ?1",
        params![task_id],
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
    let mut conn = init_db(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let p_clean = task_id.trim();
    if p_clean.is_empty() {
        return Ok(Vec::new());
    }

    let rows = if is_subagent {
        // Match specific subagent: exact, or with :sub: prefix/suffix
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
        // Match parent task and any child subagents
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
        .transaction()
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
/// Checks for keys ending with `.sub-{tool_call_id}` or containing `:sub:`.
pub fn find_subagent_dialog_key(
    root: &Path,
    parent_task_id: Option<&str>,
    tool_call_id: &str,
) -> Result<Option<String>, String> {
    let conn = init_db(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tc_clean = tool_call_id.trim();
    if tc_clean.is_empty() {
        return Ok(None);
    }

    // 1. Try parent-scoped suffix match: "{parent}:sub:%.sub-{tool_call_id}"
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

    // 2. Try general suffix match: "%.sub-{tool_call_id}"
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
    let conn = init_db(root).map_err(|e| format!("Failed to open database: {}", e))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_sqlite_dialog_roundtrip() {
        let root = PathBuf::from("target").join(format!("cvh_db_test_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&root);
        let root = root.as_path();

        let items = vec![
            ActivityItem {
                id: "user-100".to_string(),
                item_type: "user".to_string(),
                timestamp: "10:00:00".to_string(),
                content: Some("User prompt".to_string()),
                tool: None,
                duration_ms: None,
                images: Some(vec!["img1.png".to_string()]),
            },
            ActivityItem {
                id: "th-1710000000000-abcd".to_string(),
                item_type: "thought".to_string(),
                timestamp: "10:00:01".to_string(),
                content: Some("Thinking...".to_string()),
                tool: None,
                duration_ms: Some(500.0),
                images: None,
            },
            ActivityItem {
                id: "call-1".to_string(),
                item_type: "tool".to_string(),
                timestamp: "10:00:02".to_string(),
                content: None,
                tool: Some(serde_json::json!({
                    "id": "call-1",
                    "toolName": "read",
                    "status": "success"
                })),
                duration_ms: Some(42.0),
                images: None,
            },
        ];

        append_dialog_items(root, "CVH-TEST-1", &items).expect("append");

        let loaded = load_dialog_items(root, "CVH-TEST-1").expect("load");
        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0].id, "user-100");
        assert_eq!(loaded[0].images, Some(vec!["img1.png".to_string()]));
        assert_eq!(loaded[1].duration_ms, Some(500.0));
        assert_eq!(loaded[2].item_type, "tool");
        assert!(loaded[2].tool.is_some());

        // Update streaming content
        let mut updated = items.clone();
        updated[1].content = Some("Thinking deeper...".to_string());
        append_dialog_items(root, "CVH-TEST-1", &updated).expect("update");

        let loaded_updated = load_dialog_items(root, "CVH-TEST-1").expect("load updated");
        assert_eq!(loaded_updated.len(), 3);
        assert_eq!(loaded_updated[1].content.as_deref(), Some("Thinking deeper..."));

        // Clear
        clear_dialog_items(root, "CVH-TEST-1").expect("clear");
        let empty = load_dialog_items(root, "CVH-TEST-1").expect("load after clear");
        assert!(empty.is_empty());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_find_subagent_dialog_key() {
        let root = PathBuf::from("target").join(format!("cvh_db_test_find_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&root);
        let root = root.as_path();

        let sub_key = "CVH-42:sub:CVH-42.sub-call_xyz987";
        let item = ActivityItem {
            id: "sub-p-1".to_string(),
            item_type: "prompt".to_string(),
            timestamp: "10:00:00".to_string(),
            content: Some("Subagent prompt".to_string()),
            tool: None,
            duration_ms: None,
            images: None,
        };
        append_dialog_items(root, sub_key, &[item]).expect("append subagent item");

        // 1. Find by exact tool_call_id with parent_task_id
        let found1 = find_subagent_dialog_key(root, Some("CVH-42"), "call_xyz987").expect("find1");
        assert_eq!(found1.as_deref(), Some(sub_key));

        // 2. Find by tool_call_id without parent_task_id
        let found2 = find_subagent_dialog_key(root, None, "call_xyz987").expect("find2");
        assert_eq!(found2.as_deref(), Some(sub_key));

        // 3. Find when input has .sub- prefix
        let found3 = find_subagent_dialog_key(root, None, ".sub-call_xyz987").expect("find3");
        assert_eq!(found3.as_deref(), Some(sub_key));

        // 4. Non-existent returns None
        let not_found = find_subagent_dialog_key(root, Some("CVH-42"), "call_nonexistent").expect("find non");
        assert_eq!(not_found, None);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_mark_running_tools_as_aborted() {
        let root = PathBuf::from("target").join(format!("cvh_db_test_abort_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&root);
        let root = root.as_path();

        // 1. Add running tool to subagent
        let sub_key = "CVH-10:sub:CVH-10.sub-agent_1";
        let sub_tool = ActivityItem {
            id: "call-sub-run".to_string(),
            item_type: "tool".to_string(),
            timestamp: "12:00:00".to_string(),
            content: Some("running".to_string()),
            tool: Some(serde_json::json!({
                "id": "call-sub-run",
                "toolName": "glob",
                "status": "running"
            })),
            duration_ms: None,
            images: None,
        };
        append_dialog_items(root, sub_key, &[sub_tool]).expect("append sub tool");

        // 2. Abort subagent granularly
        let updated = mark_running_tools_as_aborted(root, "CVH-10.sub-agent_1", true).expect("abort sub");
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].2, "call-sub-run");
        assert_eq!(updated[0].3, "glob");

        // Verify status in DB
        let loaded = load_dialog_items(root, sub_key).expect("load sub");
        assert_eq!(loaded.len(), 1);
        let tool_val = loaded[0].tool.as_ref().expect("tool");
        assert_eq!(tool_val["status"], "failed");
        assert_eq!(tool_val["stderr"], "Прервано пользователем");

        // 3. Add running tool to parent task
        let parent_tool = ActivityItem {
            id: "call-parent-run".to_string(),
            item_type: "tool".to_string(),
            timestamp: "12:05:00".to_string(),
            content: Some("running".to_string()),
            tool: Some(serde_json::json!({
                "id": "call-parent-run",
                "toolName": "read_file",
                "status": "running"
            })),
            duration_ms: None,
            images: None,
        };
        append_dialog_items(root, "CVH-10", &[parent_tool]).expect("append parent tool");

        // 4. Abort parent task
        let updated_parent = mark_running_tools_as_aborted(root, "CVH-10", false).expect("abort parent");
        assert_eq!(updated_parent.len(), 1);
        assert_eq!(updated_parent[0].2, "call-parent-run");
        assert_eq!(updated_parent[0].3, "read_file");

        let loaded_parent = load_dialog_items(root, "CVH-10").expect("load parent");
        let parent_tool_val = loaded_parent[0].tool.as_ref().expect("parent tool");
        assert_eq!(parent_tool_val["status"], "failed");
        assert_eq!(parent_tool_val["stderr"], "Прервано пользователем");

        let _ = std::fs::remove_dir_all(root);
    }
}

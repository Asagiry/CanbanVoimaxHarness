use crate::agent::{Task, TodoItem};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerEntry {
    #[serde(default = "default_entry_id")]
    pub id: String,
    #[serde(rename = "taskId", alias = "task_id")]
    pub task_id: String,
    #[serde(default, rename = "commitHash", alias = "commit_hash")]
    pub commit_hash: Option<String>,
    pub summary: String,
    #[serde(default)]
    pub verdict: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub files: Option<Vec<String>>,
    #[serde(default, rename = "createdAt", alias = "created_at")]
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryRecord {
    #[serde(default = "default_entry_id")]
    pub id: String,
    #[serde(rename = "taskId", alias = "task_id")]
    pub task_id: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default, rename = "inputTokens", alias = "input_tokens")]
    pub input_tokens: i64,
    #[serde(default, rename = "outputTokens", alias = "output_tokens")]
    pub output_tokens: i64,
    #[serde(default, rename = "cachedTokens", alias = "cached_tokens")]
    pub cached_tokens: i64,
    #[serde(default)]
    pub tps: f64,
    #[serde(default, rename = "ttftMs", alias = "ttft_ms")]
    pub ttft_ms: f64,
    #[serde(default, rename = "durationMs", alias = "duration_ms")]
    pub duration_ms: f64,
    #[serde(default, rename = "createdAt", alias = "created_at")]
    pub created_at: i64,
}

fn default_entry_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Initializes SQLite database at <root>/.cvh/harness.db and creates tables and indexes.
pub fn init_db(root: &Path) -> Result<Connection, rusqlite::Error> {
    if root.as_os_str().is_empty() {
        return Err(rusqlite::Error::InvalidPath(root.to_path_buf()));
    }
    if !cfg!(test) && (crate::config::is_binary_or_target_dir(root) || crate::config::is_app_own_directory(root)) {
        return Err(rusqlite::Error::InvalidPath(root.to_path_buf()));
    }
    let db_dir = root.join(".cvh");
    if !db_dir.exists() {
        let _ = std::fs::create_dir_all(&db_dir);
    }
    let db_path = db_dir.join("harness.db");
    let mut conn = Connection::open(db_path)?;

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
         CREATE INDEX IF NOT EXISTS idx_dialog_task ON dialog_items(task_id, created_at);

         CREATE TABLE IF NOT EXISTS tasks (
             id TEXT PRIMARY KEY,
             title TEXT NOT NULL,
             description TEXT NOT NULL DEFAULT '',
             stage_id TEXT NOT NULL,
             board_id TEXT NOT NULL DEFAULT 'development',
             worktree_path TEXT,
             branch TEXT,
             verdict TEXT,
             custom_flow TEXT,
             status TEXT NOT NULL DEFAULT 'idle',
             model TEXT,
             parent_id TEXT,
             subtask_index INTEGER,
             waiting_for_subtask_id TEXT,
             handover_summary TEXT,
             cover_image TEXT,
             priority TEXT,
             current_tokens INTEGER,
             skip_worktree INTEGER NOT NULL DEFAULT 0,
             created_at INTEGER NOT NULL,
             updated_at INTEGER NOT NULL,
             FOREIGN KEY (parent_id) REFERENCES tasks(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_tasks_stage ON tasks(stage_id);
         CREATE INDEX IF NOT EXISTS idx_tasks_board ON tasks(board_id);
         CREATE INDEX IF NOT EXISTS idx_tasks_parent ON tasks(parent_id);

         CREATE TABLE IF NOT EXISTS task_todos (
             id TEXT NOT NULL,
             task_id TEXT NOT NULL,
             content TEXT NOT NULL,
             status TEXT NOT NULL DEFAULT 'pending',
             order_idx INTEGER NOT NULL DEFAULT 0,
             PRIMARY KEY (task_id, id),
             FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_todos_task ON task_todos(task_id);

         CREATE TABLE IF NOT EXISTS sequences (
             name TEXT PRIMARY KEY,
             current_val INTEGER NOT NULL DEFAULT 0
         );

         CREATE TABLE IF NOT EXISTS task_ledger (
             id TEXT PRIMARY KEY,
             task_id TEXT NOT NULL,
             commit_hash TEXT,
             summary TEXT NOT NULL,
             verdict TEXT,
             author TEXT,
             files_json TEXT,
             created_at INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_ledger_task ON task_ledger(task_id);

         CREATE TABLE IF NOT EXISTS telemetry (
             id TEXT PRIMARY KEY,
             task_id TEXT NOT NULL,
             model TEXT,
             input_tokens INTEGER DEFAULT 0,
             output_tokens INTEGER DEFAULT 0,
             cached_tokens INTEGER DEFAULT 0,
             tps REAL DEFAULT 0.0,
             ttft_ms REAL DEFAULT 0.0,
             duration_ms REAL DEFAULT 0.0,
             created_at INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_telemetry_task ON telemetry(task_id);",
    )?;

    // Ensure optional columns exist if table was initialized by an earlier schema
    let _ = conn.execute("ALTER TABLE dialog_items ADD COLUMN tool_json TEXT", []);
    let _ = conn.execute("ALTER TABLE dialog_items ADD COLUMN duration_ms REAL", []);
    let _ = conn.execute("ALTER TABLE tasks ADD COLUMN attachments TEXT", []);
    let _ = conn.execute("ALTER TABLE tasks ADD COLUMN tags TEXT", []);

    auto_migrate_if_needed(&mut conn, root);

    Ok(conn)
}

/// Seamless automatic data migration from legacy JSON files into SQLite tables.
fn auto_migrate_if_needed(conn: &mut Connection, root: &Path) {
    // 1. Tasks & Todos migration
    let task_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
        .unwrap_or(0);

    if task_count == 0 {
        let tasks_file = root.join(".cvh").join("tasks.json");
        let mut loaded_tasks: Option<Vec<Task>> = None;
        if tasks_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&tasks_file) {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    if let Ok(parsed) = serde_json::from_str::<Vec<Task>>(trimmed) {
                        if !parsed.is_empty() {
                            loaded_tasks = Some(parsed);
                        }
                    }
                }
            }
        }
        if loaded_tasks.is_none() {
            let backup_file = crate::commands::tasks::get_project_tasks_backup_file(root);
            if backup_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&backup_file) {
                    let trimmed = content.trim();
                    if !trimmed.is_empty() {
                        if let Ok(parsed) = serde_json::from_str::<Vec<Task>>(trimmed) {
                            if !parsed.is_empty() {
                                loaded_tasks = Some(parsed);
                            }
                        }
                    }
                }
            }
        }

        if let Some(tasks) = loaded_tasks {
            if let Ok(tx) = conn.transaction() {
                let _ = tx.execute("PRAGMA foreign_keys = OFF", []);
                for t in &tasks {
                    let custom_flow_json = t.custom_flow.as_ref().and_then(|v| serde_json::to_string(v).ok());
                    let attachments_json = t.attachments.as_ref().and_then(|v| serde_json::to_string(v).ok());
                    let tags_json = t.tags.as_ref().and_then(|v| serde_json::to_string(v).ok());
                    let skip_wt = if t.skip_worktree.unwrap_or(false) { 1i64 } else { 0i64 };
                    let cur_tokens = t.current_tokens.map(|v| v as i64);
                    let sub_idx = t.subtask_index.map(|v| v as i64);

                    let _ = tx.execute(
                        "INSERT OR REPLACE INTO tasks (
                            id, title, description, stage_id, board_id, worktree_path, branch, verdict,
                            custom_flow, status, model, parent_id, subtask_index, waiting_for_subtask_id,
                            handover_summary, cover_image, priority, current_tokens, skip_worktree,
                            created_at, updated_at, attachments, tags
                        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)",
                        params![
                            t.id,
                            t.title,
                            t.description,
                            t.stage_id,
                            t.board_id.as_deref().unwrap_or("development"),
                            t.worktree_path,
                            t.branch,
                            t.verdict,
                            custom_flow_json,
                            t.status.as_deref().unwrap_or("idle"),
                            t.model,
                            t.parent_id,
                            sub_idx,
                            t.waiting_for_subtask_id,
                            t.handover_summary,
                            t.cover_image,
                            t.priority,
                            cur_tokens,
                            skip_wt,
                            t.created_at,
                            t.updated_at,
                            attachments_json,
                            tags_json,
                        ],
                    );

                    if let Some(ref todos) = t.todos {
                        for (idx, td) in todos.iter().enumerate() {
                            let _ = tx.execute(
                                "INSERT OR REPLACE INTO task_todos (id, task_id, content, status, order_idx)
                                 VALUES (?1, ?2, ?3, ?4, ?5)",
                                params![td.id, t.id, td.content, td.status, idx as i64],
                            );
                        }
                    }
                }
                let _ = tx.execute("PRAGMA foreign_keys = ON", []);
                let _ = tx.commit();
            }
        }
    }

    // 2. Sequence migration
    let seq_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sequences WHERE name = 'task'", [], |r| r.get(0))
        .unwrap_or(0);

    if seq_count == 0 {
        let seq_paths = [
            root.join(".cvh").join("settings").join("sequence.json"),
            root.join(".cvh").join("task-sequence.json"),
        ];
        let mut last_num: u32 = 0;
        for p in &seq_paths {
            if p.exists() {
                if let Ok(c) = std::fs::read_to_string(p) {
                    let trimmed = c.trim();
                    if !trimmed.is_empty() {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                            if let Some(n) = val.get("last_task_num").and_then(|v| v.as_u64()) {
                                if n > 0 {
                                    last_num = n as u32;
                                    break;
                                }
                            }
                            if let Some(n) = val.get("sequence").and_then(|v| v.as_u64()) {
                                if n > 0 {
                                    last_num = n as u32;
                                    break;
                                }
                            }
                        }
                        if let Ok(n) = trimmed.parse::<u32>() {
                            if n > 0 {
                                last_num = n;
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Also inspect existing task numbers in tasks table
        if let Ok(mut stmt) = conn.prepare("SELECT id FROM tasks") {
            if let Ok(rows) = stmt.query_map([], |r| r.get::<_, String>(0)) {
                for id_res in rows.flatten() {
                    let base = id_res.split('.').next().unwrap_or(&id_res);
                    let digits: String = base.chars().filter(|c| c.is_ascii_digit()).collect();
                    if let Ok(n) = digits.parse::<u32>() {
                        if n > last_num {
                            last_num = n;
                        }
                    }
                }
            }
        }

        if last_num > 0 {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO sequences (name, current_val) VALUES ('task', ?1)",
                params![last_num as i64],
            );
        }
    }

    // 3. Ledger migration
    let ledger_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM task_ledger", [], |r| r.get(0))
        .unwrap_or(0);

    if ledger_count == 0 {
        let ledger_paths = [
            root.join(".cvh").join("tasks-history.json"),
            root.join(".agents").join("tasks-history.json"),
        ];
        for p in &ledger_paths {
            if p.exists() {
                if let Ok(c) = std::fs::read_to_string(p) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&c) {
                        if let Some(obj) = val.as_object() {
                            for (key, item) in obj {
                                let task_id = item.get("taskId").and_then(|v| v.as_str()).unwrap_or(key).to_string();
                                let commit_hash = item.get("commitHash").and_then(|v| v.as_str()).map(|s| s.to_string());
                                let summary = item.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let verdict = item.get("verdict").and_then(|v| v.as_str()).map(|s| s.to_string());
                                let author = item.get("author").and_then(|v| v.as_str()).map(|s| s.to_string());
                                let files_json = item.get("files").map(|v| v.to_string());
                                let created_at = item.get("timestamp").and_then(|v| v.as_i64()).unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
                                let id = format!("ledger-{}", key);
                                let _ = conn.execute(
                                    "INSERT OR IGNORE INTO task_ledger (id, task_id, commit_hash, summary, verdict, author, files_json, created_at)
                                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                                    params![id, task_id, commit_hash, summary, verdict, author, files_json, created_at],
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Retrieves all tasks from SQLite ordered chronologically by created_at.
pub fn db_get_all_tasks(root: &Path) -> Result<Vec<Task>, String> {
    let conn = init_db(root).map_err(|e| format!("Failed to open database: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, title, description, stage_id, board_id, worktree_path, branch, verdict,
                    custom_flow, status, model, parent_id, subtask_index, waiting_for_subtask_id,
                    handover_summary, cover_image, priority, current_tokens, skip_worktree,
                    created_at, updated_at, attachments, tags
             FROM tasks
             ORDER BY created_at ASC",
        )
        .map_err(|e| format!("Prepare select tasks failed: {}", e))?;

    let task_rows = stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let description: String = row.get(2)?;
            let stage_id: String = row.get(3)?;
            let board_id: Option<String> = row.get(4)?;
            let worktree_path: Option<String> = row.get(5)?;
            let branch: Option<String> = row.get(6)?;
            let verdict: Option<String> = row.get(7)?;
            let custom_flow_raw: Option<String> = row.get(8)?;
            let status: Option<String> = row.get(9)?;
            let model: Option<String> = row.get(10)?;
            let parent_id: Option<String> = row.get(11)?;
            let subtask_index_i64: Option<i64> = row.get(12)?;
            let waiting_for_subtask_id: Option<String> = row.get(13)?;
            let handover_summary: Option<String> = row.get(14)?;
            let cover_image: Option<String> = row.get(15)?;
            let priority: Option<String> = row.get(16)?;
            let current_tokens_i64: Option<i64> = row.get(17)?;
            let skip_worktree_i64: i64 = row.get(18)?;
            let created_at: i64 = row.get(19)?;
            let updated_at: i64 = row.get(20)?;
            let attachments_raw: Option<String> = row.get(21).ok();
            let tags_raw: Option<String> = row.get(22).ok();

            Ok((
                id, title, description, stage_id, board_id, worktree_path, branch, verdict,
                custom_flow_raw, status, model, parent_id, subtask_index_i64, waiting_for_subtask_id,
                handover_summary, cover_image, priority, current_tokens_i64, skip_worktree_i64,
                created_at, updated_at, attachments_raw, tags_raw,
            ))
        })
        .map_err(|e| format!("Query tasks failed: {}", e))?;

    let mut todos_by_task: HashMap<String, Vec<TodoItem>> = HashMap::new();
    let mut todo_stmt = conn
        .prepare("SELECT id, task_id, content, status FROM task_todos ORDER BY order_idx ASC")
        .map_err(|e| format!("Prepare select todos failed: {}", e))?;

    let todo_rows = todo_stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let task_id: String = row.get(1)?;
            let content: String = row.get(2)?;
            let status: String = row.get(3)?;
            Ok((task_id, TodoItem { id, content, status }))
        })
        .map_err(|e| format!("Query todos failed: {}", e))?;

    for tr in todo_rows.flatten() {
        todos_by_task.entry(tr.0).or_default().push(tr.1);
    }

    let mut tasks = Vec::new();
    for row_res in task_rows {
        let (
            id, title, description, stage_id, board_id, worktree_path, branch, verdict,
            custom_flow_raw, status, model, parent_id, subtask_index_i64, waiting_for_subtask_id,
            handover_summary, cover_image, priority, current_tokens_i64, skip_worktree_i64,
            created_at, updated_at, attachments_raw, tags_raw,
        ) = row_res.map_err(|e| format!("Row extraction failed: {}", e))?;

        let custom_flow = custom_flow_raw.and_then(|s| serde_json::from_str(&s).ok());
        let attachments = attachments_raw.and_then(|s| serde_json::from_str(&s).ok());
        let tags = tags_raw.and_then(|s| serde_json::from_str(&s).ok());
        let todos = todos_by_task.remove(&id);

        tasks.push(Task {
            id,
            title,
            description,
            stage_id,
            board_id,
            worktree_path,
            branch,
            verdict,
            custom_flow,
            status,
            model,
            parent_id,
            subtask_index: subtask_index_i64.map(|v| v as u32),
            waiting_for_subtask_id,
            handover_summary,
            cover_image,
            attachments,
            todos,
            priority,
            tags,
            current_tokens: current_tokens_i64.map(|v| v as usize),
            skip_worktree: Some(skip_worktree_i64 != 0),
            created_at,
            updated_at,
        });
    }

    Ok(tasks)
}

/// Retrieves a single task by ID from SQLite.
pub fn db_get_task(root: &Path, task_id: &str) -> Result<Option<Task>, String> {
    let conn = init_db(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, title, description, stage_id, board_id, worktree_path, branch, verdict,
                    custom_flow, status, model, parent_id, subtask_index, waiting_for_subtask_id,
                    handover_summary, cover_image, priority, current_tokens, skip_worktree,
                    created_at, updated_at, attachments, tags
             FROM tasks
             WHERE id = ?1 OR LOWER(id) = LOWER(?1)
             LIMIT 1",
        )
        .map_err(|e| format!("Prepare select task failed: {}", e))?;

    let mut rows = stmt
        .query(params![task_id])
        .map_err(|e| format!("Query task failed: {}", e))?;

    let row = match rows.next().map_err(|e| format!("Next row failed: {}", e))? {
        Some(r) => r,
        None => return Ok(None),
    };

    let id: String = row.get(0).map_err(|e| e.to_string())?;
    let title: String = row.get(1).map_err(|e| e.to_string())?;
    let description: String = row.get(2).map_err(|e| e.to_string())?;
    let stage_id: String = row.get(3).map_err(|e| e.to_string())?;
    let board_id: Option<String> = row.get(4).map_err(|e| e.to_string())?;
    let worktree_path: Option<String> = row.get(5).map_err(|e| e.to_string())?;
    let branch: Option<String> = row.get(6).map_err(|e| e.to_string())?;
    let verdict: Option<String> = row.get(7).map_err(|e| e.to_string())?;
    let custom_flow_raw: Option<String> = row.get(8).map_err(|e| e.to_string())?;
    let status: Option<String> = row.get(9).map_err(|e| e.to_string())?;
    let model: Option<String> = row.get(10).map_err(|e| e.to_string())?;
    let parent_id: Option<String> = row.get(11).map_err(|e| e.to_string())?;
    let subtask_index_i64: Option<i64> = row.get(12).map_err(|e| e.to_string())?;
    let waiting_for_subtask_id: Option<String> = row.get(13).map_err(|e| e.to_string())?;
    let handover_summary: Option<String> = row.get(14).map_err(|e| e.to_string())?;
    let cover_image: Option<String> = row.get(15).map_err(|e| e.to_string())?;
    let priority: Option<String> = row.get(16).map_err(|e| e.to_string())?;
    let current_tokens_i64: Option<i64> = row.get(17).map_err(|e| e.to_string())?;
    let skip_worktree_i64: i64 = row.get(18).map_err(|e| e.to_string())?;
    let created_at: i64 = row.get(19).map_err(|e| e.to_string())?;
    let updated_at: i64 = row.get(20).map_err(|e| e.to_string())?;
    let attachments_raw: Option<String> = row.get(21).ok();
    let tags_raw: Option<String> = row.get(22).ok();

    // Load todos for this task
    let mut todo_stmt = conn
        .prepare("SELECT id, content, status FROM task_todos WHERE task_id = ?1 ORDER BY order_idx ASC")
        .map_err(|e| format!("Prepare select todos failed: {}", e))?;
    let todo_rows = todo_stmt
        .query_map(params![&id], |r| {
            Ok(TodoItem {
                id: r.get(0)?,
                content: r.get(1)?,
                status: r.get(2)?,
            })
        })
        .map_err(|e| format!("Query todos failed: {}", e))?;

    let todos_vec: Vec<TodoItem> = todo_rows.flatten().collect();
    let todos = if todos_vec.is_empty() { None } else { Some(todos_vec) };

    let custom_flow = custom_flow_raw.and_then(|s| serde_json::from_str(&s).ok());
    let attachments = attachments_raw.and_then(|s| serde_json::from_str(&s).ok());
    let tags = tags_raw.and_then(|s| serde_json::from_str(&s).ok());

    Ok(Some(Task {
        id,
        title,
        description,
        stage_id,
        board_id,
        worktree_path,
        branch,
        verdict,
        custom_flow,
        status,
        model,
        parent_id,
        subtask_index: subtask_index_i64.map(|v| v as u32),
        waiting_for_subtask_id,
        handover_summary,
        cover_image,
        attachments,
        todos,
        priority,
        tags,
        current_tokens: current_tokens_i64.map(|v| v as usize),
        skip_worktree: Some(skip_worktree_i64 != 0),
        created_at,
        updated_at,
    }))
}

/// Atomically upserts a single task and its checklist items into SQLite.
pub fn db_upsert_task(root: &Path, task: &Task) -> Result<(), String> {
    let mut conn = init_db(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    let custom_flow_json = task.custom_flow.as_ref().and_then(|v| serde_json::to_string(v).ok());
    let attachments_json = task.attachments.as_ref().and_then(|v| serde_json::to_string(v).ok());
    let tags_json = task.tags.as_ref().and_then(|v| serde_json::to_string(v).ok());
    let skip_wt = if task.skip_worktree.unwrap_or(false) { 1i64 } else { 0i64 };
    let cur_tokens = task.current_tokens.map(|v| v as i64);
    let sub_idx = task.subtask_index.map(|v| v as i64);

    tx.execute(
        "INSERT INTO tasks (
            id, title, description, stage_id, board_id, worktree_path, branch, verdict,
            custom_flow, status, model, parent_id, subtask_index, waiting_for_subtask_id,
            handover_summary, cover_image, priority, current_tokens, skip_worktree,
            created_at, updated_at, attachments, tags
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)
        ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            description = excluded.description,
            stage_id = excluded.stage_id,
            board_id = excluded.board_id,
            worktree_path = excluded.worktree_path,
            branch = excluded.branch,
            verdict = excluded.verdict,
            custom_flow = excluded.custom_flow,
            status = excluded.status,
            model = excluded.model,
            parent_id = excluded.parent_id,
            subtask_index = excluded.subtask_index,
            waiting_for_subtask_id = excluded.waiting_for_subtask_id,
            handover_summary = excluded.handover_summary,
            cover_image = excluded.cover_image,
            priority = excluded.priority,
            current_tokens = excluded.current_tokens,
            skip_worktree = excluded.skip_worktree,
            updated_at = excluded.updated_at,
            attachments = excluded.attachments,
            tags = excluded.tags",
        params![
            task.id,
            task.title,
            task.description,
            task.stage_id,
            task.board_id.as_deref().unwrap_or("development"),
            task.worktree_path,
            task.branch,
            task.verdict,
            custom_flow_json,
            task.status.as_deref().unwrap_or("idle"),
            task.model,
            task.parent_id,
            sub_idx,
            task.waiting_for_subtask_id,
            task.handover_summary,
            task.cover_image,
            task.priority,
            cur_tokens,
            skip_wt,
            task.created_at,
            task.updated_at,
            attachments_json,
            tags_json,
        ],
    )
    .map_err(|e| format!("Failed to upsert task {}: {}", task.id, e))?;

    if let Some(ref todos) = task.todos {
        let _ = tx.execute("DELETE FROM task_todos WHERE task_id = ?1", params![task.id]);
        for (idx, td) in todos.iter().enumerate() {
            tx.execute(
                "INSERT INTO task_todos (id, task_id, content, status, order_idx)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![td.id, task.id, td.content, td.status, idx as i64],
            )
            .map_err(|e| format!("Failed to insert todo {}: {}", td.id, e))?;
        }
    }

    tx.commit()
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;
    Ok(())
}

/// Atomically upserts a batch of tasks into SQLite in a single transaction.
pub fn db_upsert_all_tasks(root: &Path, tasks: &[Task]) -> Result<(), String> {
    let mut conn = init_db(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    // Disable foreign keys temporarily during bulk batch update to accommodate unordered cascades
    let _ = tx.execute("PRAGMA foreign_keys = OFF", []);

    for task in tasks {
        let custom_flow_json = task.custom_flow.as_ref().and_then(|v| serde_json::to_string(v).ok());
        let attachments_json = task.attachments.as_ref().and_then(|v| serde_json::to_string(v).ok());
        let tags_json = task.tags.as_ref().and_then(|v| serde_json::to_string(v).ok());
        let skip_wt = if task.skip_worktree.unwrap_or(false) { 1i64 } else { 0i64 };
        let cur_tokens = task.current_tokens.map(|v| v as i64);
        let sub_idx = task.subtask_index.map(|v| v as i64);

        tx.execute(
            "INSERT INTO tasks (
                id, title, description, stage_id, board_id, worktree_path, branch, verdict,
                custom_flow, status, model, parent_id, subtask_index, waiting_for_subtask_id,
                handover_summary, cover_image, priority, current_tokens, skip_worktree,
                created_at, updated_at, attachments, tags
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                description = excluded.description,
                stage_id = excluded.stage_id,
                board_id = excluded.board_id,
                worktree_path = excluded.worktree_path,
                branch = excluded.branch,
                verdict = excluded.verdict,
                custom_flow = excluded.custom_flow,
                status = excluded.status,
                model = excluded.model,
                parent_id = excluded.parent_id,
                subtask_index = excluded.subtask_index,
                waiting_for_subtask_id = excluded.waiting_for_subtask_id,
                handover_summary = excluded.handover_summary,
                cover_image = excluded.cover_image,
                priority = excluded.priority,
                current_tokens = excluded.current_tokens,
                skip_worktree = excluded.skip_worktree,
                updated_at = excluded.updated_at,
                attachments = excluded.attachments,
                tags = excluded.tags",
            params![
                task.id,
                task.title,
                task.description,
                task.stage_id,
                task.board_id.as_deref().unwrap_or("development"),
                task.worktree_path,
                task.branch,
                task.verdict,
                custom_flow_json,
                task.status.as_deref().unwrap_or("idle"),
                task.model,
                task.parent_id,
                sub_idx,
                task.waiting_for_subtask_id,
                task.handover_summary,
                task.cover_image,
                task.priority,
                cur_tokens,
                skip_wt,
                task.created_at,
                task.updated_at,
                attachments_json,
                tags_json,
            ],
        )
        .map_err(|e| format!("Failed to upsert task {}: {}", task.id, e))?;

        if let Some(ref todos) = task.todos {
            let _ = tx.execute("DELETE FROM task_todos WHERE task_id = ?1", params![task.id]);
            for (idx, td) in todos.iter().enumerate() {
                tx.execute(
                    "INSERT INTO task_todos (id, task_id, content, status, order_idx)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![td.id, task.id, td.content, td.status, idx as i64],
                )
                .map_err(|e| format!("Failed to insert todo {}: {}", td.id, e))?;
            }
        }
    }

    let _ = tx.execute("PRAGMA foreign_keys = ON", []);
    tx.commit()
        .map_err(|e| format!("Failed to commit batch task upsert: {}", e))?;
    Ok(())
}

/// Deletes a task and all associated cascaded subtasks and todos from SQLite.
pub fn db_delete_task(root: &Path, task_id: &str) -> Result<(), String> {
    let mut conn = init_db(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    // 1. Auto-unlock any parent waiting on this task
    let _ = tx.execute(
        "UPDATE tasks
         SET waiting_for_subtask_id = NULL,
             status = CASE WHEN status = 'waiting_subtask' THEN 'idle' ELSE status END
         WHERE waiting_for_subtask_id = ?1 OR LOWER(waiting_for_subtask_id) = LOWER(?1)",
        params![task_id],
    );

    // 2. Cascade delete subtasks where parent_id matches or id starts with "{task_id}."
    let subtask_pattern = format!("{}.%", task_id);
    let _ = tx.execute(
        "DELETE FROM task_todos WHERE task_id IN (
            SELECT id FROM tasks WHERE parent_id = ?1 OR LOWER(parent_id) = LOWER(?1) OR id LIKE ?2
        )",
        params![task_id, subtask_pattern],
    );
    let _ = tx.execute(
        "DELETE FROM tasks WHERE parent_id = ?1 OR LOWER(parent_id) = LOWER(?1) OR id LIKE ?2",
        params![task_id, subtask_pattern],
    );

    // 3. Delete target task and its todos
    let _ = tx.execute("DELETE FROM task_todos WHERE task_id = ?1 OR LOWER(task_id) = LOWER(?1)", params![task_id]);
    tx.execute("DELETE FROM tasks WHERE id = ?1 OR LOWER(id) = LOWER(?1)", params![task_id])
        .map_err(|e| format!("Failed to delete task {}: {}", task_id, e))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit delete transaction: {}", e))?;
    Ok(())
}

/// Allocates the next monotonic task number from SQLite sequences table.
pub fn db_allocate_next_task_num(root: &Path) -> Result<u32, String> {
    let mut conn = init_db(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction()
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
    let conn = init_db(root).map_err(|e| format!("Failed to open database: {}", e))?;
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

/// Atomically saves the checklist (todos) for a specific task.
pub fn db_save_todos(root: &Path, task_id: &str, todos: &[TodoItem]) -> Result<(), String> {
    let mut conn = init_db(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    tx.execute(
        "DELETE FROM task_todos WHERE task_id = ?1 OR LOWER(task_id) = LOWER(?1)",
        params![task_id],
    )
    .map_err(|e| format!("Failed to clear existing todos: {}", e))?;

    for (idx, td) in todos.iter().enumerate() {
        tx.execute(
            "INSERT INTO task_todos (id, task_id, content, status, order_idx)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![td.id, task_id, td.content, td.status, idx as i64],
        )
        .map_err(|e| format!("Failed to insert todo {}: {}", td.id, e))?;
    }

    let now = chrono::Utc::now().timestamp_millis();
    let _ = tx.execute(
        "UPDATE tasks SET updated_at = ?1 WHERE id = ?2 OR LOWER(id) = LOWER(?2)",
        params![now, task_id],
    );

    tx.commit()
        .map_err(|e| format!("Failed to commit save todos transaction: {}", e))?;
    Ok(())
}

/// Records a telemetry metric row into SQLite table.
pub fn db_record_telemetry(root: &Path, record: &TelemetryRecord) -> Result<(), String> {
    let conn = init_db(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let id = if record.id.trim().is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        record.id.clone()
    };
    let created_at = if record.created_at == 0 {
        chrono::Utc::now().timestamp_millis()
    } else {
        record.created_at
    };
    conn.execute(
        "INSERT INTO telemetry (
            id, task_id, model, input_tokens, output_tokens, cached_tokens,
            tps, ttft_ms, duration_ms, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            id,
            record.task_id,
            record.model,
            record.input_tokens,
            record.output_tokens,
            record.cached_tokens,
            record.tps,
            record.ttft_ms,
            record.duration_ms,
            created_at,
        ],
    )
    .map_err(|e| format!("Failed to record telemetry: {}", e))?;
    Ok(())
}

/// Retrieves telemetry records for a specific task.
pub fn db_get_telemetry_for_task(root: &Path, task_id: &str) -> Result<Vec<TelemetryRecord>, String> {
    let conn = init_db(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, task_id, model, input_tokens, output_tokens, cached_tokens, tps, ttft_ms, duration_ms, created_at
             FROM telemetry
             WHERE task_id = ?1 OR LOWER(task_id) = LOWER(?1)
             ORDER BY created_at ASC",
        )
        .map_err(|e| format!("Prepare failed: {}", e))?;

    let rows = stmt
        .query_map(params![task_id], |r| {
            Ok(TelemetryRecord {
                id: r.get(0)?,
                task_id: r.get(1)?,
                model: r.get(2)?,
                input_tokens: r.get(3)?,
                output_tokens: r.get(4)?,
                cached_tokens: r.get(5)?,
                tps: r.get(6)?,
                ttft_ms: r.get(7)?,
                duration_ms: r.get(8)?,
                created_at: r.get(9)?,
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?;

    Ok(rows.flatten().collect())
}

/// Records or updates an engineering ledger entry in SQLite.
pub fn db_record_ledger_entry(root: &Path, entry: &LedgerEntry) -> Result<(), String> {
    let conn = init_db(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let id = if entry.id.trim().is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        entry.id.clone()
    };
    let created_at = if entry.created_at == 0 {
        chrono::Utc::now().timestamp_millis()
    } else {
        entry.created_at
    };
    let files_json = entry.files.as_ref().and_then(|f| serde_json::to_string(f).ok());
    conn.execute(
        "INSERT INTO task_ledger (
            id, task_id, commit_hash, summary, verdict, author, files_json, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(id) DO UPDATE SET
            task_id = excluded.task_id,
            commit_hash = excluded.commit_hash,
            summary = excluded.summary,
            verdict = excluded.verdict,
            author = excluded.author,
            files_json = excluded.files_json,
            created_at = excluded.created_at",
        params![
            id,
            entry.task_id,
            entry.commit_hash,
            entry.summary,
            entry.verdict,
            entry.author,
            files_json,
            created_at,
        ],
    )
    .map_err(|e| format!("Failed to record ledger entry: {}", e))?;
    Ok(())
}

/// Retrieves ledger entries for a specific task.
pub fn db_get_ledger_for_task(root: &Path, task_id: &str) -> Result<Vec<LedgerEntry>, String> {
    let conn = init_db(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, task_id, commit_hash, summary, verdict, author, files_json, created_at
             FROM task_ledger
             WHERE task_id = ?1 OR LOWER(task_id) = LOWER(?1)
             ORDER BY created_at ASC",
        )
        .map_err(|e| format!("Prepare failed: {}", e))?;

    let rows = stmt
        .query_map(params![task_id], |r| {
            let files_raw: Option<String> = r.get(6)?;
            let files = files_raw.and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok());
            Ok(LedgerEntry {
                id: r.get(0)?,
                task_id: r.get(1)?,
                commit_hash: r.get(2)?,
                summary: r.get(3)?,
                verdict: r.get(4)?,
                author: r.get(5)?,
                files,
                created_at: r.get(7)?,
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?;

    Ok(rows.flatten().collect())
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
    fn test_tasks_crud_and_todos() {
        let root = PathBuf::from("target").join(format!("cvh_db_test_tasks_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&root);
        let root = root.as_path();

        let task1 = Task {
            id: "CVH-010".to_string(),
            title: "Task 10".to_string(),
            description: "Initial description".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: Some("worktrees/cvh-010".to_string()),
            branch: Some("feat/cvh-010".to_string()),
            verdict: None,
            custom_flow: None,
            status: Some("idle".to_string()),
            model: Some("gemini-3.8-flash-high".to_string()),
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            handover_summary: None,
            cover_image: None,
            attachments: None,
            todos: Some(vec![
                TodoItem {
                    id: "td-1".to_string(),
                    content: "Todo 1".to_string(),
                    status: "pending".to_string(),
                },
                TodoItem {
                    id: "td-2".to_string(),
                    content: "Todo 2".to_string(),
                    status: "completed".to_string(),
                },
            ]),
            priority: Some("high".to_string()),
            tags: Some(vec!["backend".to_string(), "sqlite".to_string()]),
            current_tokens: Some(1500),
            skip_worktree: Some(false),
            created_at: 1000,
            updated_at: 1000,
        };

        db_upsert_task(root, &task1).expect("upsert task1");

        let loaded = db_get_task(root, "CVH-010").expect("get task").expect("task found");
        assert_eq!(loaded.id, "CVH-010");
        assert_eq!(loaded.title, "Task 10");
        assert_eq!(loaded.tags, Some(vec!["backend".to_string(), "sqlite".to_string()]));
        assert_eq!(loaded.current_tokens, Some(1500));
        let todos = loaded.todos.expect("todos present");
        assert_eq!(todos.len(), 2);
        assert_eq!(todos[0].id, "td-1");
        assert_eq!(todos[1].status, "completed");

        // Save todos
        let new_todos = vec![
            TodoItem {
                id: "td-1".to_string(),
                content: "Todo 1 updated".to_string(),
                status: "completed".to_string(),
            },
            TodoItem {
                id: "td-3".to_string(),
                content: "Todo 3 new".to_string(),
                status: "in_progress".to_string(),
            },
        ];
        db_save_todos(root, "CVH-010", &new_todos).expect("save todos");

        let loaded2 = db_get_task(root, "CVH-010").expect("get task 2").expect("task found");
        let todos2 = loaded2.todos.expect("todos2 present");
        assert_eq!(todos2.len(), 2);
        assert_eq!(todos2[0].content, "Todo 1 updated");
        assert_eq!(todos2[1].status, "in_progress");

        // Subtask with foreign key
        let subtask = Task {
            id: "CVH-010.1".to_string(),
            title: "Subtask 1".to_string(),
            description: "".to_string(),
            stage_id: "review".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("idle".to_string()),
            model: None,
            parent_id: Some("CVH-010".to_string()),
            subtask_index: Some(1),
            waiting_for_subtask_id: None,
            handover_summary: None,
            cover_image: None,
            attachments: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: Some(false),
            created_at: 1010,
            updated_at: 1010,
        };
        db_upsert_task(root, &subtask).expect("upsert subtask");

        let all_tasks = db_get_all_tasks(root).expect("get all tasks");
        assert_eq!(all_tasks.len(), 2);

        // Delete parent task -> cascade deletes subtask
        db_delete_task(root, "CVH-010").expect("delete parent");
        let all_after_delete = db_get_all_tasks(root).expect("get all after delete");
        assert_eq!(all_after_delete.len(), 0);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_sequence_allocation() {
        let root = PathBuf::from("target").join(format!("cvh_db_test_seq_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&root);
        let root = root.as_path();

        let num1 = db_allocate_next_task_num(root).expect("alloc 1");
        assert_eq!(num1, 1);

        let num2 = db_allocate_next_task_num(root).expect("alloc 2");
        assert_eq!(num2, 2);

        // Add task with higher number
        let high_task = Task {
            id: "CVH-007".to_string(),
            title: "Task 7".to_string(),
            description: "".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: None,
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            handover_summary: None,
            cover_image: None,
            attachments: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: None,
            created_at: 1000,
            updated_at: 1000,
        };
        db_upsert_task(root, &high_task).expect("upsert high task");

        let num3 = db_allocate_next_task_num(root).expect("alloc 3");
        assert_eq!(num3, 8);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_telemetry_and_ledger_persistence() {
        let root = PathBuf::from("target").join(format!("cvh_db_test_telem_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&root);
        let root = root.as_path();

        let telem = TelemetryRecord {
            id: "telem-1".to_string(),
            task_id: "CVH-042".to_string(),
            model: Some("gemini-3.8-flash-high".to_string()),
            input_tokens: 1200,
            output_tokens: 450,
            cached_tokens: 800,
            tps: 84.5,
            ttft_ms: 120.0,
            duration_ms: 5320.0,
            created_at: 2000,
        };
        db_record_telemetry(root, &telem).expect("record telemetry");

        let loaded_telem = db_get_telemetry_for_task(root, "CVH-042").expect("get telemetry");
        assert_eq!(loaded_telem.len(), 1);
        assert_eq!(loaded_telem[0].id, "telem-1");
        assert_eq!(loaded_telem[0].input_tokens, 1200);
        assert_eq!(loaded_telem[0].tps, 84.5);

        let ledger = LedgerEntry {
            id: "led-1".to_string(),
            task_id: "CVH-042".to_string(),
            commit_hash: Some("abcdef1".to_string()),
            summary: "Implemented SQLite tasks storage".to_string(),
            verdict: Some("APPROVED".to_string()),
            author: Some("agent-developer".to_string()),
            files: Some(vec!["src-tauri/src/db.rs".to_string()]),
            created_at: 2100,
        };
        db_record_ledger_entry(root, &ledger).expect("record ledger");

        let loaded_ledger = db_get_ledger_for_task(root, "CVH-042").expect("get ledger");
        assert_eq!(loaded_ledger.len(), 1);
        assert_eq!(loaded_ledger[0].summary, "Implemented SQLite tasks storage");
        assert_eq!(loaded_ledger[0].files, Some(vec!["src-tauri/src/db.rs".to_string()]));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_auto_migration_from_json() {
        let root = PathBuf::from("target").join(format!("cvh_db_test_mig_{}", uuid::Uuid::new_v4()));
        let cvh_dir = root.join(".cvh");
        let settings_dir = cvh_dir.join("settings");
        let _ = std::fs::create_dir_all(&settings_dir);
        let root = root.as_path();

        // 1. Seed tasks.json
        let sample_tasks = serde_json::json!([
            {
                "id": "CVH-001",
                "title": "Migrated Task",
                "description": "From JSON",
                "stageId": "dev",
                "boardId": "development",
                "todos": [
                    { "id": "td-m1", "content": "Migrate check", "status": "completed" }
                ],
                "createdAt": 100,
                "updatedAt": 200
            }
        ]);
        std::fs::write(cvh_dir.join("tasks.json"), sample_tasks.to_string()).expect("write tasks.json");

        // 2. Seed sequence.json
        let sample_seq = serde_json::json!({ "last_task_num": 1 });
        std::fs::write(settings_dir.join("sequence.json"), sample_seq.to_string()).expect("write sequence.json");

        // 3. Open DB -> should automatically migrate
        let conn = init_db(root).expect("init_db");
        drop(conn);

        let tasks = db_get_all_tasks(root).expect("get migrated tasks");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "CVH-001");
        assert_eq!(tasks[0].title, "Migrated Task");
        let todos = tasks[0].todos.as_ref().expect("migrated todos");
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].id, "td-m1");

        let next_seq = db_allocate_next_task_num(root).expect("next seq");
        assert_eq!(next_seq, 2);

        let _ = std::fs::remove_dir_all(root);
    }
}

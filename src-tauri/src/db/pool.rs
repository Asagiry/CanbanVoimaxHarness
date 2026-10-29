use crate::agent::Task;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, TransactionBehavior};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

pub type DbPool = Pool<SqliteConnectionManager>;
pub type DbConnection = r2d2::PooledConnection<SqliteConnectionManager>;

static INITIALIZED_DBS: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
static DB_POOLS: LazyLock<Mutex<HashMap<PathBuf, DbPool>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

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
    let mut conn = Connection::open(&db_path)?;

    // Set busy timeout to 5000ms to eliminate instant SQLITE_BUSY under WAL concurrent access
    conn.busy_timeout(std::time::Duration::from_millis(5000))?;

    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;",
    )?;

    // Verify whether essential tables exist; DDL must execute if tables are missing
    let tables_exist: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('tasks', 'task_todos', 'dialog_items')",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count == 3)
        .unwrap_or(false);

    let already_initialized = {
        let set = INITIALIZED_DBS.lock().unwrap();
        set.contains(&db_path)
    };

    if !tables_exist || !already_initialized {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS dialog_items (
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
                  stage_entered_at INTEGER,
                  created_at INTEGER NOT NULL,
                  updated_at INTEGER NOT NULL,
                  completed_at INTEGER,
                  position REAL NOT NULL DEFAULT 0.0,
                  hook_states TEXT,
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
        let _ = conn.execute("ALTER TABLE tasks ADD COLUMN position REAL NOT NULL DEFAULT 0.0", []);
        let _ = conn.execute("ALTER TABLE tasks ADD COLUMN hook_states TEXT", []);
        let _ = conn.execute("ALTER TABLE tasks ADD COLUMN completed_at INTEGER", []);
        let _ = conn.execute("ALTER TABLE tasks ADD COLUMN stage_entered_at INTEGER", []);
        let _ = conn.execute("UPDATE tasks SET stage_entered_at = created_at WHERE stage_entered_at IS NULL", []);
        let _ = conn.execute("CREATE INDEX IF NOT EXISTS idx_tasks_stage_pos ON tasks(stage_id, position)", []);

        auto_migrate_if_needed(&mut conn, root);

        let mut set = INITIALIZED_DBS.lock().unwrap();
        set.insert(db_path.clone());
    }

    Ok(conn)
}

/// Returns or creates an r2d2 connection pool for the database at `<root>/.cvh/harness.db`.
pub fn get_db_pool(root: &Path) -> Result<DbPool, rusqlite::Error> {
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

    let mut pools = DB_POOLS.lock().unwrap();
    if let Some(pool) = pools.get(&db_path) {
        return Ok(pool.clone());
    }

    // Ensure database tables and schema are initialized first
    let _ = init_db(root)?;

    let manager = SqliteConnectionManager::file(&db_path).with_init(|conn| {
        conn.busy_timeout(std::time::Duration::from_millis(5000))?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )?;
        Ok(())
    });

    let pool = Pool::builder()
        .max_size(16)
        .build(manager)
        .map_err(|e| rusqlite::Error::InvalidPath(PathBuf::from(e.to_string())))?;

    pools.insert(db_path, pool.clone());
    Ok(pool)
}

/// Retrieves a pooled database connection for `<root>/.cvh/harness.db`.
pub fn get_db_conn(root: &Path) -> Result<DbConnection, rusqlite::Error> {
    let pool = get_db_pool(root)?;
    pool.get()
        .map_err(|e| rusqlite::Error::InvalidPath(PathBuf::from(e.to_string())))
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
            if let Ok(tx) = conn.transaction_with_behavior(TransactionBehavior::Immediate) {
                let _ = tx.execute("PRAGMA foreign_keys = OFF", []);
                for t in &tasks {
                    let custom_flow_json = t.custom_flow.as_ref().and_then(|v| serde_json::to_string(v).ok());
                    let attachments_json = t.attachments.as_ref().and_then(|v| serde_json::to_string(v).ok());
                    let tags_json = t.tags.as_ref().and_then(|v| serde_json::to_string(v).ok());
                    let hook_states_json = t.hook_states.as_ref().and_then(|v| serde_json::to_string(v).ok());
                    let skip_wt = if t.skip_worktree.unwrap_or(false) { 1i64 } else { 0i64 };
                    let cur_tokens = t.current_tokens.map(|v| v as i64);
                    let sub_idx = t.subtask_index.map(|v| v as i64);
                    let pos_val = t.position.unwrap_or(0.0);

                    let _ = tx.execute(
                        "INSERT OR REPLACE INTO tasks (
                            id, title, description, stage_id, board_id, worktree_path, branch, verdict,
                            custom_flow, status, model, parent_id, subtask_index, waiting_for_subtask_id,
                            handover_summary, cover_image, priority, current_tokens, skip_worktree,
                            created_at, updated_at, completed_at, attachments, tags, position, hook_states,
                            stage_entered_at
                        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27)",
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
                            t.completed_at,
                            attachments_json,
                            tags_json,
                            pos_val,
                            hook_states_json,
                            t.stage_entered_at.unwrap_or(t.created_at),
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

    let todo_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM task_todos", [], |r| r.get(0))
        .unwrap_or(0);

    if todo_count == 0 && task_count > 0 {
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
            if let Ok(tx) = conn.transaction_with_behavior(TransactionBehavior::Immediate) {
                for t in &tasks {
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

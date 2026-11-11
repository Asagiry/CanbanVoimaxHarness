use crate::db::migrations::auto_migrate_if_needed;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

pub type DbPool = Pool<SqliteConnectionManager>;
pub type DbConnection = r2d2::PooledConnection<SqliteConnectionManager>;

static INITIALIZED_DBS: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
static DB_POOLS: LazyLock<Mutex<HashMap<PathBuf, DbPool>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Normalizes and canonicalizes a project repository root path.
/// Resolves git worktrees to parent workspace and strips Windows UNC verbatim prefix.
pub fn normalize_db_root(root: &Path) -> PathBuf {
    let resolved = crate::config::resolve_actual_repo_root(root);
    let target = if resolved.as_os_str().is_empty() {
        root.to_path_buf()
    } else {
        // In tests, ensure we didn't escape out of isolated `target/` temp dir into the live project root
        if cfg!(test)
            && root.components().any(|c| c.as_os_str() == "target")
            && !resolved.components().any(|c| c.as_os_str() == "target")
        {
            root.to_path_buf()
        } else {
            resolved
        }
    };
    if let Ok(canonical) = target.canonicalize() {
        let stripped = crate::config::strip_windows_verbatim_prefix(&canonical.to_string_lossy());
        PathBuf::from(stripped)
    } else {
        target
    }
}

fn sqlite_err(code: i32, msg: String) -> rusqlite::Error {
    rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), Some(msg))
}

fn quarantine_corrupt_db(db_path: &Path, reason: &str) {
    let ts = chrono::Utc::now().timestamp_millis();
    for suffix in ["", "-wal", "-shm"] {
        let src = PathBuf::from(format!("{}{}", db_path.to_string_lossy(), suffix));
        if src.exists() {
            let dst = PathBuf::from(format!("{}{}.corrupt-{}", db_path.to_string_lossy(), suffix, ts));
            if let Err(e) = std::fs::rename(&src, &dst) {
                eprintln!(
                    "CVH [db]: Failed to quarantine corrupt DB artifact {:?} -> {:?}: {}",
                    src, dst, e
                );
            } else {
                eprintln!(
                    "CVH [db]: Quarantined corrupt DB artifact {:?} -> {:?} ({})",
                    src, dst, reason
                );
            }
        }
    }
}

fn ensure_column_exists(
    conn: &Connection,
    table: &str,
    column: &str,
    alter_sql: &str,
) -> Result<(), rusqlite::Error> {
    let pragma = format!("PRAGMA table_info({});", table);
    let mut stmt = conn.prepare(&pragma)?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let col_name: String = row.get(1)?;
        if col_name.eq_ignore_ascii_case(column) {
            return Ok(());
        }
    }
    drop(rows);
    drop(stmt);
    conn.execute(alter_sql, [])?;
    Ok(())
}

/// Initializes SQLite database at <root>/.cvh/harness.db and creates tables and indexes.
pub fn init_db(root: &Path) -> Result<Connection, rusqlite::Error> {
    if root.as_os_str().is_empty() {
        return Err(rusqlite::Error::InvalidPath(root.to_path_buf()));
    }
    let normalized = normalize_db_root(root);
    if !cfg!(test) && (crate::config::is_binary_or_target_dir(&normalized) || crate::config::is_app_own_directory(&normalized)) {
        return Err(rusqlite::Error::InvalidPath(normalized));
    }
    let db_dir = normalized.join(".cvh");
    if !db_dir.exists() {
        std::fs::create_dir_all(&db_dir).map_err(|e| {
            sqlite_err(
                rusqlite::ffi::SQLITE_CANTOPEN,
                format!("Failed to create .cvh directory {:?}: {}", db_dir, e),
            )
        })?;
    }
    let normalized = normalize_db_root(&normalized);
    let db_path = normalized.join(".cvh").join("harness.db");
    let mut conn = Connection::open(&db_path)?;

    // Set busy timeout to 5000ms to eliminate instant SQLITE_BUSY under WAL concurrent access
    conn.busy_timeout(std::time::Duration::from_millis(5000))?;

    let quick_check_rows: Result<Vec<String>, rusqlite::Error> = (|| {
        let mut stmt = conn.prepare("PRAGMA quick_check;")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    })();
    match quick_check_rows {
        Ok(rows) if rows.len() == 1 && rows[0].eq_ignore_ascii_case("ok") => {}
        Ok(rows) => {
            let reason = format!("PRAGMA quick_check failed: {}", rows.join("; "));
            drop(conn);
            quarantine_corrupt_db(&db_path, &reason);
            return Err(sqlite_err(rusqlite::ffi::SQLITE_CORRUPT, reason));
        }
        Err(e) => {
            let reason = format!("PRAGMA quick_check error: {}", e);
            drop(conn);
            quarantine_corrupt_db(&db_path, &reason);
            return Err(sqlite_err(rusqlite::ffi::SQLITE_CORRUPT, reason));
        }
    }

    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;",
    )?;

    let encoding: String = conn.query_row("PRAGMA encoding", [], |row| row.get(0))?;
    if !encoding.eq_ignore_ascii_case("UTF-8") {
        eprintln!(
            "CVH [db]: WARN database encoding is '{}', expected 'UTF-8'. SQLite TEXT is stored as-is; changing encoding requires VACUUM INTO and is not done automatically.",
            encoding
        );
    }

    // Verify whether essential tables exist; DDL must execute if tables are missing
    let tables_exist: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('tasks', 'task_todos', 'dialog_items')",
            [],
            |row| row.get::<_, i64>(0),
        )? == 3;

    let already_initialized = {
        let set = INITIALIZED_DBS.lock().unwrap_or_else(|p| p.into_inner());
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
                 provider_id TEXT,
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
                  stage_durations TEXT,
                  total_elapsed INTEGER NOT NULL DEFAULT 0,
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
                 provider_id TEXT,
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
        ensure_column_exists(&conn, "dialog_items", "tool_json", "ALTER TABLE dialog_items ADD COLUMN tool_json TEXT")?;
        ensure_column_exists(&conn, "dialog_items", "duration_ms", "ALTER TABLE dialog_items ADD COLUMN duration_ms REAL")?;
        ensure_column_exists(&conn, "tasks", "attachments", "ALTER TABLE tasks ADD COLUMN attachments TEXT")?;
        ensure_column_exists(&conn, "tasks", "tags", "ALTER TABLE tasks ADD COLUMN tags TEXT")?;
        ensure_column_exists(&conn, "tasks", "position", "ALTER TABLE tasks ADD COLUMN position REAL NOT NULL DEFAULT 0.0")?;
        ensure_column_exists(&conn, "tasks", "hook_states", "ALTER TABLE tasks ADD COLUMN hook_states TEXT")?;
        ensure_column_exists(&conn, "tasks", "completed_at", "ALTER TABLE tasks ADD COLUMN completed_at INTEGER")?;
        ensure_column_exists(&conn, "tasks", "stage_entered_at", "ALTER TABLE tasks ADD COLUMN stage_entered_at INTEGER")?;
        ensure_column_exists(&conn, "tasks", "stage_durations", "ALTER TABLE tasks ADD COLUMN stage_durations TEXT")?;
        ensure_column_exists(&conn, "tasks", "total_elapsed", "ALTER TABLE tasks ADD COLUMN total_elapsed INTEGER DEFAULT 0")?;
        ensure_column_exists(&conn, "tasks", "provider_id", "ALTER TABLE tasks ADD COLUMN provider_id TEXT")?;
        ensure_column_exists(&conn, "tasks", "reasoning_effort", "ALTER TABLE tasks ADD COLUMN reasoning_effort TEXT")?;
        conn.execute("UPDATE tasks SET stage_entered_at = created_at WHERE stage_entered_at IS NULL", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_tasks_stage_pos ON tasks(stage_id, position)", [])?;

        auto_migrate_if_needed(&mut conn, root)?;

        let mut set = INITIALIZED_DBS.lock().unwrap_or_else(|p| p.into_inner());
        set.insert(db_path.clone());
    }

    Ok(conn)
}

/// Returns or creates an r2d2 connection pool for the database at `<root>/.cvh/harness.db`.
pub fn get_db_pool(root: &Path) -> Result<DbPool, rusqlite::Error> {
    if root.as_os_str().is_empty() {
        return Err(rusqlite::Error::InvalidPath(root.to_path_buf()));
    }
    let normalized = normalize_db_root(root);
    if !cfg!(test) && (crate::config::is_binary_or_target_dir(&normalized) || crate::config::is_app_own_directory(&normalized)) {
        return Err(rusqlite::Error::InvalidPath(normalized));
    }
    let db_dir = normalized.join(".cvh");
    if !db_dir.exists() {
        std::fs::create_dir_all(&db_dir).map_err(|e| {
            sqlite_err(
                rusqlite::ffi::SQLITE_CANTOPEN,
                format!("Failed to create .cvh directory {:?}: {}", db_dir, e),
            )
        })?;
    }
    let normalized = normalize_db_root(&normalized);
    let db_path = normalized.join(".cvh").join("harness.db");

    let mut pools = DB_POOLS.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(pool) = pools.get(&db_path) {
        return Ok(pool.clone());
    }

    // Ensure database tables, WAL mode, and schema are initialized first
    let _ = init_db(&normalized)?;

    let manager = SqliteConnectionManager::file(&db_path).with_init(|conn| {
        conn.busy_timeout(std::time::Duration::from_millis(5000))?;
        conn.execute_batch(
            "PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )?;
        Ok(())
    });

    let pool = Pool::builder()
        .max_size(4)
        .build(manager)
        .map_err(|e| sqlite_err(rusqlite::ffi::SQLITE_ERROR, format!("Failed to build DB pool: {}", e)))?;

    pools.insert(db_path, pool.clone());
    Ok(pool)
}

/// Retrieves a pooled database connection for `<root>/.cvh/harness.db`.
pub fn get_db_conn(root: &Path) -> Result<DbConnection, rusqlite::Error> {
    let pool = get_db_pool(root)?;
    pool.get()
        .map_err(|e| sqlite_err(rusqlite::ffi::SQLITE_BUSY, format!("Failed to acquire DB connection from pool: {}", e)))
}

/// Checkpoints the SQLite WAL file into the main database file.
/// If `truncate` is true, runs `PRAGMA wal_checkpoint(TRUNCATE)` to checkpoint and truncate the WAL file.
/// If `truncate` is false, runs `PRAGMA wal_checkpoint(PASSIVE)`.
pub fn checkpoint_db(root: &Path, truncate: bool) -> Result<(), rusqlite::Error> {
    if root.as_os_str().is_empty() {
        return Ok(());
    }
    let conn = get_db_conn(root)?;
    let sql = if truncate {
        "PRAGMA wal_checkpoint(TRUNCATE);"
    } else {
        "PRAGMA wal_checkpoint(PASSIVE);"
    };
    conn.execute_batch(sql)?;
    Ok(())
}

/// Checkpoints all active SQLite pools in `DB_POOLS`.
pub fn checkpoint_all_dbs(truncate: bool) {
    let pools = DB_POOLS.lock().unwrap_or_else(|p| p.into_inner());
    let sql = if truncate {
        "PRAGMA wal_checkpoint(TRUNCATE);"
    } else {
        "PRAGMA wal_checkpoint(PASSIVE);"
    };
    for (db_path, pool) in pools.iter() {
        match pool.get() {
            Ok(conn) => {
                if let Err(e) = conn.execute_batch(sql) {
                    eprintln!("CVH [db]: WAL checkpoint failed for {:?}: {}", db_path, e);
                }
            }
            Err(e) => {
                eprintln!("CVH [db]: Failed to acquire connection for WAL checkpoint {:?}: {}", db_path, e);
            }
        }
    }
}


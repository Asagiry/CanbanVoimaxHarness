use crate::agent::Task;
use crate::config::discover_project_root;
use crate::db::{DbConnection, DbPool};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};

#[derive(Clone)]
pub struct ActiveAgentHandle {
    pub abort_flag: Arc<AtomicBool>,
    pub join_handle: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

pub type AgentEntry = ActiveAgentHandle;

pub struct AppState {
    pub repo_root: RwLock<PathBuf>,
    pub active_agents: Mutex<HashMap<String, ActiveAgentHandle>>,
    pub session_registry: Arc<crate::agent::session_registry::AgentSessionRegistry>,
    pub agent_contexts: Mutex<HashMap<String, Arc<Mutex<crate::agent::compactor::AgentContextState>>>>,
    pub tasks: Arc<Mutex<Vec<Task>>>,
    pub db_pool: RwLock<Option<DbPool>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(PathBuf::new())
    }
}

impl AppState {
    pub fn new(root: PathBuf) -> Self {
        let actual_root = if !root.as_os_str().is_empty()
            && !crate::config::is_binary_or_target_dir(&root)
            && !crate::config::is_app_own_directory(&root)
            && (root.join(".cvh").exists() || root.join(".git").exists())
        {
            root
        } else {
            discover_project_root()
        };
        let db_pool = crate::db::get_db_pool(&actual_root).ok();
        Self {
            repo_root: RwLock::new(actual_root),
            active_agents: Mutex::new(HashMap::new()),
            session_registry: Arc::new(crate::agent::session_registry::AgentSessionRegistry::new()),
            agent_contexts: Mutex::new(HashMap::new()),
            tasks: Arc::new(Mutex::new(Vec::new())),
            db_pool: RwLock::new(db_pool),
        }
    }

    pub fn get_root(&self) -> PathBuf {
        self.repo_root
            .read()
            .map(|r| r.clone())
            .unwrap_or_else(|_| discover_project_root())
    }

    pub fn set_root(&self, new_root: PathBuf) -> Result<(), String> {
        if crate::config::is_binary_or_target_dir(&new_root) {
            return Err(format!("Cannot set binary or build directory as project root: {:?}", new_root));
        }
        if crate::config::is_app_own_directory(&new_root) {
            return Err(format!("Cannot set application codebase directory as project root: {:?}", new_root));
        }
        let mut guard = self
            .repo_root
            .write()
            .map_err(|e| format!("Failed to acquire write lock on repo_root: {}", e))?;
        *guard = new_root.clone();

        // Dynamically update the SQLite connection pool for the new workspace root
        if let Ok(pool) = crate::db::get_db_pool(&new_root) {
            if let Ok(mut pool_guard) = self.db_pool.write() {
                *pool_guard = Some(pool);
            }
        }
        Ok(())
    }

    /// Returns the underlying r2d2 connection pool, initializing it if absent.
    pub fn get_db_pool(&self) -> Result<DbPool, rusqlite::Error> {
        if let Ok(guard) = self.db_pool.read() {
            if let Some(ref pool) = *guard {
                return Ok(pool.clone());
            }
        }
        let pool = crate::db::get_db_pool(&self.get_root())?;
        if let Ok(mut guard) = self.db_pool.write() {
            *guard = Some(pool.clone());
        }
        Ok(pool)
    }

    /// Retrieves an active connection from the AppState connection pool or falls back to root.
    pub fn get_db_connection(&self) -> Result<DbConnection, rusqlite::Error> {
        if let Ok(guard) = self.db_pool.read() {
            if let Some(ref pool) = *guard {
                if let Ok(conn) = pool.get() {
                    return Ok(conn);
                }
            }
        }
        crate::db::get_db_conn(&self.get_root())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_db_pool_initialization_and_checkout() {
        let temp_dir = PathBuf::from("target").join(format!("cvh_appstate_pool_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(temp_dir.join(".cvh")).unwrap();

        let state = AppState::new(temp_dir.clone());
        assert_eq!(state.get_root(), temp_dir);

        let pool = state.get_db_pool().expect("get_db_pool must succeed");
        assert_eq!(pool.max_size(), 16);

        let conn = state.get_db_connection().expect("get_db_connection must succeed");
        let val: i64 = conn.query_row("SELECT 42", [], |r| r.get(0)).unwrap();
        assert_eq!(val, 42);

        let journal_mode: String = conn.query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
        assert_eq!(journal_mode.to_uppercase(), "WAL");

        drop(conn);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_app_state_db_pool_concurrent_access() {
        let temp_dir = PathBuf::from("target").join(format!("cvh_appstate_pool_concurrent_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let state = Arc::new(AppState::new(temp_dir.clone()));
        let mut handles = vec![];

        for i in 0..8 {
            let state_clone = Arc::clone(&state);
            handles.push(std::thread::spawn(move || {
                let conn = state_clone.get_db_connection().expect("concurrent connection");
                let res: i64 = conn.query_row(&format!("SELECT {}", i), [], |r| r.get(0)).unwrap();
                assert_eq!(res, i);
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_app_state_db_pool_root_switch() {
        let temp_dir1 = PathBuf::from("target").join(format!("cvh_appstate_switch_1_{}", uuid::Uuid::new_v4()));
        let temp_dir2 = PathBuf::from("target").join(format!("cvh_appstate_switch_2_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir1).unwrap();
        std::fs::create_dir_all(&temp_dir2).unwrap();

        let state = AppState::new(temp_dir1.clone());
        {
            let conn1 = state.get_db_connection().expect("conn on dir 1");
            conn1.execute("CREATE TABLE IF NOT EXISTS dir1_marker (id INT)", []).unwrap();
        }

        state.set_root(temp_dir2.clone()).expect("switch to dir 2");
        assert_eq!(state.get_root(), temp_dir2);

        {
            let conn2 = state.get_db_connection().expect("conn on dir 2");
            let table_exists: bool = conn2.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='dir1_marker'",
                [],
                |r| r.get::<_, i64>(0).map(|c| c > 0)
            ).unwrap();
            assert!(!table_exists, "dir2 DB must not contain dir1 table");
        }

        let _ = std::fs::remove_dir_all(&temp_dir1);
        let _ = std::fs::remove_dir_all(&temp_dir2);
    }
}

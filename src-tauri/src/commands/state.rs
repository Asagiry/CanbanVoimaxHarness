use crate::agent::Task;
use crate::config::discover_project_root;
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
    pub agent_contexts: Mutex<HashMap<String, Arc<Mutex<crate::agent::compactor::AgentContextState>>>>,
    pub tasks: Arc<Mutex<Vec<Task>>>,
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
        Self {
            repo_root: RwLock::new(actual_root),
            active_agents: Mutex::new(HashMap::new()),
            agent_contexts: Mutex::new(HashMap::new()),
            tasks: Arc::new(Mutex::new(Vec::new())),
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
        *guard = new_root;
        Ok(())
    }
}

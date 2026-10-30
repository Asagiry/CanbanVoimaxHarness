//! Authoritative Agent Session Registry
//! Unified source of truth for agent execution sessions, lifecycle state, abort handles, and join handles.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Running,
    Paused,
    Aborting,
    Finished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionSummary {
    pub session_id: String,
    pub task_id: String,
    pub stage_id: Option<String>,
    pub started_at: u64,
    pub status: SessionStatus,
}

pub struct AgentSessionEntry {
    pub summary: AgentSessionSummary,
    pub abort_handle: Arc<AtomicBool>,
    pub join_handle: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

pub struct AgentSessionRegistry {
    sessions: RwLock<HashMap<String, AgentSessionEntry>>,
}

impl Default for AgentSessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentSessionRegistry {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Registers a new session entry. Keyed by `summary.session_id`.
    pub fn register(
        &self,
        summary: AgentSessionSummary,
        abort_handle: Arc<AtomicBool>,
        join_handle: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
    ) {
        let mut map = self.sessions.write().unwrap_or_else(|p| p.into_inner());
        map.insert(
            summary.session_id.clone(),
            AgentSessionEntry {
                summary,
                abort_handle,
                join_handle,
            },
        );
    }

    /// Updates the status of matching sessions by session_id or task_id.
    pub fn update_status(&self, task_or_session_id: &str, status: SessionStatus) {
        let id_clean = task_or_session_id.trim();
        let mut map = self.sessions.write().unwrap_or_else(|p| p.into_inner());
        for entry in map.values_mut() {
            if entry.summary.session_id == id_clean
                || entry.summary.task_id == id_clean
                || entry.summary.task_id.eq_ignore_ascii_case(id_clean)
            {
                entry.summary.status = status;
            }
        }
    }

    /// Removes a session from the registry by session_id (or task_id fallback).
    pub fn remove(&self, session_id: &str) {
        let id_clean = session_id.trim();
        let mut map = self.sessions.write().unwrap_or_else(|p| p.into_inner());
        if map.remove(id_clean).is_none() {
            map.retain(|_, v| v.summary.task_id != id_clean && !v.summary.task_id.eq_ignore_ascii_case(id_clean));
        }
    }

    /// Returns a list of summaries for all active (non-Finished) sessions.
    pub fn get_active_sessions(&self) -> Vec<AgentSessionSummary> {
        let map = self.sessions.read().unwrap_or_else(|p| p.into_inner());
        map.values()
            .filter(|e| e.summary.status != SessionStatus::Finished)
            .map(|e| e.summary.clone())
            .collect()
    }

    /// Checks if a given task has any active (non-Finished) session.
    pub fn has_active_session(&self, task_id: &str) -> bool {
        let id_clean = task_id.trim();
        let map = self.sessions.read().unwrap_or_else(|p| p.into_inner());
        map.values().any(|e| {
            (e.summary.task_id == id_clean
                || e.summary.task_id.eq_ignore_ascii_case(id_clean)
                || e.summary.session_id == id_clean)
                && e.summary.status != SessionStatus::Finished
        })
    }

    /// Trips abort_flags, aborts join_handles, and marks matching sessions as Aborting.
    /// Returns the session IDs of all aborted sessions.
    pub fn abort_task(&self, task_id: &str) -> Vec<String> {
        let id_clean = task_id.trim();
        let mut aborted = Vec::new();
        let mut map = self.sessions.write().unwrap_or_else(|p| p.into_inner());
        for (sid, entry) in map.iter_mut() {
            let is_match = entry.summary.task_id == id_clean
                || entry.summary.task_id.eq_ignore_ascii_case(id_clean)
                || sid == id_clean
                || entry.summary.task_id.starts_with(&format!("{}.", id_clean))
                || entry.summary.task_id.starts_with(&format!("{}:sub:", id_clean))
                || entry.summary.task_id.starts_with(&format!("{}.sub-", id_clean));

            if is_match {
                entry.abort_handle.store(true, Ordering::SeqCst);
                if let Ok(guard) = entry.join_handle.try_lock() {
                    if let Some(ref handle) = *guard {
                        handle.abort();
                    }
                }
                entry.summary.status = SessionStatus::Aborting;
                aborted.push(sid.clone());
            }
        }
        aborted
    }

    /// Attaches or updates a JoinHandle for an existing session.
    pub fn set_join_handle(&self, session_id: &str, join_handle: tokio::task::JoinHandle<()>) {
        let id_clean = session_id.trim();
        let map = self.sessions.read().unwrap_or_else(|p| p.into_inner());
        if let Some(entry) = map.get(id_clean) {
            if let Ok(mut guard) = entry.join_handle.try_lock() {
                *guard = Some(join_handle);
            }
        }
    }

    /// Retrieves an optional clone of summary by session_id.
    pub fn get_session(&self, session_id: &str) -> Option<AgentSessionSummary> {
        let id_clean = session_id.trim();
        let map = self.sessions.read().unwrap_or_else(|p| p.into_inner());
        map.get(id_clean).map(|e| e.summary.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registration_and_active_queries() {
        let registry = AgentSessionRegistry::new();
        let abort_flag = Arc::new(AtomicBool::new(false));
        let join_slot = Arc::new(tokio::sync::Mutex::new(None));

        let summary = AgentSessionSummary {
            session_id: "sess-CVH-043-1".to_string(),
            task_id: "CVH-043".to_string(),
            stage_id: Some("dev".to_string()),
            started_at: 1000,
            status: SessionStatus::Running,
        };

        registry.register(summary, Arc::clone(&abort_flag), join_slot);

        assert!(registry.has_active_session("CVH-043"));
        assert!(registry.has_active_session("cvh-043"));
        assert!(!registry.has_active_session("CVH-999"));

        let active = registry.get_active_sessions();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].task_id, "CVH-043");
        assert_eq!(active[0].status, SessionStatus::Running);

        let retrieved = registry.get_session("sess-CVH-043-1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().session_id, "sess-CVH-043-1");
    }

    #[test]
    fn test_status_transitions() {
        let registry = AgentSessionRegistry::new();
        let abort_flag = Arc::new(AtomicBool::new(false));
        let join_slot = Arc::new(tokio::sync::Mutex::new(None));

        let summary = AgentSessionSummary {
            session_id: "sess-CVH-100-1".to_string(),
            task_id: "CVH-100".to_string(),
            stage_id: Some("dev".to_string()),
            started_at: 2000,
            status: SessionStatus::Running,
        };

        registry.register(summary, Arc::clone(&abort_flag), join_slot);
        assert_eq!(registry.get_active_sessions().len(), 1);

        // Transition to Paused
        registry.update_status("CVH-100", SessionStatus::Paused);
        let s = registry.get_session("sess-CVH-100-1").unwrap();
        assert_eq!(s.status, SessionStatus::Paused);
        assert!(registry.has_active_session("CVH-100"));

        // Transition to Finished
        registry.update_status("sess-CVH-100-1", SessionStatus::Finished);
        let s = registry.get_session("sess-CVH-100-1").unwrap();
        assert_eq!(s.status, SessionStatus::Finished);

        // Finished sessions are excluded from active list and has_active_session
        assert!(!registry.has_active_session("CVH-100"));
        assert_eq!(registry.get_active_sessions().len(), 0);
    }

    #[tokio::test]
    async fn test_abort_task() {
        let registry = AgentSessionRegistry::new();
        let abort_flag_parent = Arc::new(AtomicBool::new(false));
        let abort_flag_sub = Arc::new(AtomicBool::new(false));

        let join_parent = tokio::spawn(async {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        });
        let join_sub = tokio::spawn(async {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        });

        registry.register(
            AgentSessionSummary {
                session_id: "sess-CVH-200-main".to_string(),
                task_id: "CVH-200".to_string(),
                stage_id: Some("dev".to_string()),
                started_at: 3000,
                status: SessionStatus::Running,
            },
            Arc::clone(&abort_flag_parent),
            Arc::new(tokio::sync::Mutex::new(Some(join_parent))),
        );

        registry.register(
            AgentSessionSummary {
                session_id: "sess-CVH-200.sub-1".to_string(),
                task_id: "CVH-200.sub-1".to_string(),
                stage_id: Some("dev".to_string()),
                started_at: 3050,
                status: SessionStatus::Running,
            },
            Arc::clone(&abort_flag_sub),
            Arc::new(tokio::sync::Mutex::new(Some(join_sub))),
        );

        let aborted = registry.abort_task("CVH-200");
        assert_eq!(aborted.len(), 2);
        assert!(abort_flag_parent.load(Ordering::SeqCst));
        assert!(abort_flag_sub.load(Ordering::SeqCst));

        let s_parent = registry.get_session("sess-CVH-200-main").unwrap();
        let s_sub = registry.get_session("sess-CVH-200.sub-1").unwrap();
        assert_eq!(s_parent.status, SessionStatus::Aborting);
        assert_eq!(s_sub.status, SessionStatus::Aborting);
    }

    #[test]
    fn test_remove_session() {
        let registry = AgentSessionRegistry::new();
        let abort_flag = Arc::new(AtomicBool::new(false));
        let join_slot = Arc::new(tokio::sync::Mutex::new(None));

        registry.register(
            AgentSessionSummary {
                session_id: "sess-CVH-300-1".to_string(),
                task_id: "CVH-300".to_string(),
                stage_id: Some("review".to_string()),
                started_at: 4000,
                status: SessionStatus::Running,
            },
            Arc::clone(&abort_flag),
            join_slot,
        );

        assert!(registry.has_active_session("CVH-300"));
        registry.remove("sess-CVH-300-1");
        assert!(!registry.has_active_session("CVH-300"));
        assert!(registry.get_session("sess-CVH-300-1").is_none());
    }

    #[test]
    fn test_concurrent_access() {
        let registry = Arc::new(AgentSessionRegistry::new());
        let mut handles = Vec::new();

        for i in 0..10 {
            let reg = Arc::clone(&registry);
            handles.push(std::thread::spawn(move || {
                let sid = format!("sess-CONCURRENT-{}", i);
                let tid = format!("CONCURRENT-{}", i);
                let flag = Arc::new(AtomicBool::new(false));
                let slot = Arc::new(tokio::sync::Mutex::new(None));
                reg.register(
                    AgentSessionSummary {
                        session_id: sid.clone(),
                        task_id: tid.clone(),
                        stage_id: Some("dev".to_string()),
                        started_at: 1000 + i,
                        status: SessionStatus::Running,
                    },
                    flag,
                    slot,
                );
                assert!(reg.has_active_session(&tid));
                reg.update_status(&sid, SessionStatus::Finished);
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(registry.get_active_sessions().len(), 0);
    }
}

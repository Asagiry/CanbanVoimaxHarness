//! Authoritative Agent Session Registry
//! Unified source of truth for agent execution sessions, lifecycle state, abort handles, and join handles.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use crate::commands::state::{ActiveAgentHandle, AppState};
use crate::commands::tasks::healing::task_id_matches;

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
    #[serde(default)]
    pub project_path: Option<String>,
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

    /// Checks if a given task has any active (non-Finished) session (including subtasks/subagents).
    pub fn has_active_session(&self, task_id: &str) -> bool {
        let id_clean = task_id.trim();
        let map = self.sessions.read().unwrap_or_else(|p| p.into_inner());
        map.values().any(|e| {
            let is_match = e.summary.task_id == id_clean
                || e.summary.task_id.eq_ignore_ascii_case(id_clean)
                || e.summary.session_id == id_clean
                || e.summary.task_id.starts_with(&format!("{}.", id_clean))
                || e.summary.task_id.starts_with(&format!("{}:sub:", id_clean))
                || e.summary.task_id.starts_with(&format!("{}.sub-", id_clean));

            is_match && e.summary.status != SessionStatus::Finished
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

    /// Hierarchically aborts sessions for either a parent task or child subagent.
    pub async fn abort_hierarchy(
        &self,
        target_id: &str,
        state: &AppState,
    ) -> Result<AbortSessionsOutcome, String> {
        abort_hierarchical_sessions(target_id, state).await
    }
}

/// Outcome of hierarchical session abortion.
#[derive(Debug, Clone, Default)]
pub struct AbortSessionsOutcome {
    pub is_subagent: bool,
    pub had_entry: bool,
    pub aborted_session_ids: Vec<String>,
}

/// Suffix/prefix matching helper for subagent keys across registries and dialog identifiers.
pub fn is_matching_subagent_key(key: &str, target_sub_id: &str) -> bool {
    let k_stripped = key.split_once("::").map(|(_, tid)| tid).unwrap_or(key);
    let k_clean = k_stripped.trim();
    let t_clean = target_sub_id.trim();
    if k_clean == t_clean {
        return true;
    }
    let k_core = k_clean.rsplit(":sub:").next().unwrap_or(k_clean);
    let t_core = t_clean.rsplit(":sub:").next().unwrap_or(t_clean);
    if k_core == t_core {
        return true;
    }
    // Suffix match with strict delimiter boundary
    if k_core.contains(".sub-") {
        let suffix = k_core.rsplit(".sub-").next().unwrap_or(k_core);
        if suffix == t_core
            || t_core.ends_with(&format!(".sub-{}", suffix))
            || t_core.ends_with(&format!(":sub:{}", suffix))
        {
            return true;
        }
    }
    if t_core.contains(".sub-") {
        let suffix = t_core.rsplit(".sub-").next().unwrap_or(t_core);
        if suffix == k_core
            || k_core.ends_with(&format!(".sub-{}", suffix))
            || k_core.ends_with(&format!(":sub:{}", suffix))
        {
            return true;
        }
    }
    false
}

/// Determines if a task ID identifies a child subagent rather than a root parent task.
pub async fn is_subagent_task_id(
    task_id: &str,
    active_agents: Option<&std::sync::Mutex<HashMap<String, ActiveAgentHandle>>>,
) -> bool {
    let p_trim = task_id.trim();
    if p_trim.contains(".sub-") || p_trim.contains(":sub:") {
        return true;
    }

    let in_sessions = {
        let sessions = crate::agent::SUBAGENT_SESSIONS.lock().await;
        sessions.contains_key(p_trim) || sessions.values().any(|sess| {
            if let Ok(info) = sess.retry_info.try_lock() {
                info.subagent_id == p_trim
            } else {
                false
            }
        })
    };
    if in_sessions {
        return true;
    }

    if let Some(active) = active_agents {
        if let Ok(map) = active.lock() {
            return map.keys().any(|k| k == p_trim && (k.contains(".sub-") || k.contains(":sub:")));
        }
    }

    false
}

/// Hierarchical session abort:
/// - If `target_id` is a child subagent: aborts ONLY that specific subagent session across
///   `active_agents`, `session_registry`, `SUBAGENT_SESSIONS`, and `SUBAGENT_RECEIVERS`.
/// - If `target_id` is a parent task: aborts parent task handle AND cascades abort to all child subagents.
pub async fn abort_hierarchical_sessions(
    target_id: &str,
    state: &AppState,
) -> Result<AbortSessionsOutcome, String> {
    let p_trim = target_id.trim();
    if p_trim.is_empty() {
        return Ok(AbortSessionsOutcome::default());
    }

    let is_subagent = is_subagent_task_id(p_trim, Some(&state.active_agents)).await;

    if is_subagent {
        // =========================================================================
        // 1. GRANULAR SUBAGENT ABORT:
        //    Abort ONLY this specific subagent. DO NOT touch parent task or siblings!
        // =========================================================================
        let matching_handles: Vec<(String, ActiveAgentHandle)> = {
            let mut map = state
                .active_agents
                .lock()
                .map_err(|_| "Failed to lock active agents".to_string())?;

            let mut matched = Vec::new();
            let mut keys_to_remove = Vec::new();

            for k in map.keys() {
                if is_matching_subagent_key(k, p_trim) {
                    keys_to_remove.push(k.clone());
                }
            }

            for k in keys_to_remove {
                if let Some(entry) = map.remove(&k) {
                    matched.push((k, entry));
                }
            }
            matched
        };

        let aborted_sids = state.session_registry.abort_task(p_trim);
        let had_entry = !matching_handles.is_empty() || !aborted_sids.is_empty();

        for (_k, agent_entry) in matching_handles {
            agent_entry.abort_flag.store(true, Ordering::SeqCst);
            let maybe_handle = {
                let mut guard = agent_entry.join_handle.lock().await;
                guard.take()
            };
            if let Some(handle) = maybe_handle {
                handle.abort();
                let _ = tokio::time::timeout(tokio::time::Duration::from_millis(500), handle).await;
            }
        }

        // Abort matching session in SUBAGENT_SESSIONS
        let mut subagent_session_matched = false;
        {
            let sessions = crate::agent::SUBAGENT_SESSIONS.lock().await;
            for (sid, session) in sessions.iter() {
                let is_this = {
                    let mut info = session.retry_info.lock().await;
                    if is_matching_subagent_key(sid, p_trim)
                        || is_matching_subagent_key(&info.subagent_id, p_trim)
                    {
                        info.status = "aborted".to_string();
                        info.last_error = Some("Прервано пользователем".to_string());
                        info.ctx.abort_handle.store(true, Ordering::SeqCst);
                        true
                    } else {
                        false
                    }
                };
                if is_this {
                    subagent_session_matched = true;
                    session.abort_handle.store(true, Ordering::SeqCst);
                    let mut guard = session.join_handle.lock().await;
                    if let Some(h) = guard.take() {
                        h.abort();
                    }
                    if let Some(sender) = session.sender.lock().await.take() {
                        let _ = sender.send("[Субагент прерван пользователем / Subagent aborted by user]".to_string());
                    }
                }
            }
        }

        // Clean up matching receiver from SUBAGENT_RECEIVERS
        {
            let mut receivers = crate::agent::SUBAGENT_RECEIVERS.lock().await;
            let mut keys_to_remove = Vec::new();
            for k in receivers.keys() {
                if is_matching_subagent_key(k, p_trim) {
                    keys_to_remove.push(k.clone());
                }
            }
            for k in keys_to_remove {
                receivers.remove(&k);
            }
        }

        Ok(AbortSessionsOutcome {
            is_subagent: true,
            had_entry: had_entry || subagent_session_matched,
            aborted_session_ids: aborted_sids,
        })
    } else {
        // =========================================================================
        // 2. PARENT TASK ABORT:
        //    Abort parent task AND cascade abort to all its child subagents!
        // =========================================================================
        let matching_handles: Vec<(String, ActiveAgentHandle)> = {
            let mut map = state
                .active_agents
                .lock()
                .map_err(|_| "Failed to lock active agents".to_string())?;

            let mut matched = Vec::new();
            let mut keys_to_remove = Vec::new();

            for k in map.keys() {
                let k_stripped = k.split_once("::").map(|(_, tid)| tid).unwrap_or(k);
                let k_trim = k_stripped.trim();
                if task_id_matches(k_trim, p_trim)
                    || k_trim.to_ascii_lowercase().starts_with(&format!("{}.", p_trim.to_ascii_lowercase()))
                    || k_trim.to_ascii_lowercase().starts_with(&format!("{}:sub:", p_trim.to_ascii_lowercase()))
                    || k_trim.to_ascii_lowercase().starts_with(&format!("{}.sub-", p_trim.to_ascii_lowercase()))
                {
                    keys_to_remove.push(k.clone());
                }
            }

            for k in keys_to_remove {
                if let Some(entry) = map.remove(&k) {
                    matched.push((k, entry));
                }
            }
            matched
        };

        let aborted_session_ids = state.session_registry.abort_task(p_trim);
        let had_entry = !matching_handles.is_empty() || !aborted_session_ids.is_empty();

        // Abort all active agent handles and wait with timeout
        for (_k, agent_entry) in matching_handles {
            agent_entry.abort_flag.store(true, Ordering::SeqCst);
            let maybe_handle = {
                let mut guard = agent_entry.join_handle.lock().await;
                guard.take()
            };
            if let Some(handle) = maybe_handle {
                handle.abort();
                let _ = tokio::time::timeout(tokio::time::Duration::from_millis(500), handle).await;
            }
        }

        // Abort all child subagents in SUBAGENT_SESSIONS
        {
            let sessions = crate::agent::SUBAGENT_SESSIONS.lock().await;
            for (sid, session) in sessions.iter() {
                let matches_parent = {
                    let mut info = session.retry_info.lock().await;
                    if info.parent_task_id == p_trim
                        || info.subagent_id.starts_with(&format!("{}.", p_trim))
                        || info.subagent_id.starts_with(&format!("{}:sub:", p_trim))
                        || sid.starts_with(&format!("{}.", p_trim))
                        || sid.starts_with(&format!("{}:sub:", p_trim))
                    {
                        info.status = "aborted".to_string();
                        info.last_error = Some("Прервано пользователем".to_string());
                        info.ctx.abort_handle.store(true, Ordering::SeqCst);
                        true
                    } else {
                        false
                    }
                };
                if matches_parent {
                    session.abort_handle.store(true, Ordering::SeqCst);
                    let mut guard = session.join_handle.lock().await;
                    if let Some(h) = guard.take() {
                        h.abort();
                    }
                    // Take sender so awaiting receiver gets Err immediately
                    let _ = session.sender.lock().await.take();
                }
            }
        }

        // Clean up any awaiting receivers in SUBAGENT_RECEIVERS
        {
            let mut receivers = crate::agent::SUBAGENT_RECEIVERS.lock().await;
            let mut keys_to_remove = Vec::new();
            for k in receivers.keys() {
                let k_trim = k.trim();
                if task_id_matches(k_trim, p_trim)
                    || k_trim.to_ascii_lowercase().starts_with(&format!("{}.", p_trim.to_ascii_lowercase()))
                    || k_trim.to_ascii_lowercase().starts_with(&format!("{}:sub:", p_trim.to_ascii_lowercase()))
                    || k_trim.to_ascii_lowercase().starts_with(&format!("{}.sub-", p_trim.to_ascii_lowercase()))
                {
                    keys_to_remove.push(k.clone());
                }
            }
            for k in keys_to_remove {
                receivers.remove(&k);
            }
        }

        Ok(AbortSessionsOutcome {
            is_subagent: false,
            had_entry,
            aborted_session_ids,
        })
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
            project_path: None,
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
            project_path: None,
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
                project_path: None,
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
                project_path: None,
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
                project_path: None,
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
                        project_path: None,
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

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, LazyLock};
use tauri::AppHandle;
use tokio::sync::oneshot;
use crate::config::ToolPolicyConfig;

/// Execution context for tool calls, bundling path roots, policy, and abort handle.
#[derive(Clone)]
pub struct ToolContext<'a> {
    pub repo_root: &'a Path,
    pub worktree_dir: &'a Path,
    pub stage_id: Option<&'a str>,
    pub role: Option<&'a str>,
    pub abort_handle: Arc<AtomicBool>,
    pub tool_policy: &'a ToolPolicyConfig,
    pub task_id: Option<&'a str>,
    pub agent_allowed_tools: Option<&'a [String]>,
    pub app_handle: Option<AppHandle>,
    pub base_url: Option<&'a str>,
    pub api_key: Option<&'a str>,
    pub model: Option<&'a str>,
    pub temperature: Option<f32>,
    pub reasoning_effort: Option<&'a str>,
    pub call_id: Option<&'a str>,
}

#[derive(Clone)]
pub struct OwnedToolContext {
    pub repo_root: PathBuf,
    pub worktree_dir: PathBuf,
    pub stage_id: Option<String>,
    pub role: Option<String>,
    pub abort_handle: Arc<AtomicBool>,
    pub tool_policy: ToolPolicyConfig,
    pub task_id: Option<String>,
    pub agent_allowed_tools: Option<Vec<String>>,
    pub app_handle: Option<AppHandle>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub reasoning_effort: Option<String>,
    pub call_id: Option<String>,
}

impl<'a> ToolContext<'a> {
    pub fn to_owned(&self) -> OwnedToolContext {
        OwnedToolContext {
            repo_root: self.repo_root.to_path_buf(),
            worktree_dir: self.worktree_dir.to_path_buf(),
            stage_id: self.stage_id.map(|s| s.to_string()),
            role: self.role.map(|s| s.to_string()),
            abort_handle: Arc::clone(&self.abort_handle),
            tool_policy: self.tool_policy.clone(),
            task_id: self.task_id.map(|s| s.to_string()),
            agent_allowed_tools: self.agent_allowed_tools.map(|s| s.to_vec()),
            app_handle: self.app_handle.clone(),
            base_url: self.base_url.map(|s| s.to_string()),
            api_key: self.api_key.map(|s| s.to_string()),
            model: self.model.map(|s| s.to_string()),
            temperature: self.temperature,
            reasoning_effort: self.reasoning_effort.map(|s| s.to_string()),
            call_id: self.call_id.map(|s| s.to_string()),
        }
    }
}

pub type SubagentReceiver = oneshot::Receiver<String>;

#[derive(Clone)]
pub struct SubagentRetryInfo {
    pub parent_task_id: String,
    pub subagent_id: String,
    pub tool_call_id: Option<String>,
    pub title: String,
    pub role: String,
    pub prompt: String,
    pub requested_tools: Option<Vec<String>>,
    pub ctx: OwnedToolContext,
    pub status: String,
    pub created_at: i64,
    pub last_error: Option<String>,
    pub report: Option<String>,
    pub is_bg: bool,
    pub duration_ms: Option<f64>,
    pub steps: usize,
}

pub type SubagentSessionInfo = SubagentRetryInfo;

#[derive(Clone)]
pub struct SubagentSession {
    pub sender: Arc<tokio::sync::Mutex<Option<oneshot::Sender<String>>>>,
    pub retry_info: Arc<tokio::sync::Mutex<SubagentRetryInfo>>,
    pub abort_handle: Arc<AtomicBool>,
    pub join_handle: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl SubagentSession {
    pub fn new(sender: Option<oneshot::Sender<String>>, retry_info: SubagentRetryInfo) -> Self {
        Self {
            sender: Arc::new(tokio::sync::Mutex::new(sender)),
            retry_info: Arc::new(tokio::sync::Mutex::new(retry_info)),
            abort_handle: Arc::new(AtomicBool::new(false)),
            join_handle: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }
}

pub static SUBAGENT_RECEIVERS: LazyLock<tokio::sync::Mutex<HashMap<String, SubagentReceiver>>> =
    LazyLock::new(|| tokio::sync::Mutex::new(HashMap::new()));
pub static SUBAGENT_SESSIONS: LazyLock<tokio::sync::Mutex<HashMap<String, SubagentSession>>> =
    LazyLock::new(|| tokio::sync::Mutex::new(HashMap::new()));

/// Evicts completed, aborted, or failed subagent sessions from in-memory cache when capacity exceeds 30.
pub async fn prune_subagent_sessions_cache() {
    let mut sessions = SUBAGENT_SESSIONS.lock().await;
    if sessions.len() <= 30 {
        return;
    }
    let mut removable_keys = Vec::new();
    for (key, sess) in sessions.iter() {
        if let Ok(info) = sess.retry_info.try_lock() {
            if info.status == "finished" || info.status == "aborted" || info.status == "failed" {
                removable_keys.push(key.clone());
            }
        }
    }
    for k in removable_keys {
        if sessions.len() <= 30 {
            break;
        }
        sessions.remove(&k);
    }
}

/// Robust subagent session finder: searches SUBAGENT_SESSIONS by exact ID, stripped `:sub:`,
/// formatted `{parent}.sub-{id}`, or suffix matching `.sub-{tool_call_id}`.
pub async fn find_subagent_session(
    parent_task_id: Option<&str>,
    subagent_id: &str,
) -> Option<(String, SubagentSession)> {
    let sessions = SUBAGENT_SESSIONS.lock().await;
    let s_trim = subagent_id.trim();
    let p_trim = parent_task_id.map(|p| p.trim()).unwrap_or("");

    // 1. Direct match
    if let Some(session) = sessions.get(s_trim).cloned() {
        return Some((s_trim.to_string(), session));
    }

    // 2. Strip :sub: prefix if present
    if s_trim.contains(":sub:") {
        if let Some(after_sub) = s_trim.rsplit(":sub:").next() {
            if let Some(session) = sessions.get(after_sub).cloned() {
                return Some((after_sub.to_string(), session));
            }
        }
    }

    // 3. Try format!("{}.sub-{}", parent_task_id, subagent_id)
    if !p_trim.is_empty() {
        let key = format!("{}.sub-{}", p_trim, s_trim);
        if let Some(session) = sessions.get(&key).cloned() {
            return Some((key, session));
        }
    }

    // 4. Try matching tool_call_id suffix
    let tool_call_id = if let Some(pos) = s_trim.rfind(".sub-") {
        &s_trim[(pos + 5)..]
    } else {
        s_trim
    };

    if !tool_call_id.is_empty() {
        let suffix = format!(".sub-{}", tool_call_id);
        if !p_trim.is_empty() {
            for (k, v) in sessions.iter() {
                if k.starts_with(p_trim) && k.ends_with(&suffix) {
                    return Some((k.clone(), v.clone()));
                }
            }
        }
        for (k, v) in sessions.iter() {
            if k.ends_with(&suffix) {
                return Some((k.clone(), v.clone()));
            }
        }
    }

    None
}

/// Helper to look up a completed subagent's report from SQLite dialog_items.
pub fn find_subagent_report_from_db(
    root: &Path,
    parent_task_id: Option<&str>,
    subagent_id: &str,
) -> Option<String> {
    let p_trim = parent_task_id.map(str::trim).unwrap_or("");
    let s_trim = subagent_id.trim();

    let candidate_keys = [
        if s_trim.contains(":sub:") { Some(s_trim.to_string()) } else { None },
        if !p_trim.is_empty() { Some(format!("{}:sub:{}", p_trim, s_trim)) } else { None },
        if !p_trim.is_empty() && !s_trim.contains(".sub-") {
            Some(format!("{}:sub:{}.sub-{}", p_trim, p_trim, s_trim))
        } else {
            None
        },
        Some(s_trim.to_string()),
    ];

    for key_opt in candidate_keys.into_iter().flatten() {
        if let Ok(safe_id) = crate::commands::dialogs::sanitize_task_id(&key_opt) {
            if let Ok(items) = crate::db::load_dialog_items(root, &safe_id) {
                if let Some(out_item) = items.iter().rev().find(|it| {
                    it.item_type == "output"
                        && it.content.as_ref().map(|c| !c.trim().is_empty()).unwrap_or(false)
                }) {
                    if let Some(ref text) = out_item.content {
                        return Some(format!("[Subagent ID: {}]\n{}", s_trim, text.trim()));
                    }
                }
            }
        }
    }

    None
}

/// Helper to remove a receiver from SUBAGENT_RECEIVERS either by exact ID or by `.sub-{id}` suffix.
pub fn remove_subagent_receiver(
    reg: &mut HashMap<String, SubagentReceiver>,
    id: &str,
) -> Option<(String, SubagentReceiver)> {
    let trimmed = id.trim();
    if let Some(rx) = reg.remove(trimmed) {
        return Some((trimmed.to_string(), rx));
    }
    let suffix = if trimmed.starts_with(".sub-") {
        trimmed.to_string()
    } else {
        format!(".sub-{}", trimmed)
    };
    let matching_key = reg.keys().find(|k| k.ends_with(&suffix)).cloned();
    if let Some(key) = matching_key {
        if let Some(rx) = reg.remove(&key) {
            return Some((key, rx));
        }
    }
    None
}

#[derive(Clone, Debug)]
pub struct ExecutedSubagentCall {
    pub id: String,
    pub tool_name: String,
    pub role: String,
    pub prompt: String,
    pub result: String,
}

pub fn are_questions_duplicate(q1: &str, q2: &str) -> bool {
    let ws1: String = q1.split_whitespace().collect::<Vec<_>>().join(" ");
    let ws2: String = q2.split_whitespace().collect::<Vec<_>>().join(" ");
    if ws1.is_empty() || ws2.is_empty() {
        return false;
    }
    ws1 == ws2 || are_subagent_prompts_duplicate(q1, q2)
}

/// Robust subagent prompt similarity check for single-turn deduplication.
pub fn are_subagent_prompts_duplicate(p1: &str, p2: &str) -> bool {
    let t1 = p1.trim();
    let t2 = p2.trim();
    if t1 == t2 {
        return true;
    }
    let ws1: String = t1.split_whitespace().collect::<Vec<_>>().join(" ");
    let ws2: String = t2.split_whitespace().collect::<Vec<_>>().join(" ");
    if ws1 == ws2 {
        return true;
    }
    if ws1.len() >= 20 && ws2.len() >= 20 {
        if ws1.starts_with(&ws2) || ws2.starts_with(&ws1) {
            return true;
        }
        let set1: std::collections::HashSet<&str> = ws1.split(' ').collect();
        let set2: std::collections::HashSet<&str> = ws2.split(' ').collect();
        let intersection = set1.intersection(&set2).count();
        let union = set1.union(&set2).count();
        if union > 0 && (intersection as f64 / union as f64) >= 0.85 {
            return true;
        }
    }
    false
}

/// Canonical tool name normalization for aliases and SWE-bench compatibility.
pub fn canonical_tool_name(tool_name: &str) -> &str {
    crate::config::normalize_tool_name(tool_name)
}

/// Determines if a failed tool execution triggers fail-fast batch abort (§ Directive C).
/// Mutation tools (edit_file, write_file, task mutations) failing with Err / !success,
/// or execute_command failing (Err, or output with non-zero exit code) abort subsequent tools in the batch.
pub fn should_abort_tool_batch(tool_name: &str, success: bool, result_output: &str) -> bool {
    let norm = canonical_tool_name(tool_name);
    let is_mutation = matches!(
        norm,
        "edit_file"
            | "write_file"
            | "create_task"
            | "complete_task"
            | "advance_task"
            | "reject_task"
    );
    let is_failing_cmd = norm == "execute_command"
        && (!success
            || (result_output.contains("[Exit Code: ") && !result_output.contains("[Exit Code: 0]")));

    (!success && is_mutation) || is_failing_cmd
}

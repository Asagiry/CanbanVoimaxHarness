use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, LazyLock, Mutex};
use tauri::AppHandle;
use crate::agent::sanitizer::append_messages_from_dialog;
use crate::db::ActivityItem;
use crate::llm::ChatMessage;

pub type QuestionSenderMap = HashMap<String, tokio::sync::oneshot::Sender<String>>;

pub static QUESTION_REGISTRY: LazyLock<Arc<tokio::sync::Mutex<QuestionSenderMap>>> =
    LazyLock::new(|| Arc::new(tokio::sync::Mutex::new(HashMap::new())));

pub struct AgentSteering {
    pub accepting: bool,
}

impl Default for AgentSteering {
    fn default() -> Self {
        Self { accepting: true }
    }
}

pub struct SteeringLifetime(pub Arc<Mutex<AgentSteering>>);

impl Drop for SteeringLifetime {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.lock() {
            state.accepting = false;
        }
    }
}

pub fn persist_steer_message(
    root: &Path,
    task_id: &str,
    message: &str,
    images: Option<Vec<String>>,
    message_id: Option<&str>,
) -> Result<Option<ActivityItem>, String> {
    let item = ActivityItem {
        id: message_id
            .filter(|id| !id.trim().is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        item_type: "steer".into(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        content: Some(message.to_string()),
        tool: None,
        duration_ms: None,
        images,
    };
    Ok(crate::db::insert_steer_message(root, task_id, &item)?.then_some(item))
}

pub async fn has_pending_user_steer(config: &AgentSessionConfig, cursor: i64) -> Result<bool> {
    let repo_root = config.repo_root.clone();
    let task_id = config.task_id.clone();
    tokio::task::spawn_blocking(move || {
        let end = crate::db::context_dialog_cursor(&repo_root, &task_id)
            .map_err(anyhow::Error::msg)?;
        if end <= cursor {
            return Ok(false);
        }
        let pending = crate::db::load_dialog_items_between(&repo_root, &task_id, cursor, end)
            .map_err(anyhow::Error::msg)?;
        Ok(pending.iter().any(|item| matches!(item.item_type.as_str(), "user" | "steer")))
    })
    .await
    .map_err(|e| anyhow::anyhow!("Failed to join steer check task: {e}"))?
}

pub async fn drain_pending_steers(
    config: &AgentSessionConfig,
    messages: &mut Vec<ChatMessage>,
    cursor: &mut i64,
    close_if_empty: bool,
) -> Result<(bool, bool)> {
    use crate::agent::sanitizer::is_dialog_item_replayable;
    let steering = Arc::clone(&config.steering);
    let repo_root = config.repo_root.clone();
    let task_id = config.task_id.clone();
    let cur = *cursor;
    let mut snapshot = messages.clone();
    let before = snapshot.len();

    let (delta, end, appended, user_instruction) = tokio::task::spawn_blocking(move || {
        let mut gate = steering
            .lock()
            .map_err(|_| anyhow::anyhow!("Steering queue lock poisoned"))?;
        let end = crate::db::context_dialog_cursor(&repo_root, &task_id)
            .map_err(anyhow::Error::msg)?;
        let pending: Vec<_> =
            crate::db::load_dialog_items_between(&repo_root, &task_id, cur, end)
                .map_err(anyhow::Error::msg)?
                .into_iter()
                .filter(is_dialog_item_replayable)
                .collect();
        let user_instruction = pending
            .iter()
            .any(|item| matches!(item.item_type.as_str(), "user" | "steer"));
        append_messages_from_dialog(&mut snapshot, "", &pending);
        let delta = snapshot.split_off(before);
        let appended = !delta.is_empty();
        if close_if_empty && !appended {
            gate.accepting = false;
        }
        Ok::<_, anyhow::Error>((delta, end, appended, user_instruction))
    })
    .await
    .map_err(|e| anyhow::anyhow!("Failed to join drain steers task: {e}"))??;

    messages.extend(delta);
    *cursor = end;
    Ok((appended, user_instruction))
}

pub struct AgentSessionConfig {
    pub repo_root: PathBuf,
    pub worktree_path: PathBuf,
    pub role: String,
    pub stage_id: String,
    pub task_id: String,
    pub task_title: String,
    pub task_description: String,
    pub handover_summary: Option<String>,
    pub task_attachments: Option<Vec<String>>,
    pub steer_prompt: Option<String>,
    pub steer_images: Option<Vec<String>>,
    pub steering: Arc<Mutex<AgentSteering>>,
    pub advance_aborted: Option<Arc<AtomicBool>>,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub temperature: Option<f32>,
    pub reasoning_effort: Option<String>,
    pub max_turns: usize,
    pub app_handle: Option<AppHandle>,
    pub context_window: Option<u64>,
}

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

use crate::agent::{sync_context_state, AgentEvent};
use crate::db::ActivityItem;
use crate::llm::ChatMessage;

/// Session driver contract for agent execution environments.
pub trait AgentSessionDriver: Send + Sync {
    /// Emit an agent lifecycle event.
    fn emit_event(&self, event: AgentEvent);

    /// Dialogue key or SQLite session ID for persistence.
    fn dialog_key(&self) -> &str;

    /// Worktree directory root path.
    fn worktree_root(&self) -> &Path;

    /// Callback invoked when a turn has finished.
    fn on_turn_finished(&self, turn: usize, messages: &[ChatMessage]);

    /// Repository root path (defaults to worktree_root).
    fn repo_root(&self) -> &Path {
        self.worktree_root()
    }

    /// Task ID (defaults to dialog_key).
    fn task_id(&self) -> &str {
        self.dialog_key()
    }
}

/// Session driver for root agent loop (`run_agent_loop`).
pub struct MainAgentDriver {
    pub task_id: String,
    pub worktree_path: PathBuf,
    pub repo_root: PathBuf,
    pub app_handle: Option<AppHandle>,
    pub event_sink: Mutex<Box<dyn FnMut(AgentEvent) + Send>>,
}

impl MainAgentDriver {
    pub fn new<F>(
        task_id: String,
        worktree_path: PathBuf,
        repo_root: PathBuf,
        app_handle: Option<AppHandle>,
        event_sink: F,
    ) -> Self
    where
        F: FnMut(AgentEvent) + Send + 'static,
    {
        Self {
            task_id,
            worktree_path,
            repo_root,
            app_handle,
            event_sink: Mutex::new(Box::new(event_sink)),
        }
    }
}

impl AgentSessionDriver for MainAgentDriver {
    fn emit_event(&self, event: AgentEvent) {
        if let Ok(mut sink) = self.event_sink.lock() {
            sink(event);
        }
    }

    fn dialog_key(&self) -> &str {
        &self.task_id
    }

    fn worktree_root(&self) -> &Path {
        &self.worktree_path
    }

    fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    fn on_turn_finished(&self, turn: usize, messages: &[ChatMessage]) {
        let mut msgs = messages.to_vec();
        let mut rev = 0;
        sync_context_state(&self.app_handle, &self.repo_root, &self.task_id, turn, &mut msgs, &mut rev);
    }
}

/// Configuration parameters for constructing a `SubagentDriver`.
#[derive(Debug, Clone)]
pub struct SubagentDriverConfig {
    pub parent_task_id: String,
    pub subagent_id: String,
    pub subagent_dialog_id: String,
    pub tool_call_id: Option<String>,
    pub repo_root: PathBuf,
    pub worktree_dir: PathBuf,
    pub model: String,
    pub app_handle: Option<AppHandle>,
}

/// Session driver for in-process subagents (`run_inprocess_subagent`).
pub struct SubagentDriver {
    pub parent_task_id: String,
    pub subagent_id: String,
    pub subagent_dialog_id: String,
    pub tool_call_id: Option<String>,
    pub repo_root: PathBuf,
    pub worktree_dir: PathBuf,
    pub model: String,
    pub app_handle: Option<AppHandle>,
    pub turn_thought_id: Mutex<String>,
    pub thought_start: Mutex<Option<std::time::Instant>>,
    pub thought_end: Mutex<Option<std::time::Instant>>,
    pub accumulated_thoughts: Mutex<String>,
}

impl SubagentDriver {
    pub fn new(cfg: SubagentDriverConfig) -> Self {
        Self {
            parent_task_id: cfg.parent_task_id,
            subagent_id: cfg.subagent_id,
            subagent_dialog_id: cfg.subagent_dialog_id,
            tool_call_id: cfg.tool_call_id,
            repo_root: cfg.repo_root,
            worktree_dir: cfg.worktree_dir,
            model: cfg.model,
            app_handle: cfg.app_handle,
            turn_thought_id: Mutex::new(String::new()),
            thought_start: Mutex::new(None),
            thought_end: Mutex::new(None),
            accumulated_thoughts: Mutex::new(String::new()),
        }
    }

    pub fn reset_turn_thoughts(&self, thought_id: String) {
        *self.turn_thought_id.lock().unwrap_or_else(|p| p.into_inner()) = thought_id;
        *self.thought_start.lock().unwrap_or_else(|p| p.into_inner()) = None;
        *self.thought_end.lock().unwrap_or_else(|p| p.into_inner()) = None;
        self.accumulated_thoughts.lock().unwrap_or_else(|p| p.into_inner()).clear();
    }

    pub fn finalize_turn_thoughts(&self) {
        let thoughts = self.accumulated_thoughts.lock().unwrap_or_else(|p| p.into_inner()).trim().to_string();
        if !thoughts.is_empty() {
            let start = *self.thought_start.lock().unwrap_or_else(|p| p.into_inner());
            let end = *self.thought_end.lock().unwrap_or_else(|p| p.into_inner());
            let duration_ms = start.zip(end).map(|(s, e)| e.duration_since(s).as_secs_f64() * 1000.0);
            let thought_id = self.turn_thought_id.lock().unwrap_or_else(|p| p.into_inner()).clone();
            let th_item = ActivityItem {
                id: thought_id,
                item_type: "thought".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                content: Some(thoughts),
                tool: None,
                duration_ms,
                images: None,
            };
            let _ = crate::db::append_dialog_items(&self.repo_root, &self.subagent_dialog_id, std::slice::from_ref(&th_item));
            if let Some(ref app) = self.app_handle {
                let _ = app.emit("subagent:activity", serde_json::json!({
                    "parentTaskId": &self.parent_task_id,
                    "subagentId": &self.subagent_id,
                    "activity": &th_item,
                }));
            }
        }
    }
}

impl AgentSessionDriver for SubagentDriver {
    fn emit_event(&self, event: AgentEvent) {
        match event {
            AgentEvent::StreamStart { id } => {
                self.reset_turn_thoughts(format!("{id}:thought"));
                if let Some(ref app) = self.app_handle {
                    let _ = app.emit("agent:stream_start", serde_json::json!({
                        "taskId": &self.subagent_id, "streamId": id,
                    }));
                }
            }
            AgentEvent::StreamRetry { id, delay_secs } => {
                self.reset_turn_thoughts(String::new());
                if let Some(ref app) = self.app_handle {
                    let _ = app.emit("agent:stream_retry", serde_json::json!({
                        "taskId": &self.subagent_id, "streamId": id, "delaySecs": delay_secs,
                    }));
                }
            }
            AgentEvent::Thought(text) => {
                let mut start_guard = self.thought_start.lock().unwrap_or_else(|p| p.into_inner());
                if start_guard.is_none() {
                    *start_guard = Some(std::time::Instant::now());
                }
                drop(start_guard);
                *self.thought_end.lock().unwrap_or_else(|p| p.into_inner()) = Some(std::time::Instant::now());

                let mut thoughts_guard = self.accumulated_thoughts.lock().unwrap_or_else(|p| p.into_inner());
                thoughts_guard.push_str(&text);
                let current_thoughts = thoughts_guard.clone();
                drop(thoughts_guard);

                if current_thoughts.trim().is_empty() {
                    return;
                }

                let thought_id = self.turn_thought_id.lock().unwrap_or_else(|p| p.into_inner()).clone();
                let th_item = ActivityItem {
                    id: thought_id,
                    item_type: "thought".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    content: Some(current_thoughts),
                    tool: None,
                    duration_ms: None,
                    images: None,
                };
                // Intermediate streaming thought chunks are buffered in memory and written to SQLite
                // strictly upon turn finalization (finalize_turn_thoughts) to avoid SQLite WAL I/O thrashing.
                if let Some(ref app) = self.app_handle {
                    let _ = app.emit("subagent:activity", serde_json::json!({
                        "parentTaskId": &self.parent_task_id,
                        "subagentId": &self.subagent_id,
                        "activity": &th_item,
                    }));
                }
            }
            AgentEvent::Telemetry {
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cache_creation_tokens,
                cache_hit_rate,
                duration_ms,
                tps,
                ttft_ms,
                system_tokens,
                tool_tokens,
            } => {
                if let Some(ref app) = self.app_handle {
                    let _ = app.emit(
                        "agent:telemetry",
                        serde_json::json!({
                            "taskId": &self.subagent_id,
                            "inputTokens": input_tokens,
                            "outputTokens": output_tokens,
                            "cacheReadTokens": cache_read_tokens,
                            "cacheCreationTokens": cache_creation_tokens,
                            "cacheHitRate": cache_hit_rate,
                            "durationMs": duration_ms,
                            "tps": tps,
                            "ttftMs": ttft_ms,
                            "systemTokens": system_tokens,
                            "toolTokens": tool_tokens,
                        }),
                    );
                    if let Some(ref cid) = self.tool_call_id {
                        if cid != &self.subagent_id {
                            let _ = app.emit(
                                "agent:telemetry",
                                serde_json::json!({
                                    "taskId": cid,
                                    "inputTokens": input_tokens,
                                    "outputTokens": output_tokens,
                                    "cacheReadTokens": cache_read_tokens,
                                    "cacheCreationTokens": cache_creation_tokens,
                                    "cacheHitRate": cache_hit_rate,
                                    "durationMs": duration_ms,
                                    "tps": tps,
                                    "ttftMs": ttft_ms,
                                    "systemTokens": system_tokens,
                                    "toolTokens": tool_tokens,
                                }),
                            );
                        }
                    }
                }
                let telem_record = crate::db::TelemetryRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    task_id: self.subagent_id.clone(),
                    model: Some(self.model.clone()),
                    input_tokens: input_tokens as i64,
                    output_tokens: output_tokens as i64,
                    cached_tokens: cache_read_tokens as i64,
                    tps: tps as f64,
                    ttft_ms,
                    duration_ms,
                    created_at: chrono::Utc::now().timestamp_millis(),
                };
                let _ = crate::db::db_record_telemetry(&self.repo_root, &telem_record);
            }
            AgentEvent::Error(ref err) => {
                eprintln!("CVH [subagent]: error: {}", err);
            }
            _ => {}
        }
    }

    fn dialog_key(&self) -> &str {
        &self.subagent_dialog_id
    }

    fn worktree_root(&self) -> &Path {
        &self.worktree_dir
    }

    fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    fn task_id(&self) -> &str {
        &self.subagent_id
    }

    fn on_turn_finished(&self, _turn: usize, _messages: &[ChatMessage]) {
        self.finalize_turn_thoughts();
    }
}

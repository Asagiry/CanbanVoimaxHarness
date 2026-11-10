use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    #[serde(default = "default_todo_id")]
    pub id: String,
    pub content: String,
    pub status: String,
}

fn default_todo_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Task {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "stageId", alias = "stage_id", alias = "columnId")]
    pub stage_id: String,
    #[serde(rename = "boardId", alias = "board_id", default)]
    pub board_id: Option<String>,
    #[serde(rename = "worktreePath", alias = "worktree_path", default)]
    pub worktree_path: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub verdict: Option<String>,
    #[serde(rename = "customFlow", alias = "custom_flow", default)]
    pub custom_flow: Option<Vec<String>>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(rename = "providerId", alias = "provider_id", default)]
    pub provider_id: Option<String>,
    #[serde(rename = "reasoningEffort", alias = "reasoning_effort", default)]
    pub reasoning_effort: Option<String>,
    #[serde(rename = "parentId", alias = "parent_id", default)]
    pub parent_id: Option<String>,
    #[serde(rename = "subtaskIndex", alias = "subtask_index", default)]
    pub subtask_index: Option<u32>,
    #[serde(rename = "waitingForSubtaskId", alias = "waiting_for_subtask_id", default)]
    pub waiting_for_subtask_id: Option<String>,
    #[serde(rename = "handoverSummary", alias = "handover_summary", default)]
    pub handover_summary: Option<String>,
    #[serde(rename = "coverImage", alias = "cover_image", default)]
    pub cover_image: Option<String>,
    #[serde(default)]
    pub attachments: Option<Vec<String>>,
    #[serde(default)]
    pub todos: Option<Vec<TodoItem>>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(rename = "currentTokens", alias = "current_tokens", default)]
    pub current_tokens: Option<usize>,
    #[serde(
        rename = "skipWorktree",
        alias = "skip_worktree",
        alias = "skipBacklogHook",
        alias = "skip_backlog_hook",
        default
    )]
    pub skip_worktree: Option<bool>,
    #[serde(rename = "createdAt", alias = "created_at", default)]
    pub created_at: i64,
    #[serde(rename = "updatedAt", alias = "updated_at", default)]
    pub updated_at: i64,
    #[serde(rename = "stageEnteredAt", alias = "stage_entered_at", default)]
    pub stage_entered_at: Option<i64>,
    #[serde(rename = "completedAt", alias = "completed_at", default)]
    pub completed_at: Option<i64>,
    #[serde(default)]
    pub position: Option<f64>,
    #[serde(rename = "hookStates", alias = "hook_states", default)]
    pub hook_states: Option<serde_json::Value>,
    #[serde(rename = "projectPath", alias = "project_path", default)]
    pub project_path: Option<String>,
    #[serde(rename = "stageDurations", alias = "stage_durations", default)]
    pub stage_durations: Option<serde_json::Value>,
    #[serde(rename = "totalElapsed", alias = "total_elapsed", default)]
    pub total_elapsed: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Idle,
    Running,
    Paused,
    WaitingReview,
    WaitingSubtask,
    WaitingSubagent,
    Blocked,
    Done,
    #[serde(untagged)]
    Custom(String),
}

impl TaskStatus {
    pub fn as_str(&self) -> &str {
        match self {
            TaskStatus::Idle => "idle",
            TaskStatus::Running => "running",
            TaskStatus::Paused => "paused",
            TaskStatus::WaitingReview => "waiting_review",
            TaskStatus::WaitingSubtask => "waiting_subtask",
            TaskStatus::WaitingSubagent => "waiting_subagent",
            TaskStatus::Blocked => "blocked",
            TaskStatus::Done => "done",
            TaskStatus::Custom(s) => s.as_str(),
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<&str> for TaskStatus {
    fn from(s: &str) -> Self {
        match s {
            "idle" => TaskStatus::Idle,
            "running" => TaskStatus::Running,
            "paused" => TaskStatus::Paused,
            "waiting_review" => TaskStatus::WaitingReview,
            "waiting_subtask" => TaskStatus::WaitingSubtask,
            "waiting_subagent" => TaskStatus::WaitingSubagent,
            "blocked" => TaskStatus::Blocked,
            "done" => TaskStatus::Done,
            other => TaskStatus::Custom(other.to_string()),
        }
    }
}

impl From<String> for TaskStatus {
    fn from(s: String) -> Self {
        TaskStatus::from(s.as_str())
    }
}

impl Task {
    pub fn get_status(&self) -> TaskStatus {
        self.status.as_deref().map(TaskStatus::from).unwrap_or(TaskStatus::Idle)
    }

    pub fn set_status(&mut self, status: TaskStatus) {
        self.status = Some(status.as_str().to_string());
    }

    pub fn is_done(&self) -> bool {
        self.get_status() == TaskStatus::Done
    }

    pub fn is_running(&self) -> bool {
        self.get_status() == TaskStatus::Running
    }

    pub fn is_waiting_subtask(&self) -> bool {
        self.get_status() == TaskStatus::WaitingSubtask
    }

    pub fn is_waiting_subagent(&self) -> bool {
        self.get_status() == TaskStatus::WaitingSubagent
    }

    /// Returns the strongly-typed domain `TaskState` representation of this task.
    pub fn state(&self) -> crate::domain::TaskState {
        crate::domain::TaskState::from_task(self)
    }

    /// Sets the task state using the domain `TaskState` state machine,
    /// synchronizing status, model, subtask reference, and timestamp.
    pub fn set_state(&mut self, state: crate::domain::TaskState) {
        state.apply_to_task(self);
    }
}

pub type KanbanTask = Task;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum AgentEvent {
    StreamStart { id: String },
    StreamRetry { id: String, delay_secs: u64 },
    Token(String),
    Output(String),
    Thought(String),
    ToolQueued {
        id: String,
        name: String,
        arguments: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        batch: Option<serde_json::Value>,
    },
    ToolCall {
        id: String,
        name: String,
        arguments: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        batch: Option<serde_json::Value>,
    },
    ToolResult {
        id: String,
        name: String,
        result: String,
        success: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration_ms: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        batch: Option<serde_json::Value>,
    },
    TurnStart {
        turn: usize,
    },
    Telemetry {
        input_tokens: u64,
        output_tokens: u64,
        cache_read_tokens: u64,
        cache_creation_tokens: u64,
        cache_hit_rate: f32,
        duration_ms: f64,
        tps: f32,
        ttft_ms: f64,
        system_tokens: u64,
        tool_tokens: u64,
    },
    Compacted {
        saved_chars: usize,
        estimated_saved_tokens: usize,
        evaporated_count: usize,
        tokens_before: usize,
        tokens_after: usize,
    },
    Finished {
        final_response: String,
    },
    TaskUpdated {
        task_id: String,
        todos: Vec<TodoItem>,
    },
    Aborted,
    Error(String),
}

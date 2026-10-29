//! Task State Machine Domain Model (§ Phase 3.2 DDD Foundation)
//!
//! Enforces Uncle Bob's Clean Architecture & ub-mode principles:
//! - Make Illegal States Unrepresentable via Algebraic Data Types (ADT)
//! - Parse-don't-validate boundary discipline
//! - Pure transition functions and invariant verification
//! - Zero-risk pure additive layer over legacy Task struct

use std::fmt;
use std::str::FromStr;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::agent::{Task, TaskStatus};

/// Domain error variants for task state validation and transitions.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StateError {
    #[error("Subtask ID cannot be empty or blank when entering WaitingSubtask state")]
    EmptySubtaskId,

    #[error("Model identifier cannot be empty or blank when entering Running state")]
    EmptyModel,

    #[error("Illegal state transition from '{from}' to '{to}': {reason}")]
    InvalidTransition {
        from: String,
        to: String,
        reason: String,
    },

    #[error("Conflicting task state: Task in '{status}' status cannot have waiting_for_subtask_id '{subtask_id}'")]
    ConflictingSubtaskInTerminalState {
        status: String,
        subtask_id: String,
    },

    #[error("Task is marked as 'waiting_subtask' but waiting_for_subtask_id is missing or empty")]
    MissingSubtaskId,

    #[error("Unknown or invalid state: '{0}'")]
    InvalidState(String),
}

impl From<StateError> for String {
    fn from(err: StateError) -> Self {
        err.to_string()
    }
}

/// Algebraic Data Type representing the authoritative lifecycle state of a Task.
///
/// Invariants:
/// - `Idle`: Task is at rest in a stage, no agent is currently running.
/// - `Running { model }`: Agent is actively working on the task with a non-empty model identifier.
/// - `WaitingSubtask { subtask_id }`: Task is blocked awaiting completion of a child subtask.
/// - `Done { verdict }`: Task is finalized (optionally with reviewer/retro verdict).
///   An illegal state such as `Done` with an active child subtask is unrepresentable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskState {
    Idle,
    Running {
        model: String,
    },
    WaitingSubtask {
        subtask_id: String,
    },
    Done {
        verdict: Option<String>,
    },
}

impl TaskState {
    /// Returns the canonical string identifier of this state (matches `Task.status`).
    pub fn as_str(&self) -> &str {
        match self {
            TaskState::Idle => "idle",
            TaskState::Running { .. } => "running",
            TaskState::WaitingSubtask { .. } => "waiting_subtask",
            TaskState::Done { .. } => "done",
        }
    }

    /// Returns `true` if the task is currently idle.
    pub fn is_idle(&self) -> bool {
        matches!(self, TaskState::Idle)
    }

    /// Returns `true` if the task is currently running an agent.
    pub fn is_running(&self) -> bool {
        matches!(self, TaskState::Running { .. })
    }

    /// Returns `true` if the task is waiting on a child subtask.
    pub fn is_waiting_subtask(&self) -> bool {
        matches!(self, TaskState::WaitingSubtask { .. })
    }

    /// Returns `true` if the task is completed / finalized.
    pub fn is_done(&self) -> bool {
        matches!(self, TaskState::Done { .. })
    }

    /// Returns the model identifier if in `Running` state, or `None` otherwise.
    pub fn model(&self) -> Option<&str> {
        match self {
            TaskState::Running { model } => Some(model.as_str()),
            _ => None,
        }
    }

    /// Returns the subtask identifier if in `WaitingSubtask` state, or `None` otherwise.
    pub fn subtask_id(&self) -> Option<&str> {
        match self {
            TaskState::WaitingSubtask { subtask_id } => Some(subtask_id.as_str()),
            _ => None,
        }
    }

    /// Returns the completion verdict if in `Done` state, or `None` otherwise.
    pub fn verdict(&self) -> Option<&str> {
        match self {
            TaskState::Done { verdict } => verdict.as_deref(),
            _ => None,
        }
    }

    /// Constructs a `TaskState` from a legacy `Task` struct.
    ///
    /// Performs non-failing conversion, resolving missing fields to safe defaults.
    pub fn from_task(task: &Task) -> Self {
        match task.status.as_deref().unwrap_or("idle") {
            "running" => {
                let model = task
                    .model
                    .as_deref()
                    .map(|m| m.trim())
                    .filter(|m| !m.is_empty())
                    .unwrap_or("default")
                    .to_string();
                TaskState::Running { model }
            }
            "waiting_subtask" => {
                let subtask_id = task
                    .waiting_for_subtask_id
                    .as_deref()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_default()
                    .to_string();
                TaskState::WaitingSubtask { subtask_id }
            }
            "done" => {
                let verdict = task
                    .verdict
                    .as_deref()
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty())
                    .map(|v| v.to_string());
                TaskState::Done { verdict }
            }
            _ => TaskState::Idle,
        }
    }

    /// Validates domain invariants on a `Task` before converting into `TaskState`.
    ///
    /// Rejects illegal states such as:
    /// - Task marked as `done` while still holding a `waiting_for_subtask_id`
    /// - Task marked as `waiting_subtask` with missing or blank `waiting_for_subtask_id`
    pub fn validate_task(task: &Task) -> Result<(), StateError> {
        let status = task.status.as_deref().unwrap_or("idle");

        if status == "done" {
            if let Some(ref sub_id) = task.waiting_for_subtask_id {
                let trimmed = sub_id.trim();
                if !trimmed.is_empty() {
                    return Err(StateError::ConflictingSubtaskInTerminalState {
                        status: "done".to_string(),
                        subtask_id: trimmed.to_string(),
                    });
                }
            }
        } else if status == "waiting_subtask" {
            let is_empty_sub = task
                .waiting_for_subtask_id
                .as_ref()
                .map(|s| s.trim().is_empty())
                .unwrap_or(true);
            if is_empty_sub {
                return Err(StateError::MissingSubtaskId);
            }
        }

        Ok(())
    }

    /// Validates the task and converts it into a validated `TaskState`.
    pub fn try_from_task(task: &Task) -> Result<Self, StateError> {
        Self::validate_task(task)?;
        Ok(Self::from_task(task))
    }

    /// Synchronizes this `TaskState` into a mutable `Task` reference.
    ///
    /// Guarantees that illegal state combinations (such as `done` + `waiting_for_subtask_id`)
    /// are automatically cleared upon synchronization.
    pub fn apply_to_task(&self, task: &mut Task) {
        match self {
            TaskState::Idle => {
                task.status = Some("idle".to_string());
                task.waiting_for_subtask_id = None;
            }
            TaskState::Running { model } => {
                task.status = Some("running".to_string());
                task.model = Some(model.clone());
                task.waiting_for_subtask_id = None;
            }
            TaskState::WaitingSubtask { subtask_id } => {
                task.status = Some("waiting_subtask".to_string());
                task.waiting_for_subtask_id = Some(subtask_id.clone());
            }
            TaskState::Done { verdict } => {
                task.status = Some("done".to_string());
                task.verdict = verdict.clone();
                task.waiting_for_subtask_id = None;
            }
        }
        task.updated_at = chrono::Utc::now().timestamp_millis();
    }

    /// Pure state transition to `Running`.
    pub fn transition_to_running(&self, model: impl AsRef<str>) -> Result<Self, StateError> {
        transition_to_running(self, model)
    }

    /// Pure state transition to `WaitingSubtask`.
    pub fn transition_to_waiting(&self, subtask_id: impl AsRef<str>) -> Result<Self, StateError> {
        transition_to_waiting(self, subtask_id)
    }

    /// Pure state transition to `Done`.
    pub fn transition_to_done(&self, verdict: Option<String>) -> Result<Self, StateError> {
        transition_to_done(self, verdict)
    }

    /// Pure state transition to `Idle`.
    pub fn transition_to_idle(&self) -> Result<Self, StateError> {
        transition_to_idle(self)
    }

    /// Converts this state to the legacy `TaskStatus` enum.
    pub fn to_status(&self) -> TaskStatus {
        TaskStatus::from(self)
    }
}

// ============================================================================
// Pure Transition Functions
// ============================================================================

/// Pure transition function: advances current state to `Running { model }`.
///
/// Invariants:
/// - `model` cannot be empty or blank.
/// - Transitions from `Idle`, `Running` (model update), `WaitingSubtask` (subtask completed),
///   or `Done` (reopening task) are valid.
pub fn transition_to_running(
    _current: &TaskState,
    model: impl AsRef<str>,
) -> Result<TaskState, StateError> {
    let trimmed_model = model.as_ref().trim();
    if trimmed_model.is_empty() {
        return Err(StateError::EmptyModel);
    }
    Ok(TaskState::Running {
        model: trimmed_model.to_string(),
    })
}

/// Pure transition function: advances current state to `WaitingSubtask { subtask_id }`.
///
/// Invariants:
/// - `subtask_id` cannot be empty or blank.
/// - Illegal transition: A task in `Done` cannot transition to `WaitingSubtask`
///   (completed tasks cannot wait for subtasks).
pub fn transition_to_waiting(
    current: &TaskState,
    subtask_id: impl AsRef<str>,
) -> Result<TaskState, StateError> {
    if let TaskState::Done { .. } = current {
        return Err(StateError::InvalidTransition {
            from: "done".to_string(),
            to: "waiting_subtask".to_string(),
            reason: "Completed task in Done state cannot transition to WaitingSubtask; reopen to running or idle first".to_string(),
        });
    }

    let trimmed_subtask = subtask_id.as_ref().trim();
    if trimmed_subtask.is_empty() {
        return Err(StateError::EmptySubtaskId);
    }

    Ok(TaskState::WaitingSubtask {
        subtask_id: trimmed_subtask.to_string(),
    })
}

/// Pure transition function: advances current state to `Done { verdict }`.
///
/// Invariants:
/// - Illegal transition: A task in `WaitingSubtask` cannot transition directly to `Done`
///   (cannot be done while actively waiting on a child subtask; cancel or resolve subtask first).
pub fn transition_to_done(
    current: &TaskState,
    verdict: Option<String>,
) -> Result<TaskState, StateError> {
    if let TaskState::WaitingSubtask { subtask_id } = current {
        return Err(StateError::InvalidTransition {
            from: "waiting_subtask".to_string(),
            to: "done".to_string(),
            reason: format!(
                "Cannot transition directly from WaitingSubtask to Done while subtask '{subtask_id}' is active; resolve or cancel subtask first"
            ),
        });
    }

    let normalized_verdict = verdict
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    Ok(TaskState::Done {
        verdict: normalized_verdict,
    })
}

/// Pure transition function: resets current state to `Idle`.
///
/// Allowed from any state.
pub fn transition_to_idle(_current: &TaskState) -> Result<TaskState, StateError> {
    Ok(TaskState::Idle)
}

// ============================================================================
// Trait Implementations (Display, Serde, Conversions)
// ============================================================================

impl fmt::Display for TaskState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for TaskState {
    type Err = StateError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "idle" => Ok(TaskState::Idle),
            "running" => Ok(TaskState::Running {
                model: "default".to_string(),
            }),
            "waiting_subtask" => Ok(TaskState::WaitingSubtask {
                subtask_id: String::new(),
            }),
            "done" => Ok(TaskState::Done { verdict: None }),
            other => Err(StateError::InvalidState(other.to_string())),
        }
    }
}

impl TryFrom<&str> for TaskState {
    type Error = StateError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
    }
}

impl TryFrom<String> for TaskState {
    type Error = StateError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

impl From<&TaskState> for TaskStatus {
    fn from(state: &TaskState) -> Self {
        match state {
            TaskState::Idle => TaskStatus::Idle,
            TaskState::Running { .. } => TaskStatus::Running,
            TaskState::WaitingSubtask { .. } => TaskStatus::WaitingSubtask,
            TaskState::Done { .. } => TaskStatus::Done,
        }
    }
}

impl From<TaskState> for TaskStatus {
    fn from(state: TaskState) -> Self {
        TaskStatus::from(&state)
    }
}

impl From<TaskStatus> for TaskState {
    fn from(status: TaskStatus) -> Self {
        match status {
            TaskStatus::Idle => TaskState::Idle,
            TaskStatus::Running => TaskState::Running {
                model: "default".to_string(),
            },
            TaskStatus::WaitingSubtask => TaskState::WaitingSubtask {
                subtask_id: String::new(),
            },
            TaskStatus::Done => TaskState::Done { verdict: None },
            _ => TaskState::Idle,
        }
    }
}

// ============================================================================
// Serde: Canonical Tagged JSON with Bare String Fallback Deserialization
// ============================================================================

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum TaggedSerialize<'a> {
    Idle,
    Running {
        model: &'a str,
    },
    WaitingSubtask {
        subtask_id: &'a str,
    },
    Done {
        #[serde(skip_serializing_if = "Option::is_none")]
        verdict: Option<&'a str>,
    },
}

impl Serialize for TaskState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            TaskState::Idle => TaggedSerialize::Idle.serialize(serializer),
            TaskState::Running { model } => TaggedSerialize::Running {
                model: model.as_str(),
            }
            .serialize(serializer),
            TaskState::WaitingSubtask { subtask_id } => TaggedSerialize::WaitingSubtask {
                subtask_id: subtask_id.as_str(),
            }
            .serialize(serializer),
            TaskState::Done { verdict } => TaggedSerialize::Done {
                verdict: verdict.as_deref(),
            }
            .serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for TaskState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(tag = "status", rename_all = "snake_case")]
        enum Tagged {
            Idle,
            Running {
                #[serde(default = "default_model")]
                model: String,
            },
            WaitingSubtask {
                #[serde(alias = "subtaskId", default)]
                subtask_id: String,
            },
            Done {
                #[serde(default)]
                verdict: Option<String>,
            },
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Helper {
            Tagged(Tagged),
            Str(String),
        }

        fn default_model() -> String {
            "default".to_string()
        }

        match Helper::deserialize(deserializer)? {
            Helper::Tagged(tagged) => match tagged {
                Tagged::Idle => Ok(TaskState::Idle),
                Tagged::Running { model } => Ok(TaskState::Running { model }),
                Tagged::WaitingSubtask { subtask_id } => Ok(TaskState::WaitingSubtask { subtask_id }),
                Tagged::Done { verdict } => Ok(TaskState::Done { verdict }),
            },
            Helper::Str(s) => TaskState::from_str(&s).map_err(serde::de::Error::custom),
        }
    }
}

//! Unit Tests for TaskState Machine Domain Model (§ 3.2 ub-mode DDD foundation)
//!
//! 100% test coverage for:
//! - State variants and predicate methods (is_idle, is_running, is_waiting_subtask, is_done)
//! - Model, subtask_id, and verdict accessors
//! - String representations (as_str, Display, FromStr, TryFrom)
//! - Conversions with legacy TaskStatus enum
//! - JSON Serialization & Deserialization (tagged, aliases, and bare strings)
//! - Non-failing Task conversion (from_task) and strict validation (validate_task, try_from_task)
//! - Mutation synchronization (apply_to_task, task.state(), task.set_state())
//! - Pure transition functions and illegal state protection:
//!   * transition_to_running (valid from any state, rejects empty model)
//!   * transition_to_waiting (rejects empty subtask_id, rejects transition from Done)
//!   * transition_to_done (rejects transition from WaitingSubtask, normalizes verdict)
//!   * transition_to_idle (valid from all states)

use std::str::FromStr;
use crate::agent::{Task, TaskStatus};
use crate::domain::state::{
    transition_to_done, transition_to_idle, transition_to_running, transition_to_waiting,
    StateError, TaskState,
};
use crate::domain::DomainError;

// ============================================================================
// State Variants and Accessors
// ============================================================================

#[test]
fn test_state_variants_and_predicates() {
    let idle = TaskState::Idle;
    assert!(idle.is_idle());
    assert!(!idle.is_running());
    assert!(!idle.is_waiting_subtask());
    assert!(!idle.is_done());
    assert_eq!(idle.as_str(), "idle");
    assert_eq!(idle.model(), None);
    assert_eq!(idle.subtask_id(), None);
    assert_eq!(idle.verdict(), None);

    let running = TaskState::Running {
        model: "gemini-3.8-flash-high".to_string(),
    };
    assert!(!running.is_idle());
    assert!(running.is_running());
    assert!(!running.is_waiting_subtask());
    assert!(!running.is_done());
    assert_eq!(running.as_str(), "running");
    assert_eq!(running.model(), Some("gemini-3.8-flash-high"));
    assert_eq!(running.subtask_id(), None);
    assert_eq!(running.verdict(), None);

    let waiting = TaskState::WaitingSubtask {
        subtask_id: "CVH-042.1".to_string(),
    };
    assert!(!waiting.is_idle());
    assert!(!waiting.is_running());
    assert!(waiting.is_waiting_subtask());
    assert!(!waiting.is_done());
    assert_eq!(waiting.as_str(), "waiting_subtask");
    assert_eq!(waiting.model(), None);
    assert_eq!(waiting.subtask_id(), Some("CVH-042.1"));
    assert_eq!(waiting.verdict(), None);

    let done = TaskState::Done {
        verdict: Some("APPROVED".to_string()),
    };
    assert!(!done.is_idle());
    assert!(!done.is_running());
    assert!(!done.is_waiting_subtask());
    assert!(done.is_done());
    assert_eq!(done.as_str(), "done");
    assert_eq!(done.model(), None);
    assert_eq!(done.subtask_id(), None);
    assert_eq!(done.verdict(), Some("APPROVED"));

    let done_no_verdict = TaskState::Done { verdict: None };
    assert!(done_no_verdict.is_done());
    assert_eq!(done_no_verdict.verdict(), None);
}

#[test]
fn test_display_and_from_str() {
    assert_eq!(format!("{}", TaskState::Idle), "idle");
    assert_eq!(
        format!(
            "{}",
            TaskState::Running {
                model: "claude-3-7-sonnet".to_string()
            }
        ),
        "running"
    );
    assert_eq!(
        format!(
            "{}",
            TaskState::WaitingSubtask {
                subtask_id: "CVH-001.2".to_string()
            }
        ),
        "waiting_subtask"
    );
    assert_eq!(
        format!("{}", TaskState::Done { verdict: None }),
        "done"
    );

    // FromStr
    assert_eq!(TaskState::from_str("idle").unwrap(), TaskState::Idle);
    assert_eq!(
        TaskState::from_str("running").unwrap(),
        TaskState::Running {
            model: "default".to_string()
        }
    );
    assert_eq!(
        TaskState::from_str("waiting_subtask").unwrap(),
        TaskState::WaitingSubtask {
            subtask_id: String::new()
        }
    );
    assert_eq!(
        TaskState::from_str("done").unwrap(),
        TaskState::Done { verdict: None }
    );

    // TryFrom &str and String
    assert_eq!(TaskState::try_from("  IDLE  ").unwrap(), TaskState::Idle);
    assert_eq!(
        TaskState::try_from("RUNNING".to_string()).unwrap(),
        TaskState::Running {
            model: "default".to_string()
        }
    );

    // Invalid string
    assert!(matches!(
        TaskState::from_str("invalid_state"),
        Err(StateError::InvalidState(_))
    ));
}

// ============================================================================
// Serde JSON Serialization & Deserialization
// ============================================================================

#[test]
fn test_serde_json_canonical_tagged() {
    // 1. Idle
    let idle = TaskState::Idle;
    let idle_json = serde_json::to_string(&idle).expect("serialize idle");
    assert_eq!(idle_json, r#"{"status":"idle"}"#);
    let idle_deser: TaskState = serde_json::from_str(&idle_json).expect("deserialize idle");
    assert_eq!(idle_deser, idle);

    // 2. Running
    let running = TaskState::Running {
        model: "gemini-3.8-flash-high".to_string(),
    };
    let running_json = serde_json::to_string(&running).expect("serialize running");
    assert_eq!(
        running_json,
        r#"{"status":"running","model":"gemini-3.8-flash-high"}"#
    );
    let running_deser: TaskState = serde_json::from_str(&running_json).expect("deserialize running");
    assert_eq!(running_deser, running);

    // 3. WaitingSubtask with snake_case and camelCase alias
    let waiting = TaskState::WaitingSubtask {
        subtask_id: "CVH-010.1".to_string(),
    };
    let waiting_json = serde_json::to_string(&waiting).expect("serialize waiting");
    assert_eq!(
        waiting_json,
        r#"{"status":"waiting_subtask","subtask_id":"CVH-010.1"}"#
    );
    let waiting_deser: TaskState = serde_json::from_str(&waiting_json).expect("deserialize waiting");
    assert_eq!(waiting_deser, waiting);

    // Deserialization with camelCase alias subtaskId
    let camel_json = r#"{"status":"waiting_subtask","subtaskId":"CVH-010.1"}"#;
    let camel_deser: TaskState = serde_json::from_str(camel_json).expect("deserialize camelCase waiting");
    assert_eq!(camel_deser, waiting);

    // 4. Done with verdict
    let done_approved = TaskState::Done {
        verdict: Some("APPROVED".to_string()),
    };
    let done_approved_json = serde_json::to_string(&done_approved).expect("serialize done approved");
    assert_eq!(done_approved_json, r#"{"status":"done","verdict":"APPROVED"}"#);
    let done_approved_deser: TaskState =
        serde_json::from_str(&done_approved_json).expect("deserialize done approved");
    assert_eq!(done_approved_deser, done_approved);

    // 5. Done without verdict (verdict omitted when None)
    let done_none = TaskState::Done { verdict: None };
    let done_none_json = serde_json::to_string(&done_none).expect("serialize done none");
    assert_eq!(done_none_json, r#"{"status":"done"}"#);
    let done_none_deser: TaskState = serde_json::from_str(&done_none_json).expect("deserialize done none");
    assert_eq!(done_none_deser, done_none);
}

#[test]
fn test_serde_json_bare_string_fallback() {
    let idle: TaskState = serde_json::from_str(r#""idle""#).expect("deserialize bare string idle");
    assert_eq!(idle, TaskState::Idle);

    let running: TaskState = serde_json::from_str(r#""running""#).expect("deserialize bare string running");
    assert_eq!(
        running,
        TaskState::Running {
            model: "default".to_string()
        }
    );

    let waiting: TaskState =
        serde_json::from_str(r#""waiting_subtask""#).expect("deserialize bare string waiting");
    assert_eq!(
        waiting,
        TaskState::WaitingSubtask {
            subtask_id: String::new()
        }
    );

    let done: TaskState = serde_json::from_str(r#""done""#).expect("deserialize bare string done");
    assert_eq!(done, TaskState::Done { verdict: None });

    // Invalid string
    assert!(serde_json::from_str::<TaskState>(r#""not_a_valid_state""#).is_err());
}

// ============================================================================
// TaskStatus Conversion Invariants
// ============================================================================

#[test]
fn test_task_status_interop() {
    assert_eq!(TaskStatus::from(&TaskState::Idle), TaskStatus::Idle);
    assert_eq!(
        TaskStatus::from(TaskState::Running {
            model: "foo".to_string()
        }),
        TaskStatus::Running
    );
    assert_eq!(
        TaskStatus::from(TaskState::WaitingSubtask {
            subtask_id: "sub-1".to_string()
        }),
        TaskStatus::WaitingSubtask
    );
    assert_eq!(
        TaskStatus::from(TaskState::Done { verdict: None }),
        TaskStatus::Done
    );

    // From TaskStatus to TaskState
    assert_eq!(TaskState::from(TaskStatus::Idle), TaskState::Idle);
    assert_eq!(
        TaskState::from(TaskStatus::Running),
        TaskState::Running {
            model: "default".to_string()
        }
    );
    assert_eq!(
        TaskState::from(TaskStatus::WaitingSubtask),
        TaskState::WaitingSubtask {
            subtask_id: String::new()
        }
    );
    assert_eq!(
        TaskState::from(TaskStatus::Done),
        TaskState::Done { verdict: None }
    );
    assert_eq!(
        TaskState::from(TaskStatus::Paused),
        TaskState::Idle
    );
}

// ============================================================================
// Legacy Task Struct Integration (from_task, try_from_task, apply_to_task)
// ============================================================================

#[test]
fn test_from_task_mapping() {
    let mut task = Task {
        id: "CVH-100".to_string(),
        title: "Test Task".to_string(),
        status: None,
        ..Default::default()
    };

    // None defaults to Idle
    assert_eq!(TaskState::from_task(&task), TaskState::Idle);

    // Idle
    task.status = Some("idle".to_string());
    assert_eq!(TaskState::from_task(&task), TaskState::Idle);

    // Running with model
    task.status = Some("running".to_string());
    task.model = Some("gemini-3.8-flash-high".to_string());
    assert_eq!(
        TaskState::from_task(&task),
        TaskState::Running {
            model: "gemini-3.8-flash-high".to_string()
        }
    );

    // Running with blank model falls back to "default"
    task.model = Some("   ".to_string());
    assert_eq!(
        TaskState::from_task(&task),
        TaskState::Running {
            model: "default".to_string()
        }
    );

    // WaitingSubtask with subtask_id
    task.status = Some("waiting_subtask".to_string());
    task.waiting_for_subtask_id = Some("CVH-100.1".to_string());
    assert_eq!(
        TaskState::from_task(&task),
        TaskState::WaitingSubtask {
            subtask_id: "CVH-100.1".to_string()
        }
    );

    // Done with verdict
    task.status = Some("done".to_string());
    task.verdict = Some("APPROVED_WITH_CAVEATS".to_string());
    assert_eq!(
        TaskState::from_task(&task),
        TaskState::Done {
            verdict: Some("APPROVED_WITH_CAVEATS".to_string())
        }
    );
}

#[test]
fn test_task_state_validation_make_illegal_states_unrepresentable() {
    // 1. Valid tasks pass
    let valid_done = Task {
        id: "CVH-001".to_string(),
        status: Some("done".to_string()),
        verdict: Some("APPROVED".to_string()),
        waiting_for_subtask_id: None,
        ..Default::default()
    };
    assert!(TaskState::validate_task(&valid_done).is_ok());
    assert!(TaskState::try_from_task(&valid_done).is_ok());

    // 2. Illegal State: Done with active waiting_for_subtask_id
    let illegal_done_waiting = Task {
        id: "CVH-002".to_string(),
        status: Some("done".to_string()),
        verdict: Some("APPROVED".to_string()),
        waiting_for_subtask_id: Some("CVH-002.1".to_string()),
        ..Default::default()
    };
    let err = TaskState::validate_task(&illegal_done_waiting).unwrap_err();
    assert_eq!(
        err,
        StateError::ConflictingSubtaskInTerminalState {
            status: "done".to_string(),
            subtask_id: "CVH-002.1".to_string(),
        }
    );
    assert!(TaskState::try_from_task(&illegal_done_waiting).is_err());

    // 3. Illegal State: WaitingSubtask without waiting_for_subtask_id
    let illegal_waiting_empty = Task {
        id: "CVH-003".to_string(),
        status: Some("waiting_subtask".to_string()),
        waiting_for_subtask_id: None,
        ..Default::default()
    };
    assert_eq!(
        TaskState::validate_task(&illegal_waiting_empty).unwrap_err(),
        StateError::MissingSubtaskId
    );

    let illegal_waiting_blank = Task {
        id: "CVH-004".to_string(),
        status: Some("waiting_subtask".to_string()),
        waiting_for_subtask_id: Some("   ".to_string()),
        ..Default::default()
    };
    assert_eq!(
        TaskState::validate_task(&illegal_waiting_blank).unwrap_err(),
        StateError::MissingSubtaskId
    );
}

#[test]
fn test_apply_to_task_and_task_methods() {
    let mut task = Task {
        id: "CVH-200".to_string(),
        title: "Integration Task".to_string(),
        status: Some("idle".to_string()),
        ..Default::default()
    };

    // Use task.set_state to Running
    let running_state = TaskState::Running {
        model: "deepseek-coder".to_string(),
    };
    task.set_state(running_state.clone());
    assert_eq!(task.status.as_deref(), Some("running"));
    assert_eq!(task.model.as_deref(), Some("deepseek-coder"));
    assert_eq!(task.waiting_for_subtask_id, None);
    assert_eq!(task.state(), running_state);

    // Transition to WaitingSubtask
    let waiting_state = TaskState::WaitingSubtask {
        subtask_id: "CVH-200.1".to_string(),
    };
    task.set_state(waiting_state.clone());
    assert_eq!(task.status.as_deref(), Some("waiting_subtask"));
    assert_eq!(task.waiting_for_subtask_id.as_deref(), Some("CVH-200.1"));
    assert_eq!(task.state(), waiting_state);

    // Transition to Done (clears waiting_for_subtask_id to guarantee invariant)
    let done_state = TaskState::Done {
        verdict: Some("APPROVED".to_string()),
    };
    task.set_state(done_state.clone());
    assert_eq!(task.status.as_deref(), Some("done"));
    assert_eq!(task.verdict.as_deref(), Some("APPROVED"));
    assert_eq!(task.waiting_for_subtask_id, None); // Invariant maintained!
    assert_eq!(task.state(), done_state);

    // Transition back to Idle
    task.set_state(TaskState::Idle);
    assert_eq!(task.status.as_deref(), Some("idle"));
    assert_eq!(task.waiting_for_subtask_id, None);
    assert_eq!(task.state(), TaskState::Idle);
}

// ============================================================================
// Pure Transition Functions Tests
// ============================================================================

#[test]
fn test_transition_to_running() {
    let idle = TaskState::Idle;
    let running = transition_to_running(&idle, "claude-3-7-sonnet").expect("transition to running");
    assert_eq!(
        running,
        TaskState::Running {
            model: "claude-3-7-sonnet".to_string()
        }
    );

    // Model update from Running
    let running2 = running
        .transition_to_running("gemini-3.8-flash-high")
        .expect("update model");
    assert_eq!(
        running2,
        TaskState::Running {
            model: "gemini-3.8-flash-high".to_string()
        }
    );

    // Resume from WaitingSubtask
    let waiting = TaskState::WaitingSubtask {
        subtask_id: "sub-1".to_string(),
    };
    let resumed = waiting
        .transition_to_running("claude-3-7-sonnet")
        .expect("resume");
    assert_eq!(
        resumed,
        TaskState::Running {
            model: "claude-3-7-sonnet".to_string()
        }
    );

    // Reopen from Done
    let done = TaskState::Done { verdict: None };
    let reopened = done.transition_to_running("reopened-model").expect("reopen");
    assert_eq!(
        reopened,
        TaskState::Running {
            model: "reopened-model".to_string()
        }
    );

    // Empty model rejection
    assert_eq!(
        transition_to_running(&idle, ""),
        Err(StateError::EmptyModel)
    );
    assert_eq!(
        transition_to_running(&idle, "   "),
        Err(StateError::EmptyModel)
    );
}

#[test]
fn test_transition_to_waiting_and_invariants() {
    let running = TaskState::Running {
        model: "test-model".to_string(),
    };

    // Valid from Running
    let waiting = transition_to_waiting(&running, "CVH-001.1").expect("spawn subtask");
    assert_eq!(
        waiting,
        TaskState::WaitingSubtask {
            subtask_id: "CVH-001.1".to_string()
        }
    );

    // Valid from Idle
    let waiting_from_idle = TaskState::Idle
        .transition_to_waiting("CVH-001.2")
        .expect("waiting from idle");
    assert_eq!(
        waiting_from_idle,
        TaskState::WaitingSubtask {
            subtask_id: "CVH-001.2".to_string()
        }
    );

    // Empty subtask_id rejection
    assert_eq!(
        transition_to_waiting(&running, ""),
        Err(StateError::EmptySubtaskId)
    );
    assert_eq!(
        transition_to_waiting(&running, "   "),
        Err(StateError::EmptySubtaskId)
    );

    // CRUCIAL INVARIANT: Cannot transition from Done to WaitingSubtask!
    let done = TaskState::Done {
        verdict: Some("APPROVED".to_string()),
    };
    let done_err = done.transition_to_waiting("CVH-001.1").unwrap_err();
    assert!(matches!(done_err, StateError::InvalidTransition { .. }));
    assert!(done_err.to_string().contains("Completed task in Done state cannot transition to WaitingSubtask"));
}

#[test]
fn test_transition_to_done_and_invariants() {
    let running = TaskState::Running {
        model: "test-model".to_string(),
    };

    // Valid from Running
    let done = transition_to_done(&running, Some("APPROVED".to_string())).expect("transition to done");
    assert_eq!(
        done,
        TaskState::Done {
            verdict: Some("APPROVED".to_string())
        }
    );

    // Valid from Idle
    let done_from_idle = TaskState::Idle
        .transition_to_done(None)
        .expect("done from idle");
    assert_eq!(done_from_idle, TaskState::Done { verdict: None });

    // Whitespace verdict normalized to None
    let done_blank_verdict = running
        .transition_to_done(Some("   ".to_string()))
        .expect("done blank verdict");
    assert_eq!(done_blank_verdict, TaskState::Done { verdict: None });

    // CRUCIAL INVARIANT: Cannot transition to Done while in WaitingSubtask!
    let waiting = TaskState::WaitingSubtask {
        subtask_id: "CVH-001.1".to_string(),
    };
    let wait_err = waiting
        .transition_to_done(Some("APPROVED".to_string()))
        .unwrap_err();
    assert!(matches!(wait_err, StateError::InvalidTransition { .. }));
    assert!(wait_err.to_string().contains("Cannot transition directly from WaitingSubtask to Done"));
}

#[test]
fn test_transition_to_idle() {
    let running = TaskState::Running {
        model: "model".to_string(),
    };
    assert_eq!(transition_to_idle(&running).unwrap(), TaskState::Idle);

    let waiting = TaskState::WaitingSubtask {
        subtask_id: "sub-1".to_string(),
    };
    assert_eq!(waiting.transition_to_idle().unwrap(), TaskState::Idle);

    let done = TaskState::Done { verdict: None };
    assert_eq!(done.transition_to_idle().unwrap(), TaskState::Idle);

    let idle = TaskState::Idle;
    assert_eq!(idle.transition_to_idle().unwrap(), TaskState::Idle);
}

// ============================================================================
// DomainError Transparent Conversion
// ============================================================================

#[test]
fn test_domain_error_conversion() {
    let state_err = StateError::EmptyModel;
    let domain_err: DomainError = state_err.clone().into();
    assert_eq!(domain_err, DomainError::State(state_err));
    assert_eq!(
        domain_err.to_string(),
        "Model identifier cannot be empty or blank when entering Running state"
    );
}

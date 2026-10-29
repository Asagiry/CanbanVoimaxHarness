//! Unit Tests for Domain Value Objects (§ 3.1 ub-mode DDD foundation)
//!
//! 100% test coverage for:
//! - Normalization (trim, casing)
//! - Validation invariants (empty string rejection, character sets, length bounds)
//! - Equality & Ordering
//! - Deref coercion & Display
//! - Hashing & Borrow<str> lookups in HashSet / HashMap
//! - Serde JSON serialization & deserialization

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use super::ids::{BoardId, DomainError, StageId, TaskId};

// ============================================================================
// TaskId Tests
// ============================================================================

#[test]
fn test_task_id_valid_normalization() {
    let id1 = TaskId::new("  cvh-001  ").expect("valid task id");
    assert_eq!(id1.as_str(), "CVH-001");
    assert_eq!(&*id1, "CVH-001"); // Deref
    assert_eq!(format!("{}", id1), "CVH-001"); // Display

    let id2 = TaskId::new("CVH-042").expect("valid task id");
    assert_eq!(id2.as_str(), "CVH-042");

    let id3 = TaskId::new("task_123:sub:1").expect("valid task id");
    assert_eq!(id3.as_str(), "TASK_123:SUB:1");
}

#[test]
fn test_task_id_subtask_properties() {
    let root = TaskId::new("CVH-010").expect("valid");
    assert!(!root.is_subtask());
    assert_eq!(root.root_id(), "CVH-010");
    assert_eq!(root.subtask_index(), None);

    let sub1 = TaskId::new("CVH-010.1").expect("valid subtask");
    assert!(sub1.is_subtask());
    assert_eq!(sub1.root_id(), "CVH-010");
    assert_eq!(sub1.subtask_index(), Some(1));

    let sub2 = TaskId::new("cvh-010.42").expect("valid subtask");
    assert!(sub2.is_subtask());
    assert_eq!(sub2.root_id(), "CVH-010");
    assert_eq!(sub2.subtask_index(), Some(42));
}

#[test]
fn test_task_id_empty_rejection() {
    assert_eq!(TaskId::new(""), Err(DomainError::EmptyTaskId));
    assert_eq!(TaskId::new("   "), Err(DomainError::EmptyTaskId));
    assert_eq!(TaskId::new("	
"), Err(DomainError::EmptyTaskId));
}

#[test]
fn test_task_id_invalid_format_rejection() {
    // Spaces inside
    assert!(matches!(TaskId::new("CVH 001"), Err(DomainError::InvalidTaskIdFormat(_))));
    // Slash
    assert!(matches!(TaskId::new("feat/CVH-001"), Err(DomainError::InvalidTaskIdFormat(_))));
    // Special symbols
    assert!(matches!(TaskId::new("CVH@001"), Err(DomainError::InvalidTaskIdFormat(_))));
    assert!(matches!(TaskId::new("CVH#001"), Err(DomainError::InvalidTaskIdFormat(_))));
    assert!(matches!(TaskId::new("CVH$001"), Err(DomainError::InvalidTaskIdFormat(_))));
    // Length > 128
    let too_long = "A".repeat(129);
    assert!(matches!(TaskId::new(too_long), Err(DomainError::InvalidTaskIdFormat(_))));
}

#[test]
fn test_task_id_equality_and_comparisons() {
    let id_lower = TaskId::new("cvh-001").unwrap();
    let id_upper = TaskId::new("CVH-001").unwrap();
    let id_padded = TaskId::new("  CVH-001  ").unwrap();

    assert_eq!(id_lower, id_upper);
    assert_eq!(id_upper, id_padded);

    // &str and String comparisons
    assert_eq!(id_lower, "CVH-001");
    assert_eq!("CVH-001", id_lower);
    assert_eq!(id_lower, String::from("CVH-001"));
    assert_eq!(String::from("CVH-001"), id_lower);

    let id_diff = TaskId::new("CVH-002").unwrap();
    assert_ne!(id_lower, id_diff);
}

#[test]
fn test_task_id_hashset_and_hashmap() {
    let mut set = HashSet::new();
    set.insert(TaskId::new("cvh-001").unwrap());
    // Adding equivalent ID with different casing/spacing must not increase set size
    set.insert(TaskId::new("  CVH-001  ").unwrap());
    assert_eq!(set.len(), 1);

    // Lookup with TaskId
    assert!(set.contains(&TaskId::new("CVH-001").unwrap()));
    // Lookup with &str via Borrow<str>
    assert!(set.contains("CVH-001"));

    let mut map = HashMap::new();
    map.insert(TaskId::new("cvh-042").unwrap(), "Agent Implementation");
    assert_eq!(map.get(&TaskId::new("CVH-042").unwrap()), Some(&"Agent Implementation"));
    assert_eq!(map.get("CVH-042"), Some(&"Agent Implementation"));
}

#[test]
fn test_task_id_serde_json() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Wrapper {
        id: TaskId,
    }

    let item = Wrapper {
        id: TaskId::new("cvh-005").unwrap(),
    };

    // Serialize
    let json = serde_json::to_string(&item).expect("serialize");
    assert_eq!(json, r#"{"id":"CVH-005"}"#);

    // Deserialize valid
    let deserialized: Wrapper = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(deserialized, item);

    // Deserialize with whitespace & lowercase
    let padded_json = r#"{"id":"   cvh-005   "}"#;
    let from_padded: Wrapper = serde_json::from_str(padded_json).expect("deserialize padded");
    assert_eq!(from_padded.id.as_str(), "CVH-005");

    // Deserialize empty -> should fail
    let empty_json = r#"{"id":""}"#;
    assert!(serde_json::from_str::<Wrapper>(empty_json).is_err());

    // Deserialize invalid chars -> should fail
    let invalid_json = r#"{"id":"bad id with spaces"}"#;
    assert!(serde_json::from_str::<Wrapper>(invalid_json).is_err());
}

// ============================================================================
// StageId Tests
// ============================================================================

#[test]
fn test_stage_id_valid_normalization() {
    let s1 = StageId::new("  Dev  ").expect("valid stage id");
    assert_eq!(s1.as_str(), "dev");
    assert_eq!(&*s1, "dev"); // Deref
    assert_eq!(format!("{}", s1), "dev"); // Display

    let s2 = StageId::new("BACKLOG").expect("valid stage id");
    assert_eq!(s2.as_str(), "backlog");

    let s3 = StageId::new("code-review_stage").expect("valid stage id");
    assert_eq!(s3.as_str(), "code-review_stage");

    let s4 = StageId::new("ожидание").expect("unicode stage id");
    assert_eq!(s4.as_str(), "ожидание");
}

#[test]
fn test_stage_id_empty_rejection() {
    assert_eq!(StageId::new(""), Err(DomainError::EmptyStageId));
    assert_eq!(StageId::new("   "), Err(DomainError::EmptyStageId));
    assert_eq!(StageId::new("
	"), Err(DomainError::EmptyStageId));
}

#[test]
fn test_stage_id_invalid_format_rejection() {
    // Spaces inside
    assert!(matches!(StageId::new("dev stage"), Err(DomainError::InvalidStageIdFormat(_))));
    // Slash
    assert!(matches!(StageId::new("dev/column"), Err(DomainError::InvalidStageIdFormat(_))));
    // Special symbols
    assert!(matches!(StageId::new("dev!"), Err(DomainError::InvalidStageIdFormat(_))));
    // Length > 64
    let too_long = "a".repeat(65);
    assert!(matches!(StageId::new(too_long), Err(DomainError::InvalidStageIdFormat(_))));
}

#[test]
fn test_stage_id_helpers() {
    let backlog = StageId::new("backlog").unwrap();
    assert!(backlog.is_intake());
    assert!(!backlog.is_terminal());

    let waiting = StageId::new("waiting").unwrap();
    assert!(waiting.is_intake());

    let ready = StageId::new("ready").unwrap();
    assert!(ready.is_terminal());
    assert!(!ready.is_intake());

    let done = StageId::new("done").unwrap();
    assert!(done.is_terminal());

    let dev = StageId::new("dev").unwrap();
    assert!(!dev.is_intake());
    assert!(!dev.is_terminal());
}

#[test]
fn test_stage_id_hashset_and_hashmap() {
    let mut set = HashSet::new();
    set.insert(StageId::new("Dev").unwrap());
    set.insert(StageId::new("  dev  ").unwrap());
    assert_eq!(set.len(), 1);

    assert!(set.contains(&StageId::new("dev").unwrap()));
    assert!(set.contains("dev")); // Borrow<str>

    let mut map = HashMap::new();
    map.insert(StageId::new("review").unwrap(), 2);
    assert_eq!(map.get(&StageId::new("REVIEW").unwrap()), Some(&2));
    assert_eq!(map.get("review"), Some(&2));
}

#[test]
fn test_stage_id_serde_json() {
    let stage = StageId::new("  Review  ").unwrap();
    let json = serde_json::to_string(&stage).expect("serialize");
    assert_eq!(json, r#""review""#);

    let parsed: StageId = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed, StageId::new("review").unwrap());

    assert!(serde_json::from_str::<StageId>(r#""""#).is_err());
    assert!(serde_json::from_str::<StageId>(r#""invalid stage""#).is_err());
}

// ============================================================================
// BoardId Tests
// ============================================================================

#[test]
fn test_board_id_valid_normalization() {
    let b1 = BoardId::new("  Development  ").expect("valid board id");
    assert_eq!(b1.as_str(), "development");
    assert_eq!(&*b1, "development"); // Deref
    assert_eq!(format!("{}", b1), "development"); // Display
    assert!(b1.is_default());

    let b2 = BoardId::new("KANBAN").expect("valid board id");
    assert_eq!(b2.as_str(), "kanban");
    assert!(!b2.is_default());

    let b_default = BoardId::default();
    assert_eq!(b_default.as_str(), "development");
    assert!(b_default.is_default());
}

#[test]
fn test_board_id_empty_rejection() {
    assert_eq!(BoardId::new(""), Err(DomainError::EmptyBoardId));
    assert_eq!(BoardId::new("   "), Err(DomainError::EmptyBoardId));
}

#[test]
fn test_board_id_invalid_format_rejection() {
    assert!(matches!(BoardId::new("board with spaces"), Err(DomainError::InvalidBoardIdFormat(_))));
    assert!(matches!(BoardId::new("board/1"), Err(DomainError::InvalidBoardIdFormat(_))));
    let too_long = "b".repeat(65);
    assert!(matches!(BoardId::new(too_long), Err(DomainError::InvalidBoardIdFormat(_))));
}

#[test]
fn test_board_id_hashset_and_hashmap() {
    let mut set = HashSet::new();
    set.insert(BoardId::new("Kanban").unwrap());
    set.insert(BoardId::new("  kanban  ").unwrap());
    assert_eq!(set.len(), 1);

    assert!(set.contains(&BoardId::new("kanban").unwrap()));
    assert!(set.contains("kanban")); // Borrow<str>

    let mut map = HashMap::new();
    map.insert(BoardId::new("development").unwrap(), "Active Main Board");
    assert_eq!(map.get(&BoardId::new("DEVELOPMENT").unwrap()), Some(&"Active Main Board"));
    assert_eq!(map.get("development"), Some(&"Active Main Board"));
}

#[test]
fn test_board_id_serde_json() {
    let board = BoardId::new("  Modeler  ").unwrap();
    let json = serde_json::to_string(&board).expect("serialize");
    assert_eq!(json, r#""modeler""#);

    let parsed: BoardId = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed, BoardId::new("modeler").unwrap());

    assert!(serde_json::from_str::<BoardId>(r#""""#).is_err());
    assert!(serde_json::from_str::<BoardId>(r#""invalid board""#).is_err());
}

use super::*;
use crate::agent::Task;
use std::path::PathBuf;

    #[test]
    fn test_next_task_num_never_reuses_ids() {
        // Gap after deletion: len is 2 but max suffix is 3 -> next must be 4,
        // never CVH-003 again (stale dialog history) and never len+1=3.
        let tasks: Vec<Task> = serde_json::from_value(serde_json::json!([
            {"id": "CVH-001", "title": "a", "stageId": "backlog"},
            {"id": "CVH-003", "title": "b", "stageId": "backlog"},
            {"id": "CVH-003.2", "title": "sub", "stageId": "dev", "parentId": "CVH-003", "subtaskIndex": 2}
        ]))
        .expect("deserialize tasks");
        assert_eq!(next_task_num(&tasks), 4);

        // Empty disk -> first id.
        let empty: Vec<Task> = vec![];
        assert_eq!(next_task_num(&empty), 1);

        // Subtask-only suffixes must not inflate the top-level counter.
        let subs: Vec<Task> = serde_json::from_value(serde_json::json!([
            {"id": "CVH-002", "title": "a", "stageId": "backlog"},
            {"id": "CVH-002.9", "title": "sub", "stageId": "dev", "parentId": "CVH-002", "subtaskIndex": 9}
        ]))
        .expect("deserialize tasks");
        assert_eq!(next_task_num(&subs), 3);
    }

    #[tokio::test]
    async fn test_persistent_monotonic_sequence_prevent_deleted_id_reuse() {
        let tmp = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_seq_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;

        let mut tasks: Vec<Task> = serde_json::from_value(serde_json::json!([
            {"id": "CVH-001", "title": "t1", "stageId": "backlog"},
            {"id": "CVH-002", "title": "t2", "stageId": "backlog"},
            {"id": "CVH-003", "title": "t3", "stageId": "backlog"}
        ])).unwrap();

        // 1. Initial allocation on existing tasks -> must be CVH-004
        let num1 = allocate_next_task_num(&tmp, &tasks).await;
        assert_eq!(num1, 4);

        // Verify sequence was persisted to disk in .cvh/settings/sequence.json
        let seq_file = tmp.join(".cvh").join("settings").join("sequence.json");
        assert!(seq_file.is_file());
        let seq_on_disk = load_task_sequence(&tmp).await;
        assert_eq!(seq_on_disk, 4);

        // 2. User deletes CVH-004 and CVH-003 from tasks
        tasks.retain(|t| t.id != "CVH-003");
        // tasks now only has CVH-001 and CVH-002 (max_on_disk = 2).
        // BUT sequence counter on disk is 4!
        // Next allocation MUST NOT re-issue CVH-003 or CVH-004! It must be CVH-005.
        let num2 = allocate_next_task_num(&tmp, &tasks).await;
        assert_eq!(num2, 5, "Deleted task IDs (CVH-003, CVH-004) must NEVER be reissued!");

        // 3. User deletes ALL tasks (empty tasks list)
        tasks.clear();
        // Even when all tasks are gone, the sequence counter must increment monotonically
        let num3 = allocate_next_task_num(&tmp, &tasks).await;
        assert_eq!(num3, 6, "Empty tasks list must continue monotonically from stored sequence!");

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }

    #[test]
    fn test_sanitize_task_id() {
        assert!(sanitize_task_id("CVH-001").is_ok());
        assert!(sanitize_task_id("task-123").is_ok());
        assert!(sanitize_task_id("feature_test").is_ok());

        assert!(sanitize_task_id("").is_err());
        assert!(sanitize_task_id("   ").is_err());
        assert!(sanitize_task_id("../escape").is_err());
        assert!(sanitize_task_id("a/b").is_err());
        assert!(sanitize_task_id("a\\b").is_err());
        assert!(sanitize_task_id("task:1").is_err());
        assert!(sanitize_task_id("task*1").is_err());
        assert!(sanitize_task_id("task?1").is_err());
    }

    #[test]
    fn test_task_id_matches_case_and_whitespace() {
        assert!(task_id_matches("CVH-001", "cvh-001"));
        assert!(task_id_matches("cvh-001", "CVH-001"));
        assert!(task_id_matches("  CVH-001  ", "cvh-001"));
        assert!(task_id_matches("CVH-001", "  cvh-001  "));
        assert!(task_id_matches("CVH-042.1", "cvh-042.1"));
        assert!(!task_id_matches("CVH-001", "CVH-002"));
    }

    #[tokio::test]
    async fn test_create_task_with_explicit_task_id_and_sequence_sync() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_explicit_id_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;

        // Scaffold minimal .cvh settings with sequence = 1
        save_task_sequence(&tmp, 1).await.expect("save initial sequence");
        assert_eq!(load_task_sequence(&tmp).await, 1);

        // Step 1: Create task with explicit task_id "CVH-005" (as frontend optimisticId)
        let created = internal_create_task(
            &tmp,
            "Optimistic Task".to_string(),
            "Created with frontend optimistic ID".to_string(),
            "backlog".to_string(),
            Some("development".to_string()),
            Some(true), // skip_worktree for unit test speed
            None,
            Some("CVH-005".to_string()),
            None,
            None,
            None,
        )
        .await
        .expect("internal_create_task must succeed with explicit task_id");

        // Verify task used the exact explicit ID
        assert_eq!(created.id, "CVH-005");

        // Verify sequence on disk was synchronized to 5
        let seq_after_explicit = load_task_sequence(&tmp).await;
        assert_eq!(
            seq_after_explicit, 5,
            "Sequence on disk must be synchronized to the explicit task number (5)!"
        );

        // Step 2: Create next task without task_id (task_id: None)
        let created_next = internal_create_task(
            &tmp,
            "Subsequent Task".to_string(),
            "Created without explicit ID".to_string(),
            "backlog".to_string(),
            Some("development".to_string()),
            Some(true),
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("internal_create_task without task_id must allocate next monotonic ID");

        // Must allocate monotonically after 5 -> CVH-006!
        assert_eq!(created_next.id, "CVH-006");
        let seq_after_next = load_task_sequence(&tmp).await;
        assert_eq!(seq_after_next, 6);

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }

    #[tokio::test]
    async fn test_create_task_model_and_task_id_order_regression() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_model_order_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;

        save_task_sequence(&tmp, 1).await.expect("save initial sequence");

        let task = internal_create_task(
            &tmp,
            "Regression Test Task".to_string(),
            "Assert model and task_id parameters do not get swapped".to_string(),
            "backlog".to_string(),
            Some("development".to_string()),
            Some(true),
            Some("gemini-3.8-flash-high".to_string()),
            None,
            None,
            None,
            None,
        )
        .await
        .expect("create task with model and no task_id must succeed");

        assert!(
            task.id.starts_with("CVH-"),
            "Task ID must start with 'CVH-', but was '{}'",
            task.id
        );
        assert_ne!(
            task.id, "gemini-3.8-flash-high",
            "Task ID must NOT be the model name 'gemini-3.8-flash-high'"
        );
        assert_eq!(
            task.model,
            Some("gemini-3.8-flash-high".to_string()),
            "Task model must be preserved as Some('gemini-3.8-flash-high')"
        );

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }

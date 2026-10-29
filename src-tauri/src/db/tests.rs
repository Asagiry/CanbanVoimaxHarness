use crate::agent::{Task, TodoItem};
use super::*;
use std::path::PathBuf;

#[test]
fn test_sqlite_dialog_roundtrip() {
    let root = PathBuf::from("target").join(format!("cvh_db_test_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&root);
    let root = root.as_path();

    let items = vec![
        ActivityItem {
            id: "user-100".to_string(),
            item_type: "user".to_string(),
            timestamp: "10:00:00".to_string(),
            content: Some("User prompt".to_string()),
            tool: None,
            duration_ms: None,
            images: Some(vec!["img1.png".to_string()]),
        },
        ActivityItem {
            id: "th-1710000000000-abcd".to_string(),
            item_type: "thought".to_string(),
            timestamp: "10:00:01".to_string(),
            content: Some("Thinking...".to_string()),
            tool: None,
            duration_ms: Some(500.0),
            images: None,
        },
        ActivityItem {
            id: "call-1".to_string(),
            item_type: "tool".to_string(),
            timestamp: "10:00:02".to_string(),
            content: None,
            tool: Some(serde_json::json!({
                "id": "call-1",
                "toolName": "read",
                "status": "success"
            })),
            duration_ms: Some(42.0),
            images: None,
        },
    ];

    append_dialog_items(root, "CVH-TEST-1", &items).expect("append");

    let loaded = load_dialog_items(root, "CVH-TEST-1").expect("load");
    assert_eq!(loaded.len(), 3);
    assert_eq!(loaded[0].id, "user-100");
    assert_eq!(loaded[0].images, Some(vec!["img1.png".to_string()]));
    assert_eq!(loaded[1].duration_ms, Some(500.0));
    assert_eq!(loaded[2].item_type, "tool");
    assert!(loaded[2].tool.is_some());

    // Update streaming content
    let mut updated = items.clone();
    updated[1].content = Some("Thinking deeper...".to_string());
    append_dialog_items(root, "CVH-TEST-1", &updated).expect("update");

    let loaded_updated = load_dialog_items(root, "CVH-TEST-1").expect("load updated");
    assert_eq!(loaded_updated.len(), 3);
    assert_eq!(loaded_updated[1].content.as_deref(), Some("Thinking deeper..."));

    // Clear
    clear_dialog_items(root, "CVH-TEST-1").expect("clear");
    let empty = load_dialog_items(root, "CVH-TEST-1").expect("load after clear");
    assert!(empty.is_empty());

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_tasks_crud_and_todos() {
    let root = PathBuf::from("target").join(format!("cvh_db_test_tasks_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&root);
    let root = root.as_path();

    let task1 = Task {
        id: "CVH-010".to_string(),
        title: "Task 10".to_string(),
        description: "Initial description".to_string(),
        stage_id: "dev".to_string(),
        board_id: Some("development".to_string()),
        worktree_path: Some("worktrees/cvh-010".to_string()),
        branch: Some("feat/cvh-010".to_string()),
        verdict: None,
        custom_flow: None,
        status: Some("idle".to_string()),
        model: Some("gemini-3.8-flash-high".to_string()),
        parent_id: None,
        subtask_index: None,
        waiting_for_subtask_id: None,
        handover_summary: None,
        cover_image: None,
        attachments: None,
        todos: Some(vec![
            TodoItem {
                id: "td-1".to_string(),
                content: "Todo 1".to_string(),
                status: "pending".to_string(),
            },
            TodoItem {
                id: "td-2".to_string(),
                content: "Todo 2".to_string(),
                status: "completed".to_string(),
            },
        ]),
        priority: Some("high".to_string()),
        tags: Some(vec!["backend".to_string(), "sqlite".to_string()]),
        current_tokens: Some(1500),
        skip_worktree: Some(false),
        created_at: 1000,
        updated_at: 1000,
        stage_entered_at: Some(1000),
        completed_at: Some(1500),
        position: Some(0.0),
        hook_states: Some(serde_json::json!({
            "on_enter": {
                "hookName": "check.bat",
                "stageId": "dev",
                "hookType": "on_enter",
                "status": "success",
                "startedAt": 1000,
                "durationMs": 150,
                "exitCode": 0,
                "stdout": "OK",
                "stderr": "",
                "error": null
            }
        })),
    };

    db_upsert_task(root, &task1).expect("upsert task1");

    let loaded = db_get_task(root, "CVH-010").expect("get task").expect("task found");
    assert_eq!(loaded.id, "CVH-010");
    assert_eq!(loaded.title, "Task 10");
    assert_eq!(loaded.stage_entered_at, Some(1000));
    assert_eq!(loaded.tags, Some(vec!["backend".to_string(), "sqlite".to_string()]));
    assert_eq!(loaded.current_tokens, Some(1500));
    assert!(loaded.hook_states.is_some());
    assert_eq!(
        loaded.hook_states.as_ref().unwrap()["on_enter"]["status"],
        "success"
    );
    let todos = loaded.todos.expect("todos present");
    assert_eq!(todos.len(), 2);
    assert_eq!(todos[0].id, "td-1");
    assert_eq!(todos[1].status, "completed");

    // Save todos
    let new_todos = vec![
        TodoItem {
            id: "td-1".to_string(),
            content: "Todo 1 updated".to_string(),
            status: "completed".to_string(),
        },
        TodoItem {
            id: "td-3".to_string(),
            content: "Todo 3 new".to_string(),
            status: "in_progress".to_string(),
        },
    ];
    db_save_todos(root, "CVH-010", &new_todos).expect("save todos");

    let loaded2 = db_get_task(root, "CVH-010").expect("get task 2").expect("task found");
    let todos2 = loaded2.todos.expect("todos2 present");
    assert_eq!(todos2.len(), 2);
    assert_eq!(todos2[0].content, "Todo 1 updated");
    assert_eq!(todos2[1].status, "in_progress");

    // Subtask with foreign key
    let subtask = Task {
        id: "CVH-010.1".to_string(),
        title: "Subtask 1".to_string(),
        description: "".to_string(),
        stage_id: "review".to_string(),
        board_id: Some("development".to_string()),
        worktree_path: None,
        branch: None,
        verdict: None,
        custom_flow: None,
        status: Some("idle".to_string()),
        model: None,
        parent_id: Some("CVH-010".to_string()),
        subtask_index: Some(1),
        waiting_for_subtask_id: None,
        handover_summary: None,
        cover_image: None,
        attachments: None,
        todos: None,
        priority: None,
        tags: None,
        current_tokens: None,
        skip_worktree: Some(false),
        created_at: 1010,
        updated_at: 1010,
        stage_entered_at: Some(1010),
        completed_at: None,
        position: Some(1.0),
        hook_states: None,
    };
    db_upsert_task(root, &subtask).expect("upsert subtask");

    let all_tasks = db_get_all_tasks(root).expect("get all tasks");
    assert_eq!(all_tasks.len(), 2);
    assert_eq!(all_tasks[0].completed_at, Some(1500));
    assert_eq!(all_tasks[1].completed_at, None);

    // Delete parent task -> cascade deletes subtask
    db_delete_task(root, "CVH-010").expect("delete parent");
    let all_after_delete = db_get_all_tasks(root).expect("get all after delete");
    assert_eq!(all_after_delete.len(), 0);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_sequence_allocation() {
    let root = PathBuf::from("target").join(format!("cvh_db_test_seq_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&root);
    let root = root.as_path();

    let num1 = db_allocate_next_task_num(root).expect("alloc 1");
    assert_eq!(num1, 1);

    let num2 = db_allocate_next_task_num(root).expect("alloc 2");
    assert_eq!(num2, 2);

    // Add task with higher number
    let high_task = Task {
        id: "CVH-007".to_string(),
        title: "Task 7".to_string(),
        description: "".to_string(),
        stage_id: "dev".to_string(),
        board_id: Some("development".to_string()),
        worktree_path: None,
        branch: None,
        verdict: None,
        custom_flow: None,
        status: None,
        model: None,
        parent_id: None,
        subtask_index: None,
        waiting_for_subtask_id: None,
        handover_summary: None,
        cover_image: None,
        attachments: None,
        todos: None,
        priority: None,
        tags: None,
        current_tokens: None,
        skip_worktree: None,
        created_at: 1000,
        updated_at: 1000,
        stage_entered_at: Some(1000),
        completed_at: None,
        position: None,
        hook_states: None,
    };
    db_upsert_task(root, &high_task).expect("upsert high task");

    let num3 = db_allocate_next_task_num(root).expect("alloc 3");
    assert_eq!(num3, 8);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_telemetry_and_ledger_persistence() {
    let root = PathBuf::from("target").join(format!("cvh_db_test_telem_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&root);
    let root = root.as_path();

    let telem = TelemetryRecord {
        id: "telem-1".to_string(),
        task_id: "CVH-042".to_string(),
        model: Some("gemini-3.8-flash-high".to_string()),
        input_tokens: 1200,
        output_tokens: 450,
        cached_tokens: 800,
        tps: 84.5,
        ttft_ms: 120.0,
        duration_ms: 5320.0,
        created_at: 2000,
    };
    db_record_telemetry(root, &telem).expect("record telemetry");

    let loaded_telem = db_get_telemetry_for_task(root, "CVH-042").expect("get telemetry");
    assert_eq!(loaded_telem.len(), 1);
    assert_eq!(loaded_telem[0].id, "telem-1");
    assert_eq!(loaded_telem[0].input_tokens, 1200);
    assert_eq!(loaded_telem[0].tps, 84.5);

    let ledger = LedgerEntry {
        id: "led-1".to_string(),
        task_id: "CVH-042".to_string(),
        commit_hash: Some("abcdef1".to_string()),
        summary: "Implemented SQLite tasks storage".to_string(),
        verdict: Some("APPROVED".to_string()),
        author: Some("agent-developer".to_string()),
        files: Some(vec!["src-tauri/src/db.rs".to_string()]),
        created_at: 2100,
    };
    db_record_ledger_entry(root, &ledger).expect("record ledger");

    let loaded_ledger = db_get_ledger_for_task(root, "CVH-042").expect("get ledger");
    assert_eq!(loaded_ledger.len(), 1);
    assert_eq!(loaded_ledger[0].summary, "Implemented SQLite tasks storage");
    assert_eq!(loaded_ledger[0].files, Some(vec!["src-tauri/src/db.rs".to_string()]));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_auto_migration_from_json() {
    let root = PathBuf::from("target").join(format!("cvh_db_test_mig_{}", uuid::Uuid::new_v4()));
    let cvh_dir = root.join(".cvh");
    let settings_dir = cvh_dir.join("settings");
    let _ = std::fs::create_dir_all(&settings_dir);
    let root = root.as_path();

    // 1. Seed tasks.json
    let sample_tasks = serde_json::json!([
        {
            "id": "CVH-001",
            "title": "Migrated Task",
            "description": "From JSON",
            "stageId": "dev",
            "boardId": "development",
            "todos": [
                { "id": "td-m1", "content": "Migrate check", "status": "completed" }
            ],
            "createdAt": 100,
            "updatedAt": 200
        }
    ]);
    std::fs::write(cvh_dir.join("tasks.json"), sample_tasks.to_string()).expect("write tasks.json");

    // 2. Seed sequence.json
    let sample_seq = serde_json::json!({ "last_task_num": 1 });
    std::fs::write(settings_dir.join("sequence.json"), sample_seq.to_string()).expect("write sequence.json");

    // 3. Open DB -> should automatically migrate
    let conn = get_db_conn(root).expect("init_db");
    drop(conn);

    let tasks = db_get_all_tasks(root).expect("get migrated tasks");
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, "CVH-001");
    assert_eq!(tasks[0].title, "Migrated Task");
    let todos = tasks[0].todos.as_ref().expect("migrated todos");
    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0].id, "td-m1");

    let next_seq = db_allocate_next_task_num(root).expect("next seq");
    assert_eq!(next_seq, 2);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_db_connection_pool_concurrent_access() {
    let root = PathBuf::from("target").join(format!("cvh_db_test_pool_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&root);
    let root_path = root.as_path();

    let pool = get_db_pool(root_path).expect("get_db_pool");
    assert_eq!(pool.max_size(), 16);

    // Verify multiple pooled connections can be checked out and execute queries concurrently
    let conn1 = get_db_conn(root_path).expect("conn 1");
    let conn2 = get_db_conn(root_path).expect("conn 2");

    let val1: i64 = conn1.query_row("SELECT 1", [], |r| r.get(0)).expect("query 1");
    let val2: i64 = conn2.query_row("SELECT 2", [], |r| r.get(0)).expect("query 2");

    assert_eq!(val1, 1);
    assert_eq!(val2, 2);

    drop(conn1);
    drop(conn2);

    let _ = std::fs::remove_dir_all(root);
}

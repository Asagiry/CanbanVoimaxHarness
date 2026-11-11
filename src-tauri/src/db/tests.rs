use crate::agent::{Task, TodoItem};
use super::*;
use std::path::PathBuf;

#[tokio::test]
async fn retry_discards_only_partial_attempt_and_preserves_cached_context() {
    let root = PathBuf::from("target").join(format!("cvh_retry_{}", uuid::Uuid::new_v4()));
    let task = "RETRY-1";
    let previous = ActivityItem { id: "previous".into(), item_type: "output".into(), content: Some("same text".into()), ..Default::default() };
    append_dialog_items(&root, task, std::slice::from_ref(&previous)).unwrap();
    let messages = vec![crate::llm::ChatMessage::user("preserve prefix")];
    let abort = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    save_conversation_context(&root, task, 4, &messages, context_dialog_cursor(&root, task).unwrap(), &abort).await.unwrap();
    let partial = ActivityItem { id: "attempt:output".into(), ..previous.clone() };
    append_dialog_items(&root, task, std::slice::from_ref(&partial)).unwrap();
    assert_eq!(load_dialog_items(&root, task).unwrap().len(), 2);
    discard_stream_attempt(&root, task, "attempt").unwrap();
    append_dialog_items(&root, task, &[partial, ActivityItem {
        id: "attempt:thought".into(), item_type: "thought".into(), content: Some("late thought".into()), ..Default::default()
    }]).unwrap();
    let remaining = load_dialog_items(&root, task).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, previous.id);
    let restored = load_conversation_context(&root, task).unwrap().unwrap();
    assert_eq!(restored.turn, 4);
    assert_eq!(restored.messages, messages);
}

#[tokio::test]
async fn deleted_dialog_items_stay_out_of_rebuilt_context_after_stale_save() {
    let root = PathBuf::from("target").join(format!("cvh_delete_dialog_{}", uuid::Uuid::new_v4()));
    let task = "DELETE-1";
    let items = vec![
        ActivityItem { id: "remove-output".into(), item_type: "output".into(), content: Some("remove answer".into()), ..Default::default() },
        ActivityItem { id: "remove-thought".into(), item_type: "thought".into(), content: Some("remove thinking".into()), ..Default::default() },
        ActivityItem { id: "remove-tool".into(), item_type: "tool".into(), tool: Some(serde_json::json!({"id":"remove-tool", "toolName":"read_file", "stdout":"remove result"})), ..Default::default() },
        ActivityItem { id: "keep-tool".into(), item_type: "tool".into(), tool: Some(serde_json::json!({"id":"keep-tool", "toolName":"read_file", "stdout":"keep result"})), ..Default::default() },
        ActivityItem { id: "keep-output".into(), item_type: "output".into(), content: Some("keep answer".into()), ..Default::default() },
        ActivityItem { id: "legacy-error".into(), item_type: "output".into(), content: Some("\n[LLM Error: HTTP 400]\n".into()), ..Default::default() },
    ];
    append_dialog_items(&root, task, &items).unwrap();
    let messages = crate::agent::reconstruct_messages_from_dialog("system", "objective", &items);
    let abort = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    save_conversation_context(&root, task, 3, &messages, context_dialog_cursor(&root, task).unwrap(), &abort).await.unwrap();
    for id in ["remove-output", "remove-thought", "remove-tool"] { delete_dialog_item(&root, task, id).unwrap(); }
    assert!(load_conversation_context(&root, task).unwrap().is_none());
    append_dialog_items(&root, task, &items).unwrap();
    let remaining = load_dialog_items(&root, task).unwrap();
    assert_eq!(remaining.len(), 3);
    assert!(remaining.iter().all(|item| !item.id.starts_with("remove-")));
    assert_eq!(remaining.iter().find(|item| item.id == "legacy-error").unwrap().item_type, "error");
    let rebuilt = crate::agent::reconstruct_messages_from_dialog("system", "objective", &remaining);
    let json = serde_json::to_string(&rebuilt).unwrap();
    assert!(!json.contains("remove"));
    assert!(!json.contains("LLM Error"));
    assert!(json.contains("keep answer"));
    assert!(json.contains("keep result"));
    let before_sanitize = rebuilt.clone();
    let mut sanitized = rebuilt;
    crate::agent::sanitize_unclosed_tool_calls(&mut sanitized);
    assert_eq!(sanitized, before_sanitize);
}

#[test]
fn deleted_dialog_item_cannot_be_created_by_a_late_save() {
    let root = PathBuf::from("target").join(format!("cvh_delete_late_{}", uuid::Uuid::new_v4()));
    delete_dialog_item(&root, "DELETE-2", "late-output").unwrap();
    append_dialog_items(&root, "DELETE-2", &[ActivityItem {
        id: "late-output".into(), item_type: "output".into(), content: Some("late".into()), ..Default::default()
    }]).unwrap();
    assert!(load_dialog_items(&root, "DELETE-2").unwrap().is_empty());
}

#[test]
fn output_deduplication_keeps_the_visible_message_deletable() {
    let root = PathBuf::from("target").join(format!("cvh_delete_dedup_{}", uuid::Uuid::new_v4()));
    let ui = ActivityItem { id: "ui-output".into(), item_type: "output".into(), content: Some("answer".into()), ..Default::default() };
    let server = ActivityItem { id: "server-output".into(), ..ui.clone() };
    append_dialog_items(&root, "DELETE-3", std::slice::from_ref(&ui)).unwrap();
    append_dialog_items(&root, "DELETE-3", &[server]).unwrap();
    append_dialog_items(&root, "DELETE-3", std::slice::from_ref(&ui)).unwrap();
    let loaded = load_dialog_items(&root, "DELETE-3").unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, ui.id);
    delete_dialog_item(&root, "DELETE-3", &ui.id).unwrap();
    append_dialog_items(&root, "DELETE-3", &[ui]).unwrap();
    assert!(load_dialog_items(&root, "DELETE-3").unwrap().is_empty());
}

#[test]
fn deleting_question_removes_its_tool_record_without_touching_other_calls() {
    let root = PathBuf::from("target").join(format!("cvh_delete_question_{}", uuid::Uuid::new_v4()));
    let items = vec![
        ActivityItem { id: "q-question-id".into(), item_type: "question".into(), tool: Some(serde_json::json!({"id":"question-id", "toolName":"question", "params":{"callId":"call-id"}})), ..Default::default() },
        ActivityItem { id: "call-id".into(), item_type: "tool".into(), tool: Some(serde_json::json!({"id":"call-id", "toolName":"question", "stdout":"answer"})), ..Default::default() },
        ActivityItem { id: "keep".into(), item_type: "tool".into(), tool: Some(serde_json::json!({"id":"keep", "toolName":"question", "stdout":"answer"})), ..Default::default() },
    ];
    append_dialog_items(&root, "DELETE-4", &items).unwrap();
    let deleted = delete_dialog_item(&root, "DELETE-4", "q-question-id").unwrap();
    assert!(deleted.iter().any(|id| id == "call-id"));
    append_dialog_items(&root, "DELETE-4", &items).unwrap();
    let remaining = load_dialog_items(&root, "DELETE-4").unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, "keep");
}

#[test]
fn deletion_resolves_subagent_alias_to_the_persisted_dialogue() {
    let root = PathBuf::from("target").join(format!("cvh_delete_alias_{}", uuid::Uuid::new_v4()));
    let item = ActivityItem { id: "answer-id".into(), item_type: "output".into(), content: Some("answer".into()), ..Default::default() };
    append_dialog_items(&root, "PARENT:sub:PARENT.sub-call", std::slice::from_ref(&item)).unwrap();
    let canonical = resolve_dialog_item_task_id(&root, "PARENT:sub:call", &item.id).unwrap();
    assert_eq!(canonical, "PARENT:sub:PARENT.sub-call");
    assert!(resolve_dialog_item_task_id(&root, "OTHER:sub:call", &item.id).is_err());
    append_dialog_items(&root, &canonical, std::slice::from_ref(&item)).unwrap();
    delete_dialog_item(&root, &canonical, &item.id).unwrap();
    assert!(load_dialog_items(&root, &canonical).unwrap().is_empty());
}

#[test]
fn compaction_cannot_restore_a_message_deleted_during_its_request() {
    let root = PathBuf::from("target").join(format!("cvh_delete_compact_{}", uuid::Uuid::new_v4()));
    let item = ActivityItem { id: "source".into(), item_type: "output".into(), content: Some("remove me".into()), ..Default::default() };
    append_dialog_items(&root, "DELETE-5", std::slice::from_ref(&item)).unwrap();
    let summary = ActivityItem { id: "summary".into(), item_type: "compacted".into(), content: item.content.clone(), ..Default::default() };
    delete_dialog_item(&root, "DELETE-5", &item.id).unwrap();
    assert!(persist_compacted_dialog(&root, "DELETE-5", &summary, None, &[item.id]).is_err());
    assert!(load_dialog_items(&root, "DELETE-5").unwrap().is_empty());
}

#[tokio::test]
async fn cache_snapshot_preserves_batch_messages_and_pending_steer() {
    use crate::llm::{CacheControl, ChatMessage, FunctionCall, ToolCall};
    let root = PathBuf::from("target").join(format!("cvh_cache_context_{}", uuid::Uuid::new_v4()));
    let abort = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let initial = ActivityItem {
        id: uuid::Uuid::new_v4().to_string(), item_type: "user".into(),
        content: Some("objective".into()), ..Default::default()
    };
    append_dialog_items(&root, "CACHE-1", &[initial]).unwrap();
    let cursor = context_dialog_cursor(&root, "CACHE-1").unwrap();
    let calls = ["a", "b"].into_iter().map(|id| ToolCall {
        id: id.into(), call_type: "function".into(),
        function: FunctionCall { name: "read_file".into(), arguments: "{ \"path\": \"x.rs\" }".into() },
    }).collect();
    let messages = vec![
        ChatMessage::system("rules").with_cache_control(CacheControl::ephemeral()),
        ChatMessage::user_with_images("objective", vec!["data:image/png;base64,AA==".into()]),
        ChatMessage::assistant(Some("  inspect both  ".into()), Some(calls)),
        ChatMessage::tool_response("a", "first\r\n"),
        ChatMessage::tool_response("b", "second\n"),
    ];
    save_conversation_context(&root, "CACHE-1", 7, &messages, cursor, &abort).await.unwrap();
    assert_eq!(load_dialog_items(&root, "CACHE-1").unwrap().len(), 1);
    let steer = ActivityItem {
        id: uuid::Uuid::new_v4().to_string(), item_type: "steer".into(),
        content: Some("continue with this constraint".into()), ..Default::default()
    };
    append_dialog_items(&root, "CACHE-1", &[steer]).unwrap();
    let mut saved = load_conversation_context(&root, "CACHE-1").unwrap().unwrap();
    assert_eq!(saved.turn, 7);
    assert_eq!(saved.messages, messages);
    let pending = load_dialog_items_after(&root, "CACHE-1", saved.dialog_cursor).unwrap();
    assert_eq!(pending.len(), 1);
    crate::agent::append_messages_from_dialog(&mut saved.messages, "objective", &pending);
    assert_eq!(&saved.messages[..messages.len()], messages.as_slice());
    assert!(saved.messages.last().unwrap().text_content().unwrap().contains("this constraint"));

    let compacted = ActivityItem {
        id: uuid::Uuid::new_v4().to_string(), item_type: "compacted".into(),
        content: Some("summary".into()), ..Default::default()
    };
    persist_compacted_dialog(&root, "CACHE-1", &compacted, None, &[]).unwrap();
    save_conversation_context(&root, "CACHE-1", 7, &messages, cursor, &abort).await.unwrap();
    assert!(load_conversation_context(&root, "CACHE-1").unwrap().is_none());
    let cursor = context_dialog_cursor(&root, "CACHE-1").unwrap();
    save_conversation_context(&root, "CACHE-1", 8, &saved.messages, cursor, &abort).await.unwrap();
    abort.store(true, std::sync::atomic::Ordering::SeqCst);
    save_conversation_context(&root, "CACHE-1", 7, &messages, cursor, &abort).await.unwrap();
    assert_eq!(load_conversation_context(&root, "CACHE-1").unwrap().unwrap().messages, saved.messages);
    clear_dialog_items(&root, "CACHE-1").unwrap();
    assert!(load_conversation_context(&root, "CACHE-1").unwrap().is_none());
}

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
        provider_id: None,
        reasoning_effort: Some("high".to_string()),
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
        project_path: None,
        stage_durations: Some(serde_json::json!({
            "plan": 65,
            "dev": 952
        })),
        total_elapsed: Some(1017),
    };

    db_upsert_task(root, &task1).expect("upsert task1");

    let loaded = db_get_task(root, "CVH-010").expect("get task").expect("task found");
    assert_eq!(loaded.id, "CVH-010");
    assert_eq!(loaded.title, "Task 10");
    assert_eq!(loaded.stage_entered_at, Some(1000));
    assert_eq!(loaded.tags, Some(vec!["backend".to_string(), "sqlite".to_string()]));
    assert_eq!(loaded.current_tokens, Some(1500));
    assert_eq!(loaded.reasoning_effort.as_deref(), Some("high"));
    assert_eq!(loaded.total_elapsed, Some(1017));
    assert_eq!(
        loaded.stage_durations.as_ref().unwrap()["dev"],
        952
    );
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
        provider_id: None,
            reasoning_effort: None,
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
        project_path: None,
        stage_durations: None,
        total_elapsed: None,
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
        provider_id: None,
            reasoning_effort: None,
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
        project_path: None,
        stage_durations: None,
        total_elapsed: None,
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
fn test_telemetry_summary_cumulative() {
    let root = PathBuf::from("target").join(format!("cvh_db_test_telem_sum_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&root);
    let root = root.as_path();

    let empty = db_get_telemetry_summary(root, "CVH-999").expect("empty summary");
    assert_eq!(empty.total_tokens, 0);
    assert_eq!(empty.turn_count, 0);
    assert_eq!(empty.cache_hit_rate, 0.0);

    for (id, task_id, input, cached, output) in [
        ("t1", "CVH-999", 1000, 900, 100),
        ("t2", "CVH-999", 2000, 1800, 200),
        ("t3", "CVH-999.sub-1", 500, 400, 50),
        ("t4", "CVH-OTHER", 9999, 9999, 9999),
    ] {
        db_record_telemetry(
            root,
            &TelemetryRecord {
                id: id.to_string(),
                task_id: task_id.to_string(),
                model: None,
                input_tokens: input,
                output_tokens: output,
                cached_tokens: cached,
                tps: 0.0,
                ttft_ms: 0.0,
                duration_ms: 0.0,
                created_at: 3000,
            },
        )
        .expect("record telemetry");
    }

    let summary = db_get_telemetry_summary(root, "CVH-999").expect("summary");
    assert_eq!(summary.total_input_tokens, 3500);
    assert_eq!(summary.total_cached_tokens, 3100);
    assert_eq!(summary.total_output_tokens, 350);
    assert_eq!(summary.total_tokens, 3850);
    assert_eq!(summary.turn_count, 3);
    assert!((summary.cache_hit_rate - (3100.0 / 3500.0 * 100.0)).abs() < 0.001);

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
fn handover_steer_dedup_on_repeated_insert() {
    let root = PathBuf::from("target").join(format!("cvh_handover_dedup_{}", uuid::Uuid::new_v4()));
    let task = "HANDOVER-1";
    let content = "### [ОТЧЕТ ОБ ИСПРАВЛЕНИЯХ / HANDOVER SUMMARY]\n\nЗамечания исправлены. Отчет исполнителя:\nSame summary";
    let first = ActivityItem { id: "handover-1".into(), item_type: "steer".into(), content: Some(content.into()), ..Default::default() };
    append_dialog_items(&root, task, std::slice::from_ref(&first)).unwrap();
    let second = ActivityItem { id: "handover-2".into(), item_type: "steer".into(), content: Some(content.into()), ..Default::default() };
    append_dialog_items(&root, task, std::slice::from_ref(&second)).unwrap();
    let items = load_dialog_items(&root, task).unwrap();
    assert_eq!(items.iter().filter(|i| i.item_type == "steer").count(), 1);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn empty_context_snapshot_returns_none_not_eof() {
    let root = PathBuf::from("target").join(format!("cvh_empty_snapshot_{}", uuid::Uuid::new_v4()));
    let task = "EMPTY-1";
    let conn = get_db_conn(&root).unwrap();
    conn.execute(
        "INSERT INTO dialog_items (id, task_id, type, content, created_at) VALUES (?1, ?2, 'context_snapshot', ?3, ?4)",
        rusqlite::params![format!("context:{task}"), task, "", chrono::Utc::now().timestamp_millis()],
    ).unwrap();
    drop(conn);
    assert!(load_conversation_context(&root, task).unwrap().is_none());
    let conn = get_db_conn(&root).unwrap();
    conn.execute(
        "UPDATE dialog_items SET content = ?1 WHERE task_id = ?2 AND type = 'context_snapshot'",
        rusqlite::params!["   ", task],
    ).unwrap();
    drop(conn);
    assert!(load_conversation_context(&root, task).unwrap().is_none());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn empty_tool_json_delete_succeeds() {
    let root = PathBuf::from("target").join(format!("cvh_empty_tool_{}", uuid::Uuid::new_v4()));
    let task = "EMPTY-2";
    let conn = get_db_conn(&root).unwrap();
    conn.execute(
        "INSERT INTO dialog_items (id, task_id, type, tool_json, created_at) VALUES (?1, ?2, 'tool', ?3, ?4)",
        rusqlite::params!["empty-tool", task, "", chrono::Utc::now().timestamp_millis()],
    ).unwrap();
    drop(conn);
    let deleted = delete_dialog_item(&root, task, "empty-tool").unwrap();
    assert_eq!(deleted, vec!["empty-tool".to_string()]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn test_db_connection_pool_concurrent_access() {
    let root = PathBuf::from("target").join(format!("cvh_db_test_pool_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&root);
    let root_path = root.as_path();

    let pool = get_db_pool(root_path).expect("get_db_pool");
    assert_eq!(pool.max_size(), 4);

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

#[tokio::test]
async fn test_isolated_context_snapshot_persistence_and_marker() {
    let root = PathBuf::from("target").join(format!("cvh_iso_ctx_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&root);
    let task_id = "CTX-TEST-1";

    let messages = vec![
        crate::llm::ChatMessage::user("Test prompt"),
        crate::llm::ChatMessage::assistant(Some("Test response".into()), None),
    ];
    let abort = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    // 1. Save context snapshot
    save_conversation_context(&root, task_id, 1, &messages, 0, &abort).await.expect("save context");

    // 2. Verify dedicated context file exists on disk
    let file_path = context_file_path(&root, task_id);
    assert!(file_path.is_file(), "Context file must exist at {:?}", file_path);
    let disk_content = std::fs::read_to_string(&file_path).expect("read context file");
    assert!(disk_content.contains("Test prompt"));
    assert!(disk_content.contains("Test response"));

    // 3. Verify dialog_items in SQLite stores only the lightweight marker 'file'
    let conn = get_db_conn(&root).expect("db conn");
    let marker: String = conn.query_row(
        "SELECT content FROM dialog_items WHERE task_id = ?1 AND type = 'context_snapshot'",
        rusqlite::params![task_id],
        |r| r.get(0),
    ).expect("query marker");
    assert_eq!(marker, "file", "dialog_items.content must be 'file', not multi-megabyte JSON");
    drop(conn);

    // 4. Verify load_conversation_context restores from the file
    let loaded = load_conversation_context(&root, task_id).expect("load context").expect("context present");
    assert_eq!(loaded.turn, 1);
    assert_eq!(loaded.messages.len(), 2);
    assert_eq!(loaded.messages[0].content, messages[0].content);

    // 5. Test backward compatibility: legacy snapshot stored in dialog_items.content
    let legacy_task_id = "CTX-LEGACY-2";
    let legacy_ctx = ConversationContext {
        turn: 5,
        messages: vec![crate::llm::ChatMessage::user("Legacy context in SQLite")],
        dialog_cursor: 10,
    };
    let legacy_json = serde_json::to_string(&legacy_ctx).expect("serialize legacy");
    let conn = get_db_conn(&root).expect("db conn");
    conn.execute(
        "INSERT INTO dialog_items (id, task_id, type, content, created_at) VALUES (?1, ?2, 'context_snapshot', ?3, ?4)",
        rusqlite::params![format!("context:{legacy_task_id}"), legacy_task_id, legacy_json, chrono::Utc::now().timestamp_millis()],
    ).expect("insert legacy snapshot");
    drop(conn);

    let loaded_legacy = load_conversation_context(&root, legacy_task_id)
        .expect("load legacy context")
        .expect("legacy context present");
    assert_eq!(loaded_legacy.turn, 5);
    assert_eq!(loaded_legacy.messages[0].content, legacy_ctx.messages[0].content);

    // 6. Test clear_dialog_items also purges context file
    clear_dialog_items(&root, task_id).expect("clear dialog items");
    assert!(!file_path.exists(), "Context file must be deleted when dialog items are cleared");
    assert!(load_conversation_context(&root, task_id).expect("load after clear").is_none());

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn test_todo_preservation_when_todos_none_or_empty() {
    let root = PathBuf::from("target").join(format!("cvh_todo_pres_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&root);
    let task_id = "TODO-TEST-1";

    let mut task = Task {
        id: task_id.to_string(),
        title: "Test Todo Preservation".to_string(),
        description: "Testing todo safety".to_string(),
        stage_id: "backlog".to_string(),
        board_id: Some("development".to_string()),
        worktree_path: None,
        branch: None,
        verdict: None,
        custom_flow: None,
        status: Some("idle".to_string()),
        model: None,
        provider_id: None,
        reasoning_effort: None,
        parent_id: None,
        subtask_index: None,
        waiting_for_subtask_id: None,
        handover_summary: None,
        cover_image: None,
        priority: None,
        current_tokens: None,
        skip_worktree: Some(false),
        created_at: 100,
        updated_at: 100,
        stage_entered_at: Some(100),
        completed_at: None,
        position: None,
        hook_states: None,
        stage_durations: None,
        total_elapsed: None,
        project_path: None,
        attachments: None,
        tags: None,
        todos: Some(vec![
            TodoItem { id: "td-1".to_string(), content: "Todo item 1".to_string(), status: "pending".to_string() },
            TodoItem { id: "td-2".to_string(), content: "Todo item 2".to_string(), status: "completed".to_string() },
        ]),
    };

    // 1. Initial upsert with 2 todos
    db_upsert_task(&root, &task).expect("initial upsert");
    let loaded = db_get_task(&root, task_id).expect("get task").expect("task exists");
    assert_eq!(loaded.todos.as_ref().map(|t| t.len()), Some(2));

    // 2. Upsert with todos: None -> MUST NOT wipe out existing todos
    task.todos = None;
    task.title = "Updated Title with None Todos".to_string();
    db_upsert_task(&root, &task).expect("upsert with None todos");

    let loaded_after_none = db_get_task(&root, task_id).expect("get task").expect("task exists");
    assert_eq!(loaded_after_none.title, "Updated Title with None Todos");
    assert_eq!(loaded_after_none.todos.as_ref().map(|t| t.len()), Some(2), "Existing todos must be preserved when todos is None");

    // 3. Upsert with todos: None -> continues to preserve existing todos
    task.todos = None;
    task.title = "Updated Title with None Todos Again".to_string();
    db_upsert_task(&root, &task).expect("upsert with none todos");

    let loaded_after_none_again = db_get_task(&root, task_id).expect("get task").expect("task exists");
    assert_eq!(loaded_after_none_again.title, "Updated Title with None Todos Again");
    assert_eq!(loaded_after_none_again.todos.as_ref().map(|t| t.len()), Some(2), "Existing todos must be preserved when todos is None");

    // 4. Upsert with explicit new todos -> replaces todos
    task.todos = Some(vec![
        TodoItem { id: "td-3".to_string(), content: "New single todo".to_string(), status: "pending".to_string() },
    ]);
    db_upsert_task(&root, &task).expect("upsert with explicit todos");

    let loaded_after_explicit = db_get_task(&root, task_id).expect("get task").expect("task exists");
    assert_eq!(loaded_after_explicit.todos.as_ref().map(|t| t.len()), Some(1));
    assert_eq!(loaded_after_explicit.todos.as_ref().unwrap()[0].id, "td-3");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn test_wal_checkpoint_db_and_all_dbs() {
    let root = PathBuf::from("target").join(format!("cvh_wal_chk_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&root);

    // Open connection and write data
    let conn = get_db_conn(&root).expect("db conn");
    conn.execute_batch("CREATE TABLE IF NOT EXISTS chk_test (id INTEGER PRIMARY KEY); INSERT INTO chk_test DEFAULT VALUES;").expect("write data");
    drop(conn);

    // Passive checkpoint
    checkpoint_db(&root, false).expect("passive checkpoint");

    // Truncate checkpoint
    checkpoint_db(&root, true).expect("truncate checkpoint");

    // Checkpoint all active DBs
    checkpoint_all_dbs(true);

    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn test_tasks_json_not_written_and_no_zombie_resurrection() {
    let root = PathBuf::from("target").join(format!("cvh_no_zombie_{}", uuid::Uuid::new_v4()));
    let _ = tokio::fs::create_dir_all(root.join(".cvh")).await;

    let task = Task {
        id: "LIVE-001".to_string(),
        title: "Live Task".to_string(),
        description: "Testing sole SSOT".to_string(),
        stage_id: "backlog".to_string(),
        board_id: Some("development".to_string()),
        worktree_path: None,
        branch: None,
        verdict: None,
        custom_flow: None,
        status: Some("idle".to_string()),
        model: None,
        provider_id: None,
        reasoning_effort: None,
        parent_id: None,
        subtask_index: None,
        waiting_for_subtask_id: None,
        handover_summary: None,
        cover_image: None,
        priority: None,
        current_tokens: None,
        skip_worktree: Some(false),
        created_at: 100,
        updated_at: 100,
        stage_entered_at: Some(100),
        completed_at: None,
        position: None,
        hook_states: None,
        stage_durations: None,
        total_elapsed: None,
        project_path: None,
        attachments: None,
        tags: None,
        todos: None,
    };

    // 1. save_tasks_to_disk must NOT write .cvh/tasks.json
    crate::commands::tasks::storage::save_tasks_to_disk(&root, &[task]).await.expect("save tasks");
    let tasks_json = root.join(".cvh").join("tasks.json");
    assert!(!tasks_json.exists(), ".cvh/tasks.json must not be written to disk on save");

    // 2. Verify task is in SQLite
    let loaded = crate::commands::tasks::storage::load_tasks_from_disk(&root).await;
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, "LIVE-001");

    // 3. Delete task from SQLite
    db_delete_task(&root, "LIVE-001").expect("delete task");
    assert!(db_get_all_tasks(&root).expect("db tasks").is_empty());

    // 4. Plant a legacy tasks.json containing a zombie task
    let zombie = serde_json::json!([{
        "id": "ZOMBIE-999",
        "title": "Zombie Task",
        "stageId": "backlog"
    }]);
    tokio::fs::write(&tasks_json, zombie.to_string()).await.expect("write zombie file");

    // 5. load_tasks_from_disk must NOT resurrect the zombie task!
    let empty_board = crate::commands::tasks::storage::load_tasks_from_disk(&root).await;
    assert_eq!(empty_board.len(), 0, "Empty board in SQLite must NOT resurrect zombie tasks from tasks.json");

    let _ = tokio::fs::remove_dir_all(&root).await;
}

#[tokio::test]
async fn test_empty_todos_clears_old_todos_in_sqlite() {
    let root = PathBuf::from("target").join(format!("cvh_ghost_todos_{}", uuid::Uuid::new_v4()));
    let task_id = "GHOST-001";
    let task = Task {
        id: task_id.to_string(),
        title: "Ghost Todos Task".to_string(),
        description: "Testing ghost todos fix".to_string(),
        stage_id: "backlog".to_string(),
        board_id: None,
        worktree_path: None,
        branch: None,
        verdict: None,
        custom_flow: None,
        status: Some("idle".to_string()),
        model: None,
        provider_id: None,
        reasoning_effort: None,
        parent_id: None,
        subtask_index: None,
        waiting_for_subtask_id: None,
        handover_summary: None,
        cover_image: None,
        priority: None,
        current_tokens: None,
        skip_worktree: Some(true),
        created_at: 100,
        updated_at: 100,
        stage_entered_at: None,
        completed_at: None,
        position: None,
        hook_states: None,
        stage_durations: None,
        total_elapsed: None,
        project_path: None,
        attachments: None,
        tags: None,
        todos: Some(vec![
            TodoItem { id: "td-1".into(), content: "item 1".into(), status: "todo".into() },
            TodoItem { id: "td-2".into(), content: "item 2".into(), status: "completed".into() },
        ]),
    };

    db_upsert_task(&root, &task).expect("initial upsert with todos");
    let loaded = db_get_all_tasks(&root).expect("load tasks");
    assert_eq!(loaded[0].todos.as_ref().unwrap().len(), 2);

    // Now upsert with empty todos list (e.g. user removed all checklist items)
    let mut updated_task = task.clone();
    updated_task.todos = Some(vec![]);
    db_upsert_task(&root, &updated_task).expect("upsert with empty todos");

    let loaded_after = db_get_all_tasks(&root).expect("load tasks after clearing todos");
    assert_eq!(loaded_after[0].todos.as_ref().map(|v| v.len()).unwrap_or(0), 0, "Empty todos must not leave ghost todos in SQLite");

    // Also test batch db_upsert_all_tasks
    let mut batch_task = task.clone();
    batch_task.id = "GHOST-BATCH".to_string();
    batch_task.todos = Some(vec![
        TodoItem { id: "td-b1".into(), content: "batch item".into(), status: "todo".into() },
    ]);
    db_upsert_all_tasks(&root, &[batch_task.clone()]).expect("batch upsert");
    assert_eq!(db_get_all_tasks(&root).expect("load").iter().find(|t| t.id == "GHOST-BATCH").unwrap().todos.as_ref().unwrap().len(), 1);

    batch_task.todos = Some(vec![]);
    db_upsert_all_tasks(&root, &[batch_task]).expect("batch upsert empty");
    assert_eq!(db_get_all_tasks(&root).expect("load").iter().find(|t| t.id == "GHOST-BATCH").unwrap().todos.as_ref().map(|v| v.len()).unwrap_or(0), 0);

    let _ = tokio::fs::remove_dir_all(&root).await;
}

#[test]
fn db_upsert_tasks_batch_preserves_todos_when_none() {
    let root = PathBuf::from("target").join(format!("cvh_batch_todos_{}", uuid::Uuid::new_v4()));
    let mut task = Task {
        id: "BATCH-TODO-1".into(),
        title: "Keep todos".into(),
        description: String::new(),
        stage_id: "backlog".into(),
        board_id: None,
        worktree_path: None,
        branch: None,
        verdict: None,
        custom_flow: None,
        status: Some("idle".into()),
        model: None,
        provider_id: None,
        reasoning_effort: None,
        parent_id: None,
        subtask_index: None,
        waiting_for_subtask_id: None,
        handover_summary: None,
        cover_image: None,
        priority: Some("medium".into()),
        current_tokens: None,
        skip_worktree: Some(true),
        created_at: 1,
        updated_at: 1,
        stage_entered_at: None,
        completed_at: None,
        position: None,
        hook_states: None,
        stage_durations: None,
        total_elapsed: None,
        project_path: None,
        attachments: None,
        tags: None,
        todos: Some(vec![TodoItem {
            id: "td-1".into(),
            content: "Stay intact".into(),
            status: "todo".into(),
        }]),
    };
    db_upsert_tasks_batch(&root, std::slice::from_ref(&task)).expect("initial batch upsert");

    task.todos = None;
    task.title = "Updated title".into();
    db_upsert_tasks_batch(&root, std::slice::from_ref(&task)).expect("second batch upsert");

    let loaded = db_get_all_tasks(&root).expect("load tasks");
    let found = loaded.into_iter().find(|t| t.id == "BATCH-TODO-1").expect("task exists");
    assert_eq!(found.title, "Updated title");
    assert_eq!(found.todos.as_ref().map(|v| v.len()).unwrap_or(0), 1);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn context_file_path_is_injective_and_missing_file_errors() {
    let root = PathBuf::from("target").join(format!("cvh_ctx_inj_{}", uuid::Uuid::new_v4()));
    let p_dot = context_file_path(&root, "CVH-1.2");
    let p_under = context_file_path(&root, "CVH-1_2");
    assert_ne!(p_dot, p_under);

    // Insert a 'file' context_snapshot marker without creating the file on disk.
    append_dialog_items(&root, "MISSING-CTX-1", &[ActivityItem {
        id: "ctx-missing".into(),
        item_type: "context_snapshot".into(),
        content: Some("file".into()),
        timestamp: "2026-01-01T00:00:00Z".into(),
        tool: None,
        duration_ms: None,
        images: None,
    }]).expect("append context marker");

    let err = load_conversation_context(&root, "MISSING-CTX-1").err().expect("missing file should error");
    assert!(err.contains("Failed to read context file"), "unexpected error: {err}");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn escape_like_prevents_wildcard_prefix_collision() {
    let root = PathBuf::from("target").join(format!("cvh_like_{}", uuid::Uuid::new_v4()));
    let call_1 = ActivityItem {
        id: "t1".into(),
        item_type: "tool".into(),
        content: None,
        timestamp: "2026-01-01T00:00:01Z".into(),
        tool: Some(serde_json::json!({ "id": "call_1", "name": "read_file" })),
        duration_ms: None,
        images: None,
    };
    let call_10 = ActivityItem {
        id: "t10".into(),
        item_type: "tool".into(),
        content: None,
        timestamp: "2026-01-01T00:00:02Z".into(),
        tool: Some(serde_json::json!({ "id": "call_10", "name": "read_file" })),
        duration_ms: None,
        images: None,
    };
    append_dialog_items(&root, "LIKE-TASK", &[call_1, call_10]).unwrap();

    db_update_dialog_tool_result(
        &root,
        "LIKE-TASK",
        "call_1",
        r#"{"id":"call_1","name":"read_file","status":"done","output":"ok-1"}"#,
        Some(5.0),
    )
    .unwrap();

    let items = load_dialog_items(&root, "LIKE-TASK").unwrap();
    let item_1 = items.iter().find(|i| i.id == "t1").unwrap();
    let item_10 = items.iter().find(|i| i.id == "t10").unwrap();
    assert_eq!(item_1.tool.as_ref().unwrap()["status"], "done");
    assert!(item_10.tool.as_ref().unwrap().get("status").is_none());

    // Also verify subagent clear prefix CVH-1 doesn't wipe CVH-10::sub::1
    append_dialog_items(&root, "CVH-10::sub::1", &[ActivityItem {
        id: "s10".into(),
        item_type: "output".into(),
        content: Some("keep me".into()),
        timestamp: "2026-01-01T00:00:01Z".into(),
        tool: None,
        duration_ms: None,
        images: None,
    }]).unwrap();
    clear_dialog_items(&root, "CVH-1").unwrap();
    assert_eq!(load_dialog_items(&root, "CVH-10::sub::1").unwrap().len(), 1);

    let _ = std::fs::remove_dir_all(&root);
}


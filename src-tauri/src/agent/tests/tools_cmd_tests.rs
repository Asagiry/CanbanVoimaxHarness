use super::*;
use crate::config::ToolPolicyConfig;
use std::sync::atomic::Ordering;
use std::sync::Arc;

static QUESTION_TEST_MUTEX: std::sync::LazyLock<tokio::sync::Mutex<()>> = std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

    #[tokio::test]
    async fn test_execute_command_timeout_cleanup_and_log_spilling() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_cmd_spill_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dirs");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: flag,
            tool_policy: &policy,
            task_id: Some("CVH-102"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // 1. Successful command execution
        #[cfg(target_os = "windows")]
        let cmd = "echo EXEC_SUCCESS";
        #[cfg(not(target_os = "windows"))]
        let cmd = "echo EXEC_SUCCESS";

        let res = execute_tool(&tool_ctx, "execute_command", &serde_json::json!({
            "command": cmd
        }))
        .await
        .expect("execute_command");

        assert!(res.contains("[Exit Code: 0]"), "got: {}", res);
        assert!(res.contains("EXEC_SUCCESS"), "got: {}", res);

        // 2. Command timeout test
        #[cfg(target_os = "windows")]
        let sleep_cmd = "ping 127.0.0.1 -n 5 > nul";
        #[cfg(not(target_os = "windows"))]
        let sleep_cmd = "sleep 5";

        let timeout_err = execute_tool(&tool_ctx, "execute_command", &serde_json::json!({
            "command": sleep_cmd,
            "timeout_seconds": 1
        }))
        .await
        .expect_err("Command must time out");

        assert!(timeout_err.to_string().contains("timed out"), "got: {}", timeout_err);

        // 3. Output spilling (§ Directive 1):
        // Generic commands: ceiling is 500 lines / 64KB
        let mut large_output = String::new();
        for i in 1..=520 {
            large_output.push_str(&format!("This is log line {}\n", i));
        }

        let spilled = maybe_spill_output(&base, "execute_command", large_output)
            .await
            .expect("spill output");

        assert!(spilled.contains("[Output truncated: 520 lines"), "got: {}", spilled);
        assert!(spilled.contains("spilled to .cvh/spill/"), "got: {}", spilled);
        assert!(spilled.contains("--- HEAD (first 20 lines) ---"), "got: {}", spilled);
        assert!(spilled.contains("--- TAIL (last 20 lines) ---"), "got: {}", spilled);

        // Verify subagent reports NEVER spill
        let mut subagent_rep = String::new();
        for i in 1..=600 {
            subagent_rep.push_str(&format!("Subagent finding {}\n", i));
        }
        let unspilled_sub = maybe_spill_output(&base, "subagent", subagent_rep.clone())
            .await
            .expect("subagent unspilled");
        assert_eq!(unspilled_sub, subagent_rep, "subagent reports must NEVER be truncated");

        let unspilled_collect = maybe_spill_output(&base, "collect_subagent", subagent_rep.clone())
            .await
            .expect("collect_subagent unspilled");
        assert_eq!(unspilled_collect, subagent_rep, "collect_subagent reports must NEVER be truncated");

        // Verify read_file: 600 lines does NOT spill (safe limit is 2000 lines)
        let unspilled_read = maybe_spill_output(&base, "read_file", subagent_rep)
            .await
            .expect("read_file unspilled at 600 lines");
        assert!(!unspilled_read.contains("[Output truncated"), "read_file must not spill under 2000 lines");

        // read_file with 2100 lines DOES spill
        let mut massive_file = String::new();
        for i in 1..=2100 {
            massive_file.push_str(&format!("File line {}\n", i));
        }
        let spilled_read = maybe_spill_output(&base, "read_file", massive_file)
            .await
            .expect("read_file spilled at 2100 lines");
        assert!(spilled_read.contains("[Output truncated: 2100 lines"), "got: {}", spilled_read);

        // Verify spilled log file actually exists on disk
        let spill_dir = base.join(".cvh").join("spill");
        assert!(spill_dir.is_dir(), "spill dir must exist");
        let mut entries = tokio::fs::read_dir(&spill_dir).await.expect("read spill dir");
        let first_file = entries.next_entry().await.expect("entry").expect("some entry");
        assert!(first_file.file_name().to_string_lossy().contains(".log"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn test_execute_command_windows_quoting_python() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_cmd_quote_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dirs");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: flag,
            tool_policy: &policy,
            task_id: Some("CVH-QUOTE-TEST"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // 1. Verify python -c "print(1)" succeeds with exit code 0 and stdout "1"
        let res = execute_tool(&tool_ctx, "execute_command", &serde_json::json!({
            "command": "python -c \"print(1)\""
        }))
        .await
        .expect("execute_command with quotes");

        assert!(res.contains("[Exit Code: 0]"), "got: {}", res);
        assert!(res.contains("--- STDOUT ---
1"), "stdout should contain 1, got: {}", res);

        // 2. Verify complex nested quotes: python -c "import json; print(json.dumps({'status': 'ok'}))"
        let res_complex = execute_tool(&tool_ctx, "execute_command", &serde_json::json!({
            "command": "python -c \"import json; print(json.dumps({'status': 'ok'}))\""
        }))
        .await
        .expect("execute_command with complex quotes");

        assert!(res_complex.contains("[Exit Code: 0]"), "got: {}", res_complex);
        assert!(res_complex.contains(r#"{"status": "ok"}"#), "stdout should contain json, got: {}", res_complex);

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    // -------------------------------------------------------------------------
    // e) Orchestration & Invariants: monotonic ID with gaps, complete_task, cvh-dev gating
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_question_tool_schema_and_pause_unblock() {
        let _lock = QUESTION_TEST_MUTEX.lock().await;
        let base = std::path::PathBuf::from("target").join(format!("cvh_q_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-090"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // Spawn responder task which polls QUESTION_REGISTRY and delivers answer
        let responder = tokio::spawn(async {
            for _ in 0..60 {
                tokio::time::sleep(Duration::from_millis(50)).await;
                let mut reg = QUESTION_REGISTRY.lock().await;
                if !reg.is_empty() {
                    let q_id = reg.keys().next().cloned().unwrap();
                    if let Some(sender) = reg.remove(&q_id) {
                        let _ = sender.send("SQLite".to_string());
                        return true;
                    }
                }
            }
            false
        });

        let res = execute_tool(&tool_ctx, "question", &serde_json::json!({
            "text": "Which database engine?",
            "options": [
                { "label": "SQLite", "description": "Local embedded" },
                { "label": "PostgreSQL", "description": "Client-server" }
            ],
            "multiple": false
        }))
        .await
        .expect("question execution");

        let did_answer = responder.await.expect("join responder");
        assert!(did_answer, "Question was not registered in QUESTION_REGISTRY within timeout");
        assert_eq!(res, "SQLite");

        // Verify SQLite persisted question activity item with selectedAnswer
        let dialog_items = crate::db::load_dialog_items(&base, "CVH-090").expect("load items");
        assert!(!dialog_items.is_empty(), "Question must be persisted in SQLite");
        let q_item = dialog_items.iter().find(|i| i.item_type == "question").expect("question item");
        assert!(q_item.content.as_deref().unwrap().contains("Which database engine?"));
        let tool_meta = q_item.tool.as_ref().expect("tool meta");
        assert_eq!(tool_meta["status"], "success");
        assert_eq!(tool_meta["selectedAnswer"], "SQLite");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_question_tool_abort_flag_cleanup() {
        let _lock = QUESTION_TEST_MUTEX.lock().await;
        let base = std::path::PathBuf::from("target").join(format!("cvh_q_abort_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = Arc::clone(&flag);
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: flag,
            tool_policy: &policy,
            task_id: Some("CVH-091"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // Spawn task to trigger abort flag after brief pause
        let canceller = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            flag_clone.store(true, Ordering::SeqCst);
        });

        let err = execute_tool(&tool_ctx, "ask_user", &serde_json::json!({
            "text": "Will be aborted?"
        }))
        .await
        .expect_err("Must error on abort");

        canceller.await.expect("join canceller");
        assert!(err.to_string().contains("abort") || err.to_string().contains("Operation aborted"), "got: {}", err);

        // Verify registry is cleaned up
        let reg = QUESTION_REGISTRY.lock().await;
        assert!(reg.is_empty(), "Registry must be empty after abort");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_todo_write_checklist_updates_and_status_transitions() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_todo_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        // Seed task in tasks.json
        let task = make_test_task("CVH-095", "dev", "development");
        crate::commands::tasks::save_tasks_to_disk(&base, &[task]).await.expect("save task");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: flag,
            tool_policy: &policy,
            task_id: Some("CVH-095"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // Call todo_write with varied statuses (including 'done' and 'running' which normalize)
        let todo_args = serde_json::json!({
            "todos": [
                { "content": "Scaffold backend", "status": "done" },
                { "content": "Execute unit tests", "status": "running" },
                { "content": "Review PR", "status": "pending" },
                { "content": "Write docs", "status": "completed" }
            ]
        });

        let res = execute_tool(&tool_ctx, "todo_write", &todo_args)
            .await
            .expect("todo_write execution");

        assert_eq!(res, "Updated todo list: 2 completed, 1 in progress, 1 pending.");

        // Check on-disk persistence
        let tasks = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t = tasks.iter().find(|t| t.id == "CVH-095").expect("task exists");
        let todos = t.todos.as_ref().expect("todos must be persisted");
        assert_eq!(todos.len(), 4);
        assert_eq!(todos[0].status, "completed");
        assert_eq!(todos[1].status, "in_progress");
        assert_eq!(todos[2].status, "pending");
        assert_eq!(todos[3].status, "completed");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    // -------------------------------------------------------------------------
    // d) Worktree Filesystem & Shell: read, write, edit, glob, grep, execute
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_tool_list_tasks() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_list_tasks_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dirs");
        crate::db::init_db(&base).expect("init db");

        let task1 = Task {
            id: "CVH-101".to_string(),
            title: "Optimize compiler sensor".to_string(),
            description: "Deep compiler optimizations".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            handover_summary: Some("Compiler partially optimized".to_string()),
            cover_image: None,
            attachments: None,
            todos: None,
            priority: Some("high".to_string()),
            tags: None,
            current_tokens: None,
            skip_worktree: None,
            created_at: 1000,
            updated_at: 2000,
            stage_entered_at: None,
            completed_at: None,
            position: Some(0.0),
            hook_states: None,
        };

        let task2 = Task {
            id: "CVH-102".to_string(),
            title: "Audit security invariants".to_string(),
            description: "Review token leaks".to_string(),
            stage_id: "review".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: Some("APPROVED".to_string()),
            custom_flow: None,
            status: Some("idle".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            handover_summary: Some("Security invariants verified".to_string()),
            cover_image: None,
            attachments: None,
            todos: None,
            priority: Some("medium".to_string()),
            tags: None,
            current_tokens: None,
            skip_worktree: None,
            created_at: 1000,
            updated_at: 3000,
            stage_entered_at: None,
            completed_at: None,
            position: Some(1.0),
            hook_states: None,
        };

        crate::db::db_upsert_task(&base, &task1).expect("upsert task1");
        crate::db::db_upsert_task(&base, &task2).expect("upsert task2");

        let policy = ToolPolicyConfig::default();
        let abort_flag = Arc::new(AtomicBool::new(false));
        let ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: Arc::clone(&abort_flag),
            tool_policy: &policy,
            task_id: Some("CVH-101"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // 1. List all tasks (default limit, sorted newest updated_at first)
        let res_str = execute_tool(&ctx, "list_tasks", &serde_json::json!({})).await.expect("list_tasks");
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&res_str).expect("parse json");
        assert_eq!(parsed.len(), 2);
        // CVH-102 has updated_at 3000, CVH-101 has 2000 -> CVH-102 should be first
        assert_eq!(parsed[0]["id"], "CVH-102");
        assert_eq!(parsed[0]["stage_id"], "review");
        assert_eq!(parsed[0]["status"], "idle");
        assert_eq!(parsed[0]["role"], "reviewer");
        assert_eq!(parsed[1]["id"], "CVH-101");
        assert_eq!(parsed[1]["stage_id"], "dev");
        assert_eq!(parsed[1]["role"], "cvh-dev");

        // 2. Filter by stage_id
        let res_dev = execute_tool(&ctx, "list_tasks", &serde_json::json!({ "stage_id": "dev" })).await.expect("list_tasks stage");
        let parsed_dev: Vec<serde_json::Value> = serde_json::from_str(&res_dev).expect("parse json");
        assert_eq!(parsed_dev.len(), 1);
        assert_eq!(parsed_dev[0]["id"], "CVH-101");

        // 3. Filter by query
        let res_q = execute_tool(&ctx, "list_tasks", &serde_json::json!({ "query": "security" })).await.expect("list_tasks query");
        let parsed_q: Vec<serde_json::Value> = serde_json::from_str(&res_q).expect("parse json");
        assert_eq!(parsed_q.len(), 1);
        assert_eq!(parsed_q[0]["id"], "CVH-102");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_tool_get_task_history() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_task_hist_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dirs");
        crate::db::init_db(&base).expect("init db");

        let task = Task {
            id: "CVH-201".to_string(),
            title: "Implement task inspection tools".to_string(),
            description: "Add list_tasks, get_task_history, and load_rule tools for agents.".to_string(),
            stage_id: "review".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: Some("APPROVED".to_string()),
            custom_flow: None,
            status: Some("idle".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            handover_summary: Some("Tools implemented and verified with sensors.".to_string()),
            cover_image: None,
            attachments: None,
            todos: None,
            priority: Some("high".to_string()),
            tags: None,
            current_tokens: None,
            skip_worktree: None,
            created_at: 1000,
            updated_at: 2000,
            stage_entered_at: None,
            completed_at: None,
            position: Some(0.0),
            hook_states: None,
        };
        crate::db::db_upsert_task(&base, &task).expect("upsert task");

        let dialog_items = vec![
            ActivityItem {
                id: "item-1".to_string(),
                item_type: "steer".to_string(),
                timestamp: "10:00:00".to_string(),
                content: Some("Follow AGENTS.md invariants strictly.".to_string()),
                tool: None,
                duration_ms: None,
                images: None,
            },
            ActivityItem {
                id: "item-2".to_string(),
                item_type: "tool".to_string(),
                timestamp: "10:00:05".to_string(),
                content: None,
                tool: Some(serde_json::json!({
                    "id": "tc-1",
                    "toolName": "read_file",
                    "params": { "file_path": "src-tauri/src/agent.rs" },
                    "status": "success",
                    "durationMs": 42.0,
                    "stdout": "pub fn get_standard_tools() -> Vec<ToolDefinition> { ... }"
                })),
                duration_ms: Some(42.0),
                images: None,
            },
            ActivityItem {
                id: "item-3".to_string(),
                item_type: "tool".to_string(),
                timestamp: "10:00:10".to_string(),
                content: None,
                tool: Some(serde_json::json!({
                    "id": "tc-2",
                    "toolName": "execute_command",
                    "params": { "command": "cargo check" },
                    "status": "failed",
                    "durationMs": 150.0,
                    "stderr": "error[E0425]: cannot find value 'load_rule'"
                })),
                duration_ms: Some(150.0),
                images: None,
            },
            ActivityItem {
                id: "item-4".to_string(),
                item_type: "output".to_string(),
                timestamp: "10:00:15".to_string(),
                content: Some("Fixed load_rule and verified check successfully.".to_string()),
                tool: None,
                duration_ms: None,
                images: None,
            },
        ];
        crate::db::append_dialog_items(&base, "CVH-201", &dialog_items).expect("append dialog");

        let policy = ToolPolicyConfig::default();
        let abort_flag = Arc::new(AtomicBool::new(false));
        let ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("review"),
            role: Some("reviewer"),
            abort_handle: Arc::clone(&abort_flag),
            tool_policy: &policy,
            task_id: Some("CVH-201"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        let report = execute_tool(
            &ctx,
            "get_task_history",
            &serde_json::json!({ "task_id": "CVH-201", "include_tool_outputs": true }),
        )
        .await
        .expect("get_task_history");
        assert!(report.contains("# Task History: CVH-201 - Implement task inspection tools (Этап: review, Статус: idle, Роль: reviewer)"));
        assert!(report.contains("## Initial Objective:\nAdd list_tasks, get_task_history, and load_rule tools for agents."));
        assert!(report.contains("## Execution Steps:"));
        assert!(report.contains("Шаг 1 [User Steer / System]: Follow AGENTS.md invariants strictly."));
        assert!(report.contains(r#"Шаг 2 [Tool read_file]: params: {"file_path":"src-tauri/src/agent.rs"}, status: success, duration: 42ms, error: none."#));
        assert!(report.contains("Output: pub fn get_standard_tools()"));
        assert!(report.contains(r#"Шаг 3 [Tool execute_command]: params: {"command":"cargo check"}, status: failed, duration: 150ms, error: error[E0425]: cannot find value 'load_rule'."#));
        assert!(report.contains("## Handover & Terminal Response:"));
        assert!(report.contains("- Handover Summary: Tools implemented and verified with sensors."));
        assert!(report.contains("- Final Output / Verdict: [APPROVED] Fixed load_rule and verified check successfully."));
        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[test]
    fn test_cp866_decoding_and_output_fallback() {
        use super::{decode_cp866, decode_output_bytes};

        // 1. Plain ASCII
        let ascii_bytes = b"Hello, World!";
        assert_eq!(decode_output_bytes(ascii_bytes), "Hello, World!");

        // 2. Standard UTF-8 Cyrillic
        let utf8_bytes = "Привет, мир!".as_bytes();
        assert_eq!(decode_output_bytes(utf8_bytes), "Привет, мир!");

        // 3. Pure CP866 bytes for "Привет"
        // П = 0x8F, р = 0xE0, и = 0xA8, в = 0xA2, е = 0xA5, т = 0xE2
        let cp866_bytes = [0x8Fu8, 0xE0, 0xA8, 0xA2, 0xA5, 0xE2];
        assert_eq!(decode_cp866(&cp866_bytes), "Привет");
        // decode_output_bytes should fall back to CP866 because it's invalid UTF-8
        assert_eq!(decode_output_bytes(&cp866_bytes), "Привет");

        // 4. Output with replacement char \u{FFFD}
        let with_fffd = "error \u{FFFD}".as_bytes();
        // Since from_utf8 succeeds but contains \u{FFFD}, it falls back to decode_cp866
        let fallback_result = decode_output_bytes(with_fffd);
        assert!(!fallback_result.is_empty());
    }

    #[test]
    fn test_build_enriched_windows_paths() {
        use super::build_enriched_windows_paths;
        use std::ffi::OsString;

        let initial_path = OsString::from(r"C:\Windows\System32;C:\Windows");
        let enriched = build_enriched_windows_paths(&initial_path);
        assert!(!enriched.is_empty());
        assert!(enriched.iter().any(|p| p.to_string_lossy().contains("Windows")));
    }

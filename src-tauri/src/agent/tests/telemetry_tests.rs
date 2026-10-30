use super::*;
use crate::config::ToolPolicyConfig;
use std::sync::Arc;

    #[test]
    fn test_task_status_adt_and_domain_methods() {
        let mut task = Task {
            id: "CVH-123".to_string(),
            title: "Test Task".to_string(),
            status: None,
            ..Default::default()
        };

        // Default status is Idle
        assert_eq!(task.get_status(), TaskStatus::Idle);
        assert!(!task.is_done());
        assert!(!task.is_running());
        assert!(!task.is_waiting_subtask());

        // Set status Running
        task.set_status(TaskStatus::Running);
        assert_eq!(task.get_status(), TaskStatus::Running);
        assert!(task.is_running());
        assert_eq!(task.status.as_deref(), Some("running"));

        // Set status WaitingSubtask
        task.set_status(TaskStatus::WaitingSubtask);
        assert_eq!(task.get_status(), TaskStatus::WaitingSubtask);
        assert!(task.is_waiting_subtask());

        // Set status Done
        task.set_status(TaskStatus::Done);
        assert_eq!(task.get_status(), TaskStatus::Done);
        assert!(task.is_done());
        assert_eq!(task.status.as_deref(), Some("done"));

        // Custom status
        task.set_status(TaskStatus::Custom("reviewing".to_string()));
        assert_eq!(task.get_status(), TaskStatus::Custom("reviewing".to_string()));
        assert_eq!(task.get_status().as_str(), "reviewing");
    }



    #[tokio::test]
    async fn test_orchestrator_create_task_and_create_tasks() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_orch_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        // Seed parent orchestrator task CVH-001
        let parent_task = Task {
            id: "CVH-001".to_string(),
            title: "Parent Orchestrator Task".to_string(),
            description: "Decomposing feature".to_string(),
            stage_id: "backlog".to_string(),
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
            handover_summary: None,
            cover_image: None,
            attachments: None,
            todos: None,
            priority: Some("high".to_string()),
            tags: Some(vec!["orchestrator".to_string()]),
            current_tokens: None,
            skip_worktree: None,
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };
        crate::commands::tasks::save_tasks_to_disk(&base, &[parent_task]).await.expect("save parent");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("backlog"),
            role: Some("orchestrator"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-001"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // 1. Single create_task call
        let single_args = serde_json::json!({
            "title": "Backend Rust Implementation",
            "description": "Implement create_task and complete_task tools",
            "column_id": "dev",
            "priority": "high",
            "tags": ["rust", "tauri"]
        });
        let single_res = execute_tool(&tool_ctx, "create_task", &single_args)
            .await
            .expect("create_task execution");

        let parsed_single: serde_json::Value = serde_json::from_str(&single_res).expect("parse single json");
        assert_eq!(parsed_single["status"], "created");
        assert_eq!(parsed_single["task_id"], "CVH-002");
        assert_eq!(parsed_single["title"], "Backend Rust Implementation");
        assert_eq!(parsed_single["column_id"], "dev");

        // Verify on disk that CVH-002 has parent_id = None (standalone root task by default)
        let tasks_on_disk = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let created_child = tasks_on_disk.iter().find(|t| t.id == "CVH-002").expect("CVH-002 on disk");
        assert_eq!(created_child.parent_id.as_deref(), None);
        assert_eq!(created_child.priority.as_deref(), Some("high"));
        assert_eq!(created_child.tags.as_ref().unwrap(), &vec!["rust".to_string(), "tauri".to_string()]);

        // 2. Batch create_tasks call
        let batch_args = serde_json::json!({
            "tasks": [
                {
                    "title": "Frontend UI Matrix",
                    "description": "Integrate into toolPolicyStore.ts",
                    "column_id": "dev",
                    "priority": "medium",
                    "is_subtask": true
                },
                {
                    "title": "End-to-End Test Suite",
                    "description": "Run cargo check and vitest",
                    "column_id": "dev",
                    "priority": "high"
                }
            ]
        });
        let batch_res = execute_tool(&tool_ctx, "create_tasks", &batch_args)
            .await
            .expect("create_tasks execution");

        let parsed_batch: serde_json::Value = serde_json::from_str(&batch_res).expect("parse batch json");
        assert_eq!(parsed_batch["status"], "created");
        assert_eq!(parsed_batch["count"], 2);

        let tasks_after_batch = crate::commands::tasks::load_tasks_from_disk(&base).await;
        assert_eq!(tasks_after_batch.len(), 4); // CVH-001, CVH-002, CVH-003, CVH-004

        let t3 = tasks_after_batch.iter().find(|t| t.id == "CVH-003").expect("CVH-003");
        let t4 = tasks_after_batch.iter().find(|t| t.id == "CVH-004").expect("CVH-004");
        assert_eq!(t3.parent_id.as_deref(), Some("CVH-001"));
        assert_eq!(t4.parent_id.as_deref(), None);

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_orchestrator_monotonic_id_with_gaps() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_gaps_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        // Seed CVH-002 and CVH-007 (gap between 2 and 7)
        let t1 = make_test_task("CVH-002", "dev", "development");
        let t2 = make_test_task("CVH-007", "dev", "development");
        crate::commands::tasks::save_tasks_to_disk(&base, &[t1, t2]).await.expect("save tasks");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("orchestrator"),
            abort_handle: flag,
            tool_policy: &policy,
            task_id: Some("CVH-007"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // Creating a task must assign CVH-008 (monotonic max+1, never reuses 001, 003, etc.)
        let res = execute_tool(&tool_ctx, "create_task", &serde_json::json!({
            "title": "Next In Sequence Task"
        }))
        .await
        .expect("create_task");

        let parsed: serde_json::Value = serde_json::from_str(&res).expect("parse json");
        assert_eq!(parsed["task_id"], "CVH-008", "Must assign CVH-008, not fill gaps");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_complete_task_anti_bypass_invariant() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_antibypass_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        // Seed a regular developer task CVH-010 that has NO child tasks
        let regular_task = Task {
            id: "CVH-010".to_string(),
            title: "Regular Developer Task".to_string(),
            description: "Working on code".to_string(),
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
            handover_summary: None,
            cover_image: None,
            attachments: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: None,
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };
        crate::commands::tasks::save_tasks_to_disk(&base, &[regular_task]).await.expect("save regular task");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let dev_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-010"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // 1. When allowed by tool-policy (default for dev), calling complete_task succeeds directly without requiring child tasks
        let complete_res = execute_tool(&dev_ctx, "complete_task", &serde_json::json!({
            "summary": "Finished task directly without requiring child subtasks"
        }))
        .await
        .expect("complete_task must succeed when allowed by tool-policy");

        let parsed_res: serde_json::Value = serde_json::from_str(&complete_res).expect("parse complete json");
        assert_eq!(parsed_res["task_id"], "CVH-010");
        assert_eq!(parsed_res["status"], "done");
        assert_eq!(parsed_res["column_id"], "ready");

        // Verify on disk that CVH-010 moved to 'ready', status is 'done', and handover_summary is recorded
        let tasks_final = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t10_done = tasks_final.iter().find(|t| t.id == "CVH-010").unwrap();
        assert_eq!(t10_done.stage_id, "ready");
        assert_eq!(t10_done.status.as_deref(), Some("done"));
        assert_eq!(t10_done.handover_summary.as_deref(), Some("Finished task directly without requiring child subtasks"));

        // 2. When policy explicitly denies complete_task, it is rejected with PermissionDenied
        let mut denied_policy = ToolPolicyConfig::default();
        denied_policy.stage_overrides.get_mut("dev").unwrap().insert("complete_task".to_string(), false);
        let denied_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &denied_policy,
            task_id: Some("CVH-010"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };
        let bypass_err = execute_tool(&denied_ctx, "complete_task", &serde_json::json!({
            "summary": "Should be denied by policy"
        }))
        .await
        .expect_err("complete_task must be rejected when disabled by policy");
        assert!(bypass_err.to_string().contains("PermissionDenied"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_complete_task_in_orchestrator_role_without_children() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_orch_comp_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let (boards_cfg, _) = make_test_board(&base);
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed task in 'dev' with role 'orchestrator'
        let task = make_test_task("CVH-110", "dev", "test-board");
        crate::commands::tasks::save_tasks_to_disk(&base, &[task]).await.expect("save task");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("orchestrator"), // Orchestrator role bypasses child subtask requirement
            abort_handle: flag,
            tool_policy: &policy,
            task_id: Some("CVH-110"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        let res = execute_tool(&tool_ctx, "complete_task", &serde_json::json!({
            "summary": "High level orchestration concluded"
        }))
        .await
        .expect("complete_task in orchestrator role");

        let parsed: serde_json::Value = serde_json::from_str(&res).expect("parse json");
        assert_eq!(parsed["task_id"], "CVH-110");
        assert_eq!(parsed["status"], "done");
        assert_eq!(parsed["column_id"], "ready");

        let tasks = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t = tasks.iter().find(|t| t.id == "CVH-110").unwrap();
        assert_eq!(t.stage_id, "ready");
        assert_eq!(t.status.as_deref(), Some("done"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }



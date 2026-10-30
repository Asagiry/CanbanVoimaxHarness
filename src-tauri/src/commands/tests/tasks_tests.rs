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


    #[tokio::test]
    async fn test_advance_task_and_board_switch_finalization() {
        let base = PathBuf::from("target").join(format!("cvh_adv_switch_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let b1_cols = vec![
            crate::config::PipelineColumn {
                id: "backlog".to_string(),
                title: "Backlog".to_string(),
                color: None,
                icon: None,
                persona: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
                is_intake: Some(true),
            },
            crate::config::PipelineColumn {
                id: "dev".to_string(),
                title: "Dev".to_string(),
                color: None,
                icon: None,
                persona: Some("developer".to_string()),
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
                is_intake: None,
            },
            crate::config::PipelineColumn {
                id: "ready".to_string(),
                title: "Ready".to_string(),
                color: None,
                icon: None,
                persona: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: Some(true),
                is_intake: None,
            },
        ];

        let b2_cols = vec![
            crate::config::PipelineColumn {
                id: "todo".to_string(),
                title: "Todo".to_string(),
                color: None,
                icon: None,
                persona: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
                is_intake: Some(true),
            },
            crate::config::PipelineColumn {
                id: "in_progress".to_string(),
                title: "In Progress".to_string(),
                color: None,
                icon: None,
                persona: Some("developer".to_string()),
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
                is_intake: None,
            },
            crate::config::PipelineColumn {
                id: "done".to_string(),
                title: "Done".to_string(),
                color: None,
                icon: None,
                persona: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: Some(true),
                is_intake: None,
            },
        ];

        let mut boards_cfg = crate::config::BoardsConfig {
            active_board_id: "dev-board".to_string(),
            boards: vec![
                crate::config::BoardDef {
                    id: "dev-board".to_string(),
                    name: "Development".to_string(),
                    columns: b1_cols,
                },
                crate::config::BoardDef {
                    id: "kanban-board".to_string(),
                    name: "Kanban".to_string(),
                    columns: b2_cols,
                },
            ],
        };
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed task on dev-board in stage dev
        let task1 = Task {
            id: "CVH-001".to_string(),
            title: "Task 1".to_string(),
            description: "Desc 1".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("dev-board".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: None,
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };
        save_tasks_to_disk(&base, &[task1]).await.expect("save task1");

        // 1. Normal advance on same board -> transitions to 'ready' with status 'done'
        let adv_res = internal_advance_task(&base, "CVH-001", "Summary for task 1", None).await.expect("advance task1");
        assert!(adv_res.contains("УСПЕШНО"));
        let loaded = load_tasks_from_disk(&base).await;
        let t1 = loaded.iter().find(|t| t.id == "CVH-001").unwrap();
        assert_eq!(t1.stage_id, "ready");
        assert_eq!(t1.status.as_deref(), Some("done"));

        // 2. Board switch scenario (Ironclad Board Preservation Invariant):
        // Seed task 2 on dev-board in stage dev
        let task2 = Task {
            id: "CVH-002".to_string(),
            title: "Task 2".to_string(),
            description: "Desc 2".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("dev-board".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: None,
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };
        save_tasks_to_disk(&base, &[t1.clone(), task2]).await.expect("save task2");

        // Now active board switches to kanban-board, but task 2 belongs to dev-board.
        boards_cfg.active_board_id = "kanban-board".to_string();
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards switch");

        // Advance task 2: active board changed -> task MUST strictly preserve its board_id ('dev-board')
        // and advance to terminal column ('ready') on dev-board without hopping to kanban-board!
        let adv_switch_res = internal_advance_task(&base, "CVH-002", "Summary after board switch", None).await.expect("advance task2 switch");
        assert!(adv_switch_res.contains("УСПЕШНО"));
        assert!(adv_switch_res.contains("завершена"));

        let loaded2 = load_tasks_from_disk(&base).await;
        let t2 = loaded2.iter().find(|t| t.id == "CVH-002").unwrap();
        assert_eq!(t2.board_id.as_deref(), Some("dev-board"), "Task must strictly preserve dev-board despite active board switch");
        assert_eq!(t2.stage_id, "ready");
        assert_eq!(t2.status.as_deref(), Some("done"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_move_task_to_backlog_resets_idle_status() {
        let base = PathBuf::from("target").join(format!("cvh_backlog_abort_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let cols = vec![
            crate::config::PipelineColumn {
                id: "backlog".to_string(),
                title: "Backlog".to_string(),
                color: None,
                icon: None,
                persona: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
                is_intake: Some(true),
            },
            crate::config::PipelineColumn {
                id: "dev".to_string(),
                title: "Dev".to_string(),
                color: None,
                icon: None,
                persona: Some("developer".to_string()),
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: Some(true),
                is_intake: None,
            },
        ];

        let boards_cfg = crate::config::BoardsConfig {
            active_board_id: "default-board".to_string(),
            boards: vec![
                crate::config::BoardDef {
                    id: "default-board".to_string(),
                    name: "Default".to_string(),
                    columns: cols,
                },
            ],
        };
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed a running task in dev
        let task = Task {
            id: "CVH-ABORT-1".to_string(),
            title: "Running Task".to_string(),
            description: "Desc".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("default-board".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: None,
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };
        save_tasks_to_disk(&base, &[task]).await.expect("save task");

        // Move to backlog
        let moved = internal_move_task_stage(
            &base,
            "CVH-ABORT-1",
            "backlog",
            None,
            None,
            None,
            None,
            false,
            None,
        )
        .await
        .expect("move to backlog");

        assert_eq!(moved.stage_id, "backlog");
        assert_eq!(moved.status.as_deref(), Some("idle"));

        let loaded = load_tasks_from_disk(&base).await;
        assert_eq!(loaded[0].stage_id, "backlog");
        assert_eq!(loaded[0].status.as_deref(), Some("idle"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_resolve_task_active_agent_model_strict_task_override() {
        let base = PathBuf::from("target").join(format!("cvh_model_res_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let llm_settings = crate::config::LlmSettings {
            model: "global-default-model".to_string(),
            ..Default::default()
        };

        let mut task = Task {
            id: "CVH-001".to_string(),
            title: "Task".to_string(),
            description: String::new(),
            stage_id: "dev".to_string(),
            board_id: None,
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: None,
            model: Some("user-custom-model".to_string()),
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: None,
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };

        // 1. When task.model is set, it strictly overrides global model and column model!
        let resolved = resolve_task_active_agent_model(&base, Some(&task), &llm_settings).await;
        assert_eq!(resolved, "user-custom-model");

        // 2. When task.model is None or empty, it falls back to global model
        task.model = None;
        let fallback_global = resolve_task_active_agent_model(&base, Some(&task), &llm_settings).await;
        assert_eq!(fallback_global, "global-default-model");

        task.model = Some("   ".to_string());
        let fallback_whitespace = resolve_task_active_agent_model(&base, Some(&task), &llm_settings).await;
        assert_eq!(fallback_whitespace, "global-default-model");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[test]
    fn test_is_matching_subagent_key() {
        use crate::commands::tasks::is_matching_subagent_key;

        // Exact match
        assert!(is_matching_subagent_key("CVH-001.sub-abc123", "CVH-001.sub-abc123"));

        // Strip :sub: prefix
        assert!(is_matching_subagent_key("CVH-001.sub-abc123", "CVH-001:sub:CVH-001.sub-abc123"));
        assert!(is_matching_subagent_key("CVH-001:sub:CVH-001.sub-abc123", "CVH-001.sub-abc123"));

        // Short suffix match
        assert!(is_matching_subagent_key("CVH-001.sub-abc123", "CVH-001:sub:abc123"));
        assert!(is_matching_subagent_key("CVH-001.sub-abc123", "abc123"));

        // Sibling subagents must NEVER match
        assert!(!is_matching_subagent_key("CVH-001.sub-xyz999", "CVH-001.sub-abc123"));

        // Parent task must NEVER match child subagent
        assert!(!is_matching_subagent_key("CVH-001", "CVH-001.sub-abc123"));
        assert!(!is_matching_subagent_key("CVH-001", "abc123"));
    }


    #[test]
    fn test_active_agents_duplicate_run_check() {
        let active_map = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
        let task_id = "CVH-042".to_string();

        // 1. Not in map -> not running
        let is_running_initial = active_map.lock().map(|map| map.contains_key(&task_id)).unwrap_or(false);
        assert!(!is_running_initial);

        // 2. Insert dummy handle
        let abort_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let join_slot = std::sync::Arc::new(tokio::sync::Mutex::new(None));
        active_map.lock().unwrap().insert(
            task_id.clone(),
            crate::commands::ActiveAgentHandle {
                abort_flag,
                join_handle: join_slot,
            },
        );

        // 3. Now in map -> is_running is true, reject duplicate run
        let is_running_after = active_map.lock().map(|map| map.contains_key(&task_id)).unwrap_or(false);
        assert!(is_running_after);
    }


    #[tokio::test]
    async fn test_internal_advance_task_creates_review_subtask_and_no_self_abort() {
        let base = PathBuf::from("target").join(format!("cvh_review_subtask_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let cols = vec![
            crate::config::PipelineColumn {
                id: "backlog".to_string(),
                title: "Backlog".to_string(),
                color: None,
                icon: None,
                persona: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
                is_intake: Some(true),
            },
            crate::config::PipelineColumn {
                id: "dev".to_string(),
                title: "Development".to_string(),
                color: None,
                icon: None,
                persona: Some("cvh-dev".to_string()),
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: Some(true),
                terminal: None,
                is_intake: None,
            },
            crate::config::PipelineColumn {
                id: "review".to_string(),
                title: "Review".to_string(),
                color: None,
                icon: None,
                persona: Some("reviewer".to_string()),
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: Some(true),
                terminal: None,
                is_intake: None,
            },
            crate::config::PipelineColumn {
                id: "ready".to_string(),
                title: "Ready".to_string(),
                color: None,
                icon: None,
                persona: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: Some(true),
                is_intake: None,
            },
        ];

        let boards_cfg = crate::config::BoardsConfig {
            active_board_id: "dev-board".to_string(),
            boards: vec![crate::config::BoardDef {
                id: "dev-board".to_string(),
                name: "Development".to_string(),
                columns: cols,
            }],
        };
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed parent task in 'dev' stage with running status and worktree details
        let parent_task = Task {
            id: "CVH-100".to_string(),
            title: "Implement Review Subtask".to_string(),
            description: "Implement § 2.2 Review Subtask Hierarchy".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("dev-board".to_string()),
            worktree_path: Some("C:\\fake\\worktree".to_string()),
            branch: Some("feat/CVH-100".to_string()),
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: Some("custom-model".to_string()),
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            todos: None,
            priority: Some("high".to_string()),
            tags: None,
            current_tokens: None,
            skip_worktree: Some(false),
            created_at: 1000,
            updated_at: 1000,
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };
        save_tasks_to_disk(&base, &[parent_task]).await.expect("save parent task");

        // 1. Advance task targeting review stage:
        // Must create child subtask CVH-100.1 in 'review' column and set parent to 'waiting_subtask'
        let adv_res = internal_advance_task(
            &base,
            "CVH-100",
            "Handover: implemented feature and tests",
            None,
        )
        .await
        .expect("internal_advance_task must succeed");

        assert!(adv_res.contains("УСПЕШНО: Создана подзадача CVH-100.1"));

        let loaded = load_tasks_from_disk(&base).await;
        assert_eq!(loaded.len(), 2, "Must contain parent and child subtask");

        let parent = loaded.iter().find(|t| t.id == "CVH-100").unwrap();
        assert_eq!(parent.stage_id, "dev", "Parent must remain pinned in 'dev' stage (§ 2.2)");
        assert_eq!(parent.status.as_deref(), Some("waiting_subtask"), "Parent status must be 'waiting_subtask'");
        assert_eq!(parent.waiting_for_subtask_id.as_deref(), Some("CVH-100.1"), "Parent must be waiting for CVH-100.1");

        let subtask = loaded.iter().find(|t| t.id == "CVH-100.1").unwrap();
        assert_eq!(subtask.parent_id.as_deref(), Some("CVH-100"));
        assert_eq!(subtask.subtask_index, Some(1));
        assert_eq!(subtask.stage_id, "review", "Subtask must be in target review column");
        assert_eq!(subtask.status.as_deref(), Some("running"), "Target has persona 'reviewer', status must be 'running'");
        assert_eq!(subtask.title, "Implement Review Subtask (Review)");
        assert_eq!(subtask.description, "Handover: implemented feature and tests");
        assert_eq!(subtask.handover_summary.as_deref(), Some("Handover: implemented feature and tests"));
        assert_eq!(subtask.worktree_path.as_deref(), Some("C:\\fake\\worktree"));
        assert_eq!(subtask.branch.as_deref(), Some("feat/CVH-100"));
        assert_eq!(subtask.skip_worktree, Some(false));
        assert_eq!(subtask.model.as_deref(), Some("custom-model"));
        assert_eq!(subtask.priority.as_deref(), Some("high"));

        // Verify SQLite dialog items contain handover summary on parent (deduplicated on subtask dialog to prevent phantom bubbles)
        let parent_dialog = crate::db::load_dialog_items(&base, "CVH-100").expect("load parent dialog");
        assert!(parent_dialog.iter().any(|d| d.content.as_deref().unwrap_or("").contains("Handover: implemented feature and tests")));
        assert_eq!(subtask.handover_summary.as_deref(), Some("Handover: implemented feature and tests"));

        // 1b. Verify subtask deduplication protection (§ 1):
        // Calling advance_task a second time must NOT create CVH-100.2!
        let dup_attempt = internal_advance_task_to_stage(
            &base,
            "CVH-100",
            None,
            "Second handover attempt",
            None,
        )
        .await
        .expect("dedup check should succeed with guard message");
        assert!(dup_attempt.contains("уже активна"), "Must indicate subtask is already active: {}", dup_attempt);
        let loaded_dedup = load_tasks_from_disk(&base).await;
        assert_eq!(loaded_dedup.len(), 2, "Must NOT create duplicate subtask (still 2 tasks)");

        // 2. Verify zero self-abort invariant:
        // Advancing when is_autonomous_advance is true must never abort running tasks
        let moved_autonomous = internal_move_task_stage(
            &base,
            "CVH-100.1",
            "ready",
            Some("APPROVED".to_string()),
            Some("done".to_string()),
            Some("Review approved".to_string()),
            None,
            true, // is_autonomous_advance = true!
            None,
        )
        .await
        .expect("autonomous move to ready must succeed");

        assert_eq!(moved_autonomous.stage_id, "ready");
        assert_eq!(moved_autonomous.status.as_deref(), Some("done"));

        // Verify relational symmetry (§ 2.2, § 2.7 & § 2.22):
        // Parent task CVH-100 must also be automatically advanced to 'ready' with status 'done' and verdict 'APPROVED'
        let loaded_after = load_tasks_from_disk(&base).await;
        let parent_after = loaded_after.iter().find(|t| t.id == "CVH-100").unwrap();
        assert_eq!(parent_after.stage_id, "ready", "Parent must be advanced to 'ready' alongside approved child subtask");
        assert_eq!(parent_after.status.as_deref(), Some("done"), "Parent status must be 'done'");
        assert_eq!(parent_after.verdict.as_deref(), Some("APPROVED"), "Parent verdict must be 'APPROVED'");
        assert_eq!(parent_after.waiting_for_subtask_id, None, "Parent waiting_for_subtask_id must be cleared");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_subtask_advancement_5_columns_and_nesting() {
        let base = PathBuf::from("target").join(format!("cvh_5col_nesting_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let cols = vec![
            crate::config::PipelineColumn {
                id: "backlog".to_string(),
                title: "Backlog".to_string(),
                color: None,
                icon: None,
                persona: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
                is_intake: Some(true),
            },
            crate::config::PipelineColumn {
                id: "dev".to_string(),
                title: "Development".to_string(),
                color: None,
                icon: None,
                persona: Some("developer".to_string()),
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: Some(true),
                terminal: None,
                is_intake: None,
            },
            crate::config::PipelineColumn {
                id: "review".to_string(),
                title: "Review".to_string(),
                color: None,
                icon: None,
                persona: Some("reviewer".to_string()),
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: Some(true),
                terminal: None,
                is_intake: None,
            },
            crate::config::PipelineColumn {
                id: "retro".to_string(),
                title: "Retrospective".to_string(),
                color: None,
                icon: None,
                persona: Some("retro-verifier".to_string()),
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: Some(true),
                terminal: None,
                is_intake: None,
            },
            crate::config::PipelineColumn {
                id: "ready".to_string(),
                title: "Ready".to_string(),
                color: None,
                icon: None,
                persona: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: Some(true),
                is_intake: None,
            },
        ];

        let boards_cfg = crate::config::BoardsConfig {
            active_board_id: "5col-board".to_string(),
            boards: vec![crate::config::BoardDef {
                id: "5col-board".to_string(),
                name: "5-Column Board".to_string(),
                columns: cols,
            }],
        };
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed parent task in dev
        let parent = Task {
            id: "CVH-200".to_string(),
            title: "Task in 5-stage pipeline".to_string(),
            description: "Desc".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("5col-board".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: Some(true),
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };
        save_tasks_to_disk(&base, &[parent]).await.expect("save parent");

        // 1. Advance parent: creates review subtask CVH-200.1
        let adv_res = internal_advance_task(&base, "CVH-200", "Handover to review", None)
            .await
            .expect("advance to review subtask");
        assert!(adv_res.contains("УСПЕШНО: Создана подзадача CVH-200.1"));

        let loaded = load_tasks_from_disk(&base).await;
        let p1 = loaded.iter().find(|t| t.id == "CVH-200").unwrap();
        assert_eq!(p1.status.as_deref(), Some("waiting_subtask"));
        assert_eq!(p1.waiting_for_subtask_id.as_deref(), Some("CVH-200.1"));

        // 2. Advance subtask CVH-200.1 to ready (terminal column):
        // In 5-column pipeline [backlog, dev, review, retro, ready]:
        // When review subtask finishes, parent must advance past review into 'retro'!
        let move_sub = internal_move_task_stage(
            &base,
            "CVH-200.1",
            "ready",
            Some("APPROVED".to_string()),
            Some("done".to_string()),
            Some("Review approved".to_string()),
            None,
            true,
            None,
        )
        .await
        .expect("move subtask to ready");
        assert_eq!(move_sub.stage_id, "ready");
        assert_eq!(move_sub.status.as_deref(), Some("done"));

        let loaded2 = load_tasks_from_disk(&base).await;
        let p2 = loaded2.iter().find(|t| t.id == "CVH-200").unwrap();
        assert_eq!(p2.stage_id, "dev", "Parent stays pinned in dev stage");
        assert_eq!(p2.status.as_deref(), Some("waiting_subtask"), "Parent status is waiting_subtask");
        assert_eq!(p2.waiting_for_subtask_id.as_deref(), Some("CVH-200.2"), "Parent waits for CVH-200.2 in retro");

        let s2 = loaded2.iter().find(|t| t.id == "CVH-200.2").unwrap();
        assert_eq!(s2.stage_id, "retro", "Subtask CVH-200.2 is in retro stage");
        assert_eq!(s2.status.as_deref(), Some("running"));

        // 3. Now advance subtask CVH-200.2 from retro to ready:
        let move_retro_ready = internal_move_task_stage(
            &base,
            "CVH-200.2",
            "ready",
            Some("APPROVED".to_string()),
            Some("done".to_string()),
            Some("Retro completed".to_string()),
            None,
            true,
            None,
        )
        .await
        .expect("move retro subtask to ready");
        assert_eq!(move_retro_ready.stage_id, "ready");
        assert_eq!(move_retro_ready.status.as_deref(), Some("done"));

        let loaded3 = load_tasks_from_disk(&base).await;
        let p3 = loaded3.iter().find(|t| t.id == "CVH-200").unwrap();
        let s3 = loaded3.iter().find(|t| t.id == "CVH-200.1").unwrap();
        assert_eq!(p3.stage_id, "ready");
        assert_eq!(p3.status.as_deref(), Some("done"));
        assert_eq!(s3.stage_id, "ready");
        assert_eq!(s3.status.as_deref(), Some("done"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_nested_subtasks_advancement() {
        let base = PathBuf::from("target").join(format!("cvh_nested_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let cols = vec![
            crate::config::PipelineColumn {
                id: "backlog".to_string(),
                title: "Backlog".to_string(),
                color: None,
                icon: None,
                persona: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
                is_intake: Some(true),
            },
            crate::config::PipelineColumn {
                id: "dev".to_string(),
                title: "Development".to_string(),
                color: None,
                icon: None,
                persona: Some("developer".to_string()),
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
                is_intake: None,
            },
            crate::config::PipelineColumn {
                id: "ready".to_string(),
                title: "Ready".to_string(),
                color: None,
                icon: None,
                persona: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: Some(true),
                is_intake: None,
            },
        ];

        let boards_cfg = crate::config::BoardsConfig {
            active_board_id: "nest-board".to_string(),
            boards: vec![crate::config::BoardDef {
                id: "nest-board".to_string(),
                name: "Nest Board".to_string(),
                columns: cols,
            }],
        };
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Grandparent CVH-300 waiting on Parent CVH-300.1
        let gp = Task {
            id: "CVH-300".to_string(),
            title: "Grandparent Task".to_string(),
            description: "".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("nest-board".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("waiting_subtask".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: Some("CVH-300.1".to_string()),
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: Some(true),
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };

        // Parent CVH-300.1 waiting on Child CVH-300.1.1
        let parent = Task {
            id: "CVH-300.1".to_string(),
            title: "Child Subtask".to_string(),
            description: "".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("nest-board".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("waiting_subtask".to_string()),
            model: None,
            parent_id: Some("CVH-300".to_string()),
            subtask_index: Some(1),
            waiting_for_subtask_id: Some("CVH-300.1.1".to_string()),
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: Some(true),
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };

        // Child CVH-300.1.1 running in dev
        let child = Task {
            id: "CVH-300.1.1".to_string(),
            title: "Grandchild Subtask".to_string(),
            description: "".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("nest-board".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: Some("CVH-300.1".to_string()),
            subtask_index: Some(1),
            waiting_for_subtask_id: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: Some(true),
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };

        save_tasks_to_disk(&base, &[gp, parent, child]).await.expect("save tasks");

        // Advance grandchild CVH-300.1.1 to ready:
        // Must recursively advance CVH-300.1 to ready, AND CVH-300 to ready!
        let _ = internal_move_task_stage(
            &base,
            "CVH-300.1.1",
            "ready",
            Some("APPROVED".to_string()),
            Some("done".to_string()),
            None,
            None,
            true,
            None,
        )
        .await
        .expect("move grandchild to ready");

        let loaded = load_tasks_from_disk(&base).await;
        let gp_after = loaded.iter().find(|t| t.id == "CVH-300").unwrap();
        let p_after = loaded.iter().find(|t| t.id == "CVH-300.1").unwrap();
        let c_after = loaded.iter().find(|t| t.id == "CVH-300.1.1").unwrap();

        assert_eq!(c_after.stage_id, "ready");
        assert_eq!(c_after.status.as_deref(), Some("done"));

        assert_eq!(p_after.stage_id, "ready", "Parent must be advanced to ready recursively");
        assert_eq!(p_after.status.as_deref(), Some("done"));
        assert_eq!(p_after.waiting_for_subtask_id, None);

        assert_eq!(gp_after.stage_id, "ready", "Grandparent must be advanced to ready recursively");
        assert_eq!(gp_after.status.as_deref(), Some("done"));
        assert_eq!(gp_after.waiting_for_subtask_id, None);

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_race_condition_create_task_early_persistence_and_move() {
        // Simulates the exact user issue:
        // 1. Task is created and persisted immediately to disk and memory (Point 1)
        // 2. While worktree creation is still underway, user moves the task to 'dev' (Point 2)
        // 3. internal_move_task_stage succeeds immediately without 'Task not found'
        // 4. Worktree completes and updates worktree_path/branch WITHOUT overwriting stage back to 'backlog'!
        let base = PathBuf::from("target").join(format!("cvh_race_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        // Seed basic board config
        let dev_board = crate::config::BoardsConfig {
            active_board_id: "development".to_string(),
            boards: vec![crate::config::BoardDef {
                id: "development".to_string(),
                name: "Development".to_string(),
                columns: vec![
                    test_col("backlog"),
                    test_col("dev"),
                    test_col("ready"),
                ],
            }],
        };
        crate::config::save_boards_config(&base, &dev_board).await.expect("save boards");

        let task_id = "CVH-101".to_string();
        let mut new_task = Task {
            id: task_id.clone(),
            title: "Quick move task".to_string(),
            description: "Fast click to dev".to_string(),
            stage_id: "backlog".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("idle".to_string()),
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
            skip_worktree: Some(false),
            created_at: chrono::Utc::now().timestamp_millis(),
            updated_at: chrono::Utc::now().timestamp_millis(),
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };

        // Step 1: Early persistence (create_task fix)
        save_tasks_to_disk(&base, &[new_task.clone()]).await.expect("early save");

        // Step 2: User moves task to 'dev' concurrently (internal_move_task_stage)
        let moved = internal_move_task_stage(
            &base,
            &task_id,
            "dev",
            None,
            Some("running".to_string()),
            None,
            None,
            false,
            None,
        )
        .await
        .expect("move task to dev must succeed immediately!");

        assert_eq!(moved.stage_id, "dev");
        assert_eq!(moved.status.as_deref(), Some("running"));

        // Step 3: Worktree completion completes late and updates worktree_path / branch
        new_task.worktree_path = Some("worktrees/cvh-101".to_string());
        new_task.branch = Some("feat/cvh-101".to_string());

        let mut current_tasks = load_tasks_from_disk(&base).await;
        if let Some(t) = current_tasks.iter_mut().find(|t| t.id == new_task.id) {
            t.worktree_path = new_task.worktree_path.clone();
            t.branch = new_task.branch.clone();
            t.updated_at = chrono::Utc::now().timestamp_millis();
        }
        save_tasks_to_disk(&base, &current_tasks).await.expect("save post worktree");

        // Verify: The task retained stage 'dev' and status 'running' and received the worktree!
        let final_loaded = load_tasks_from_disk(&base).await;
        let final_task = final_loaded.iter().find(|t| t.id == task_id).expect("task must exist on disk");
        assert_eq!(final_task.stage_id, "dev", "Stage must NOT be reverted to backlog!");
        assert_eq!(final_task.status.as_deref(), Some("running"));
        assert_eq!(final_task.worktree_path.as_deref(), Some("worktrees/cvh-101"));
        assert_eq!(final_task.branch.as_deref(), Some("feat/cvh-101"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_run_agent_preserves_in_memory_tasks_logic() {
        // Simulates Point 3: Disk has task A, in-memory AppState has task A and task B.
        // run_agent must merge in-memory tasks so task B is NOT wiped by disk_tasks snapshot!
        let base = PathBuf::from("target").join(format!("cvh_run_agent_mem_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let task_a = Task {
            id: "CVH-001".to_string(),
            title: "Task A".to_string(),
            description: "A".to_string(),
            stage_id: "backlog".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("idle".to_string()),
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

        let task_b = Task {
            id: "CVH-002".to_string(),
            title: "Task B (In-Memory)".to_string(),
            description: "B".to_string(),
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

        // Disk has only Task A
        save_tasks_to_disk(&base, &[task_a.clone()]).await.expect("save task_a to disk");

        // Memory has Task A and Task B
        let mem_tasks = vec![task_a.clone(), task_b.clone()];

        // run_agent task resolution logic:
        let mut tasks_to_use = load_tasks_from_disk(&base).await;
        let initial_disk_count = tasks_to_use.len();
        for mem_task in &mem_tasks {
            if !tasks_to_use.iter().any(|t| t.id == mem_task.id) {
                tasks_to_use.push(mem_task.clone());
            }
        }

        // Verify task B is resolved
        let found_b = tasks_to_use.iter().find(|t| t.id == "CVH-002");
        assert!(found_b.is_some(), "Task B must be found in merged task list!");

        // Verify disk is updated with in-memory task
        if tasks_to_use.len() > initial_disk_count {
            save_tasks_to_disk(&base, &tasks_to_use).await.expect("flush memory tasks to disk");
        }

        let disk_after = load_tasks_from_disk(&base).await;
        assert_eq!(disk_after.len(), 2, "Task B must now be safely persisted to disk!");
        assert!(disk_after.iter().any(|t| t.id == "CVH-002"));

        let _ = tokio::fs::remove_dir_all(&base).await;
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
    async fn test_move_task_to_dev_autorun_and_running_status() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_dev_autorun_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;

        // Scaffold minimal boards.json with development board containing backlog and dev (persona cvh-dev)
        let boards_cfg = serde_json::json!({
            "active_board_id": "development",
            "boards": [
                {
                    "id": "development",
                    "name": "Разработка",
                    "columns": [
                        { "id": "backlog", "title": "Бэклог", "auto_run": false },
                        { "id": "dev", "title": "В работе", "persona": "cvh-dev", "auto_run": true }
                    ]
                }
            ]
        });
        let boards_path = tmp.join(".cvh").join("boards.json");
        let _ = tokio::fs::create_dir_all(tmp.join(".cvh")).await;
        tokio::fs::write(&boards_path, serde_json::to_string_pretty(&boards_cfg).unwrap())
            .await
            .expect("write boards.json");

        // Create task in backlog
        let task = internal_create_task(
            &tmp,
            "Dev Task".to_string(),
            "Move to dev test".to_string(),
            "backlog".to_string(),
            Some("development".to_string()),
            Some(true),
            None,
            Some("CVH-005".to_string()),
            None,
            None,
            None,
        )
        .await
        .expect("create task");

        assert_eq!(task.stage_id, "backlog");
        assert_ne!(task.status.as_deref(), Some("running"));

        // Move task to 'dev' column
        let moved = internal_move_task_stage(
            &tmp,
            "CVH-005",
            "dev",
            None,
            None,
            None,
            None,
            false,
            Some("development".to_string()),
        )
        .await
        .expect("move task to dev must succeed");

        assert_eq!(moved.stage_id, "dev");
        assert_eq!(
            moved.status.as_deref(),
            Some("running"),
            "Task moving to stage with persona 'cvh-dev' must automatically enter 'running' status!"
        );

        let _ = tokio::fs::remove_dir_all(&tmp).await;
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
    async fn test_move_task_stage_case_insensitive_and_trimmed() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_case_insensitive_test_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;

        let dev_board = crate::config::BoardsConfig {
            active_board_id: "development".to_string(),
            boards: vec![crate::config::BoardDef {
                id: "development".to_string(),
                name: "Development".to_string(),
                columns: vec![
                    test_col("backlog"),
                    test_col("dev"),
                    test_col("ready"),
                ],
            }],
        };
        crate::config::save_boards_config(&tmp, &dev_board).await.expect("save boards");

        let task = Task {
            id: "CVH-001".to_string(),
            title: "Case Insensitive Task".to_string(),
            description: "Test".to_string(),
            stage_id: "backlog".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("idle".to_string()),
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
            skip_worktree: Some(true),
            created_at: chrono::Utc::now().timestamp_millis(),
            updated_at: chrono::Utc::now().timestamp_millis(),
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };
        save_tasks_to_disk(&tmp, &[task]).await.expect("save task");

        // Move using lowercase with spaces "  cvh-001  "
        let moved = internal_move_task_stage(
            &tmp,
            "  cvh-001  ",
            "dev",
            None,
            None,
            None,
            None,
            false,
            Some("development".to_string()),
        )
        .await
        .expect("move task with lowercase and whitespace must succeed!");

        assert_eq!(moved.id, "CVH-001");
        assert_eq!(moved.stage_id, "dev");

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }


    #[tokio::test]
    async fn test_terminal_task_clears_waiting_for_subtask_and_heal_tasks_cleans_stale_subtasks() {
        let tmp = PathBuf::from("target").join(format!("cvh_heal_subtasks_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&tmp).await.expect("test dir");

        let bcfg = crate::config::BoardsConfig {
            active_board_id: "development".to_string(),
            boards: vec![crate::config::BoardDef {
                id: "development".to_string(),
                name: "Development".to_string(),
                columns: vec![
                    crate::config::PipelineColumn {
                        id: "backlog".to_string(),
                        title: "Backlog".to_string(),
                        color: None,
                        icon: None,
                        persona: None,
                        on_enter_script: None,
                        context_window: None,
                        model_override: None,
                        reasoning_effort: None,
                        temperature: None,
                        gatekeeper_sensor: None,
                        allowed_tools: None,
                        hooks: HashMap::new(),
                        routes: None,
                        auto_run: None,
                        terminal: None,
                        is_intake: Some(true),
                    },
                    crate::config::PipelineColumn {
                        id: "dev".to_string(),
                        title: "Dev".to_string(),
                        color: None,
                        icon: None,
                        persona: Some("developer".to_string()),
                        on_enter_script: None,
                        context_window: None,
                        model_override: None,
                        reasoning_effort: None,
                        temperature: None,
                        gatekeeper_sensor: None,
                        allowed_tools: None,
                        hooks: HashMap::new(),
                        routes: None,
                        auto_run: Some(true),
                        terminal: None,
                        is_intake: None,
                    },
                    crate::config::PipelineColumn {
                        id: "ready".to_string(),
                        title: "Ready".to_string(),
                        color: None,
                        icon: None,
                        persona: None,
                        on_enter_script: None,
                        context_window: None,
                        model_override: None,
                        reasoning_effort: None,
                        temperature: None,
                        gatekeeper_sensor: None,
                        allowed_tools: None,
                        hooks: HashMap::new(),
                        routes: None,
                        auto_run: None,
                        terminal: Some(true),
                        is_intake: None,
                    },
                ],
            }],
        };

        let mut tasks = vec![
            Task {
                id: "CVH-500".to_string(),
                title: "Parent in ready".to_string(),
                description: String::new(),
                stage_id: "ready".to_string(),
                board_id: Some("development".to_string()),
                worktree_path: None,
                branch: None,
                verdict: None,
                custom_flow: None,
                status: Some("done".to_string()),
                model: None,
                parent_id: None,
                subtask_index: None,
                waiting_for_subtask_id: Some("CVH-500.1".to_string()),
                handover_summary: None,
                cover_image: None,
                attachments: None,
                todos: None,
                priority: None,
                tags: None,
                current_tokens: None,
                skip_worktree: Some(true),
                created_at: 0,
                updated_at: 0,
                stage_entered_at: None,
            completed_at: None,
                position: None,
                hook_states: None,
            },
            Task {
                id: "CVH-500.1".to_string(),
                title: "Completed subtask".to_string(),
                description: String::new(),
                stage_id: "dev".to_string(),
                board_id: Some("development".to_string()),
                worktree_path: None,
                branch: None,
                verdict: None,
                custom_flow: None,
                status: Some("done".to_string()),
                model: None,
                parent_id: Some("CVH-500".to_string()),
                subtask_index: Some(1),
                waiting_for_subtask_id: None,
                handover_summary: None,
                cover_image: None,
                attachments: None,
                todos: None,
                priority: None,
                tags: None,
                current_tokens: None,
                skip_worktree: Some(true),
                created_at: 0,
                updated_at: 0,
                stage_entered_at: None,
            completed_at: None,
                position: None,
                hook_states: None,
            },
            Task {
                id: "CVH-501".to_string(),
                title: "Parent with completed child".to_string(),
                description: String::new(),
                stage_id: "dev".to_string(),
                board_id: Some("development".to_string()),
                worktree_path: None,
                branch: None,
                verdict: None,
                custom_flow: None,
                status: Some("waiting_subtask".to_string()),
                model: None,
                parent_id: None,
                subtask_index: None,
                waiting_for_subtask_id: Some("CVH-501.1".to_string()),
                handover_summary: None,
                cover_image: None,
                attachments: None,
                todos: None,
                priority: None,
                tags: None,
                current_tokens: None,
                skip_worktree: Some(true),
                created_at: 0,
                updated_at: 0,
                stage_entered_at: None,
            completed_at: None,
                position: None,
                hook_states: None,
            },
            Task {
                id: "CVH-501.1".to_string(),
                title: "Subtask 501.1".to_string(),
                description: String::new(),
                stage_id: "dev".to_string(),
                board_id: Some("development".to_string()),
                worktree_path: None,
                branch: None,
                verdict: None,
                custom_flow: None,
                status: Some("done".to_string()),
                model: None,
                parent_id: Some("CVH-501".to_string()),
                subtask_index: Some(1),
                waiting_for_subtask_id: None,
                handover_summary: None,
                cover_image: None,
                attachments: None,
                todos: None,
                priority: None,
                tags: None,
                current_tokens: None,
                skip_worktree: Some(true),
                created_at: 0,
                updated_at: 0,
                stage_entered_at: None,
            completed_at: None,
                position: None,
                hook_states: None,
            },
        ];

        // 1. Verify heal_tasks cleans up waiting_for_subtask_id
        let healed = heal_tasks(&mut tasks, &bcfg);
        assert!(healed, "heal_tasks must return true when tasks are modified");

        assert_eq!(tasks[0].waiting_for_subtask_id, None, "Terminal parent must have waiting_for_subtask_id None");
        assert_eq!(tasks[0].status.as_deref(), Some("done"));

        assert_eq!(tasks[2].waiting_for_subtask_id, None, "Parent with completed child must have waiting_for_subtask_id cleared");
        assert_eq!(tasks[2].status.as_deref(), Some("idle"), "Status must revert from waiting_subtask to idle");

        // 2. Verify subtask advance to terminal via internal_advance_task_to_stage clears parent waiting_for_subtask_id
        crate::config::save_boards_config(&tmp, &bcfg).await.expect("save boards");
        let parent_active = Task {
            id: "CVH-600".to_string(),
            title: "Active Parent".to_string(),
            description: String::new(),
            stage_id: "dev".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("waiting_subtask".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: Some("CVH-600.1".to_string()),
            handover_summary: None,
            cover_image: None,
            attachments: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: Some(true),
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };
        let child_active = Task {
            id: "CVH-600.1".to_string(),
            title: "Child in dev".to_string(),
            description: String::new(),
            stage_id: "dev".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: Some("CVH-600".to_string()),
            subtask_index: Some(1),
            waiting_for_subtask_id: None,
            handover_summary: None,
            cover_image: None,
            attachments: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: Some(true),
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };
        save_tasks_to_disk(&tmp, &[parent_active, child_active]).await.expect("save active tasks");

        let adv_res = internal_advance_task_to_stage(&tmp, "CVH-600.1", Some("ready"), "Child finished", None)
            .await
            .expect("advance child to ready");
        assert!(adv_res.contains("УСПЕШНО"));

        let loaded = load_tasks_from_disk(&tmp).await;
        let parent_after = loaded.iter().find(|t| t.id == "CVH-600").expect("parent CVH-600");
        assert_eq!(parent_after.stage_id, "ready");
        assert_eq!(parent_after.status.as_deref(), Some("done"));
        assert!(parent_after.completed_at.is_some(), "Parent completed_at must be Some when completed");
        assert_eq!(parent_after.waiting_for_subtask_id, None, "Parent waiting_for_subtask_id must be None after advance to ready");

        let child_after = loaded.iter().find(|t| t.id == "CVH-600.1").expect("child CVH-600.1");
        assert_eq!(child_after.status.as_deref(), Some("done"));
        assert!(child_after.completed_at.is_some(), "Child completed_at must be Some when completed");

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }


    #[tokio::test]
    async fn test_move_task_to_plan_stage_autorun_and_running_status() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_plan_autorun_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;

        // Scaffold minimal boards.json with development board containing backlog, plan (persona planner), dev, and ready
        let boards_cfg = serde_json::json!({
            "active_board_id": "development",
            "boards": [
                {
                    "id": "development",
                    "name": "Разработка",
                    "columns": [
                        { "id": "backlog", "title": "Бэклог", "auto_run": false },
                        { "id": "plan", "title": "Планирование", "persona": "planner", "auto_run": true },
                        { "id": "dev", "title": "В работе", "persona": "cvh-dev", "auto_run": true },
                        { "id": "ready", "title": "Готово", "terminal": true }
                    ]
                }
            ]
        });
        let boards_path = tmp.join(".cvh").join("boards.json");
        let _ = tokio::fs::create_dir_all(tmp.join(".cvh")).await;
        tokio::fs::write(&boards_path, serde_json::to_string_pretty(&boards_cfg).unwrap())
            .await
            .expect("write boards.json");

        // Create task in backlog
        let task = internal_create_task(
            &tmp,
            "Plan Task".to_string(),
            "Move to plan test".to_string(),
            "backlog".to_string(),
            Some("development".to_string()),
            Some(true),
            None,
            Some("CVH-101".to_string()),
            None,
            None,
            None,
        )
        .await
        .expect("create task");

        assert_eq!(task.stage_id, "backlog");
        assert_ne!(task.status.as_deref(), Some("running"));

        // Move task to 'plan' column
        let moved = internal_move_task_stage(
            &tmp,
            "CVH-101",
            "plan",
            None,
            None,
            None,
            None,
            false,
            Some("development".to_string()),
        )
        .await
        .expect("move task to plan must succeed");

        assert_eq!(moved.stage_id, "plan", "Task must move to 'plan'");
        assert_ne!(moved.stage_id, "ready", "Task must NEVER falsely move to 'ready'");
        assert_eq!(
            moved.status.as_deref(),
            Some("running"),
            "Task moving to stage with persona 'planner' and auto_run=true must enter 'running' status!"
        );
        assert_ne!(moved.status.as_deref(), Some("done"), "Task status must NOT be 'done'");

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }


    #[tokio::test]
    async fn test_completed_at_lifecycle_on_stage_transitions() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_completed_at_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;
        let _ = tokio::fs::create_dir_all(tmp.join(".cvh")).await;

        let boards_cfg = serde_json::json!({
            "active_board_id": "development",
            "boards": [
                {
                    "id": "development",
                    "name": "Разработка",
                    "columns": [
                        { "id": "backlog", "title": "Бэклог", "auto_run": false },
                        { "id": "dev", "title": "В работе", "auto_run": false },
                        { "id": "ready", "title": "Готово", "terminal": true }
                    ]
                }
            ]
        });
        tokio::fs::write(
            tmp.join(".cvh").join("boards.json"),
            serde_json::to_string_pretty(&boards_cfg).unwrap(),
        )
        .await
        .expect("write boards.json");

        let task = internal_create_task(
            &tmp,
            "CompletedAt Lifecycle Task".to_string(),
            "Testing completed_at tracking".to_string(),
            "dev".to_string(),
            Some("development".to_string()),
            Some(true),
            None,
            Some("CVH-999".to_string()),
            None,
            None,
            None,
        )
        .await
        .expect("create task");

        assert_eq!(task.completed_at, None, "Task in dev must have completed_at None");

        // 1. Move to terminal 'ready'
        let moved_to_ready = internal_move_task_stage(
            &tmp,
            "CVH-999",
            "ready",
            None,
            None,
            None,
            None,
            false,
            Some("development".to_string()),
        )
        .await
        .expect("move to ready");

        assert_eq!(moved_to_ready.stage_id, "ready");
        assert_eq!(moved_to_ready.status.as_deref(), Some("done"));
        assert!(moved_to_ready.completed_at.is_some(), "Task in ready must have completed_at timestamp");

        // 2. Move from 'ready' back to 'dev' (rework)
        let moved_back_to_dev = internal_move_task_stage(
            &tmp,
            "CVH-999",
            "dev",
            None,
            None,
            None,
            None,
            false,
            Some("development".to_string()),
        )
        .await
        .expect("move back to dev");

        assert_eq!(moved_back_to_dev.stage_id, "dev");
        assert_eq!(moved_back_to_dev.completed_at, None, "Task moving out of terminal must reset completed_at to None");

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }


    #[tokio::test]
    async fn test_advance_task_sets_subtask_completed_at() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_subtask_completed_at_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;
        let _ = tokio::fs::create_dir_all(tmp.join(".cvh")).await;

        let boards_cfg = serde_json::json!({
            "active_board_id": "development",
            "boards": [
                {
                    "id": "development",
                    "name": "Разработка",
                    "columns": [
                        { "id": "backlog", "title": "Бэклог", "auto_run": false },
                        { "id": "plan", "title": "План", "auto_run": false },
                        { "id": "dev", "title": "В работе", "auto_run": false },
                        { "id": "ready", "title": "Готово", "terminal": true }
                    ]
                }
            ]
        });
        tokio::fs::write(
            tmp.join(".cvh").join("boards.json"),
            serde_json::to_string_pretty(&boards_cfg).unwrap(),
        )
        .await
        .expect("write boards.json");

        let _parent = internal_create_task(
            &tmp,
            "Parent Subtask CompletedAt Test".to_string(),
            "Testing subtask completed_at".to_string(),
            "plan".to_string(),
            Some("development".to_string()),
            Some(true),
            None,
            Some("CVH-777".to_string()),
            None,
            None,
            None,
        )
        .await
        .expect("create parent task");

        let sub1 = internal_create_subtask(
            &tmp,
            "CVH-777",
            Some("plan"),
            Some("Parent Subtask CompletedAt Test (План)"),
            Some("Planning..."),
            None,
        )
        .await
        .expect("create sub1");

        assert_eq!(sub1.completed_at, None);

        // Advance subtask to dev (non-terminal transition)
        let adv_res = internal_advance_task_to_stage(&tmp, &sub1.id, Some("dev"), "Plan completed", None)
            .await
            .expect("advance to dev");
        assert!(adv_res.contains("УСПЕШНО"));

        let loaded = load_tasks_from_disk(&tmp).await;
        let sub1_after = loaded.iter().find(|t| t.id == sub1.id).expect("sub1 loaded");
        assert_eq!(sub1_after.status.as_deref(), Some("done"));
        assert!(sub1_after.completed_at.is_some(), "Subtask 1 completed_at must be populated after advance");

        // Find the created subtask 2 in dev
        let sub2 = loaded.iter().find(|t| t.parent_id.as_deref() == Some("CVH-777") && t.stage_id == "dev").expect("sub2");
        assert_eq!(sub2.completed_at, None);

        // Advance subtask 2 to ready (terminal transition)
        let adv_terminal = internal_advance_task_to_stage(&tmp, &sub2.id, Some("ready"), "Implementation finished", None)
            .await
            .expect("advance to ready");
        assert!(adv_terminal.contains("УСПЕШНО"));

        let loaded_final = load_tasks_from_disk(&tmp).await;
        let sub2_final = loaded_final.iter().find(|t| t.id == sub2.id).expect("sub2 final");
        assert_eq!(sub2_final.status.as_deref(), Some("done"));
        assert!(sub2_final.completed_at.is_some(), "Subtask 2 completed_at must be populated after terminal advance");

        let parent_final = loaded_final.iter().find(|t| t.id == "CVH-777").expect("parent final");
        assert_eq!(parent_final.status.as_deref(), Some("done"));
        assert!(parent_final.completed_at.is_some(), "Parent task completed_at must be populated after terminal advance");

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }


    #[tokio::test]
    async fn test_advance_task_auto_completes_unfinished_todos() {
        let base = PathBuf::from("target").join(format!("cvh_auto_complete_todos_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let dev_board = crate::config::BoardsConfig {
            active_board_id: "development".to_string(),
            boards: vec![crate::config::BoardDef {
                id: "development".to_string(),
                name: "Development".to_string(),
                columns: vec![
                    test_col("backlog"),
                    test_col("dev"),
                    test_col("ready"),
                ],
            }],
        };
        crate::config::save_boards_config(&base, &dev_board).await.expect("save boards");

        let task_id = "CVH-TODO-1";
        let initial_todos = vec![
            crate::agent::TodoItem {
                id: "todo-1".to_string(),
                content: "Implement feature".to_string(),
                status: "in_progress".to_string(),
            },
            crate::agent::TodoItem {
                id: "todo-2".to_string(),
                content: "Run test suite".to_string(),
                status: "pending".to_string(),
            },
        ];

        let task = Task {
            id: task_id.to_string(),
            title: "Task with unfinished todos".to_string(),
            description: "Testing fallback auto-completion of todos".to_string(),
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
            todos: Some(initial_todos.clone()),
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: Some(true),
            created_at: chrono::Utc::now().timestamp_millis(),
            updated_at: chrono::Utc::now().timestamp_millis(),
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };

        save_tasks_to_disk(&base, &[task]).await.expect("save task");
        crate::db::db_save_todos(&base, task_id, &initial_todos).expect("save initial todos to db");

        // Verify todos before advance
        let pre_task = crate::db::db_get_task(&base, task_id).expect("load task").expect("task exists");
        let pre_todos = pre_task.todos.expect("pre todos exist");
        assert_eq!(pre_todos.len(), 2);
        assert_eq!(pre_todos[0].status, "in_progress");
        assert_eq!(pre_todos[1].status, "pending");

        // Advance task to next stage (from dev to ready)
        let adv_res = internal_advance_task_to_stage(
            &base,
            task_id,
            Some("ready"),
            "Completed all work and verified",
            None,
        )
        .await
        .expect("internal_advance_task_to_stage must succeed");

        assert!(adv_res.contains("УСПЕШНО"));

        // Verify task in memory/disk after advance
        let disk_tasks = load_tasks_from_disk(&base).await;
        let advanced_task = disk_tasks.iter().find(|t| t.id == task_id).expect("task on disk");
        assert_eq!(advanced_task.stage_id, "ready");
        let disk_todos = advanced_task.todos.as_ref().expect("disk todos present");
        assert_eq!(disk_todos.len(), 2);
        assert_eq!(disk_todos[0].status, "completed");
        assert_eq!(disk_todos[1].status, "completed");

        // Verify task and todos directly in SQLite
        let db_task = crate::db::db_get_task(&base, task_id).expect("db_get_task").expect("task in db");
        let db_todos = db_task.todos.as_ref().expect("db todos present");
        assert_eq!(db_todos.len(), 2);
        assert_eq!(db_todos[0].status, "completed");
        assert_eq!(db_todos[1].status, "completed");

        // Also test idempotency: calling auto_complete_task_todos again returns false
        let mut task_clone = db_task.clone();
        let changed_again = auto_complete_task_todos(&base, &mut task_clone, None);
        assert!(!changed_again, "auto_complete_task_todos must be idempotent and return false when already completed");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_stage_entered_at_persistence_and_stage_transitions() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_stage_entered_at_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&base).await;
        let _ = tokio::fs::create_dir_all(base.join(".cvh")).await;

        let boards_cfg = serde_json::json!({
            "active_board_id": "development",
            "boards": [
                {
                    "id": "development",
                    "name": "Разработка",
                    "columns": [
                        { "id": "backlog", "title": "Бэклог", "auto_run": false },
                        { "id": "dev", "title": "В работе", "auto_run": false },
                        { "id": "ready", "title": "Готово", "terminal": true }
                    ]
                }
            ]
        });
        tokio::fs::write(
            base.join(".cvh").join("boards.json"),
            serde_json::to_string_pretty(&boards_cfg).unwrap(),
        )
        .await
        .expect("write boards.json");

        let task_id = "CVH-998";
        let task = internal_create_task(
            &base,
            "Timer Test Task".to_string(),
            "Testing stage_entered_at persistence".to_string(),
            "backlog".to_string(),
            Some("development".to_string()),
            Some(true),
            None,
            Some(task_id.to_string()),
            None,
            None,
            None,
        )
        .await
        .expect("create task");

        // 1. Initial creation must set stage_entered_at
        assert!(task.stage_entered_at.is_some(), "New task must have stage_entered_at set");
        let initial_stage_entered_at = task.stage_entered_at.unwrap();
        assert!(initial_stage_entered_at > 0);

        // 2. Direct SQLite load must match
        let db_task = crate::db::db_get_task(&base, task_id)
            .expect("db_get_task")
            .expect("task must exist in SQLite");
        assert_eq!(db_task.stage_entered_at, Some(initial_stage_entered_at));

        // 3. save_tasks from client (even with stage_entered_at: None) must preserve existing timestamp if stage unchanged
        let mut client_task = db_task.clone();
        client_task.stage_entered_at = None;
        client_task.title = "Updated Title".to_string();
        internal_save_tasks(&base, vec![client_task], None)
            .await
            .expect("save_tasks");

        let db_after_save = crate::db::db_get_task(&base, task_id)
            .expect("db_get_task")
            .expect("task exists");
        assert_eq!(
            db_after_save.stage_entered_at,
            Some(initial_stage_entered_at),
            "save_tasks must preserve stage_entered_at when stage has not changed"
        );
        assert_eq!(db_after_save.title, "Updated Title");

        // 4. Moving to a new stage ('dev') MUST update stage_entered_at
        tokio::time::sleep(tokio::time::Duration::from_millis(15)).await;
        let moved_task = internal_move_task_stage(
            &base,
            task_id,
            "dev",
            None,
            Some("running".to_string()),
            None,
            None,
            false,
            None,
        )
        .await
        .expect("move to dev");

        assert_eq!(moved_task.stage_id, "dev");
        assert!(moved_task.stage_entered_at.is_some());
        let dev_stage_entered_at = moved_task.stage_entered_at.unwrap();
        assert!(
            dev_stage_entered_at > initial_stage_entered_at,
            "stage_entered_at must be updated when stage changes"
        );

        // 5. Updating status within the same stage ('dev') must NOT reset stage_entered_at
        tokio::time::sleep(tokio::time::Duration::from_millis(15)).await;
        let same_stage_update = internal_move_task_stage(
            &base,
            task_id,
            "dev",
            None,
            Some("idle".to_string()),
            None,
            None,
            false,
            None,
        )
        .await
        .expect("update status in same stage");

        assert_eq!(same_stage_update.stage_id, "dev");
        assert_eq!(
            same_stage_update.stage_entered_at,
            Some(dev_stage_entered_at),
            "stage_entered_at must NOT change when staying in the same stage"
        );

        let _ = tokio::fs::remove_dir_all(&base).await;
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


    #[tokio::test]
    async fn test_foreign_project_tasks_isolation_and_purge() {
        let base1 = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join(format!("cvh_foreign_proj1_{}", uuid::Uuid::new_v4()));
        let base2 = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join(format!("cvh_foreign_proj2_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base1.join(".cvh")).await.expect("test dir 1");
        tokio::fs::create_dir_all(base2.join(".cvh")).await.expect("test dir 2");

        // Create tasks in proj1: one local, one alien parent, one alien child (pointing to proj2 worktree)
        let local_task = Task {
            id: "CVH-001".to_string(),
            title: "Local Task in Proj1".to_string(),
            description: "".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: Some(base1.join("worktrees").join("CVH-001").to_string_lossy().to_string()),
            branch: Some("feat/CVH-001".to_string()),
            verdict: None,
            custom_flow: None,
            status: Some("idle".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: None,
            created_at: 100,
            updated_at: 100,
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };

        let foreign_parent = Task {
            id: "CVH-099".to_string(),
            title: "Alien Parent Task".to_string(),
            description: "".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("idle".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: Some("CVH-099.1".to_string()),
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: None,
            created_at: 200,
            updated_at: 200,
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };

        let foreign_child = Task {
            id: "CVH-099.1".to_string(),
            title: "Alien Child Subtask in Proj2".to_string(),
            description: "".to_string(),
            stage_id: "review".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: Some(base2.join("worktrees").join("CVH-099.1").to_string_lossy().to_string()),
            branch: Some("feat/CVH-099".to_string()),
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: Some("CVH-099".to_string()),
            subtask_index: Some(1),
            waiting_for_subtask_id: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: None,
            created_at: 300,
            updated_at: 300,
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };

        // Directly insert foreign tasks into Proj1's SQLite to simulate contamination
        crate::db::db_upsert_all_tasks(&base1, &[local_task.clone(), foreign_parent.clone(), foreign_child.clone()]).expect("upsert tasks into db");

        // Verify contaminated DB has 3 tasks
        let contaminated = crate::db::db_get_all_tasks(&base1).expect("read db");
        assert_eq!(contaminated.len(), 3);

        // load_tasks_from_disk on base1 must detect and purge the foreign tasks!
        let sanitized = load_tasks_from_disk(&base1).await;
        assert_eq!(sanitized.len(), 1, "Must contain ONLY local tasks after auto-purge");
        assert_eq!(sanitized[0].id, "CVH-001");

        // Verify DB was cleaned up (alien tasks deleted)
        let db_after = crate::db::db_get_all_tasks(&base1).expect("read db after");
        assert_eq!(db_after.len(), 1);
        assert_eq!(db_after[0].id, "CVH-001");

        let _ = tokio::fs::remove_dir_all(&base1).await;
        let _ = tokio::fs::remove_dir_all(&base2).await;
    }



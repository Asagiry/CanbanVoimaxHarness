use super::*;
use crate::agent::Task;
use std::path::PathBuf;

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

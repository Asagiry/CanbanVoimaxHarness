use super::*;
use crate::config::ToolPolicyConfig;
use std::collections::HashMap;
use std::sync::Arc;

    #[tokio::test]
    async fn test_advance_and_move_task_tool_dispatch() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_adv_tool_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let b_cols = vec![
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
            active_board_id: "dev-board".to_string(),
            boards: vec![
                crate::config::BoardDef {
                    id: "dev-board".to_string(),
                    name: "Development".to_string(),
                    columns: b_cols,
                },
            ],
        };
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        let task = Task {
            id: "CVH-050".to_string(),
            title: "Move Task Test".to_string(),
            description: "Testing move task aliasing and target_stage".to_string(),
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
            task_id: Some("CVH-050"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // 1. advance_task without summary -> uses default summary and advances to next column (review)
        let adv_res = execute_tool(&tool_ctx, "advance_task", &serde_json::json!({}))
            .await
            .expect("advance_task with default summary");
        assert!(adv_res.contains("УСПЕШНО"));

        let tasks_after_adv = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t = tasks_after_adv.iter().find(|t| t.id == "CVH-050").unwrap();
        assert_eq!(t.stage_id, "dev");
        assert_eq!(t.status.as_deref(), Some("waiting_subtask"));
        assert_eq!(t.waiting_for_subtask_id.as_deref(), Some("CVH-050.1"));

        let sub = tasks_after_adv.iter().find(|t| t.id == "CVH-050.1").unwrap();
        assert_eq!(sub.stage_id, "review");

        // 2. move_task with target_stage: "ready" and explicit summary
        let move_res = execute_tool(&tool_ctx, "move_task", &serde_json::json!({
            "target_stage": "ready",
            "summary": "Completed code review, verified clean build."
        }))
        .await
        .expect("move_task to ready");
        assert!(move_res.contains("УСПЕШНО"));

        let tasks_after_move = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t_ready = tasks_after_move.iter().find(|t| t.id == "CVH-050").unwrap();
        assert_eq!(t_ready.stage_id, "ready");
        assert_eq!(t_ready.status.as_deref(), Some("done"));
        assert_eq!(t_ready.handover_summary.as_deref(), Some("Completed code review, verified clean build."));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_pipeline_advance_task_sequential_and_boundary_error() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_adv_seq_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let (boards_cfg, _) = make_test_board(&base);
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed task in backlog
        let task = make_test_task("CVH-060", "backlog", "test-board");
        crate::commands::tasks::save_tasks_to_disk(&base, &[task]).await.expect("save task");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("backlog"),
            role: Some("developer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-060"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // 1. Advance from backlog -> creates dev subtask CVH-060.1
        let res1 = execute_tool(&tool_ctx, "advance_task", &serde_json::json!({ "summary": "Started dev work" }))
            .await
            .expect("advance backlog -> dev");
        assert!(res1.contains("УСПЕШНО"), "got: {}", res1);

        let tasks1 = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t1 = tasks1.iter().find(|t| t.id == "CVH-060").expect("find task");
        assert_eq!(t1.stage_id, "backlog", "Parent must remain in backlog");
        assert_eq!(t1.status.as_deref(), Some("waiting_subtask"));
        assert_eq!(t1.waiting_for_subtask_id.as_deref(), Some("CVH-060.1"));

        let sub1 = tasks1.iter().find(|t| t.id == "CVH-060.1").expect("find dev subtask");
        assert_eq!(sub1.stage_id, "dev", "Subtask must be in dev stage");

        // 2. Advance subtask CVH-060.1 -> creates review subtask CVH-060.2 (§ 2.2)
        let res2 = execute_tool(&tool_ctx, "advance_task", &serde_json::json!({ "task_id": "CVH-060.1", "summary": "Implemented feature" }))
            .await
            .expect("advance dev -> review");
        assert!(res2.contains("УСПЕШНО"), "got: {}", res2);

        let tasks2 = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t2 = tasks2.iter().find(|t| t.id == "CVH-060").expect("find parent task");
        assert_eq!(t2.stage_id, "backlog", "Parent must remain in backlog");
        assert_eq!(t2.status.as_deref(), Some("waiting_subtask"));
        assert_eq!(t2.waiting_for_subtask_id.as_deref(), Some("CVH-060.2"));

        let sub1_done = tasks2.iter().find(|t| t.id == "CVH-060.1").expect("find dev subtask");
        assert_eq!(sub1_done.status.as_deref(), Some("done"));

        let sub2 = tasks2.iter().find(|t| t.id == "CVH-060.2").expect("find review subtask");
        assert_eq!(sub2.stage_id, "review", "Subtask must be in review stage");

        // 3. Advance subtask CVH-060.2 from review -> ready (terminal)
        let res3 = execute_tool(&tool_ctx, "advance_task", &serde_json::json!({ "task_id": "CVH-060.2", "summary": "Review approved" }))
            .await
            .expect("advance review -> ready");
        assert!(res3.contains("УСПЕШНО"), "got: {}", res3);

        let tasks3 = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t3 = tasks3.iter().find(|t| t.id == "CVH-060").expect("find parent task");
        assert_eq!(t3.stage_id, "ready");
        assert_eq!(t3.status.as_deref(), Some("done"));

        let sub2_done = tasks3.iter().find(|t| t.id == "CVH-060.2").expect("find subtask");
        assert_eq!(sub2_done.status.as_deref(), Some("done"));

        // 4. Boundary error: Advance when already in terminal ready stage must fail
        let res_err = execute_tool(&tool_ctx, "advance_task", &serde_json::json!({ "task_id": "CVH-060" }))
            .await
            .expect_err("advance from terminal column must fail");
        assert!(res_err.to_string().contains("уже находится на последнем этапе пайплайна"), "got: {}", res_err);

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_pipeline_move_task_explicit_target_stage() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_move_tgt_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let (boards_cfg, _) = make_test_board(&base);
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed task in dev
        let task = make_test_task("CVH-061", "dev", "test-board");
        crate::commands::tasks::save_tasks_to_disk(&base, &[task]).await.expect("save task");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-061"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // Move directly from dev to ready via target_stage
        let res = execute_tool(&tool_ctx, "move_task", &serde_json::json!({
            "target_stage": "ready",
            "summary": "Fast-tracked verification"
        }))
        .await
        .expect("move_task with target_stage");
        assert!(res.contains("УСПЕШНО"));

        let tasks = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t = tasks.iter().find(|t| t.id == "CVH-061").expect("find task");
        assert_eq!(t.stage_id, "ready");
        assert_eq!(t.status.as_deref(), Some("done"));
        assert_eq!(t.handover_summary.as_deref(), Some("Fast-tracked verification"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_pipeline_reject_task_moves_back_and_sets_changes_requested() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_reject_ok_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let (boards_cfg, _) = make_test_board(&base);
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed task in 'review' (stage index 2: backlog=0, dev=1, review=2)
        let task = make_test_task("CVH-062", "review", "test-board");
        crate::commands::tasks::save_tasks_to_disk(&base, &[task]).await.expect("save task");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("review"),
            role: Some("reviewer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-062"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // Reject task with reason
        let res = execute_tool(&tool_ctx, "reject_task", &serde_json::json!({
            "reason": "Clippy sensor emitted 3 warnings"
        }))
        .await
        .expect("reject_task from review");

        assert!(res.contains("ОТКЛОНЕНО"), "got: {}", res);
        assert!(res.contains("dev"), "got: {}", res);

        let tasks = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t = tasks.iter().find(|t| t.id == "CVH-062").expect("find task");
        assert_eq!(t.stage_id, "dev", "Must move back to dev");
        assert_eq!(t.status.as_deref(), Some("idle"), "Status must be reset to idle");
        assert_eq!(t.verdict.as_deref(), Some("CHANGES_REQUESTED"), "Verdict must be CHANGES_REQUESTED");
        assert_eq!(t.handover_summary.as_deref(), Some("ОТКЛОНЕНО РЕВЬЮ: Clippy sensor emitted 3 warnings"));

        // Verify reject with explicit task_id in args when tool_ctx.task_id is None
        let tool_ctx_no_id = ToolContext {
            task_id: None,
            ..tool_ctx
        };
        // Move task to review again
        let mut tasks_mut = tasks.clone();
        tasks_mut[0].stage_id = "review".to_string();
        crate::commands::tasks::save_tasks_to_disk(&base, &tasks_mut).await.expect("reset to review");

        let res_explicit = execute_tool(&tool_ctx_no_id, "reject_task", &serde_json::json!({
            "task_id": "CVH-062",
            "reason": "Unit tests failing"
        }))
        .await
        .expect("reject_task with explicit task_id");
        assert!(res_explicit.contains("ОТКЛОНЕНО"));

        let tasks_explicit = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t_exp = tasks_explicit.iter().find(|t| t.id == "CVH-062").expect("find task");
        assert_eq!(t_exp.stage_id, "dev");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_pipeline_reject_task_blocks_first_working_stage_to_backlog() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_reject_inv_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let (boards_cfg, _) = make_test_board(&base);
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed task in 'dev' (first working column right after backlog)
        let task = make_test_task("CVH-063", "dev", "test-board");
        crate::commands::tasks::save_tasks_to_disk(&base, &[task]).await.expect("save task");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-063"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // Attempting to reject from 'dev' (first working stage) back to backlog must be BLOCKED
        let err = execute_tool(&tool_ctx, "reject_task", &serde_json::json!({
            "reason": "Cannot proceed"
        }))
        .await
        .expect_err("Rejecting from first working stage back to backlog must be rejected");

        assert!(
            err.to_string().contains("Запрещено возвращать задачу из первой рабочей стадии назад в бэклог. Задача должна быть доработана или закрыта."),
            "Expected ironclad invariant error, got: {}",
            err
        );

        // Verify task did not move and remains in 'dev'
        let tasks = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t = tasks.iter().find(|t| t.id == "CVH-063").expect("find task");
        assert_eq!(t.stage_id, "dev");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_pipeline_reject_task_from_initial_column_fails() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_reject_init_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let (boards_cfg, _) = make_test_board(&base);
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed task in 'backlog' (index 0)
        let task = make_test_task("CVH-064", "backlog", "test-board");
        crate::commands::tasks::save_tasks_to_disk(&base, &[task]).await.expect("save task");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("backlog"),
            role: Some("developer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-064"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        let err = execute_tool(&tool_ctx, "reject_task", &serde_json::json!({ "reason": "No previous" }))
            .await
            .expect_err("Reject from initial column must fail");
        assert!(err.to_string().contains("уже находится в начальной колонке"), "got: {}", err);

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_advance_task_hook_failure_aborts_subtask_creation() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_hook_fail_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let (mut boards_cfg, _) = make_test_board(&base);
        #[cfg(target_os = "windows")]
        let hook_filename = "fail_hook.bat";
        #[cfg(not(target_os = "windows"))]
        let hook_filename = "fail_hook.sh";

        let hook_path = base.join(hook_filename);
        #[cfg(target_os = "windows")]
        tokio::fs::write(&hook_path, "@echo off\r\necho Hook check failed: sensor error\r\nexit /b 1\r\n").await.unwrap();
        #[cfg(not(target_os = "windows"))]
        {
            tokio::fs::write(&hook_path, "#!/bin/sh\necho 'Hook check failed: sensor error'\nexit 1\n").await.unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755));
            }
        }

        if let Some(board) = boards_cfg.boards.iter_mut().find(|b| b.id == "test-board") {
            if let Some(col) = board.columns.iter_mut().find(|c| c.id == "review") {
                col.hooks.insert("on_enter".to_string(), hook_filename.into());
            }
        }
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed task in 'dev' stage
        let task = make_test_task("CVH-080", "dev", "test-board");
        crate::commands::tasks::save_tasks_to_disk(&base, &[task]).await.expect("save task");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-080"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // Advance task towards review
        let err = execute_tool(&tool_ctx, "advance_task", &serde_json::json!({
            "summary": "Handover to review"
        }))
        .await
        .expect_err("Advance must fail because on_enter hook failed");

        assert!(err.to_string().contains("отклонил переход") || err.to_string().contains("HOOK REJECTED") || err.to_string().contains("Hook check failed"), "got: {}", err);

        // Verify task state: NO new subtasks created, task stays in 'dev', verdict is CHANGES_REQUESTED
        let tasks = crate::commands::tasks::load_tasks_from_disk(&base).await;
        assert_eq!(tasks.len(), 1, "No subtask must be created when hook rejects!");
        let t = tasks.iter().find(|t| t.id == "CVH-080").unwrap();
        assert_eq!(t.stage_id, "dev");
        assert_eq!(t.verdict.as_deref(), Some("CHANGES_REQUESTED"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_rejection_and_advance_resumes_existing_subtask_without_duplication() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_dup_res_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let (boards_cfg, _) = make_test_board(&base);
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // 1. Seed parent task and first subtask CVH-090.1 in 'dev'
        let mut parent = make_test_task("CVH-090", "dev", "test-board");
        parent.waiting_for_subtask_id = Some("CVH-090.1".to_string());
        parent.status = Some("waiting_subtask".to_string());

        let mut sub1 = make_test_task("CVH-090.1", "dev", "test-board");
        sub1.parent_id = Some("CVH-090".to_string());
        sub1.subtask_index = Some(1);
        sub1.status = Some("running".to_string());

        crate::commands::tasks::save_tasks_to_disk(&base, &[parent, sub1]).await.expect("save tasks");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();

        // 2. Developer in CVH-090.1 calls advance_task -> creates CVH-090.2 in 'review'
        let dev_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-090.1"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        let adv1 = execute_tool(&dev_ctx, "advance_task", &serde_json::json!({
            "summary": "Initial implementation finished"
        }))
        .await
        .expect("advance_task for CVH-090.1 must succeed");
        assert!(adv1.contains("CVH-090.2"));

        let tasks_after_adv1 = crate::commands::tasks::load_tasks_from_disk(&base).await;
        assert_eq!(tasks_after_adv1.len(), 3, "Parent + CVH-090.1 + CVH-090.2");
        let sub2 = tasks_after_adv1.iter().find(|t| t.id == "CVH-090.2").expect("subtask 2 exists");
        assert_eq!(sub2.stage_id, "review");

        // 3. Reviewer in CVH-090.2 rejects the task
        let rev_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("review"),
            role: Some("reviewer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-090.2"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        let rej_res = execute_tool(&rev_ctx, "reject_task", &serde_json::json!({
            "reason": "Fix bug in line 42"
        }))
        .await
        .expect("reject_task must succeed");

        assert!(rej_res.contains("ОТКЛОНЕНО"));
        assert!(rej_res.contains("CVH-090.1"));

        // Verify state after rejection:
        // CVH-090.2 remains in 'review' with status idle and CHANGES_REQUESTED
        // CVH-090.1 is reactivated in 'dev' with status running and CHANGES_REQUESTED
        // Parent CVH-090 is waiting for CVH-090.1
        let tasks_after_rej = crate::commands::tasks::load_tasks_from_disk(&base).await;
        assert_eq!(tasks_after_rej.len(), 3, "No tasks added during rejection");

        let p_rej = tasks_after_rej.iter().find(|t| t.id == "CVH-090").unwrap();
        let s1_rej = tasks_after_rej.iter().find(|t| t.id == "CVH-090.1").unwrap();
        let s2_rej = tasks_after_rej.iter().find(|t| t.id == "CVH-090.2").unwrap();

        assert_eq!(p_rej.waiting_for_subtask_id.as_deref(), Some("CVH-090.1"));
        assert_eq!(s1_rej.stage_id, "dev");
        assert_eq!(s1_rej.status.as_deref(), Some("running"));
        assert_eq!(s1_rej.verdict.as_deref(), Some("CHANGES_REQUESTED"));
        assert_eq!(s2_rej.stage_id, "review");
        assert_eq!(s2_rej.status.as_deref(), Some("idle"));
        assert_eq!(s2_rej.verdict.as_deref(), Some("CHANGES_REQUESTED"));

        // 4. Developer fixes bug in CVH-090.1 and calls advance_task again!
        let adv2 = execute_tool(&dev_ctx, "advance_task", &serde_json::json!({
            "summary": "Fixed bug in line 42 and added regression test"
        }))
        .await
        .expect("second advance_task must succeed");

        assert!(adv2.contains("Возобновлена существующая подзадача CVH-090.2"));

        // 5. CRITICAL INVARIANT: Verify that CVH-090.3 was NOT created!
        let tasks_final = crate::commands::tasks::load_tasks_from_disk(&base).await;
        assert_eq!(tasks_final.len(), 3, "CVH-090.3 must NOT be created! CVH-090.2 must be reused!");

        let s1_fin = tasks_final.iter().find(|t| t.id == "CVH-090.1").unwrap();
        let s2_fin = tasks_final.iter().find(|t| t.id == "CVH-090.2").unwrap();
        let p_fin = tasks_final.iter().find(|t| t.id == "CVH-090").unwrap();

        assert_eq!(s1_fin.status.as_deref(), Some("done"));
        assert_eq!(s2_fin.stage_id, "review");
        assert_eq!(s2_fin.status.as_deref(), Some("running"));
        assert_eq!(s2_fin.verdict, None, "Previous verdict must be cleared on resumption");
        assert_eq!(s2_fin.handover_summary.as_deref(), Some("Fixed bug in line 42 and added regression test"));
        assert_eq!(p_fin.waiting_for_subtask_id.as_deref(), Some("CVH-090.2"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    // -------------------------------------------------------------------------
    // b) Subagents & Delegation: synchronous subagent, subagent_bg & collect
    // -------------------------------------------------------------------------

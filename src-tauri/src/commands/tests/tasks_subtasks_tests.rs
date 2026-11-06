use super::*;
use crate::agent::Task;
use std::path::PathBuf;

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
                wip_limit: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
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
                wip_limit: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
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
                wip_limit: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
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
                wip_limit: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
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
                wip_limit: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
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
                stages: cols,
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
            provider_id: None,
            reasoning_effort: None,
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
            project_path: None,
        stage_durations: None,
        total_elapsed: None,
        };
        save_tasks_to_disk(&base, &[parent]).await.expect("save parent");

        // 1. Advance parent: creates review subtask CVH-200.1
        let adv_res = internal_advance_task(&base, "CVH-200", "Handover to review", None)
            .await
            .expect("advance to review subtask");
        assert!(adv_res.contains("УСПЕШНО: Создана подзадача CVH-200.1"));

        let loaded = load_tasks_from_disk(&base).await;
        let p1 = loaded.iter().find(|t| t.id == "CVH-200").unwrap();
        assert_eq!(p1.status.as_deref(), Some("done"));
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
        assert_eq!(p2.status.as_deref(), Some("done"), "Parent status is done");
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
                wip_limit: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
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
                wip_limit: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
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
                wip_limit: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
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
                stages: cols,
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
            status: Some("idle".to_string()),
            model: None,
            provider_id: None,
            reasoning_effort: None,
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
            project_path: None,
        stage_durations: None,
        total_elapsed: None,
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
            status: Some("idle".to_string()),
            model: None,
            provider_id: None,
            reasoning_effort: None,
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
            project_path: None,
        stage_durations: None,
        total_elapsed: None,
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
            provider_id: None,
            reasoning_effort: None,
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
            project_path: None,
        stage_durations: None,
        total_elapsed: None,
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
                wip_limit: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
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
                wip_limit: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
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
                wip_limit: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
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
                wip_limit: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
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
                stages: cols,
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
            provider_id: None,
            reasoning_effort: None,
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
            project_path: None,
        stage_durations: None,
        total_elapsed: None,
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
        assert_eq!(parent.status.as_deref(), Some("done"), "Parent status must be 'done'");
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
    async fn test_terminal_task_clears_waiting_for_subtask_and_heal_tasks_cleans_stale_subtasks() {
        let tmp = PathBuf::from("target").join(format!("cvh_heal_subtasks_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&tmp).await.expect("test dir");

        let bcfg = crate::config::BoardsConfig {
            active_board_id: "development".to_string(),
            boards: vec![crate::config::BoardDef {
                id: "development".to_string(),
                name: "Development".to_string(),
                stages: vec![
                    crate::config::PipelineColumn {
                        id: "backlog".to_string(),
                        title: "Backlog".to_string(),
                        color: None,
                        icon: None,
                        persona: None,
                        wip_limit: None,
                        on_enter_script: None,
                        context_window: None,
                        model_override: None,
                        reasoning_effort: None,
                        temperature: None,
                        gatekeeper_sensor: None,
                        allowed_tools: None,
                        hooks: HashMap::new(),
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
                        wip_limit: None,
                        on_enter_script: None,
                        context_window: None,
                        model_override: None,
                        reasoning_effort: None,
                        temperature: None,
                        gatekeeper_sensor: None,
                        allowed_tools: None,
                        hooks: HashMap::new(),
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
                        wip_limit: None,
                        on_enter_script: None,
                        context_window: None,
                        model_override: None,
                        reasoning_effort: None,
                        temperature: None,
                        gatekeeper_sensor: None,
                        allowed_tools: None,
                        hooks: HashMap::new(),
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
                provider_id: None,
            reasoning_effort: None,
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
            project_path: None,
        stage_durations: None,
        total_elapsed: None,
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
                provider_id: None,
            reasoning_effort: None,
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
            project_path: None,
        stage_durations: None,
        total_elapsed: None,
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
                provider_id: None,
            reasoning_effort: None,
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
            project_path: None,
        stage_durations: None,
        total_elapsed: None,
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
                provider_id: None,
            reasoning_effort: None,
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
            project_path: None,
        stage_durations: None,
        total_elapsed: None,
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
            status: Some("idle".to_string()),
            model: None,
            provider_id: None,
            reasoning_effort: None,
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
            project_path: None,
        stage_durations: None,
        total_elapsed: None,
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
            provider_id: None,
            reasoning_effort: None,
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
            project_path: None,
        stage_durations: None,
        total_elapsed: None,
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
    async fn test_subtask_advance_does_not_forward_summary_to_parent_dialog() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_subtask_no_dialog_leak_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;
        let _ = tokio::fs::create_dir_all(tmp.join(".cvh")).await;

        let boards_cfg = serde_json::json!({
            "active_board_id": "development",
            "boards": [
                {
                    "id": "development",
                    "name": "Разработка",
                    "columns": [
                        { "id": "plan", "title": "Планирование", "auto_run": false },
                        { "id": "dev", "title": "Разработка", "auto_run": false },
                        { "id": "review", "title": "Ревью", "auto_run": false },
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

        // 1. Create root parent task CVH-888
        let _parent = internal_create_task(
            &tmp,
            "Root Task".to_string(),
            "Parent Description".to_string(),
            "plan".to_string(),
            Some("development".to_string()),
            Some(true),
            None,
            Some("CVH-888".to_string()),
            None,
            None,
            None,
        )
        .await
        .expect("create parent task");

        // 2. Advance parent to 'dev', which creates child subtask CVH-888.1
        let adv_parent = internal_advance_task_to_stage(
            &tmp,
            "CVH-888",
            Some("dev"),
            "Planner summary for root",
            None,
        )
        .await
        .expect("advance parent to dev");
        assert!(adv_parent.contains("УСПЕШНО"));

        // Verify parent dialog has its own canonical summary
        let parent_dialog_after_plan = crate::db::load_dialog_items(&tmp, "CVH-888").expect("load parent dialog");
        assert!(parent_dialog_after_plan.iter().any(|d| {
            let c = d.content.as_deref().unwrap_or("");
            c.contains("Planner summary for root") && c.contains("### Саммари этапа plan: Root Task")
        }));

        // 3. Child subtask CVH-888.1 advances to 'review', creating CVH-888.2
        let adv_sub1 = internal_advance_task_to_stage(
            &tmp,
            "CVH-888.1",
            Some("review"),
            "Developer summary from sub1",
            None,
        )
        .await
        .expect("advance sub1 to review");
        assert!(adv_sub1.contains("УСПЕШНО"));

        // Verify subtask CVH-888.1 dialog has its own summary
        let sub1_dialog = crate::db::load_dialog_items(&tmp, "CVH-888.1").expect("load sub1 dialog");
        assert!(sub1_dialog.iter().any(|d| {
            d.content.as_deref().unwrap_or("").contains("Developer summary from sub1")
        }));

        // CRITICAL INVARIANT: Parent CVH-888 dialog must NOT receive CVH-888.1 summary
        let parent_dialog_after_sub1 = crate::db::load_dialog_items(&tmp, "CVH-888").expect("load parent dialog");
        assert!(
            !parent_dialog_after_sub1.iter().any(|d| {
                d.content.as_deref().unwrap_or("").contains("Developer summary from sub1")
            }),
            "Parent dialog must NOT contain subtask 1's summary!"
        );

        // 4. Subtask CVH-888.2 advances to terminal 'ready'
        let adv_sub2 = internal_advance_task_to_stage(
            &tmp,
            "CVH-888.2",
            Some("ready"),
            "Reviewer approved summary from sub2",
            None,
        )
        .await
        .expect("advance sub2 to ready");
        assert!(adv_sub2.contains("УСПЕШНО"));

        // Subtask CVH-888.2 dialog has its own summary
        let sub2_dialog = crate::db::load_dialog_items(&tmp, "CVH-888.2").expect("load sub2 dialog");
        assert!(sub2_dialog.iter().any(|d| {
            d.content.as_deref().unwrap_or("").contains("Reviewer approved summary from sub2")
        }));

        // CRITICAL INVARIANT: Parent CVH-888 dialog must NOT receive sub2 terminal summary or finalization summary
        let parent_dialog_final = crate::db::load_dialog_items(&tmp, "CVH-888").expect("load parent dialog");
        assert!(
            !parent_dialog_final.iter().any(|d| {
                let c = d.content.as_deref().unwrap_or("");
                c.contains("Reviewer approved summary from sub2") || c.contains("Саммари финализации задачи")
            }),
            "Parent dialog must NOT contain subtask 2's summary or finalization summary!"
        );

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }

    #[tokio::test]
    async fn test_subtask_non_terminal_advance_auto_completes_todos() {
        let base = PathBuf::from("target").join(format!("cvh_sub_todo_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let cols = vec![
            crate::config::PipelineColumn {
                id: "backlog".to_string(),
                title: "Backlog".to_string(),
                color: None,
                icon: None,
                persona: None,
                wip_limit: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
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
                wip_limit: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
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
                wip_limit: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
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
                wip_limit: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                auto_run: None,
                terminal: Some(true),
                is_intake: None,
            },
        ];

        let boards_cfg = crate::config::BoardsConfig {
            active_board_id: "sub-todo-board".to_string(),
            boards: vec![crate::config::BoardDef {
                id: "sub-todo-board".to_string(),
                name: "Sub Todo Board".to_string(),
                stages: cols,
            }],
        };
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        let parent = Task {
            id: "CVH-300".to_string(),
            title: "Parent task".to_string(),
            description: "Desc".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("sub-todo-board".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            provider_id: None,
            reasoning_effort: None,
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
            project_path: None,
        stage_durations: None,
        total_elapsed: None,
        };
        save_tasks_to_disk(&base, &[parent]).await.expect("save parent");

        let adv_res = internal_advance_task(&base, "CVH-300", "Handover to review", None)
            .await
            .expect("advance to review subtask");
        assert!(adv_res.contains("УСПЕШНО: Создана подзадача CVH-300.1"));

        let sub_todos = vec![
            crate::agent::TodoItem {
                id: "todo-1".to_string(),
                content: "Done item".to_string(),
                status: "completed".to_string(),
            },
            crate::agent::TodoItem {
                id: "todo-2".to_string(),
                content: "Unfinished item".to_string(),
                status: "in_progress".to_string(),
            },
        ];
        crate::db::db_save_todos(&base, "CVH-300.1", &sub_todos).expect("save sub todos");
        {
            let mut tasks = load_tasks_from_disk(&base).await;
            if let Some(sub) = tasks.iter_mut().find(|t| t.id == "CVH-300.1") {
                sub.todos = Some(sub_todos.clone());
                let _ = crate::db::db_upsert_task(&base, sub);
            }
            let _ = save_tasks_to_disk(&base, &tasks).await;
        }

        let adv_sub = internal_advance_task_to_stage(
            &base,
            "CVH-300.1",
            Some("ready"),
            "Subtask done",
            None,
        )
        .await
        .expect("advance subtask to ready");
        assert!(adv_sub.contains("УСПЕШНО"));

        let loaded = load_tasks_from_disk(&base).await;
        let sub = loaded.iter().find(|t| t.id == "CVH-300.1").expect("subtask exists");
        let disk_todos = sub.todos.as_ref().expect("disk todos present");
        assert_eq!(disk_todos.len(), 2);
        assert_eq!(disk_todos[0].status, "completed");
        assert_eq!(disk_todos[1].status, "completed");

        let db_task = crate::db::db_get_task(&base, "CVH-300.1").expect("db_get_task").expect("task in db");
        let db_todos = db_task.todos.as_ref().expect("db todos present");
        assert_eq!(db_todos.len(), 2);
        assert_eq!(db_todos[0].status, "completed");
        assert_eq!(db_todos[1].status, "completed");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

use super::*;
use crate::agent::Task;
use crate::config::normalize_pipeline_columns;
use std::path::PathBuf;


    #[test]
    fn test_normalize_preserves_custom_column_order() {
        let cols = vec![
            test_col("dev"),
            test_col("ready"),
            test_col("review"),
            test_col("backlog"),
            test_col("qa"),
        ];
        let out = normalize_pipeline_columns(cols).expect("normalize failed");
        assert_eq!(ids(&out), vec!["dev", "ready", "review", "backlog", "qa"]);
        assert_eq!(out[0].is_intake, Some(true));
        assert_eq!(out.last().unwrap().terminal, Some(true));
    }


    #[test]
    fn test_normalize_already_ordered_keeps_order_and_sets_intake_terminal() {
        let cols = vec![
            test_col("backlog"),
            test_col("a"),
            test_col("b"),
            test_col("ready"),
        ];
        let out = normalize_pipeline_columns(cols).expect("normalize failed");
        assert_eq!(ids(&out), vec!["backlog", "a", "b", "ready"]);
        assert_eq!(out[0].is_intake, Some(true));
        assert_eq!(out.last().unwrap().terminal, Some(true));
    }


    #[test]
    fn test_normalize_without_specials_preserves_custom_columns_without_forced_injection() {
        let cols = vec![test_col("a"), test_col("b"), test_col("c")];
        let out = normalize_pipeline_columns(cols).expect("normalize failed");
        assert_eq!(ids(&out), vec!["a", "b", "c"]);
        assert_eq!(out[0].is_intake, Some(true));
        assert_eq!(out.last().unwrap().terminal, Some(true));
    }


    #[test]
    fn test_normalize_empty_returns_canonical() {
        let out = normalize_pipeline_columns(vec![]).expect("normalize failed");
        assert_eq!(ids(&out), vec!["backlog", "dev", "ready"]);
        assert_eq!(out[0].is_intake, Some(true));
        assert_eq!(out.last().unwrap().terminal, Some(true));
    }


    #[test]
    fn test_normalize_duplicate_ids_error() {
        let cols = vec![test_col("dev"), test_col("dev")];
        let err = normalize_pipeline_columns(cols).expect_err("expected duplicate error");
        assert!(err.contains("dev"), "unexpected error text: {}", err);
    }


    #[test]
    fn test_slugify_board_name_cyrillic_and_latin() {
        assert_eq!(slugify_board_name("3D Моделирование"), "3d-modelirovanie");
        assert_eq!(slugify_board_name("Разработка"), "razrabotka");
        assert_eq!(slugify_board_name("Dev Pipeline"), "dev-pipeline");
        assert_eq!(slugify_board_name("Engineering Core 2"), "engineering-core-2");
    }


    #[test]
    fn test_task_board_id_serialization() {
        let json_with_board = r#"{
            "id": "CVH-100",
            "title": "3D asset task",
            "stageId": "concept",
            "boardId": "modeler"
        }"#;

        let task: Task = serde_json::from_str(json_with_board).expect("deserialize task");
        assert_eq!(task.board_id.as_deref(), Some("modeler"));

        let json_without_board = r#"{
            "id": "CVH-101",
            "title": "Legacy task",
            "stageId": "dev"
        }"#;

        let task_without_board: Task = serde_json::from_str(json_without_board).expect("deserialize task without board");
        assert!(task_without_board.board_id.is_none());

        let default_cfg = crate::config::default_boards_config();
        let fixed = ensure_task_board_id(vec![task_without_board], &default_cfg);
        assert_eq!(fixed[0].board_id.as_deref(), Some("development"));
    }


    #[test]
    fn test_self_healing_orphaned_board_and_column() {
        use crate::commands::tasks::{heal_task_board_and_column, heal_tasks};
        use crate::config::{BoardDef, BoardsConfig, PipelineColumn};

        let custom_cfg = BoardsConfig {
            active_board_id: "feature-board".to_string(),
            boards: vec![
                BoardDef {
                    id: "feature-board".to_string(),
                    name: "Feature Board".to_string(),
                    columns: vec![
                        PipelineColumn {
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
                            hooks: std::collections::HashMap::new(),
                            routes: None,
                            auto_run: None,
                            terminal: None,
                            is_intake: Some(true),
                        },
                        PipelineColumn {
                            id: "in_progress".to_string(),
                            title: "In Progress".to_string(),
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
                            hooks: std::collections::HashMap::new(),
                            routes: None,
                            auto_run: None,
                            terminal: None,
                            is_intake: None,
                        },
                        PipelineColumn {
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
                            hooks: std::collections::HashMap::new(),
                            routes: None,
                            auto_run: None,
                            terminal: Some(true),
                            is_intake: None,
                        },
                    ],
                },
            ],
        };

        // 1. Orphaned board_id should be remapped to active_board_id ("feature-board")
        // and non-existent column "old_unknown_stage" should be healed to "backlog"
        let mut orphaned_task = Task {
            id: "CVH-200".to_string(),
            title: "Orphaned Task".to_string(),
            description: String::new(),
            stage_id: "old_unknown_stage".to_string(),
            board_id: Some("deleted_board".to_string()),
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
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };

        let modified = heal_task_board_and_column(&mut orphaned_task, &custom_cfg);
        assert!(modified);
        assert_eq!(orphaned_task.board_id.as_deref(), Some("feature-board"));
        assert_eq!(orphaned_task.stage_id, "backlog");

        // 2. Valid task should remain unchanged
        let mut valid_task = Task {
            id: "CVH-201".to_string(),
            title: "Valid Task".to_string(),
            description: String::new(),
            stage_id: "in_progress".to_string(),
            board_id: Some("feature-board".to_string()),
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
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };

        let modified_valid = heal_task_board_and_column(&mut valid_task, &custom_cfg);
        assert!(!modified_valid);
        assert_eq!(valid_task.board_id.as_deref(), Some("feature-board"));
        assert_eq!(valid_task.stage_id, "in_progress");

        // 3. heal_tasks on vector
        let mut tasks = vec![orphaned_task.clone(), valid_task.clone()];
        tasks[0].board_id = None;
        let any_mod = heal_tasks(&mut tasks, &custom_cfg);
        assert!(any_mod);
        assert_eq!(tasks[0].board_id.as_deref(), Some("feature-board"));
    }


    #[tokio::test]
    async fn test_multi_board_move_task_board_preservation() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_test_multiboard_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("create .cvh");

        // Boards: 'development' (3 cols) and 'complex' (4 cols)
        let boards_json = serde_json::json!({
            "activeBoardId": "development",
            "boards": [
                {
                    "id": "development",
                    "name": "Development",
                    "columns": [
                        { "id": "backlog", "title": "Backlog", "color": "#71717a", "icon": "Inbox", "persona": null, "hooks": {} },
                        { "id": "dev", "title": "Dev", "color": "#3b82f6", "icon": "Code2", "persona": "cvh-dev", "hooks": {} },
                        { "id": "ready", "title": "Ready", "color": "#22c55e", "icon": "CheckCircle2", "persona": null, "terminal": true, "hooks": {} }
                    ]
                },
                {
                    "id": "complex",
                    "name": "Complex",
                    "columns": [
                        { "id": "backlog", "title": "Backlog", "color": "#71717a", "icon": "Inbox", "persona": null, "hooks": {} },
                        { "id": "dev", "title": "Dev", "color": "#3b82f6", "icon": "Code2", "persona": "cvh-dev", "hooks": {} },
                        { "id": "review", "title": "Review", "color": "#f59e0b", "icon": "ShieldCheck", "persona": "reviewer", "hooks": {} },
                        { "id": "ready", "title": "Ready", "color": "#22c55e", "icon": "CheckCircle2", "persona": null, "terminal": true, "hooks": {} }
                    ]
                }
            ]
        });
        tokio::fs::write(base.join(".cvh").join("boards.json"), serde_json::to_string(&boards_json).unwrap())
            .await
            .expect("write boards.json");

        let task = Task {
            id: "TASK-MB-1".to_string(),
            title: "Multi-board task".to_string(),
            description: "Test board preservation".to_string(),
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
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: Some(true),
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            todos: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };
        save_tasks_to_disk(&base, &[task]).await.expect("save task");

        // 1. Move within 'development' board to 'dev' with explicit board_id_override:
        let m1 = internal_move_task_stage(
            &base,
            "TASK-MB-1",
            "dev",
            None,
            None,
            None,
            None,
            false,
            Some("development".to_string()),
        )
        .await
        .expect("move to dev");
        assert_eq!(m1.stage_id, "dev");
        assert_eq!(m1.board_id.as_deref(), Some("development"), "Must strictly preserve active board_id");

        // 2. Move to 'ready' without override (task's current_board is development, which has ready):
        let m2 = internal_move_task_stage(
            &base,
            "TASK-MB-1",
            "ready",
            None,
            None,
            None,
            None,
            false,
            None,
        )
        .await
        .expect("move to ready");
        assert_eq!(m2.stage_id, "ready");
        assert_eq!(m2.board_id.as_deref(), Some("development"), "Must stay on development board");

        // 3. Move to 'review' which ONLY exists on 'complex' board:
        let m3 = internal_move_task_stage(
            &base,
            "TASK-MB-1",
            "review",
            None,
            None,
            None,
            None,
            false,
            None,
        )
        .await
        .expect("move to review");
        assert_eq!(m3.stage_id, "review");
        assert_eq!(m3.board_id.as_deref(), Some("complex"), "Must cleanly transition to board containing stage");

        // 4. Move from 'review' back to 'dev' without override (current_board is now complex, which has dev):
        let m4 = internal_move_task_stage(
            &base,
            "TASK-MB-1",
            "dev",
            None,
            None,
            None,
            None,
            false,
            None,
        )
        .await
        .expect("move to dev on complex");
        assert_eq!(m4.stage_id, "dev");
        assert_eq!(m4.board_id.as_deref(), Some("complex"), "Must stay on complex board");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_multi_board_pipeline_routing_and_board_preservation() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_test_routing_preservation_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("create .cvh");

        // Boards: 'development' (active) and 'nastroyka' (Setup)
        let boards_json = serde_json::json!({
            "activeBoardId": "development",
            "boards": [
                {
                    "id": "development",
                    "name": "CanbanVoimaxHarness",
                    "columns": [
                        { "id": "backlog", "title": "Backlog", "persona": null, "hooks": {} },
                        { "id": "dev", "title": "Dev", "persona": "cvh-dev", "hooks": {} },
                        { "id": "ready", "title": "Ready", "persona": null, "terminal": true, "hooks": {} }
                    ]
                },
                {
                    "id": "nastroyka",
                    "name": "Setup",
                    "columns": [
                        { "id": "backlog", "title": "Backlog", "persona": null, "hooks": {} },
                        { "id": "dev", "title": "CVH Development", "persona": "developer", "hooks": {} },
                        { "id": "ready", "title": "Ready", "persona": null, "terminal": true, "hooks": {} }
                    ]
                }
            ]
        });
        tokio::fs::write(base.join(".cvh").join("boards.json"), serde_json::to_string(&boards_json).unwrap())
            .await
            .expect("write boards.json");

        // Seed task CVH-011 on board 'nastroyka' in stage 'dev'
        let task = Task {
            id: "CVH-011".to_string(),
            title: "Setup Task".to_string(),
            description: "Test board preservation during advance".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("nastroyka".to_string()),
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
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: Some(true),
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            todos: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };
        save_tasks_to_disk(&base, &[task]).await.expect("save task");

        // Advance task CVH-011: active board is 'development', but task is on 'nastroyka'
        let adv_res = internal_advance_task(&base, "CVH-011", "Task completed on nastroyka board", None)
            .await
            .expect("advance task");
        assert!(adv_res.contains("УСПЕШНО"));

        let loaded = load_tasks_from_disk(&base).await;
        let t = loaded.iter().find(|t| t.id == "CVH-011").unwrap();
        // Task MUST preserve its board 'nastroyka' and MUST NOT hop to 'development'
        assert_eq!(t.board_id.as_deref(), Some("nastroyka"), "Task must stay on 'nastroyka' board!");
        assert_eq!(t.stage_id, "ready");
        assert_eq!(t.status.as_deref(), Some("done"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_move_task_to_plan_from_pipeline_when_not_in_board() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_plan_pipeline_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;
        let _ = tokio::fs::create_dir_all(tmp.join(".cvh")).await;

        // Boards config does NOT contain 'plan' column
        let boards_cfg = serde_json::json!({
            "active_board_id": "development",
            "boards": [
                {
                    "id": "development",
                    "name": "Разработка",
                    "columns": [
                        { "id": "backlog", "title": "Бэклог", "auto_run": false },
                        { "id": "dev", "title": "В работе", "persona": "cvh-dev", "auto_run": true },
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

        // Pipeline config DOES contain 'plan' column
        let pipeline_cfg = serde_json::json!({
            "id": "development",
            "title": "Pipeline",
            "columns": [
                { "id": "backlog", "title": "Бэклог" },
                { "id": "plan", "title": "Планирование", "persona": "planner", "auto_run": true },
                { "id": "dev", "title": "В работе", "persona": "cvh-dev", "auto_run": true },
                { "id": "ready", "title": "Готово", "terminal": true }
            ]
        });
        tokio::fs::write(
            tmp.join(".cvh").join("pipeline.json"),
            serde_json::to_string_pretty(&pipeline_cfg).unwrap(),
        )
        .await
        .expect("write pipeline.json");

        let _task = internal_create_task(
            &tmp,
            "Pipeline Plan Task".to_string(),
            "Move to plan from pipeline".to_string(),
            "backlog".to_string(),
            Some("development".to_string()),
            Some(true),
            None,
            Some("CVH-102".to_string()),
            None,
            None,
            None,
        )
        .await
        .expect("create task");

        // Move task to 'plan' column which only exists in pipeline.json
        let moved = internal_move_task_stage(
            &tmp,
            "CVH-102",
            "plan",
            None,
            None,
            None,
            None,
            false,
            Some("development".to_string()),
        )
        .await
        .expect("move task to plan from pipeline must succeed");

        assert_eq!(moved.stage_id, "plan", "Task must move to 'plan' from pipeline");
        assert_ne!(moved.stage_id, "ready", "Task must NEVER falsely fall back to 'ready'");
        assert_eq!(moved.status.as_deref(), Some("running"));

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }


    #[tokio::test]
    async fn test_move_task_to_non_existent_stage_returns_error_not_ready() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_non_existent_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;
        let _ = tokio::fs::create_dir_all(tmp.join(".cvh")).await;

        let boards_cfg = serde_json::json!({
            "active_board_id": "development",
            "boards": [
                {
                    "id": "development",
                    "name": "Разработка",
                    "columns": [
                        { "id": "backlog", "title": "Бэклог" },
                        { "id": "dev", "title": "В работе" },
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

        let _task = internal_create_task(
            &tmp,
            "NonExistent Stage Task".to_string(),
            "Move to non_existent".to_string(),
            "backlog".to_string(),
            Some("development".to_string()),
            Some(true),
            None,
            Some("CVH-103".to_string()),
            None,
            None,
            None,
        )
        .await
        .expect("create task");

        // Attempt move to 'non_existent' column
        let res = internal_move_task_stage(
            &tmp,
            "CVH-103",
            "non_existent",
            None,
            None,
            None,
            None,
            false,
            Some("development".to_string()),
        )
        .await;

        assert!(res.is_err(), "Moving to non-existent stage must fail!");
        let err = res.unwrap_err();
        assert!(
            err.contains("Target stage 'non_existent' not found in active board or pipeline columns. Available:"),
            "Error message must specify available stages and not silently fall back to ready! Got: {}",
            err
        );

        // Verify task on disk is still in backlog and NOT in ready / done
        let loaded = load_tasks_from_disk(&tmp).await;
        let t = loaded.iter().find(|t| t.id == "CVH-103").expect("task exists");
        assert_eq!(t.stage_id, "backlog", "Task must remain in backlog");
        assert_ne!(t.stage_id, "ready", "Task must NEVER be placed into ready");
        assert_ne!(t.status.as_deref(), Some("done"), "Task status must NOT be done");

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }



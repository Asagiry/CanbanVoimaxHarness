use super::*;
use crate::agent::Task;
use std::path::PathBuf;

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

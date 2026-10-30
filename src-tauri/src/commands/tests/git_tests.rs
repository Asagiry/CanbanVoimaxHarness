use super::*;
use crate::agent::Task;
use std::path::PathBuf;


    #[test]
    fn test_apply_worktree_from_hook_stdout() {
        let mut task = task_with_wt("CVH-001", None);
        apply_worktree_from_hook_stdout(
            &mut task,
            "some log\nCVH_WORKTREE=/tmp/wt-cvh-001\nCVH_BRANCH=feat/cvh-001\n",
        );
        assert_eq!(task.worktree_path.as_deref(), Some("/tmp/wt-cvh-001"));
        assert_eq!(task.branch.as_deref(), Some("feat/cvh-001"));

        // CVH_BRANCH does not overwrite existing non-empty branch
        let mut task2 = task_with_wt("CVH-002", None);
        task2.branch = Some("feat/keep".to_string());
        apply_worktree_from_hook_stdout(&mut task2, "CVH_BRANCH=feat/other");
        assert_eq!(task2.branch.as_deref(), Some("feat/keep"));

        // Plain stdout without markers leaves fields intact
        let mut task3 = task_with_wt("CVH-003", None);
        apply_worktree_from_hook_stdout(&mut task3, "plain log, no markers\n");
        assert!(task3.worktree_path.is_none());

        // Windows backslashes in CVH_WORKTREE are normalized to forward slashes
        let mut task4 = task_with_wt("CVH-004", None);
        apply_worktree_from_hook_stdout(&mut task4, "CVH_WORKTREE=worktrees\\CVH-004\n");
        assert_eq!(task4.worktree_path.as_deref(), Some("worktrees/CVH-004"));
    }


    #[tokio::test]
    async fn test_scaffold_backlog_hook_with_and_without_git() {
        // Scaffold creates clean minimal pipeline with empty hooks (pure script governance)
        let with_git =
            PathBuf::from("target").join(format!("cvh_scaffold_git_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(with_git.join(".git")).await.expect("git dir");
        crate::config::scaffold_cvh_project(&with_git).await.expect("scaffold");
        let raw = tokio::fs::read_to_string(with_git.join(".cvh").join("pipeline.json"))
            .await
            .expect("read pipeline");
        let pipe: crate::config::PipelineConfig =
            serde_json::from_str(&raw).expect("parse pipeline");
        let backlog = pipe.columns.iter().find(|c| c.id == "backlog").expect("backlog");
        assert!(backlog.hooks.is_empty());

        // Repeated scaffold preserves existing pipeline.json
        crate::config::scaffold_cvh_project(&with_git).await.expect("rescaffold");
        let raw2 = tokio::fs::read_to_string(with_git.join(".cvh").join("pipeline.json"))
            .await
            .expect("reread");
        assert_eq!(raw, raw2);

        // Without .git -> hooks empty
        let no_git =
            PathBuf::from("target").join(format!("cvh_scaffold_nogit_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&no_git).await.expect("plain dir");
        crate::config::scaffold_cvh_project(&no_git).await.expect("scaffold");
        let raw_n = tokio::fs::read_to_string(no_git.join(".cvh").join("pipeline.json"))
            .await
            .expect("read pipeline");
        let pipe_n: crate::config::PipelineConfig =
            serde_json::from_str(&raw_n).expect("parse pipeline");
        let backlog_n = pipe_n.columns.iter().find(|c| c.id == "backlog").expect("backlog");
        assert!(backlog_n.hooks.is_empty());

        let _ = tokio::fs::remove_dir_all(&with_git).await;
        let _ = tokio::fs::remove_dir_all(&no_git).await;
    }

    #[test]
    fn test_skip_worktree_flag_serialization_and_defaults() {
        // Deserialization with skipWorktree = true
        let t1: Task = serde_json::from_value(serde_json::json!({
            "id": "CVH-101",
            "title": "Skip wt task",
            "stageId": "dev",
            "skipWorktree": true
        })).expect("deserialize t1");
        assert_eq!(t1.skip_worktree, Some(true));

        // Deserialization with alias skip_worktree = false
        let t2: Task = serde_json::from_value(serde_json::json!({
            "id": "CVH-102",
            "title": "Don't skip wt task",
            "stageId": "dev",
            "skip_worktree": false
        })).expect("deserialize t2");
        assert_eq!(t2.skip_worktree, Some(false));

        // Deserialization with alias skipBacklogHook = true
        let t_sbh: Task = serde_json::from_value(serde_json::json!({
            "id": "CVH-102b",
            "title": "Skip backlog hook task",
            "stageId": "backlog",
            "skipBacklogHook": true
        })).expect("deserialize t_sbh");
        assert_eq!(t_sbh.skip_worktree, Some(true));

        // Deserialization with alias skip_backlog_hook = true
        let t_sbh2: Task = serde_json::from_value(serde_json::json!({
            "id": "CVH-102c",
            "title": "Skip backlog hook task snake",
            "stageId": "backlog",
            "skip_backlog_hook": true
        })).expect("deserialize t_sbh2");
        assert_eq!(t_sbh2.skip_worktree, Some(true));

        // Deserialization with default when omitted
        let t3: Task = serde_json::from_value(serde_json::json!({
            "id": "CVH-103",
            "title": "Default wt task",
            "stageId": "dev"
        })).expect("deserialize t3");
        assert_eq!(t3.skip_worktree, None);
    }


    #[tokio::test]
    async fn test_skip_worktree_prevents_builtin_worktree_in_move_stage() {
        let base = PathBuf::from("target").join(format!("cvh_skip_wt_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dir");

        let pipe = crate::config::default_pipeline_config();
        crate::config::save_pipeline_config(&base, &pipe).await.expect("save pipeline");

        let task = Task {
            id: "CVH-SKIP-1".to_string(),
            title: "No Worktree Task".to_string(),
            description: "Root only".to_string(),
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
        save_tasks_to_disk(&base, &[task]).await.expect("save task");

        // Move to dev stage
        let moved = internal_move_task_stage(
            &base,
            "CVH-SKIP-1",
            "dev",
            None,
            None,
            None,
            None,
            false,
            None,
        )
        .await
        .expect("move task stage");

        // When skip_worktree is true, NO worktree or branch must be created!
        assert_eq!(moved.stage_id, "dev");
        assert_eq!(moved.worktree_path, None);
        assert_eq!(moved.branch, None);

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_get_task_git_diff_subtask_inherits_parent_worktree() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_diff_subtask_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;
        let _ = tokio::fs::create_dir_all(tmp.join(".cvh")).await;

        // 1. Create a fake worktree directory on disk for parent task CVH-999
        let wt_dir = tmp.join("worktrees").join("CVH-999");
        tokio::fs::create_dir_all(&wt_dir).await.expect("create fake worktree dir");

        // 2. Parent task has worktree_path and branch set
        let parent = Task {
            id: "CVH-999".to_string(),
            title: "Parent Task".to_string(),
            description: "Parent desc".to_string(),
            stage_id: "plan".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: Some("worktrees/CVH-999".to_string()),
            branch: Some("feat/CVH-999".to_string()),
            verdict: None,
            custom_flow: None,
            status: Some("waiting_subtask".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: Some("CVH-999.1".to_string()),
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

        // 3. Child subtask CVH-999.1 has NO worktree_path and NO branch
        let child = Task {
            id: "CVH-999.1".to_string(),
            title: "Child in dev".to_string(),
            description: "Child desc".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: Some("CVH-999".to_string()),
            subtask_index: Some(1),
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

        save_tasks_to_disk(&tmp, &[parent, child]).await.expect("save tasks to disk");

        // Call resolve_task_worktree for the subtask CVH-999.1
        let (resolved_task, workdir, branch) = crate::commands::git::resolve_task_worktree(&tmp, "CVH-999.1", None)
            .await
            .expect("resolve_task_worktree must succeed for subtask");

        // Assert workdir is inside worktrees/CVH-999 and NOT falling back to tmp (root)
        assert_eq!(workdir, wt_dir, "Subtask workdir must inherit parent worktree path and not fall back to root repo");
        assert_eq!(resolved_task.worktree_path.as_deref(), Some("worktrees/CVH-999"), "Subtask worktree_path must be auto-healed");
        assert_eq!(resolved_task.branch.as_deref(), Some("feat/CVH-999"), "Subtask branch must be auto-healed from parent");
        assert_ne!(branch, "main", "Branch must never be main when isolated worktree exists");

        // Verify task persisted in SQLite/disk now has worktree_path and branch
        let loaded = load_tasks_from_disk(&tmp).await;
        let loaded_child = loaded.iter().find(|t| t.id == "CVH-999.1").expect("child exists");
        assert_eq!(loaded_child.worktree_path.as_deref(), Some("worktrees/CVH-999"));
        assert_eq!(loaded_child.branch.as_deref(), Some("feat/CVH-999"));

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }


    #[tokio::test]
    async fn test_resolve_task_worktree_filesystem_heuristic_heals_parent_and_child() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_diff_heuristic_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;
        let _ = tokio::fs::create_dir_all(tmp.join(".cvh")).await;

        // Create worktrees/CVH-888 on disk physically
        let wt_dir = tmp.join("worktrees").join("CVH-888");
        tokio::fs::create_dir_all(&wt_dir).await.expect("create fake worktree dir");

        // Both parent and child have worktree_path: None and branch: None
        let parent = Task {
            id: "CVH-888".to_string(),
            title: "Parent Wiped".to_string(),
            description: "desc".to_string(),
            stage_id: "plan".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("waiting_subtask".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: Some("CVH-888.1".to_string()),
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
        let child = Task {
            id: "CVH-888.1".to_string(),
            title: "Child in dev".to_string(),
            description: "desc".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: Some("CVH-888".to_string()),
            subtask_index: Some(1),
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

        save_tasks_to_disk(&tmp, &[parent, child]).await.expect("save tasks");

        let (resolved_task, workdir, branch) = crate::commands::git::resolve_task_worktree(&tmp, "CVH-888.1", None)
            .await
            .expect("resolve_task_worktree must heal from filesystem heuristic");

        assert_eq!(workdir, wt_dir);
        assert_eq!(resolved_task.worktree_path.as_deref(), Some("worktrees/CVH-888"));
        assert_eq!(resolved_task.branch.as_deref(), Some("feat/CVH-888"));
        assert_ne!(branch, "main", "Branch must not be main when worktree is resolved");

        // Verify parent is also healed on disk
        let loaded = load_tasks_from_disk(&tmp).await;
        let loaded_parent = loaded.iter().find(|t| t.id == "CVH-888").expect("parent exists");
        assert_eq!(loaded_parent.worktree_path.as_deref(), Some("worktrees/CVH-888"));
        assert_eq!(loaded_parent.branch.as_deref(), Some("feat/CVH-888"));

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }


    #[tokio::test]
    async fn test_save_tasks_preserves_worktree_path_and_branch_from_client_wipes() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_preserve_wt_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;
        let _ = tokio::fs::create_dir_all(tmp.join(".cvh")).await;

        let task = Task {
            id: "CVH-777".to_string(),
            title: "Task with WT".to_string(),
            description: "desc".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: Some("worktrees/CVH-777".to_string()),
            branch: Some("feat/CVH-777".to_string()),
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
        save_tasks_to_disk(&tmp, &[task]).await.expect("initial save");

        let state = AppState::new(tmp.clone());

        // Client sends updated task with worktree_path: None and branch: None
        let client_task = Task {
            id: "CVH-777".to_string(),
            title: "Task with WT Updated Title".to_string(),
            description: "desc".to_string(),
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

        internal_save_tasks(&tmp, vec![client_task], Some(&state.tasks)).await.expect("internal_save_tasks");

        let loaded = load_tasks_from_disk(&tmp).await;
        let loaded_task = loaded.iter().find(|t| t.id == "CVH-777").expect("task exists");
        assert_eq!(loaded_task.title, "Task with WT Updated Title");
        assert_eq!(loaded_task.worktree_path.as_deref(), Some("worktrees/CVH-777"), "worktree_path must NOT be wiped by client save");
        assert_eq!(loaded_task.branch.as_deref(), Some("feat/CVH-777"), "branch must NOT be wiped by client save");

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }



//! Tests for Task Transitions & Lost Update Prevention

use super::*;
use crate::agent::Task;
use crate::commands::tasks::storage::load_tasks_from_disk;
use super::transition_hooks::upsert_single_task;

fn sample_task(id: &str, stage_id: &str, status: &str) -> Task {
    Task {
        id: id.to_string(),
        title: format!("Title {}", id),
        description: format!("Desc {}", id),
        stage_id: stage_id.to_string(),
        board_id: Some("default".to_string()),
        worktree_path: None,
        branch: None,
        verdict: None,
        custom_flow: None,
        status: Some(status.to_string()),
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
        skip_worktree: Some(true),
        created_at: 1000,
        updated_at: 1000,
        stage_entered_at: Some(1000),
        completed_at: None,
        position: Some(1000.0),
        hook_states: None,
        project_path: None,
        stage_durations: None,
        total_elapsed: None,
    }
}

#[tokio::test]
async fn move_task_stage_preserves_concurrent_task_updates_during_on_enter_hook() {
    let root_buf = std::env::temp_dir().join(format!("cvh_transitions_lost_update_{}", uuid::Uuid::new_v4()));
    let root = root_buf.as_path();
    std::fs::create_dir_all(root.join(".cvh")).expect("create .cvh");

    // Create an on_enter script that concurrently updates CVH-002 in SQLite while CVH-001's hook is running
    #[cfg(windows)]
    let script_name = "concurrent_update.ps1";
    #[cfg(not(windows))]
    let script_name = "concurrent_update.sh";

    let script_path = root.join(".cvh").join(script_name);
    #[cfg(windows)]
    std::fs::write(
        &script_path,
        "Start-Sleep -Milliseconds 200\r\nWrite-Output 'hook-ran'\r\n",
    )
    .expect("write hook");
    #[cfg(not(windows))]
    std::fs::write(&script_path, "#!/bin/sh\nsleep 0.2\necho 'hook-ran'\n").expect("write hook");

    let boards_json = serde_json::json!({
        "activeBoardId": "development",
        "boards": [
            {
                "id": "development",
                "name": "Dev Board",
                "stages": [
                    { "id": "backlog", "title": "Backlog", "isIntake": true },
                    { "id": "dev", "title": "Dev", "onEnterScript": format!(".cvh/{}", script_name) },
                    { "id": "ready", "title": "Ready", "terminal": true }
                ]
            }
        ]
    });
    std::fs::write(
        root.join(".cvh").join("boards.json"),
        serde_json::to_string_pretty(&boards_json).unwrap(),
    )
    .expect("write boards.json");

    let mut task1 = sample_task("CVH-001", "backlog", "idle");
    let mut task2 = sample_task("CVH-002", "dev", "running");
    upsert_single_task(root, &mut task1).expect("upsert task1");
    upsert_single_task(root, &mut task2).expect("upsert task2");

    let root_for_concurrent = root.to_path_buf();
    let concurrent_writer = async move {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        task2.status = Some("done".to_string());
        task2.stage_id = "ready".to_string();
        task2.handover_summary = Some("Concurrent completion summary".to_string());
        upsert_single_task(&root_for_concurrent, &mut task2).expect("concurrent upsert task2");
    };

    let (move_res, _) = tokio::join!(
        internal_move_task_stage(
            root,
            "CVH-001",
            "dev",
            None,
            None,
            None,
            None,
            false,
            None,
        ),
        concurrent_writer
    );

    let moved = move_res.expect("move CVH-001 to dev");

    assert_eq!(moved.stage_id, "dev");

    let final_tasks = load_tasks_from_disk(root).await;
    let t2 = final_tasks
        .iter()
        .find(|t| t.id == "CVH-002")
        .expect("CVH-002 present");
    assert_eq!(t2.stage_id, "ready");
    assert_eq!(t2.status.as_deref(), Some("done"));
    assert_eq!(
        t2.handover_summary.as_deref(),
        Some("Concurrent completion summary")
    );
}

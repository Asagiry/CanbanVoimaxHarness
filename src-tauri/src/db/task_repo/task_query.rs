use std::collections::HashMap;
use crate::agent::{Task, TodoItem};
use crate::db::get_db_conn;
use rusqlite::params;
use std::path::Path;

/// Retrieves all tasks from SQLite ordered by position and created_at.
pub fn db_get_all_tasks(root: &Path) -> Result<Vec<Task>, String> {
    let conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, title, description, stage_id, board_id, worktree_path, branch, verdict,
                    custom_flow, status, model, provider_id, reasoning_effort, parent_id, subtask_index, waiting_for_subtask_id,
                    handover_summary, cover_image, priority, current_tokens, skip_worktree,
                    created_at, updated_at, attachments, tags, position, hook_states, completed_at,
                    stage_entered_at, stage_durations, total_elapsed
             FROM tasks
             ORDER BY position ASC, created_at ASC",
        )
        .map_err(|e| format!("Prepare select tasks failed: {}", e))?;

    let task_rows = stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let description: String = row.get(2)?;
            let stage_id: String = row.get(3)?;
            let board_id: Option<String> = row.get(4)?;
            let worktree_path: Option<String> = row.get(5)?;
            let branch: Option<String> = row.get(6)?;
            let verdict: Option<String> = row.get(7)?;
            let custom_flow_raw: Option<String> = row.get(8)?;
            let status: Option<String> = row.get(9)?;
            let model: Option<String> = row.get(10)?;
            let provider_id: Option<String> = row.get(11)?;
            let reasoning_effort: Option<String> = row.get(12).ok();
            let parent_id: Option<String> = row.get(13)?;
            let subtask_index_i64: Option<i64> = row.get(14)?;
            let waiting_for_subtask_id: Option<String> = row.get(15)?;
            let handover_summary: Option<String> = row.get(16)?;
            let cover_image: Option<String> = row.get(17)?;
            let priority: Option<String> = row.get(18)?;
            let current_tokens_i64: Option<i64> = row.get(19)?;
            let skip_worktree_i64: i64 = row.get(20)?;
            let created_at: i64 = row.get(21)?;
            let updated_at: i64 = row.get(22)?;
            let attachments_raw: Option<String> = row.get(23).ok();
            let tags_raw: Option<String> = row.get(24).ok();
            let position: Option<f64> = row.get(25).ok();
            let hook_states_raw: Option<String> = row.get(26).ok();
            let completed_at: Option<i64> = row.get(27).ok();
            let stage_entered_at: Option<i64> = row.get(28).ok();
            let stage_durations_raw: Option<String> = row.get(29).ok();
            let total_elapsed: Option<i64> = row.get(30).ok();

            Ok((
                id, title, description, stage_id, board_id, worktree_path, branch, verdict,
                custom_flow_raw, status, model, provider_id, reasoning_effort, parent_id, subtask_index_i64, waiting_for_subtask_id,
                handover_summary, cover_image, priority, current_tokens_i64, skip_worktree_i64,
                created_at, updated_at, attachments_raw, tags_raw, position, hook_states_raw, completed_at,
                stage_entered_at, stage_durations_raw, total_elapsed,
            ))
        })
        .map_err(|e| format!("Query tasks failed: {}", e))?;

    let mut todos_by_task: HashMap<String, Vec<TodoItem>> = HashMap::new();
    let mut todo_stmt = conn
        .prepare("SELECT id, task_id, content, status FROM task_todos ORDER BY order_idx ASC")
        .map_err(|e| format!("Prepare select todos failed: {}", e))?;

    let todo_rows = todo_stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let task_id: String = row.get(1)?;
            let content: String = row.get(2)?;
            let status: String = row.get(3)?;
            Ok((task_id, TodoItem { id, content, status }))
        })
        .map_err(|e| format!("Query todos failed: {}", e))?;

    for tr in todo_rows.flatten() {
        todos_by_task.entry(tr.0).or_default().push(tr.1);
    }

    let mut tasks = Vec::new();
    for row_res in task_rows {
        let (
            id, title, description, stage_id, board_id, worktree_path, branch, verdict,
            custom_flow_raw, status, model, provider_id, reasoning_effort, parent_id, subtask_index_i64, waiting_for_subtask_id,
            handover_summary, cover_image, priority, current_tokens_i64, skip_worktree_i64,
            created_at, updated_at, attachments_raw, tags_raw, position, hook_states_raw, completed_at,
            stage_entered_at, stage_durations_raw, total_elapsed,
        ) = row_res.map_err(|e| format!("Row extraction failed: {}", e))?;

        let custom_flow = custom_flow_raw.and_then(|s| serde_json::from_str(&s).ok());
        let attachments = attachments_raw.and_then(|s| serde_json::from_str(&s).ok());
        let tags = tags_raw.and_then(|s| serde_json::from_str(&s).ok());
        let hook_states = hook_states_raw.and_then(|s| serde_json::from_str(&s).ok());
        let stage_durations = stage_durations_raw.and_then(|s| serde_json::from_str(&s).ok());
        let todos = todos_by_task.remove(&id);

        tasks.push(Task {
            id,
            title,
            description,
            stage_id,
            board_id,
            worktree_path,
            branch,
            verdict,
            custom_flow,
            status,
            model,
            provider_id,
            reasoning_effort,
            parent_id,
            subtask_index: subtask_index_i64.map(|v| v as u32),
            waiting_for_subtask_id,
            handover_summary,
            cover_image,
            attachments,
            todos,
            priority,
            tags,
            current_tokens: current_tokens_i64.map(|v| v as usize),
            skip_worktree: Some(skip_worktree_i64 != 0),
            created_at,
            updated_at,
            stage_entered_at,
            completed_at,
            position,
            hook_states,
            project_path: Some(root.to_string_lossy().to_string()),
            stage_durations,
            total_elapsed,
        });
    }

    Ok(tasks)
}

/// Retrieves a single task and its checklist items by ID from SQLite.
pub fn db_get_task(root: &Path, task_id: &str) -> Result<Option<Task>, String> {
    let conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, title, description, stage_id, board_id, worktree_path, branch, verdict,
                    custom_flow, status, model, provider_id, reasoning_effort, parent_id, subtask_index, waiting_for_subtask_id,
                    handover_summary, cover_image, priority, current_tokens, skip_worktree,
                    created_at, updated_at, attachments, tags, position, hook_states, completed_at,
                    stage_entered_at, stage_durations, total_elapsed
             FROM tasks
             WHERE id = ?1 OR LOWER(id) = LOWER(?1)
             LIMIT 1",
        )
        .map_err(|e| format!("Prepare select task failed: {}", e))?;

    let mut rows = stmt
        .query(params![task_id])
        .map_err(|e| format!("Query task failed: {}", e))?;

    let row = match rows.next().map_err(|e| format!("Next row failed: {}", e))? {
        Some(r) => r,
        None => return Ok(None),
    };

    let id: String = row.get(0).map_err(|e| e.to_string())?;
    let title: String = row.get(1).map_err(|e| e.to_string())?;
    let description: String = row.get(2).map_err(|e| e.to_string())?;
    let stage_id: String = row.get(3).map_err(|e| e.to_string())?;
    let board_id: Option<String> = row.get(4).map_err(|e| e.to_string())?;
    let worktree_path: Option<String> = row.get(5).map_err(|e| e.to_string())?;
    let branch: Option<String> = row.get(6).map_err(|e| e.to_string())?;
    let verdict: Option<String> = row.get(7).map_err(|e| e.to_string())?;
    let custom_flow_raw: Option<String> = row.get(8).map_err(|e| e.to_string())?;
    let status: Option<String> = row.get(9).map_err(|e| e.to_string())?;
    let model: Option<String> = row.get(10).map_err(|e| e.to_string())?;
    let provider_id: Option<String> = row.get(11).map_err(|e| e.to_string())?;
    let reasoning_effort: Option<String> = row.get(12).ok();
    let parent_id: Option<String> = row.get(13).map_err(|e| e.to_string())?;
    let subtask_index_i64: Option<i64> = row.get(14).map_err(|e| e.to_string())?;
    let waiting_for_subtask_id: Option<String> = row.get(15).map_err(|e| e.to_string())?;
    let handover_summary: Option<String> = row.get(16).map_err(|e| e.to_string())?;
    let cover_image: Option<String> = row.get(17).map_err(|e| e.to_string())?;
    let priority: Option<String> = row.get(18).map_err(|e| e.to_string())?;
    let current_tokens_i64: Option<i64> = row.get(19).map_err(|e| e.to_string())?;
    let skip_worktree_i64: i64 = row.get(20).map_err(|e| e.to_string())?;
    let created_at: i64 = row.get(21).map_err(|e| e.to_string())?;
    let updated_at: i64 = row.get(22).map_err(|e| e.to_string())?;
    let attachments_raw: Option<String> = row.get(23).ok();
    let tags_raw: Option<String> = row.get(24).ok();
    let position: Option<f64> = row.get(25).ok();
    let hook_states_raw: Option<String> = row.get(26).ok();
    let completed_at: Option<i64> = row.get(27).ok();
    let stage_entered_at: Option<i64> = row.get(28).ok();
    let stage_durations_raw: Option<String> = row.get(29).ok();
    let total_elapsed: Option<i64> = row.get(30).ok();

    // Load todos for this task
    let mut todo_stmt = conn
        .prepare("SELECT id, content, status FROM task_todos WHERE task_id = ?1 ORDER BY order_idx ASC")
        .map_err(|e| format!("Prepare select todos failed: {}", e))?;
    let todo_rows = todo_stmt
        .query_map(params![&id], |r| {
            Ok(TodoItem {
                id: r.get(0)?,
                content: r.get(1)?,
                status: r.get(2)?,
            })
        })
        .map_err(|e| format!("Query todos failed: {}", e))?;

    let todos_vec: Vec<TodoItem> = todo_rows.flatten().collect();
    let todos = if todos_vec.is_empty() { None } else { Some(todos_vec) };

    let custom_flow = custom_flow_raw.and_then(|s| serde_json::from_str(&s).ok());
    let attachments = attachments_raw.and_then(|s| serde_json::from_str(&s).ok());
    let tags = tags_raw.and_then(|s| serde_json::from_str(&s).ok());
    let hook_states = hook_states_raw.and_then(|s| serde_json::from_str(&s).ok());
    let stage_durations = stage_durations_raw.and_then(|s| serde_json::from_str(&s).ok());

    Ok(Some(Task {
        id,
        title,
        description,
        stage_id,
        board_id,
        worktree_path,
        branch,
        verdict,
        custom_flow,
        status,
        model,
        provider_id,
        reasoning_effort,
        parent_id,
        subtask_index: subtask_index_i64.map(|v| v as u32),
        waiting_for_subtask_id,
        handover_summary,
        cover_image,
        attachments,
        todos,
        priority,
        tags,
        current_tokens: current_tokens_i64.map(|v| v as usize),
        skip_worktree: Some(skip_worktree_i64 != 0),
        created_at,
        updated_at,
        stage_entered_at,
        completed_at,
        position,
        hook_states,
        project_path: Some(root.to_string_lossy().to_string()),
        stage_durations,
        total_elapsed,
    }))
}

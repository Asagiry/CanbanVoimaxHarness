use std::collections::HashMap;
use crate::agent::{Task, TodoItem};
use crate::db::get_db_conn;
use rusqlite::{params, TransactionBehavior};
use std::path::Path;

/// Retrieves all tasks from SQLite ordered by position and created_at.
pub fn db_get_all_tasks(root: &Path) -> Result<Vec<Task>, String> {
    let conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, title, description, stage_id, board_id, worktree_path, branch, verdict,
                    custom_flow, status, model, parent_id, subtask_index, waiting_for_subtask_id,
                    handover_summary, cover_image, priority, current_tokens, skip_worktree,
                    created_at, updated_at, attachments, tags, position, hook_states, completed_at,
                    stage_entered_at
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
            let parent_id: Option<String> = row.get(11)?;
            let subtask_index_i64: Option<i64> = row.get(12)?;
            let waiting_for_subtask_id: Option<String> = row.get(13)?;
            let handover_summary: Option<String> = row.get(14)?;
            let cover_image: Option<String> = row.get(15)?;
            let priority: Option<String> = row.get(16)?;
            let current_tokens_i64: Option<i64> = row.get(17)?;
            let skip_worktree_i64: i64 = row.get(18)?;
            let created_at: i64 = row.get(19)?;
            let updated_at: i64 = row.get(20)?;
            let attachments_raw: Option<String> = row.get(21).ok();
            let tags_raw: Option<String> = row.get(22).ok();
            let position: Option<f64> = row.get(23).ok();
            let hook_states_raw: Option<String> = row.get(24).ok();
            let completed_at: Option<i64> = row.get(25).ok();
            let stage_entered_at: Option<i64> = row.get(26).ok();

            Ok((
                id, title, description, stage_id, board_id, worktree_path, branch, verdict,
                custom_flow_raw, status, model, parent_id, subtask_index_i64, waiting_for_subtask_id,
                handover_summary, cover_image, priority, current_tokens_i64, skip_worktree_i64,
                created_at, updated_at, attachments_raw, tags_raw, position, hook_states_raw, completed_at,
                stage_entered_at,
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
            custom_flow_raw, status, model, parent_id, subtask_index_i64, waiting_for_subtask_id,
            handover_summary, cover_image, priority, current_tokens_i64, skip_worktree_i64,
            created_at, updated_at, attachments_raw, tags_raw, position, hook_states_raw, completed_at,
            stage_entered_at,
        ) = row_res.map_err(|e| format!("Row extraction failed: {}", e))?;

        let custom_flow = custom_flow_raw.and_then(|s| serde_json::from_str(&s).ok());
        let attachments = attachments_raw.and_then(|s| serde_json::from_str(&s).ok());
        let tags = tags_raw.and_then(|s| serde_json::from_str(&s).ok());
        let hook_states = hook_states_raw.and_then(|s| serde_json::from_str(&s).ok());
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
        });
    }

    Ok(tasks)
}

/// Retrieves a single task by ID from SQLite.
pub fn db_get_task(root: &Path, task_id: &str) -> Result<Option<Task>, String> {
    let conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, title, description, stage_id, board_id, worktree_path, branch, verdict,
                    custom_flow, status, model, parent_id, subtask_index, waiting_for_subtask_id,
                    handover_summary, cover_image, priority, current_tokens, skip_worktree,
                    created_at, updated_at, attachments, tags, position, hook_states, completed_at,
                    stage_entered_at
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
    let parent_id: Option<String> = row.get(11).map_err(|e| e.to_string())?;
    let subtask_index_i64: Option<i64> = row.get(12).map_err(|e| e.to_string())?;
    let waiting_for_subtask_id: Option<String> = row.get(13).map_err(|e| e.to_string())?;
    let handover_summary: Option<String> = row.get(14).map_err(|e| e.to_string())?;
    let cover_image: Option<String> = row.get(15).map_err(|e| e.to_string())?;
    let priority: Option<String> = row.get(16).map_err(|e| e.to_string())?;
    let current_tokens_i64: Option<i64> = row.get(17).map_err(|e| e.to_string())?;
    let skip_worktree_i64: i64 = row.get(18).map_err(|e| e.to_string())?;
    let created_at: i64 = row.get(19).map_err(|e| e.to_string())?;
    let updated_at: i64 = row.get(20).map_err(|e| e.to_string())?;
    let attachments_raw: Option<String> = row.get(21).ok();
    let tags_raw: Option<String> = row.get(22).ok();
    let position: Option<f64> = row.get(23).ok();
    let hook_states_raw: Option<String> = row.get(24).ok();
    let completed_at: Option<i64> = row.get(25).ok();
    let stage_entered_at: Option<i64> = row.get(26).ok();

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
    }))
}

/// Atomically upserts a single task and its checklist items into SQLite.
pub fn db_upsert_task(root: &Path, task: &Task) -> Result<(), String> {
    let mut conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    let custom_flow_json = task.custom_flow.as_ref().and_then(|v| serde_json::to_string(v).ok());
    let attachments_json = task.attachments.as_ref().and_then(|v| serde_json::to_string(v).ok());
    let tags_json = task.tags.as_ref().and_then(|v| serde_json::to_string(v).ok());
    let hook_states_json = task.hook_states.as_ref().and_then(|v| serde_json::to_string(v).ok());
    let skip_wt = if task.skip_worktree.unwrap_or(false) { 1i64 } else { 0i64 };
    let cur_tokens = task.current_tokens.map(|v| v as i64);
    let sub_idx = task.subtask_index.map(|v| v as i64);
    let pos_val = task.position.unwrap_or(0.0);

    tx.execute(
        "INSERT INTO tasks (
            id, title, description, stage_id, board_id, worktree_path, branch, verdict,
            custom_flow, status, model, parent_id, subtask_index, waiting_for_subtask_id,
            handover_summary, cover_image, priority, current_tokens, skip_worktree,
            created_at, updated_at, completed_at, attachments, tags, position, hook_states,
            stage_entered_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27)
        ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            description = excluded.description,
            stage_id = excluded.stage_id,
            board_id = excluded.board_id,
            worktree_path = excluded.worktree_path,
            branch = excluded.branch,
            verdict = excluded.verdict,
            custom_flow = excluded.custom_flow,
            status = excluded.status,
            model = excluded.model,
            parent_id = excluded.parent_id,
            subtask_index = excluded.subtask_index,
            waiting_for_subtask_id = excluded.waiting_for_subtask_id,
            handover_summary = excluded.handover_summary,
            cover_image = excluded.cover_image,
            priority = excluded.priority,
            current_tokens = excluded.current_tokens,
            skip_worktree = excluded.skip_worktree,
            updated_at = excluded.updated_at,
            completed_at = excluded.completed_at,
            attachments = excluded.attachments,
            tags = excluded.tags,
            position = excluded.position,
            hook_states = excluded.hook_states,
            stage_entered_at = COALESCE(excluded.stage_entered_at, tasks.stage_entered_at, excluded.created_at)",
        params![
            task.id,
            task.title,
            task.description,
            task.stage_id,
            task.board_id.as_deref().unwrap_or("development"),
            task.worktree_path,
            task.branch,
            task.verdict,
            custom_flow_json,
            task.status.as_deref().unwrap_or("idle"),
            task.model,
            task.parent_id,
            sub_idx,
            task.waiting_for_subtask_id,
            task.handover_summary,
            task.cover_image,
            task.priority,
            cur_tokens,
            skip_wt,
            task.created_at,
            task.updated_at,
            task.completed_at,
            attachments_json,
            tags_json,
            pos_val,
            hook_states_json,
            task.stage_entered_at,
        ],
    )
    .map_err(|e| format!("Failed to upsert task {}: {}", task.id, e))?;

    if let Some(ref todos) = task.todos {
        let _ = tx.execute("DELETE FROM task_todos WHERE task_id = ?1", params![task.id]);
        for (idx, td) in todos.iter().enumerate() {
            tx.execute(
                "INSERT INTO task_todos (id, task_id, content, status, order_idx)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![td.id, task.id, td.content, td.status, idx as i64],
            )
            .map_err(|e| format!("Failed to insert todo {}: {}", td.id, e))?;
        }
    }

    tx.commit()
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;
    Ok(())
}

/// Atomically upserts a batch of tasks into SQLite in a single transaction.
pub fn db_upsert_all_tasks(root: &Path, tasks: &[Task]) -> Result<(), String> {
    let mut conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    // Disable foreign keys temporarily during bulk batch update to accommodate unordered cascades
    let _ = tx.execute("PRAGMA foreign_keys = OFF", []);

    for task in tasks {
        let custom_flow_json = task.custom_flow.as_ref().and_then(|v| serde_json::to_string(v).ok());
        let attachments_json = task.attachments.as_ref().and_then(|v| serde_json::to_string(v).ok());
        let tags_json = task.tags.as_ref().and_then(|v| serde_json::to_string(v).ok());
        let hook_states_json = task.hook_states.as_ref().and_then(|v| serde_json::to_string(v).ok());
        let skip_wt = if task.skip_worktree.unwrap_or(false) { 1i64 } else { 0i64 };
        let cur_tokens = task.current_tokens.map(|v| v as i64);
        let sub_idx = task.subtask_index.map(|v| v as i64);
        let pos_val = task.position.unwrap_or(0.0);

        tx.execute(
            "INSERT INTO tasks (
                id, title, description, stage_id, board_id, worktree_path, branch, verdict,
                custom_flow, status, model, parent_id, subtask_index, waiting_for_subtask_id,
                handover_summary, cover_image, priority, current_tokens, skip_worktree,
                created_at, updated_at, completed_at, attachments, tags, position, hook_states,
                stage_entered_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                description = excluded.description,
                stage_id = excluded.stage_id,
                board_id = excluded.board_id,
                worktree_path = excluded.worktree_path,
                branch = excluded.branch,
                verdict = excluded.verdict,
                custom_flow = excluded.custom_flow,
                status = excluded.status,
                model = excluded.model,
                parent_id = excluded.parent_id,
                subtask_index = excluded.subtask_index,
                waiting_for_subtask_id = excluded.waiting_for_subtask_id,
                handover_summary = excluded.handover_summary,
                cover_image = excluded.cover_image,
                priority = excluded.priority,
                current_tokens = excluded.current_tokens,
                skip_worktree = excluded.skip_worktree,
                updated_at = excluded.updated_at,
                completed_at = excluded.completed_at,
                attachments = excluded.attachments,
                tags = excluded.tags,
                position = excluded.position,
                hook_states = excluded.hook_states,
                stage_entered_at = COALESCE(excluded.stage_entered_at, tasks.stage_entered_at, excluded.created_at)",
            params![
                task.id,
                task.title,
                task.description,
                task.stage_id,
                task.board_id.as_deref().unwrap_or("development"),
                task.worktree_path,
                task.branch,
                task.verdict,
                custom_flow_json,
                task.status.as_deref().unwrap_or("idle"),
                task.model,
                task.parent_id,
                sub_idx,
                task.waiting_for_subtask_id,
                task.handover_summary,
                task.cover_image,
                task.priority,
                cur_tokens,
                skip_wt,
                task.created_at,
                task.updated_at,
                task.completed_at,
                attachments_json,
                tags_json,
                pos_val,
                hook_states_json,
                task.stage_entered_at,
            ],
        )
        .map_err(|e| format!("Failed to upsert task {}: {}", task.id, e))?;

        if let Some(ref todos) = task.todos {
            let _ = tx.execute("DELETE FROM task_todos WHERE task_id = ?1", params![task.id]);
            for (idx, td) in todos.iter().enumerate() {
                tx.execute(
                    "INSERT INTO task_todos (id, task_id, content, status, order_idx)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![td.id, task.id, td.content, td.status, idx as i64],
                )
                .map_err(|e| format!("Failed to insert todo {}: {}", td.id, e))?;
            }
        }
    }

    let _ = tx.execute("PRAGMA foreign_keys = ON", []);
    tx.commit()
        .map_err(|e| format!("Failed to commit batch task upsert: {}", e))?;
    Ok(())
}

/// Deletes a task and all associated cascaded subtasks and todos from SQLite.
pub fn db_delete_task(root: &Path, task_id: &str) -> Result<(), String> {
    let mut conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    // 1. Auto-unlock any parent waiting on this task
    let _ = tx.execute(
        "UPDATE tasks
         SET waiting_for_subtask_id = NULL,
             status = CASE WHEN status = 'waiting_subtask' THEN 'idle' ELSE status END
         WHERE waiting_for_subtask_id = ?1 OR LOWER(waiting_for_subtask_id) = LOWER(?1)",
        params![task_id],
    );

    // 2. Cascade delete subtasks where parent_id matches or id starts with "{task_id}."
    let subtask_pattern = format!("{}.%", task_id);
    let _ = tx.execute(
        "DELETE FROM task_todos WHERE task_id IN (
            SELECT id FROM tasks WHERE parent_id = ?1 OR LOWER(parent_id) = LOWER(?1) OR id LIKE ?2
        )",
        params![task_id, subtask_pattern],
    );
    let _ = tx.execute(
        "DELETE FROM tasks WHERE parent_id = ?1 OR LOWER(parent_id) = LOWER(?1) OR id LIKE ?2",
        params![task_id, subtask_pattern],
    );

    // 3. Delete target task and its todos
    let _ = tx.execute("DELETE FROM task_todos WHERE task_id = ?1 OR LOWER(task_id) = LOWER(?1)", params![task_id]);
    tx.execute("DELETE FROM tasks WHERE id = ?1 OR LOWER(id) = LOWER(?1)", params![task_id])
        .map_err(|e| format!("Failed to delete task {}: {}", task_id, e))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit delete transaction: {}", e))?;
    Ok(())
}

/// Allocates the next monotonic task number from SQLite sequences table.
pub fn db_allocate_next_task_num(root: &Path) -> Result<u32, String> {
    let mut conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    let current_val: u32 = tx
        .query_row(
            "SELECT current_val FROM sequences WHERE name = 'task'",
            [],
            |r| r.get::<_, i64>(0).map(|v| v.max(0) as u32),
        )
        .unwrap_or(0);

    let mut max_in_db: u32 = 0;
    {
        let mut stmt = tx
            .prepare("SELECT id FROM tasks")
            .map_err(|e| format!("Prepare query failed: {}", e))?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| format!("Query failed: {}", e))?;
        for id in rows.flatten() {
            let base = id.split('.').next().unwrap_or(&id);
            let digits: String = base.chars().filter(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = digits.parse::<u32>() {
                if n > max_in_db {
                    max_in_db = n;
                }
            }
        }
    }

    let mut next_num = current_val.max(max_in_db).saturating_add(1).max(1);

    loop {
        let candidate_id = format!("CVH-{:03}", next_num);
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?1 OR LOWER(id) = LOWER(?1))",
                params![candidate_id],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if !exists {
            break;
        }
        next_num = next_num.saturating_add(1);
    }

    tx.execute(
        "INSERT INTO sequences (name, current_val) VALUES ('task', ?1)
         ON CONFLICT(name) DO UPDATE SET current_val = excluded.current_val",
        params![next_num as i64],
    )
    .map_err(|e| format!("Failed to update sequence: {}", e))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit sequence allocation: {}", e))?;

    // Mirror to .cvh/settings/sequence.json and .cvh/task-sequence.json for external tools
    let settings_dir = root.join(".cvh").join("settings");
    let _ = std::fs::create_dir_all(&settings_dir);
    let seq_path = settings_dir.join("sequence.json");
    if let Ok(content) = serde_json::to_string_pretty(&serde_json::json!({
        "last_task_num": next_num
    })) {
        let _ = std::fs::write(&seq_path, &content);
        let alt_path = root.join(".cvh").join("task-sequence.json");
        let _ = std::fs::write(&alt_path, &content);
    }

    Ok(next_num)
}

/// Explicitly updates or sets the sequence counter for task IDs in SQLite and disk files.
pub fn db_update_task_sequence(root: &Path, val: u32) -> Result<(), String> {
    let conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    conn.execute(
        "INSERT INTO sequences (name, current_val) VALUES ('task', ?1)
         ON CONFLICT(name) DO UPDATE SET current_val = MAX(current_val, excluded.current_val)",
        params![val as i64],
    )
    .map_err(|e| format!("Failed to update sequence: {}", e))?;

    let settings_dir = root.join(".cvh").join("settings");
    let _ = std::fs::create_dir_all(&settings_dir);
    let seq_path = settings_dir.join("sequence.json");
    if let Ok(content) = serde_json::to_string_pretty(&serde_json::json!({
        "last_task_num": val
    })) {
        let _ = std::fs::write(&seq_path, &content);
        let alt_path = root.join(".cvh").join("task-sequence.json");
        let _ = std::fs::write(&alt_path, &content);
    }
    Ok(())
}

/// Atomically saves the checklist (todos) for a specific task.
pub fn db_save_todos(root: &Path, task_id: &str, todos: &[TodoItem]) -> Result<(), String> {
    let mut conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    tx.execute(
        "DELETE FROM task_todos WHERE task_id = ?1 OR LOWER(task_id) = LOWER(?1)",
        params![task_id],
    )
    .map_err(|e| format!("Failed to clear existing todos: {}", e))?;

    for (idx, td) in todos.iter().enumerate() {
        tx.execute(
            "INSERT INTO task_todos (id, task_id, content, status, order_idx)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![td.id, task_id, td.content, td.status, idx as i64],
        )
        .map_err(|e| format!("Failed to insert todo {}: {}", td.id, e))?;
    }

    let now = chrono::Utc::now().timestamp_millis();
    let _ = tx.execute(
        "UPDATE tasks SET updated_at = ?1 WHERE id = ?2 OR LOWER(id) = LOWER(?2)",
        params![now, task_id],
    );

    tx.commit()
        .map_err(|e| format!("Failed to commit save todos transaction: {}", e))?;
    Ok(())
}

/// Atomically marks all unfinished todos (status != 'completed') as 'completed' for a specific task.
/// Returns the updated list of todos if any exist, or Ok(None) if there were no todos.
pub fn db_complete_all_todos(root: &Path, task_id: &str) -> Result<Option<Vec<TodoItem>>, String> {
    let mut conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    let changed = tx
        .execute(
            "UPDATE task_todos SET status = 'completed' WHERE (task_id = ?1 OR LOWER(task_id) = LOWER(?1)) AND status != 'completed'",
            params![task_id],
        )
        .map_err(|e| format!("Failed to update task_todos: {}", e))?;

    let now = chrono::Utc::now().timestamp_millis();
    if changed > 0 {
        let _ = tx.execute(
            "UPDATE tasks SET updated_at = ?1 WHERE id = ?2 OR LOWER(id) = LOWER(?2)",
            params![now, task_id],
        );
    }

    let todos: Vec<TodoItem> = {
        let mut stmt = tx
            .prepare("SELECT id, content, status FROM task_todos WHERE task_id = ?1 OR LOWER(task_id) = LOWER(?1) ORDER BY order_idx ASC")
            .map_err(|e| format!("Failed to prepare select todos: {}", e))?;

        let rows = stmt
            .query_map(params![task_id], |r| {
                Ok(TodoItem {
                    id: r.get(0)?,
                    content: r.get(1)?,
                    status: r.get(2)?,
                })
            })
            .map_err(|e| format!("Failed to query task_todos: {}", e))?;

        rows.flatten().collect()
    };

    tx.commit()
        .map_err(|e| format!("Failed to commit complete todos transaction: {}", e))?;

    if todos.is_empty() {
        Ok(None)
    } else {
        Ok(Some(todos))
    }
}

/// Checks whether a task has any unfinished todos in SQLite.
pub fn db_has_unfinished_todos(root: &Path, task_id: &str) -> bool {
    let conn = match get_db_conn(root) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM task_todos WHERE (task_id = ?1 OR LOWER(task_id) = LOWER(?1)) AND status != 'completed'",
            params![task_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    count > 0
}

// Aliases for convenience
pub use db_get_all_tasks as db_load_tasks;

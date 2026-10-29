//! Planning & HITL Tool Handlers (todo_write, question)
//! Extracted from agent.rs per ub-mode Clean Code & Command Pattern principles.

use std::sync::atomic::Ordering;
use std::time::Duration;
use anyhow::{bail, Context, Result};
use tauri::Emitter;

use crate::agent::{TodoItem, ToolContext, QUESTION_REGISTRY};
use crate::db::ActivityItem;

pub async fn execute_todo_write(
    ctx: &ToolContext<'_>,
    args: &serde_json::Value,
) -> Result<String> {
    let repo_root = ctx.repo_root;
    let task_id = ctx.task_id;

    let raw_todos = args
        .get("todos")
        .and_then(|v| v.as_array())
        .context("Missing required parameter 'todos' (must be array)")?;

    let total_raw = raw_todos.len();

    let _min_items = ctx
        .tool_policy
        .tool_settings
        .get("todo_write")
        .and_then(|v| v.get("min_items"))
        .and_then(|v| v.as_u64())
        .unwrap_or(3) as usize;

    let max_items = ctx
        .tool_policy
        .tool_settings
        .get("todo_write")
        .and_then(|v| v.get("max_items"))
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;

    let mut parsed_todos: Vec<TodoItem> = Vec::new();
    let mut completed_cnt = 0;
    let mut in_progress_cnt = 0;
    let mut pending_cnt = 0;

    for (idx, item) in raw_todos.iter().enumerate() {
        if parsed_todos.len() >= max_items {
            break;
        }
        let content = item
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if content.is_empty() {
            continue;
        }
        let raw_status = item
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("pending")
            .trim()
            .to_lowercase();

        let status = match raw_status.as_str() {
            "completed" | "done" => {
                completed_cnt += 1;
                "completed".to_string()
            }
            "in_progress" | "running" => {
                in_progress_cnt += 1;
                "in_progress".to_string()
            }
            _ => {
                pending_cnt += 1;
                "pending".to_string()
            }
        };

        let id = item
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("todo-{}", idx + 1));

        parsed_todos.push(TodoItem { id, content, status });
    }

    // If task_id provided, atomically persist to .cvh/tasks.json or SQLite dialog
    if let Some(tid) = task_id {
        if tid.contains(".sub-") {
            let subagent_dialog_id = if tid.contains(":sub:") {
                tid.to_string()
            } else if let Some((parent_id, _)) = tid.split_once(".sub-") {
                format!("{}:sub:{}", parent_id, tid)
            } else {
                tid.to_string()
            };
            let todo_act = ActivityItem {
                id: format!("sub-todos-{}", chrono::Utc::now().timestamp_millis()),
                item_type: "todos".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                content: serde_json::to_string(&parsed_todos).ok(),
                tool: None,
                duration_ms: None,
                images: None,
            };
            let _ = crate::db::append_dialog_items(repo_root, &subagent_dialog_id, std::slice::from_ref(&todo_act));
            if let Some(ref app) = ctx.app_handle {
                let _ = app.emit("subagent:activity", serde_json::json!({
                    "subagentId": tid,
                    "activity": &todo_act,
                }));
            }
        } else {
            // Atomically save todos to SQLite table task_todos
            let _ = crate::db::db_save_todos(repo_root, tid, &parsed_todos);

            // Update in SQLite tasks table if task exists
            if let Ok(Some(mut db_task)) = crate::db::db_get_task(repo_root, tid) {
                db_task.todos = Some(parsed_todos.clone());
                db_task.updated_at = chrono::Utc::now().timestamp_millis();
                let _ = crate::db::db_upsert_task(repo_root, &db_task);
            }
        }
    }

    if total_raw > max_items {
        let truncated_count = total_raw - max_items;
        Ok(format!(
            "Updated todo list: {} completed, {} in progress, {} pending. (Note: {} items were truncated to stay within the max limit of {}).",
            completed_cnt, in_progress_cnt, pending_cnt, truncated_count, max_items
        ))
    } else {
        Ok(format!(
            "Updated todo list: {} completed, {} in progress, {} pending.",
            completed_cnt, in_progress_cnt, pending_cnt
        ))
    }
}

pub async fn execute_question(
    ctx: &ToolContext<'_>,
    args: &serde_json::Value,
) -> Result<String> {
    let repo_root = ctx.repo_root;
    let task_id = ctx.task_id;
    let abort_handle = &ctx.abort_handle;

    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .context("Missing required parameter 'text'")?;
    let options = args.get("options").cloned();
    let multiple = args.get("multiple").and_then(|v| v.as_bool()).unwrap_or(false);

    let question_id = uuid::Uuid::new_v4().to_string();
    let task_id_str = args
        .get("task_id")
        .or_else(|| args.get("taskId"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .or(task_id)
        .unwrap_or("unknown")
        .to_string();

    // 1. Thread-safe oneshot channel registration (zero-polling)
    let (tx, mut rx) = tokio::sync::oneshot::channel::<String>();
    {
        let mut reg = QUESTION_REGISTRY.lock().await;
        reg.insert(question_id.clone(), tx);
    }

    // 2. Persist activity item in SQLite dialog_items with type: "question"
    let timestamp_str = chrono::Utc::now().to_rfc3339();
    let question_payload = serde_json::json!({
        "taskId": task_id_str,
        "questionId": question_id,
        "text": text,
        "options": options,
        "multiple": multiple,
    });

    let activity_item = crate::db::ActivityItem {
        id: format!("q-{}", question_id),
        item_type: "question".to_string(),
        timestamp: timestamp_str,
        content: Some(text.to_string()),
        tool: Some(serde_json::json!({
            "id": question_id,
            "toolName": "question",
            "status": "running",
            "params": question_payload,
        })),
        duration_ms: None,
        images: None,
    };

    if let Err(e) = crate::db::append_dialog_items(repo_root, &task_id_str, &[activity_item]) {
        eprintln!("CVH [question]: Failed to persist question activity item: {}", e);
    }

    // 3. Emit Tauri event agent:question with { taskId, questionId, text, options, multiple }
    if let Some(ref app) = ctx.app_handle {
        let _ = app.emit(
            "agent:question",
            serde_json::json!({
                "taskId": task_id_str,
                "questionId": question_id,
                "text": text,
                "options": options,
                "multiple": multiple,
            }),
        );
    }

    // 4. Zero-Polling Tokio Pause: wait on rx.await (with abort check)
    let answer = loop {
        tokio::select! {
            res = &mut rx => {
                match res {
                    Ok(ans) => break ans,
                    Err(_) => bail!("Question channel was closed without an answer"),
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(250)) => {
                if abort_handle.load(Ordering::SeqCst) {
                    let mut reg = QUESTION_REGISTRY.lock().await;
                    reg.remove(&question_id);
                    bail!("Agent execution was aborted while waiting for answer to question");
                }
            }
        }
    };

    // 5. Update question activity item in SQLite dialog_items to locked/answered state
    let answered_item = crate::db::ActivityItem {
        id: format!("q-{}", question_id),
        item_type: "question".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        content: Some(text.to_string()),
        tool: Some(serde_json::json!({
            "id": question_id,
            "toolName": "question",
            "status": "success",
            "selectedAnswer": &answer,
            "params": {
                "taskId": task_id_str,
                "questionId": question_id,
                "text": text,
                "options": options,
                "multiple": multiple,
                "selectedAnswer": &answer,
            }
        })),
        duration_ms: None,
        images: None,
    };
    let _ = crate::db::append_dialog_items(repo_root, &task_id_str, &[answered_item]);

    Ok(answer)
}

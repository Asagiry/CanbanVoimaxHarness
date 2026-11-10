use anyhow::{bail, Result};
use tauri::{Emitter, Manager};

use super::params::*;
use crate::agent::{Task, ToolContext};

pub struct TaskInput {
    pub title: String,
    pub description: String,
    pub column_id: Option<String>,
    pub board_id: Option<String>,
    pub tags: Option<Vec<String>>,
    pub priority: Option<String>,
    pub model: Option<String>,
    pub is_subtask: Option<bool>,
    pub parent_id: Option<String>,
}

pub fn parse_task_input(item: &serde_json::Value, idx_hint: Option<usize>) -> Result<TaskInput> {
    let title_raw = item.get("title").and_then(|v| v.as_str()).ok_or_else(|| {
        match idx_hint {
            Some(idx) => anyhow::anyhow!("Missing 'title' in tasks[{}]", idx),
            None => anyhow::anyhow!("Missing required parameter 'title'"),
        }
    })?;
    let title = title_raw.trim().to_string();
    if title.is_empty() {
        match idx_hint {
            Some(idx) => bail!("'title' in tasks[{}] cannot be empty", idx),
            None => bail!("Parameter 'title' cannot be empty"),
        }
    }
    let description = item
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let column_id = get_str_param(item, STAGE_KEYS).map(str::to_string);
    let board_id = get_str_param(item, BOARD_KEYS).map(str::to_string);
    let tags = item.get("tags").and_then(|v| v.as_array()).map(|arr| {
        arr.iter().filter_map(|t| t.as_str().map(str::to_string)).collect::<Vec<_>>()
    });
    let priority = get_str_param(item, &["priority"]).map(str::to_string);
    let model = get_str_param(item, &["model"]).map(str::to_string);
    let is_subtask = get_bool_param(item, SUBTASK_KEYS);
    let parent_id = get_str_param(item, PARENT_ID_KEYS).map(str::to_string);

    Ok(TaskInput {
        title,
        description,
        column_id,
        board_id,
        tags,
        priority,
        model,
        is_subtask,
        parent_id,
    })
}

pub async fn execute_create_task(
    ctx: &ToolContext<'_>,
    args: &serde_json::Value,
    is_batch_call: bool,
) -> Result<String> {
    let repo_root = ctx.repo_root;
    let task_id = ctx.task_id;

    // Check if batch call (via create_tasks alias or tasks array in arguments)
    let is_batch = is_batch_call || args.get("tasks").and_then(|v| v.as_array()).is_some();

    let mut inputs = Vec::new();

    if let Some(tasks_arr) = args.get("tasks").and_then(|v| v.as_array()) {
        if tasks_arr.is_empty() {
            bail!("'tasks' array must not be empty");
        }
        for (idx, item) in tasks_arr.iter().enumerate() {
            inputs.push(parse_task_input(item, Some(idx))?);
        }
    } else {
        inputs.push(parse_task_input(args, None)?);
    }

    let mut tasks = crate::commands::load_tasks_from_disk(repo_root).await;
    let current_task_opt = task_id.and_then(|tid| tasks.iter().find(|t| t.id == tid).cloned());
    let boards_cfg = crate::config::load_boards_config(repo_root).await.ok();

    let mut created_results = Vec::new();
    let mut autorun_task_ids = Vec::new();
    let now = chrono::Utc::now().timestamp_millis();
    let mut next_num = crate::commands::tasks::allocate_next_task_num(repo_root, &tasks).await;

    let pipe_cols = crate::config::load_pipeline_config(repo_root).await.ok().map(|p| p.stages);

    for input in inputs {
        while tasks.iter().any(|t| t.id == format!("CVH-{:03}", next_num)) {
            next_num = next_num.saturating_add(1);
        }
        let new_id = format!("CVH-{:03}", next_num);
        let _ = crate::commands::tasks::save_task_sequence(repo_root, next_num).await;
        next_num = next_num.saturating_add(1);

        // Resolve target board (defaults to current task's board)
        let effective_board_id = if let Some(ref bid) = input.board_id {
            bid.clone()
        } else if let Some(ref ct) = current_task_opt {
            ct.board_id.clone().unwrap_or_else(|| {
                boards_cfg.as_ref().map(|b| b.active_board_id.clone()).unwrap_or_else(|| "development".to_string())
            })
        } else {
            boards_cfg.as_ref().map(|b| b.active_board_id.clone()).unwrap_or_else(|| "development".to_string())
        };

        let target_board_cols = boards_cfg.as_ref().and_then(|bc| {
            bc.boards.iter().find(|b| b.id == effective_board_id).map(|b| b.stages.clone())
        });

        // Resolve column (defaults to "backlog" or target board's backlog column)
        let effective_column_id = if let Some(ref cid) = input.column_id {
            cid.clone()
        } else if let Some(ref cols) = target_board_cols {
            if let Some(backlog_col) = cols.iter().find(|c| c.is_intake == Some(true) || crate::commands::tasks::stage_id_matches(&c.id, "backlog")) {
                backlog_col.id.clone()
            } else if let Some(first) = cols.first() {
                first.id.clone()
            } else {
                "backlog".to_string()
            }
        } else {
            "backlog".to_string()
        };

        let is_subtask = input.is_subtask.unwrap_or(false);
        let parent_id = if is_subtask {
            input.parent_id.or_else(|| task_id.map(str::to_string))
        } else {
            input.parent_id
        };

        let target_col = target_board_cols.as_ref().and_then(|cols| {
            cols.iter().find(|c| c.id == effective_column_id)
        }).or_else(|| {
            pipe_cols.as_ref().and_then(|cols| cols.iter().find(|c| c.id == effective_column_id))
        });

        let is_backlog = target_col.map(|c| c.is_intake == Some(true)).unwrap_or(false) || crate::commands::tasks::stage_id_matches(&effective_column_id, "backlog");
        let should_autorun = !is_backlog && target_col.map(|c| {
            c.auto_run == Some(true) || c.persona.is_some()
        }).unwrap_or(false);

        if should_autorun {
            autorun_task_ids.push(new_id.clone());
        }

        let new_task = Task {
            id: new_id.clone(),
            title: input.title.clone(),
            description: input.description.clone(),
            stage_id: effective_column_id.clone(),
            board_id: Some(effective_board_id.clone()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("idle".to_string()),
            model: input.model.or_else(|| current_task_opt.as_ref().and_then(|ct| ct.model.clone())),
            provider_id: current_task_opt.as_ref().and_then(|ct| ct.provider_id.clone()),
            reasoning_effort: current_task_opt.as_ref().and_then(|ct| ct.reasoning_effort.clone()),
            parent_id,
            subtask_index: None,
            waiting_for_subtask_id: None,
            handover_summary: None,
            cover_image: None,
            attachments: None,
            todos: None,
            priority: input.priority,
            tags: input.tags,
            current_tokens: None,
            skip_worktree: None,
            created_at: now,
            updated_at: now,
            stage_entered_at: Some(now),
            completed_at: None,
            position: None,
            hook_states: None,
            project_path: Some(ctx.repo_root.to_string_lossy().to_string()),
            stage_durations: None,
            total_elapsed: None,
        };

        tasks.push(new_task);

        created_results.push(serde_json::json!({
            "task_id": new_id,
            "title": input.title,
            "column_id": effective_column_id,
            "board_id": effective_board_id,
            "status": "created"
        }));
    }

    // Sync in-memory AppState
    if let Some(app) = ctx.app_handle.as_ref() {
        if let Some(state_ref) = app.try_state::<crate::commands::state::AppState>() {
            if let Ok(mut guard) = state_ref.tasks.lock() {
                *guard = tasks.clone();
            }
        }
    }

    // Persist to disk
    crate::commands::tasks::save_tasks_to_disk(repo_root, &tasks)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Emit tasks:updated via Tauri AppHandle
    if let Some(app) = ctx.app_handle.as_ref() {
        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
        for auto_tid in autorun_task_ids {
            let _ = app.emit("agent:autorun", serde_json::json!({ "taskId": &auto_tid }));
        }
    }

    // Record dialog activity in SQLite
    if let Some(parent_tid) = task_id {
        let act_content = if created_results.len() == 1 {
            format!(
                "### Оркестратор создал задачу\n- **ID**: {}\n- **Название**: {}\n- **Колонка**: {}\n- **Доска**: {}",
                created_results[0]["task_id"].as_str().unwrap_or_default(),
                created_results[0]["title"].as_str().unwrap_or_default(),
                created_results[0]["column_id"].as_str().unwrap_or_default(),
                created_results[0]["board_id"].as_str().unwrap_or_default()
            )
        } else {
            let mut msg = format!("### Оркестратор пакетно создал {} задач:\n", created_results.len());
            for item in &created_results {
                msg.push_str(&format!(
                    "- **{}**: {} ({})\n",
                    item["task_id"].as_str().unwrap_or_default(),
                    item["title"].as_str().unwrap_or_default(),
                    item["column_id"].as_str().unwrap_or_default()
                ));
            }
            msg
        };
        let _ = crate::commands::dialogs::append_task_dialog_activity(
            repo_root,
            parent_tid,
            "orchestration_create",
            &act_content,
            ctx.app_handle.as_ref(),
        ).await;
    }

    if !is_batch && created_results.len() == 1 {
        Ok(serde_json::to_string_pretty(&created_results[0])?)
    } else {
        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "status": "created",
            "count": created_results.len(),
            "tasks": created_results
        }))?)
    }
}

//! Task Management Tool Handlers (create_task, complete_task, list_tasks, get_task_history)
//! Extracted from agent.rs per ub-mode Clean Code & Command Pattern principles.

use anyhow::{bail, Context, Result};
use serde::Serialize;
use tauri::{Emitter, Manager};

use crate::agent::{get_stage_personas, resolve_task_role, Task, ToolContext};

pub async fn execute_create_task(
    ctx: &ToolContext<'_>,
    args: &serde_json::Value,
    is_batch_call: bool,
) -> Result<String> {
    let repo_root = ctx.repo_root;
    let task_id = ctx.task_id;

    // Check if batch call (via create_tasks alias or tasks array in arguments)
    let is_batch = is_batch_call || args.get("tasks").and_then(|v| v.as_array()).is_some();

    struct TaskInput {
        title: String,
        description: String,
        column_id: Option<String>,
        board_id: Option<String>,
        tags: Option<Vec<String>>,
        priority: Option<String>,
        model: Option<String>,
        is_subtask: Option<bool>,
        parent_id: Option<String>,
    }

    let mut inputs = Vec::new();

    if let Some(tasks_arr) = args.get("tasks").and_then(|v| v.as_array()) {
        if tasks_arr.is_empty() {
            bail!("'tasks' array must not be empty");
        }
        for (idx, item) in tasks_arr.iter().enumerate() {
            let title = item
                .get("title")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing 'title' in tasks[{}]", idx))?
                .trim()
                .to_string();
            if title.is_empty() {
                bail!("'title' in tasks[{}] cannot be empty", idx);
            }
            let description = item
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let column_id = item
                .get("column_id")
                .or_else(|| item.get("columnId"))
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let board_id = item
                .get("board_id")
                .or_else(|| item.get("boardId"))
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let tags = item.get("tags").and_then(|v| v.as_array()).map(|arr| {
                arr.iter().filter_map(|t| t.as_str().map(str::to_string)).collect::<Vec<_>>()
            });
            let priority = item
                .get("priority")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let model = item
                .get("model")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let is_subtask = item
                .get("is_subtask")
                .or_else(|| item.get("isSubtask"))
                .and_then(|v| v.as_bool());
            let parent_id = item
                .get("parent_id")
                .or_else(|| item.get("parentId"))
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);

            inputs.push(TaskInput {
                title,
                description,
                column_id,
                board_id,
                tags,
                priority,
                model,
                is_subtask,
                parent_id,
            });
        }
    } else {
        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .context("Missing required parameter 'title'")?
            .trim()
            .to_string();
        if title.is_empty() {
            bail!("Parameter 'title' cannot be empty");
        }
        let description = args
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let column_id = args
            .get("column_id")
            .or_else(|| args.get("columnId"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let board_id = args
            .get("board_id")
            .or_else(|| args.get("boardId"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let tags = args.get("tags").and_then(|v| v.as_array()).map(|arr| {
            arr.iter().filter_map(|t| t.as_str().map(str::to_string)).collect::<Vec<_>>()
        });
        let priority = args
            .get("priority")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let model = args
            .get("model")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let is_subtask = args
            .get("is_subtask")
            .or_else(|| args.get("isSubtask"))
            .and_then(|v| v.as_bool());
        let parent_id = args
            .get("parent_id")
            .or_else(|| args.get("parentId"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        inputs.push(TaskInput {
            title,
            description,
            column_id,
            board_id,
            tags,
            priority,
            model,
            is_subtask,
            parent_id,
        });
    }

    let mut tasks = crate::commands::load_tasks_from_disk(repo_root).await;
    let current_task_opt = task_id.and_then(|tid| tasks.iter().find(|t| t.id == tid).cloned());
    let boards_cfg = crate::config::load_boards_config(repo_root).await.ok();

    let mut created_results = Vec::new();
    let mut autorun_task_ids = Vec::new();
    let now = chrono::Utc::now().timestamp_millis();
    let mut next_num = crate::commands::tasks::allocate_next_task_num(repo_root, &tasks).await;

    let pipe_cols = crate::config::load_pipeline_config(repo_root).await.ok().map(|p| p.columns);

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
            bc.boards.iter().find(|b| b.id == effective_board_id).map(|b| b.columns.clone())
        });

        // Resolve column (defaults to "backlog" or target board's backlog column)
        let effective_column_id = if let Some(ref cid) = input.column_id {
            cid.clone()
        } else if let Some(ref cols) = target_board_cols {
            if let Some(backlog_col) = cols.iter().find(|c| crate::commands::tasks::stage_id_matches(&c.id, "backlog")) {
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

        let is_backlog = crate::commands::tasks::stage_id_matches(&effective_column_id, "backlog");
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

pub async fn execute_complete_task(
    ctx: &ToolContext<'_>,
    args: &serde_json::Value,
) -> Result<String> {
    let repo_root = ctx.repo_root;
    let task_id = ctx.task_id;

    let summary = args
        .get("summary")
        .and_then(|v| v.as_str())
        .context("Missing required parameter 'summary'")?;

    let current_task_id = args
        .get("task_id")
        .or_else(|| args.get("taskId"))
        .and_then(|v| v.as_str())
        .or(task_id)
        .context("No active task ID for complete_task")?;

    let mut tasks = crate::commands::load_tasks_from_disk(repo_root).await;
    let current_task_idx = tasks
        .iter()
        .position(|t| crate::commands::task_id_matches(&t.id, current_task_id))
        .ok_or_else(|| anyhow::anyhow!("Task {} not found", current_task_id))?;

    let current_task = &tasks[current_task_idx];

    let boards_cfg = crate::config::load_boards_config(repo_root).await.ok();
    let board_cols = boards_cfg.as_ref().and_then(|bc| {
        let b = if let Some(ref bid) = current_task.board_id {
            bc.boards.iter().find(|b| &b.id == bid)
        } else {
            bc.boards.iter().find(|b| b.id == bc.active_board_id)
        };
        b.map(|b| b.columns.clone())
    });

    // Find terminal column (ready / done)
    let columns = match board_cols {
        Some(cols) => cols,
        None => {
            let pipeline = crate::config::load_pipeline_config(repo_root).await
                .context("Failed to load pipeline configuration")?;
            pipeline.columns
        }
    };

    let terminal_col = columns
        .iter()
        .find(|c| {
            c.terminal == Some(true)
                || crate::commands::tasks::stage_id_matches(&c.id, "ready")
                || crate::commands::tasks::stage_id_matches(&c.id, "done")
        })
        .or_else(|| columns.last())
        .ok_or_else(|| anyhow::anyhow!("No terminal column found in board/pipeline"))?;
    let terminal_col_id = terminal_col.id.clone();
    let old_stage = tasks[current_task_idx].stage_id.clone();

    tasks[current_task_idx].stage_id = terminal_col_id.clone();
    tasks[current_task_idx].status = Some("done".to_string());
    tasks[current_task_idx].verdict = Some("APPROVED".to_string());
    tasks[current_task_idx].handover_summary = Some(summary.to_string());
    tasks[current_task_idx].updated_at = chrono::Utc::now().timestamp_millis();
    let updated_task = tasks[current_task_idx].clone();

    // In-memory state sync
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

    // Emit events
    if let Some(app) = ctx.app_handle.as_ref() {
        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
        let _ = app.emit(
            "task:moved",
            serde_json::json!({
                "taskId": current_task_id,
                "fromStage": old_stage,
                "toStage": &terminal_col_id,
                "stageId": &terminal_col_id,
                "status": "done",
                "verdict": "APPROVED",
                "task": &updated_task
            }),
        );
    }

    // Append dialog activity in SQLite
    let _ = crate::commands::dialogs::append_task_dialog_activity(
        repo_root,
        current_task_id,
        "orchestration_complete",
        &format!("### Оркестратор завершил задачу\n\n**Итоговое саммари декомпозиции:**\n{}", summary),
        ctx.app_handle.as_ref(),
    ).await;

    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "task_id": current_task_id,
        "title": updated_task.title,
        "column_id": terminal_col_id,
        "status": "done",
        "handover_summary": summary
    }))?)
}

pub async fn execute_list_tasks(
    ctx: &ToolContext<'_>,
    args: &serde_json::Value,
) -> Result<String> {
    let repo_root = ctx.repo_root;

    let mut tasks = crate::db::db_get_all_tasks(repo_root)
        .map_err(|e| anyhow::anyhow!("Failed to read tasks from database: {}", e))?;

    if let Some(target_stage) = args.get("stage_id").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()) {
        let target = target_stage.trim();
        tasks.retain(|t| t.stage_id.eq_ignore_ascii_case(target));
    }

    if let Some(q) = args.get("query").and_then(|v| v.as_str()).map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()) {
        tasks.retain(|t| {
            t.id.to_lowercase().contains(&q)
                || t.title.to_lowercase().contains(&q)
                || t.handover_summary.as_ref().map(|h| h.to_lowercase().contains(&q)).unwrap_or(false)
        });
    }

    // Sort by updated_at / created_at (fresh first)
    tasks.sort_by(|a, b| {
        let time_b = if b.updated_at > 0 { b.updated_at } else { b.created_at };
        let time_a = if a.updated_at > 0 { a.updated_at } else { a.created_at };
        time_b.cmp(&time_a)
    });

    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|v| (v as usize).clamp(1, 50))
        .unwrap_or(20);

    tasks.truncate(limit);

    let personas = get_stage_personas(repo_root).await;

    #[derive(Serialize)]
    struct CompactTaskItem {
        id: String,
        title: String,
        stage_id: String,
        status: String,
        role: String,
        priority: String,
        handover_summary: Option<String>,
        updated_at: i64,
    }

    let compact_list: Vec<CompactTaskItem> = tasks
        .into_iter()
        .map(|t| {
            let role = resolve_task_role(&t.stage_id, &personas);
            let status = t.status.unwrap_or_else(|| "idle".to_string());
            let priority = t.priority.unwrap_or_else(|| "medium".to_string());
            let updated_at = if t.updated_at > 0 { t.updated_at } else { t.created_at };
            CompactTaskItem {
                id: t.id,
                title: t.title,
                stage_id: t.stage_id,
                status,
                role,
                priority,
                handover_summary: t.handover_summary,
                updated_at,
            }
        })
        .collect();

    Ok(serde_json::to_string_pretty(&compact_list)?)
}

pub async fn execute_get_task_history(
    ctx: &ToolContext<'_>,
    args: &serde_json::Value,
) -> Result<String> {
    let repo_root = ctx.repo_root;

    let target_task_id = args
        .get("task_id")
        .or_else(|| args.get("taskId"))
        .and_then(|v| v.as_str())
        .context("Missing required parameter 'task_id'")?;
    let include_tool_outputs = args
        .get("include_tool_outputs")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let maybe_task = crate::db::db_get_task(repo_root, target_task_id)
        .map_err(|e| anyhow::anyhow!("Database error loading task '{}': {}", target_task_id, e))?;

    let task = maybe_task.ok_or_else(|| anyhow::anyhow!("Task '{}' not found", target_task_id))?;
    let items = crate::db::load_dialog_items(repo_root, &task.id)
        .unwrap_or_default();

    let personas = get_stage_personas(repo_root).await;
    let role = resolve_task_role(&task.stage_id, &personas);
    let status = task.status.as_deref().unwrap_or("idle");
    let mut report = String::new();
    report.push_str(&format!(
        "# Task History: {} - {} (Этап: {}, Статус: {}, Роль: {})\n\n",
        task.id, task.title, task.stage_id, status, role
    ));

    report.push_str("## Initial Objective:\n");
    let desc = task.description.trim();
    if desc.is_empty() {
        report.push_str("(No description provided)\n\n");
    } else {
        report.push_str(desc);
        report.push_str("\n\n");
    }

    report.push_str("## Execution Steps:\n");
    let mut step_num = 1;
    let mut last_terminal_text: Option<String> = None;

    for it in &items {
        match it.item_type.as_str() {
            "steer" | "system" | "prompt" => {
                let content = it.content.as_deref().unwrap_or("").trim();
                if !content.is_empty() {
                    report.push_str(&format!("- Шаг {} [User Steer / System]: {}\n", step_num, content));
                    step_num += 1;
                }
            }
            "tool" => {
                let (tool_name, params_str, status_str, duration, error_snippet, output_snippet) = if let Some(ref t) = it.tool {
                    let name = t.get("toolName").or_else(|| t.get("name")).and_then(|v| v.as_str()).unwrap_or("unknown");
                    let params = t.get("params").or_else(|| t.get("arguments")).cloned().unwrap_or_else(|| serde_json::json!({}));
                    let params_str = serde_json::to_string(&params).unwrap_or_else(|_| "{}".to_string());
                    let status = t.get("status").and_then(|v| v.as_str()).unwrap_or("success");
                    let duration = t.get("durationMs").and_then(|v| v.as_f64()).or(it.duration_ms).unwrap_or(0.0);
                    let err = t.get("stderr").or_else(|| t.get("error")).and_then(|v| v.as_str())
                        .or_else(|| if status == "failed" { it.content.as_deref() } else { None })
                        .unwrap_or("none");
                    let out = t.get("stdout").or_else(|| t.get("result")).and_then(|v| v.as_str())
                        .or(it.content.as_deref());
                    (name, params_str, status, duration, err, out)
                } else {
                    ("unknown", "{}".to_string(), "unknown", it.duration_ms.unwrap_or(0.0), "none", it.content.as_deref())
                };

                report.push_str(&format!(
                    "- Шаг {} [Tool {}]: params: {}, status: {}, duration: {:.0}ms, error: {}.\n",
                    step_num, tool_name, params_str, status_str, duration, error_snippet
                ));

                if include_tool_outputs {
                    if let Some(out) = output_snippet {
                        let trimmed = out.trim();
                        if !trimmed.is_empty() {
                            let compacted = if trimmed.chars().count() > 300 {
                                format!("{}... [compacted]", trimmed.chars().take(300).collect::<String>())
                            } else {
                                trimmed.to_string()
                            };
                            report.push_str(&format!("  Output: {}\n", compacted));
                        }
                    }
                }
                step_num += 1;
                if let Some(ref c) = it.content {
                    if !c.trim().is_empty() {
                        last_terminal_text = Some(c.trim().to_string());
                    }
                }
            }
            "output" | "assistant" | "response" => {
                if let Some(ref c) = it.content {
                    if !c.trim().is_empty() {
                        last_terminal_text = Some(c.trim().to_string());
                    }
                }
            }
            _ => {}
        }
    }

    if step_num == 1 {
        report.push_str("(No execution steps recorded)\n");
    }
    report.push('\n');

    report.push_str("## Handover & Terminal Response:\n");
    let handover = task.handover_summary.as_deref().filter(|s| !s.trim().is_empty()).unwrap_or("None");
    report.push_str(&format!("- Handover Summary: {}\n", handover));

    let verdict_text = match (task.verdict.as_deref(), last_terminal_text.as_deref()) {
        (Some(v), Some(out)) => format!("[{}] {}", v, out),
        (Some(v), None) => format!("[{}]", v),
        (None, Some(out)) => out.to_string(),
        (None, None) => "None".to_string(),
    };
    report.push_str(&format!("- Final Output / Verdict: {}\n", verdict_text));

    Ok(report)
}

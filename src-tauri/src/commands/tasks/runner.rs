//! Agent Execution, Abort, Compaction, and Steering Subsystem
//! Extracted from commands/tasks.rs per ub-mode Clean Code principles.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::agent::{run_agent_loop, AgentEvent, AgentSessionConfig, Task};
use crate::commands::state::{ActiveAgentHandle, AppState};
use crate::config::{
    load_boards_config, load_llm_settings, load_pipeline_config, load_project_config, ProjectConfig,
};
use super::healing::task_id_matches;
use super::storage::{load_tasks_from_disk, save_tasks_to_disk};

pub fn focus_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
        Ok(())
    } else {
        Err("Main window not found".to_string())
    }
}

/// Resolves the effective active agent model for a task, following the canonical priority hierarchy:
/// 1. Task-level model override (`task.model`)
/// 2. Global LLM settings model (`llm_settings.model`)
/// 3. Stage persona agent settings (`agent_setting.model`)
/// 4. Column model override (`col.model_override`)
/// 5. Project default model (`proj_cfg.default_model`)
/// 6. Hardcoded fallback ("gemini-3.8-flash-high")
pub async fn resolve_task_active_agent_model(
    root: &Path,
    task: Option<&Task>,
    llm_settings: &crate::config::LlmSettings,
) -> String {
    fn non_empty_opt(v: &Option<String>) -> Option<String> {
        v.as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    if let Some(m) = task.and_then(|t| non_empty_opt(&t.model)) {
        return m;
    }

    let llm_model = {
        let m = llm_settings.model.trim();
        if m.is_empty() {
            None
        } else {
            Some(m.to_string())
        }
    };

    let (col_override, agent_model) = if let Some(t) = task {
        let pipeline = load_pipeline_config(root)
            .await
            .unwrap_or_else(|_| crate::config::default_pipeline_config());
        let boards_cfg = load_boards_config(root).await.ok();
        let col_from_board = boards_cfg.as_ref().and_then(|bc| {
            let b = if let Some(ref bid) = t.board_id {
                bc.boards.iter().find(|b| &b.id == bid)
            } else {
                bc.boards.iter().find(|b| b.id == bc.active_board_id)
            };
            b.and_then(|b| b.columns.iter().find(|c| c.id == t.stage_id).cloned())
        });
        let col = col_from_board
            .or_else(|| pipeline.columns.iter().find(|c| c.id == t.stage_id).cloned());
        let col_override = col.as_ref().and_then(|c| non_empty_opt(&c.model_override));
        let persona_role = col
            .as_ref()
            .and_then(|c| c.persona.clone())
            .unwrap_or_else(|| "developer".to_string());
        let agent_setting = crate::config::load_agent_setting(root, &persona_role).await;
        (col_override, non_empty_opt(&agent_setting.model))
    } else {
        (None, None)
    };

    let proj_cfg = load_project_config(root).await.ok();
    let proj_default = proj_cfg.and_then(|p| non_empty_opt(&p.default_model));

    llm_model
        .or(agent_model)
        .or(col_override)
        .or(proj_default)
        .unwrap_or_else(|| "gemini-3.8-flash-high".to_string())
}

pub async fn run_agent(
    task_id: String,
    prompt: Option<String>,
    images: Option<Vec<String>>,
    app_handle: AppHandle,
) -> Result<String, String> {
    let root = app_handle.state::<AppState>().get_root();
    crate::logger::cvh_log_with_details(
        Some(&root),
        "INFO",
        "RUN_AGENT",
        &format!("run_agent IPC called for task_id='{}', prompt_len={}", task_id, prompt.as_ref().map(|p| p.len()).unwrap_or(0)),
        Some(&format!("root='{}'", root.display())),
    );
    {
        let is_running = {
            let state = app_handle.state::<AppState>();
            let mut is_active = false;
            if let Ok(mut map) = state.active_agents.lock() {
                // Sweep completed or aborted handles first
                let mut stale_keys = Vec::new();
                for (k, entry) in map.iter() {
                    let aborted = entry.abort_flag.load(Ordering::SeqCst);
                    let finished = if let Ok(guard) = entry.join_handle.try_lock() {
                        guard.as_ref().map(|h| h.is_finished()).unwrap_or(true)
                    } else {
                        false
                    };
                    if aborted || finished {
                        stale_keys.push(k.clone());
                    }
                }
                for k in stale_keys {
                    map.remove(&k);
                }

                is_active = map.keys().any(|k| task_id_matches(k, &task_id));
            }
            is_active
        };
        if is_running {
            crate::logger::cvh_log(
                Some(&root),
                "WARN",
                "RUN_AGENT",
                &format!("run_agent: Task '{}' already has an active agent loop, rejecting duplicate run", task_id),
            );
            return Ok(format!("Agent already running for task {}", task_id));
        }
    }

    // Check disk and in-memory state: merge them so in-memory tasks are never lost (§ 2.22).
    let mut tasks_to_use = load_tasks_from_disk(&root).await;
    let initial_disk_count = tasks_to_use.len();
    if let Ok(guard) = app_handle.state::<AppState>().tasks.lock() {
        for mem_task in guard.iter() {
            if !tasks_to_use.iter().any(|t| task_id_matches(&t.id, &mem_task.id)) {
                tasks_to_use.push(mem_task.clone());
            }
        }
    }

    let task_opt = tasks_to_use.iter().find(|t| task_id_matches(&t.id, &task_id)).cloned();

    let mut task = match task_opt {
        Some(t) => {
            if let Ok(mut guard) = app_handle.state::<AppState>().tasks.lock() {
                *guard = tasks_to_use.clone();
            }
            if tasks_to_use.len() > initial_disk_count {
                let _ = save_tasks_to_disk(&root, &tasks_to_use).await;
            }
            t
        }
        None => {
            let mem_count = app_handle.state::<AppState>().tasks.lock().map(|g| g.len()).unwrap_or(0);
            let available_ids: Vec<String> = tasks_to_use.iter().map(|t| t.id.clone()).collect();
            let err_msg = format!("Task {} not found", task_id);
            crate::logger::cvh_log_with_details(
                Some(&root),
                "ERROR",
                "RUN_AGENT",
                &format!("run_agent REJECTED: Task '{}' not found (in-memory: {}, disk: {})", task_id, mem_count, tasks_to_use.len()),
                Some(&format!("Available tasks in SQLite: [{}]", available_ids.join(", "))),
            );
            return Err(err_msg);
        }
    };

    let steer_prompt = if let Some(p) = prompt {
        let p_trimmed = p.trim();
        if !p_trimmed.is_empty() {
            let has_existing_context = app_handle
                .state::<AppState>()
                .agent_contexts
                .lock()
                .ok()
                .and_then(|map| map.get(&task_id).and_then(|cs| cs.lock().map(|g| g.messages.len() > 2).ok()))
                .unwrap_or(false);

            if task.description.trim().is_empty() && !has_existing_context && images.is_none() {
                task.description = p_trimmed.to_string();
                task.updated_at = Utc::now().timestamp_millis();

                // Persist description update immediately
                let mut all_tasks = load_tasks_from_disk(&root).await;
                if let Some(t) = all_tasks.iter_mut().find(|t| task_id_matches(&t.id, &task_id)) {
                    t.description = task.description.clone();
                    t.updated_at = task.updated_at;
                    let _ = save_tasks_to_disk(&root, &all_tasks).await;
                }
                if let Ok(mut guard) = app_handle.state::<AppState>().tasks.lock() {
                    if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, &task_id)) {
                        t.description = task.description.clone();
                        t.updated_at = task.updated_at;
                    }
                }
                None
            } else if p_trimmed == task.description.trim() && images.is_none() {
                // If prompt matches task description, this is an autorun / task run, NOT a steer!
                None
            } else {
                Some(p_trimmed.to_string())
            }
        } else {
            None
        }
    } else {
        None
    };

    // Strict guard against duplicate runs while waiting for subtask (§ 1):
    if task.status.as_deref() == Some("waiting_subtask") && steer_prompt.is_none() {
        crate::logger::cvh_log(
            Some(&root),
            "WARN",
            "RUN_AGENT",
            &format!(
                "run_agent: Task '{}' is waiting for subtask to finish (waiting_for_subtask_id={:?}), rejecting autorun",
                task_id, task.waiting_for_subtask_id
            ),
        );
        return Ok(format!("Task {} is waiting for subtask to finish", task_id));
    }

    let pipeline = load_pipeline_config(&root)
        .await
        .unwrap_or_else(|_| crate::config::default_pipeline_config());

    let col = {
        let boards_cfg = load_boards_config(&root).await.ok();
        let col_from_board = boards_cfg.as_ref().and_then(|bc| {
            let b = if let Some(ref bid) = task.board_id {
                bc.boards.iter().find(|b| &b.id == bid)
            } else {
                bc.boards.iter().find(|b| b.id == bc.active_board_id)
            };
            b.and_then(|b| b.columns.iter().find(|c| c.id == task.stage_id).cloned())
        });
        col_from_board
            .or_else(|| pipeline.columns.iter().find(|c| c.id == task.stage_id).cloned())
            .unwrap_or_else(|| {
                eprintln!(
                    "CVH [run_agent]: Column '{}' not found in board or pipeline, falling back to default column",
                    task.stage_id
                );
                pipeline.columns.first().cloned().unwrap_or_else(|| crate::config::PipelineColumn {
                    id: task.stage_id.clone(),
                    title: task.stage_id.clone(),
                    color: None,
                    icon: None,
                    persona: Some("developer".to_string()),
                    on_enter_script: None,
                    context_window: None,
                    model_override: None,
                    reasoning_effort: None,
                    temperature: Some(0.2),
                    gatekeeper_sensor: None,
                    allowed_tools: None,
                    hooks: HashMap::new(),
                    routes: None,
                    auto_run: None,
                    terminal: None,
                    is_intake: None,
                })
            })
    };

    let persona_role = col.persona.clone().unwrap_or_else(|| "developer".to_string());

    let proj_cfg = load_project_config(&root)
        .await
        .unwrap_or_else(|_| ProjectConfig {
            name: "CanbanVoimaxHarness".to_string(),
            version: "0.1.0".to_string(),
            description: None,
            active_pipeline: None,
            default_model: Some("gemini-3.8-flash-high".to_string()),
            api_base_url: Some("http://127.0.0.1:8045/v1".to_string()),
            tool_permissions: None,
            providers: HashMap::new(),
        });

    let llm_settings = load_llm_settings(&root)
        .await
        .unwrap_or_default();

    let agent_setting = crate::config::load_agent_setting(&root, &persona_role).await;
    let resolved_context_window = agent_setting.context_window.unwrap_or(128_000);
    eprintln!(
        "CVH [run_agent]: task_id={} persona='{}' context_window={}",
        task.id, persona_role, resolved_context_window
    );

    let model = resolve_task_active_agent_model(&root, Some(&task), &llm_settings).await;

    let mut base_url = if !llm_settings.base_url.trim().is_empty() {
        llm_settings.base_url.trim().to_string()
    } else {
        proj_cfg
            .api_base_url
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:8045/v1".to_string())
    };

    if base_url.trim().is_empty() {
        base_url = "http://127.0.0.1:8045/v1".to_string();
    }

    let api_key = if !llm_settings.api_key.trim().is_empty() {
        Some(llm_settings.api_key.trim().to_string())
    } else {
        None
    };

    eprintln!(
        "CVH [run_agent]: task_id={}, model={}, base_url={}, api_key_present={}",
        task.id,
        model,
        base_url,
        api_key.is_some()
    );

    let temperature = llm_settings
        .temperature
        .or(agent_setting.temperature)
        .or(col.temperature);

    let worktree_dir = match &task.worktree_path {
        Some(p) if !p.trim().is_empty() => {
            let pb = Path::new(p.trim());
            let abs = if pb.is_absolute() {
                pb.to_path_buf()
            } else {
                root.join(pb)
            };
            if abs.exists() {
                abs
            } else {
                eprintln!(
                    "CVH [run_agent]: WARN task {} worktree {:?} does not exist — creating is forbidden, falling back to repo root {:?}",
                    task_id, p, root
                );
                root.clone()
            }
        }
        _ => {
            eprintln!(
                "CVH [run_agent]: WARN task {} has no worktree_path — creating is forbidden, falling back to repo root {:?}",
                task_id, root
            );
            root.clone()
        }
    };

    let abort_flag = Arc::new(AtomicBool::new(false));
    let join_slot = Arc::new(tokio::sync::Mutex::new(None));
    let session_id = format!("sess-{}-{}", task_id, Utc::now().timestamp_millis());
    let session_started_at = Utc::now().timestamp_millis() as u64;
    let session_summary = crate::agent::session_registry::AgentSessionSummary {
        session_id: session_id.clone(),
        task_id: task_id.clone(),
        stage_id: Some(task.stage_id.clone()),
        started_at: session_started_at,
        status: crate::agent::session_registry::SessionStatus::Running,
    };

    // Store in active agents registry & authoritative session registry
    {
        let state = app_handle.state::<AppState>();
        state.session_registry.register(
            session_summary.clone(),
            Arc::clone(&abort_flag),
            Arc::clone(&join_slot),
        );
        let mut map = state
            .active_agents
            .lock()
            .map_err(|_| "Failed to lock active agents".to_string())?;
        map.insert(
            task_id.clone(),
            ActiveAgentHandle {
                abort_flag: Arc::clone(&abort_flag),
                join_handle: Arc::clone(&join_slot),
            },
        );
        if let Ok(mut ctx_map) = state.agent_contexts.lock() {
            ctx_map.entry(task_id.clone()).or_insert_with(|| {
                std::sync::Arc::new(std::sync::Mutex::new(crate::agent::compactor::AgentContextState::default()))
            });
        };
    }

    // Persist status "running" to database and update state.tasks
    {
        let mut all_tasks = load_tasks_from_disk(&root).await;
        if let Some(t) = all_tasks.iter_mut().find(|t| task_id_matches(&t.id, &task_id)) {
            t.status = Some("running".to_string());
            t.updated_at = Utc::now().timestamp_millis();
            let _ = save_tasks_to_disk(&root, &all_tasks).await;
        }
        if let Ok(mut guard) = app_handle.state::<AppState>().tasks.lock() {
            *guard = all_tasks;
        }
    }

    crate::logger::cvh_log_with_details(
        Some(&root),
        "INFO",
        "RUN_AGENT",
        &format!("run_agent: launching ReAct loop for task '{}'", task.id),
        Some(&format!(
            "persona='{}', model='{}', base_url='{}', worktree='{}', steer_prompt={:?}",
            persona_role, model, base_url, worktree_dir.display(), steer_prompt
        )),
    );

    // Emit live status: running
    let _ = app_handle.emit(
        "agent:status",
        serde_json::json!({ "taskId": task.id, "status": "running" }),
    );
    let _ = app_handle.emit("agent:session_status", &session_summary);

    let mut initial_description = task.description.clone();
    if task
        .worktree_path
        .as_deref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
    {
        initial_description.push_str(&format!(
            "\n\n[HOOK] worktree: {} — твой изолированный workdir, работаешь тут.",
            worktree_dir.to_string_lossy()
        ));
    }

    let agent_cfg = AgentSessionConfig {
        repo_root: root.clone(),
        worktree_path: worktree_dir,
        role: persona_role,
        stage_id: task.stage_id.clone(),
        task_id: task.id.clone(),
        task_title: task.title.clone(),
        task_description: initial_description,
        handover_summary: task.handover_summary.clone(),
        task_attachments: task.attachments.clone(),
        steer_prompt,
        steer_images: images.clone(),
        base_url,
        api_key,
        model,
        temperature,
        reasoning_effort: col.reasoning_effort.clone(),
        max_turns: 25,
        app_handle: Some(app_handle.clone()),
        context_window: Some(resolved_context_window),
    };

    let tid = task.id.clone();
    let app = app_handle.clone();
    let root_task = root.clone();
    let abort_flag_task = Arc::clone(&abort_flag);
    let session_id_task = session_id.clone();

    let handle = tokio::spawn(async move {
        let app_inner = app.clone();
        let tid_inner = tid.clone();
        let root_inner = root_task.clone();
        let model_inner = agent_cfg.model.clone();

        let event_sink = move |event: AgentEvent| {
            match event {
                AgentEvent::Token(token) => {
                    let payload = serde_json::json!({
                        "taskId": tid_inner,
                        "output": &token,
                        "content": &token,
                        "token": &token
                    });
                    let _ = app_inner.emit("agent:output", &payload);
                }
                AgentEvent::Output(output) => {
                    let payload = serde_json::json!({
                        "taskId": tid_inner,
                        "output": &output,
                        "content": &output,
                        "token": &output
                    });
                    let _ = app_inner.emit("agent:output", &payload);
                }
                AgentEvent::Thought(thought) => {
                    let _ = app_inner.emit(
                        "agent:thought",
                        serde_json::json!({ "taskId": tid_inner, "thought": &thought, "delta": &thought }),
                    );
                }
                AgentEvent::ToolCall { id, name, arguments } => {
                    let _ = app_inner.emit(
                        "agent:tool_call",
                        serde_json::json!({ "taskId": tid_inner, "id": id, "name": name, "arguments": arguments }),
                    );

                    let parsed_args = serde_json::from_str::<serde_json::Value>(&arguments)
                        .unwrap_or_else(|_| serde_json::json!({ "raw": &arguments }));
                    let tool_obj = serde_json::json!({
                        "id": &id,
                        "toolName": &name,
                        "name": &name,
                        "arguments": &arguments,
                        "params": parsed_args,
                        "status": "running",
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "durationMs": 0,
                    });
                    let tool_item = crate::db::ActivityItem {
                        id: id.clone(),
                        item_type: "tool".to_string(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        content: None,
                        tool: Some(tool_obj),
                        duration_ms: None,
                        images: None,
                    };
                    let _ = crate::db::append_dialog_items(&root_inner, &tid_inner, std::slice::from_ref(&tool_item));
                }
                AgentEvent::ToolResult { id, name, result, success } => {
                    let sanitized_result = if result.len() > 65536 {
                        let half = 32768;
                        let head_end = result
                            .char_indices()
                            .map(|(i, _)| i)
                            .take_while(|&i| i <= half)
                            .last()
                            .unwrap_or(0);
                        let tail_target = result.len() - half;
                        let tail_start = result
                            .char_indices()
                            .map(|(i, _)| i)
                            .find(|&i| i >= tail_target)
                            .unwrap_or(result.len());
                        format!(
                            "{}\n\n... [IPC payload truncated ({} bytes total). Full output in SQLite] ...\n\n{}",
                            &result[..head_end],
                            result.len(),
                            &result[tail_start..]
                        )
                    } else {
                        result.clone()
                    };
                    let _ = app_inner.emit(
                        "agent:tool_result",
                        serde_json::json!({ "taskId": tid_inner, "id": id, "name": name, "result": &sanitized_result, "success": success }),
                    );

                    let status_str = if success { "success" } else { "failed" };
                    let mut tool_obj = crate::db::db_get_dialog_tool_json(&root_inner, &tid_inner, &id)
                        .unwrap_or_else(|| serde_json::json!({
                            "id": &id,
                            "toolName": &name,
                            "name": &name,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        }));

                    if let Some(obj) = tool_obj.as_object_mut() {
                        obj.insert("status".to_string(), serde_json::json!(status_str));
                        // Invariant § 2.20 & Defect 7: Persist FULL output in SQLite, keeping sanitized only for IPC
                        obj.insert("stdout".to_string(), serde_json::json!(&result));
                        obj.insert("result".to_string(), serde_json::json!(&result));
                        if !success {
                            obj.insert("stderr".to_string(), serde_json::json!(&result));
                        }
                    }

                    let tool_json_str = serde_json::to_string(&tool_obj).unwrap_or_default();
                    // Invariant § 2.4 & Defect 6: Targeted update without deserializing/rewriting entire dialog history
                    let _ = crate::db::db_update_dialog_tool_result(&root_inner, &tid_inner, &id, &tool_json_str, None);
                }
                AgentEvent::TurnStart { turn } => {
                    let _ = app_inner.emit(
                        "agent:turn_start",
                        serde_json::json!({ "taskId": tid_inner, "turn": turn }),
                    );
                }
                AgentEvent::Telemetry {
                    input_tokens,
                    output_tokens,
                    cache_read_tokens,
                    cache_creation_tokens,
                    cache_hit_rate,
                    duration_ms,
                    tps,
                    ttft_ms,
                    system_tokens,
                    tool_tokens,
                } => {
                    let _ = app_inner.emit(
                        "agent:telemetry",
                        serde_json::json!({
                            "taskId": tid_inner,
                            "inputTokens": input_tokens,
                            "outputTokens": output_tokens,
                            "cacheReadTokens": cache_read_tokens,
                            "cacheCreationTokens": cache_creation_tokens,
                            "cacheHitRate": cache_hit_rate,
                            "durationMs": duration_ms,
                            "tps": tps,
                            "ttftMs": ttft_ms,
                            "systemTokens": system_tokens,
                            "toolTokens": tool_tokens,
                        }),
                    );

                    let telem_record = crate::db::TelemetryRecord {
                        id: uuid::Uuid::new_v4().to_string(),
                        task_id: tid_inner.clone(),
                        model: Some(model_inner.clone()),
                        input_tokens: input_tokens as i64,
                        output_tokens: output_tokens as i64,
                        cached_tokens: cache_read_tokens as i64,
                        tps: tps as f64,
                        ttft_ms,
                        duration_ms,
                        created_at: chrono::Utc::now().timestamp_millis(),
                    };
                    let _ = crate::db::db_record_telemetry(&root_inner, &telem_record);
                }
                AgentEvent::Compacted {
                    saved_chars,
                    estimated_saved_tokens,
                    evaporated_count,
                    tokens_before,
                    tokens_after,
                } => {
                    let _ = app_inner.emit(
                        "agent:compacted",
                        serde_json::json!({
                            "taskId": tid_inner,
                            "savedChars": saved_chars,
                            "estimatedSavedTokens": estimated_saved_tokens,
                            "saved_tokens": estimated_saved_tokens,
                            "evaporatedCount": evaporated_count,
                            "tokens_before": tokens_before,
                            "tokens_after": tokens_after,
                            "tokensBefore": tokens_before,
                            "tokensAfter": tokens_after,
                        }),
                    );
                }
                AgentEvent::Finished { final_response } => {
                    let _ = app_inner.emit(
                        "agent:finished",
                        serde_json::json!({
                            "taskId": tid_inner,
                            "status": "finished",
                            "response": &final_response
                        }),
                    );
                    if !final_response.trim().is_empty() {
                        let resp_item = crate::db::ActivityItem {
                            id: uuid::Uuid::new_v4().to_string(),
                            item_type: "output".to_string(),
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            content: Some(final_response.clone()),
                            tool: None,
                            duration_ms: None,
                            images: None,
                        };
                        let _ = crate::db::append_dialog_items(&root_inner, &tid_inner, std::slice::from_ref(&resp_item));
                    }
                }
                AgentEvent::TaskUpdated { task_id, todos } => {
                    let _ = app_inner.emit(
                        "task:updated",
                        serde_json::json!({
                            "taskId": &task_id,
                            "todos": &todos,
                        }),
                    );
                    let _ = app_inner.emit(
                        "tasks:updated",
                        serde_json::json!({ "taskId": &task_id }),
                    );
                }
                AgentEvent::Aborted => {
                    let _ = app_inner.emit(
                        "agent:aborted",
                        serde_json::json!({ "taskId": tid_inner }),
                    );
                    let _ = app_inner.emit(
                        "agent:status",
                        serde_json::json!({ "taskId": tid_inner, "status": "aborted" }),
                    );
                }
                AgentEvent::Error(err) => {
                    let _ = app_inner.emit(
                        "agent:error",
                        serde_json::json!({ "taskId": tid_inner, "error": &err }),
                    );
                    let _ = app_inner.emit(
                        "agent:status",
                        serde_json::json!({ "taskId": tid_inner, "status": "error", "error": &err }),
                    );
                    let _ = app_inner.emit(
                        "agent:output",
                        serde_json::json!({
                            "taskId": tid_inner,
                            "output": format!("\n[LLM Error: {}]\n", &err),
                            "content": format!("\n[LLM Error: {}]\n", &err),
                        }),
                    );
                }
            }
        };

        let result = run_agent_loop(agent_cfg, abort_flag, event_sink).await;

        match &result {
            Ok(final_response) => {
                crate::logger::cvh_log_with_details(
                    Some(&root_task),
                    "INFO",
                    "RUN_AGENT",
                    &format!("run_agent: agent loop finished successfully for task '{}'", tid),
                    Some(&format!("Final response length: {} chars", final_response.len())),
                );
                let _ = app.emit(
                    "agent:status",
                    serde_json::json!({ "taskId": tid, "status": "finished" }),
                );

                let mut tasks = load_tasks_from_disk(&root_task).await;
                if let Some(task_idx) = tasks.iter().position(|t| t.id == tid) {
                    let mut task_changed = false;

                    if tasks[task_idx].status.as_deref() == Some("running") {
                        tasks[task_idx].status = Some("idle".to_string());
                        tasks[task_idx].updated_at = Utc::now().timestamp_millis();
                        task_changed = true;
                    }

                    if task_changed {
                        let _ = save_tasks_to_disk(&root_task, &tasks).await;
                        if let Some(state_ref) = app.try_state::<AppState>() {
                            if let Ok(mut guard) = state_ref.tasks.lock() {
                                *guard = tasks.clone();
                            }
                        }
                        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
                    }
                }
            }
            Err(err) => {
                let is_aborted = abort_flag_task.load(Ordering::SeqCst);
                let err_str = err.to_string();
                if is_aborted {
                    crate::logger::cvh_log(
                        Some(&root_task),
                        "INFO",
                        "RUN_AGENT",
                        &format!("run_agent: agent loop ABORTED by user for task '{}'", tid),
                    );
                    eprintln!("CVH [run_agent]: Agent execution aborted by user for task {}", tid);
                    let _ = app.emit(
                        "agent:status",
                        serde_json::json!({ "taskId": tid, "status": "idle" }),
                    );
                    let mut tasks = load_tasks_from_disk(&root_task).await;
                    if let Some(task_idx) = tasks.iter().position(|t| t.id == tid) {
                        if tasks[task_idx].status.as_deref() == Some("running") {
                            tasks[task_idx].status = Some("idle".to_string());
                            tasks[task_idx].updated_at = Utc::now().timestamp_millis();
                            let _ = save_tasks_to_disk(&root_task, &tasks).await;
                            if let Some(state_ref) = app.try_state::<AppState>() {
                                if let Ok(mut guard) = state_ref.tasks.lock() {
                                    *guard = tasks.clone();
                                }
                            }
                            let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
                        }
                    }
                } else {
                    eprintln!("CVH [run_agent]: Agent execution failed for task {}: {}", tid, err_str);
                    let _ = app.emit(
                        "agent:status",
                        serde_json::json!({ "taskId": tid, "status": "error", "error": &err_str }),
                    );
                    let _ = app.emit(
                        "agent:error",
                        serde_json::json!({ "taskId": tid, "error": &err_str }),
                    );
                    let _ = app.emit(
                        "agent:output",
                        serde_json::json!({
                            "taskId": tid,
                            "output": format!("\n[LLM Error: {}]\n", &err_str),
                            "content": format!("\n[LLM Error: {}]\n", &err_str),
                        }),
                    );
                    let mut tasks = load_tasks_from_disk(&root_task).await;
                    if let Some(task_idx) = tasks.iter().position(|t| t.id == tid) {
                        tasks[task_idx].status = Some("error".to_string());
                        tasks[task_idx].updated_at = Utc::now().timestamp_millis();
                        let _ = save_tasks_to_disk(&root_task, &tasks).await;
                        if let Some(state_ref) = app.try_state::<AppState>() {
                            if let Ok(mut guard) = state_ref.tasks.lock() {
                                *guard = tasks.clone();
                            }
                        }
                    }
                }
            }
        }

        // Clean up from active_agents and session_registry upon agent completion only if this entry still belongs to this run
        if let Some(state_ref) = app.try_state::<AppState>() {
            state_ref.session_registry.update_status(
                &session_id_task,
                crate::agent::session_registry::SessionStatus::Finished,
            );
            if let Some(summary) = state_ref.session_registry.get_session(&session_id_task) {
                let _ = app.emit("agent:session_status", &summary);
            }
            if let Ok(mut map) = state_ref.active_agents.lock() {
                if let Some(entry) = map.get(&tid) {
                    if Arc::ptr_eq(&entry.abort_flag, &abort_flag_task) {
                        map.remove(&tid);
                        crate::logger::cvh_log(
                            Some(&root_task),
                            "INFO",
                            "RUN_AGENT",
                            &format!("run_agent: active agent handle removed for task '{}'", tid),
                        );
                    }
                }
            }
        }
    });

    {
        let mut guard = join_slot.lock().await;
        *guard = Some(handle);
    }

    Ok(format!("Agent launched for task {}", task_id))
}

/// Suffix/prefix matching helper for subagent keys across registries and dialog identifiers.
pub fn is_matching_subagent_key(key: &str, target_sub_id: &str) -> bool {
    let k_clean = key.trim();
    let t_clean = target_sub_id.trim();
    if k_clean == t_clean {
        return true;
    }
    let k_core = k_clean.rsplit(":sub:").next().unwrap_or(k_clean);
    let t_core = t_clean.rsplit(":sub:").next().unwrap_or(t_clean);
    if k_core == t_core {
        return true;
    }
    // Suffix match with strict delimiter boundary
    if k_core.contains(".sub-") {
        let suffix = k_core.rsplit(".sub-").next().unwrap_or(k_core);
        if suffix == t_core || t_core.ends_with(&format!(".sub-{}", suffix)) || t_core.ends_with(&format!(":sub:{}", suffix)) {
            return true;
        }
    }
    if t_core.contains(".sub-") {
        let suffix = t_core.rsplit(".sub-").next().unwrap_or(t_core);
        if suffix == k_core || k_core.ends_with(&format!(".sub-{}", suffix)) || k_core.ends_with(&format!(":sub:{}", suffix)) {
            return true;
        }
    }
    false
}

/// Agent & Child Subagent Abort:
///
/// If task_id is a child subagent:
///   1. Aborts ONLY this specific subagent (sets abort_flag, handle.abort(), status = "aborted").
///   2. Marks mid-flight running tools for this subagent as failed in SQLite.
///   3. Leaves parent task and sibling subagents completely intact and running!
///
/// If task_id is a parent task:
///   1. Aborts parent task handle and cascaded child subagents.
///   2. Marks mid-flight running tools for parent and subagents as failed.
///   3. Updates task status to 'idle' on disk and memory.
pub async fn internal_abort_agent(
    root: &Path,
    task_id: &str,
    state: &AppState,
    app_handle: Option<&AppHandle>,
) -> Result<bool, String> {
    let p_trim = task_id.trim();
    if p_trim.is_empty() {
        return Ok(false);
    }

    // Determine if task_id identifies a child subagent
    let is_subagent = if p_trim.contains(".sub-") || p_trim.contains(":sub:") {
        true
    } else {
        let in_sessions = {
            let sessions = crate::agent::SUBAGENT_SESSIONS.lock().await;
            sessions.contains_key(p_trim) || sessions.values().any(|sess| {
                if let Ok(info) = sess.retry_info.try_lock() {
                    info.subagent_id == p_trim
                } else {
                    false
                }
            })
        };
        if in_sessions {
            true
        } else {
            let in_active = {
                if let Ok(map) = state.active_agents.lock() {
                    map.keys().any(|k| k == p_trim && (k.contains(".sub-") || k.contains(":sub:")))
                } else {
                    false
                }
            };
            in_active
        }
    };

    if is_subagent {
        // =========================================================================
        // 1. GRANULAR SUBAGENT ABORT:
        //    Abort ONLY this specific subagent. DO NOT touch parent task or siblings!
        // =========================================================================
        let matching_handles: Vec<(String, ActiveAgentHandle)> = {
            let mut map = state
                .active_agents
                .lock()
                .map_err(|_| "Failed to lock active agents".to_string())?;

            let mut matched = Vec::new();
            let mut keys_to_remove = Vec::new();

            for k in map.keys() {
                if is_matching_subagent_key(k, p_trim) {
                    keys_to_remove.push(k.clone());
                }
            }

            for k in keys_to_remove {
                if let Some(entry) = map.remove(&k) {
                    matched.push((k, entry));
                }
            }
            matched
        };

        let aborted_sids = state.session_registry.abort_task(p_trim);
        if let Some(app) = app_handle {
            for sid in &aborted_sids {
                if let Some(summary) = state.session_registry.get_session(sid) {
                    let _ = app.emit("agent:session_status", &summary);
                }
            }
        }

        let had_entry = !matching_handles.is_empty() || !aborted_sids.is_empty();

        for (_k, agent_entry) in matching_handles {
            agent_entry.abort_flag.store(true, Ordering::SeqCst);
            let maybe_handle = {
                let mut guard = agent_entry.join_handle.lock().await;
                guard.take()
            };
            if let Some(handle) = maybe_handle {
                handle.abort();
                let _ = tokio::time::timeout(tokio::time::Duration::from_millis(500), handle).await;
            }
        }

        // Abort matching session in SUBAGENT_SESSIONS
        let mut subagent_session_matched = false;
        {
            let sessions = crate::agent::SUBAGENT_SESSIONS.lock().await;
            for (sid, session) in sessions.iter() {
                let is_this = {
                    let mut info = session.retry_info.lock().await;
                    if is_matching_subagent_key(sid, p_trim)
                        || is_matching_subagent_key(&info.subagent_id, p_trim)
                    {
                        info.status = "aborted".to_string();
                        info.last_error = Some("Прервано пользователем".to_string());
                        info.ctx.abort_handle.store(true, Ordering::SeqCst);
                        true
                    } else {
                        false
                    }
                };
                if is_this {
                    subagent_session_matched = true;
                    session.abort_handle.store(true, Ordering::SeqCst);
                    let mut guard = session.join_handle.lock().await;
                    if let Some(h) = guard.take() {
                        h.abort();
                    }
                    if let Some(sender) = session.sender.lock().await.take() {
                        let _ = sender.send("[Субагент прерван пользователем / Subagent aborted by user]".to_string());
                    }
                }
            }
        }

        // Clean up matching receiver from SUBAGENT_RECEIVERS
        {
            let mut receivers = crate::agent::SUBAGENT_RECEIVERS.lock().await;
            let mut keys_to_remove = Vec::new();
            for k in receivers.keys() {
                if is_matching_subagent_key(k, p_trim) {
                    keys_to_remove.push(k.clone());
                }
            }
            for k in keys_to_remove {
                receivers.remove(&k);
            }
        }

        // Mark mid-flight running tools for this subagent as failed in SQLite
        if let Ok(aborted_tools) = crate::db::mark_running_tools_as_aborted(root, p_trim, true) {
            if let Some(app) = app_handle {
                for (_item_id, item_task_id, tool_id, tool_name) in aborted_tools {
                    let _ = app.emit(
                        "agent:tool_result",
                        serde_json::json!({
                            "taskId": item_task_id,
                            "id": tool_id,
                            "name": tool_name,
                            "result": "Прервано пользователем",
                            "success": false,
                        }),
                    );
                }
            }
        }

        for sid in &aborted_sids {
            state.session_registry.update_status(sid, crate::agent::session_registry::SessionStatus::Finished);
            if let Some(app) = app_handle {
                if let Some(summary) = state.session_registry.get_session(sid) {
                    let _ = app.emit("agent:session_status", &summary);
                }
            }
        }

        if let Some(app) = app_handle {
            let _ = app.emit(
                "subagent:status",
                serde_json::json!({ "subagentId": p_trim, "status": "aborted" }),
            );
            let _ = app.emit(
                "subagent:aborted",
                serde_json::json!({ "subagentId": p_trim }),
            );
        }

        return Ok(had_entry || subagent_session_matched);
    }

    // =========================================================================
    // 2. PARENT TASK ABORT:
    //    Abort parent task AND cascade abort to all its child subagents!
    // =========================================================================
    // 1. Collect all matching handles from state.active_agents (parent + any subagents)
    let matching_handles: Vec<(String, ActiveAgentHandle)> = {
        let mut map = state
            .active_agents
            .lock()
            .map_err(|_| "Failed to lock active agents".to_string())?;

        let mut matched = Vec::new();
        let mut keys_to_remove = Vec::new();

        for k in map.keys() {
            let k_trim = k.trim();
            if task_id_matches(k_trim, p_trim)
                || k_trim.to_ascii_lowercase().starts_with(&format!("{}.", p_trim.to_ascii_lowercase()))
                || k_trim.to_ascii_lowercase().starts_with(&format!("{}:sub:", p_trim.to_ascii_lowercase()))
                || k_trim.to_ascii_lowercase().starts_with(&format!("{}.sub-", p_trim.to_ascii_lowercase()))
            {
                keys_to_remove.push(k.clone());
            }
        }

        for k in keys_to_remove {
            if let Some(entry) = map.remove(&k) {
                matched.push((k, entry));
            }
        }
        matched
    };

    let aborted_session_ids = state.session_registry.abort_task(p_trim);
    if let Some(app) = app_handle {
        for sid in &aborted_session_ids {
            if let Some(summary) = state.session_registry.get_session(sid) {
                let _ = app.emit("agent:session_status", &summary);
            }
        }
    }

    let had_entry = !matching_handles.is_empty() || !aborted_session_ids.is_empty();

    // 2. Abort all active agent handles and wait with timeout
    for (_k, agent_entry) in matching_handles {
        agent_entry.abort_flag.store(true, Ordering::SeqCst);
        let maybe_handle = {
            let mut guard = agent_entry.join_handle.lock().await;
            guard.take()
        };
        if let Some(handle) = maybe_handle {
            handle.abort();
            let _ = tokio::time::timeout(tokio::time::Duration::from_millis(500), handle).await;
        }
    }

    // 3. Abort all child subagents in SUBAGENT_SESSIONS
    {
        let sessions = crate::agent::SUBAGENT_SESSIONS.lock().await;
        for (sid, session) in sessions.iter() {
            let matches_parent = {
                let mut info = session.retry_info.lock().await;
                if info.parent_task_id == p_trim
                    || info.subagent_id.starts_with(&format!("{}.", p_trim))
                    || info.subagent_id.starts_with(&format!("{}:sub:", p_trim))
                    || sid.starts_with(&format!("{}.", p_trim))
                    || sid.starts_with(&format!("{}:sub:", p_trim))
                {
                    info.status = "aborted".to_string();
                    info.last_error = Some("Прервано пользователем".to_string());
                    info.ctx.abort_handle.store(true, Ordering::SeqCst);
                    true
                } else {
                    false
                }
            };
            if matches_parent {
                session.abort_handle.store(true, Ordering::SeqCst);
                let mut guard = session.join_handle.lock().await;
                if let Some(h) = guard.take() {
                    h.abort();
                }
                // Take sender so awaiting receiver gets Err immediately
                let _ = session.sender.lock().await.take();
            }
        }
    }

    // 4. Clean up any awaiting receivers in SUBAGENT_RECEIVERS
    {
        let mut receivers = crate::agent::SUBAGENT_RECEIVERS.lock().await;
        let mut keys_to_remove = Vec::new();
        for k in receivers.keys() {
            let k_trim = k.trim();
            if task_id_matches(k_trim, p_trim)
                || k_trim.to_ascii_lowercase().starts_with(&format!("{}.", p_trim.to_ascii_lowercase()))
                || k_trim.to_ascii_lowercase().starts_with(&format!("{}:sub:", p_trim.to_ascii_lowercase()))
                || k_trim.to_ascii_lowercase().starts_with(&format!("{}.sub-", p_trim.to_ascii_lowercase()))
            {
                keys_to_remove.push(k.clone());
            }
        }
        for k in keys_to_remove {
            receivers.remove(&k);
        }
    }

    // 5. Mark all running mid-flight tools in SQLite for parent and subagents as failed
    if let Ok(aborted_tools) = crate::db::mark_running_tools_as_aborted(root, p_trim, false) {
        if let Some(app) = app_handle {
            for (_item_id, item_task_id, tool_id, tool_name) in aborted_tools {
                let _ = app.emit(
                    "agent:tool_result",
                    serde_json::json!({
                        "taskId": item_task_id,
                        "id": tool_id,
                        "name": tool_name,
                        "result": "Прервано пользователем",
                        "success": false,
                    }),
                );
            }
        }
    }

    // 6. Update task status on disk & memory to "idle"
    let mut tasks = load_tasks_from_disk(root).await;
    // MERGE IN-MEMORY TASKS:
    // Merge in-memory tasks from state.tasks before updating status to avoid wiping unsaved disk tasks (§ 2.22)
    if let Ok(guard) = state.tasks.lock() {
        for mem_t in guard.iter() {
            if let Some(disk_t) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &mem_t.id)) {
                if mem_t.updated_at > disk_t.updated_at {
                    *disk_t = mem_t.clone();
                }
            } else {
                tasks.push(mem_t.clone());
            }
        }
    }
    let mut task_updated = false;
    if let Some(t) = tasks.iter_mut().find(|t| task_id_matches(&t.id, p_trim)) {
        if t.status.as_deref() != Some("idle") {
            t.status = Some("idle".to_string());
            t.updated_at = Utc::now().timestamp_millis();
            task_updated = true;
        }
    }

    if task_updated {
        if let Ok(mut guard) = state.tasks.lock() {
            *guard = tasks.clone();
        }
        let _ = save_tasks_to_disk(root, &tasks).await;
        if let Some(app) = app_handle {
            let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
        }
    }

    for sid in &aborted_session_ids {
        state.session_registry.update_status(sid, crate::agent::session_registry::SessionStatus::Finished);
        if let Some(app) = app_handle {
            if let Some(summary) = state.session_registry.get_session(sid) {
                let _ = app.emit("agent:session_status", &summary);
            }
        }
    }

    if let Some(app) = app_handle {
        let _ = app.emit(
            "agent:status",
            serde_json::json!({ "taskId": p_trim, "status": "idle" }),
        );
        let _ = app.emit(
            "subagent:aborted",
            serde_json::json!({ "parentTaskId": p_trim }),
        );
    }

    Ok(had_entry || task_updated)
}

pub async fn abort_agent(
    task_id: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let root = state.get_root();
    let p_trim = task_id.trim();
    let is_sub = p_trim.contains(".sub-")
        || p_trim.contains(":sub:")
        || {
            let sessions = crate::agent::SUBAGENT_SESSIONS.lock().await;
            sessions.contains_key(p_trim) || sessions.values().any(|sess| {
                if let Ok(info) = sess.retry_info.try_lock() {
                    info.subagent_id == p_trim
                } else {
                    false
                }
            })
        };
    let _ = internal_abort_agent(&root, &task_id, &state, Some(&app_handle)).await?;
    if !is_sub {
        let _ = app_handle.emit(
            "agent:status",
            serde_json::json!({ "taskId": &task_id, "status": "idle" }),
        );
    }
    Ok(())
}

pub async fn get_active_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<crate::agent::session_registry::AgentSessionSummary>, String> {
    Ok(state.session_registry.get_active_sessions())
}

pub async fn compact_task_context(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<crate::agent::compactor::CompactionResult, String> {
    let root = state.get_root();
    let tasks = load_tasks_from_disk(&root).await;
    let task = tasks.iter().find(|t| t.id == task_id);

    let snapshot = crate::agent::compactor::ExecutionStateSnapshot {
        modified_files: vec!["src/agent.rs".to_string(), "src/commands.rs".to_string()],
        sensor_status: "PASSED (Sensors verified)".to_string(),
        next_step: format!("Continuing autonomous execution for {}", task_id),
        verdict: task.and_then(|t| t.verdict.clone()).unwrap_or_else(|| "IN_PROGRESS".to_string()),
    };

    let mut sample_messages = vec![
        crate::llm::ChatMessage::system("Base system prompt instructions"),
        crate::llm::ChatMessage::user(format!("Task execution prompt for {}", task_id)),
        crate::llm::ChatMessage::assistant(Some("Investigating files".to_string()), None),
        crate::llm::ChatMessage::tool_response("call_01", "A".repeat(800)),
        crate::llm::ChatMessage::assistant(Some("Applied modifications".to_string()), None),
    ];

    let result = crate::agent::compactor::compact_conversation(&mut sample_messages, Some(snapshot));
    Ok(result)
}

pub async fn compact_context(
    task_id: String,
    tool_id: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::agent::compactor::CompactionReport, String> {
    let root = state.get_root();

    let tool_item_id = tool_id.unwrap_or_else(|| {
        format!(
            "compact_context-{}-{}",
            task_id,
            chrono::Utc::now().timestamp_millis()
        )
    });

    // 0. Emit agent:tool_call immediately so chat feed displays running compact_context card
    let _ = app_handle.emit(
        "agent:tool_call",
        serde_json::json!({
            "taskId": &task_id,
            "id": &tool_item_id,
            "name": "compact_context",
            "arguments": serde_json::json!({
                "taskId": &task_id,
                "title": "Сжатие контекста...",
            }).to_string(),
        }),
    );

    // 1. Load LLM configuration and resolve active agent model
    let llm_settings = crate::config::load_llm_settings(&root).await.unwrap_or_default();
    let tasks = load_tasks_from_disk(&root).await;
    let task = tasks.iter().find(|t| t.id == task_id).cloned();

    let active_agent_model = resolve_task_active_agent_model(&root, task.as_ref(), &llm_settings).await;

    // 2. Resolve compactor model (inheritance support)
    let compactor_model = crate::agent::compactor::resolve_compactor_model(
        &llm_settings.compactor_model,
        &active_agent_model,
    );

    let base_url = if !llm_settings.base_url.trim().is_empty() {
        llm_settings.base_url.trim().to_string()
    } else {
        "http://127.0.0.1:8045/v1".to_string()
    };

    let api_key = if !llm_settings.api_key.trim().is_empty() {
        Some(llm_settings.api_key.trim())
    } else {
        None
    };

    // 3. Load dialogue items from SQLite
    let dialog_items = crate::db::load_dialog_items(&root, &task_id).unwrap_or_default();

    // 4. Determine items to condense before the last 4 active messages (2 full recent turns)
    let mut ids_to_archive: Vec<String> = Vec::new();
    let mut messages_to_condense: Vec<crate::llm::ChatMessage> = Vec::new();

    if dialog_items.len() > 4 {
        let split_idx = dialog_items.len().saturating_sub(4);
        let items_slice = &dialog_items[..split_idx];

        for it in items_slice {
            ids_to_archive.push(it.id.clone());
            match it.item_type.as_str() {
                "user" | "steer" => {
                    messages_to_condense.push(crate::llm::ChatMessage::user(
                        it.content.clone().unwrap_or_default(),
                    ));
                }
                "thought" => {
                    let mut msg = crate::llm::ChatMessage::assistant(
                        it.content.clone(),
                        None,
                    );
                    msg.name = Some("thought".to_string());
                    messages_to_condense.push(msg);
                }
                "tool" => {
                    if let Some(ref t) = it.tool {
                        let tool_id = t.get("id").and_then(|v| v.as_str()).unwrap_or("tool_call");
                        let stdout = t.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
                        let tool_name = t.get("toolName").and_then(|v| v.as_str()).unwrap_or("tool");
                        let params = t.get("params").cloned().unwrap_or(serde_json::Value::Null);

                        // Reconstruct Assistant ToolCall message so compactor extracts tool name and parameters
                        messages_to_condense.push(crate::llm::ChatMessage::assistant(
                            None,
                            Some(vec![crate::llm::ToolCall {
                                id: tool_id.to_string(),
                                call_type: "function".to_string(),
                                function: crate::llm::FunctionCall {
                                    name: tool_name.to_string(),
                                    arguments: params.to_string(),
                                },
                            }]),
                        ));
                        messages_to_condense.push(crate::llm::ChatMessage::tool_response(
                            tool_id,
                            stdout.to_string(),
                        ));
                    }
                }
                _ => {
                    if let Some(ref c) = it.content {
                        messages_to_condense.push(crate::llm::ChatMessage::assistant(
                            Some(c.clone()),
                            None,
                        ));
                    }
                }
            }
        }
    } else {
        // Fallback to in-memory active agent context if available, or task description
        let mem_msgs = {
            if let Ok(map) = state.agent_contexts.lock() {
                map.get(&task_id).and_then(|cs| cs.lock().ok().map(|g| g.messages.clone()))
            } else {
                None
            }
        };

        if let Some(msgs) = mem_msgs {
            if msgs.len() > 4 {
                let split_idx = msgs.len().saturating_sub(4);
                messages_to_condense.extend(msgs[..split_idx].iter().cloned());
            }
        }

        if messages_to_condense.is_empty() {
            if let Some(ref t) = task {
                messages_to_condense.push(crate::llm::ChatMessage::user(format!(
                    "Task {}: {}\n{}",
                    t.id, t.title, t.description
                )));
            } else {
                messages_to_condense.push(crate::llm::ChatMessage::user(format!(
                    "Task {}",
                    task_id
                )));
            }
        }
    }

    let tokens_before = crate::agent::compactor::estimate_tokens(&messages_to_condense);

    // 5. Run LLM condensation with the 7-section structured prompt
    let compacted_markdown = crate::agent::compactor::condense_context_with_llm(
        &task_id,
        &messages_to_condense,
        &compactor_model,
        &base_url,
        api_key,
    )
    .await;

    let tokens_after = crate::agent::compactor::estimate_text_tokens(&compacted_markdown);
    let saved_tokens = tokens_before.saturating_sub(tokens_after);
    let compacted_count = if !ids_to_archive.is_empty() {
        ids_to_archive.len()
    } else {
        messages_to_condense.len().max(1)
    };

    // 6. Build tool execution item and compacted summary item for SQLite and in-chat display
    let stdout_msg = format!(
        "Контекст успешно сжат. Сохранено {} токенов. Контекст сокращен с {} до {} токенов.",
        saved_tokens, tokens_before, tokens_after
    );
    let tool_act_item = crate::db::ActivityItem {
        id: tool_item_id.clone(),
        item_type: "tool".to_string(),
        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        content: None,
        tool: Some(serde_json::json!({
            "id": &tool_item_id,
            "toolName": "compact_context",
            "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
            "durationMs": 0,
            "status": "success",
            "params": {
                "taskId": &task_id,
                "title": format!("Сжато (-{} токенов)", saved_tokens),
                "savedTokens": saved_tokens,
            },
            "stdout": &stdout_msg,
        })),
        duration_ms: Some(0.0),
        images: None,
    };

    let compacted_item = crate::db::ActivityItem {
        id: format!("compacted-{}-{}", task_id, chrono::Utc::now().timestamp_millis()),
        item_type: "compacted".to_string(),
        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        content: Some(compacted_markdown.clone()),
        tool: Some(serde_json::json!({
            "compactedCount": compacted_count,
            "savedTokens": saved_tokens,
            "tokensBefore": tokens_before,
            "tokensAfter": tokens_after,
            "compactorModel": compactor_model,
        })),
        duration_ms: None,
        images: None,
    };

    crate::db::persist_compacted_dialog(&root, &task_id, &compacted_item, Some(&tool_act_item), &ids_to_archive)
        .map_err(|e| format!("Failed to persist compacted dialog: {}", e))?;

    // 7. Store task.current_tokens in memory and persist to disk (Fixes 100% gauge on re-entering drawer)
    {
        let mut disk_tasks = load_tasks_from_disk(&root).await;
        if let Some(t) = disk_tasks.iter_mut().find(|t| t.id == task_id) {
            t.current_tokens = Some(tokens_after);
        }
        let _ = save_tasks_to_disk(&root, &disk_tasks).await;

        if let Ok(mut guard) = state.tasks.lock() {
            if let Some(t) = guard.iter_mut().find(|t| t.id == task_id) {
                t.current_tokens = Some(tokens_after);
            }
        }
    }

    // 8. Update in-memory active agent context if present (preserving Turn 0 anchors and last 4 messages)
    let context_state_opt = {
        if let Ok(map) = state.agent_contexts.lock() {
            map.get(&task_id).cloned()
        } else {
            None
        }
    };

    if let Some(cs) = context_state_opt {
        if let Ok(mut guard) = cs.lock() {
            let mut new_messages = Vec::new();
            if !guard.messages.is_empty() {
                new_messages.push(guard.messages[0].clone()); // System prompt
            }
            if guard.messages.len() > 1 && guard.messages[1].role == crate::llm::Role::User {
                new_messages.push(guard.messages[1].clone()); // Initial user spec
            }
            new_messages.push(crate::llm::ChatMessage::system(format!(
                "# Compacted Context Summary\n\n{}",
                compacted_markdown
            )));
            if guard.messages.len() > 4 {
                let tail_start = guard.messages.len().saturating_sub(4);
                let actual_tail = tail_start.max(2);
                for msg in &guard.messages[actual_tail..] {
                    new_messages.push(msg.clone());
                }
            }
            // Enforce safe ceiling guarantee on in-memory context as well
            let _ = crate::agent::compactor::compact_conversation_to_target(&mut new_messages, 96_000, None);
            guard.messages = new_messages;
        }
    }

    let report = crate::agent::compactor::CompactionReport {
        tokens_before,
        tokens_after,
        evaporated_count: compacted_count,
        saved_tokens,
        compacted_messages_count: compacted_count,
        summary: Some(compacted_markdown),
    };

    // 9. Emit Tauri events for compact_context tool execution card, compaction report, and telemetry
    let _ = app_handle.emit(
        "agent:tool_result",
        serde_json::json!({
            "taskId": &task_id,
            "id": &tool_item_id,
            "name": "compact_context",
            "result": &stdout_msg,
            "success": true,
        }),
    );
    let _ = app_handle.emit("agent:compacted", &report);
    let _ = app_handle.emit(
        "agent:telemetry",
        serde_json::json!({
            "taskId": task_id,
            "inputTokens": tokens_after,
            "outputTokens": 0,
            "cacheReadTokens": 0,
            "cacheCreationTokens": 0,
            "cacheHitRate": 90.0,
        }),
    );
    let _ = app_handle.emit("dialog:updated", serde_json::json!({ "taskId": task_id }));

    Ok(report)
}

pub async fn steer_agent(
    task_id: String,
    message: String,
    images: Option<Vec<String>>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let root = state.get_root();

    let task_found = {
        if let Ok(guard) = state.tasks.lock() {
            guard.iter().any(|t| task_id_matches(&t.id, &task_id))
        } else {
            false
        }
    };

    if !task_found {
        let tasks = load_tasks_from_disk(&root).await;
        if !tasks.iter().any(|t| task_id_matches(&t.id, &task_id)) {
            return Err(format!("Task {} not found", task_id));
        }
    }

    // Persist steer message into SQLite dialog_items
    // only if not already persisted (e.g. from optimistic client save)
    let mut already_in_db = false;
    if let Ok(mut items) = crate::db::load_dialog_items(&root, &task_id) {
        if let Some(it) = items.iter_mut().rev().take(3).find(|it| {
            (it.item_type == "steer" || it.item_type == "user")
                && it.content.as_deref().map(str::trim) == Some(message.trim())
        }) {
            already_in_db = true;
            if (it.images.is_none() || it.images.as_ref().map(|v| v.is_empty()).unwrap_or(false))
                && images.as_ref().map(|v| !v.is_empty()).unwrap_or(false)
            {
                it.images = images.clone();
                let _ = crate::db::append_dialog_items(&root, &task_id, std::slice::from_ref(it));
            }
        }
    }

    if !already_in_db {
        let new_item = crate::db::ActivityItem {
            id: uuid::Uuid::new_v4().to_string(),
            item_type: "steer".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            content: Some(message.clone()),
            tool: None,
            duration_ms: None,
            images: images.clone(),
        };
        let _ = crate::db::append_dialog_items(&root, &task_id, std::slice::from_ref(&new_item));
        let _ = app_handle.emit("task:dialog", serde_json::json!({
            "taskId": task_id,
            "item": new_item,
        }));
    }

    let steer_payload = serde_json::json!({
        "taskId": task_id,
        "content": message,
        "type": "steer",
        "images": images,
    });
    let _ = app_handle.emit("agent:steer", steer_payload);

    {
        let prev_entry = {
            let mut map = state
                .active_agents
                .lock()
                .map_err(|_| "Failed to lock active agents".to_string())?;
            map.remove(&task_id)
        };

        if let Some(agent_entry) = prev_entry {
            agent_entry.abort_flag.store(true, Ordering::SeqCst);
            let maybe_handle = {
                let mut guard = agent_entry.join_handle.lock().await;
                guard.take()
            };
            if let Some(handle) = maybe_handle {
                if tokio::time::timeout(tokio::time::Duration::from_millis(5000), handle)
                    .await
                    .is_err()
                {
                    eprintln!(
                        "CVH [steer_agent]: Warning: previous agent task for {} did not finish within 5s timeout, proceeding with steer",
                        task_id
                    );
                }
            }
        }
    }

    {
        let mut current_tasks = load_tasks_from_disk(&root).await;
        if let Some(t) = current_tasks.iter_mut().find(|t| t.id == task_id) {
            t.updated_at = Utc::now().timestamp_millis();
            let _ = save_tasks_to_disk(&root, &current_tasks).await;
        }
        if let Ok(mut guard) = state.tasks.lock() {
            if let Some(t) = guard.iter_mut().find(|t| t.id == task_id) {
                t.updated_at = Utc::now().timestamp_millis();
            }
        }
    }

    run_agent(task_id, Some(message), images, app_handle).await
}

/// Resolves the context window strictly for a task or agent persona role.
/// Priority:
/// 1. Direct persona override if provided (e.g. activeSubagentRole).
/// 2. Active agent persona associated with the task's column.
/// 3. Setting in `.cvh/agents/<role>/setting` (`context_window` or `contextWindow`).
/// 4. Fallback strictly to 128_000.
pub async fn get_task_context_window(
    task_id: Option<String>,
    persona: Option<String>,
    app_handle: AppHandle,
) -> Result<u64, String> {
    let root = app_handle.state::<AppState>().get_root();

    if let Some(ref p) = persona {
        let p_trimmed = p.trim();
        if !p_trimmed.is_empty() {
            let setting = crate::config::load_agent_setting(&root, p_trimmed).await;
            return Ok(setting.context_window.unwrap_or(128_000));
        }
    }

    if let Some(ref tid) = task_id {
        let tid_trimmed = tid.trim();
        if !tid_trimmed.is_empty() {
            let disk_tasks = load_tasks_from_disk(&root).await;
            let task_opt = disk_tasks.iter().find(|t| t.id == tid_trimmed).cloned().or_else(|| {
                app_handle.state::<AppState>().tasks.lock().ok()?.iter().find(|t| t.id == tid_trimmed).cloned()
            });

            if let Some(task) = task_opt {
                let boards_cfg = load_boards_config(&root).await.ok();
                let col = boards_cfg.as_ref().and_then(|bc| {
                    let b = if let Some(ref bid) = task.board_id {
                        bc.boards.iter().find(|b| b.id == *bid)
                    } else {
                        bc.boards.iter().find(|b| b.id == bc.active_board_id).or_else(|| bc.boards.first())
                    };
                    b.and_then(|board| board.columns.iter().find(|c| c.id == task.stage_id).cloned())
                });

                let persona_role = col
                    .and_then(|c| c.persona)
                    .unwrap_or_else(|| "developer".to_string());

                let setting = crate::config::load_agent_setting(&root, &persona_role).await;
                return Ok(setting.context_window.unwrap_or(128_000));
            }

            if crate::config::agent_dir(&root, tid_trimmed).exists() {
                let setting = crate::config::load_agent_setting(&root, tid_trimmed).await;
                return Ok(setting.context_window.unwrap_or(128_000));
            }
        }
    }

    Ok(128_000)
}

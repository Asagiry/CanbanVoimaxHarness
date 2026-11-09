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
/// 1. Explicit Provider / Model pair from the steer bar (`task.provider_id`
///    + `task.model` — only when the provider option is set, so old rows
///      without a pair never resurrect stale SQLite models)
/// 2. Stage persona agent settings (`agent_setting.model`)
/// 3. Column model override (`col.model_override`)
/// 4. Project default model (`proj_cfg.default_model`)
/// 5. Hardcoded fallback ("crouton-alpha-free")
///
/// NOTE: `task.model` alone (without `provider_id`) and `llm_settings.model`
/// (global) remain intentionally ignored — legacy rows may carry stale SQLite
/// models, and the global setting is not per-task intent.
///
/// The candidate is then validated against ALL provider catalogs: a model
/// listed nowhere is replaced with the first catalog model so the request
/// never hits "model not found" on a wrong endpoint.
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

    let (col_override, agent_model) = if let Some(t) = task {
        // Explicit pair chosen in the steer bar (provider + model) is the
        // freshest user intent: it wins over persona/col/project defaults.
        // Old rows without provider_id fall through — that keeps the
        // stale-SQLite-model problem from the pre-pair era away.
        let explicit_pair = non_empty_opt(&t.provider_id)
            .and_then(|_| non_empty_opt(&t.model));
        if let Some(pair_model) = explicit_pair {
            return pair_model;
        }
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
            b.and_then(|b| b.stages.iter().find(|c| c.id == t.stage_id).cloned())
        });
        let col = col_from_board
            .or_else(|| pipeline.stages.iter().find(|c| c.id == t.stage_id).cloned());
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

    let candidate = agent_model
        .or(col_override)
        .or(proj_default)
        .unwrap_or_else(|| "crouton-alpha-free".to_string());

    // Validate candidate exists in ANY provider catalog (all providers active).
    // Unknown model falls back to first catalog model of first provider with a catalog.
    if !llm_settings.providers.is_empty() {
        let in_any = llm_settings.providers.iter().any(|p| p.models.iter().any(|m| m.id == candidate));
        if !in_any {
            if let Some(first) = llm_settings.providers.iter().flat_map(|p| p.models.iter()).next() {
                eprintln!(
                    "CVH [resolve_model]: model '{}' not in any provider catalog, using '{}'",
                    candidate, first.id
                );
                return first.id.clone();
            }
        }
    }
    candidate
}

pub fn cleanup_active_agent_reservation(app_handle: &AppHandle, root: &std::path::Path, task_id: &str) {
    if let Some(state) = app_handle.try_state::<AppState>() {
        state.remove_active_agent(root, task_id);
    }
}

pub async fn run_agent(
    task_id: String,
    prompt: Option<String>,
    images: Option<Vec<String>>,
    project_path: Option<String>,
    app_handle: AppHandle,
) -> Result<String, String> {
    run_agent_inner(task_id, prompt, images, project_path, app_handle, true).await
}

async fn run_agent_inner(
    task_id: String,
    prompt: Option<String>,
    images: Option<Vec<String>>,
    project_path: Option<String>,
    app_handle: AppHandle,
    persist_steer: bool,
) -> Result<String, String> {
    let root = project_path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| app_handle.state::<AppState>().get_root());
    crate::logger::cvh_log_with_details(
        Some(&root),
        "INFO",
        "RUN_AGENT",
        &format!("run_agent IPC called for task_id='{}', prompt_len={}", task_id, prompt.as_ref().map(|p| p.len()).unwrap_or(0)),
        Some(&format!("root='{}'", root.display())),
    );

    // Atomically check and reserve the active agent slot BEFORE any async operations (TOCTOU guard)
    let (abort_flag, join_slot, steering, advance_aborted) = {
        let state = app_handle.state::<AppState>();
        let mut map = state
            .active_agents
            .lock()
            .map_err(|_| "Failed to lock active agents".to_string())?;

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

        if AppState::is_agent_active_in_map(&map, &root, &task_id) {
            crate::logger::cvh_log(
                Some(&root),
                "WARN",
                "RUN_AGENT",
                &format!("run_agent: Task '{}' already has an active agent loop in root '{:?}', rejecting duplicate run", task_id, root),
            );
            return Ok(format!("Agent already running for task {}", task_id));
        }

        let agent_k = AppState::agent_key(&root, &task_id);
        let abort_flag = Arc::new(AtomicBool::new(false));
        let join_slot = Arc::new(tokio::sync::Mutex::new(None));
        let steering = Arc::new(std::sync::Mutex::new(crate::agent::AgentSteering::default()));
        let advance_aborted = Arc::new(AtomicBool::new(false));
        map.insert(
            agent_k,
            ActiveAgentHandle {
                abort_flag: Arc::clone(&abort_flag),
                join_handle: Arc::clone(&join_slot),
                steering: Some(Arc::clone(&steering)),
                advance_aborted: Arc::clone(&advance_aborted),
            },
        );
        (abort_flag, join_slot, steering, advance_aborted)
    };

    // Check disk and in-memory state: merge them so in-memory tasks are never lost (§ 2.22).
    let mut tasks_to_use = load_tasks_from_disk(&root).await;
    let initial_disk_count = tasks_to_use.len();
    let is_current_project = app_handle.state::<AppState>().get_root() == root;
    if is_current_project {
        if let Ok(guard) = app_handle.state::<AppState>().tasks.lock() {
            for mem_task in guard.iter() {
                if !tasks_to_use.iter().any(|t| task_id_matches(&t.id, &mem_task.id)) {
                    tasks_to_use.push(mem_task.clone());
                }
            }
        }
    }

    let task_opt = tasks_to_use.iter().find(|t| task_id_matches(&t.id, &task_id)).cloned();

    let mut task = match task_opt {
        Some(t) => {
            if is_current_project {
                if let Ok(mut guard) = app_handle.state::<AppState>().tasks.lock() {
                    *guard = tasks_to_use.clone();
                }
            }
            if tasks_to_use.len() > initial_disk_count {
                let _ = save_tasks_to_disk(&root, &tasks_to_use).await;
            }
            t
        }
        None => {
            let mem_count = if is_current_project {
                app_handle.state::<AppState>().tasks.lock().map(|g| g.len()).unwrap_or(0)
            } else {
                0
            };
            let available_ids: Vec<String> = tasks_to_use.iter().map(|t| t.id.clone()).collect();
            let err_msg = format!("Task {} not found", task_id);
            crate::logger::cvh_log_with_details(
                Some(&root),
                "ERROR",
                "RUN_AGENT",
                &format!("run_agent REJECTED: Task '{}' not found (in-memory: {}, disk: {})", task_id, mem_count, tasks_to_use.len()),
                Some(&format!("Available tasks in SQLite: [{}]", available_ids.join(", "))),
            );
            cleanup_active_agent_reservation(&app_handle, &root, &task_id);
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

    // Strict guard against duplicate runs while waiting for subtask or subagent:
    let is_waiting_for_subtask = task
        .waiting_for_subtask_id
        .as_deref()
        .map(|id| !id.trim().is_empty())
        .unwrap_or(false)
        || task.status.as_deref() == Some("waiting_subtask");
    if (is_waiting_for_subtask || task.status.as_deref() == Some("waiting_subagent")) && steer_prompt.is_none() {
        crate::logger::cvh_log(
            Some(&root),
            "WARN",
            "RUN_AGENT",
            &format!(
                "run_agent: Task '{}' has active subtask or subagent (waiting_for_subtask_id={:?}, status={:?}), rejecting autorun",
                task_id, task.waiting_for_subtask_id, task.status
            ),
        );
        cleanup_active_agent_reservation(&app_handle, &root, &task_id);
        return Ok(format!("Task {} is waiting for subtask/subagent to finish", task_id));
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
            b.and_then(|b| b.stages.iter().find(|c| c.id == task.stage_id).cloned())
        });
        col_from_board
            .or_else(|| pipeline.stages.iter().find(|c| c.id == task.stage_id).cloned())
            .unwrap_or_else(|| {
                eprintln!(
                    "CVH [run_agent]: Stage '{}' not found in board or pipeline, falling back to default stage",
                    task.stage_id
                );
                pipeline.stages.first().cloned().unwrap_or_else(|| crate::config::PipelineStage {
                    id: task.stage_id.clone(),
                    title: task.stage_id.clone(),
                    color: None,
                    icon: None,
                    persona: Some("developer".to_string()),
                    wip_limit: None,
                    on_enter_script: None,
                    context_window: None,
                    model_override: None,
                    reasoning_effort: None,
                    temperature: Some(0.2),
                    gatekeeper_sensor: None,
                    allowed_tools: None,
                    hooks: HashMap::new(),
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
            default_model: Some("crouton-alpha-free".to_string()),
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

    // Dispatch endpoint comes from the explicitly selected Provider / Model
    // pair (never searched by model alone): the chosen provider owns URL+key.
    let (provider_base_url, api_key) =
        llm_settings.endpoint_for_pair(task.provider_id.as_deref(), &model);
    let mut base_url = if !provider_base_url.trim().is_empty() {
        provider_base_url
    } else {
        proj_cfg
            .api_base_url
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:8045/v1".to_string())
    };

    if base_url.trim().is_empty() {
        base_url = "http://127.0.0.1:8045/v1".to_string();
    }

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

    if abort_flag.load(Ordering::SeqCst) {
        cleanup_active_agent_reservation(&app_handle, &root, &task_id);
        return Ok(format!("Agent execution cancelled before start for task {}", task_id));
    }

    let session_id = format!("sess-{}-{}", task_id, Utc::now().timestamp_millis());
    let session_started_at = Utc::now().timestamp_millis() as u64;
    let session_summary = crate::agent::session_registry::AgentSessionSummary {
        session_id: session_id.clone(),
        task_id: task_id.clone(),
        stage_id: Some(task.stage_id.clone()),
        started_at: session_started_at,
        status: crate::agent::session_registry::SessionStatus::Running,
        project_path: Some(root.to_string_lossy().to_string()),
    };

    // Register in authoritative session registry & initialize agent context (active_agents already reserved)
    {
        let state = app_handle.state::<AppState>();
        state.session_registry.register(
            session_summary.clone(),
            Arc::clone(&abort_flag),
            Arc::clone(&join_slot),
        );
        let _ = state.get_or_create_agent_context(&root, &task_id);
    }

    // Persist status "running" to database and update state.tasks
    {
        let mut all_tasks = load_tasks_from_disk(&root).await;
        if let Some(t) = all_tasks.iter_mut().find(|t| task_id_matches(&t.id, &task_id)) {
            t.status = Some("running".to_string());
            t.updated_at = Utc::now().timestamp_millis();
            let _ = save_tasks_to_disk(&root, &all_tasks).await;
        }
        if app_handle.state::<AppState>().get_root() == root {
            if let Ok(mut guard) = app_handle.state::<AppState>().tasks.lock() {
                *guard = all_tasks;
            }
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
    let project_path_str = root.to_string_lossy().to_string();
    let _ = app_handle.emit(
        "agent:status",
        serde_json::json!({ "taskId": task.id, "status": "running", "projectPath": &project_path_str }),
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
        steer_prompt: if persist_steer { steer_prompt } else { None },
        steer_images: images.clone(),
        steering,
        advance_aborted: Some(Arc::clone(&advance_aborted)),
        base_url,
        api_key,
        model,
        temperature,
        // Per-agent setting wins over the stage default; both normalized
        // (`"off"`/empty/unknown → `None` → omitted from the payload).
        reasoning_effort: crate::config::normalize_reasoning_effort(
            task.reasoning_effort.as_deref(),
        )
        .or_else(|| {
            crate::config::normalize_reasoning_effort(agent_setting.reasoning_effort.as_deref())
        })
        .or_else(|| {
            crate::config::normalize_reasoning_effort(col.reasoning_effort.as_deref())
        }),
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

        let active_stream_id = std::sync::Mutex::new(None::<String>);
        let event_sink = move |event: AgentEvent| {
            match event {
                AgentEvent::StreamStart { id } => {
                    *active_stream_id.lock().unwrap() = Some(id.clone());
                    let _ = app_inner.emit("agent:stream_start", serde_json::json!({
                        "taskId": tid_inner, "streamId": id,
                    }));
                }
                AgentEvent::StreamRetry { id, delay_secs } => {
                    let _ = app_inner.emit("agent:stream_retry", serde_json::json!({
                        "taskId": tid_inner, "streamId": id, "delaySecs": delay_secs,
                    }));
                    *active_stream_id.lock().unwrap() = None;
                }
                AgentEvent::Token(token) => {
                    let payload = serde_json::json!({
                        "taskId": tid_inner,
                        "streamId": active_stream_id.lock().unwrap().clone(),
                        "output": &token,
                        "content": &token,
                        "token": &token
                    });
                    let _ = app_inner.emit("agent:output", &payload);
                }
                AgentEvent::Output(output) => {
                    let payload = serde_json::json!({
                        "taskId": tid_inner,
                        "streamId": active_stream_id.lock().unwrap().clone(),
                        "output": &output,
                        "content": &output,
                        "token": &output
                    });
                    let _ = app_inner.emit("agent:output", &payload);
                }
                AgentEvent::Thought(thought) => {
                    let _ = app_inner.emit(
                        "agent:thought",
                        serde_json::json!({ "taskId": tid_inner, "streamId": active_stream_id.lock().unwrap().clone(), "thought": &thought, "delta": &thought }),
                    );
                }
                AgentEvent::ToolQueued { id, name, arguments, batch } => {
                    let _ = app_inner.emit(
                        "agent:tool_call",
                        serde_json::json!({
                            "taskId": tid_inner,
                            "id": id,
                            "name": name,
                            "arguments": arguments,
                            "batch": batch,
                            "status": "queued",
                        }),
                    );
                }
                AgentEvent::ToolCall { id, name, arguments, batch } => {
                    let _ = app_inner.emit(
                        "agent:tool_call",
                        serde_json::json!({
                            "taskId": tid_inner,
                            "id": id,
                            "name": name,
                            "arguments": arguments,
                            "batch": batch,
                            "status": "running",
                        }),
                    );

                    let parsed_args = if arguments.trim().is_empty() {
                        serde_json::json!({})
                    } else {
                        serde_json::from_str::<serde_json::Value>(&arguments)
                            .unwrap_or_else(|_| serde_json::json!({ "raw": &arguments }))
                    };
                    let mut tool_obj = serde_json::json!({
                        "id": &id,
                        "toolName": &name,
                        "name": &name,
                        "arguments": &arguments,
                        "params": parsed_args,
                        "status": "running",
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "durationMs": 0,
                    });
                    if let Some(ref b) = batch {
                        tool_obj["batch"] = b.clone();
                    }
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
                AgentEvent::ToolResult { id, name, result, success, duration_ms, batch } => {
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
                        serde_json::json!({
                            "taskId": tid_inner,
                            "id": id,
                            "name": name,
                            "result": &sanitized_result,
                            "success": success,
                            "durationMs": duration_ms,
                            "batch": batch,
                        }),
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
                        obj.insert("durationMs".to_string(), serde_json::json!(duration_ms.unwrap_or(0.0)));
                        if !success {
                            obj.insert("stderr".to_string(), serde_json::json!(&result));
                        }
                        if let Some(ref b) = batch {
                            obj.insert("batch".to_string(), b.clone());
                        }
                    }

                    let tool_json_str = serde_json::to_string(&tool_obj).unwrap_or_default();
                    // Invariant § 2.4 & Defect 6: Targeted update without deserializing/rewriting entire dialog history
                    let _ = crate::db::db_update_dialog_tool_result(&root_inner, &tid_inner, &id, &tool_json_str, duration_ms);
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
                    let resp_id = format!("resp-{}", uuid::Uuid::new_v4());
                    let _ = app_inner.emit(
                        "agent:finished",
                        serde_json::json!({
                            "taskId": tid_inner,
                            "status": "finished",
                            "response": &final_response,
                            "id": &resp_id
                        }),
                    );
                    if !final_response.trim().is_empty() {
                        let resp_item = crate::db::ActivityItem {
                            id: resp_id,
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
                    eprintln!("CVH [agent]: {}", err);
                }
            }
        };

        let result = run_agent_loop(agent_cfg, abort_flag, event_sink).await;
        let project_path_str = root_task.to_string_lossy().to_string();

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
                    serde_json::json!({ "taskId": tid, "status": "finished", "projectPath": &project_path_str }),
                );

                if let Some(state_ref) = app.try_state::<AppState>() {
                    state_ref.remove_agent_context(&root_task, &tid);
                }

                let mut tasks = load_tasks_from_disk(&root_task).await;
                if let Some(task_idx) = tasks.iter().position(|t| t.id == tid) {
                    let mut task_changed = false;

                    if tasks[task_idx].status.as_deref() != Some("done") {
                        tasks[task_idx].status = Some("idle".to_string());
                        tasks[task_idx].updated_at = Utc::now().timestamp_millis();
                        task_changed = true;
                    }

                    if task_changed {
                        let _ = crate::db::db_upsert_task(&root_task, &tasks[task_idx]);
                        let _ = save_tasks_to_disk(&root_task, &tasks).await;
                        if let Some(state_ref) = app.try_state::<AppState>() {
                            if state_ref.get_root() == root_task {
                                if let Ok(mut guard) = state_ref.tasks.lock() {
                                    *guard = tasks.clone();
                                }
                            }
                        }
                        let _ = app.emit("task:updated", serde_json::json!({ "task": &tasks[task_idx], "projectPath": &project_path_str }));
                        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks, "projectPath": &project_path_str }));
                        let _ = app.emit(
                            "agent:status",
                            serde_json::json!({ "taskId": tid, "status": "idle", "projectPath": &project_path_str }),
                        );
                    }
                }
            }
            Err(err) => {
                let is_aborted = abort_flag_task.load(Ordering::SeqCst);
                let err_str = err.to_string();

                if let Some(state_ref) = app.try_state::<AppState>() {
                    state_ref.remove_agent_context(&root_task, &tid);
                }

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
                        serde_json::json!({ "taskId": tid, "status": "idle", "projectPath": &project_path_str }),
                    );
                    let mut tasks = load_tasks_from_disk(&root_task).await;
                    if let Some(task_idx) = tasks.iter().position(|t| t.id == tid) {
                        if tasks[task_idx].status.as_deref() != Some("done") {
                            tasks[task_idx].status = Some("idle".to_string());
                            tasks[task_idx].updated_at = Utc::now().timestamp_millis();
                            let _ = crate::db::db_upsert_task(&root_task, &tasks[task_idx]);
                        }
                        let parent_id = tasks[task_idx].parent_id.clone();
                        if let Some(parent_idx) = tasks.iter().position(|t| {
                            t.waiting_for_subtask_id.as_deref().map(|s| s.eq_ignore_ascii_case(&tid)).unwrap_or(false)
                                || parent_id.as_deref().map(|pid| t.id.eq_ignore_ascii_case(pid) && t.waiting_for_subtask_id.is_some()).unwrap_or(false)
                        }) {
                            tasks[parent_idx].waiting_for_subtask_id = None;
                            if tasks[parent_idx].status.as_deref() == Some("waiting_subtask") {
                                tasks[parent_idx].status = Some("idle".to_string());
                            }
                            tasks[parent_idx].updated_at = Utc::now().timestamp_millis();
                            let _ = crate::db::db_upsert_task(&root_task, &tasks[parent_idx]);
                            let _ = app.emit("task:updated", serde_json::json!({ "task": &tasks[parent_idx], "projectPath": &project_path_str }));
                        }
                        let _ = save_tasks_to_disk(&root_task, &tasks).await;
                        if let Some(state_ref) = app.try_state::<AppState>() {
                            if state_ref.get_root() == root_task {
                                if let Ok(mut guard) = state_ref.tasks.lock() {
                                    *guard = tasks.clone();
                                }
                            }
                        }
                        let _ = app.emit("task:updated", serde_json::json!({ "task": &tasks[task_idx], "projectPath": &project_path_str }));
                        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks, "projectPath": &project_path_str }));
                    }
                } else {
                    eprintln!("CVH [run_agent]: Agent execution failed for task {}: {}", tid, err_str);
                    let _ = app.emit(
                        "agent:status",
                        serde_json::json!({ "taskId": tid, "status": "error", "error": &err_str, "projectPath": &project_path_str }),
                    );
                    let _ = app.emit(
                        "agent:error",
                        serde_json::json!({ "taskId": tid, "error": &err_str, "projectPath": &project_path_str }),
                    );

                    let mut tasks = load_tasks_from_disk(&root_task).await;
                    if let Some(task_idx) = tasks.iter().position(|t| t.id == tid) {
                        tasks[task_idx].status = Some("error".to_string());
                        tasks[task_idx].updated_at = Utc::now().timestamp_millis();
                        let _ = crate::db::db_upsert_task(&root_task, &tasks[task_idx]);
                        let parent_id = tasks[task_idx].parent_id.clone();
                        if let Some(parent_idx) = tasks.iter().position(|t| {
                            t.waiting_for_subtask_id.as_deref().map(|s| s.eq_ignore_ascii_case(&tid)).unwrap_or(false)
                                || parent_id.as_deref().map(|pid| t.id.eq_ignore_ascii_case(pid) && t.waiting_for_subtask_id.is_some()).unwrap_or(false)
                        }) {
                            tasks[parent_idx].waiting_for_subtask_id = None;
                            if tasks[parent_idx].status.as_deref() == Some("waiting_subtask") {
                                tasks[parent_idx].status = Some("idle".to_string());
                            }
                            tasks[parent_idx].updated_at = Utc::now().timestamp_millis();
                            let _ = crate::db::db_upsert_task(&root_task, &tasks[parent_idx]);
                            let _ = app.emit("task:updated", serde_json::json!({ "task": &tasks[parent_idx], "projectPath": &project_path_str }));
                        }
                        let _ = save_tasks_to_disk(&root_task, &tasks).await;
                        if let Some(state_ref) = app.try_state::<AppState>() {
                            if state_ref.get_root() == root_task {
                                if let Ok(mut guard) = state_ref.tasks.lock() {
                                    *guard = tasks.clone();
                                }
                            }
                        }
                        let _ = app.emit("task:updated", serde_json::json!({ "task": &tasks[task_idx], "projectPath": &project_path_str }));
                        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks, "projectPath": &project_path_str }));
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
            state_ref.remove_active_agent_if_same(&root_task, &tid, &abort_flag_task);
            state_ref.remove_agent_context(&root_task, &tid);
            crate::logger::cvh_log(
                Some(&root_task),
                "INFO",
                "RUN_AGENT",
                &format!("run_agent: active agent handle and context removed for task '{}'", tid),
            );
        }
    });

    {
        let mut guard = join_slot.lock().await;
        *guard = Some(handle);
    }

    Ok(format!("Agent launched for task {}", task_id))
}

pub use crate::agent::session_registry::is_matching_subagent_key;

/// Agent & Child Subagent Abort:
///
/// If task_id is a child subagent:
///   1. Aborts ONLY this specific subagent via `session_registry::abort_hierarchical_sessions`.
///   2. Marks mid-flight running tools for this subagent as failed in SQLite.
///   3. Leaves parent task and sibling subagents completely intact and running!
///
/// If task_id is a parent task:
///   1. Aborts parent task handle and cascaded child subagents via `session_registry::abort_hierarchical_sessions`.
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

    state.remove_agent_context(root, p_trim);

    // 1. Abort subagent and parent sessions hierarchically in session_registry
    let outcome = crate::agent::session_registry::abort_hierarchical_sessions(p_trim, state).await?;

    // 2. Emit session status updates for any aborted sessions
    if let Some(app) = app_handle {
        for sid in &outcome.aborted_session_ids {
            if let Some(summary) = state.session_registry.get_session(sid) {
                let _ = app.emit("agent:session_status", &summary);
            }
        }
    }

    // 3. Mark mid-flight running tools as failed in SQLite & emit tool_result
    if let Ok(aborted_tools) = crate::db::mark_running_tools_as_aborted(root, p_trim, outcome.is_subagent) {
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

    // 4. Update session statuses to Finished in session_registry
    for sid in &outcome.aborted_session_ids {
        state.session_registry.update_status(sid, crate::agent::session_registry::SessionStatus::Finished);
        if let Some(app) = app_handle {
            if let Some(summary) = state.session_registry.get_session(sid) {
                let _ = app.emit("agent:session_status", &summary);
            }
        }
    }

    // 5. Handle UI and task lifecycle: subagent vs parent task
    if outcome.is_subagent {
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
        return Ok(outcome.had_entry);
    }

    // Parent task: update task status on disk & memory to "idle"
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
    let subtask_parent_id = tasks.iter().find(|t| task_id_matches(&t.id, p_trim)).and_then(|t| t.parent_id.clone());
    if let Some(parent) = tasks.iter_mut().find(|t| {
        t.waiting_for_subtask_id.as_deref().map(|s| s.eq_ignore_ascii_case(p_trim)).unwrap_or(false)
            || subtask_parent_id.as_deref().map(|pid| t.id.eq_ignore_ascii_case(pid) && t.waiting_for_subtask_id.is_some()).unwrap_or(false)
    }) {
        parent.waiting_for_subtask_id = None;
        if parent.status.as_deref() == Some("waiting_subtask") {
            parent.status = Some("idle".to_string());
        }
        parent.updated_at = Utc::now().timestamp_millis();
        let _ = crate::db::db_upsert_task(root, parent);
        task_updated = true;
    }
    if let Some(t) = tasks.iter_mut().find(|t| task_id_matches(&t.id, p_trim)) {
        if t.status.as_deref() != Some("idle") {
            t.status = Some("idle".to_string());
            t.updated_at = Utc::now().timestamp_millis();
            let _ = crate::db::db_upsert_task(root, t);
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

    Ok(outcome.had_entry || task_updated)
}

pub async fn abort_agent(
    task_id: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let root = state.get_root();
    let p_trim = task_id.trim();
    state.remove_agent_context(&root, p_trim);
    let is_sub = crate::agent::session_registry::is_subagent_task_id(p_trim, Some(&state.active_agents)).await;
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

pub async fn compact_context(
    task_id: String,
    tool_id: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::agent::compactor::CompactionReport, String> {
    crate::agent::compactor::compact_context_for_task(task_id, tool_id, app_handle, state).await
}

pub fn persist_or_queue_steer(
    root: &Path,
    task_id: &str,
    message: &str,
    images: Option<Vec<String>>,
    message_id: Option<&str>,
    active: Option<&ActiveAgentHandle>,
) -> Result<(bool, Option<crate::db::ActivityItem>), String> {
    if let Some(entry) = active {
        if let Some(steering) = &entry.steering {
            let gate = steering.lock().map_err(|_| "Steering queue lock poisoned".to_string())?;
            let finished = entry.join_handle.try_lock().ok()
                .and_then(|handle| handle.as_ref().map(|handle| handle.is_finished())).unwrap_or(false);
            let queued = gate.accepting && !entry.abort_flag.load(Ordering::SeqCst) && !finished;
            let item = crate::agent::persist_steer_message(root, task_id, message, images, message_id)?;
            return Ok((queued, item));
        }
    }
    Ok((false, crate::agent::persist_steer_message(root, task_id, message, images, message_id)?))
}

pub async fn steer_agent(
    task_id: String,
    message: String,
    images: Option<Vec<String>>,
    message_id: Option<String>,
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

    let previous = state.get_active_agent(&root, &task_id);
    let was_aborted = previous.as_ref().is_some_and(|entry| entry.abort_flag.load(Ordering::SeqCst));
    let message_id = message_id.filter(|id| !id.trim().is_empty()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let (queued, item) = persist_or_queue_steer(&root, &task_id, &message, images.clone(), Some(&message_id), previous.as_ref())?;
    let inserted = item.is_some();
    if let Some(item) = item {
        let payload = serde_json::json!({ "taskId": task_id, "item": item });
        let _ = app_handle.emit("task:activity", &payload);
        let _ = app_handle.emit("task:dialog", &payload);
    }

    let steer_payload = serde_json::json!({
        "taskId": task_id,
        "content": message,
        "type": "steer",
        "images": images,
    });
    let _ = app_handle.emit("agent:steer", steer_payload);

    if !inserted {
        let cursor = crate::db::load_conversation_context(&root, &task_id)?.map(|context| context.dialog_cursor).unwrap_or(0);
        if !crate::db::load_dialog_items_after(&root, &task_id, cursor)?.iter().any(|item| item.id == message_id) {
            return Ok(format!("Steer message {} was already delivered", message_id));
        }
    }
    if queued {
        return Ok(format!("Steer queued for task {} until the current tool batch completes", task_id));
    }
    if let Some(entry) = previous {
        let is_running = entry.join_handle.try_lock().ok()
            .and_then(|h| h.as_ref().map(|h| !h.is_finished()))
            .unwrap_or(false);
        if is_running && !entry.abort_flag.load(Ordering::SeqCst) {
            return Ok(format!("Steer queued for active task {}", task_id));
        }
        if !was_aborted && entry.abort_flag.load(Ordering::SeqCst) && !entry.advance_aborted.load(Ordering::SeqCst) {
            return Ok(format!("Steer saved for stopped task {}", task_id));
        }
    }

    let disk_tasks = load_tasks_from_disk(&root).await;
    let current_task = disk_tasks.iter().find(|t| task_id_matches(&t.id, &task_id)).cloned();
    if let Some(ref t) = current_task {
        let is_terminal = t.completed_at.is_some() && (t.status.as_deref() == Some("done") || t.stage_id == "ready");
        if is_terminal {
            return Ok(format!("Task completed; steer saved to dialog, reopen to continue {}", task_id));
        }
        let waiting_sub = t.waiting_for_subtask_id.as_deref().filter(|s| !s.trim().is_empty()).map(str::to_string);
        if let Some(sub_id) = waiting_sub {
            if disk_tasks.iter().any(|c| task_id_matches(&c.id, &sub_id)) {
                crate::logger::cvh_log(Some(&root), "INFO", "STEER_AGENT", &format!("steer rerouted from waiting parent {} to subtask {}", task_id, sub_id));
                let _ = app_handle.emit("agent:status", serde_json::json!({ "taskId": sub_id, "status": "running", "projectPath": root.to_string_lossy().to_string() }));
                return Box::pin(steer_agent(sub_id, message, images, None, app_handle, state)).await;
            }
        }
    }

    {
        let mut current_tasks = load_tasks_from_disk(&root).await;
        if let Some(t) = current_tasks.iter_mut().find(|t| t.id == task_id) {
            t.updated_at = Utc::now().timestamp_millis();
            let _ = save_tasks_to_disk(&root, &current_tasks).await;
        }
        if state.get_root() == root {
            if let Ok(mut guard) = state.tasks.lock() {
                if let Some(t) = guard.iter_mut().find(|t| t.id == task_id) {
                    t.updated_at = Utc::now().timestamp_millis();
                }
            }
        }
    }

    run_agent_inner(task_id, Some(message), images, Some(root.to_string_lossy().to_string()), app_handle, false).await
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
                    b.and_then(|board| board.stages.iter().find(|c| c.id == task.stage_id).cloned())
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

pub async fn get_task_telemetry_summary(
    task_id: String,
    app_handle: AppHandle,
) -> Result<crate::db::TelemetrySummary, String> {
    let root = app_handle.state::<AppState>().get_root();
    crate::db::db_get_telemetry_summary(&root, task_id.trim())
}

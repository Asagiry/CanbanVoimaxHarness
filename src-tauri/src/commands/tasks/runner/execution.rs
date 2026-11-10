use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager};

use crate::agent::{run_agent_loop, AgentSessionConfig};
use crate::commands::state::{ActiveAgentHandle, AppState};
use crate::config::{load_boards_config, load_llm_settings, load_pipeline_config};
use crate::commands::healing::task_id_matches;
use crate::commands::tasks::storage::{load_tasks_from_disk, save_tasks_to_disk};
use super::execution_events::{build_agent_event_sink, handle_agent_run_completion};
use super::model_resolve::{cleanup_active_agent_reservation, resolve_task_active_agent_model};

pub async fn run_agent(
    task_id: String,
    prompt: Option<String>,
    images: Option<Vec<String>>,
    project_path: Option<String>,
    app_handle: AppHandle,
) -> Result<String, String> {
    run_agent_inner(task_id, prompt, images, project_path, app_handle, true).await
}

pub(crate) async fn run_agent_inner(
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
            .unwrap_or_else(|p| p.into_inner());

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

    let mut tasks_to_use = load_tasks_from_disk(&root).await;
    let initial_disk_count = tasks_to_use.len();
    let app_state = app_handle.state::<AppState>();
    let is_current_project = app_state.get_root() == root;
    if is_current_project {
        let guard = app_state.tasks.lock().unwrap_or_else(|p| p.into_inner());
        for mem_task in guard.iter() {
            if !tasks_to_use.iter().any(|t| task_id_matches(&t.id, &mem_task.id)) {
                tasks_to_use.push(mem_task.clone());
            }
        }
    }

    let task_opt = tasks_to_use.iter().find(|t| task_id_matches(&t.id, &task_id)).cloned();

    let mut task = match task_opt {
        Some(t) => {
            if is_current_project {
                let mut guard = app_state.tasks.lock().unwrap_or_else(|p| p.into_inner());
                *guard = tasks_to_use.clone();
            }
            if tasks_to_use.len() > initial_disk_count {
                let _ = save_tasks_to_disk(&root, &tasks_to_use).await;
            }
            t
        }
        None => {
            let mem_count = if is_current_project {
                app_state.tasks.lock().unwrap_or_else(|p| p.into_inner()).len()
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
            let has_existing_context = {
                let map = app_state.agent_contexts.lock().unwrap_or_else(|p| p.into_inner());
                map.get(&task_id).and_then(|cs| {
                    let g = cs.lock().unwrap_or_else(|p| p.into_inner());
                    Some(g.messages.len() > 2)
                }).unwrap_or(false)
            };

            if task.description.trim().is_empty() && !has_existing_context && images.is_none() {
                task.description = p_trimmed.to_string();
                task.updated_at = Utc::now().timestamp_millis();

                let mut all_tasks = load_tasks_from_disk(&root).await;
                if let Some(t) = all_tasks.iter_mut().find(|t| task_id_matches(&t.id, &task_id)) {
                    t.description = task.description.clone();
                    t.updated_at = task.updated_at;
                    let _ = save_tasks_to_disk(&root, &all_tasks).await;
                }
                let mut guard = app_state.tasks.lock().unwrap_or_else(|p| p.into_inner());
                if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, &task_id)) {
                    t.description = task.description.clone();
                    t.updated_at = task.updated_at;
                }
                None
            } else if p_trimmed == task.description.trim() && images.is_none() {
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
                    hooks: HashMap::new(),
                    auto_run: None,
                    is_intake: None,
                    terminal: None,
                })
            })
    };

    let persona_role = col
        .persona
        .clone()
        .unwrap_or_else(|| "developer".to_string());

    let agent_setting = crate::config::load_agent_setting(&root, &persona_role).await;
    let resolved_context_window = agent_setting.context_window.unwrap_or(128_000);

    let llm_settings = load_llm_settings(&root).await.unwrap_or_default();
    let model = resolve_task_active_agent_model(&root, Some(&task), &llm_settings).await;
    if model.trim().is_empty() {
        return Err("Модель не выбрана: укажите модель в карточке задачи, настройках агента или Settings → Providers".to_string());
    }

    let base_url = llm_settings
        .providers
        .iter()
        .find(|p| p.models.iter().any(|m| m.id == model))
        .map(|p| p.base_url.clone())
        .unwrap_or_else(|| "http://localhost:11434/v1".to_string());

    let api_key = llm_settings
        .providers
        .iter()
        .find(|p| p.models.iter().any(|m| m.id == model))
        .map(|p| p.api_key.clone());

    let worktree_dir = if let Some(ref rel_wt) = task.worktree_path {
        let trimmed = rel_wt.trim();
        if !trimmed.is_empty() {
            let pb = std::path::Path::new(trimmed);
            if pb.is_absolute() {
                pb.to_path_buf()
            } else {
                root.join(pb)
            }
        } else {
            root.clone()
        }
    } else {
        root.clone()
    };

    let session_id = format!("sess-{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().timestamp_millis();
    let session_summary = crate::agent::session_registry::AgentSessionSummary {
        session_id: session_id.clone(),
        task_id: task_id.clone(),
        stage_id: Some(task.stage_id.clone()),
        started_at: now as u64,
        status: crate::agent::session_registry::SessionStatus::Running,
        project_path: Some(root.to_string_lossy().to_string()),
    };

    {
        let state = app_handle.state::<AppState>();
        state.session_registry.register(
            session_summary.clone(),
            Arc::clone(&abort_flag),
            Arc::clone(&join_slot),
        );
        let _ = state.get_or_create_agent_context(&root, &task_id);
    }

    {
        let mut all_tasks = load_tasks_from_disk(&root).await;
        if let Some(t) = all_tasks.iter_mut().find(|t| task_id_matches(&t.id, &task_id)) {
            t.status = Some("running".to_string());
            t.completed_at = None;
            t.waiting_for_subtask_id = None;
            t.updated_at = Utc::now().timestamp_millis();
            let _ = save_tasks_to_disk(&root, &all_tasks).await;
        }
        if app_handle.state::<AppState>().get_root() == root {
            if let Ok(mut guard) = app_handle.state::<AppState>().tasks.lock() {
                *guard = all_tasks;
            }
        }
    }

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
        temperature: col.temperature,
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

        let event_sink = build_agent_event_sink(app_inner, tid_inner, root_inner, model_inner);
        let result = run_agent_loop(agent_cfg, abort_flag, event_sink).await;

        handle_agent_run_completion(
            result,
            tid,
            root_task,
            app,
            abort_flag_task,
            session_id_task,
        ).await;
    });

    {
        let mut guard = join_slot.lock().await;
        *guard = Some(handle);
    }

    Ok(format!("Agent launched for task {}", task_id))
}

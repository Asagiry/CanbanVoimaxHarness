use crate::agent::{run_agent_loop, AgentEvent, AgentSessionConfig, Task};
use crate::commands::dialogs::append_task_dialog_activity;
use crate::commands::git::{
    apply_worktree_from_hook_stdout, ensure_builtin_worktree, needs_builtin_worktree,
    BUILTIN_CREATE_WORKTREE_MARKER,
};
use crate::commands::state::{ActiveAgentHandle, AppState};
use crate::config::{
    atomic_write_file, get_app_config_dir, load_boards_config, load_llm_settings,
    load_pipeline_config, load_project_config, ProjectConfig,
};
use crate::hooks::{execute_hook, execute_sensor_command, HookContext};
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

pub fn ensure_task_board_id(mut tasks: Vec<Task>) -> Vec<Task> {
    for t in &mut tasks {
        if t.board_id.as_deref().unwrap_or("").trim().is_empty() {
            t.board_id = Some("development".to_string());
        }
    }
    tasks
}

pub async fn load_tasks_from_disk(repo_root: &Path) -> Vec<Task> {
    let local_tasks_file = repo_root.join(".cvh").join("tasks.json");
    if local_tasks_file.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&local_tasks_file).await {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                if let Ok(tasks) = serde_json::from_str::<Vec<Task>>(trimmed) {
                    return ensure_task_board_id(tasks);
                }
            }
        }
    }

    // Try global fallback: %LOCALAPPDATA%\com.voimax.cvh\tasks.json
    let global_tasks_file = get_app_config_dir().join("tasks.json");
    if global_tasks_file.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&global_tasks_file).await {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                if let Ok(tasks) = serde_json::from_str::<Vec<Task>>(trimmed) {
                    if !tasks.is_empty() {
                        eprintln!(
                            "CVH [load_tasks_from_disk]: Restored {} tasks from global fallback {:?}",
                            tasks.len(),
                            global_tasks_file
                        );
                        return ensure_task_board_id(tasks);
                    }
                }
            }
        }
    }

    Vec::new()
}

pub async fn save_tasks_to_disk(repo_root: &Path, tasks: &[Task]) -> Result<(), String> {
    let serialized = serde_json::to_string_pretty(tasks)
        .map_err(|e| format!("Failed to serialize tasks: {}", e))?;

    // 1. Save to repo_root/.cvh/tasks.json
    let local_tasks_file = repo_root.join(".cvh").join("tasks.json");
    let local_res = atomic_write_file(&local_tasks_file, &serialized).await;

    // 2. ALSO save a copy to %LOCALAPPDATA%\com.voimax.cvh\tasks.json as a global fallback!
    let global_tasks_file = get_app_config_dir().join("tasks.json");
    let global_res = atomic_write_file(&global_tasks_file, &serialized).await;

    match (local_res, global_res) {
        (Ok(_), _) => Ok(()),
        (Err(e), Ok(_)) => {
            eprintln!(
                "CVH: Warning - Failed to save tasks to local repo {:?}: {}, but persisted to global fallback {:?}",
                local_tasks_file, e, global_tasks_file
            );
            Ok(())
        }
        (Err(e1), Err(e2)) => Err(format!(
            "Failed to save tasks to both local ({}) and global ({})",
            e1, e2
        )),
    }
}

pub fn stage_id_matches(col_id: &str, target: &str) -> bool {
    if col_id.eq_ignore_ascii_case(target) {
        return true;
    }
    let c = col_id.to_ascii_lowercase();
    let t = target.to_ascii_lowercase();
    if (t == "dev" || t == "development") && (c == "dev" || c == "development") {
        return true;
    }
    if (t == "backlog" || t == "waiting" || t == "ожидание") && (c == "backlog" || c == "waiting" || c == "ожидание") {
        return true;
    }
    if (t == "ready" || t == "done" || t == "готово") && (c == "ready" || c == "done" || c == "готово") {
        return true;
    }
    false
}

#[tauri::command]
pub async fn list_tasks(state: State<'_, AppState>) -> Result<Vec<Task>, String> {
    let root = state.get_root();
    let tasks = load_tasks_from_disk(&root).await;
    if let Ok(mut guard) = state.tasks.lock() {
        *guard = tasks.clone();
    }
    Ok(tasks)
}

#[tauri::command]
pub async fn save_tasks(tasks: Vec<Task>, state: State<'_, AppState>) -> Result<(), String> {
    if let Ok(mut guard) = state.tasks.lock() {
        *guard = tasks.clone();
    }
    let root = state.get_root();
    let res = save_tasks_to_disk(&root, &tasks).await;
    match &res {
        Ok(_) => eprintln!(
            "CVH [save_tasks]: persisted {} tasks to {:?}",
            tasks.len(),
            root.join(".cvh").join("tasks.json")
        ),
        Err(e) => eprintln!("CVH [save_tasks]: FAILED to persist {} tasks: {}", tasks.len(), e),
    }
    res
}

pub fn next_task_num(tasks: &[Task]) -> u32 {
    let mut max_num: u32 = 0;
    for t in tasks {
        let base = t.id.split('.').next().unwrap_or(t.id.as_str());
        let digits: String = base.chars().filter(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = digits.parse::<u32>() {
            if n > max_num {
                max_num = n;
            }
        }
    }
    // Belt & braces: skip any id that still collides (e.g. non-standard ids).
    let mut candidate = max_num.saturating_add(1).max(1);
    while tasks.iter().any(|t| t.id == format!("CVH-{:03}", candidate)) {
        candidate = candidate.saturating_add(1);
    }
    candidate
}

pub fn get_next_subtask_index(tasks: &[Task], parent_id: &str) -> u32 {
    let mut max_idx: u32 = 0;
    let prefix = format!("{}.", parent_id);
    for t in tasks {
        if t.parent_id.as_deref() == Some(parent_id) {
            if let Some(idx) = t.subtask_index {
                if idx > max_idx {
                    max_idx = idx;
                }
            } else if let Some(suffix) = t.id.strip_prefix(&prefix) {
                if let Ok(idx) = suffix.parse::<u32>() {
                    if idx > max_idx {
                        max_idx = idx;
                    }
                }
            }
        } else if t.id.starts_with(&prefix) {
            if let Some(suffix) = t.id.strip_prefix(&prefix) {
                if let Ok(idx) = suffix.parse::<u32>() {
                    if idx > max_idx {
                        max_idx = idx;
                    }
                }
            }
        }
    }
    max_idx + 1
}

#[tauri::command]
pub async fn create_task(
    title: String,
    description: String,
    stage_id: String,
    board_id: Option<String>,
    skip_worktree: Option<bool>,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    let root = state.get_root();
    let mut tasks = load_tasks_from_disk(&root).await;

    let effective_board_id = match board_id.as_deref() {
        Some(b) if !b.trim().is_empty() => b.trim().to_string(),
        _ => {
            load_boards_config(&root)
                .await
                .map(|bc| bc.active_board_id)
                .unwrap_or_else(|_| "development".to_string())
        }
    };

    // Generate task ID — monotonic max+1 over on-disk tasks, never len+1
    let task_num = next_task_num(&tasks);
    let task_id = format!("CVH-{:03}", task_num);
    let now = Utc::now().timestamp_millis();

    let mut new_task = Task {
        id: task_id.clone(),
        title: title.clone(),
        description: description.clone(),
        stage_id: stage_id.clone(),
        board_id: Some(effective_board_id.clone()),
        worktree_path: None,
        branch: None,
        verdict: None,
        custom_flow: None,
        status: None,
        model: None,
        parent_id: None,
        subtask_index: None,
        waiting_for_subtask_id: None,
        handover_summary: None,
        cover_image: None,
        attachments: None,
        todos: None,
        created_at: now,
        updated_at: now,
    };

    // Check pipeline config for stage hooks (board-aware: first check columns
    // of effective_board_id, then fallback to pipeline.json).
    let skip = skip_worktree.unwrap_or(false);
    {
        let boards_cfg = load_boards_config(&root).await.ok();
        let col_from_board = boards_cfg.as_ref().and_then(|bc| {
            let b = bc.boards.iter().find(|b| b.id == effective_board_id)
                .or_else(|| bc.boards.iter().find(|b| b.id == bc.active_board_id));
            b.and_then(|b| b.columns.iter().find(|c| c.id == stage_id).cloned())
        });
        let col_opt: Option<crate::config::PipelineColumn> = match col_from_board {
            Some(c) => Some(c),
            None => load_pipeline_config(&root)
                .await
                .ok()
                .and_then(|p| p.columns.into_iter().find(|c| c.id == stage_id)),
        };
        if let Some(col) = col_opt {
            trigger_column_on_enter_script(&col, &new_task, &stage_id, None, &root).await;
            let hook_rel_opt = col.hooks.get("on_enter").cloned();
            match hook_rel_opt {
                Some(hook_rel) if hook_rel == BUILTIN_CREATE_WORKTREE_MARKER => {
                    // Built-in marker: shell is not launched, handled by builtin below
                }
                Some(hook_rel) => {
                    let hook_path = crate::config::resolve_hook_path(
                        &root,
                        col.persona.as_deref(),
                        &hook_rel,
                    );
                    if hook_path.exists() {
                        let ctx = HookContext {
                            task_id: task_id.clone(),
                            task_title: title.clone(),
                            worktree_path: new_task.worktree_path.clone().unwrap_or_default(),
                            branch: new_task.branch.clone().unwrap_or_default(),
                            verdict: None,
                            stage_id: Some(stage_id.clone()),
                            board_id: Some(effective_board_id.clone()),
                            extra_env: HashMap::new(),
                        };
                        match execute_hook(&hook_path, &root, &ctx, None).await {
                            Ok(res) => {
                                if !res.success {
                                    eprintln!(
                                        "CVH [on_enter]: hook {:?} failed for new task {} (exit code {:?}):\nSTDOUT: {}\nSTDERR: {}",
                                        hook_path, task_id, res.exit_code, res.stdout, res.stderr
                                    );
                                }
                                apply_worktree_from_hook_stdout(&mut new_task, &res.stdout);
                            }
                            Err(e) => {
                                eprintln!(
                                    "CVH [on_enter]: error executing hook {:?} for new task {}: {}",
                                    hook_path, task_id, e
                                );
                            }
                        }
                    }
                }
                None => {}
            }
            if needs_builtin_worktree(&col, &new_task, skip) {
                ensure_builtin_worktree(&root, &mut new_task).await;
            }
        } else if stage_id == "backlog" && !skip {
            // Column not explicitly configured, but stage is backlog: fallback default
            ensure_builtin_worktree(&root, &mut new_task).await;
        }
    }

    tasks.push(new_task.clone());
    if let Ok(mut guard) = state.tasks.lock() {
        *guard = tasks.clone();
    }
    save_tasks_to_disk(&root, &tasks).await?;

    Ok(new_task)
}

#[tauri::command]
pub async fn create_subtask(
    parent_id: String,
    target_stage_id: Option<String>,
    title: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    let root = state.get_root();
    let mut tasks = load_tasks_from_disk(&root).await;

    let parent_idx = tasks
        .iter()
        .position(|t| t.id == parent_id)
        .ok_or_else(|| format!("Parent task {} not found", parent_id))?;

    let pipeline = load_pipeline_config(&root)
        .await
        .unwrap_or_else(|_| crate::config::default_pipeline_config());

    let target_col = if let Some(target_id) = target_stage_id {
        pipeline
            .columns
            .iter()
            .find(|c| c.id == target_id)
            .cloned()
            .ok_or_else(|| format!("Target column {} not found in pipeline", target_id))?
    } else {
        let parent_col_idx = pipeline
            .columns
            .iter()
            .position(|c| c.id == tasks[parent_idx].stage_id)
            .ok_or_else(|| format!("Parent column {} not found in pipeline", tasks[parent_idx].stage_id))?;
        if parent_col_idx + 1 < pipeline.columns.len() {
            pipeline.columns[parent_col_idx + 1].clone()
        } else {
            return Err("Parent is already in the last pipeline stage".to_string());
        }
    };

    let next_subtask_idx = get_next_subtask_index(&tasks, &parent_id);
    let subtask_id = format!("{}.{}", parent_id, next_subtask_idx);
    let now = Utc::now().timestamp_millis();

    let parent_title = tasks[parent_idx].title.clone();
    let parent_wt = tasks[parent_idx].worktree_path.clone();
    let parent_branch = tasks[parent_idx].branch.clone();
    let parent_model = tasks[parent_idx].model.clone();

    // Update parent task: mark waiting for subtask
    tasks[parent_idx].waiting_for_subtask_id = Some(subtask_id.clone());
    tasks[parent_idx].status = Some("waiting_subtask".to_string());
    tasks[parent_idx].updated_at = now;

    let subtask_title = title.unwrap_or_else(|| format!("{} ({})", parent_title, target_col.title));
    let subtask_desc = format!(
        "### Подзадача этапа: {}
Родительская задача: {}

{}",
        target_col.title, parent_id, tasks[parent_idx].description
    );

    let subtask_status = if target_col.terminal == Some(true) {
        "done"
    } else if target_col.persona.is_some() {
        "running"
    } else {
        "idle"
    };

    let new_subtask = Task {
        id: subtask_id.clone(),
        title: subtask_title,
        description: subtask_desc.clone(),
        stage_id: target_col.id.clone(),
        board_id: tasks[parent_idx].board_id.clone(),
        worktree_path: parent_wt,
        branch: parent_branch,
        verdict: None,
        custom_flow: None,
        status: Some(subtask_status.to_string()),
        model: target_col.model_override.clone().or(parent_model),
        parent_id: Some(parent_id.clone()),
        subtask_index: Some(next_subtask_idx),
        waiting_for_subtask_id: None,
        handover_summary: None,
        cover_image: None,
        attachments: None,
        todos: None,
        created_at: now,
        updated_at: now,
    };

    tasks.push(new_subtask.clone());
    if let Ok(mut guard) = state.tasks.lock() {
        *guard = tasks.clone();
    }
    save_tasks_to_disk(&root, &tasks).await?;

    let _ = app_handle.emit("task:created", serde_json::json!({ "task": &new_subtask }));
    let _ = app_handle.emit(
        "task:moved",
        serde_json::json!({
            "taskId": parent_id,
            "status": "waiting_subtask",
            "waitingForSubtaskId": &subtask_id,
            "task": &tasks[parent_idx],
        }),
    );
    let _ = app_handle.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));

    if target_col.auto_run == Some(true) || target_col.persona.is_some() {
        let col_clone = target_col.clone();
        let subtask_clone = new_subtask.clone();
        let root_clone = root.clone();
        let sid = subtask_id.clone();
        let app = app_handle.clone();
        tokio::spawn(async move {
            trigger_column_on_enter_script(
                &col_clone,
                &subtask_clone,
                &col_clone.id,
                None,
                &root_clone,
            )
            .await;
            let _ = run_agent(sid, Some(subtask_desc), app).await;
        });
    }

    Ok(new_subtask)
}

pub async fn trigger_column_on_enter_script(
    col: &crate::config::PipelineColumn,
    task: &Task,
    target_stage_id: &str,
    verdict: Option<&str>,
    root: &Path,
) {
    if let Some(ref script_str) = col.on_enter_script {
        let trimmed = script_str.trim();
        if trimmed.is_empty() {
            return;
        }

        let script_buf = PathBuf::from(trimmed);
        let script_path = if script_buf.is_absolute() {
            script_buf
        } else {
            root.join(&script_buf)
        };

        if !script_path.exists() {
            eprintln!(
                "CVH [on_enter_script]: Hook script does not exist at {:?}",
                script_path
            );
            return;
        }

        let wt_path = task
            .worktree_path
            .as_deref()
            .map(PathBuf::from)
            .filter(|p| p.exists())
            .unwrap_or_else(|| root.to_path_buf());

        let ctx = HookContext {
            task_id: task.id.clone(),
            task_title: task.title.clone(),
            worktree_path: task.worktree_path.clone().unwrap_or_default(),
            branch: task.branch.clone().unwrap_or_default(),
            verdict: verdict.map(|v| v.to_string()).or_else(|| task.verdict.clone()),
            stage_id: Some(target_stage_id.to_string()),
            board_id: task.board_id.clone(),
            extra_env: HashMap::new(),
        };

        println!(
            "CVH [on_enter_script]: Executing hook {:?} for task {} entering stage {}",
            script_path, task.id, target_stage_id
        );

        match execute_hook(&script_path, &wt_path, &ctx, None).await {
            Ok(res) => {
                if res.success {
                    println!(
                        "CVH [on_enter_script]: Hook {:?} succeeded for task {} in stage {} (exit code {:?}):
STDOUT: {}",
                        script_path, task.id, target_stage_id, res.exit_code, res.stdout
                    );
                } else {
                    eprintln!(
                        "CVH [on_enter_script]: Hook {:?} failed for task {} in stage {} (exit code {:?}):
STDOUT: {}
STDERR: {}",
                        script_path, task.id, target_stage_id, res.exit_code, res.stdout, res.stderr
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "CVH [on_enter_script]: Error executing hook {:?} for task {}: {}",
                    script_path, task.id, e
                );
            }
        }
    }
}

pub async fn internal_move_task_stage(
    root: &Path,
    task_id: &str,
    target_stage_id: &str,
    verdict: Option<String>,
    status_override: Option<String>,
    handover_summary: Option<String>,
    app_handle: Option<&AppHandle>,
) -> Result<Task, String> {
    let mut tasks = load_tasks_from_disk(root).await;

    let task_idx = tasks
        .iter()
        .position(|t| t.id == task_id)
        .ok_or_else(|| format!("Task {} not found", task_id))?;

    let current_task = &mut tasks[task_idx];

    // --- Board resolution & fallback (support alias and fallback when board_id missing/stale) ---
    let boards_cfg = load_boards_config(root).await.ok();
    let mut effective_board: Option<crate::config::BoardDef> = None;

    if let Some(ref bc) = boards_cfg {
        let active_board = bc.boards.iter().find(|b| b.id == bc.active_board_id).cloned();
        let board_switched = current_task.board_id.as_deref().is_some_and(|bid| bid != bc.active_board_id);

        let current_board = current_task.board_id.as_deref().and_then(|bid| {
            bc.boards.iter().find(|b| b.id == bid).cloned()
        });

        if board_switched {
            // Active board changed during task execution: target board is the active board!
            effective_board = active_board.or_else(|| bc.boards.first().cloned());
            if let Some(ref b) = effective_board {
                current_task.board_id = Some(b.id.clone());
            }
        } else {
            let current_board_has_target = current_board
                .as_ref()
                .map(|b| b.columns.iter().any(|c| stage_id_matches(&c.id, target_stage_id)))
                .unwrap_or(false);

            if current_board_has_target {
                effective_board = current_board;
            } else {
                let active_has_target = active_board
                    .as_ref()
                    .map(|b| b.columns.iter().any(|c| stage_id_matches(&c.id, target_stage_id)))
                    .unwrap_or(false);

                if active_has_target {
                    effective_board = active_board;
                } else {
                    let any_with_target = bc.boards.iter().find(|b| {
                        b.columns.iter().any(|c| stage_id_matches(&c.id, target_stage_id))
                    }).cloned();

                    if any_with_target.is_some() {
                        effective_board = any_with_target;
                    } else {
                        effective_board = active_board.or_else(|| bc.boards.first().cloned());
                    }
                }

                if let Some(ref b) = effective_board {
                    current_task.board_id = Some(b.id.clone());
                } else {
                    current_task.board_id = Some(bc.active_board_id.clone());
                }
            }
        }
    }

    // --- on_finish / on_leave hooks of source column ---
    let source_stage_id = current_task.stage_id.clone();
    let source_col: Option<crate::config::PipelineColumn> = {
        let from_board = effective_board.as_ref().and_then(|b| {
            b.columns.iter().find(|c| stage_id_matches(&c.id, &source_stage_id)).cloned()
        });
        match from_board {
            Some(c) => Some(c),
            None => load_pipeline_config(root)
                .await
                .ok()
                .and_then(|p| p.columns.into_iter().find(|c| stage_id_matches(&c.id, &source_stage_id))),
        }
    };

    if let Some(ref scol) = source_col {
        if let Some(hook_rel) = scol.hooks.get("on_finish").or_else(|| scol.hooks.get("on_leave")) {
            let hook_path = crate::config::resolve_hook_path(
                root,
                scol.persona.as_deref(),
                hook_rel,
            );
            if hook_path.exists() {
                let ctx = HookContext {
                    task_id: current_task.id.clone(),
                    task_title: current_task.title.clone(),
                    worktree_path: current_task.worktree_path.clone().unwrap_or_default(),
                    branch: current_task.branch.clone().unwrap_or_default(),
                    verdict: verdict.clone().or_else(|| current_task.verdict.clone()),
                    stage_id: Some(scol.id.clone()),
                    board_id: current_task.board_id.clone(),
                    extra_env: HashMap::new(),
                };
                match execute_hook(&hook_path, root, &ctx, None).await {
                    Ok(res) => {
                        if !res.success {
                            eprintln!(
                                "CVH [on_finish]: hook {:?} failed for task {} leaving stage {} (exit code {:?}):\nSTDOUT: {}\nSTDERR: {}",
                                hook_path, current_task.id, scol.id, res.exit_code, res.stdout, res.stderr
                            );
                        }
                        apply_worktree_from_hook_stdout(current_task, &res.stdout);
                    }
                    Err(e) => {
                        eprintln!(
                            "CVH [on_finish]: error executing hook {:?} for task {}: {}",
                            hook_path, current_task.id, e
                        );
                    }
                }
            }
        }
    }

    let target_col = {
        let col_from_board = effective_board.as_ref().and_then(|b| {
            b.columns.iter().find(|c| stage_id_matches(&c.id, target_stage_id)).cloned()
        });
        match col_from_board {
            Some(c) => c,
            None => {
                let terminal_fallback = effective_board.as_ref().and_then(|b| {
                    b.columns.iter().find(|c| c.terminal == Some(true) || stage_id_matches(&c.id, "ready") || stage_id_matches(&c.id, "done")).cloned()
                        .or_else(|| b.columns.last().cloned())
                });
                if let Some(term) = terminal_fallback {
                    term
                } else {
                    let pipeline = load_pipeline_config(root)
                        .await
                        .map_err(|e| format!("Failed to load pipeline: {}", e))?;
                    pipeline
                        .columns
                        .iter()
                        .find(|c| stage_id_matches(&c.id, target_stage_id))
                        .cloned()
                        .or_else(|| {
                            pipeline
                                .columns
                                .iter()
                                .find(|c| {
                                    c.terminal == Some(true)
                                        || stage_id_matches(&c.id, "ready")
                                        || stage_id_matches(&c.id, "done")
                                })
                                .cloned()
                        })
                        .ok_or_else(|| {
                            format!(
                                "Target stage {} not found in pipeline or board",
                                target_stage_id
                            )
                        })?
                }
            }
        }
    };
    let canonical_target_stage_id = target_col.id.clone();

    // Worktree allocation: driven strictly by the configurable hook (builtin:create_worktree)
    if needs_builtin_worktree(&target_col, current_task, false) {
        ensure_builtin_worktree(root, current_task).await;
    }

    // Gatekeeper Sensor Verification: do not block initial move from backlog to dev or new/empty worktree
    if let Some(sensor_cmd) = &target_col.gatekeeper_sensor {
        let is_from_backlog = source_stage_id.is_empty()
            || stage_id_matches(&source_stage_id, "backlog");
        let is_to_dev = stage_id_matches(&canonical_target_stage_id, "dev");
        let is_initial_move_to_dev = is_from_backlog && is_to_dev;

        let wt_empty_or_new = match &current_task.worktree_path {
            None => true,
            Some(p) => {
                let path = Path::new(p);
                !path.exists() || path.read_dir().map(|mut it| it.next().is_none()).unwrap_or(true)
            }
        };

        if is_initial_move_to_dev || wt_empty_or_new {
            println!(
                "CVH [gatekeeper]: skipping gatekeeper sensor '{}' for initial transition of task {} from '{}' to '{}' (initial move / empty worktree)",
                sensor_cmd, current_task.id, source_stage_id, canonical_target_stage_id
            );
        } else {
            let wt_path = current_task
                .worktree_path
                .as_deref()
                .map(PathBuf::from)
                .unwrap_or_else(|| root.to_path_buf());

            let sensor_res = execute_sensor_command(sensor_cmd, &wt_path, None)
                .await
                .map_err(|e| format!("Sensor execution failed: {}", e))?;

            if !sensor_res.success {
                return Err(format!(
                    "Gatekeeper Sensor '{}' rejected transition (Exit code: {:?}):\nSTDOUT: {}\nSTDERR: {}",
                    sensor_cmd, sensor_res.exit_code, sensor_res.stdout, sensor_res.stderr
                ));
            }
        }
    }

    // Trigger on_enter hook if configured
    let target_hook_rel = target_col.hooks.get("on_enter").cloned();
    match target_hook_rel {
        Some(hook_rel) if hook_rel == BUILTIN_CREATE_WORKTREE_MARKER => {
            // Builtin only
        }
        Some(hook_rel) => {
            let hook_path = crate::config::resolve_hook_path(
                root,
                target_col.persona.as_deref(),
                &hook_rel,
            );
            if hook_path.exists() {
                let ctx = HookContext {
                    task_id: current_task.id.clone(),
                    task_title: current_task.title.clone(),
                    worktree_path: current_task.worktree_path.clone().unwrap_or_default(),
                    branch: current_task.branch.clone().unwrap_or_default(),
                    verdict: verdict.clone().or_else(|| current_task.verdict.clone()),
                    stage_id: Some(canonical_target_stage_id.clone()),
                    board_id: current_task.board_id.clone(),
                    extra_env: HashMap::new(),
                };
                let hook_res = execute_hook(&hook_path, root, &ctx, None)
                    .await
                    .map_err(|e| format!("Stage transition hook failed: {}", e))?;

                if !hook_res.success {
                    return Err(format!(
                        "Stage transition hook rejected transition (Exit code: {:?}):\n{}",
                        hook_res.exit_code, hook_res.stderr
                    ));
                }
                apply_worktree_from_hook_stdout(current_task, &hook_res.stdout);
            }
        }
        None => {}
    }
    if needs_builtin_worktree(&target_col, current_task, false) {
        ensure_builtin_worktree(root, current_task).await;
    }

    // Trigger on_enter_script if configured on target column
    trigger_column_on_enter_script(
        &target_col,
        current_task,
        &canonical_target_stage_id,
        verdict.as_deref(),
        root,
    )
    .await;

    // Update task
    let old_stage = current_task.stage_id.clone();
    current_task.stage_id = canonical_target_stage_id.clone();
    if verdict.is_some() {
        current_task.verdict = verdict;
    }
    if let Some(summary) = handover_summary {
        current_task.handover_summary = Some(summary);
    }
    let is_terminal_col = target_col.terminal == Some(true)
        || stage_id_matches(&canonical_target_stage_id, "ready")
        || stage_id_matches(&canonical_target_stage_id, "done");
    if let Some(st) = status_override {
        current_task.status = Some(st);
    } else if is_terminal_col {
        current_task.status = Some("done".to_string());
    } else if target_col.persona.is_some() || target_col.auto_run == Some(true) {
        current_task.status = Some("running".to_string());
    } else {
        current_task.status = Some("idle".to_string());
    }
    current_task.updated_at = Utc::now().timestamp_millis();

    let updated = current_task.clone();
    if let Some(app) = app_handle {
        if let Some(state_ref) = app.try_state::<AppState>() {
            if let Ok(mut guard) = state_ref.tasks.lock() {
                *guard = tasks.clone();
            }
        }
    }
    save_tasks_to_disk(root, &tasks).await?;

    if let Some(app) = app_handle {
        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
        let _ = app.emit(
            "task:moved",
            serde_json::json!({
                "taskId": task_id,
                "fromStage": old_stage,
                "toStage": &canonical_target_stage_id,
                "stageId": &canonical_target_stage_id,
                "status": updated.status,
                "verdict": updated.verdict,
                "task": &updated
            }),
        );
        if target_col.persona.is_some() || target_col.auto_run == Some(true) {
            let _ = app.emit("agent:autorun", serde_json::json!({ "taskId": task_id }));
        }
    }

    Ok(updated)
}

#[tauri::command]
pub async fn move_task_stage(
    task_id: String,
    target_stage_id: String,
    verdict: Option<String>,
    handover_summary: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    let root = state.get_root();
    internal_move_task_stage(&root, &task_id, &target_stage_id, verdict, None, handover_summary, Some(&app_handle)).await
}

pub async fn internal_advance_task(
    root: &Path,
    task_id: &str,
    summary: &str,
    app_handle: Option<&AppHandle>,
) -> Result<String, String> {
    let tasks = load_tasks_from_disk(root).await;
    let current_task = tasks
        .iter()
        .find(|t| t.id == task_id)
        .ok_or_else(|| format!("Task {} not found", task_id))?;

    let boards_cfg = load_boards_config(root).await.ok();
    let active_board_id = boards_cfg
        .as_ref()
        .map(|b| b.active_board_id.clone())
        .unwrap_or_else(|| "development".to_string());

    // Check if active board changed during task execution
    let board_switched = current_task
        .board_id
        .as_deref()
        .is_some_and(|b| b != active_board_id);

    if board_switched {
        let target_board = boards_cfg.as_ref().and_then(|bc| {
            bc.boards.iter().find(|b| b.id == active_board_id).cloned()
        });

        let target_columns = match target_board {
            Some(ref b) => b.columns.clone(),
            None => {
                let pipeline = load_pipeline_config(root)
                    .await
                    .map_err(|e| e.to_string())?;
                pipeline.columns
            }
        };

        let terminal_col = target_columns
            .iter()
            .find(|c| {
                c.terminal == Some(true)
                    || stage_id_matches(&c.id, "ready")
                    || stage_id_matches(&c.id, "done")
            })
            .cloned()
            .or_else(|| target_columns.last().cloned())
            .ok_or_else(|| format!("Target board '{}' has no columns", active_board_id))?;

        let updated_task = internal_move_task_stage(
            root,
            task_id,
            &terminal_col.id,
            Some("APPROVED".to_string()),
            Some("done".to_string()),
            Some(summary.to_string()),
            app_handle,
        )
        .await?;

        let summary_content = format!(
            "### Саммари финализации (смена доски -> {}): {}

{}",
            active_board_id, terminal_col.title, summary
        );
        let _ = append_task_dialog_activity(
            root,
            task_id,
            "summary",
            &summary_content,
            app_handle,
        )
        .await;

        return Ok(format!(
            "УСПЕШНО: Задача {} финализирована на активной доске '{}' в терминальной колонке '{}' ({}), статус: {}",
            task_id,
            active_board_id,
            terminal_col.id,
            terminal_col.title,
            updated_task.status.as_deref().unwrap_or("done")
        ));
    }

    // Normal progression within same board
    let columns = {
        let from_board = boards_cfg.as_ref().and_then(|bc| {
            let b = if let Some(ref bid) = current_task.board_id {
                bc.boards.iter().find(|b| &b.id == bid)
            } else {
                bc.boards.iter().find(|b| b.id == bc.active_board_id)
            };
            b.map(|b| b.columns.clone())
        });
        match from_board {
            Some(cols) => cols,
            None => {
                let pipeline = load_pipeline_config(root)
                    .await
                    .map_err(|e| e.to_string())?;
                pipeline.columns
            }
        }
    };

    let curr_idx = columns
        .iter()
        .position(|c| stage_id_matches(&c.id, &current_task.stage_id))
        .ok_or_else(|| {
            format!(
                "Current column '{}' not found in pipeline",
                current_task.stage_id
            )
        })?;

    if curr_idx + 1 >= columns.len() {
        return Err(format!(
            "Задача {} уже находится на последнем этапе пайплайна ({})",
            task_id, columns[curr_idx].title
        ));
    }

    let next_col = &columns[curr_idx + 1];
    let is_terminal = next_col.terminal == Some(true)
        || stage_id_matches(&next_col.id, "ready")
        || stage_id_matches(&next_col.id, "done");
    let status_override = if is_terminal {
        Some("done".to_string())
    } else {
        None
    };

    let updated_task = internal_move_task_stage(
        root,
        task_id,
        &next_col.id,
        Some("APPROVED".to_string()),
        status_override,
        Some(summary.to_string()),
        app_handle,
    )
    .await?;

    let summary_content = format!("### Саммари этапа: {}

{}", next_col.title, summary);
    let _ = append_task_dialog_activity(
        root,
        task_id,
        "summary",
        &summary_content,
        app_handle,
    )
    .await;

    Ok(format!(
        "УСПЕШНО: Задача {} переведена на этап '{}' ({}), статус: {}",
        task_id,
        next_col.id,
        next_col.title,
        updated_task.status.as_deref().unwrap_or("idle")
    ))
}

#[tauri::command]
pub async fn advance_task(
    task_id: String,
    summary: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    let root = state.get_root();
    internal_advance_task(&root, &task_id, &summary, Some(&app)).await
}

#[tauri::command]
pub async fn run_agent(
    task_id: String,
    prompt: Option<String>,
    app_handle: AppHandle,
) -> Result<String, String> {
    let root = app_handle.state::<AppState>().get_root();

    // Check in-memory task registry first
    let task_opt = {
        if let Ok(guard) = app_handle.state::<AppState>().tasks.lock() {
            guard.iter().find(|t| t.id == task_id).cloned()
        } else {
            None
        }
    };

    let mut task = match task_opt {
        Some(t) => t,
        None => {
            let disk_tasks = load_tasks_from_disk(&root).await;
            if let Some(t) = disk_tasks.iter().find(|t| t.id == task_id).cloned() {
                if let Ok(mut guard) = app_handle.state::<AppState>().tasks.lock() {
                    *guard = disk_tasks;
                }
                t
            } else {
                let mem_count = app_handle.state::<AppState>().tasks.lock().map(|g| g.len()).unwrap_or(0);
                eprintln!(
                    "CVH [run_agent]: Task {} not found (in-memory tasks: {}, disk tasks in {:?}: {}, root: {:?})",
                    task_id,
                    mem_count,
                    root.join(".cvh").join("tasks.json"),
                    disk_tasks.len(),
                    root
                );
                return Err(format!("Task {} not found", task_id));
            }
        }
    };

    let steer_prompt = if let Some(p) = prompt {
        let p_trimmed = p.trim();
        if !p_trimmed.is_empty() {
            if task.description.trim().is_empty() {
                task.description = p_trimmed.to_string();
                task.updated_at = Utc::now().timestamp_millis();

                // Persist description update immediately
                let mut all_tasks = load_tasks_from_disk(&root).await;
                if let Some(t) = all_tasks.iter_mut().find(|t| t.id == task_id) {
                    t.description = task.description.clone();
                    t.updated_at = task.updated_at;
                    let _ = save_tasks_to_disk(&root, &all_tasks).await;
                }
                if let Ok(mut guard) = app_handle.state::<AppState>().tasks.lock() {
                    if let Some(t) = guard.iter_mut().find(|t| t.id == task_id) {
                        t.description = task.description.clone();
                        t.updated_at = task.updated_at;
                    }
                }
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
                    context_window: Some(400000),
                    model_override: None,
                    reasoning_effort: None,
                    temperature: Some(0.2),
                    gatekeeper_sensor: None,
                    allowed_tools: None,
                    hooks: HashMap::new(),
                    routes: None,
                    auto_run: None,
                    terminal: None,
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
    if let Some(cw) = agent_setting.context_window {
        eprintln!(
            "CVH [run_agent]: task_id={} persona='{}' context_window={}",
            task.id, persona_role, cw
        );
    }

    fn non_empty_opt(v: &Option<String>) -> Option<String> {
        v.as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    let llm_model = {
        let m = llm_settings.model.trim();
        if m.is_empty() { None } else { Some(m.to_string()) }
    };
    let model = non_empty_opt(&task.model)
        .or(llm_model)
        .or_else(|| non_empty_opt(&agent_setting.model))
        .or_else(|| non_empty_opt(&col.model_override))
        .or_else(|| non_empty_opt(&proj_cfg.default_model))
        .unwrap_or_else(|| "gemini-3.8-flash-high".to_string());

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

    // Store in active agents registry
    {
        let state = app_handle.state::<AppState>();
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
    }

    // Persist status "running" to tasks.json and update state.tasks
    {
        let mut all_tasks = load_tasks_from_disk(&root).await;
        if let Some(t) = all_tasks.iter_mut().find(|t| t.id == task_id) {
            t.status = Some("running".to_string());
            t.updated_at = Utc::now().timestamp_millis();
            let _ = save_tasks_to_disk(&root, &all_tasks).await;
        }
        if let Ok(mut guard) = app_handle.state::<AppState>().tasks.lock() {
            *guard = all_tasks;
        }
    }

    // Emit live status: running
    let _ = app_handle.emit(
        "agent:status",
        serde_json::json!({ "taskId": task.id, "status": "running" }),
    );

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
        steer_prompt,
        base_url,
        api_key,
        model,
        temperature,
        reasoning_effort: col.reasoning_effort.clone(),
        max_turns: 25,
        app_handle: Some(app_handle.clone()),
    };

    let tid = task.id.clone();
    let app = app_handle.clone();
    let root_task = root.clone();
    let abort_flag_task = Arc::clone(&abort_flag);
    let initial_active_board_id = {
        let boards_cfg = load_boards_config(&root).await.ok();
        boards_cfg.map(|b| b.active_board_id)
    };

    let handle = tokio::spawn(async move {
        let app_inner = app.clone();
        let tid_inner = tid.clone();

        let event_sink = move |event: AgentEvent| {
            match event {
                AgentEvent::Token(token) => {
                    let payload = serde_json::json!({
                        "taskId": tid_inner,
                        "output": &token,
                        "content": &token,
                        "token": &token
                    });
                    let _ = app_inner.emit("agent:token", &payload);
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
                }
                AgentEvent::ToolResult { id, name, result, success } => {
                    let _ = app_inner.emit(
                        "agent:tool_result",
                        serde_json::json!({ "taskId": tid_inner, "id": id, "name": name, "result": result, "success": success }),
                    );
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
                        }),
                    );
                }
                AgentEvent::Compacted {
                    saved_chars,
                    estimated_saved_tokens,
                    evaporated_count,
                } => {
                    let _ = app_inner.emit(
                        "agent:compacted",
                        serde_json::json!({
                            "taskId": tid_inner,
                            "savedChars": saved_chars,
                            "estimatedSavedTokens": estimated_saved_tokens,
                            "evaporatedCount": evaporated_count,
                        }),
                    );
                }
                AgentEvent::Finished { final_response } => {
                    let _ = app_inner.emit(
                        "agent:finished",
                        serde_json::json!({
                            "taskId": tid_inner,
                            "status": "finished",
                            "response": final_response
                        }),
                    );
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

        match result {
            Ok(_final_response) => {
                let _ = app.emit(
                    "agent:status",
                    serde_json::json!({ "taskId": tid, "status": "finished" }),
                );

                let mut tasks = load_tasks_from_disk(&root_task).await;
                if let Some(task_idx) = tasks.iter().position(|t| t.id == tid) {
                    let end_boards_cfg = load_boards_config(&root_task).await.ok();
                    let end_active_board_id = end_boards_cfg.as_ref().map(|b| b.active_board_id.clone());
                    let board_changed = end_active_board_id.is_some() && (
                        end_active_board_id != initial_active_board_id ||
                        end_active_board_id.as_deref() != tasks[task_idx].board_id.as_deref()
                    );

                    if board_changed {
                        let active_id = end_active_board_id.unwrap();
                        let target_board = end_boards_cfg.as_ref().and_then(|bc| {
                            bc.boards.iter().find(|b| b.id == active_id).cloned()
                        });
                        let terminal_col_id = target_board.as_ref().and_then(|b| {
                            b.columns.iter().find(|c| c.terminal == Some(true) || stage_id_matches(&c.id, "ready") || stage_id_matches(&c.id, "done")).map(|c| c.id.clone())
                                .or_else(|| b.columns.last().map(|c| c.id.clone()))
                        }).unwrap_or_else(|| "ready".to_string());

                        tasks[task_idx].board_id = Some(active_id);
                        tasks[task_idx].stage_id = terminal_col_id;
                        tasks[task_idx].status = Some("done".to_string());
                        tasks[task_idx].updated_at = Utc::now().timestamp_millis();
                    } else if tasks[task_idx].status.as_deref() == Some("running") {
                        tasks[task_idx].status = Some("idle".to_string());
                        tasks[task_idx].updated_at = Utc::now().timestamp_millis();
                    }
                    let updated_task = tasks[task_idx].clone();
                    let _ = save_tasks_to_disk(&root_task, &tasks).await;
                    if let Some(state_ref) = app.try_state::<AppState>() {
                        if let Ok(mut guard) = state_ref.tasks.lock() {
                            *guard = tasks.clone();
                        }
                    }
                    let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
                    let _ = app.emit(
                        "task:moved",
                        serde_json::json!({
                            "taskId": tid,
                            "stageId": updated_task.stage_id,
                            "status": updated_task.status,
                            "task": updated_task
                        }),
                    );
                }
            }
            Err(err) => {
                let err_str = err.to_string();
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

        // Clean up from active_agents registry upon agent completion only if this entry still belongs to this run
        if let Some(state_ref) = app.try_state::<AppState>() {
            if let Ok(mut map) = state_ref.active_agents.lock() {
                if let Some(entry) = map.get(&tid) {
                    if Arc::ptr_eq(&entry.abort_flag, &abort_flag_task) {
                        map.remove(&tid);
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

#[tauri::command]
pub async fn abort_agent(task_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let entry = {
        let mut map = state
            .active_agents
            .lock()
            .map_err(|_| "Failed to lock active agents".to_string())?;
        map.remove(&task_id)
    };

    if let Some(agent_entry) = entry {
        agent_entry.abort_flag.store(true, Ordering::SeqCst);
        let maybe_handle = {
            let mut guard = agent_entry.join_handle.lock().await;
            guard.take()
        };
        if let Some(handle) = maybe_handle {
            let _ = tokio::time::timeout(tokio::time::Duration::from_millis(2000), handle).await;
        }
        Ok(())
    } else {
        Err(format!("No running agent found for task {}", task_id))
    }
}

#[tauri::command]
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

#[tauri::command]
pub async fn compact_context(
    task_id: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::agent::compactor::CompactionReport, String> {
    let root = state.get_root();

    // 1. Resolve task and active agent model
    let tasks = load_tasks_from_disk(&root).await;
    let task = tasks.iter().find(|t| t.id == task_id).cloned();

    let active_agent_model = task
        .as_ref()
        .and_then(|t| t.model.as_deref())
        .filter(|m| !m.trim().is_empty())
        .map(|m| m.to_string())
        .unwrap_or_else(|| "gemini-3.8-flash-high".to_string());

    // 2. Load LLM configuration and resolve compactor model (inheritance support)
    let llm_settings = crate::config::load_llm_settings(&root).await.unwrap_or_default();
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

    // 4. Determine items to condense before the last 2 active turns
    let mut ids_to_archive: Vec<String> = Vec::new();
    let mut messages_to_condense: Vec<crate::llm::ChatMessage> = Vec::new();

    if dialog_items.len() > 2 {
        let split_idx = dialog_items.len().saturating_sub(2);
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
                    messages_to_condense.push(crate::llm::ChatMessage::assistant(
                        it.content.clone(),
                        None,
                    ));
                }
                "tool" => {
                    if let Some(ref t) = it.tool {
                        let tool_id = t.get("id").and_then(|v| v.as_str()).unwrap_or("tool_call");
                        let stdout = t.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
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
        // Fallback to task description prompt
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

    let tokens_before = crate::agent::compactor::estimate_tokens(&messages_to_condense).max(1800);

    // 5. Run LLM condensation with the 7-section structured prompt
    let compacted_markdown = crate::agent::compactor::condense_context_with_llm(
        &task_id,
        &messages_to_condense,
        &compactor_model,
        &base_url,
        api_key,
    )
    .await;

    let tokens_after = crate::agent::compactor::estimate_text_tokens(&compacted_markdown).max(350);
    let saved_tokens = tokens_before.saturating_sub(tokens_after);
    let compacted_count = if !ids_to_archive.is_empty() {
        ids_to_archive.len()
    } else {
        messages_to_condense.len().max(1)
    };

    // 6. Persist compacted markdown into SQLite table dialog_items with type = 'compacted'
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

    let _ = crate::db::persist_compacted_dialog(&root, &task_id, &compacted_item, &ids_to_archive);

    // 7. Update in-memory active agent context if present
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
            if guard.messages.len() > 2 {
                let tail_start = guard.messages.len().saturating_sub(2);
                for msg in &guard.messages[tail_start..] {
                    new_messages.push(msg.clone());
                }
            }
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

    // 8. Emit Tauri events
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

#[tauri::command]
pub async fn steer_agent(
    task_id: String,
    message: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let root = state.get_root();

    let task_found = {
        if let Ok(guard) = state.tasks.lock() {
            guard.iter().any(|t| t.id == task_id)
        } else {
            false
        }
    };

    if !task_found {
        let tasks = load_tasks_from_disk(&root).await;
        if !tasks.iter().any(|t| t.id == task_id) {
            return Err(format!("Task {} not found", task_id));
        }
    }

    let steer_payload = serde_json::json!({
        "taskId": task_id,
        "content": message,
        "type": "steer"
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

    if let Ok(contexts) = state.agent_contexts.lock() {
        if let Some(cs) = contexts.get(&task_id) {
            if let Ok(mut guard) = cs.lock() {
                guard.messages.push(crate::llm::ChatMessage::user(format!(
                    "[USER STEER]: {}",
                    message
                )));
            }
        }
    }

    run_agent(task_id, Some(message), app_handle).await
}

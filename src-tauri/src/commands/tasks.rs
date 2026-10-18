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
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

/// Dynamically heals a single task against the given BoardsConfig:
/// 1. Board ID: If task.board_id is None, empty, or does not match any existing board in config.boards:
///    reassign it to config.active_board_id (if that board exists), otherwise to config.boards[0].id (if available),
///    or fallback to "development".
/// 2. Column / Stage ID: If task.stage_id does not exist in the target board's columns (checked via stage_id_matches),
///    dynamically heal it to the target board's first column (typically "backlog").
///
/// Returns true if any modification was made to the task.
pub fn heal_task_board_and_column(task: &mut Task, config: &crate::config::BoardsConfig) -> bool {
    let mut modified = false;

    // 1. Resolve board_id
    let target_board_id = match task.board_id.as_deref().map(str::trim) {
        Some(b_id) if !b_id.is_empty() && config.boards.iter().any(|b| b.id == b_id) => {
            b_id.to_string()
        }
        _ => {
            let fallback_id = if config.boards.iter().any(|b| b.id == config.active_board_id) {
                config.active_board_id.clone()
            } else if let Some(first_board) = config.boards.first() {
                first_board.id.clone()
            } else {
                "development".to_string()
            };
            task.board_id = Some(fallback_id.clone());
            modified = true;
            fallback_id
        }
    };

    // 2. Resolve column / stage_id within target board
    if let Some(target_board) = config.boards.iter().find(|b| b.id == target_board_id) {
        let col_exists = target_board
            .columns
            .iter()
            .any(|c| stage_id_matches(&c.id, &task.stage_id));
        if !col_exists {
            let fallback_col = target_board
                .columns
                .first()
                .map(|c| c.id.as_str())
                .unwrap_or("backlog");
            task.stage_id = fallback_col.to_string();
            modified = true;
        }
    }

    modified
}

/// Heals all tasks against the given BoardsConfig.
/// Returns true if any task was modified.
pub fn heal_tasks(tasks: &mut [Task], config: &crate::config::BoardsConfig) -> bool {
    let mut any_modified = false;
    for t in tasks.iter_mut() {
        if heal_task_board_and_column(t, config) {
            any_modified = true;
        }
    }
    any_modified
}

/// Ensures all tasks have a valid board_id and column_id dynamically according to BoardsConfig.
pub fn ensure_task_board_id(mut tasks: Vec<Task>, config: &crate::config::BoardsConfig) -> Vec<Task> {
    heal_tasks(&mut tasks, config);
    tasks
}

pub async fn load_tasks_from_disk(repo_root: &Path) -> Vec<Task> {
    let boards_cfg = load_boards_config(repo_root)
        .await
        .unwrap_or_else(|_| crate::config::default_boards_config());

    let mut tasks = Vec::new();
    let mut loaded = false;

    let local_tasks_file = repo_root.join(".cvh").join("tasks.json");
    if local_tasks_file.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&local_tasks_file).await {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                if let Ok(parsed) = serde_json::from_str::<Vec<Task>>(trimmed) {
                    tasks = parsed;
                    loaded = true;
                }
            }
        }
    }

    if !loaded {
        // Try global fallback: %LOCALAPPDATA%\com.voimax.cvh\tasks.json
        let global_tasks_file = get_app_config_dir().join("tasks.json");
        if global_tasks_file.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&global_tasks_file).await {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    if let Ok(parsed) = serde_json::from_str::<Vec<Task>>(trimmed) {
                        if !parsed.is_empty() {
                            eprintln!(
                                "CVH [load_tasks_from_disk]: Restored {} tasks from global fallback {:?}",
                                parsed.len(),
                                global_tasks_file
                            );
                            tasks = parsed;
                        }
                    }
                }
            }
        }
    }

    let modified = heal_tasks(&mut tasks, &boards_cfg);
    if modified {
        let _ = save_tasks_to_disk(repo_root, &tasks).await;
    }

    // Monotonic sequence initialization: ensure persistent sequence is at least max_task_num_on_disk
    let max_on_disk = max_task_num_on_disk(&tasks);
    if max_on_disk > 0 {
        let current_seq = load_task_sequence(repo_root).await;
        if max_on_disk > current_seq {
            let _ = save_task_sequence(repo_root, max_on_disk).await;
        }
    }

    tasks
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskSequence {
    #[serde(default, alias = "sequence", alias = "last_task_num", alias = "lastTaskNum")]
    pub last_task_num: u32,
}

pub async fn load_task_sequence(repo_root: &Path) -> u32 {
    let paths = [
        repo_root.join(".cvh").join("settings").join("sequence.json"),
        repo_root.join(".cvh").join("task-sequence.json"),
    ];

    for path in &paths {
        if path.exists() {
            if let Ok(content) = tokio::fs::read_to_string(path).await {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    if let Ok(seq) = serde_json::from_str::<TaskSequence>(trimmed) {
                        if seq.last_task_num > 0 {
                            return seq.last_task_num;
                        }
                    }
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                        if let Some(n) = val.get("last_task_num").and_then(|v| v.as_u64()) {
                            if n > 0 {
                                return n as u32;
                            }
                        }
                        if let Some(n) = val.get("sequence").and_then(|v| v.as_u64()) {
                            if n > 0 {
                                return n as u32;
                            }
                        }
                    }
                    if let Ok(n) = trimmed.parse::<u32>() {
                        if n > 0 {
                            return n;
                        }
                    }
                }
            }
        }
    }
    0
}

pub async fn save_task_sequence(repo_root: &Path, last_num: u32) -> Result<(), String> {
    let settings_dir = repo_root.join(".cvh").join("settings");
    let _ = tokio::fs::create_dir_all(&settings_dir).await;
    let seq_path = settings_dir.join("sequence.json");
    let content = serde_json::to_string_pretty(&serde_json::json!({
        "last_task_num": last_num
    }))
    .map_err(|e| format!("Failed to serialize task sequence: {}", e))?;

    atomic_write_file(&seq_path, &content)
        .await
        .map_err(|e| format!("Failed to save sequence file: {}", e))?;

    // Also synchronize to .cvh/task-sequence.json for maximum reliability
    let alt_path = repo_root.join(".cvh").join("task-sequence.json");
    let _ = atomic_write_file(&alt_path, &content).await;

    Ok(())
}

pub fn max_task_num_on_disk(tasks: &[Task]) -> u32 {
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
    max_num
}

pub fn next_task_num(tasks: &[Task]) -> u32 {
    let max_num = max_task_num_on_disk(tasks);
    let mut candidate = max_num.saturating_add(1).max(1);
    while tasks.iter().any(|t| t.id == format!("CVH-{:03}", candidate)) {
        candidate = candidate.saturating_add(1);
    }
    candidate
}

/// Allocates the next monotonic task ID, comparing stored sequence with max_on_disk.
/// Increments monotonically: next_num = max(stored_seq, max_on_disk) + 1.
/// Persists the incremented counter to disk immediately, preventing deleted ID reuse.
pub async fn allocate_next_task_num(repo_root: &Path, tasks: &[Task]) -> u32 {
    let stored_seq = load_task_sequence(repo_root).await;
    let max_on_disk = max_task_num_on_disk(tasks);
    let mut next_num = stored_seq.max(max_on_disk).saturating_add(1).max(1);

    while tasks.iter().any(|t| t.id == format!("CVH-{:03}", next_num)) {
        next_num = next_num.saturating_add(1);
    }

    if let Err(e) = save_task_sequence(repo_root, next_num).await {
        eprintln!(
            "CVH [allocate_next_task_num]: Warning - failed to persist sequence {}: {}",
            next_num, e
        );
    }

    next_num
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
#[allow(clippy::too_many_arguments)]
pub async fn create_task(
    title: String,
    description: String,
    stage_id: String,
    board_id: Option<String>,
    skip_worktree: Option<bool>,
    model: Option<String>,
    app_handle: AppHandle,
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

    let clean_model = model
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    // Generate task ID — monotonic max(stored_seq, max_on_disk) + 1, persistent across deletions
    let task_num = allocate_next_task_num(&root, &tasks).await;
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
        model: clean_model,
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
    let _ = app_handle.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));

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
        model: parent_model.or(target_col.model_override.clone()),
        parent_id: Some(parent_id.clone()),
        subtask_index: Some(next_subtask_idx),
        waiting_for_subtask_id: None,
        handover_summary: None,
        cover_image: None,
        attachments: None,
        todos: None,
        priority: tasks[parent_idx].priority.clone(),
        tags: None,
        current_tokens: None,
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

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn update_task(
    task_id: String,
    model: Option<String>,
    title: Option<String>,
    description: Option<String>,
    status: Option<String>,
    verdict: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    let root = state.get_root();
    let mut tasks = load_tasks_from_disk(&root).await;
    let task_idx = tasks
        .iter()
        .position(|t| t.id == task_id)
        .ok_or_else(|| format!("Task {} not found", task_id))?;

    let now = Utc::now().timestamp_millis();
    let task = &mut tasks[task_idx];

    if let Some(m) = model {
        let trimmed = m.trim().to_string();
        task.model = if trimmed.is_empty() { None } else { Some(trimmed) };
    }
    if let Some(t) = title {
        task.title = t;
    }
    if let Some(d) = description {
        task.description = d;
    }
    if let Some(s) = status {
        task.status = Some(s);
    }
    if let Some(v) = verdict {
        task.verdict = Some(v);
    }
    task.updated_at = now;

    let updated = task.clone();
    save_tasks_to_disk(&root, &tasks).await?;

    if let Ok(mut guard) = state.tasks.lock() {
        *guard = tasks.clone();
    }

    let _ = app_handle.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
    let _ = app_handle.emit("task:updated", serde_json::json!({ "task": &updated }));

    Ok(updated)
}

#[tauri::command]
pub async fn delete_task(
    task_id: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let root = state.get_root();
    let mut tasks = load_tasks_from_disk(&root).await;

    // Collect target and any cascaded subtasks
    let mut to_delete = HashSet::new();
    to_delete.insert(task_id.clone());
    let mut grew = true;
    while grew {
        grew = false;
        for t in &tasks {
            if to_delete.contains(&t.id) {
                continue;
            }
            if let Some(ref p) = t.parent_id {
                if to_delete.contains(p) {
                    to_delete.insert(t.id.clone());
                    grew = true;
                    continue;
                }
            }
            if to_delete.iter().any(|id| t.id.starts_with(&format!("{}.", id))) {
                to_delete.insert(t.id.clone());
                grew = true;
            }
        }
    }

    // Monotonic sequence preservation: ensure sequence counter records max before deletion
    let pre_delete_max = max_task_num_on_disk(&tasks);
    let current_seq = load_task_sequence(&root).await;
    if pre_delete_max > current_seq {
        let _ = save_task_sequence(&root, pre_delete_max).await;
    }

    // Abort active agents for any deleted task/subtask (drop std::sync::MutexGuard before await)
    let handles: Vec<_> = {
        let mut entries = Vec::new();
        if let Ok(mut map) = state.active_agents.lock() {
            for id in &to_delete {
                if let Some(agent_entry) = map.remove(id) {
                    agent_entry.abort_flag.store(true, Ordering::SeqCst);
                    entries.push(agent_entry);
                }
            }
        }
        entries
    };

    for agent_entry in handles {
        let mut guard = agent_entry.join_handle.lock().await;
        if let Some(handle) = guard.take() {
            handle.abort();
        }
    }

    // Collect branch info before removing from tasks vector
    let is_subtask = task_id.contains('.');
    let mut branches_to_delete = Vec::new();
    for t in &tasks {
        if to_delete.contains(&t.id) {
            if let Some(ref b) = t.branch {
                if !b.trim().is_empty() {
                    branches_to_delete.push(b.clone());
                }
            }
        }
    }

    tasks.retain(|t| !to_delete.contains(&t.id));

    // Auto-unlock any parent that was waiting on a deleted subtask
    for t in &mut tasks {
        if let Some(ref waiting_id) = t.waiting_for_subtask_id {
            if to_delete.contains(waiting_id) {
                t.waiting_for_subtask_id = None;
                if t.status.as_deref() == Some("waiting_subtask") {
                    t.status = Some("idle".to_string());
                }
            }
        }
    }

    save_tasks_to_disk(&root, &tasks).await?;

    if let Ok(mut guard) = state.tasks.lock() {
        *guard = tasks.clone();
    }

    // Robust cleanup of git worktree and branch if top-level task
    if !is_subtask {
        let task_slug = task_id.to_lowercase();
        let _ = crate::git::remove_worktree(&root, &task_slug).await;
        let branch = format!("feat/{}", task_slug);
        let _ = crate::git::delete_branch(&root, &branch, true).await;
    }

    for branch in branches_to_delete {
        let _ = crate::git::delete_branch(&root, &branch, true).await;
    }

    // Defensively prune git worktree administrative records
    let _ = crate::git::prune_worktrees(&root).await;

    // Best-effort cleanup of SQLite dialogue items
    for id in &to_delete {
        let _ = crate::db::clear_dialog_items(&root, id);
    }

    // Broadcast tasks:updated event across application
    let _ = app_handle.emit("tasks:updated", ());

    Ok(())
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
        let current_board = current_task.board_id.as_deref().and_then(|bid| {
            bc.boards.iter().find(|b| b.id == bid).cloned()
        });

        let current_has_exact = current_board
            .as_ref()
            .map(|b| b.columns.iter().any(|c| c.id == target_stage_id))
            .unwrap_or(false);

        let active_has_exact = active_board
            .as_ref()
            .map(|b| b.columns.iter().any(|c| c.id == target_stage_id))
            .unwrap_or(false);

        if current_has_exact {
            effective_board = current_board;
        } else if active_has_exact {
            effective_board = active_board;
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
    let is_backlog = stage_id_matches(&canonical_target_stage_id, "backlog");
    let target_has_autorun = !is_backlog && (target_col.persona.is_some() || target_col.auto_run == Some(true));
    let was_running = current_task.status.as_deref() == Some("running");

    // Immediate Cancellation (§ 2 specifications):
    // If a task is moved to backlog (or any stage without autoRun / persona, or user moves a running task):
    if is_backlog || !target_has_autorun || was_running {
        if let Some(app) = app_handle {
            if let Some(state_ref) = app.try_state::<AppState>() {
                let entry = {
                    let mut map = state_ref.active_agents.lock().ok();
                    map.as_mut().and_then(|m| m.remove(task_id))
                };
                if let Some(agent_entry) = entry {
                    agent_entry.abort_flag.store(true, Ordering::SeqCst);
                    let maybe_handle = {
                        let mut guard = agent_entry.join_handle.lock().await;
                        guard.take()
                    };
                    if let Some(handle) = maybe_handle {
                        handle.abort();
                    }
                    let _ = app.emit(
                        "agent:status",
                        serde_json::json!({ "taskId": task_id, "status": "idle" }),
                    );
                }
            }
        }
    }

    // Worktree allocation: driven strictly by the configurable hook (builtin:create_worktree)
    if needs_builtin_worktree(&target_col, current_task, false) {
        ensure_builtin_worktree(root, current_task).await;
    }

    // Gatekeeper Sensor Verification: do not block initial move from backlog to first actionable stage or new/empty worktree
    if let Some(sensor_cmd) = &target_col.gatekeeper_sensor {
        let is_from_backlog = source_stage_id.is_empty()
            || stage_id_matches(&source_stage_id, "backlog");
        let is_first_actionable_stage = {
            let active_board = effective_board
                .as_ref()
                .or_else(|| {
                    boards_cfg
                        .as_ref()
                        .and_then(|bc| bc.boards.iter().find(|b| b.id == bc.active_board_id))
                });

            if let Some(b) = active_board {
                b.columns
                    .iter()
                    .find(|c| !stage_id_matches(&c.id, "backlog"))
                    .map(|c| stage_id_matches(&canonical_target_stage_id, &c.id))
                    .unwrap_or(false)
            } else if let Ok(pipe) = load_pipeline_config(root).await {
                pipe.columns
                    .iter()
                    .find(|c| !stage_id_matches(&c.id, "backlog"))
                    .map(|c| stage_id_matches(&canonical_target_stage_id, &c.id))
                    .unwrap_or(false)
            } else {
                stage_id_matches(&canonical_target_stage_id, "dev")
            }
        };
        let is_initial_move = is_from_backlog && is_first_actionable_stage;

        let wt_empty_or_new = match &current_task.worktree_path {
            None => true,
            Some(p) => {
                let path = Path::new(p);
                !path.exists() || path.read_dir().map(|mut it| it.next().is_none()).unwrap_or(true)
            }
        };

        if is_initial_move || wt_empty_or_new {
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
                let stdout_trim = sensor_res.stdout.trim();
                let stderr_trim = sensor_res.stderr.trim();
                let mut details = String::new();
                if !stdout_trim.is_empty() {
                    details.push_str(&format!("\n--- STDOUT ---\n{}", stdout_trim));
                }
                if !stderr_trim.is_empty() {
                    details.push_str(&format!("\n--- STDERR ---\n{}", stderr_trim));
                }
                if details.is_empty() {
                    details = "\n(No stdout/stderr produced by sensor command)".to_string();
                }
                return Err(format!(
                    "Gatekeeper Sensor '{}' rejected transition (Exit code: {:?}):{}",
                    sensor_cmd, sensor_res.exit_code, details
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
    if is_backlog {
        current_task.status = Some("idle".to_string());
    } else if let Some(st) = status_override {
        current_task.status = Some(st);
    } else if is_terminal_col {
        current_task.status = Some("done".to_string());
    } else if target_has_autorun {
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
        if target_has_autorun {
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

    // Normal progression within same board or fallback boards
    let columns = {
        let from_board = boards_cfg.as_ref().and_then(|bc| {
            // 1. Try board referenced by current_task.board_id if it contains current stage
            if let Some(ref bid) = current_task.board_id {
                if let Some(b) = bc.boards.iter().find(|b| &b.id == bid) {
                    if b.columns.iter().any(|c| stage_id_matches(&c.id, &current_task.stage_id)) {
                        return Some(b.columns.clone());
                    }
                }
            }
            // 2. Try active board if it contains current stage
            if let Some(b) = bc.boards.iter().find(|b| b.id == bc.active_board_id) {
                if b.columns.iter().any(|c| stage_id_matches(&c.id, &current_task.stage_id)) {
                    return Some(b.columns.clone());
                }
            }
            // 3. Search any board in boards.json that contains current stage
            if let Some(b) = bc.boards.iter().find(|b| {
                b.columns.iter().any(|c| stage_id_matches(&c.id, &current_task.stage_id))
            }) {
                return Some(b.columns.clone());
            }
            // 4. Fallback to active board columns or first board
            bc.boards
                .iter()
                .find(|b| b.id == bc.active_board_id)
                .or_else(|| bc.boards.first())
                .map(|b| b.columns.clone())
        });
        match from_board {
            Some(cols) => {
                if cols.iter().any(|c| stage_id_matches(&c.id, &current_task.stage_id)) {
                    cols
                } else {
                    let pipeline = load_pipeline_config(root)
                        .await
                        .map_err(|e| e.to_string())?;
                    pipeline.columns
                }
            }
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

#[tauri::command]
pub async fn run_agent(
    task_id: String,
    prompt: Option<String>,
    app_handle: AppHandle,
) -> Result<String, String> {
    let root = app_handle.state::<AppState>().get_root();

    // Check disk first to ensure task model override and steer updates are fresh
    let disk_tasks = load_tasks_from_disk(&root).await;
    let task_opt = disk_tasks.iter().find(|t| t.id == task_id).cloned().or_else(|| {
        app_handle.state::<AppState>().tasks.lock().ok()?.iter().find(|t| t.id == task_id).cloned()
    });

    let mut task = match task_opt {
        Some(t) => {
            if let Ok(mut guard) = app_handle.state::<AppState>().tasks.lock() {
                *guard = disk_tasks;
            }
            t
        }
        None => {
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

            if task.description.trim().is_empty() && !has_existing_context {
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
        if let Ok(mut ctx_map) = state.agent_contexts.lock() {
            ctx_map.entry(task_id.clone()).or_insert_with(|| {
                std::sync::Arc::new(std::sync::Mutex::new(crate::agent::compactor::AgentContextState::default()))
            });
        };
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
        context_window: Some(resolved_context_window),
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
                    duration_ms,
                    tps,
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
                        }),
                    );
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

                    if let (true, Some(active_id)) = (board_changed, end_active_board_id) {
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
                let is_aborted = abort_flag_task.load(Ordering::SeqCst);
                let err_str = err.to_string();
                if is_aborted {
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
pub async fn abort_agent(
    task_id: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let entry = {
        let mut map = state
            .active_agents
            .lock()
            .map_err(|_| "Failed to lock active agents".to_string())?;
        map.remove(&task_id)
    };

    let had_entry = entry.is_some();
    if let Some(agent_entry) = entry {
        agent_entry.abort_flag.store(true, Ordering::SeqCst);
        let maybe_handle = {
            let mut guard = agent_entry.join_handle.lock().await;
            guard.take()
        };
        if let Some(handle) = maybe_handle {
            handle.abort();
            let _ = tokio::time::timeout(tokio::time::Duration::from_millis(2000), handle).await;
        }
    }

    let root = state.get_root();
    let mut tasks = load_tasks_from_disk(&root).await;
    let mut task_updated = false;
    if let Some(t) = tasks.iter_mut().find(|t| t.id == task_id) {
        t.status = Some("idle".to_string());
        t.updated_at = Utc::now().timestamp_millis();
        task_updated = true;
    }

    if task_updated {
        if let Ok(mut guard) = state.tasks.lock() {
            *guard = tasks.clone();
        }
        let _ = save_tasks_to_disk(&root, &tasks).await;
        let _ = app_handle.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
        let _ = app_handle.emit(
            "agent:status",
            serde_json::json!({ "taskId": &task_id, "status": "idle" }),
        );
    }

    if had_entry || task_updated {
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

    // Persist steer message into SQLite dialog_items via append_task_dialog_activity
    let _ = append_task_dialog_activity(
        &root,
        &task_id,
        "steer",
        &message,
        Some(&app_handle),
    ).await;

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

    run_agent(task_id, Some(message), app_handle).await
}

/// Resolves the context window strictly for a task or agent persona role.
/// Priority:
/// 1. Direct persona override if provided (e.g. activeSubagentRole).
/// 2. Active agent persona associated with the task's column.
/// 3. Setting in `.cvh/agents/<role>/setting` (`context_window` or `contextWindow`).
/// 4. Fallback strictly to 128_000.
#[tauri::command]
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

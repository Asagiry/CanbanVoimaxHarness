use crate::agent::{run_agent_loop, AgentEvent, AgentSessionConfig, Task};
use crate::commands::dialogs::append_task_dialog_activity;
use crate::commands::git::apply_worktree_from_hook_stdout;
use crate::commands::state::{ActiveAgentHandle, AppState};
use crate::config::{
    atomic_write_file, get_app_config_dir, load_boards_config, load_llm_settings,
    load_pipeline_config, load_project_config, resolve_actual_repo_root, ProjectConfig,
};
use crate::hooks::{execute_hook, execute_sensor_command, HookContext};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
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

/// Heals all tasks against the given BoardsConfig and enforces relational subtask invariants.
/// Returns true if any task was modified.
pub fn heal_tasks(tasks: &mut [Task], config: &crate::config::BoardsConfig) -> bool {
    let mut any_modified = false;
    for t in tasks.iter_mut() {
        if heal_task_board_and_column(t, config) {
            any_modified = true;
        }
    }

    // Relational subtask and terminal status self-healing (§ 2.2, § 2.7 & § 3):
    let active_task_ids: HashSet<String> = tasks
        .iter()
        .filter(|t| t.status.as_deref() != Some("done"))
        .map(|t| t.id.clone())
        .collect();

    for t in tasks.iter_mut() {
        let is_terminal = stage_id_matches(&t.stage_id, "ready")
            || stage_id_matches(&t.stage_id, "done")
            || t.status.as_deref() == Some("done")
            || config.boards.iter().any(|b| {
                b.columns.iter().any(|c| stage_id_matches(&c.id, &t.stage_id) && c.terminal == Some(true))
            });

        if is_terminal {
            if t.waiting_for_subtask_id.is_some() {
                t.waiting_for_subtask_id = None;
                any_modified = true;
            }
            if t.status.as_deref() == Some("waiting_subtask") {
                t.status = Some("done".to_string());
                any_modified = true;
            }
        } else if let Some(ref sub_id) = t.waiting_for_subtask_id {
            if !active_task_ids.contains(sub_id) {
                t.waiting_for_subtask_id = None;
                if t.status.as_deref() == Some("waiting_subtask") {
                    t.status = Some("idle".to_string());
                }
                any_modified = true;
            }
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
    let actual_root = resolve_actual_repo_root(repo_root);
    if actual_root.as_os_str().is_empty() {
        return Vec::new();
    }
    let boards_cfg = load_boards_config(&actual_root)
        .await
        .unwrap_or_else(|_| crate::config::default_boards_config());

    // 1. Primary Source of Truth: SQLite Database (.cvh/harness.db)
    let mut tasks = match crate::db::db_get_all_tasks(&actual_root) {
        Ok(t) => t,
        Err(e) => {
            crate::logger::cvh_log(
                Some(&actual_root),
                "WARN",
                "LOAD_TASKS",
                &format!("db_get_all_tasks warning: {}. Checking JSON fallbacks...", e),
            );
            Vec::new()
        }
    };

    // 2. One-time migration: if SQLite returned 0 tasks, but legacy .cvh/tasks.json exists
    let local_tasks_file = actual_root.join(".cvh").join("tasks.json");
    if tasks.is_empty() {
        if local_tasks_file.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&local_tasks_file).await {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    if let Ok(parsed) = serde_json::from_str::<Vec<Task>>(trimmed) {
                        if !parsed.is_empty() {
                            tasks = parsed;
                            let _ = crate::db::db_upsert_all_tasks(&actual_root, &tasks);
                            crate::logger::cvh_log(
                                Some(&actual_root),
                                "INFO",
                                "MIGRATION",
                                &format!("Migrated {} tasks from legacy tasks.json to SQLite", tasks.len()),
                            );
                        }
                    }
                }
            }
            // Safely remove legacy file so it no longer clutters disk
            let _ = tokio::fs::remove_file(&local_tasks_file).await;
        }

        if tasks.is_empty() {
            let backup_file = get_project_tasks_backup_file(&actual_root);
            if backup_file.exists() {
                if let Ok(content) = tokio::fs::read_to_string(&backup_file).await {
                    let trimmed = content.trim();
                    if !trimmed.is_empty() {
                        if let Ok(parsed) = serde_json::from_str::<Vec<Task>>(trimmed) {
                            if !parsed.is_empty() {
                                eprintln!(
                                    "CVH [load_tasks_from_disk]: Restored {} tasks from project backup {:?}",
                                    parsed.len(),
                                    backup_file
                                );
                                tasks = parsed;
                                let _ = crate::db::db_upsert_all_tasks(&actual_root, &tasks);
                            }
                        }
                    }
                }
            }
        }
    } else if local_tasks_file.exists() {
        // Safe cleanup if SQLite already has tasks but stale tasks.json still lingers
        let _ = tokio::fs::remove_file(&local_tasks_file).await;
    }

    let modified = heal_tasks(&mut tasks, &boards_cfg);
    if modified {
        crate::logger::cvh_log(
            Some(&actual_root),
            "INFO",
            "LOAD_TASKS",
            &format!("Healed tasks against boards config, persisting {} tasks", tasks.len()),
        );
        let _ = save_tasks_to_disk(&actual_root, &tasks).await;
    }

    let ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    crate::logger::cvh_log_with_details(
        Some(&actual_root),
        "DEBUG",
        "LOAD_TASKS",
        &format!("load_tasks_from_disk: loaded {} tasks for {:?}", tasks.len(), actual_root),
        Some(&format!("Task IDs: [{}]", ids.join(", "))),
    );

    // Monotonic sequence initialization: ensure persistent sequence is at least max_task_num_on_disk
    let max_on_disk = max_task_num_on_disk(&tasks);
    if max_on_disk > 0 {
        let current_seq = load_task_sequence(&actual_root).await;
        if max_on_disk > current_seq {
            let _ = save_task_sequence(&actual_root, max_on_disk).await;
        }
    }

    tasks
}

pub fn get_project_tasks_backup_file(project_root: &Path) -> PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    project_root.to_string_lossy().to_lowercase().hash(&mut hasher);
    let hash = hasher.finish();
    get_app_config_dir().join("projects").join(format!("{:016x}", hash)).join("tasks.json")
}

pub async fn save_tasks_to_disk(repo_root: &Path, tasks: &[Task]) -> Result<(), String> {
    let actual_root = resolve_actual_repo_root(repo_root);
    if actual_root.as_os_str().is_empty() {
        return Ok(());
    }

    // 1. 100% Single Source of Truth: High-speed SQLite database (.cvh/harness.db)
    let db_res = crate::db::db_upsert_all_tasks(&actual_root, tasks);
    if let Err(ref e) = db_res {
        crate::logger::cvh_log(
            Some(&actual_root),
            "WARN",
            "SAVE_TASKS",
            &format!("save_tasks_to_disk: SQLite persistence warning: {}", e),
        );
    }

    // 2. Snapshot to project backup in LocalAppData for disaster recovery
    let serialized = serde_json::to_string_pretty(tasks)
        .map_err(|e| format!("Failed to serialize tasks: {}", e))?;

    let backup_file = get_project_tasks_backup_file(&actual_root);
    let backup_res = atomic_write_file(&backup_file, &serialized).await;

    let ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    match (db_res, backup_res) {
        (Ok(_), Ok(_)) => {
            crate::logger::cvh_log_with_details(
                Some(&actual_root),
                "INFO",
                "SAVE_TASKS",
                &format!("save_tasks_to_disk: successfully saved {} tasks to SQLite & disaster backup", tasks.len()),
                Some(&format!("Saved IDs: [{}]", ids.join(", "))),
            );
            Ok(())
        }
        (Ok(_), Err(e)) => {
            crate::logger::cvh_log_with_details(
                Some(&actual_root),
                "WARN",
                "SAVE_TASKS",
                &format!("save_tasks_to_disk: saved {} tasks to SQLite, but disaster backup failed: {}", tasks.len(), e),
                Some(&format!("Saved IDs: [{}]", ids.join(", "))),
            );
            Ok(())
        }
        (Err(e), Ok(_)) => {
            crate::logger::cvh_log_with_details(
                Some(&actual_root),
                "WARN",
                "SAVE_TASKS",
                &format!("save_tasks_to_disk: SQLite error: {}, but persisted to disaster backup {:?}", e, backup_file),
                Some(&format!("Saved IDs: [{}]", ids.join(", "))),
            );
            Ok(())
        }
        (Err(e1), Err(e2)) => {
            let err_msg = format!("Failed to save tasks to both SQLite ({}) and backup ({})", e1, e2);
            crate::logger::cvh_log(
                Some(&actual_root),
                "ERROR",
                "SAVE_TASKS",
                &err_msg,
            );
            Err(err_msg)
        }
    }
}

pub fn task_id_matches(id1: &str, id2: &str) -> bool {
    id1.trim().eq_ignore_ascii_case(id2.trim())
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
    let root = resolve_actual_repo_root(&state.get_root());
    let tasks = load_tasks_from_disk(&root).await;
    let ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    crate::logger::cvh_log_with_details(
        Some(&root),
        "INFO",
        "LIST_TASKS",
        &format!("list_tasks command: returning {} tasks for root '{}'", tasks.len(), root.display()),
        Some(&format!("Task IDs: [{}]", ids.join(", "))),
    );
    if let Ok(mut guard) = state.tasks.lock() {
        *guard = tasks.clone();
    }
    Ok(tasks)
}

#[tauri::command]
pub async fn save_tasks(tasks: Vec<Task>, state: State<'_, AppState>) -> Result<(), String> {
    let root = resolve_actual_repo_root(&state.get_root());
    internal_save_tasks(&root, tasks, Some(&state.tasks)).await
}

pub async fn internal_save_tasks(
    root: &Path,
    tasks: Vec<Task>,
    state_tasks: Option<&Arc<Mutex<Vec<Task>>>>,
) -> Result<(), String> {
    let incoming_ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    crate::logger::cvh_log_with_details(
        Some(root),
        "INFO",
        "SAVE_TASKS",
        &format!("save_tasks command: received {} tasks from client for root '{}'", tasks.len(), root.display()),
        Some(&format!("Incoming IDs: [{}]", incoming_ids.join(", "))),
    );
    let disk_tasks = load_tasks_from_disk(root).await;
    let boards_cfg = crate::config::load_boards_config(root).await.ok();

    // Defensive reconciliation: prevent stale frontend saves from destroying newly created
    // subtasks or wiping out active parent 'waiting_subtask' invariants (§ 2.2 & § 2.22).
    let mut merged_tasks = tasks;

    // 1. Preserve existing subtasks or recent tasks from disk if missing from incoming tasks:
    // (delete_task has its own dedicated IPC command and is the ONLY authorized deletion path).
    for dt in &disk_tasks {
        if !merged_tasks.iter().any(|t| t.id == dt.id) {
            merged_tasks.push(dt.clone());
        }
    }

    // 2. Preserve waiting_for_subtask_id and waiting_subtask status if the subtask is still active
    let active_sub_ids: HashSet<String> = merged_tasks
        .iter()
        .filter(|sub| sub.status.as_deref() != Some("done"))
        .map(|sub| sub.id.clone())
        .collect();

    for t in &mut merged_tasks {
        if let Some(dt) = disk_tasks.iter().find(|d| d.id == t.id) {
            let is_term = stage_id_matches(&t.stage_id, "ready")
                || stage_id_matches(&t.stage_id, "done")
                || t.status.as_deref() == Some("done")
                || boards_cfg.as_ref().is_some_and(|cfg| {
                    cfg.boards.iter().any(|b| {
                        b.columns.iter().any(|c| stage_id_matches(&c.id, &t.stage_id) && c.terminal == Some(true))
                    })
                });

            if is_term {
                t.waiting_for_subtask_id = None;
                if t.status.as_deref() == Some("waiting_subtask") {
                    t.status = Some("done".to_string());
                }
            } else if dt.status.as_deref() == Some("waiting_subtask") && t.status.as_deref() != Some("waiting_subtask") {
                if let Some(ref sub_id) = dt.waiting_for_subtask_id {
                    if active_sub_ids.contains(sub_id) {
                        t.status = Some("waiting_subtask".to_string());
                        t.waiting_for_subtask_id = Some(sub_id.clone());
                    } else {
                        t.waiting_for_subtask_id = None;
                    }
                }
            } else if dt.waiting_for_subtask_id.is_some() && t.waiting_for_subtask_id.is_none() {
                if let Some(ref sub_id) = dt.waiting_for_subtask_id {
                    if active_sub_ids.contains(sub_id) {
                        t.waiting_for_subtask_id = Some(sub_id.clone());
                    }
                }
            }
            if t.attachments.is_none() && dt.attachments.is_some() {
                t.attachments = dt.attachments.clone();
            }
            if t.cover_image.is_none() && dt.cover_image.is_some() {
                t.cover_image = dt.cover_image.clone();
            }
            if t.handover_summary.is_none() && dt.handover_summary.is_some() {
                t.handover_summary = dt.handover_summary.clone();
            }
            if t.hook_states.is_none() && dt.hook_states.is_some() {
                t.hook_states = dt.hook_states.clone();
            }
            if t.worktree_path.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true) && dt.worktree_path.is_some() {
                t.worktree_path = dt.worktree_path.clone();
            }
            if t.branch.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true) && dt.branch.is_some() {
                t.branch = dt.branch.clone();
            }
            if t.parent_id.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true) && dt.parent_id.is_some() {
                t.parent_id = dt.parent_id.clone();
            }
        }
    }

    if let Some(st) = state_tasks {
        if let Ok(mut guard) = st.lock() {
            *guard = merged_tasks.clone();
        }
    }
    let res = save_tasks_to_disk(root, &merged_tasks).await;
    match &res {
        Ok(_) => {
            let merged_ids: Vec<&str> = merged_tasks.iter().map(|t| t.id.as_str()).collect();
            crate::logger::cvh_log_with_details(
                Some(root),
                "INFO",
                "SAVE_TASKS",
                &format!("save_tasks command: successfully persisted {} merged tasks to disk", merged_tasks.len()),
                Some(&format!("Merged IDs: [{}]", merged_ids.join(", "))),
            );
        }
        Err(e) => {
            crate::logger::cvh_log(
                Some(root),
                "ERROR",
                "SAVE_TASKS",
                &format!("save_tasks command: FAILED to persist {} tasks: {}", merged_tasks.len(), e),
            );
        }
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
    let max_on_disk = max_task_num_on_disk(tasks);
    match crate::db::db_allocate_next_task_num(repo_root) {
        Ok(num) => {
            if num <= max_on_disk {
                let corrected = max_on_disk.saturating_add(1);
                let _ = crate::db::db_update_task_sequence(repo_root, corrected);
                corrected
            } else {
                num
            }
        }
        Err(e) => {
            eprintln!(
                "CVH [allocate_next_task_num]: db_allocate_next_task_num warning: {}. Falling back to disk seq.",
                e
            );
            let stored_seq = load_task_sequence(repo_root).await;
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
    }
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

/// Resolves the task ID: uses explicit task_id from frontend (synchronizing sequence.json)
/// or allocates the next monotonic ID if not provided.
pub async fn resolve_or_allocate_task_id(
    repo_root: &Path,
    tasks: &[Task],
    task_id: Option<&str>,
) -> String {
    match task_id.map(str::trim).filter(|s| !s.is_empty()) {
        Some(explicit_id) => {
            let base = explicit_id.split('.').next().unwrap_or(explicit_id);
            let digits: String = base.chars().filter(|c| c.is_ascii_digit()).collect();
            if let Ok(task_num) = digits.parse::<u32>() {
                let stored_seq = load_task_sequence(repo_root).await;
                let seq_to_save = stored_seq.max(task_num);
                if let Err(e) = save_task_sequence(repo_root, seq_to_save).await {
                    eprintln!(
                        "CVH [resolve_or_allocate_task_id]: Warning - failed to persist sequence {}: {}",
                        seq_to_save, e
                    );
                }
            }
            explicit_id.to_string()
        }
        None => {
            let task_num = allocate_next_task_num(repo_root, tasks).await;
            format!("CVH-{:03}", task_num)
        }
    }
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn create_task(
    title: String,
    description: String,
    stage_id: String,
    board_id: Option<String>,
    skip_worktree: Option<bool>,
    skip_backlog_hook: Option<bool>,
    model: Option<String>,
    task_id: Option<String>,
    attachments: Option<Vec<String>>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    let effective_skip = skip_backlog_hook.or(skip_worktree);
    let root = state.get_root();
    crate::logger::cvh_log_with_details(
        Some(&root),
        "INFO",
        "CREATE_TASK",
        &format!(
            "create_task IPC called: title='{}', stage_id='{}', board_id={:?}, task_id_param={:?}, skip_backlog_hook={:?}, model={:?}",
            title, stage_id, board_id, task_id, effective_skip, model
        ),
        Some(&format!("root='{}'", root.display())),
    );
    internal_create_task(
        &root,
        title,
        description,
        stage_id,
        board_id,
        effective_skip,
        model,
        task_id,
        attachments,
        Some(&app_handle),
        Some(state.inner()),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub fn record_task_hook_result(
    task: &mut Task,
    stage_id: &str,
    hook_type: &str,
    hook_name: &str,
    started_at: i64,
    duration_ms: u64,
    success: bool,
    exit_code: Option<i32>,
    stdout: &str,
    stderr: &str,
    error: Option<&str>,
) {
    let mut map: serde_json::Map<String, serde_json::Value> = task
        .hook_states
        .as_ref()
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();

    map.insert(
        hook_type.to_string(),
        serde_json::json!({
            "hookName": hook_name,
            "stageId": stage_id,
            "hookType": hook_type,
            "status": if success { "success" } else { "failed" },
            "startedAt": started_at,
            "durationMs": duration_ms,
            "exitCode": exit_code,
            "stdout": stdout,
            "stderr": stderr,
            "error": error,
        }),
    );

    task.hook_states = Some(serde_json::Value::Object(map));
}

#[allow(clippy::too_many_arguments)]
pub async fn internal_create_task(
    root: &Path,
    title: String,
    description: String,
    stage_id: String,
    board_id: Option<String>,
    skip_worktree: Option<bool>,
    model: Option<String>,
    task_id: Option<String>,
    attachments: Option<Vec<String>>,
    app_handle: Option<&AppHandle>,
    state: Option<&AppState>,
) -> Result<Task, String> {
    let actual_root = resolve_actual_repo_root(root);
    let root = &actual_root;
    crate::logger::cvh_log_with_details(
        Some(root),
        "INFO",
        "CREATE_TASK",
        &format!(
            "internal_create_task starting: title='{}', stage_id='{}', board_id={:?}, task_id_param={:?}",
            title, stage_id, board_id, task_id
        ),
        Some(&format!("actual_root='{}'", root.display())),
    );
    let mut tasks = load_tasks_from_disk(root).await;

    let effective_board_id = match board_id.as_deref() {
        Some(b) if !b.trim().is_empty() => b.trim().to_string(),
        _ => {
            load_boards_config(root)
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

    // Generate task ID — use explicit task_id (syncing sequence) or allocate monotonic ID
    let task_id = resolve_or_allocate_task_id(root, &tasks, task_id.as_deref()).await;
    crate::logger::cvh_log(
        Some(root),
        "INFO",
        "CREATE_TASK",
        &format!("Resolved task_id='{}' for task title='{}'", task_id, title),
    );
    let now = Utc::now().timestamp_millis();

    let next_pos = tasks
        .iter()
        .filter(|t| t.stage_id == stage_id)
        .filter_map(|t| t.position)
        .fold(0.0f64, f64::max) + 1000.0;

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
        attachments,
        todos: None,
        priority: None,
        tags: None,
        current_tokens: None,
        skip_worktree: skip_worktree.or(Some(false)),
        created_at: now,
        updated_at: now,
        position: Some(next_pos),
        hook_states: None,
    };

    // RACE CONDITION PREVENTION (AGENTS.md):
    // Atomically register the new task in in-memory state.tasks and save to disk IMMEDIATELY
    // before launching slow operations (worktree creation, hooks).
    // This guarantees that any concurrent move_task_stage or run_agent calls will find the task.
    if let Some(state_ref) = state {
        if let Ok(mut guard) = state_ref.tasks.lock() {
            if !guard.iter().any(|t| t.id == new_task.id) {
                guard.push(new_task.clone());
            }
            tasks = guard.clone();
        } else if !tasks.iter().any(|t| t.id == new_task.id) {
            tasks.push(new_task.clone());
        }
    } else if !tasks.iter().any(|t| t.id == new_task.id) {
        tasks.push(new_task.clone());
    }
    let _ = crate::db::db_upsert_task(root, &new_task);
    if let Err(e) = save_tasks_to_disk(root, &tasks).await {
        crate::logger::cvh_log(
            Some(root),
            "WARN",
            "CREATE_TASK",
            &format!("Initial save_tasks_to_disk warning for task '{}': {}. Task registered in state.", new_task.id, e),
        );
    }
    crate::logger::cvh_log(
        Some(root),
        "INFO",
        "CREATE_TASK",
        &format!("Initial task '{}' successfully registered in state and saved to disk (total: {})", new_task.id, tasks.len()),
    );
    if let Some(app) = app_handle {
        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
    }

    // Check pipeline config for stage hooks (board-aware: first check columns
    // of effective_board_id, then fallback to pipeline.json).
    // If skip_backlog_hook / skip_worktree is set, do NOT run on_enter hooks and do NOT provision builtin worktree.
    let skip = new_task.skip_worktree.unwrap_or(false);
    if !skip {
        let boards_cfg = load_boards_config(root).await.ok();
        let col_from_board = boards_cfg.as_ref().and_then(|bc| {
            let b = bc.boards.iter().find(|b| b.id == effective_board_id)
                .or_else(|| bc.boards.iter().find(|b| b.id == bc.active_board_id));
            b.and_then(|b| b.columns.iter().find(|c| c.id == stage_id).cloned())
        });
        let col_opt: Option<crate::config::PipelineColumn> = match col_from_board {
            Some(c) => Some(c),
            None => load_pipeline_config(root)
                .await
                .ok()
                .and_then(|p| p.columns.into_iter().find(|c| c.id == stage_id)),
        };
        if let Some(col) = col_opt {
            trigger_column_on_enter_script(&col, &new_task, &stage_id, None, root).await;
            if let Some(hook_rel) = col.hooks.get("on_enter").cloned() {
                let hook_path = crate::config::resolve_hook_path(
                    root,
                    col.persona.as_deref(),
                    &hook_rel,
                );
                if hook_path.exists() {
                    if let Some(app) = app_handle {
                        let _ = app.emit("hook:start", serde_json::json!({
                            "taskId": task_id,
                            "stageId": stage_id,
                            "hookType": "on_enter",
                            "hookName": hook_rel
                        }));
                    }
                    let hook_started_at = chrono::Utc::now().timestamp_millis();
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
                    match execute_hook(&hook_path, root, &ctx, None).await {
                        Ok(res) => {
                            let duration_ms = (chrono::Utc::now().timestamp_millis() - hook_started_at).max(0) as u64;
                            if !res.success {
                                let err_details = if !res.stderr.trim().is_empty() {
                                    res.stderr.trim().to_string()
                                } else if !res.stdout.trim().is_empty() {
                                    res.stdout.trim().to_string()
                                } else {
                                    format!("Exit code {:?}", res.exit_code)
                                };
                                record_task_hook_result(
                                    &mut new_task,
                                    &stage_id,
                                    "on_enter",
                                    &hook_rel,
                                    hook_started_at,
                                    duration_ms,
                                    false,
                                    res.exit_code,
                                    &res.stdout,
                                    &res.stderr,
                                    Some(&err_details),
                                );
                                if let Some(app) = app_handle {
                                    let _ = app.emit("hook:finish", serde_json::json!({
                                        "taskId": task_id,
                                        "stageId": stage_id,
                                        "hookType": "on_enter",
                                        "success": false,
                                        "exitCode": res.exit_code,
                                        "stdout": res.stdout,
                                        "stderr": res.stderr,
                                        "error": err_details
                                    }));
                                }
                                let steer_content = format!(
                                    "### [ОТКЛОНЕНО ХУКОМ / HOOK REJECTED: {}]\n\nХук завершился с ошибкой (код {:?}).\n\n**Лог ошибки:**\n```\n{}\n```\n\nИсправь указанную ошибку.",
                                    hook_rel, res.exit_code, err_details
                                );
                                let _ = append_task_dialog_activity(
                                    root,
                                    &task_id,
                                    "steer",
                                    &steer_content,
                                    app_handle,
                                ).await;
                                new_task.verdict = Some("CHANGES_REQUESTED".to_string());
                            } else {
                                record_task_hook_result(
                                    &mut new_task,
                                    &stage_id,
                                    "on_enter",
                                    &hook_rel,
                                    hook_started_at,
                                    duration_ms,
                                    true,
                                    res.exit_code,
                                    &res.stdout,
                                    &res.stderr,
                                    None,
                                );
                                if let Some(app) = app_handle {
                                    let _ = app.emit("hook:finish", serde_json::json!({
                                        "taskId": task_id,
                                        "stageId": stage_id,
                                        "hookType": "on_enter",
                                        "success": true,
                                        "exitCode": res.exit_code,
                                        "stdout": res.stdout,
                                        "stderr": res.stderr
                                    }));
                                }
                                apply_worktree_from_hook_stdout(&mut new_task, &res.stdout);
                            }
                        }
                        Err(e) => {
                            let duration_ms = (chrono::Utc::now().timestamp_millis() - hook_started_at).max(0) as u64;
                            let err_details = e.to_string();
                            record_task_hook_result(
                                &mut new_task,
                                &stage_id,
                                "on_enter",
                                &hook_rel,
                                hook_started_at,
                                duration_ms,
                                false,
                                None,
                                "",
                                &err_details,
                                Some(&err_details),
                            );
                            if let Some(app) = app_handle {
                                let _ = app.emit("hook:finish", serde_json::json!({
                                    "taskId": task_id,
                                    "stageId": stage_id,
                                    "hookType": "on_enter",
                                    "success": false,
                                    "exitCode": None::<i32>,
                                    "stdout": "",
                                    "stderr": err_details,
                                    "error": err_details
                                }));
                            }
                            let steer_content = format!(
                                "### [ОТКЛОНЕНО ХУКОМ / HOOK REJECTED: {}]\n\nОшибка запуска хука: {}\n\nИсправь указанную ошибку.",
                                hook_rel, err_details
                            );
                            let _ = append_task_dialog_activity(
                                root,
                                &task_id,
                                "steer",
                                &steer_content,
                                app_handle,
                            ).await;
                            new_task.verdict = Some("CHANGES_REQUESTED".to_string());
                        }
                    }
                }
            }
        }
    }

    // After worktree creation and hooks, update worktree_path and branch fields.
    // If the task was moved or modified concurrently, preserve its latest stage_id and status.
    let mut updated_tasks = load_tasks_from_disk(root).await;
    if let Some(t) = updated_tasks.iter_mut().find(|t| t.id == new_task.id) {
        t.worktree_path = new_task.worktree_path.clone();
        t.branch = new_task.branch.clone();
        t.hook_states = new_task.hook_states.clone();
        t.updated_at = Utc::now().timestamp_millis();
        new_task = t.clone();
    } else {
        updated_tasks.push(new_task.clone());
    }

    if let Some(state_ref) = state {
        if let Ok(mut guard) = state_ref.tasks.lock() {
            if let Some(t) = guard.iter_mut().find(|t| t.id == new_task.id) {
                t.worktree_path = new_task.worktree_path.clone();
                t.branch = new_task.branch.clone();
                t.hook_states = new_task.hook_states.clone();
                t.updated_at = new_task.updated_at;
                new_task.stage_id = t.stage_id.clone();
                new_task.status = t.status.clone();
                new_task.board_id = t.board_id.clone();
            } else {
                guard.push(new_task.clone());
            }
            for mem_t in guard.iter() {
                if !updated_tasks.iter().any(|t| t.id == mem_t.id) {
                    updated_tasks.push(mem_t.clone());
                }
            }
        }
    }
    if let Some(t) = updated_tasks.iter_mut().find(|t| t.id == new_task.id) {
        t.stage_id = new_task.stage_id.clone();
        t.status = new_task.status.clone();
        t.board_id = new_task.board_id.clone();
    }
    if let Err(e) = save_tasks_to_disk(root, &updated_tasks).await {
        crate::logger::cvh_log(
            Some(root),
            "WARN",
            "CREATE_TASK",
            &format!("Warning: save_tasks_to_disk failed for task '{}': {}. In-memory state preserved.", new_task.id, e),
        );
    }
    crate::logger::cvh_log_with_details(
        Some(root),
        "INFO",
        "CREATE_TASK",
        &format!("Successfully created and persisted task '{}'", new_task.id),
        Some(&format!(
            "title='{}', stage='{}', board={:?}, worktree={:?}, branch={:?}",
            new_task.title, new_task.stage_id, new_task.board_id, new_task.worktree_path, new_task.branch
        )),
    );
    if let Some(app) = app_handle {
        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &updated_tasks }));
    }

    Ok(new_task)
}

pub fn is_review_column(col: &crate::config::PipelineColumn) -> bool {
    col.persona.as_deref() == Some("reviewer")
        || stage_id_matches(&col.id, "review")
        || col.id.to_lowercase().contains("review")
        || col.id.to_lowercase().contains("ревью")
}

#[allow(clippy::too_many_arguments)]
pub async fn internal_create_subtask(
    root: &Path,
    parent_id: &str,
    target_stage_id: Option<&str>,
    title: Option<&str>,
    handover_summary: Option<&str>,
    app_handle: Option<&AppHandle>,
) -> Result<Task, String> {
    let root_resolved = resolve_actual_repo_root(root);
    let root = &root_resolved;
    let mut tasks = load_tasks_from_disk(root).await;

    // Merge in-memory tasks if state is available in app_handle to prevent stale data
    if let Some(app) = app_handle {
        if let Some(state_ref) = app.try_state::<AppState>() {
            if let Ok(guard) = state_ref.tasks.lock() {
                for mem_t in guard.iter() {
                    if let Some(disk_t) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &mem_t.id)) {
                        if disk_t.worktree_path.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true) && mem_t.worktree_path.is_some() {
                            disk_t.worktree_path = mem_t.worktree_path.clone();
                        }
                        if disk_t.branch.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true) && mem_t.branch.is_some() {
                            disk_t.branch = mem_t.branch.clone();
                        }
                    }
                }
            }
        }
    }

    let parent_idx = tasks
        .iter()
        .position(|t| task_id_matches(&t.id, parent_id))
        .ok_or_else(|| format!("Parent task {} not found", parent_id))?;

    let boards_cfg = load_boards_config(root).await.ok();
    let effective_board = if let Some(ref bc) = boards_cfg {
        let parent_board_id = tasks[parent_idx].board_id.as_deref();
        bc.boards
            .iter()
            .find(|b| Some(b.id.as_str()) == parent_board_id)
            .cloned()
            .or_else(|| bc.boards.iter().find(|b| b.id == bc.active_board_id).cloned())
            .or_else(|| bc.boards.first().cloned())
    } else {
        None
    };

    let target_col = if let Some(target_id) = target_stage_id {
        let from_board = effective_board.as_ref().and_then(|b| {
            b.columns.iter().find(|c| stage_id_matches(&c.id, target_id)).cloned()
        });
        match from_board {
            Some(c) => c,
            None => {
                let pipeline = load_pipeline_config(root)
                    .await
                    .unwrap_or_else(|_| crate::config::default_pipeline_config());
                pipeline
                    .columns
                    .into_iter()
                    .find(|c| stage_id_matches(&c.id, target_id))
                    .ok_or_else(|| format!("Target column {} not found in pipeline", target_id))?
            }
        }
    } else {
        let cols = if let Some(ref b) = effective_board {
            b.columns.clone()
        } else {
            let pipeline = load_pipeline_config(root)
                .await
                .unwrap_or_else(|_| crate::config::default_pipeline_config());
            pipeline.columns
        };
        let parent_col_idx = cols
            .iter()
            .position(|c| stage_id_matches(&c.id, &tasks[parent_idx].stage_id))
            .ok_or_else(|| format!("Parent column {} not found in pipeline", tasks[parent_idx].stage_id))?;
        if parent_col_idx + 1 < cols.len() {
            cols[parent_col_idx + 1].clone()
        } else {
            return Err("Parent is already in the last pipeline stage".to_string());
        }
    };

    let next_subtask_idx = get_next_subtask_index(&tasks, parent_id);
    let subtask_id = format!("{}.{}", parent_id, next_subtask_idx);
    let now = Utc::now().timestamp_millis();

    let parent_title = tasks[parent_idx].title.clone();
    let mut parent_wt = tasks[parent_idx].worktree_path.clone();
    let mut parent_branch = tasks[parent_idx].branch.clone();
    let parent_model = tasks[parent_idx].model.clone();
    let parent_skip_wt = tasks[parent_idx].skip_worktree;

    // If parent worktree is missing or empty, check filesystem heuristics
    if parent_wt.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true) {
        let candidates = [
            format!("worktrees/{}", parent_id),
            format!("worktrees/{}", parent_id.to_lowercase()),
            format!(".cvh/worktrees/{}", parent_id),
            format!(".cvh/worktrees/{}", parent_id.to_lowercase()),
        ];
        for rel in &candidates {
            if root.join(rel).exists() {
                parent_wt = Some(rel.clone());
                tasks[parent_idx].worktree_path = Some(rel.clone());
                if parent_branch.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true) {
                    let fb = format!("feat/{}", parent_id);
                    parent_branch = Some(fb.clone());
                    tasks[parent_idx].branch = Some(fb);
                }
                let _ = crate::db::db_upsert_task(root, &tasks[parent_idx]);
                break;
            }
        }
    }

    // Update parent task: mark waiting for subtask, keep parent pinned in current stage (§ 2.2)
    tasks[parent_idx].waiting_for_subtask_id = Some(subtask_id.clone());
    tasks[parent_idx].status = Some("waiting_subtask".to_string());
    tasks[parent_idx].updated_at = now;

    let subtask_title = title
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{} ({})", parent_title, target_col.title));

    let subtask_desc = handover_summary
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!(
            "### Подзадача этапа: {}\nРодительская задача: {}\n\n{}",
            target_col.title, parent_id, tasks[parent_idx].description
        ));

    let subtask_status = if target_col.terminal == Some(true) {
        "done"
    } else if target_col.persona.is_some() || target_col.auto_run == Some(true) {
        "running"
    } else {
        "idle"
    };

    let next_subtask_pos = tasks
        .iter()
        .filter(|t| t.stage_id == target_col.id)
        .filter_map(|t| t.position)
        .fold(0.0f64, f64::max) + 1000.0;

    let new_subtask = Task {
        id: subtask_id.clone(),
        title: subtask_title,
        description: subtask_desc,
        stage_id: target_col.id.clone(),
        board_id: tasks[parent_idx].board_id.clone(),
        worktree_path: parent_wt,
        branch: parent_branch,
        verdict: None,
        custom_flow: None,
        status: Some(subtask_status.to_string()),
        model: parent_model.or_else(|| target_col.model_override.clone()),
        parent_id: Some(parent_id.to_string()),
        subtask_index: Some(next_subtask_idx),
        waiting_for_subtask_id: None,
        handover_summary: handover_summary.map(|s| s.to_string()),
        cover_image: None,
        attachments: tasks[parent_idx].attachments.clone(),
        todos: None,
        priority: tasks[parent_idx].priority.clone(),
        tags: None,
        current_tokens: None,
        skip_worktree: parent_skip_wt,
        created_at: now,
        updated_at: now,
        position: Some(next_subtask_pos),
        hook_states: tasks[parent_idx].hook_states.clone(),
    };

    let parent_task_clone = tasks[parent_idx].clone();
    tasks.push(new_subtask.clone());

    let _ = crate::db::db_upsert_task(root, &new_subtask);
    let _ = crate::db::db_upsert_task(root, &parent_task_clone);

    if let Some(app) = app_handle {
        if let Some(state_ref) = app.try_state::<AppState>() {
            if let Ok(mut guard) = state_ref.tasks.lock() {
                *guard = tasks.clone();
            }
        }
    }
    save_tasks_to_disk(root, &tasks).await?;

    if let Some(app) = app_handle {
        let _ = app.emit("task:created", serde_json::json!({ "task": &new_subtask }));
        let _ = app.emit("task:updated", serde_json::json!({ "task": &parent_task_clone }));
        let _ = app.emit(
            "task:moved",
            serde_json::json!({
                "taskId": parent_id,
                "status": "waiting_subtask",
                "waitingForSubtaskId": &subtask_id,
                "task": &parent_task_clone,
            }),
        );
        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));

        if target_col.auto_run == Some(true) || target_col.persona.is_some() {
            trigger_column_on_enter_script(
                &target_col,
                &new_subtask,
                &target_col.id,
                None,
                root,
            )
            .await;
            let _ = app.emit("agent:autorun", serde_json::json!({ "taskId": &subtask_id }));
        }
    }

    Ok(new_subtask)
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
    internal_create_subtask(
        &root,
        &parent_id,
        target_stage_id.as_deref(),
        title.as_deref(),
        None,
        Some(&app_handle),
    )
    .await
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
        .position(|t| task_id_matches(&t.id, &task_id))
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
    let _ = crate::db::db_upsert_task(&root, &updated);
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
    for t in &tasks {
        if task_id_matches(&t.id, &task_id) {
            to_delete.insert(t.id.clone());
        }
    }
    let mut grew = true;
    while grew {
        grew = false;
        for t in &tasks {
            if to_delete.iter().any(|id| task_id_matches(&t.id, id)) {
                continue;
            }
            if let Some(ref p) = t.parent_id {
                if to_delete.iter().any(|id| task_id_matches(p, id)) {
                    to_delete.insert(t.id.clone());
                    grew = true;
                    continue;
                }
            }
            if to_delete.iter().any(|id| t.id.to_ascii_lowercase().starts_with(&format!("{}.", id.trim().to_ascii_lowercase()))) {
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

    // Memory leak prevention: prune deleted task IDs and cascaded subtasks from agent_contexts
    if let Ok(mut ctx_map) = state.agent_contexts.lock() {
        for id in &to_delete {
            ctx_map.remove(id);
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

    // Delete from SQLite database tasks and todos
    for id in &to_delete {
        let _ = crate::db::db_delete_task(&root, id);
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

    // Broadcast tasks:updated and task:deleted events across application
    let _ = app_handle.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
    let _ = app_handle.emit(
        "task:deleted",
        serde_json::json!({
            "taskId": &task_id,
            "deletedIds": &to_delete.iter().cloned().collect::<Vec<_>>()
        }),
    );

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

#[derive(Debug, Default)]
pub struct ParentAdvanceResult {
    pub moved_events: Vec<serde_json::Value>,
    pub created_tasks: Vec<Task>,
    pub autorun_task_ids: Vec<String>,
}

/// Propagates advancement of completed/terminal subtasks up the hierarchy to parent tasks (§ 2.2 & § 2.7).
/// Universal relay progression: if next stage is non-terminal, parent spawns next isolated child subtask;
/// if next stage is terminal (ready/done), parent moves to ready with status done.
pub fn propagate_parent_subtask_advance(
    tasks: &mut Vec<Task>,
    completed_subtask_id: &str,
    subtask_source_stage: &str,
    verdict: Option<&str>,
    boards_cfg: Option<&crate::config::BoardsConfig>,
    pipeline: Option<&crate::config::PipelineConfig>,
) -> ParentAdvanceResult {
    let mut moved_events = Vec::new();
    let mut created_tasks = Vec::new();
    let mut autorun_task_ids = Vec::new();

    let mut current_child_id = completed_subtask_id.to_string();
    let mut current_source_stage = subtask_source_stage.to_string();

    // 1. Ensure the completed subtask has status "done"
    if let Some(sub_idx) = tasks.iter().position(|t| task_id_matches(&t.id, completed_subtask_id)) {
        tasks[sub_idx].status = Some("done".to_string());
        tasks[sub_idx].updated_at = Utc::now().timestamp_millis();
    }

    while let Some(parent_idx) = tasks.iter().position(|t| {
        t.waiting_for_subtask_id.as_deref() == Some(&current_child_id)
            || (t.status.as_deref() == Some("waiting_subtask")
                && tasks
                    .iter()
                    .find(|sub| sub.id == current_child_id)
                    .and_then(|sub| sub.parent_id.as_deref())
                    == Some(&t.id))
            || tasks
                .iter()
                .find(|sub| sub.id == current_child_id)
                .and_then(|sub| sub.parent_id.as_deref())
                == Some(&t.id)
    }) {
        let parent_id = tasks[parent_idx].id.clone();
        let old_parent_stage = tasks[parent_idx].stage_id.clone();
        let parent_board_id = tasks[parent_idx].board_id.clone();
        let parent_title = tasks[parent_idx].title.clone();

        // Resolve columns for parent task's board
        let cols = {
            let from_boards = boards_cfg.and_then(|bc| {
                if let Some(ref bid) = parent_board_id {
                    if let Some(b) = bc.boards.iter().find(|b| &b.id == bid) {
                        return Some(b.columns.clone());
                    }
                }
                if let Some(b) = bc.boards.iter().find(|b| b.id == bc.active_board_id) {
                    return Some(b.columns.clone());
                }
                bc.boards.first().map(|b| b.columns.clone())
            });
            from_boards.or_else(|| pipeline.map(|p| p.columns.clone())).unwrap_or_default()
        };

        // Find the next stage for the parent task:
        // Look up index of the subtask's source stage (e.g. "dev" or "review"):
        let sub_stage_idx = cols
            .iter()
            .position(|c| stage_id_matches(&c.id, &current_source_stage))
            .or_else(|| cols.iter().position(|c| stage_id_matches(&c.id, &old_parent_stage)));

        let next_col_opt = if let Some(idx) = sub_stage_idx {
            if idx + 1 < cols.len() {
                Some(cols[idx + 1].clone())
            } else {
                cols.last().cloned()
            }
        } else {
            cols.iter()
                .find(|c| c.terminal == Some(true) || stage_id_matches(&c.id, "ready") || stage_id_matches(&c.id, "done"))
                .cloned()
                .or_else(|| cols.last().cloned())
        };

        let next_col = match next_col_opt {
            Some(c) => c,
            None => break,
        };

        let is_terminal = next_col.terminal == Some(true)
            || stage_id_matches(&next_col.id, "ready")
            || stage_id_matches(&next_col.id, "done");

        if is_terminal {
            tasks[parent_idx].waiting_for_subtask_id = None;
            tasks[parent_idx].stage_id = next_col.id.clone();
            tasks[parent_idx].verdict = Some(verdict.unwrap_or("APPROVED").to_string());
            tasks[parent_idx].status = Some("done".to_string());
            tasks[parent_idx].updated_at = Utc::now().timestamp_millis();

            // Also mark any child subtasks of this parent as done so they collapse into parent (§ 3)
            for sub in tasks.iter_mut() {
                if sub.parent_id.as_deref() == Some(&parent_id) {
                    sub.status = Some("done".to_string());
                }
            }

            let parent_clone = tasks[parent_idx].clone();
            moved_events.push(serde_json::json!({
                "taskId": parent_id,
                "fromStage": old_parent_stage,
                "toStage": &next_col.id,
                "stageId": &next_col.id,
                "status": parent_clone.status,
                "verdict": parent_clone.verdict,
                "task": &parent_clone
            }));

            // If parent task also advanced to a terminal column, check if grandparent was waiting on it!
            current_child_id = parent_id;
            current_source_stage = next_col.id;
        } else {
            // Next stage is not terminal -> universal progression spawns next isolated child subtask!
            let next_subtask_idx = get_next_subtask_index(tasks, &parent_id);
            let new_subtask_id = format!("{}.{}", parent_id, next_subtask_idx);
            let now = Utc::now().timestamp_millis();

            tasks[parent_idx].waiting_for_subtask_id = Some(new_subtask_id.clone());
            tasks[parent_idx].status = Some("waiting_subtask".to_string());
            tasks[parent_idx].updated_at = now;

            let has_autorun = next_col.persona.is_some() || next_col.auto_run == Some(true);
            let subtask_status = if has_autorun { "running" } else { "idle" };

            let parent_wt = tasks[parent_idx].worktree_path.clone();
            let parent_branch = tasks[parent_idx].branch.clone();
            let parent_model = tasks[parent_idx].model.clone();
            let parent_skip_wt = tasks[parent_idx].skip_worktree;
            let parent_priority = tasks[parent_idx].priority.clone();

            let next_subtask_pos = tasks
                .iter()
                .filter(|t| t.stage_id == next_col.id)
                .filter_map(|t| t.position)
                .fold(0.0f64, f64::max) + 1000.0;

            let new_subtask = Task {
                id: new_subtask_id.clone(),
                title: format!("{} ({})", parent_title, next_col.title),
                description: format!(
                    "### Подзадача этапа: {}
Родительская задача: {}

{}",
                    next_col.title, parent_id, tasks[parent_idx].description
                ),
                stage_id: next_col.id.clone(),
                board_id: tasks[parent_idx].board_id.clone(),
                worktree_path: parent_wt,
                branch: parent_branch,
                verdict: None,
                custom_flow: None,
                status: Some(subtask_status.to_string()),
                model: parent_model.or_else(|| next_col.model_override.clone()),
                parent_id: Some(parent_id.clone()),
                subtask_index: Some(next_subtask_idx),
                waiting_for_subtask_id: None,
                handover_summary: None,
                cover_image: None,
                attachments: tasks[parent_idx].attachments.clone(),
                todos: None,
                priority: parent_priority,
                tags: None,
                current_tokens: None,
                skip_worktree: parent_skip_wt,
                created_at: now,
                updated_at: now,
                position: Some(next_subtask_pos),
                hook_states: tasks[parent_idx].hook_states.clone(),
            };

            if has_autorun {
                autorun_task_ids.push(new_subtask_id.clone());
            }

            let parent_clone = tasks[parent_idx].clone();
            moved_events.push(serde_json::json!({
                "taskId": parent_id,
                "fromStage": &old_parent_stage,
                "toStage": &old_parent_stage,
                "stageId": &old_parent_stage,
                "status": "waiting_subtask",
                "waitingForSubtaskId": &new_subtask_id,
                "verdict": parent_clone.verdict,
                "task": &parent_clone
            }));

            created_tasks.push(new_subtask.clone());
            tasks.push(new_subtask);

            // Parent is now waiting on the newly created subtask; break recursion
            break;
        }
    }

    ParentAdvanceResult {
        moved_events,
        created_tasks,
        autorun_task_ids,
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn internal_move_task_stage(
    root: &Path,
    task_id: &str,
    target_stage_id: &str,
    verdict: Option<String>,
    status_override: Option<String>,
    handover_summary: Option<String>,
    app_handle: Option<&AppHandle>,
    is_autonomous_advance: bool,
    board_id_override: Option<String>,
) -> Result<Task, String> {
    let actual_root = resolve_actual_repo_root(root);
    let root = &actual_root;
    crate::logger::cvh_log_with_details(
        Some(root),
        "INFO",
        "MOVE_STAGE",
        &format!(
            "internal_move_task_stage: task_id='{}', target_stage='{}', board_override={:?}, verdict={:?}, auto_adv={}",
            task_id, target_stage_id, board_id_override, verdict, is_autonomous_advance
        ),
        Some(&format!("actual_root='{}'", root.display())),
    );
    let mut tasks = load_tasks_from_disk(root).await;

    // In-memory fallback: if task was just created and not yet on disk,
    // or if memory has tasks not yet on disk, retrieve from AppState (§ 2.22).
    if !tasks.iter().any(|t| task_id_matches(&t.id, task_id)) {
        crate::logger::cvh_log(
            Some(root),
            "WARN",
            "MOVE_STAGE",
            &format!("Task '{}' not found in {} disk tasks. Checking in-memory AppState tasks...", task_id, tasks.len()),
        );
        if let Some(app) = app_handle {
            if let Some(state) = app.try_state::<AppState>() {
                if let Ok(guard) = state.tasks.lock() {
                    for mem_t in guard.iter() {
                        if !tasks.iter().any(|t| task_id_matches(&t.id, &mem_t.id)) {
                            tasks.push(mem_t.clone());
                        }
                    }
                }
            }
        }
    }

    let task_idx = match tasks.iter().position(|t| task_id_matches(&t.id, task_id)) {
        Some(idx) => idx,
        None => {
            let disk_ids: Vec<String> = tasks.iter().map(|t| t.id.clone()).collect();
            let err_msg = format!("Task {} not found", task_id);
            crate::logger::cvh_log_with_details(
                Some(root),
                "ERROR",
                "MOVE_STAGE",
                &format!("REJECTED move_task_stage: {}", err_msg),
                Some(&format!("Available tasks on disk/memory (count: {}): [{}]", disk_ids.len(), disk_ids.join(", "))),
            );
            return Err(err_msg);
        }
    };

    let current_task = &mut tasks[task_idx];

    // --- Board resolution & fallback (support alias and fallback when board_id missing/stale) ---
    // ZERO INVISIBLE TASKS (§ 2.18):
    // 1. If explicit board_id_override is provided and contains target stage, stay on it.
    // 2. Otherwise if current_task.board_id contains target stage, preserve it (NO accidental board hopping!).
    // 3. Otherwise if active_board contains target stage, use active_board.
    // 4. Only if no priority board contains the stage, search any other board with matching target stage.
    let boards_cfg = load_boards_config(root).await.ok();
    let mut effective_board: Option<crate::config::BoardDef> = None;

    if let Some(ref bc) = boards_cfg {
        let active_board = bc.boards.iter().find(|b| b.id == bc.active_board_id).cloned();
        let override_board = board_id_override.as_deref().and_then(|bid| {
            bc.boards.iter().find(|b| b.id == bid).cloned()
        });
        let current_board = current_task.board_id.as_deref().and_then(|bid| {
            bc.boards.iter().find(|b| b.id == bid).cloned()
        });

        let board_has_target = |b: &crate::config::BoardDef| -> bool {
            b.columns.iter().any(|c| c.id == target_stage_id || stage_id_matches(&c.id, target_stage_id))
        };

        // 1. Explicit override board requested by caller (e.g. active board during user move)
        if let Some(ref ob) = override_board {
            if board_has_target(ob) {
                effective_board = Some(ob.clone());
                current_task.board_id = Some(ob.id.clone());
            }
        }

        // 2. Task's current board if it contains the target stage (preserves task on its board)
        if effective_board.is_none() {
            if let Some(ref cb) = current_board {
                if board_has_target(cb) {
                    effective_board = Some(cb.clone());
                    // Keep current_task.board_id intact!
                }
            }
        }

        // 3. Active board if it contains the target stage
        if effective_board.is_none() {
            if let Some(ref ab) = active_board {
                if board_has_target(ab) {
                    effective_board = Some(ab.clone());
                    current_task.board_id = Some(ab.id.clone());
                }
            }
        }

        // 4. Any board that defines the target stage (cross-board pipeline advance)
        if effective_board.is_none() {
            let any_with_target = bc.boards.iter().find(|b| board_has_target(b)).cloned();
            if let Some(ref b) = any_with_target {
                effective_board = Some(b.clone());
                current_task.board_id = Some(b.id.clone());
            } else {
                effective_board = active_board.or_else(|| bc.boards.first().cloned());
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
        if let Some(hook_rel) = scol.hooks.get("on_leave").or_else(|| scol.hooks.get("on_exit")) {
            let hook_path = crate::config::resolve_hook_path(
                root,
                scol.persona.as_deref(),
                hook_rel,
            );
            if hook_path.exists() {
                if let Some(app) = app_handle {
                    let _ = app.emit("hook:start", serde_json::json!({
                        "taskId": current_task.id,
                        "stageId": scol.id,
                        "hookType": "on_leave",
                        "hookName": hook_rel
                    }));
                }
                let hook_started_at = chrono::Utc::now().timestamp_millis();
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
                let hook_res = execute_hook(&hook_path, root, &ctx, None).await;
                match hook_res {
                    Ok(res) => {
                        let duration_ms = (chrono::Utc::now().timestamp_millis() - hook_started_at).max(0) as u64;
                        if !res.success {
                            let err_details = if !res.stderr.trim().is_empty() {
                                res.stderr.trim().to_string()
                            } else if !res.stdout.trim().is_empty() {
                                res.stdout.trim().to_string()
                            } else {
                                format!("Exit code {:?}", res.exit_code)
                            };
                            record_task_hook_result(
                                current_task,
                                &scol.id,
                                "on_leave",
                                hook_rel,
                                hook_started_at,
                                duration_ms,
                                false,
                                res.exit_code,
                                &res.stdout,
                                &res.stderr,
                                Some(&err_details),
                            );

                            if let Some(app) = app_handle {
                                let _ = app.emit("hook:finish", serde_json::json!({
                                    "taskId": current_task.id,
                                    "stageId": scol.id,
                                    "hookType": "on_leave",
                                    "success": false,
                                    "exitCode": res.exit_code,
                                    "stdout": res.stdout,
                                    "stderr": res.stderr,
                                    "error": err_details
                                }));
                            }

                            // Steer message to task dialogue
                            let steer_content = format!(
                                "### [ОТКЛОНЕНО ХУКОМ / HOOK REJECTED: {}]\n\nХук завершился с ошибкой (код {:?}).\n\n**Лог ошибки:**\n```\n{}\n```\n\nИсправь указанную ошибку.",
                                hook_rel, res.exit_code, err_details
                            );
                            let _ = append_task_dialog_activity(
                                root,
                                &current_task.id,
                                "steer",
                                &steer_content,
                                app_handle,
                            ).await;

                            // Update task state on failure: card stays in source stage
                            let scol_has_autorun = scol.persona.is_some() || scol.auto_run == Some(true);
                            current_task.verdict = Some("CHANGES_REQUESTED".to_string());
                            current_task.status = if scol_has_autorun {
                                Some("running".to_string())
                            } else {
                                Some("idle".to_string())
                            };
                            current_task.updated_at = Utc::now().timestamp_millis();
                            let failed_task = current_task.clone();
                            let _ = crate::db::db_upsert_task(root, &failed_task);
                            save_tasks_to_disk(root, &tasks).await.map_err(|e| e.to_string())?;

                            if let Some(app) = app_handle {
                                if let Some(state_ref) = app.try_state::<AppState>() {
                                    if let Ok(mut guard) = state_ref.tasks.lock() {
                                        if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, &failed_task.id)) {
                                            t.verdict = failed_task.verdict.clone();
                                            t.status = failed_task.status.clone();
                                            t.updated_at = failed_task.updated_at;
                                        }
                                    }
                                }
                                let _ = app.emit("task:moved", serde_json::json!({
                                    "task": &failed_task,
                                    "rejected": true,
                                    "reason": err_details
                                }));
                                let _ = app.emit("task:updated", serde_json::json!({
                                    "taskId": failed_task.id,
                                    "task": &failed_task
                                }));
                                let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));

                                if scol_has_autorun {
                                    let _ = app.emit("agent:autorun", serde_json::json!({
                                        "taskId": failed_task.id,
                                        "prompt": steer_content
                                    }));
                                }
                            }

                            return Err(format!(
                                "Хук on_leave '{}' отклонил переход (Exit code: {:?}):\n{}",
                                hook_rel, res.exit_code, err_details
                            ));
                        }

                        record_task_hook_result(
                            current_task,
                            &scol.id,
                            "on_leave",
                            hook_rel,
                            hook_started_at,
                            duration_ms,
                            true,
                            res.exit_code,
                            &res.stdout,
                            &res.stderr,
                            None,
                        );

                        if let Some(app) = app_handle {
                            let _ = app.emit("hook:finish", serde_json::json!({
                                "taskId": current_task.id,
                                "stageId": scol.id,
                                "hookType": "on_leave",
                                "success": true,
                                "exitCode": res.exit_code,
                                "stdout": res.stdout,
                                "stderr": res.stderr
                            }));
                        }
                        apply_worktree_from_hook_stdout(current_task, &res.stdout);
                    }
                    Err(e) => {
                        let duration_ms = (chrono::Utc::now().timestamp_millis() - hook_started_at).max(0) as u64;
                        let err_details = e.to_string();
                        record_task_hook_result(
                            current_task,
                            &scol.id,
                            "on_leave",
                            hook_rel,
                            hook_started_at,
                            duration_ms,
                            false,
                            None,
                            "",
                            &err_details,
                            Some(&err_details),
                        );
                        if let Some(app) = app_handle {
                            let _ = app.emit("hook:finish", serde_json::json!({
                                "taskId": current_task.id,
                                "stageId": scol.id,
                                "hookType": "on_leave",
                                "success": false,
                                "exitCode": None::<i32>,
                                "stdout": "",
                                "stderr": err_details,
                                "error": err_details
                            }));
                        }
                        let steer_content = format!(
                            "### [ОТКЛОНЕНО ХУКОМ / HOOK REJECTED: {}]\n\nОшибка запуска хука: {}\n\nИсправь указанную ошибку.",
                            hook_rel, err_details
                        );
                        let _ = append_task_dialog_activity(
                            root,
                            &current_task.id,
                            "steer",
                            &steer_content,
                            app_handle,
                        ).await;

                        let scol_has_autorun = scol.persona.is_some() || scol.auto_run == Some(true);
                        current_task.verdict = Some("CHANGES_REQUESTED".to_string());
                        current_task.status = if scol_has_autorun {
                            Some("running".to_string())
                        } else {
                            Some("idle".to_string())
                        };
                        current_task.updated_at = Utc::now().timestamp_millis();
                        let failed_task = current_task.clone();
                        let _ = crate::db::db_upsert_task(root, &failed_task);
                        save_tasks_to_disk(root, &tasks).await.map_err(|e| e.to_string())?;

                        if let Some(app) = app_handle {
                            if let Some(state_ref) = app.try_state::<AppState>() {
                                if let Ok(mut guard) = state_ref.tasks.lock() {
                                    if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, &failed_task.id)) {
                                        t.verdict = failed_task.verdict.clone();
                                        t.status = failed_task.status.clone();
                                        t.updated_at = failed_task.updated_at;
                                    }
                                }
                            }
                            let _ = app.emit("task:moved", serde_json::json!({
                                "task": &failed_task,
                                "rejected": true,
                                "reason": err_details
                            }));
                            let _ = app.emit("task:updated", serde_json::json!({
                                "taskId": failed_task.id,
                                "task": &failed_task
                            }));
                            let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));

                            if scol_has_autorun {
                                let _ = app.emit("agent:autorun", serde_json::json!({
                                    "taskId": failed_task.id,
                                    "prompt": steer_content
                                }));
                            }
                        }

                        return Err(format!("Хук on_leave '{}' ошибка исполнения: {}", hook_rel, err_details));
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
                let pipeline = load_pipeline_config(root)
                    .await
                    .unwrap_or_else(|_| crate::config::default_pipeline_config());
                let col_from_pipeline = pipeline
                    .columns
                    .iter()
                    .find(|c| stage_id_matches(&c.id, target_stage_id))
                    .cloned();

                match col_from_pipeline {
                    Some(c) => c,
                    None => {
                        let is_explicit_terminal_request = target_stage_id.trim().is_empty()
                            || stage_id_matches(target_stage_id, "ready")
                            || stage_id_matches(target_stage_id, "done")
                            || stage_id_matches(target_stage_id, "terminal");

                        let terminal_opt = if is_explicit_terminal_request {
                            effective_board.as_ref().and_then(|b| {
                                b.columns.iter().find(|c| c.terminal == Some(true) || stage_id_matches(&c.id, "ready") || stage_id_matches(&c.id, "done")).cloned()
                                    .or_else(|| b.columns.last().cloned())
                            }).or_else(|| {
                                pipeline.columns.iter().find(|c| {
                                    c.terminal == Some(true)
                                        || stage_id_matches(&c.id, "ready")
                                        || stage_id_matches(&c.id, "done")
                                }).cloned()
                            })
                        } else {
                            None
                        };

                        if let Some(term) = terminal_opt {
                            term
                        } else {
                            let mut available_stages: Vec<String> = Vec::new();
                            if let Some(ref b) = effective_board {
                                for c in &b.columns {
                                    if !available_stages.contains(&c.id) {
                                        available_stages.push(c.id.clone());
                                    }
                                }
                            }
                            for c in &pipeline.columns {
                                if !available_stages.contains(&c.id) {
                                    available_stages.push(c.id.clone());
                                }
                            }
                            return Err(format!(
                                "Target stage '{}' not found in active board or pipeline columns. Available: {:?}",
                                target_stage_id,
                                available_stages
                            ));
                        }
                    }
                }
            }
        }
    };
    let canonical_target_stage_id = target_col.id.clone();
    let is_backlog = target_col.is_intake == Some(true)
        || stage_id_matches(&canonical_target_stage_id, "backlog");
    let target_has_autorun = !is_backlog && (target_col.persona.is_some() || target_col.auto_run == Some(true));
    let was_running = current_task.status.as_deref() == Some("running");

    // Immediate Cancellation (§ 2 specifications):
    // If a task is moved to backlog / intake (or any stage without autoRun / persona, or user moves a running task):
    // When is_autonomous_advance is true, the calling agent is advancing autonomously and will terminate naturally via § 2.27.
    // Do NOT abort it mid-flight!
    if !is_autonomous_advance && (is_backlog || !target_has_autorun || was_running) {
        if let Some(app) = app_handle {
            if let Some(state_ref) = app.try_state::<AppState>() {
                let _ = internal_abort_agent(root, task_id, state_ref.inner(), Some(app)).await;
            }
        }
    }



    // Gatekeeper Sensor Verification: do not block initial move from backlog/intake to first actionable stage or new/empty worktree
    if let Some(sensor_cmd) = &target_col.gatekeeper_sensor {
        let is_source_intake = {
            let cols = if let Some(b) = &effective_board {
                b.columns.clone()
            } else if let Some(bc) = &boards_cfg {
                bc.boards.iter().find(|b| b.id == bc.active_board_id).map(|b| b.columns.clone()).unwrap_or_default()
            } else if let Ok(pipe) = load_pipeline_config(root).await {
                pipe.columns
            } else {
                Vec::new()
            };
            source_stage_id.is_empty()
                || cols
                    .iter()
                    .position(|c| stage_id_matches(&c.id, &source_stage_id))
                    .map(|idx| idx == 0 || cols[idx].is_intake == Some(true))
                    .unwrap_or(false)
        };

        let abs_wt_path = match current_task.worktree_path.as_deref() {
            Some(p) => {
                let pb = PathBuf::from(p);
                if pb.is_absolute() { pb } else { root.join(pb) }
            }
            None => root.to_path_buf(),
        };
        let effective_wt_path = if abs_wt_path.exists() {
            abs_wt_path.clone()
        } else {
            root.to_path_buf()
        };

        let wt_missing_or_empty = current_task.worktree_path.is_none()
            || !abs_wt_path.exists()
            || abs_wt_path.read_dir().map(|mut it| it.next().is_none()).unwrap_or(true);

        if is_source_intake {
            crate::logger::cvh_log(
                Some(root),
                "INFO",
                "GATEKEEPER",
                &format!(
                    "Skipping gatekeeper sensor '{}' for initial intake transition of task {} from '{}' to '{}'",
                    sensor_cmd, current_task.id, source_stage_id, canonical_target_stage_id
                ),
            );
        } else if wt_missing_or_empty {
            crate::logger::cvh_log(
                Some(root),
                "INFO",
                "GATEKEEPER",
                &format!(
                    "Skipping gatekeeper sensor '{}' for task {} because worktree does not exist or is empty",
                    sensor_cmd, current_task.id
                ),
            );
        } else {
            crate::logger::cvh_log(
                Some(root),
                "INFO",
                "GATEKEEPER",
                &format!("Executing gatekeeper sensor '{}' for task '{}' in {:?}", sensor_cmd, current_task.id, effective_wt_path),
            );
            let sensor_res = execute_sensor_command(sensor_cmd, &effective_wt_path, None)
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
                crate::logger::cvh_log_with_details(
                    Some(root),
                    "WARN",
                    "GATEKEEPER",
                    &format!("Gatekeeper sensor '{}' REJECTED transition for task '{}' (exit code: {:?})", sensor_cmd, current_task.id, sensor_res.exit_code),
                    Some(&details),
                );

                let steer_content = format!(
                    "### [ОТКЛОНЕНО СЕНСОРОМ / SENSOR REJECTED: {}]\n\nGatekeeper-сенсор целевой колонки '{}' завершился с ошибкой (код {:?}).\n\n**Лог ошибки:**\n```\n{}\n```\n\nИсправь указанную ошибку.",
                    sensor_cmd, canonical_target_stage_id, sensor_res.exit_code, details
                );
                let _ = append_task_dialog_activity(
                    root,
                    &current_task.id,
                    "steer",
                    &steer_content,
                    app_handle,
                ).await;

                let scol_has_autorun = source_col.as_ref().map(|c| c.persona.is_some() || c.auto_run == Some(true)).unwrap_or(false);
                current_task.verdict = Some("CHANGES_REQUESTED".to_string());
                current_task.status = if scol_has_autorun {
                    Some("running".to_string())
                } else {
                    Some("idle".to_string())
                };
                current_task.updated_at = Utc::now().timestamp_millis();
                let failed_task = current_task.clone();
                let _ = crate::db::db_upsert_task(root, &failed_task);
                save_tasks_to_disk(root, &tasks).await.map_err(|e| e.to_string())?;

                if let Some(app) = app_handle {
                    if let Some(state_ref) = app.try_state::<AppState>() {
                        if let Ok(mut guard) = state_ref.tasks.lock() {
                            if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, &failed_task.id)) {
                                t.verdict = failed_task.verdict.clone();
                                t.status = failed_task.status.clone();
                                t.updated_at = failed_task.updated_at;
                            }
                        }
                    }
                    let _ = app.emit("task:moved", serde_json::json!({
                        "task": &failed_task,
                        "rejected": true,
                        "reason": details
                    }));
                    let _ = app.emit("task:updated", serde_json::json!({
                        "taskId": failed_task.id,
                        "task": &failed_task
                    }));
                    let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));

                    if scol_has_autorun {
                        let _ = app.emit("agent:autorun", serde_json::json!({
                            "taskId": failed_task.id,
                            "prompt": steer_content
                        }));
                    }
                }

                return Err(format!(
                    "Gatekeeper Sensor '{}' rejected transition (Exit code: {:?}):{}",
                    sensor_cmd, sensor_res.exit_code, details
                ));
            }
            crate::logger::cvh_log(
                Some(root),
                "INFO",
                "GATEKEEPER",
                &format!("Gatekeeper sensor '{}' PASSED for task '{}'", sensor_cmd, current_task.id),
            );
        }
    }

    // Trigger on_enter hook if configured
    if let Some(hook_rel) = target_col.hooks.get("on_enter").cloned() {
        let hook_path = crate::config::resolve_hook_path(
            root,
            target_col.persona.as_deref(),
            &hook_rel,
        );
        if hook_path.exists() {
            if let Some(app) = app_handle {
                let _ = app.emit("hook:start", serde_json::json!({
                    "taskId": current_task.id,
                    "stageId": canonical_target_stage_id,
                    "hookType": "on_enter",
                    "hookName": hook_rel
                }));
            }
            let hook_started_at = chrono::Utc::now().timestamp_millis();
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
            let hook_res = execute_hook(&hook_path, root, &ctx, None).await;
            match hook_res {
                Ok(res) => {
                    let duration_ms = (chrono::Utc::now().timestamp_millis() - hook_started_at).max(0) as u64;
                    if !res.success {
                        let err_details = if !res.stderr.trim().is_empty() {
                            res.stderr.trim().to_string()
                        } else if !res.stdout.trim().is_empty() {
                            res.stdout.trim().to_string()
                        } else {
                            format!("Exit code {:?}", res.exit_code)
                        };
                        record_task_hook_result(
                            current_task,
                            &canonical_target_stage_id,
                            "on_enter",
                            &hook_rel,
                            hook_started_at,
                            duration_ms,
                            false,
                            res.exit_code,
                            &res.stdout,
                            &res.stderr,
                            Some(&err_details),
                        );

                        if let Some(app) = app_handle {
                            let _ = app.emit("hook:finish", serde_json::json!({
                                "taskId": current_task.id,
                                "stageId": canonical_target_stage_id,
                                "hookType": "on_enter",
                                "success": false,
                                "exitCode": res.exit_code,
                                "stdout": res.stdout,
                                "stderr": res.stderr,
                                "error": err_details
                            }));
                        }

                        // Steer message to task dialogue
                        let steer_content = format!(
                            "### [ОТКЛОНЕНО ХУКОМ / HOOK REJECTED: {}]\n\nХук на входе в колонку '{}' завершился с ошибкой (код {:?}).\n\n**Лог ошибки:**\n```\n{}\n```\n\nИсправь указанную ошибку.",
                            hook_rel, canonical_target_stage_id, res.exit_code, err_details
                        );
                        let _ = append_task_dialog_activity(
                            root,
                            &current_task.id,
                            "steer",
                            &steer_content,
                            app_handle,
                        ).await;

                        // Task stays in source stage!
                        let scol_has_autorun = source_col.as_ref().map(|c| c.persona.is_some() || c.auto_run == Some(true)).unwrap_or(false);
                        current_task.verdict = Some("CHANGES_REQUESTED".to_string());
                        current_task.status = if scol_has_autorun {
                            Some("running".to_string())
                        } else {
                            Some("idle".to_string())
                        };
                        current_task.updated_at = Utc::now().timestamp_millis();
                        let failed_task = current_task.clone();
                        let _ = crate::db::db_upsert_task(root, &failed_task);
                        save_tasks_to_disk(root, &tasks).await.map_err(|e| e.to_string())?;

                        if let Some(app) = app_handle {
                            if let Some(state_ref) = app.try_state::<AppState>() {
                                if let Ok(mut guard) = state_ref.tasks.lock() {
                                    if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, &failed_task.id)) {
                                        t.verdict = failed_task.verdict.clone();
                                        t.status = failed_task.status.clone();
                                        t.updated_at = failed_task.updated_at;
                                    }
                                }
                            }
                            let _ = app.emit("task:moved", serde_json::json!({
                                "task": &failed_task,
                                "rejected": true,
                                "reason": err_details
                            }));
                            let _ = app.emit("task:updated", serde_json::json!({
                                "taskId": failed_task.id,
                                "task": &failed_task
                            }));
                            let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));

                            if scol_has_autorun {
                                let _ = app.emit("agent:autorun", serde_json::json!({
                                    "taskId": failed_task.id,
                                    "prompt": steer_content
                                }));
                            }
                        }

                        return Err(format!(
                            "Хук on_enter '{}' отклонил переход (Exit code: {:?}):\n{}",
                            hook_rel, res.exit_code, err_details
                        ));
                    }

                    record_task_hook_result(
                        current_task,
                        &canonical_target_stage_id,
                        "on_enter",
                        &hook_rel,
                        hook_started_at,
                        duration_ms,
                        true,
                        res.exit_code,
                        &res.stdout,
                        &res.stderr,
                        None,
                    );

                    if let Some(app) = app_handle {
                        let _ = app.emit("hook:finish", serde_json::json!({
                            "taskId": current_task.id,
                            "stageId": canonical_target_stage_id,
                            "hookType": "on_enter",
                            "success": true,
                            "exitCode": res.exit_code,
                            "stdout": res.stdout,
                            "stderr": res.stderr
                        }));
                    }
                    apply_worktree_from_hook_stdout(current_task, &res.stdout);
                }
                Err(e) => {
                    let duration_ms = (chrono::Utc::now().timestamp_millis() - hook_started_at).max(0) as u64;
                    let err_details = e.to_string();
                    record_task_hook_result(
                        current_task,
                        &canonical_target_stage_id,
                        "on_enter",
                        &hook_rel,
                        hook_started_at,
                        duration_ms,
                        false,
                        None,
                        "",
                        &err_details,
                        Some(&err_details),
                    );
                    if let Some(app) = app_handle {
                        let _ = app.emit("hook:finish", serde_json::json!({
                            "taskId": current_task.id,
                            "stageId": canonical_target_stage_id,
                            "hookType": "on_enter",
                            "success": false,
                            "exitCode": None::<i32>,
                            "stdout": "",
                            "stderr": err_details,
                            "error": err_details
                        }));
                    }
                    let steer_content = format!(
                        "### [ОТКЛОНЕНО ХУКОМ / HOOK REJECTED: {}]\n\nОшибка запуска входного хука: {}\n\nИсправь указанную ошибку.",
                        hook_rel, err_details
                    );
                    let _ = append_task_dialog_activity(
                        root,
                        &current_task.id,
                        "steer",
                        &steer_content,
                        app_handle,
                    ).await;

                    let scol_has_autorun = source_col.as_ref().map(|c| c.persona.is_some() || c.auto_run == Some(true)).unwrap_or(false);
                    current_task.verdict = Some("CHANGES_REQUESTED".to_string());
                    current_task.status = if scol_has_autorun {
                        Some("running".to_string())
                    } else {
                        Some("idle".to_string())
                    };
                    current_task.updated_at = Utc::now().timestamp_millis();
                    let failed_task = current_task.clone();
                    let _ = crate::db::db_upsert_task(root, &failed_task);
                    save_tasks_to_disk(root, &tasks).await.map_err(|e| e.to_string())?;

                    if let Some(app) = app_handle {
                        if let Some(state_ref) = app.try_state::<AppState>() {
                            if let Ok(mut guard) = state_ref.tasks.lock() {
                                if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, &failed_task.id)) {
                                    t.verdict = failed_task.verdict.clone();
                                    t.status = failed_task.status.clone();
                                    t.updated_at = failed_task.updated_at;
                                }
                            }
                        }
                        let _ = app.emit("task:moved", serde_json::json!({
                            "task": &failed_task,
                            "rejected": true,
                            "reason": err_details
                        }));
                        let _ = app.emit("task:updated", serde_json::json!({
                            "taskId": failed_task.id,
                            "task": &failed_task
                        }));
                        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));

                        if scol_has_autorun {
                            let _ = app.emit("agent:autorun", serde_json::json!({
                                "taskId": failed_task.id,
                                "prompt": steer_content
                            }));
                        }
                    }

                    return Err(format!("Хук on_enter '{}' ошибка исполнения: {}", hook_rel, err_details));
                }
            }
        }
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
        current_task.verdict = verdict.clone();
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
    if is_terminal_col || current_task.status.as_deref() == Some("done") {
        current_task.waiting_for_subtask_id = None;
    }
    current_task.updated_at = Utc::now().timestamp_millis();

    if old_stage != canonical_target_stage_id {
        let cur_id = tasks[task_idx].id.clone();
        let max_pos = tasks
            .iter()
            .filter(|t| !task_id_matches(&t.id, &cur_id) && stage_id_matches(&t.stage_id, &canonical_target_stage_id))
            .filter_map(|t| t.position)
            .fold(0.0f64, f64::max);
        tasks[task_idx].position = Some(max_pos + 1000.0);
    } else if tasks[task_idx].position.is_none() {
        tasks[task_idx].position = Some(0.0);
    }

    let updated = tasks[task_idx].clone();
    let _ = crate::db::db_upsert_task(root, &updated);
    let parent_id_opt = updated.parent_id.clone();

    // If a top-level parent task moves to terminal, ensure all child subtasks are also marked done (§ 3)
    if is_terminal_col && parent_id_opt.is_none() {
        let cur_id = updated.id.clone();
        for t in tasks.iter_mut() {
            if task_id_matches(&t.id, &cur_id) {
                t.waiting_for_subtask_id = None;
                t.status = Some("done".to_string());
                t.updated_at = Utc::now().timestamp_millis();
            }
            if t.parent_id.as_deref() == Some(&cur_id) {
                t.status = Some("done".to_string());
                t.updated_at = Utc::now().timestamp_millis();
            }
        }
    }

    // Relational symmetry (§ 2.7 & § 2.22):
    let mut parent_moved_events: Vec<serde_json::Value> = Vec::new();
    let mut parent_created_tasks: Vec<Task> = Vec::new();
    let mut parent_autorun_ids: Vec<String> = Vec::new();

    if verdict.as_deref() == Some("CHANGES_REQUESTED") {
        // 1. When CHANGES_REQUESTED is emitted by a subtask, unlock the parent task in its development stage:
        if let Some(ref pid) = parent_id_opt {
            if let Some(parent_idx) = tasks.iter().position(|t| &t.id == pid) {
                if tasks[parent_idx].status.as_deref() == Some("waiting_subtask")
                    || tasks[parent_idx].waiting_for_subtask_id.as_deref() == Some(task_id)
                {
                    tasks[parent_idx].status = Some("idle".to_string());
                    tasks[parent_idx].verdict = Some("CHANGES_REQUESTED".to_string());
                    tasks[parent_idx].waiting_for_subtask_id = None;
                    tasks[parent_idx].updated_at = Utc::now().timestamp_millis();
                    let parent_clone = tasks[parent_idx].clone();
                    parent_moved_events.push(serde_json::json!({
                        "taskId": pid,
                        "fromStage": &parent_clone.stage_id,
                        "toStage": &parent_clone.stage_id,
                        "stageId": &parent_clone.stage_id,
                        "status": parent_clone.status,
                        "verdict": parent_clone.verdict,
                        "task": &parent_clone
                    }));
                }
            }
        }
    } else if is_terminal_col {
        // 2. POSITIVE APPROVAL / TERMINAL ADVANCE:
        // When a child subtask advances to a terminal column (ready/done),
        // propagate advancement to the parent task (and grandparents recursively)
        // advancing them past the subtask column into the next pipeline stage (§ 2.2, § 2.7 & § 2.22).
        let pipeline_opt = load_pipeline_config(root).await.ok();
        let parent_res = propagate_parent_subtask_advance(
            &mut tasks,
            task_id,
            &old_stage,
            verdict.as_deref(),
            boards_cfg.as_ref(),
            pipeline_opt.as_ref(),
        );
        parent_moved_events = parent_res.moved_events;
        parent_created_tasks = parent_res.created_tasks;
        parent_autorun_ids = parent_res.autorun_task_ids;
    }

    if let Some(app) = app_handle {
        if let Some(state_ref) = app.try_state::<AppState>() {
            if let Ok(mut guard) = state_ref.tasks.lock() {
                for mem_t in guard.iter() {
                    if !tasks.iter().any(|t| task_id_matches(&t.id, &mem_t.id)) {
                        tasks.push(mem_t.clone());
                    }
                }
                *guard = tasks.clone();
            }
        }
    }
    for ctask in &parent_created_tasks {
        let _ = crate::db::db_upsert_task(root, ctask);
    }

    if let Err(e) = save_tasks_to_disk(root, &tasks).await {
        crate::logger::cvh_log(
            Some(root),
            "ERROR",
            "MOVE_STAGE",
            &format!("Failed to save tasks to disk after moving task '{}': {}", task_id, e),
        );
        return Err(e);
    }
    crate::logger::cvh_log_with_details(
        Some(root),
        "INFO",
        "MOVE_STAGE",
        &format!(
            "Task '{}' stage successfully updated from '{}' to '{}' (status: {:?}, board: {:?})",
            task_id, old_stage, canonical_target_stage_id, updated.status, updated.board_id
        ),
        Some(&format!("worktree={:?}, branch={:?}", updated.worktree_path, updated.branch)),
    );

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
        for ctask in &parent_created_tasks {
            let _ = app.emit("task:created", serde_json::json!({ "task": ctask }));
            let _ = app.emit("task:updated", serde_json::json!({ "task": ctask }));
        }
        for p_event in &parent_moved_events {
            let _ = app.emit("task:moved", p_event);
            if let Some(ptask) = p_event.get("task") {
                let _ = app.emit("task:updated", serde_json::json!({ "task": ptask }));
            }
        }
        if target_has_autorun {
            let _ = app.emit("agent:autorun", serde_json::json!({ "taskId": task_id }));
        }
        for auto_pid in parent_autorun_ids {
            let _ = app.emit("agent:autorun", serde_json::json!({ "taskId": auto_pid }));
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
    board_id: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    let root = state.get_root();
    crate::logger::cvh_log_with_details(
        Some(&root),
        "INFO",
        "MOVE_STAGE",
        &format!(
            "move_task_stage IPC called: task_id='{}', target_stage='{}', board_id={:?}, verdict={:?}",
            task_id, target_stage_id, board_id, verdict
        ),
        Some(&format!("root='{}'", root.display())),
    );
    let is_intake = stage_id_matches(&target_stage_id, "backlog") || {
        let boards_cfg = crate::config::load_boards_config(&root).await.ok();
        boards_cfg.as_ref().is_some_and(|cfg| {
            let active = cfg.boards.iter().find(|b| b.id == cfg.active_board_id);
            active.is_some_and(|b| {
                b.columns.iter().find(|c| stage_id_matches(&c.id, &target_stage_id)).is_some_and(|c| c.is_intake == Some(true))
            })
        })
    };
    let is_running_or_active = {
        if let Ok(map) = state.active_agents.lock() {
            let tid_trimmed = task_id.trim();
            map.keys().any(|k| {
                task_id_matches(k, tid_trimmed)
                    || k.to_ascii_lowercase().starts_with(&format!("{}.", tid_trimmed.to_ascii_lowercase()))
                    || k.to_ascii_lowercase().starts_with(&format!("{}:sub:", tid_trimmed.to_ascii_lowercase()))
                    || k.to_ascii_lowercase().starts_with(&format!("{}.sub-", tid_trimmed.to_ascii_lowercase()))
            })
        } else {
            false
        }
    };

    if is_intake || is_running_or_active {
        let _ = internal_abort_agent(&root, &task_id, &state, Some(&app_handle)).await;
    }

    internal_move_task_stage(
        &root,
        &task_id,
        &target_stage_id,
        verdict,
        None,
        handover_summary,
        Some(&app_handle),
        false,
        board_id,
    )
    .await
}

#[tauri::command]
pub async fn reorder_task(
    task_id: String,
    target_stage_id: String,
    new_index: usize,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    let root = state.get_root();
    let mut tasks = load_tasks_from_disk(&root).await;

    let task_idx = tasks
        .iter()
        .position(|t| task_id_matches(&t.id, &task_id))
        .ok_or_else(|| format!("Task {} not found", task_id))?;

    let old_stage_id = tasks[task_idx].stage_id.clone();
    let is_stage_change = !stage_id_matches(&old_stage_id, &target_stage_id);

    if is_stage_change {
        let _ = internal_move_task_stage(
            &root,
            &task_id,
            &target_stage_id,
            None,
            None,
            None,
            Some(&app_handle),
            false,
            None,
        )
        .await?;
        tasks = load_tasks_from_disk(&root).await;
    }

    let mut stage_tuples: Vec<(String, f64, i64)> = tasks
        .iter()
        .filter(|t| stage_id_matches(&t.stage_id, &target_stage_id) && !task_id_matches(&t.id, &task_id))
        .map(|t| (t.id.clone(), t.position.unwrap_or(0.0), t.created_at))
        .collect();

    stage_tuples.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.2.cmp(&b.2))
    });

    let mut ordered_ids: Vec<String> = stage_tuples.into_iter().map(|(id, _, _)| id).collect();
    let insert_pos = new_index.min(ordered_ids.len());
    ordered_ids.insert(insert_pos, task_id.clone());

    let now = Utc::now().timestamp_millis();
    let mut updated_task = None;

    for (idx, tid) in ordered_ids.iter().enumerate() {
        let new_pos = (idx as f64) * 1000.0;
        if let Some(t) = tasks.iter_mut().find(|t| task_id_matches(&t.id, tid)) {
            t.position = Some(new_pos);
            t.updated_at = now;
            let _ = crate::db::db_upsert_task(&root, t);
            if task_id_matches(&t.id, &task_id) {
                updated_task = Some(t.clone());
            }
        }
    }

    save_tasks_to_disk(&root, &tasks).await?;

    if let Ok(mut guard) = state.tasks.lock() {
        *guard = tasks.clone();
    }

    let _ = app_handle.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
    if let Some(ref ut) = updated_task {
        let _ = app_handle.emit("task:updated", serde_json::json!({ "taskId": ut.id, "task": ut }));
    }

    updated_task.ok_or_else(|| format!("Failed to reorder task {}", task_id))
}

pub async fn internal_advance_task(
    root: &Path,
    task_id: &str,
    summary: &str,
    app_handle: Option<&AppHandle>,
) -> Result<String, String> {
    internal_advance_task_to_stage(root, task_id, None, summary, app_handle).await
}

async fn handle_advance_hook_or_sensor_rejection(
    root: &Path,
    task_id: &str,
    scol: Option<&crate::config::PipelineColumn>,
    steer_content: &str,
    error_msg: String,
    app_handle: Option<&AppHandle>,
) -> Result<String, String> {
    let _ = append_task_dialog_activity(
        root,
        task_id,
        "steer",
        steer_content,
        app_handle,
    )
    .await;

    let mut tasks = load_tasks_from_disk(root).await;
    let scol_has_autorun = scol.map(|c| c.persona.is_some() || c.auto_run == Some(true)).unwrap_or(true);
    let updated_status = if scol_has_autorun {
        Some("running".to_string())
    } else {
        Some("idle".to_string())
    };

    let mut failed_task = None;
    if let Some(t) = tasks.iter_mut().find(|t| task_id_matches(&t.id, task_id)) {
        t.verdict = Some("CHANGES_REQUESTED".to_string());
        t.status = updated_status.clone();
        t.updated_at = Utc::now().timestamp_millis();
        let _ = crate::db::db_upsert_task(root, t);
        failed_task = Some(t.clone());
    }
    let _ = save_tasks_to_disk(root, &tasks).await;

    if let Some(app) = app_handle {
        if let Some(state_ref) = app.try_state::<AppState>() {
            if let Ok(mut guard) = state_ref.tasks.lock() {
                if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, task_id)) {
                    t.verdict = Some("CHANGES_REQUESTED".to_string());
                    t.status = updated_status.clone();
                    t.updated_at = Utc::now().timestamp_millis();
                }
            }
        }
        if let Some(ref ft) = failed_task {
            let _ = app.emit("task:updated", serde_json::json!({ "taskId": ft.id, "task": ft }));
        }
        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
        if scol_has_autorun {
            let _ = app.emit("agent:autorun", serde_json::json!({
                "taskId": task_id,
                "prompt": steer_content,
            }));
        }
    }

    Err(error_msg)
}

pub async fn internal_advance_task_to_stage(
    root: &Path,
    task_id: &str,
    target_stage_override: Option<&str>,
    summary: &str,
    app_handle: Option<&AppHandle>,
) -> Result<String, String> {
    let root_resolved = resolve_actual_repo_root(root);
    let root = &root_resolved;
    let tasks = load_tasks_from_disk(root).await;
    let mut current_task = tasks
        .iter()
        .find(|t| task_id_matches(&t.id, task_id))
        .cloned()
        .ok_or_else(|| format!("Task {} not found", task_id))?;

    let boards_cfg = load_boards_config(root).await.ok();

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

    let target_col = if let Some(target_stage_id) = target_stage_override {
        columns
            .iter()
            .find(|c| stage_id_matches(&c.id, target_stage_id))
            .cloned()
            .ok_or_else(|| {
                let available_stages: Vec<&str> = columns.iter().map(|c| c.id.as_str()).collect();
                format!(
                    "Target stage '{}' not found in columns. Available stages: {:?}",
                    target_stage_id,
                    available_stages
                )
            })?
    } else {
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

        columns[curr_idx + 1].clone()
    };

    // Universal Relay Progression (§ 2.2 & § 2.7):
    let is_terminal = target_col.terminal == Some(true)
        || stage_id_matches(&target_col.id, "ready")
        || stage_id_matches(&target_col.id, "done");

    let source_col = columns
        .iter()
        .find(|c| stage_id_matches(&c.id, &current_task.stage_id))
        .cloned();

    // 1. Check and execute on_leave / on_exit hook of source stage
    if let Some(ref sc) = source_col {
        if let Some(hook_rel) = sc.hooks.get("on_leave").or_else(|| sc.hooks.get("on_exit")) {
            let hook_path = crate::config::resolve_hook_path(root, sc.persona.as_deref(), hook_rel);
            if hook_path.exists() {
                if let Some(app) = app_handle {
                    let _ = app.emit("hook:start", serde_json::json!({
                        "taskId": current_task.id,
                        "stageId": sc.id,
                        "hookType": "on_leave",
                        "hookName": hook_rel
                    }));
                }
                let hook_started_at = chrono::Utc::now().timestamp_millis();
                let ctx = HookContext {
                    task_id: current_task.id.clone(),
                    task_title: current_task.title.clone(),
                    worktree_path: current_task.worktree_path.clone().unwrap_or_default(),
                    branch: current_task.branch.clone().unwrap_or_default(),
                    verdict: current_task.verdict.clone(),
                    stage_id: Some(sc.id.clone()),
                    board_id: current_task.board_id.clone(),
                    extra_env: HashMap::new(),
                };
                let hook_res = execute_hook(&hook_path, root, &ctx, None).await;
                match hook_res {
                    Ok(res) => {
                        let duration_ms = (chrono::Utc::now().timestamp_millis() - hook_started_at).max(0) as u64;
                        if !res.success {
                            let err_details = if !res.stderr.trim().is_empty() {
                                res.stderr.trim().to_string()
                            } else if !res.stdout.trim().is_empty() {
                                res.stdout.trim().to_string()
                            } else {
                                format!("Exit code {:?}", res.exit_code)
                            };
                            record_task_hook_result(
                                &mut current_task,
                                &sc.id,
                                "on_leave",
                                hook_rel,
                                hook_started_at,
                                duration_ms,
                                false,
                                res.exit_code,
                                &res.stdout,
                                &res.stderr,
                                Some(&err_details),
                            );
                            let _ = crate::db::db_upsert_task(root, &current_task);
                            if let Some(app) = app_handle {
                                let _ = app.emit("hook:finish", serde_json::json!({
                                    "taskId": current_task.id,
                                    "stageId": sc.id,
                                    "hookType": "on_leave",
                                    "success": false,
                                    "exitCode": res.exit_code,
                                    "stdout": res.stdout,
                                    "stderr": res.stderr,
                                    "error": err_details
                                }));
                            }
                            let steer_content = format!(
                                "### [ОТКЛОНЕНО ХУКОМ / HOOK REJECTED: {}]\n\nХук на выходе из колонки '{}' завершился с ошибкой (код {:?}).\n\n**Лог ошибки:**\n```\n{}\n```\n\nИсправь указанную ошибку.",
                                hook_rel, sc.id, res.exit_code, err_details
                            );
                            let err_msg = format!("Хук on_leave '{}' отклонил переход (Exit code: {:?}):\n{}", hook_rel, res.exit_code, err_details);
                            return handle_advance_hook_or_sensor_rejection(
                                root,
                                task_id,
                                Some(sc),
                                &steer_content,
                                err_msg,
                                app_handle,
                            ).await;
                        }
                        record_task_hook_result(
                            &mut current_task,
                            &sc.id,
                            "on_leave",
                            hook_rel,
                            hook_started_at,
                            duration_ms,
                            true,
                            res.exit_code,
                            &res.stdout,
                            &res.stderr,
                            None,
                        );
                        let _ = crate::db::db_upsert_task(root, &current_task);
                        if let Some(app) = app_handle {
                            let _ = app.emit("hook:finish", serde_json::json!({
                                "taskId": current_task.id,
                                "stageId": sc.id,
                                "hookType": "on_leave",
                                "success": true,
                                "exitCode": res.exit_code,
                                "stdout": res.stdout,
                                "stderr": res.stderr
                            }));
                        }
                        apply_worktree_from_hook_stdout(&mut current_task, &res.stdout);
                        let _ = crate::db::db_upsert_task(root, &current_task);
                        if let Some(app) = app_handle {
                            if let Some(state_ref) = app.try_state::<AppState>() {
                                if let Ok(mut guard) = state_ref.tasks.lock() {
                                    if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, &current_task.id)) {
                                        t.worktree_path = current_task.worktree_path.clone();
                                        t.branch = current_task.branch.clone();
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let duration_ms = (chrono::Utc::now().timestamp_millis() - hook_started_at).max(0) as u64;
                        let err_details = e.to_string();
                        record_task_hook_result(
                            &mut current_task,
                            &sc.id,
                            "on_leave",
                            hook_rel,
                            hook_started_at,
                            duration_ms,
                            false,
                            None,
                            "",
                            &err_details,
                            Some(&err_details),
                        );
                        let _ = crate::db::db_upsert_task(root, &current_task);
                        if let Some(app) = app_handle {
                            let _ = app.emit("hook:finish", serde_json::json!({
                                "taskId": current_task.id,
                                "stageId": sc.id,
                                "hookType": "on_leave",
                                "success": false,
                                "exitCode": None::<i32>,
                                "stdout": "",
                                "stderr": err_details,
                                "error": err_details
                            }));
                        }
                        let steer_content = format!(
                            "### [ОТКЛОНЕНО ХУКОМ / HOOK REJECTED: {}]\n\nОшибка запуска хука на выходе из колонки '{}': {}\n\nИсправь указанную ошибку.",
                            hook_rel, sc.id, err_details
                        );
                        let err_msg = format!("Хук on_leave '{}' ошибка исполнения: {}", hook_rel, err_details);
                        return handle_advance_hook_or_sensor_rejection(
                            root,
                            task_id,
                            Some(sc),
                            &steer_content,
                            err_msg,
                            app_handle,
                        ).await;
                    }
                }
            }
        }
    }

    // 2. Check and execute gatekeeper sensor of target column
    if let Some(sensor_cmd) = &target_col.gatekeeper_sensor {
        let abs_wt_path = match current_task.worktree_path.as_deref() {
            Some(p) => {
                let pb = PathBuf::from(p);
                if pb.is_absolute() { pb } else { root.join(pb) }
            }
            None => root.to_path_buf(),
        };
        let effective_wt_path = if abs_wt_path.exists() {
            abs_wt_path.clone()
        } else {
            root.to_path_buf()
        };

        let source_idx = columns
            .iter()
            .position(|c| stage_id_matches(&c.id, &current_task.stage_id));
        let is_source_intake = current_task.stage_id.is_empty()
            || source_idx == Some(0)
            || source_col
                .as_ref()
                .map(|c| c.is_intake == Some(true))
                .unwrap_or(false);

        let wt_missing_or_empty = current_task.worktree_path.is_none()
            || !abs_wt_path.exists()
            || abs_wt_path.read_dir().map(|mut it| it.next().is_none()).unwrap_or(true);

        if is_source_intake {
            crate::logger::cvh_log(
                Some(root),
                "INFO",
                "GATEKEEPER",
                &format!(
                    "Skipping gatekeeper sensor '{}' for initial intake transition of task {} from '{}' to '{}'",
                    sensor_cmd, current_task.id, current_task.stage_id, target_col.id
                ),
            );
        } else if wt_missing_or_empty {
            crate::logger::cvh_log(
                Some(root),
                "INFO",
                "GATEKEEPER",
                &format!(
                    "Skipping gatekeeper sensor '{}' for task {} because worktree does not exist or is empty",
                    sensor_cmd, current_task.id
                ),
            );
        } else {
            crate::logger::cvh_log(
                Some(root),
                "INFO",
                "GATEKEEPER",
                &format!("Executing gatekeeper sensor '{}' for task '{}' in {:?}", sensor_cmd, current_task.id, effective_wt_path),
            );
            let sensor_res = execute_sensor_command(sensor_cmd, &effective_wt_path, None).await;
            match sensor_res {
                Ok(res) => {
                    if !res.success {
                        let stdout_trim = res.stdout.trim();
                        let stderr_trim = res.stderr.trim();
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
                        crate::logger::cvh_log_with_details(
                            Some(root),
                            "WARN",
                            "GATEKEEPER",
                            &format!("Gatekeeper sensor '{}' REJECTED advance for task '{}' (exit code: {:?})", sensor_cmd, current_task.id, res.exit_code),
                            Some(&details),
                        );
                        let steer_content = format!(
                            "### [ОТКЛОНЕНО СЕНСОРОМ / SENSOR REJECTED: {}]\n\nGatekeeper-сенсор целевой колонки '{}' завершился с ошибкой (код {:?}).\n\n**Лог ошибки:**\n```\n{}\n```\n\nИсправь указанную ошибку.",
                            sensor_cmd, target_col.id, res.exit_code, details
                        );
                        let err_msg = format!("Gatekeeper Sensor '{}' rejected transition (Exit code: {:?}):{}", sensor_cmd, res.exit_code, details);
                        return handle_advance_hook_or_sensor_rejection(
                            root,
                            task_id,
                            source_col.as_ref(),
                            &steer_content,
                            err_msg,
                            app_handle,
                        ).await;
                    }
                    crate::logger::cvh_log(
                        Some(root),
                        "INFO",
                        "GATEKEEPER",
                        &format!("Gatekeeper sensor '{}' PASSED for task '{}'", sensor_cmd, current_task.id),
                    );
                }
                Err(e) => {
                    let err_details = e.to_string();
                    let steer_content = format!(
                        "### [ОТКЛОНЕНО СЕНСОРОМ / SENSOR REJECTED: {}]\n\nОшибка исполнения gatekeeper-сенсора целевой колонки '{}': {}\n\nИсправь указанную ошибку.",
                        sensor_cmd, target_col.id, err_details
                    );
                    let err_msg = format!("Gatekeeper Sensor '{}' execution failed: {}", sensor_cmd, err_details);
                    return handle_advance_hook_or_sensor_rejection(
                        root,
                        task_id,
                        source_col.as_ref(),
                        &steer_content,
                        err_msg,
                        app_handle,
                    ).await;
                }
            }
        }
    }

    // 3. Check and execute on_enter hook of target column
    if let Some(hook_rel) = target_col.hooks.get("on_enter").cloned() {
        let hook_path = crate::config::resolve_hook_path(
            root,
            target_col.persona.as_deref(),
            &hook_rel,
        );
        if hook_path.exists() {
            if let Some(app) = app_handle {
                let _ = app.emit("hook:start", serde_json::json!({
                    "taskId": current_task.id,
                    "stageId": target_col.id,
                    "hookType": "on_enter",
                    "hookName": hook_rel
                }));
            }
            let hook_started_at = chrono::Utc::now().timestamp_millis();
            let ctx = HookContext {
                task_id: current_task.id.clone(),
                task_title: current_task.title.clone(),
                worktree_path: current_task.worktree_path.clone().unwrap_or_default(),
                branch: current_task.branch.clone().unwrap_or_default(),
                verdict: current_task.verdict.clone(),
                stage_id: Some(target_col.id.clone()),
                board_id: current_task.board_id.clone(),
                extra_env: HashMap::new(),
            };
            let hook_res = execute_hook(&hook_path, root, &ctx, None).await;
            match hook_res {
                Ok(res) => {
                    let duration_ms = (chrono::Utc::now().timestamp_millis() - hook_started_at).max(0) as u64;
                    if !res.success {
                        let err_details = if !res.stderr.trim().is_empty() {
                            res.stderr.trim().to_string()
                        } else if !res.stdout.trim().is_empty() {
                            res.stdout.trim().to_string()
                        } else {
                            format!("Exit code {:?}", res.exit_code)
                        };
                        record_task_hook_result(
                            &mut current_task,
                            &target_col.id,
                            "on_enter",
                            &hook_rel,
                            hook_started_at,
                            duration_ms,
                            false,
                            res.exit_code,
                            &res.stdout,
                            &res.stderr,
                            Some(&err_details),
                        );
                        let _ = crate::db::db_upsert_task(root, &current_task);
                        if let Some(app) = app_handle {
                            let _ = app.emit("hook:finish", serde_json::json!({
                                "taskId": current_task.id,
                                "stageId": target_col.id,
                                "hookType": "on_enter",
                                "success": false,
                                "exitCode": res.exit_code,
                                "stdout": res.stdout,
                                "stderr": res.stderr,
                                "error": err_details
                            }));
                        }
                        let steer_content = format!(
                            "### [ОТКЛОНЕНО ХУКОМ / HOOK REJECTED: {}]\n\nХук на входе в колонку '{}' завершился с ошибкой (код {:?}).\n\n**Лог ошибки:**\n```\n{}\n```\n\nИсправь указанную ошибку.",
                            hook_rel, target_col.id, res.exit_code, err_details
                        );
                        let err_msg = format!("Хук on_enter '{}' целевой колонки '{}' отклонил переход (Exit code: {:?}):\n{}", hook_rel, target_col.id, res.exit_code, err_details);
                        return handle_advance_hook_or_sensor_rejection(
                            root,
                            task_id,
                            source_col.as_ref(),
                            &steer_content,
                            err_msg,
                            app_handle,
                        ).await;
                    }
                    record_task_hook_result(
                        &mut current_task,
                        &target_col.id,
                        "on_enter",
                        &hook_rel,
                        hook_started_at,
                        duration_ms,
                        true,
                        res.exit_code,
                        &res.stdout,
                        &res.stderr,
                        None,
                    );
                    let _ = crate::db::db_upsert_task(root, &current_task);
                    if let Some(app) = app_handle {
                        let _ = app.emit("hook:finish", serde_json::json!({
                            "taskId": current_task.id,
                            "stageId": target_col.id,
                            "hookType": "on_enter",
                            "success": true,
                            "exitCode": res.exit_code,
                            "stdout": res.stdout,
                            "stderr": res.stderr
                        }));
                    }
                    apply_worktree_from_hook_stdout(&mut current_task, &res.stdout);
                    let _ = crate::db::db_upsert_task(root, &current_task);
                    if let Some(app) = app_handle {
                        if let Some(state_ref) = app.try_state::<AppState>() {
                            if let Ok(mut guard) = state_ref.tasks.lock() {
                                if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, &current_task.id)) {
                                    t.worktree_path = current_task.worktree_path.clone();
                                    t.branch = current_task.branch.clone();
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    let duration_ms = (chrono::Utc::now().timestamp_millis() - hook_started_at).max(0) as u64;
                    let err_details = e.to_string();
                    record_task_hook_result(
                        &mut current_task,
                        &target_col.id,
                        "on_enter",
                        &hook_rel,
                        hook_started_at,
                        duration_ms,
                        false,
                        None,
                        "",
                        &err_details,
                        Some(&err_details),
                    );
                    let _ = crate::db::db_upsert_task(root, &current_task);
                    if let Some(app) = app_handle {
                        let _ = app.emit("hook:finish", serde_json::json!({
                            "taskId": current_task.id,
                            "stageId": target_col.id,
                            "hookType": "on_enter",
                            "success": false,
                            "exitCode": None::<i32>,
                            "stdout": "",
                            "stderr": err_details,
                            "error": err_details
                        }));
                    }
                    let steer_content = format!(
                        "### [ОТКЛОНЕНО ХУКОМ / HOOK REJECTED: {}]\n\nОшибка запуска хука на входе в колонку '{}': {}\n\nИсправь указанную ошибку.",
                        hook_rel, target_col.id, err_details
                    );
                    let err_msg = format!("Хук on_enter '{}' целевой колонки '{}' ошибка исполнения: {}", hook_rel, target_col.id, err_details);
                    return handle_advance_hook_or_sensor_rejection(
                        root,
                        task_id,
                        source_col.as_ref(),
                        &steer_content,
                        err_msg,
                        app_handle,
                    ).await;
                }
            }
        }
    }

    if let Some(ref pid) = current_task.parent_id {
        // CASE A: The task calling advance_task is a child subtask (e.g. CVH-009.1)
        let subtask_id = current_task.id.clone();
        let parent_id = pid.clone();

        let parent_task = tasks
            .iter()
            .find(|t| task_id_matches(&t.id, &parent_id))
            .cloned()
            .ok_or_else(|| format!("Parent task {} not found", parent_id))?;

        if is_terminal {
            // 1. Mark this subtask as done
            let mut tasks = load_tasks_from_disk(root).await;
            if let Some(sub) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &subtask_id)) {
                sub.status = Some("done".to_string());
                sub.updated_at = Utc::now().timestamp_millis();
                let _ = crate::db::db_upsert_task(root, sub);
            }
            if let Some(app) = app_handle {
                if let Some(state_ref) = app.try_state::<AppState>() {
                    if let Ok(mut guard) = state_ref.tasks.lock() {
                        if let Some(sub) = guard.iter_mut().find(|t| task_id_matches(&t.id, &subtask_id)) {
                            sub.status = Some("done".to_string());
                            sub.updated_at = Utc::now().timestamp_millis();
                        }
                    }
                }
            }
            let _ = save_tasks_to_disk(root, &tasks).await;
            if let Some(app) = app_handle {
                if let Some(sub) = tasks.iter().find(|t| task_id_matches(&t.id, &subtask_id)) {
                    let _ = app.emit("task:updated", serde_json::json!({ "task": sub }));
                    let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
                }
            }

            let sub_summary_content = format!(
                "### Саммари этапа {}: {}\n\n{}",
                current_task.stage_id, current_task.title, summary
            );
            let _ = append_task_dialog_activity(root, &subtask_id, "summary", &sub_summary_content, app_handle).await;

            let updated_parent = internal_move_task_stage(
                root,
                &parent_id,
                &target_col.id,
                Some("APPROVED".to_string()),
                Some("done".to_string()),
                Some(summary.to_string()),
                app_handle,
                true,
                None,
            )
            .await?;

            let mut final_tasks = load_tasks_from_disk(root).await;
            for t in final_tasks.iter_mut() {
                if task_id_matches(&t.id, &parent_id) {
                    t.waiting_for_subtask_id = None;
                    t.status = Some("done".to_string());
                    t.stage_id = target_col.id.clone();
                    t.updated_at = Utc::now().timestamp_millis();
                    let _ = crate::db::db_upsert_task(root, t);
                }
                if t.parent_id.as_deref() == Some(&parent_id) {
                    t.status = Some("done".to_string());
                    t.updated_at = Utc::now().timestamp_millis();
                    let _ = crate::db::db_upsert_task(root, t);
                }
            }
            let _ = save_tasks_to_disk(root, &final_tasks).await;
            if let Some(app) = app_handle {
                if let Some(state_ref) = app.try_state::<AppState>() {
                    if let Ok(mut guard) = state_ref.tasks.lock() {
                        *guard = final_tasks.clone();
                    }
                }
                if let Some(parent) = final_tasks.iter().find(|t| task_id_matches(&t.id, &parent_id)) {
                    let _ = app.emit("task:updated", serde_json::json!({ "task": parent }));
                }
                let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &final_tasks }));
            }

            let parent_summary_content = format!(
                "### Саммари финализации задачи {}: {}\n\n{}",
                parent_id, target_col.title, summary
            );
            let _ = append_task_dialog_activity(root, &parent_id, "summary", &parent_summary_content, app_handle).await;

            return Ok(format!(
                "УСПЕШНО: Задача {} завершена в терминальной колонке '{}' ({}), статус: {}",
                parent_id,
                target_col.id,
                target_col.title,
                updated_parent.status.as_deref().unwrap_or("done")
            ));
        } else {
            let mut tasks = load_tasks_from_disk(root).await;
            let existing_target_sub = tasks
                .iter()
                .filter(|t| {
                    t.parent_id.as_deref().is_some_and(|p| task_id_matches(p, &parent_id))
                        && stage_id_matches(&t.stage_id, &target_col.id)
                        && !task_id_matches(&t.id, &subtask_id)
                        && (t.verdict.as_deref() == Some("CHANGES_REQUESTED") || t.status.as_deref() != Some("done"))
                })
                .max_by_key(|t| (t.subtask_index.unwrap_or(0), t.created_at))
                .cloned();

            if let Some(existing_sub) = existing_target_sub {
                let target_has_autorun = target_col.persona.is_some() || target_col.auto_run == Some(true);
                let next_status = if target_has_autorun { "running" } else { "idle" };

                // 1. Mark current subtask as done
                if let Some(sub) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &subtask_id)) {
                    sub.status = Some("done".to_string());
                    sub.updated_at = Utc::now().timestamp_millis();
                    let _ = crate::db::db_upsert_task(root, sub);
                }

                // 2. Update parent task waiting_for_subtask_id
                if let Some(parent) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &parent_id)) {
                    parent.waiting_for_subtask_id = Some(existing_sub.id.clone());
                    parent.status = Some("waiting_subtask".to_string());
                    parent.updated_at = Utc::now().timestamp_millis();
                    let _ = crate::db::db_upsert_task(root, parent);
                }

                // 3. Reactivate existing target subtask
                if let Some(target_sub) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &existing_sub.id)) {
                    target_sub.status = Some(next_status.to_string());
                    target_sub.verdict = None;
                    target_sub.handover_summary = Some(summary.to_string());
                    target_sub.updated_at = Utc::now().timestamp_millis();
                    let _ = crate::db::db_upsert_task(root, target_sub);
                }

                let _ = save_tasks_to_disk(root, &tasks).await;
                if let Some(app) = app_handle {
                    if let Some(state_ref) = app.try_state::<AppState>() {
                        if let Ok(mut guard) = state_ref.tasks.lock() {
                            *guard = tasks.clone();
                        }
                    }
                }

                let sub_summary_content = format!(
                    "### Саммари этапа {}: {}\n\n{}",
                    current_task.stage_id, current_task.title, summary
                );
                let _ = append_task_dialog_activity(root, &subtask_id, "summary", &sub_summary_content, app_handle).await;

                let parent_summary_content = format!(
                    "### Саммари этапа (передано на {} {}):\n\n{}",
                    existing_sub.id, target_col.title, summary
                );
                let _ = append_task_dialog_activity(root, &parent_id, "summary", &parent_summary_content, app_handle).await;

                let handover_prompt = format!(
                    "### [ОТЧЕТ ОБ ИСПРАВЛЕНИЯХ / HANDOVER SUMMARY]\n\nЗамечания исправлены. Отчет исполнителя:\n{}",
                    summary
                );
                let _ = append_task_dialog_activity(root, &existing_sub.id, "steer", &handover_prompt, app_handle).await;

                if let Some(app) = app_handle {
                    if let Some(sub) = tasks.iter().find(|t| task_id_matches(&t.id, &subtask_id)) {
                        let _ = app.emit("task:updated", serde_json::json!({ "task": sub }));
                    }
                    if let Some(parent) = tasks.iter().find(|t| task_id_matches(&t.id, &parent_id)) {
                        let _ = app.emit("task:updated", serde_json::json!({ "task": parent }));
                    }
                    if let Some(target_sub) = tasks.iter().find(|t| task_id_matches(&t.id, &existing_sub.id)) {
                        let _ = app.emit("task:updated", serde_json::json!({ "task": target_sub }));
                    }
                    let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));

                    if target_has_autorun {
                        let _ = app.emit("agent:autorun", serde_json::json!({
                            "taskId": existing_sub.id,
                            "prompt": handover_prompt,
                        }));
                    }
                }

                return Ok(format!(
                    "УСПЕШНО: Возобновлена существующая подзадача {} ({}) для задачи {}",
                    existing_sub.id, target_col.title, parent_id
                ));
            } else {
                // 1. Mark this subtask as done
                if let Some(sub) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &subtask_id)) {
                    sub.status = Some("done".to_string());
                    sub.updated_at = Utc::now().timestamp_millis();
                    let _ = crate::db::db_upsert_task(root, sub);
                }
                if let Some(app) = app_handle {
                    if let Some(state_ref) = app.try_state::<AppState>() {
                        if let Ok(mut guard) = state_ref.tasks.lock() {
                            if let Some(sub) = guard.iter_mut().find(|t| task_id_matches(&t.id, &subtask_id)) {
                                sub.status = Some("done".to_string());
                                sub.updated_at = Utc::now().timestamp_millis();
                            }
                        }
                    }
                }
                let _ = save_tasks_to_disk(root, &tasks).await;
                if let Some(app) = app_handle {
                    if let Some(sub) = tasks.iter().find(|t| task_id_matches(&t.id, &subtask_id)) {
                        let _ = app.emit("task:updated", serde_json::json!({ "task": sub }));
                        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
                    }
                }

                let sub_summary_content = format!(
                    "### Саммари этапа {}: {}\n\n{}",
                    current_task.stage_id, current_task.title, summary
                );
                let _ = append_task_dialog_activity(root, &subtask_id, "summary", &sub_summary_content, app_handle).await;

                let next_subtask = internal_create_subtask(
                    root,
                    &parent_id,
                    Some(&target_col.id),
                    Some(&format!("{} ({})", parent_task.title, target_col.title)),
                    Some(summary),
                    app_handle,
                )
                .await?;

                let next_subtask_id = next_subtask.id.clone();
                let parent_summary_content = format!(
                    "### Саммари этапа (передано на {} {}):\n\n{}",
                    next_subtask_id, target_col.title, summary
                );
                let _ = append_task_dialog_activity(root, &parent_id, "summary", &parent_summary_content, app_handle).await;

                return Ok(format!(
                    "УСПЕШНО: Создана подзадача {} ({}) для задачи {}",
                    next_subtask_id, target_col.title, parent_id
                ));
            }
        }
    }

    // CASE B: The task calling advance_task is a root parent task (e.g. CVH-009)
    if !is_terminal {
        // Strict guard against duplicate subtasks when subtask is already running:
        if let Some(ref existing_sub_id) = current_task.waiting_for_subtask_id {
            if let Some(existing_sub) = tasks.iter().find(|t| task_id_matches(&t.id, existing_sub_id)) {
                if existing_sub.status.as_deref() != Some("done")
                    && existing_sub.verdict.as_deref() != Some("CHANGES_REQUESTED")
                {
                    crate::logger::cvh_log(
                        Some(root),
                        "WARN",
                        "ADVANCE_TASK",
                        &format!(
                            "advance_task: Task '{}' already has active waiting_for_subtask_id '{}' (status: {:?})",
                            task_id, existing_sub_id, existing_sub.status
                        ),
                    );
                    return Ok(format!(
                        "Подзадача {} уже активна для задачи {}",
                        existing_sub_id, task_id
                    ));
                }
            }
        }
        if let Some(existing_sub) = tasks.iter().find(|t| {
            t.parent_id.as_deref().is_some_and(|p| task_id_matches(p, task_id))
                && stage_id_matches(&t.stage_id, &target_col.id)
                && t.status.as_deref() != Some("done")
                && t.verdict.as_deref() != Some("CHANGES_REQUESTED")
        }) {
            crate::logger::cvh_log(
                Some(root),
                "WARN",
                "ADVANCE_TASK",
                &format!(
                    "advance_task: Task '{}' already has active subtask '{}' in column '{}' (status: {:?})",
                    task_id, existing_sub.id, target_col.title, existing_sub.status
                ),
            );
            return Ok(format!(
                "Подзадача {} уже выполняется в колонке {}",
                existing_sub.id, target_col.title
            ));
        }

        let existing_target_sub = tasks
            .iter()
            .filter(|t| {
                t.parent_id.as_deref().is_some_and(|p| task_id_matches(p, task_id))
                    && stage_id_matches(&t.stage_id, &target_col.id)
                    && (t.verdict.as_deref() == Some("CHANGES_REQUESTED") || t.status.as_deref() != Some("done"))
            })
            .max_by_key(|t| (t.subtask_index.unwrap_or(0), t.created_at))
            .cloned();

        if let Some(existing_sub) = existing_target_sub {
            let target_has_autorun = target_col.persona.is_some() || target_col.auto_run == Some(true);
            let next_status = if target_has_autorun { "running" } else { "idle" };

            let mut tasks = load_tasks_from_disk(root).await;
            if let Some(parent) = tasks.iter_mut().find(|t| task_id_matches(&t.id, task_id)) {
                parent.waiting_for_subtask_id = Some(existing_sub.id.clone());
                parent.status = Some("waiting_subtask".to_string());
                parent.updated_at = Utc::now().timestamp_millis();
                let _ = crate::db::db_upsert_task(root, parent);
            }

            if let Some(target_sub) = tasks.iter_mut().find(|t| task_id_matches(&t.id, &existing_sub.id)) {
                target_sub.status = Some(next_status.to_string());
                target_sub.verdict = None;
                target_sub.handover_summary = Some(summary.to_string());
                target_sub.updated_at = Utc::now().timestamp_millis();
                let _ = crate::db::db_upsert_task(root, target_sub);
            }

            let _ = save_tasks_to_disk(root, &tasks).await;
            if let Some(app) = app_handle {
                if let Some(state_ref) = app.try_state::<AppState>() {
                    if let Ok(mut guard) = state_ref.tasks.lock() {
                        *guard = tasks.clone();
                    }
                }
            }

            let parent_summary_content = format!(
                "### Саммари этапа (передано на {} {}):\n\n{}",
                existing_sub.id, target_col.title, summary
            );
            let _ = append_task_dialog_activity(root, task_id, "summary", &parent_summary_content, app_handle).await;

            let handover_prompt = format!(
                "### [ОТЧЕТ ОБ ИСПРАВЛЕНИЯХ / HANDOVER SUMMARY]\n\nЗамечания исправлены. Отчет исполнителя:\n{}",
                summary
            );
            let _ = append_task_dialog_activity(root, &existing_sub.id, "steer", &handover_prompt, app_handle).await;

            if let Some(app) = app_handle {
                if let Some(parent) = tasks.iter().find(|t| task_id_matches(&t.id, task_id)) {
                    let _ = app.emit("task:updated", serde_json::json!({ "task": parent }));
                }
                if let Some(target_sub) = tasks.iter().find(|t| task_id_matches(&t.id, &existing_sub.id)) {
                    let _ = app.emit("task:updated", serde_json::json!({ "task": target_sub }));
                }
                let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));

                if target_has_autorun {
                    let _ = app.emit("agent:autorun", serde_json::json!({
                        "taskId": existing_sub.id,
                        "prompt": handover_prompt,
                    }));
                }
            }

            return Ok(format!(
                "УСПЕШНО: Возобновлена существующая подзадача {} ({}) для задачи {}",
                existing_sub.id, target_col.title, task_id
            ));
        }

        let subtask = internal_create_subtask(
            root,
            task_id,
            Some(&target_col.id),
            Some(&format!("{} ({})", current_task.title, target_col.title)),
            Some(summary),
            app_handle,
        )
        .await?;

        let subtask_id = subtask.id.clone();

        let parent_summary_content = format!(
            "### Саммари этапа (передано на {} {}):\n\n{}",
            subtask_id, target_col.title, summary
        );
        let _ = append_task_dialog_activity(
            root,
            task_id,
            "summary",
            &parent_summary_content,
            app_handle,
        )
        .await;

        return Ok(format!(
            "УСПЕШНО: Создана подзадача {} ({}) для задачи {}",
            subtask_id, target_col.title, task_id
        ));
    }

    // Terminal column advance for parent task:
    let updated_task = internal_move_task_stage(
        root,
        task_id,
        &target_col.id,
        Some("APPROVED".to_string()),
        Some("done".to_string()),
        Some(summary.to_string()),
        app_handle,
        true, // is_autonomous_advance
        None,
    )
    .await?;

    let summary_content = format!("### Саммари этапа: {}\n\n{}", target_col.title, summary);
    let _ = append_task_dialog_activity(
        root,
        task_id,
        "summary",
        &summary_content,
        app_handle,
    )
    .await;

    Ok(format!(
        "УСПЕШНО: Задача {} завершена в терминальной колонке '{}' ({}), статус: {}",
        task_id,
        target_col.id,
        target_col.title,
        updated_task.status.as_deref().unwrap_or("done")
    ))}

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
                        result
                    };
                    let _ = app_inner.emit(
                        "agent:tool_result",
                        serde_json::json!({ "taskId": tid_inner, "id": id, "name": name, "result": &sanitized_result, "success": success }),
                    );

                    let status_str = if success { "success" } else { "failed" };
                    let mut updated_item = None;
                    if let Ok(items) = crate::db::load_dialog_items(&root_inner, &tid_inner) {
                        if let Some(mut existing) = items.into_iter().find(|it| it.id == id) {
                            let mut tool_val = existing.tool.take().unwrap_or_else(|| serde_json::json!({}));
                            if let Some(obj) = tool_val.as_object_mut() {
                                obj.insert("status".to_string(), serde_json::json!(status_str));
                                obj.insert("stdout".to_string(), serde_json::json!(&sanitized_result));
                                obj.insert("result".to_string(), serde_json::json!(&sanitized_result));
                                if !success {
                                    obj.insert("stderr".to_string(), serde_json::json!(&sanitized_result));
                                }
                            }
                            existing.tool = Some(tool_val);
                            updated_item = Some(existing);
                        }
                    }
                    let item_to_save = updated_item.unwrap_or_else(|| {
                        let tool_obj = serde_json::json!({
                            "id": &id,
                            "toolName": &name,
                            "name": &name,
                            "status": status_str,
                            "stdout": &sanitized_result,
                            "result": &sanitized_result,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        });
                        crate::db::ActivityItem {
                            id: id.clone(),
                            item_type: "tool".to_string(),
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            content: None,
                            tool: Some(tool_obj),
                            duration_ms: None,
                            images: None,
                        }
                    });
                    let _ = crate::db::append_dialog_items(&root_inner, &tid_inner, std::slice::from_ref(&item_to_save));
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

        // Clean up from active_agents registry upon agent completion only if this entry still belongs to this run
        if let Some(state_ref) = app.try_state::<AppState>() {
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

        let had_entry = !matching_handles.is_empty();

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

    let had_entry = !matching_handles.is_empty();

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
    // MERGE IN-MEMORY TASKS (AGENTS.md):
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

#[tauri::command]
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

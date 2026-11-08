//! Task Persistence and Monotonic Sequence Storage Subsystem (§ 2.26)
//! Extracted from commands/tasks.rs per ub-mode Clean Code principles.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::agent::Task;
use crate::config::{
    atomic_write_file, get_app_config_dir, load_boards_config, resolve_actual_repo_root,
};
use super::healing::heal_tasks;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskSequence {
    #[serde(default, alias = "sequence", alias = "last_task_num", alias = "lastTaskNum")]
    pub last_task_num: u32,
}

/// Checks if a filesystem path lies within the specified project root directory.
pub fn is_path_inside_project(path: &Path, root: &Path) -> bool {
    let norm_path = normalize_path_for_comparison(path);
    let norm_root = normalize_path_for_comparison(root);
    if norm_root.is_empty() || norm_path.is_empty() {
        return true;
    }
    norm_path == norm_root || norm_path.starts_with(&format!("{}/", norm_root.trim_end_matches('/')))
}

fn normalize_path_for_comparison(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    let s = s.trim();
    let s = s.strip_prefix("//?/").unwrap_or(s);
    let s = s.strip_prefix("\\\\?\\").unwrap_or(s);
    let s = s.trim_end_matches('/');
    #[cfg(windows)]
    {
        s.to_lowercase()
    }
    #[cfg(not(windows))]
    {
        s.to_string()
    }
}

/// Identifies all task IDs in a slice that belong to a foreign workspace root,
/// including mismatched project_path, worktree paths outside the project root, and cascading child/parent relationships.
pub fn find_foreign_task_ids(tasks: &[Task], repo_root: &Path) -> HashSet<String> {
    let mut foreign_ids = HashSet::new();
    if repo_root.as_os_str().is_empty() {
        return foreign_ids;
    }

    let norm_repo_root = normalize_path_for_comparison(repo_root);

    // 1. Direct check: project_path and worktree_path
    for t in tasks {
        if let Some(ref p_path) = t.project_path {
            let norm_p = normalize_path_for_comparison(Path::new(p_path));
            if !norm_p.is_empty() && !norm_repo_root.is_empty() && norm_p != norm_repo_root {
                foreign_ids.insert(t.id.clone());
                continue;
            }
        }
        if let Some(ref wt_str) = t.worktree_path {
            let trimmed = wt_str.trim();
            if !trimmed.is_empty() {
                let wt = Path::new(trimmed);
                let is_foreign = if wt.starts_with("..") {
                    true
                } else if wt.is_absolute() {
                    if is_path_inside_project(wt, repo_root) {
                        false
                    } else if cfg!(test) {
                        wt.to_string_lossy().contains("foreign")
                    } else {
                        true
                    }
                } else {
                    false
                };
                if is_foreign {
                    foreign_ids.insert(t.id.clone());
                }
            }
        }
    }

    // 2. Cascade propagation: child tasks whose parent is foreign, or parent tasks whose children are foreign
    let mut changed = true;
    while changed {
        changed = false;
        for t in tasks {
            if foreign_ids.contains(&t.id) {
                continue;
            }
            if let Some(ref pid) = t.parent_id {
                if foreign_ids.contains(pid) {
                    foreign_ids.insert(t.id.clone());
                    changed = true;
                    continue;
                }
            }
            if tasks.iter().any(|c| c.parent_id.as_deref() == Some(&t.id) && foreign_ids.contains(&c.id)) {
                foreign_ids.insert(t.id.clone());
                changed = true;
            }
        }
    }
    foreign_ids
}

pub fn get_project_tasks_backup_file(project_root: &Path) -> PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    project_root.to_string_lossy().to_lowercase().hash(&mut hasher);
    let hash = hasher.finish();
    get_app_config_dir().join("projects").join(format!("{:016x}", hash)).join("tasks.json")
}

pub async fn load_tasks_from_disk(repo_root: &Path) -> Vec<Task> {
    let actual_root = resolve_actual_repo_root(repo_root);
    if actual_root.as_os_str().is_empty() {
        return Vec::new();
    }
    let boards_cfg = load_boards_config(&actual_root)
        .await
        .unwrap_or_else(|_| crate::config::default_boards_config());

    // 1. Sole Source of Truth: High-speed SQLite Database (.cvh/harness.db)
    let mut tasks = match crate::db::db_get_all_tasks(&actual_root) {
        Ok(t) => t,
        Err(e) => {
            crate::logger::cvh_log(
                Some(&actual_root),
                "WARN",
                "LOAD_TASKS",
                &format!("db_get_all_tasks warning: {}", e),
            );
            Vec::new()
        }
    };

    // 2. Purge foreign tasks that were contaminated into this SQLite database
    let foreign_ids = find_foreign_task_ids(&tasks, &actual_root);
    if !foreign_ids.is_empty() {
        crate::logger::cvh_log(
            Some(&actual_root),
            "WARN",
            "LOAD_TASKS",
            &format!(
                "Purging {} foreign project tasks from SQLite database at {:?}: [{}]",
                foreign_ids.len(),
                actual_root,
                foreign_ids.iter().cloned().collect::<Vec<_>>().join(", ")
            ),
        );
        for fid in &foreign_ids {
            let _ = crate::db::db_delete_task(&actual_root, fid);
        }
        tasks.retain(|t| !foreign_ids.contains(&t.id));
    }

    for t in tasks.iter_mut() {
        t.project_path = Some(actual_root.to_string_lossy().to_string());
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

pub async fn save_tasks_to_disk(repo_root: &Path, tasks: &[Task]) -> Result<(), String> {
    let actual_root = resolve_actual_repo_root(repo_root);
    if actual_root.as_os_str().is_empty() {
        return Ok(());
    }

    let foreign_ids = find_foreign_task_ids(tasks, &actual_root);
    let mut filtered_tasks: Vec<Task> = if !foreign_ids.is_empty() {
        crate::logger::cvh_log(
            Some(&actual_root),
            "WARN",
            "SAVE_TASKS",
            &format!(
                "Filtered out {} foreign tasks before writing to disk for root '{:?}': [{}]",
                foreign_ids.len(),
                actual_root,
                foreign_ids.iter().cloned().collect::<Vec<_>>().join(", ")
            ),
        );
        tasks.iter().filter(|t| !foreign_ids.contains(&t.id)).cloned().collect()
    } else {
        tasks.to_vec()
    };
    for t in filtered_tasks.iter_mut() {
        if t.project_path.is_none() {
            t.project_path = Some(actual_root.to_string_lossy().to_string());
        }
    }
    let tasks = &filtered_tasks;

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

    let ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    crate::logger::cvh_log_with_details(
        Some(&actual_root),
        "INFO",
        "SAVE_TASKS",
        &format!("save_tasks_to_disk: saved {} tasks to SQLite", tasks.len()),
        Some(&format!("Saved IDs: [{}]", ids.join(", "))),
    );
    db_res.map_err(|e| format!("save_tasks_to_disk: SQLite persistence error: {}", e))
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

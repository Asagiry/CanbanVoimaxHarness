//! Task and Board Self-Healing Subsystem (§ 2.18 Architectural Invariants)
//! Extracted from commands/tasks.rs per ub-mode Clean Code principles.

use std::collections::HashSet;
use crate::agent::Task;
use crate::config::BoardsConfig;

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

/// Dynamically heals a single task against the given BoardsConfig:
/// 1. Board ID: If task.board_id is None, empty, or does not match any existing board in config.boards:
///    reassign it to config.active_board_id (if that board exists), otherwise to config.boards[0].id (if available),
///    or fallback to "development".
/// 2. Column / Stage ID: If task.stage_id does not exist in the target board's columns (checked via stage_id_matches),
///    dynamically heal it to the target board's first column (typically "backlog").
///
/// Returns true if any modification was made to the task.
pub fn heal_task_board_and_column(task: &mut Task, config: &BoardsConfig) -> bool {
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
pub fn heal_tasks(tasks: &mut [Task], config: &BoardsConfig) -> bool {
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
pub fn ensure_task_board_id(mut tasks: Vec<Task>, config: &BoardsConfig) -> Vec<Task> {
    heal_tasks(&mut tasks, config);
    tasks
}

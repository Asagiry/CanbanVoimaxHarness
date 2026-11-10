use chrono::Utc;
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager};

use crate::agent::Task;
use crate::commands::state::AppState;
use crate::commands::healing::task_id_matches;

/// Upserts a single mutated task into SQLite (ensuring `project_path` is set) without
/// overwriting any other concurrent tasks on the board.
pub fn upsert_single_task(root: &Path, task: &mut Task) -> Result<(), String> {
    if task.project_path.is_none() && !root.as_os_str().is_empty() {
        task.project_path = Some(root.to_string_lossy().to_string());
    }
    crate::db::db_upsert_task(root, task)
        .map_err(|e| format!("Failed to upsert task '{}' in SQLite: {}", task.id, e))
}

/// Synchronizes a single mutated or newly created task into in-memory `AppState`
/// without overwriting concurrent updates to other tasks.
pub fn sync_single_task_in_app_state(root: &Path, task: &Task, app_handle: Option<&AppHandle>) {
    if let Some(app) = app_handle {
        if let Some(state_ref) = app.try_state::<AppState>() {
            if state_ref.get_root().as_path() == root {
                let mut guard = state_ref.tasks.lock().unwrap_or_else(|p| p.into_inner());
                if let Some(existing) = guard.iter_mut().find(|t| task_id_matches(&t.id, &task.id)) {
                    *existing = task.clone();
                } else {
                    guard.push(task.clone());
                }
            }
        }
    }
}

/// Formats a stage completion summary activity entry for the task dialog log.
pub fn format_stage_summary_activity(
    stage_label: &str,
    task_title: Option<&str>,
    summary: &str,
) -> String {
    match task_title {
        Some(title) => format!("### Саммари этапа {}: {}\n\n{}", stage_label, title, summary),
        None => format!("### Саммари этапа: {}\n\n{}", stage_label, summary),
    }
}

/// Formats a handover steer prompt when reactivating a previously rejected subtask.
pub fn format_handover_steer_prompt(summary: &str) -> String {
    format!(
        "### [ОТЧЕТ ОБ ИСПРАВЛЕНИЯХ / HANDOVER SUMMARY]\n\nЗамечания исправлены. Отчет исполнителя:\n{}",
        summary
    )
}

/// Fallback: auto-completes any unfinished task todos (status != 'completed')
/// when advancing or completing a task, updating SQLite, memory, and emitting UI events.
/// Returns true if any todos were modified/completed, false if already completed or absent.
pub fn auto_complete_task_todos(
    root: &Path,
    task: &mut Task,
    app_handle: Option<&AppHandle>,
) -> bool {
    let memory_had_unfinished = task.todos.as_ref().is_some_and(|tds| {
        tds.iter().any(|td| td.status != "completed")
    });
    let db_had_unfinished = if memory_had_unfinished {
        true
    } else {
        crate::db::db_has_unfinished_todos(root, &task.id).unwrap_or(true)
    };

    if !memory_had_unfinished && !db_had_unfinished {
        return false;
    }

    let mut completed_count = 0usize;
    if let Some(ref mut todos) = task.todos {
        for td in todos.iter_mut() {
            if td.status != "completed" {
                td.status = "completed".to_string();
                completed_count += 1;
            }
        }
    }
    if completed_count > 0 {
        crate::logger::cvh_log(
            Some(root),
            "INFO",
            "AUTO_COMPLETE_TODOS",
            &format!("Auto-completed {} unfinished TODO item(s) for task {}", completed_count, task.id),
        );
    }

    match crate::db::db_complete_all_todos(root, &task.id) {
        Ok(Some(db_todos)) => {
            if task.todos.is_none() {
                task.todos = Some(db_todos);
            }
        }
        Ok(None) => {}
        Err(e) => {
            crate::logger::cvh_log(
                Some(root),
                "WARN",
                "AUTO_COMPLETE_TODOS",
                &format!("db_complete_all_todos error for task {}: {}", task.id, e),
            );
        }
    }

    task.updated_at = Utc::now().timestamp_millis();
    let _ = upsert_single_task(root, task);

    if let Some(app) = app_handle {
        if let Some(state_ref) = app.try_state::<AppState>() {
            if state_ref.get_root().as_path() == root {
                let mut guard = state_ref.tasks.lock().unwrap_or_else(|p| p.into_inner());
                if let Some(t) = guard.iter_mut().find(|t| task_id_matches(&t.id, &task.id)) {
                    t.todos = task.todos.clone();
                    t.updated_at = task.updated_at;
                }
            }
        }
        let project_path_str = root.to_string_lossy().to_string();
        let _ = app.emit("task:updated", serde_json::json!({
            "taskId": &task.id,
            "todos": &task.todos,
            "task": &task,
            "projectPath": &project_path_str,
        }));
    }

    true
}

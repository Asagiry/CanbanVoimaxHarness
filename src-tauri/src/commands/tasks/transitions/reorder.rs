//! Task Reordering within/across Stages (§ 2.2)

use chrono::Utc;
use tauri::{AppHandle, Emitter, State};

use crate::agent::Task;
use crate::commands::state::AppState;
use crate::commands::tasks::healing::{stage_id_matches, task_id_matches};
use crate::commands::tasks::storage::load_tasks_from_disk;
use super::transition_hooks::sync_single_task_in_app_state;
use super::move_stage::internal_move_task_stage;

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
    let mut tasks_to_persist: Vec<Task> = Vec::new();

    for (idx, tid) in ordered_ids.iter().enumerate() {
        let new_pos = (idx as f64) * 1000.0;
        if let Some(t) = tasks.iter_mut().find(|t| task_id_matches(&t.id, tid)) {
            t.position = Some(new_pos);
            t.updated_at = now;
            if t.project_path.is_none() && !root.as_os_str().is_empty() {
                t.project_path = Some(root.to_string_lossy().to_string());
            }
            tasks_to_persist.push(t.clone());
            if task_id_matches(&t.id, &task_id) {
                updated_task = Some(t.clone());
            }
        }
    }

    crate::db::db_upsert_all_tasks(&root, &tasks_to_persist)?;
    for t in &tasks_to_persist {
        sync_single_task_in_app_state(&root, t, Some(&app_handle));
    }

    let project_path_str = root.to_string_lossy().to_string();
    if let Err(e) = app_handle.emit("tasks:updated", serde_json::json!({ "tasks": &tasks, "projectPath": &project_path_str })) {
        eprintln!("CVH [WARN]: Failed to emit tasks:updated event in reorder_task: {}", e);
    }
    if let Some(ref ut) = updated_task {
        if let Err(e) = app_handle.emit("task:updated", serde_json::json!({ "taskId": ut.id, "task": ut, "projectPath": &project_path_str })) {
            eprintln!("CVH [WARN]: Failed to emit task:updated event in reorder_task: {}", e);
        }
    }

    updated_task.ok_or_else(|| format!("Failed to reorder task {}", task_id))
}

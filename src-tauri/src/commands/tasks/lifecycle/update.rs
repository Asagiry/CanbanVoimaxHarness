use chrono::Utc;
use tauri::{AppHandle, Emitter, State};

use crate::agent::Task;
use crate::commands::state::AppState;
use crate::commands::healing::task_id_matches;
use crate::commands::tasks::storage::{save_tasks_to_disk, try_load_tasks_from_disk};

#[allow(clippy::too_many_arguments)]
pub async fn update_task(
    task_id: String,
    model: Option<String>,
    provider_id: Option<String>,
    reasoning_effort: Option<String>,
    title: Option<String>,
    description: Option<String>,
    status: Option<String>,
    verdict: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    let root = state.get_root();
    let updated_subagent = crate::agent::update_subagent_session_model_or_effort(
        &root,
        &task_id,
        model.as_deref(),
        provider_id.as_deref(),
        reasoning_effort.as_deref(),
    )
    .await;

    let mut tasks = try_load_tasks_from_disk(&root).await?;
    let Some(task_idx) = tasks.iter().position(|t| task_id_matches(&t.id, &task_id)) else {
        if updated_subagent || task_id.contains(".sub-") || task_id.contains(":sub:") {
            return Ok(Task {
                id: task_id,
                model: model.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
                provider_id: provider_id.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
                reasoning_effort: crate::config::normalize_reasoning_effort(reasoning_effort.as_deref()),
                ..Default::default()
            });
        }
        return Err(format!("Task {} not found", task_id));
    };

    let now = Utc::now().timestamp_millis();
    let task = &mut tasks[task_idx];

    if let Some(m) = model {
        let trimmed = m.trim().to_string();
        task.model = if trimmed.is_empty() { None } else { Some(trimmed) };
    }
    if let Some(pid) = provider_id {
        let trimmed = pid.trim().to_string();
        task.provider_id = if trimmed.is_empty() { None } else { Some(trimmed) };
    }
    if let Some(eff) = reasoning_effort {
        let trimmed = eff.trim();
        task.reasoning_effort = if trimmed.is_empty() {
            None
        } else {
            crate::config::normalize_reasoning_effort(Some(trimmed))
                .or_else(|| Some(trimmed.to_string()))
        };
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
    crate::db::db_upsert_task(&root, &updated)?;
    save_tasks_to_disk(&root, &tasks).await?;

    if state.is_same_root(&root) {
        let mut guard = state.tasks.lock().unwrap_or_else(|p| p.into_inner());
        *guard = tasks.clone();
    }

    let project_path_str = root.to_string_lossy().to_string();
    let _ = app_handle.emit("tasks:updated", serde_json::json!({ "tasks": &tasks, "projectPath": &project_path_str }));
    let _ = app_handle.emit("task:updated", serde_json::json!({ "task": &updated, "projectPath": &project_path_str }));

    Ok(updated)
}

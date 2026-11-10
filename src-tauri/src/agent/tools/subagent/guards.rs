use tauri::{Emitter, Manager};

pub fn update_task_execution_status(
    app: Option<&tauri::AppHandle>,
    root: &std::path::Path,
    task_id: &str,
    new_status: &str,
) {
    if let Some(app) = app {
        let mut updated_task = None;
        if let Some(state) = app.try_state::<crate::commands::AppState>() {
            if let Ok(mut tasks) = state.tasks.lock() {
                if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
                    task.status = Some(new_status.to_string());
                    task.updated_at = chrono::Utc::now().timestamp_millis();
                    updated_task = Some(task.clone());
                }
            }
        }
        if let Some(task) = updated_task {
            let _ = crate::db::db_upsert_task(root, &task);
            let root_buf = root.to_path_buf();
            let app_clone = app.clone();
            tokio::spawn(async move {
                if let Some(state) = app_clone.try_state::<crate::commands::AppState>() {
                    let tasks = if let Ok(guard) = state.tasks.lock() {
                        guard.clone()
                    } else {
                        vec![]
                    };
                    if !tasks.is_empty() {
                        let _ = crate::commands::save_tasks_to_disk(&root_buf, &tasks).await;
                    }
                }
            });
            let _ = app.emit("task:updated", serde_json::json!({ "task": &task }));
            let _ = app.emit("agent:status", serde_json::json!({
                "taskId": task_id,
                "status": new_status,
            }));
        }
    }
}

pub struct TaskStatusGuard<'a> {
    app: Option<&'a tauri::AppHandle>,
    root: &'a std::path::Path,
    task_id: &'a str,
    active: bool,
}

impl<'a> TaskStatusGuard<'a> {
    pub fn new(app: Option<&'a tauri::AppHandle>, root: &'a std::path::Path, task_id: &'a str) -> Self {
        update_task_execution_status(app, root, task_id, "waiting_subagent");
        Self { app, root, task_id, active: true }
    }

    pub fn disarm_to_running(&mut self) {
        if self.active {
            update_task_execution_status(self.app, self.root, self.task_id, "running");
            self.active = false;
        }
    }
}

impl<'a> Drop for TaskStatusGuard<'a> {
    fn drop(&mut self) {
        if self.active {
            update_task_execution_status(self.app, self.root, self.task_id, "running");
        }
    }
}

pub struct SubagentActiveHandleGuard<'a> {
    pub app_handle: Option<&'a tauri::AppHandle>,
    pub repo_root: &'a std::path::Path,
    pub subagent_id: &'a str,
}

impl<'a> Drop for SubagentActiveHandleGuard<'a> {
    fn drop(&mut self) {
        if let Some(app) = self.app_handle {
            if let Some(state_ref) = app.try_state::<crate::commands::AppState>() {
                state_ref.remove_active_agent(self.repo_root, self.subagent_id);
            }
        }
    }
}

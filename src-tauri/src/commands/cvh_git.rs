use crate::commands::state::AppState;
use crate::git::cvh_save::{cvh_git_save_at, cvh_git_status_at, CvhGitStatus};
use tauri::State;

#[tauri::command]
pub async fn cvh_git_status(state: State<'_, AppState>) -> Result<CvhGitStatus, String> {
    cvh_git_status_at(&state.get_root()).await
}

#[tauri::command]
pub async fn cvh_git_save(
    message: Option<String>,
    state: State<'_, AppState>,
) -> Result<CvhGitStatus, String> {
    let msg = match message.map(|m| m.trim().to_string()) {
        Some(m) if !m.is_empty() => m,
        _ => format!(
            "chore(cvh): save rules and agents ({})",
            chrono::Local::now().format("%Y-%m-%d %H:%M")
        ),
    };
    cvh_git_save_at(&state.get_root(), &msg).await
}

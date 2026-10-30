//! Task Commands IPC Gateway & Facade Module
//! Exposes Tauri IPC commands and delegates to domain subsystems:
//! - healing: Board self-healing and ID matching
//! - storage: SQLite persistence, disaster backup, and monotonic sequence allocation
//! - lifecycle: Task CRUD, subtask generation, updates, deletion
//! - transitions: Pipeline stage movement, gatekeeper sensors, hooks, and subtask auto-reactivation
//! - runner: Agent execution loop, steering, abort, and context compaction

use tauri::{AppHandle, State};
use crate::agent::Task;
use crate::commands::state::AppState;

pub mod healing;
pub mod lifecycle;
pub mod runner;
pub mod storage;
pub mod transitions;

pub use healing::*;
pub use lifecycle::{
    internal_create_subtask, internal_create_task, internal_save_tasks, is_review_column,
    record_task_hook_result,
};
pub use runner::{
    get_active_sessions as internal_get_active_sessions, internal_abort_agent,
    is_matching_subagent_key, resolve_task_active_agent_model,
};
pub use storage::*;
pub use transitions::{
    auto_complete_task_todos, internal_advance_task, internal_advance_task_to_stage,
    internal_move_task_stage, propagate_parent_subtask_advance, trigger_column_on_enter_script,
};

#[tauri::command]
pub async fn list_tasks(state: State<'_, AppState>) -> Result<Vec<Task>, String> {
    lifecycle::list_tasks(state).await
}

#[tauri::command]
pub async fn save_tasks(tasks: Vec<Task>, state: State<'_, AppState>) -> Result<(), String> {
    lifecycle::save_tasks(tasks, state).await
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
    lifecycle::create_task(
        title,
        description,
        stage_id,
        board_id,
        skip_worktree,
        skip_backlog_hook,
        model,
        task_id,
        attachments,
        app_handle,
        state,
    )
    .await
}

#[tauri::command]
pub async fn create_subtask(
    parent_id: String,
    target_stage_id: Option<String>,
    title: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    lifecycle::create_subtask(parent_id, target_stage_id, title, app_handle, state).await
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
    lifecycle::update_task(
        task_id,
        model,
        title,
        description,
        status,
        verdict,
        app_handle,
        state,
    )
    .await
}

#[tauri::command]
pub async fn delete_task(
    task_id: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    lifecycle::delete_task(task_id, app_handle, state).await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn move_task_stage(
    task_id: String,
    target_stage_id: String,
    verdict: Option<String>,
    handover_summary: Option<String>,
    board_id: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    transitions::move_task_stage(
        task_id,
        target_stage_id,
        verdict,
        handover_summary,
        board_id,
        app_handle,
        state,
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
    transitions::reorder_task(task_id, target_stage_id, new_index, app_handle, state).await
}

#[tauri::command]
pub async fn advance_task(
    task_id: String,
    summary: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    transitions::advance_task(task_id, summary, state, app).await
}

#[tauri::command]
pub fn focus_main_window(app: AppHandle) -> Result<(), String> {
    runner::focus_main_window(app)
}

#[tauri::command]
pub async fn run_agent(
    task_id: String,
    prompt: Option<String>,
    images: Option<Vec<String>>,
    app_handle: AppHandle,
) -> Result<String, String> {
    runner::run_agent(task_id, prompt, images, app_handle).await
}

#[tauri::command]
pub async fn abort_agent(
    task_id: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    runner::abort_agent(task_id, state, app_handle).await
}

#[tauri::command]
pub async fn get_active_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<crate::agent::session_registry::AgentSessionSummary>, String> {
    runner::get_active_sessions(state).await
}

#[tauri::command]
pub async fn compact_task_context(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<crate::agent::compactor::CompactionResult, String> {
    runner::compact_task_context(task_id, state).await
}

#[tauri::command]
pub async fn compact_context(
    task_id: String,
    tool_id: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::agent::compactor::CompactionReport, String> {
    runner::compact_context(task_id, tool_id, app_handle, state).await
}

#[tauri::command]
pub async fn steer_agent(
    task_id: String,
    message: String,
    images: Option<Vec<String>>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    runner::steer_agent(task_id, message, images, app_handle, state).await
}

#[tauri::command]
pub async fn get_task_context_window(
    task_id: Option<String>,
    persona: Option<String>,
    app_handle: AppHandle,
) -> Result<u64, String> {
    runner::get_task_context_window(task_id, persona, app_handle).await
}

use std::path::Path;
use tauri::{AppHandle, Manager};

use crate::agent::Task;
use crate::commands::state::AppState;
use crate::config::{
    load_boards_config, load_pipeline_config, load_project_config,
};
use crate::commands::tasks::storage::load_tasks_from_disk;

pub fn focus_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
        Ok(())
    } else {
        Err("Main window not found".to_string())
    }
}

/// Resolves the effective active agent model for a task, following the canonical priority hierarchy:
/// 1. Explicit Provider / Model pair from the steer bar (`task.provider_id`
///    + `task.model` — only when the provider option is set, so old rows
///      without a pair never resurrect stale SQLite models)
/// 2. Stage persona agent settings (`agent_setting.model`)
/// 3. Column model override (`col.model_override`)
/// 4. Project default model (`proj_cfg.default_model`)
/// 5. First configured model from provider catalogs (or empty when none configured)
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

    let (col_override, agent_model) = if let Some(t) = task {
        let explicit_pair = non_empty_opt(&t.provider_id)
            .and_then(|_| non_empty_opt(&t.model));
        if let Some(pair_model) = explicit_pair {
            return pair_model;
        }
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
            b.and_then(|b| b.stages.iter().find(|c| c.id == t.stage_id).cloned())
        });
        let col = col_from_board
            .or_else(|| pipeline.stages.iter().find(|c| c.id == t.stage_id).cloned());
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

    let candidate = agent_model
        .or(col_override)
        .or(proj_default)
        .unwrap_or_default();

    if !llm_settings.providers.is_empty() {
        let in_any = !candidate.is_empty()
            && llm_settings
                .providers
                .iter()
                .any(|p| p.models.iter().any(|m| m.id == candidate));
        if !in_any {
            if let Some(first) = llm_settings.providers.iter().flat_map(|p| p.models.iter()).next() {
                if !candidate.is_empty() {
                    eprintln!(
                        "CVH [resolve_model]: model '{}' not in any provider catalog, using '{}'",
                        candidate, first.id
                    );
                }
                return first.id.clone();
            }
        }
    }
    candidate
}

pub fn cleanup_active_agent_reservation(app_handle: &AppHandle, root: &std::path::Path, task_id: &str) {
    if let Some(state) = app_handle.try_state::<AppState>() {
        state.remove_active_agent(root, task_id);
    }
}

pub async fn compact_context(
    task_id: String,
    tool_id: Option<String>,
    app_handle: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<crate::agent::compactor::CompactionReport, String> {
    crate::agent::compactor::compact_context_for_task(task_id, tool_id, app_handle, state).await
}

/// Resolves the context window strictly for a task or agent persona role.
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
                let binding = app_handle.state::<AppState>();
                let guard = binding.tasks.lock().unwrap_or_else(|p| p.into_inner());
                guard.iter().find(|t| t.id == tid_trimmed).cloned()
            });

            if let Some(task) = task_opt {
                let boards_cfg = load_boards_config(&root).await.ok();
                let col = boards_cfg.as_ref().and_then(|bc| {
                    let b = if let Some(ref bid) = task.board_id {
                        bc.boards.iter().find(|b| b.id == *bid)
                    } else {
                        bc.boards.iter().find(|b| b.id == bc.active_board_id).or_else(|| bc.boards.first())
                    };
                    b.and_then(|board| board.stages.iter().find(|c| c.id == task.stage_id).cloned())
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

pub async fn get_task_telemetry_summary(
    task_id: String,
    app_handle: AppHandle,
) -> Result<crate::db::TelemetrySummary, String> {
    let root = app_handle.state::<AppState>().get_root();
    crate::db::db_get_telemetry_summary(&root, task_id.trim())
}

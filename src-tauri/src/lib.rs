pub mod agent;
pub mod commands;
pub mod config;
pub mod error;
pub mod git;
pub mod hooks;
pub mod llm;

pub use error::AppError;

use commands::*;
use std::path::PathBuf;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let state = AppState::new(current_dir);

    tauri::Builder::default()
        .manage(state)
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(monitors) = window.available_monitors() {
                    // Position window on second monitor if available
                    if let Some(second_monitor) = monitors.iter().find(|m| m.position().x < 0 || m.position().x >= 2500) {
                        let pos = second_monitor.position();
                        let size = second_monitor.size();
                        let target_x = pos.x + ((size.width as i32 - 1600) / 2).max(20);
                        let target_y = pos.y + ((size.height as i32 - 950) / 2).max(20);
                        let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                            x: target_x,
                            y: target_y,
                        }));
                    }
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_pipeline,
            save_pipeline,
            get_project_config_cmd,
            get_registry,
            get_cortex_tree,
            read_cortex_file,
            save_cortex_file,
            list_tasks,
            create_task,
            move_task_stage,
            run_agent,
            abort_agent,
            cleanup_task_worktree,
            get_tool_policies,
            save_tool_policies,
            compact_task_context,
            compact_context,
            steer_agent,
            get_llm_settings,
            save_llm_settings,
            select_project_folder,
            set_active_project,
            list_projects,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CanbanVoimaxHarness desktop application");
}

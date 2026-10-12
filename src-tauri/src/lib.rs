pub mod agent;
pub mod commands;
pub mod compactor;
pub mod config;
pub mod git;
pub mod hooks;
pub mod llm;

use commands::*;
use std::path::PathBuf;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let state = AppState::new(current_dir);

    tauri::Builder::default()
        .manage(state)
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running CanbanVoimaxHarness desktop application");
}

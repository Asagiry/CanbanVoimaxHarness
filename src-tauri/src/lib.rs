pub mod agent;
pub mod commands;
pub mod db;
pub mod config;
pub mod error;
pub mod git;
pub mod hooks;
pub mod llm;
pub mod udf;

pub use error::AppError;

use commands::*;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Ensure WebView2 UDF is verified available and sanitized before webviews are initialized
    #[cfg(target_os = "windows")]
    {
        if std::env::var("WEBVIEW2_USER_DATA_FOLDER").is_err() {
            let udf = udf::resolve_and_prepare_webview2_udf();
            std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", udf.to_string_lossy().as_ref());
        }
    }

    let project_root = config::discover_project_root();
    let state = AppState::new(project_root);

    tauri::Builder::default()
        // Single-instance MUST stay the first registered plugin: a second launch
        // forwards its args to the running instance and exits before WebView2
        // grabs the shared user-data folder (fixes 0x800700AA ERROR_BUSY).
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .setup(|_app| {
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_pipeline,
            save_pipeline,
            list_boards,
            create_board,
            save_board,
            delete_board,
            set_active_board,
            get_active_board_id,
            list_agent_personas,
            create_agent_persona,
            delete_agent_persona,
            rename_agent_persona,
            set_agent_persona_color,
            set_agent_persona_icon,
            get_project_config_cmd,
            get_active_project,
            get_registry,
            get_cortex_tree,
            read_cortex_file,
            save_cortex_file,
            list_cortex_agent_files,
            list_tasks,
            save_tasks,
            save_task_dialog,
            load_task_dialog,
            load_subagent_dialog,
            list_subagents,
            list_subagent_dialog_keys,
            retry_subagent,
            clear_task_dialog,
            log_ui_error,
            create_task,
            create_subtask,
            update_task,
            delete_task,
            move_task_stage,
            advance_task,
            run_agent,
            abort_agent,
            cleanup_task_worktree,
            get_task_git_diff,
            get_tool_policies,
            save_tool_policies,
            list_custom_tools,
            save_custom_tool,
            delete_custom_tool,
            compact_task_context,
            compact_context,
            get_task_context_window,
            steer_agent,
            answer_agent_question,
            get_llm_settings,
            save_llm_settings,
            fetch_available_models,
            select_project_folder,
            choose_project_folder,
            set_active_project,
            set_project_root,
            list_projects,
            open_project,
            init_project_harness,
            send_copilot_message,
            abort_copilot,
            load_copilot_history,
            clear_copilot_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CanbanVoimaxHarness desktop application");
}

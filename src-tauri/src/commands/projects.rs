use crate::commands::state::AppState;
use crate::commands::tasks::load_tasks_from_disk;
use crate::config::{
    load_project_config, load_project_registry, persist_active_project,
    register_or_update_project, scaffold_cvh_project, ProjectConfig, ProjectEntry, ProjectRegistry,
};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn get_project_config_cmd(state: State<'_, AppState>) -> Result<ProjectConfig, String> {
    let root = state.get_root();
    load_project_config(&root)
        .await
        .map_err(|e| format!("Failed to load project config: {}", e))
}

#[tauri::command]
pub async fn get_registry(registry_path: Option<String>) -> Result<ProjectRegistry, String> {
    let p = registry_path.map(PathBuf::from);
    load_project_registry(p)
        .await
        .map_err(|e| format!("Failed to load project registry: {}", e))
}

#[tauri::command]
pub async fn select_project_folder(
    path: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    if let Some(p) = path {
        if !p.trim().is_empty() {
            let path_buf = PathBuf::from(p.trim());
            if path_buf.exists() && path_buf.is_dir() {
                let _ = scaffold_cvh_project(&path_buf).await;
                persist_active_project(&path_buf).await;
                let _ = state.set_root(path_buf.clone());
                let tasks = load_tasks_from_disk(&path_buf).await;
                if let Ok(mut guard) = state.tasks.lock() {
                    *guard = tasks;
                }
                return Ok(Some(path_buf.to_string_lossy().to_string()));
            }
        }
    }

    // tauri-plugin-dialog replaces the direct rfd dependency to avoid its
    // COM/OLE STA initialization conflicting with WebView2. The blocking call
    // runs on the blocking-pool thread so the event loop is never stalled.
    let selected = tokio::task::spawn_blocking(move || {
        app_handle
            .dialog()
            .file()
            .set_title("Select Project Folder")
            .blocking_pick_folder()
    })
    .await
    .map_err(|e| format!("Folder dialog task failed: {}", e))?
    .and_then(|folder| folder.simplified().into_path().ok());

    if let Some(ref folder) = selected {
        let _ = scaffold_cvh_project(folder).await;
        persist_active_project(folder).await;
        let _ = state.set_root(folder.clone());
        let tasks = load_tasks_from_disk(folder).await;
        if let Ok(mut guard) = state.tasks.lock() {
            *guard = tasks;
        }
    }

    Ok(selected.map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn set_active_project(
    path: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<ProjectConfig, String> {
    let new_path = PathBuf::from(path.trim());
    if !new_path.exists() {
        return Err(format!("Specified project directory does not exist: {:?}", new_path));
    }
    if !new_path.is_dir() {
        return Err(format!("Specified path is not a directory: {:?}", new_path));
    }

    // Persist to %LOCALAPPDATA%\com.voimax.cvh\active_project.txt
    persist_active_project(&new_path).await;

    // Scaffold empty .cvh structure (agents/, worktrees/, skills/, settings/) if missing
    scaffold_cvh_project(&new_path)
        .await
        .map_err(|e| format!("Failed to scaffold .cvh structure: {}", e))?;

    // Update AppState repo_root using write lock
    state.set_root(new_path.clone())?;

    // Reload state.tasks for the new project root
    let new_tasks = load_tasks_from_disk(&new_path).await;
    if let Ok(mut guard) = state.tasks.lock() {
        *guard = new_tasks;
    }

    // Register or update project in global registry
    let _ = register_or_update_project(None, &new_path).await;

    // Reload or default project configuration
    let proj_cfg = match load_project_config(&new_path).await {
        Ok(cfg) => cfg,
        Err(_) => ProjectConfig {
            name: new_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("CanbanProject")
                .to_string(),
            version: "0.1.0".to_string(),
            description: None,
            active_pipeline: None,
            default_model: Some("gemini-3.8-flash-high".to_string()),
            api_base_url: Some("http://127.0.0.1:8045/v1".to_string()),
            tool_permissions: None,
            providers: HashMap::new(),
        },
    };

    // Emit event to notify frontend of project switch
    let _ = app_handle.emit(
        "project:changed",
        serde_json::json!({
            "rootPath": new_path.to_string_lossy().to_string(),
            "name": proj_cfg.name
        }),
    );

    Ok(proj_cfg)
}

#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectEntry>, String> {
    let root = state.get_root();
    let registry = load_project_registry(None)
        .await
        .unwrap_or_default();

    let mut projects = registry.projects;

    // Ensure active project is represented in the list
    let root_str = root.to_string_lossy().to_string();
    let contains_current = projects
        .iter()
        .any(|p| p.path == root_str || p.root_path.as_deref() == Some(&root_str));

    if !contains_current && root.exists() {
        let entry = ProjectEntry::from_path(&root);
        projects.insert(0, entry);
    }

    Ok(projects)
}

// Backward-compatible alias for choose_project_folder
#[tauri::command]
pub async fn choose_project_folder(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    select_project_folder(None, app_handle, state).await
}

// Backward-compatible alias for set_project_root
#[tauri::command]
pub async fn set_project_root(
    path: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<String, String> {
    persist_active_project(&PathBuf::from(path.trim())).await;
    let _cfg = set_active_project(path.clone(), state, app_handle).await?;
    Ok(path)
}

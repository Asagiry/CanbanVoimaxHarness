use crate::commands::boards::{
    insert_or_update_frontmatter_color, insert_or_update_frontmatter_icon, list_agent_personas_in,
};
use crate::commands::state::AppState;
use crate::config::{atomic_write_file, load_boards_config, load_pipeline_config};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CortexDiskFile {
    pub name: String,
    pub path: String,
    pub file_type: String,
}

#[tauri::command]
pub async fn read_cortex_file(path: String, state: State<'_, AppState>) -> Result<String, String> {
    let root = state.get_root();
    let target = if Path::new(&path).is_absolute() {
        PathBuf::from(path.clone())
    } else {
        root.join(&path)
    };

    if !target.exists() && path.contains("cvh-dev") {
        let _ = crate::config::ensure_cvh_dev_scaffold(&root).await;
    }

    if !target.exists() && path.contains("cvh-dev") && (path.ends_with("agent.md") || path.ends_with("persona.md") || path.ends_with("cvh-dev.md")) {
        return Ok(crate::agent::BUILTIN_CVH_DEV_PROMPT.to_string());
    }

    if !target.exists() && path.contains("developer") && (path.ends_with("agent.md") || path.ends_with("persona.md") || path.ends_with("developer.md")) {
        return Ok(crate::agent::BUILTIN_DEVELOPER_PROMPT.to_string());
    }

    tokio::fs::read_to_string(&target)
        .await
        .map_err(|e| format!("Failed to read cortex file {:?}: {}", target, e))
}

#[tauri::command]
pub async fn save_cortex_file(
    path: String,
    content: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let root = state.get_root();
    let target = if Path::new(&path).is_absolute() {
        PathBuf::from(&path)
    } else {
        root.join(&path)
    };

    atomic_write_file(&target, &content)
        .await
        .map_err(|e| format!("Failed to save cortex file {:?}: {}", target, e))?;

    let norm_path = path.replace('\\', "/");

    // Двусторонняя синхронизация icon и color в persona.md при сохранении setting агента
    if norm_path.ends_with("/setting") && (norm_path.contains(".cvh/agents/") || norm_path.contains("/agents/")) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(val_obj) = val.as_object() {
                let icon_opt = val_obj.get("icon").and_then(|v| v.as_str()).map(|s| s.trim()).filter(|s| !s.is_empty());
                let color_opt = val_obj.get("color").and_then(|v| v.as_str()).map(|s| s.trim()).filter(|s| !s.is_empty());

                if icon_opt.is_some() || color_opt.is_some() {
                    if let Some(parent) = target.parent() {
                        let persona_md = parent.join("persona.md");
                        let agent_md = parent.join("agent.md");
                        let target_persona = if persona_md.exists() {
                            Some(persona_md)
                        } else if agent_md.exists() {
                            Some(agent_md)
                        } else {
                            Some(persona_md)
                        };

                        if let Some(p_path) = target_persona {
                            let p_content = tokio::fs::read_to_string(&p_path).await.unwrap_or_default();
                            let mut updated = p_content;
                            if let Some(icon) = icon_opt {
                                updated = insert_or_update_frontmatter_icon(&updated, icon);
                            }
                            if let Some(color) = color_opt {
                                updated = insert_or_update_frontmatter_color(&updated, color);
                            }
                            let _ = atomic_write_file(&p_path, &updated).await;
                        }
                    }
                }
            }
        }
    }

    if norm_path.ends_with("boards.json") {
        if let Ok(bc) = load_boards_config(&root).await {
            let _ = app.emit(
                "boards:updated",
                serde_json::json!({ "boards": &bc.boards, "activeBoardId": &bc.active_board_id }),
            );
        }
        if let Ok(pipe) = load_pipeline_config(&root).await {
            let _ = app.emit("pipeline:updated", serde_json::json!({ "pipeline": &pipe }));
        }
    } else if norm_path.ends_with("pipeline.json") {
        if let Ok(pipe) = load_pipeline_config(&root).await {
            let _ = app.emit("pipeline:updated", serde_json::json!({ "pipeline": &pipe }));
        }
        if let Ok(bc) = load_boards_config(&root).await {
            let _ = app.emit(
                "boards:updated",
                serde_json::json!({ "boards": &bc.boards, "activeBoardId": &bc.active_board_id }),
            );
        }
    } else if norm_path.contains(".cvh/agents") || norm_path.contains("/agents/") {
        let list = list_agent_personas_in(&root).await;
        let _ = app.emit("agent:personas_updated", &list);
    }

    if norm_path.contains(".cvh/rules") || norm_path.contains("/rules/") {
        let _ = app.emit("cortex:rules_updated", serde_json::json!({ "path": &norm_path }));
    }

    Ok(())
}

pub async fn list_global_rules_in(root: &Path) -> Result<Vec<CortexDiskFile>, String> {
    let global_rules_dir = crate::config::global_rules_dir(root);
    if !global_rules_dir.exists() {
        let _ = tokio::fs::create_dir_all(&global_rules_dir).await;
    }
    let mut files = Vec::new();
    if let Ok(mut entries) = tokio::fs::read_dir(&global_rules_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let p = entry.path();
            if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("md") {
                if let Some(name) = entry.file_name().to_str() {
                    files.push(CortexDiskFile {
                        name: name.to_string(),
                        path: format!(".cvh/rules/{}", name),
                        file_type: "rule".to_string(),
                    });
                }
            }
        }
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}

#[tauri::command]
pub async fn list_global_rules(
    state: State<'_, AppState>,
) -> Result<Vec<CortexDiskFile>, String> {
    let root = state.get_root();
    list_global_rules_in(&root).await
}

pub async fn delete_cortex_file_at(root: &Path, rel_path: &str) -> Result<(), String> {
    let norm = rel_path.replace('\\', "/");
    let trimmed = norm.trim_start_matches('/');

    // Security invariant: only files inside .cvh/rules/, .cvh/agents/, or .cvh/hooks/ may be deleted
    if !trimmed.starts_with(".cvh/rules/")
        && !trimmed.starts_with(".cvh/agents/")
        && !trimmed.starts_with(".cvh/hooks/")
    {
        return Err("Forbidden: file path must be inside .cvh/rules/, .cvh/agents/, or .cvh/hooks/".to_string());
    }

    // Security invariant: strictly reject path traversal attempts
    if Path::new(trimmed).components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err("Forbidden: directory traversal detected".to_string());
    }

    let target = root.join(trimmed);
    if !target.exists() {
        return Err(format!("File does not exist: {}", rel_path));
    }

    let root_canon = root.canonicalize().map_err(|e| format!("Root path error: {}", e))?;
    let target_canon = target.canonicalize().map_err(|e| format!("Target path error: {}", e))?;
    if !target_canon.starts_with(&root_canon) {
        return Err("Forbidden: path outside repository root".to_string());
    }

    tokio::fs::remove_file(&target_canon)
        .await
        .map_err(|e| format!("Failed to delete cortex file {:?}: {}", target_canon, e))?;

    Ok(())
}

#[tauri::command]
pub async fn delete_cortex_file(
    path: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let root = state.get_root();
    delete_cortex_file_at(&root, &path).await?;

    let norm = path.replace('\\', "/");
    let trimmed = norm.trim_start_matches('/');
    if trimmed.starts_with(".cvh/rules/") || trimmed.contains("/rules/") {
        let _ = app.emit("cortex:rules_updated", serde_json::json!({ "path": &path }));
    }
    if trimmed.starts_with(".cvh/agents/") {
        let list = list_agent_personas_in(&root).await;
        let _ = app.emit("agent:personas_updated", &list);
    }

    Ok(())
}

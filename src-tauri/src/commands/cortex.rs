use crate::commands::boards::list_agent_personas_in;
use crate::commands::state::AppState;
use crate::config::{atomic_write_file, load_boards_config, load_pipeline_config};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CortexNode {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(rename = "isDir")]
    pub is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<CortexNode>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CortexDiskFile {
    pub name: String,
    pub path: String,
    pub file_type: String,
}

#[tauri::command]
pub async fn get_cortex_tree(state: State<'_, AppState>) -> Result<Vec<CortexNode>, String> {
    let root = state.get_root();
    let mut root_nodes = Vec::new();

    // 1. Column Personas (.cvh/agents/)
    let agents_dir = root.join(".cvh").join("agents");
    if agents_dir.exists() {
        let mut persona_children = Vec::new();
        if let Ok(mut entries) = tokio::fs::read_dir(&agents_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let name = entry.file_name().to_string_lossy().to_string();
                let rel_path = format!(".cvh/agents/{}", name);
                persona_children.push(CortexNode {
                    id: format!("persona-{}", name),
                    name,
                    path: rel_path,
                    is_dir: false,
                    children: None,
                });
            }
        }
        root_nodes.push(CortexNode {
            id: "cortex-personas".to_string(),
            name: "Personas".to_string(),
            path: ".cvh/agents".to_string(),
            is_dir: true,
            children: Some(persona_children),
        });
    }

    // 2. Transition Hooks (.cvh/hooks/)
    let hooks_dir = root.join(".cvh").join("hooks");
    if hooks_dir.exists() {
        let mut hook_children = Vec::new();
        if let Ok(mut entries) = tokio::fs::read_dir(&hooks_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let name = entry.file_name().to_string_lossy().to_string();
                let rel_path = format!(".cvh/hooks/{}", name);
                hook_children.push(CortexNode {
                    id: format!("hook-{}", name),
                    name,
                    path: rel_path,
                    is_dir: false,
                    children: None,
                });
            }
        }
        root_nodes.push(CortexNode {
            id: "cortex-hooks".to_string(),
            name: "Transition Hooks".to_string(),
            path: ".cvh/hooks".to_string(),
            is_dir: true,
            children: Some(hook_children),
        });
    }

    // 3. Skills Repository (.agents/skills/)
    let skills_dir = root.join(".agents").join("skills");
    if skills_dir.exists() {
        let mut skill_children = Vec::new();
        if let Ok(mut entries) = tokio::fs::read_dir(&skills_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
                    let s_name = entry.file_name().to_string_lossy().to_string();
                    let skill_doc = format!(".agents/skills/{}/SKILL.md", s_name);
                    skill_children.push(CortexNode {
                        id: format!("skill-{}", s_name),
                        name: s_name,
                        path: skill_doc,
                        is_dir: false,
                        children: None,
                    });
                }
            }
        }
        root_nodes.push(CortexNode {
            id: "cortex-skills".to_string(),
            name: "Skills & Rules".to_string(),
            path: ".agents/skills".to_string(),
            is_dir: true,
            children: Some(skill_children),
        });
    }

    // 4. Invariants (AGENTS.md)
    let agents_md = root.join("AGENTS.md");
    if agents_md.exists() {
        root_nodes.push(CortexNode {
            id: "cortex-agents-md".to_string(),
            name: "AGENTS.md (Invariants)".to_string(),
            path: "AGENTS.md".to_string(),
            is_dir: false,
            children: None,
        });
    }

    Ok(root_nodes)
}

#[tauri::command]
pub async fn list_cortex_agent_files(
    role: String,
    state: State<'_, AppState>,
) -> Result<Vec<CortexDiskFile>, String> {
    let root = state.get_root();
    let role_dir = crate::config::agent_dir(&root, &role);
    let mut files = Vec::new();

    let persona_p = role_dir.join("persona.md");
    let agent_p = role_dir.join("agent.md");
    if persona_p.is_file() {
        files.push(CortexDiskFile {
            name: "persona.md".to_string(),
            path: format!(".cvh/agents/{}/persona.md", role),
            file_type: "persona".to_string(),
        });
    } else if agent_p.is_file() {
        files.push(CortexDiskFile {
            name: "agent.md".to_string(),
            path: format!(".cvh/agents/{}/agent.md", role),
            file_type: "persona".to_string(),
        });
    }

    let rules_dir = role_dir.join("rules");
    if let Ok(mut entries) = tokio::fs::read_dir(&rules_dir).await {
        let mut r_files = Vec::new();
        while let Ok(Some(entry)) = entries.next_entry().await {
            let p = entry.path();
            if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("md") {
                if let Some(name) = entry.file_name().to_str() {
                    r_files.push(name.to_string());
                }
            }
        }
        r_files.sort();
        for r in r_files {
            files.push(CortexDiskFile {
                name: r.clone(),
                path: format!(".cvh/agents/{}/rules/{}", role, r),
                file_type: "rule".to_string(),
            });
        }
    }

    let setting_p = role_dir.join("setting");
    if setting_p.is_file() {
        files.push(CortexDiskFile {
            name: "setting".to_string(),
            path: format!(".cvh/agents/{}/setting", role),
            file_type: "settings".to_string(),
        });
    }

    Ok(files)
}

#[tauri::command]
pub async fn read_cortex_file(path: String, state: State<'_, AppState>) -> Result<String, String> {
    let root = state.get_root();
    let target = if Path::new(&path).is_absolute() {
        PathBuf::from(path.clone())
    } else {
        root.join(&path)
    };

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

    Ok(())
}

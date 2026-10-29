//! File I/O operations for atomic file writes, board, pipeline, and settings persistence.

use anyhow::{Context, Result};
use chrono::Utc;
use std::path::{Path, PathBuf};

use crate::config::discovery::get_app_config_dir;
use crate::config::models::*;

pub async fn load_boards_config(root_path: impl AsRef<Path>) -> Result<BoardsConfig> {
    let root = root_path.as_ref();
    let boards_file = root.join(".cvh").join("boards.json");
    if boards_file.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&boards_file).await {
            if let Ok(cfg) = serde_json::from_str::<BoardsConfig>(&content) {
                if !cfg.boards.is_empty() {
                    return Ok(cfg);
                }
            }
        }
    }

    let pipeline_file = root.join(".cvh").join("pipeline.json");
    if pipeline_file.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&pipeline_file).await {
            if let Ok(pipe) = serde_json::from_str::<PipelineConfig>(&content) {
                if let Some(boards) = pipe.boards {
                    if !boards.is_empty() {
                        let active = pipe.active_board_id.unwrap_or_else(default_board_id);
                        return Ok(BoardsConfig {
                            active_board_id: active,
                            boards,
                        });
                    }
                }
                let def_board = BoardDef {
                    id: "development".to_string(),
                    name: if pipe.title.trim().is_empty() { "Разработка".to_string() } else { pipe.title },
                    columns: if pipe.columns.is_empty() { default_pipeline_config().columns } else { pipe.columns },
                };
                return Ok(BoardsConfig {
                    active_board_id: "development".to_string(),
                    boards: vec![def_board],
                });
            }
        }
    }

    Ok(default_boards_config())
}


pub async fn save_boards_config(root_path: impl AsRef<Path>, config: &BoardsConfig) -> Result<()> {
    let root = root_path.as_ref();
    let mut normalized_config = config.clone();
    for board in &mut normalized_config.boards {
        board.columns = normalize_pipeline_columns(std::mem::take(&mut board.columns))
            .map_err(|e| anyhow::anyhow!("{}", e))?;
    }
    let boards_file = root.join(".cvh").join("boards.json");
    let serialized_boards = serde_json::to_string_pretty(&normalized_config)
        .context("Failed to serialize BoardsConfig")?;
    atomic_write_file(&boards_file, &serialized_boards).await?;

    let mut pipeline = load_pipeline_config(root).await.unwrap_or_else(|_| default_pipeline_config());
    pipeline.boards = Some(normalized_config.boards.clone());
    pipeline.active_board_id = Some(normalized_config.active_board_id.clone());
    if let Some(active_board) = normalized_config.boards.iter().find(|b| b.id == normalized_config.active_board_id) {
        pipeline.columns = active_board.columns.clone();
        pipeline.title = active_board.name.clone();
    }
    save_pipeline_config(root, &pipeline).await?;

    Ok(())
}


pub fn get_app_llm_settings_path() -> PathBuf {
    get_app_config_dir().join("llm.json")
}

pub async fn load_llm_settings(repo_root: &Path) -> Result<LlmSettings> {
    // 1. Project-local .cvh/settings/llm.json or .cvh/llm.json (if present)
    let settings_dir_path = repo_root.join(".cvh").join("settings").join("llm.json");
    if settings_dir_path.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&settings_dir_path).await {
            if let Ok(settings) = serde_json::from_str::<LlmSettings>(&content) {
                return Ok(settings);
            }
        }
    }

    let local_path = repo_root.join(".cvh").join("llm.json");
    if local_path.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&local_path).await {
            if let Ok(settings) = serde_json::from_str::<LlmSettings>(&content) {
                return Ok(settings);
            }
        }
    }

    // 2. Global app config (%LOCALAPPDATA%\com.voimax.cvh\llm.json)
    let global_path = get_app_llm_settings_path();
    if global_path.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&global_path).await {
            if let Ok(settings) = serde_json::from_str::<LlmSettings>(&content) {
                return Ok(settings);
            }
        }
    }

    // 3. Fallback to default
    Ok(LlmSettings::default())
}

pub async fn save_llm_settings_to_disk(repo_root: &Path, settings: &LlmSettings) -> Result<()> {
    let serialized = serde_json::to_string_pretty(settings)
        .context("Failed to serialize LlmSettings to JSON")?;

    // Always persist to global app config directory (%LOCALAPPDATA%\com.voimax.cvh\llm.json)
    let global_path = get_app_llm_settings_path();
    atomic_write_file(&global_path, &serialized)
        .await
        .with_context(|| format!("Failed to write global LLM settings to {:?}", global_path))?;

    // If repo has .cvh directory, also mirror to .cvh/llm.json and .cvh/settings/llm.json
    let repo_cvh = repo_root.join(".cvh");
    if repo_cvh.exists() {
        let local_path = repo_cvh.join("llm.json");
        let _ = atomic_write_file(&local_path, &serialized).await;

        let settings_dir = repo_cvh.join("settings");
        if settings_dir.exists() {
            let settings_path = settings_dir.join("llm.json");
            let _ = atomic_write_file(&settings_path, &serialized).await;
        }
    }

    Ok(())
}

// ============================================================================
// Agent storage: persona — ЕДИНСТВЕННЫЙ flat-путь `.cvh/agents/<role>.md`.
// Per-role aux dirs `.cvh/agents/<role>/` (без agent.md внутри):
//   - `rules/*.md`    — agent rules, alphabetical order in system prompt
//   - `hooks/`        — scripts named `on_enter.*` / `on_finish.*` by event
//   - `setting`       — JSON without extension:
//                       {model?, temperature?, context_window?, allowed_tools?}
// ============================================================================



/// Best-effort load of `.cvh/agents/<role>/setting`.
/// Missing/empty/broken file → [`AgentSetting::default()`] (never an error).
/// A present `context_window` is logged via eprintln for observability.
pub async fn load_agent_setting(repo_root: &Path, role: &str) -> AgentSetting {
    let path = agent_setting_path(repo_root, role);
    let content = match tokio::fs::read_to_string(&path).await {
        Ok(c) => c,
        Err(_) => return AgentSetting::default(),
    };
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return AgentSetting::default();
    }
    match serde_json::from_str::<AgentSetting>(trimmed) {
        Ok(s) => {
            if let Some(cw) = s.context_window {
                eprintln!(
                    "CVH [agent_setting]: role='{}' context_window={}",
                    role, cw
                );
            }
            s
        }
        Err(e) => {
            eprintln!(
                "CVH [agent_setting]: broken setting for role='{}' at {:?}: {}. Using defaults.",
                role, path, e
            );
            AgentSetting::default()
        }
    }
}


pub async fn atomic_write_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create parent directory for {:?}", path))?;
    }

    let tmp_path = path.with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));
    let atomic_res: Result<()> = async {
        tokio::fs::write(&tmp_path, content)
            .await
            .with_context(|| format!("Failed to write temporary file {:?}", tmp_path))?;

        tokio::fs::rename(&tmp_path, path)
            .await
            .with_context(|| format!("Failed to replace destination file {:?} with temporary file {:?}", path, tmp_path))?;

        Ok(())
    }
    .await;

    if let Err(atomic_err) = atomic_res {
        eprintln!(
            "CVH [atomic_write_file]: Atomic write failed for {:?} ({:#}), falling back to direct write",
            path, atomic_err
        );
        let _ = tokio::fs::remove_file(&tmp_path).await;
        tokio::fs::write(path, content)
            .await
            .with_context(|| format!("Failed to write file directly to {:?} (fallback after atomic error: {:#})", path, atomic_err))?;
    }

    Ok(())
}


pub async fn load_project_config(root_path: impl AsRef<Path>) -> Result<ProjectConfig> {
    let config_path = root_path.as_ref().join(".cvh").join("config.json");
    let content = tokio::fs::read_to_string(&config_path)
        .await
        .with_context(|| format!("Failed to read project config at {:?}", config_path))?;

    let config: ProjectConfig = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse project config JSON at {:?}", config_path))?;

    Ok(config)
}

pub async fn load_pipeline_config(root_path: impl AsRef<Path>) -> Result<PipelineConfig> {
    let pipeline_path = root_path.as_ref().join(".cvh").join("pipeline.json");
    if !pipeline_path.exists() {
        return Ok(default_pipeline_config());
    }

    match tokio::fs::read_to_string(&pipeline_path).await {
        Ok(content) => match serde_json::from_str::<PipelineConfig>(&content) {
            Ok(pipeline) => Ok(pipeline),
            Err(e) => {
                eprintln!(
                    "CVH [load_pipeline_config]: Failed to parse pipeline JSON at {:?}: {}. Falling back to default 5-stage pipeline.",
                    pipeline_path, e
                );
                Ok(default_pipeline_config())
            }
        },
        Err(e) => {
            eprintln!(
                "CVH [load_pipeline_config]: Failed to read pipeline file at {:?}: {}. Falling back to default 5-stage pipeline.",
                pipeline_path, e
            );
            Ok(default_pipeline_config())
        }
    }
}

pub async fn save_pipeline_config(root_path: impl AsRef<Path>, config: &PipelineConfig) -> Result<()> {
    let root = root_path.as_ref();
    let mut normalized = config.clone();
    normalized.columns = normalize_pipeline_columns(std::mem::take(&mut normalized.columns))
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    if let Some(ref mut boards) = normalized.boards {
        for b in boards.iter_mut() {
            b.columns = normalize_pipeline_columns(std::mem::take(&mut b.columns))
                .map_err(|e| anyhow::anyhow!("{}", e))?;
        }
    }
    let pipeline_path = root.join(".cvh").join("pipeline.json");
    let serialized = serde_json::to_string_pretty(&normalized)
        .context("Failed to serialize PipelineConfig to JSON")?;
    atomic_write_file(&pipeline_path, &serialized).await?;

    let boards_file = root.join(".cvh").join("boards.json");
    if let Some(ref boards) = normalized.boards {
        let boards_cfg = BoardsConfig {
            active_board_id: normalized.active_board_id.clone().unwrap_or_else(default_board_id),
            boards: boards.clone(),
        };
        if let Ok(serialized_boards) = serde_json::to_string_pretty(&boards_cfg) {
            let _ = atomic_write_file(&boards_file, &serialized_boards).await;
        }
    } else if let Ok(mut boards_cfg) = load_boards_config(root).await {
        if let Some(active_board) = boards_cfg.boards.iter_mut().find(|b| b.id == boards_cfg.active_board_id) {
            active_board.columns = normalized.columns.clone();
            if let Ok(serialized_boards) = serde_json::to_string_pretty(&boards_cfg) {
                let _ = atomic_write_file(&boards_file, &serialized_boards).await;
            }
        }
    }
    Ok(())
}


pub fn get_default_registry_path() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".cvh").join("registry.json")
}

pub async fn load_project_registry(registry_path: Option<PathBuf>) -> Result<ProjectRegistry> {
    let path = registry_path.unwrap_or_else(get_default_registry_path);
    if !path.exists() {
        return Ok(ProjectRegistry::default());
    }

    let content = tokio::fs::read_to_string(&path)
        .await
        .with_context(|| format!("Failed to read project registry at {:?}", path))?;

    let registry: ProjectRegistry = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse project registry at {:?}", path))?;

    Ok(registry)
}

pub async fn save_project_registry(registry_path: Option<PathBuf>, registry: &ProjectRegistry) -> Result<()> {
    let path = registry_path.unwrap_or_else(get_default_registry_path);
    let serialized = serde_json::to_string_pretty(registry)
        .context("Failed to serialize ProjectRegistry to JSON")?;
    atomic_write_file(&path, &serialized).await
}

pub async fn register_or_update_project(
    registry_path: Option<PathBuf>,
    project_path: &Path,
) -> Result<ProjectRegistry> {
    let mut registry = load_project_registry(registry_path.clone()).await.unwrap_or_default();
    let norm_path = project_path.to_string_lossy().to_string();

    let mut found = false;
    for entry in &mut registry.projects {
        if entry.path == norm_path || entry.root_path.as_deref() == Some(&norm_path) {
            entry.last_opened = Utc::now();
            entry.root_path = Some(norm_path.clone());
            registry.active_project_id = Some(entry.id.clone());
            found = true;
            break;
        }
    }

    if !found {
        let new_entry = ProjectEntry::from_path(project_path);
        let id = new_entry.id.clone();
        registry.projects.push(new_entry);
        registry.active_project_id = Some(id);
    }

    let _ = save_project_registry(registry_path, &registry).await;
    Ok(registry)
}

// ============================================================================
// Tool Policy Matrix Configuration
// ============================================================================

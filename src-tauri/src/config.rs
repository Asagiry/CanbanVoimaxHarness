use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(rename = "type")]
    pub provider_type: String,
    #[serde(rename = "baseUrl")]
    pub base_url: String,
    #[serde(rename = "defaultModel", default)]
    pub default_model: Option<String>,
    #[serde(rename = "apiKey", default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ToolPermissions {
    #[serde(rename = "globalAllowedTools", default)]
    pub global_allowed_tools: Vec<String>,
    #[serde(rename = "stageAllowedTools", default)]
    pub stage_allowed_tools: HashMap<String, Vec<String>>,
}

pub fn normalize_tool_name(name: &str) -> &str {
    match name {
        "bash" => "execute_command",
        "str_replace" => "edit_file",
        other => other,
    }
}

impl ToolPermissions {
    pub fn new(
        global_allowed_tools: Vec<String>,
        stage_allowed_tools: HashMap<String, Vec<String>>,
    ) -> Self {
        Self {
            global_allowed_tools,
            stage_allowed_tools,
        }
    }

    /// Check if tool (or its SWE-bench alias) is permitted for the given stage_id.
    ///
    /// AND-semantics: `effective = global && stage` (a missing stage entry
    /// defaults to `true`, an empty global list means "no global restriction").
    /// A global disable can no longer be overridden back on by a stage entry.
    pub fn is_tool_allowed(&self, stage_id: &str, tool_name: &str) -> bool {
        let canonical = normalize_tool_name(tool_name);

        let global_ok = if self.global_allowed_tools.is_empty() {
            true
        } else {
            self.global_allowed_tools.iter().any(|t| {
                let ct = normalize_tool_name(t);
                ct == canonical || t == tool_name
            })
        };

        let stage_ok = match self.stage_allowed_tools.get(stage_id) {
            Some(stage_tools) => stage_tools.iter().any(|t| {
                let ct = normalize_tool_name(t);
                ct == canonical || t == tool_name
            }),
            // No stage record: no stage-level restriction.
            None => true,
        };

        global_ok && stage_ok
    }

    pub fn get_allowed_tools_for_stage(&self, stage_id: &str) -> Option<Vec<String>> {
        if let Some(tools) = self.stage_allowed_tools.get(stage_id) {
            Some(tools.clone())
        } else if !self.global_allowed_tools.is_empty() {
            Some(self.global_allowed_tools.clone())
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "activePipeline", default)]
    pub active_pipeline: Option<String>,
    #[serde(rename = "defaultModel", default)]
    pub default_model: Option<String>,
    #[serde(rename = "apiBaseUrl", default)]
    pub api_base_url: Option<String>,
    #[serde(rename = "toolPermissions", default)]
    pub tool_permissions: Option<ToolPermissions>,
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PipelineColumn {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub persona: Option<String>,
    #[serde(rename = "onEnterScript", default)]
    pub on_enter_script: Option<String>,
    #[serde(rename = "contextWindow", default)]
    pub context_window: Option<u64>,
    #[serde(rename = "modelOverride", default)]
    pub model_override: Option<String>,
    #[serde(rename = "reasoningEffort", default)]
    pub reasoning_effort: Option<String>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(rename = "gatekeeperSensor", default)]
    pub gatekeeper_sensor: Option<String>,
    #[serde(rename = "allowedTools", default)]
    pub allowed_tools: Option<Vec<String>>,
    #[serde(default)]
    pub hooks: HashMap<String, String>,
    #[serde(default)]
    pub routes: Option<HashMap<String, String>>,
    #[serde(rename = "autoRun", default)]
    pub auto_run: Option<bool>,
    #[serde(default)]
    pub terminal: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoardDef {
    pub id: String,
    pub name: String,
    pub columns: Vec<PipelineColumn>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardsConfig {
    #[serde(rename = "activeBoardId", default = "default_board_id")]
    pub active_board_id: String,
    #[serde(default)]
    pub boards: Vec<BoardDef>,
}

pub fn default_board_id() -> String {
    "development".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub id: String,
    pub title: String,
    #[serde(rename = "toolPermissions", default)]
    pub tool_permissions: Option<ToolPermissions>,
    pub columns: Vec<PipelineColumn>,
    #[serde(default)]
    pub boards: Option<Vec<BoardDef>>,
    #[serde(rename = "activeBoardId", default)]
    pub active_board_id: Option<String>,
}

pub fn resolve_allowed_tools(
    pipeline: Option<&PipelineConfig>,
    project: Option<&ProjectConfig>,
    stage_id: &str,
) -> Option<Vec<String>> {
    // 1. Column-specific allowedTools
    if let Some(pipe) = pipeline {
        if let Some(col) = pipe.columns.iter().find(|c| c.id == stage_id) {
            if let Some(ref tools) = col.allowed_tools {
                return Some(tools.clone());
            }
        }
        // 2. Pipeline-level stageAllowedTools / globalAllowedTools
        if let Some(ref tp) = pipe.tool_permissions {
            if let Some(tools) = tp.get_allowed_tools_for_stage(stage_id) {
                return Some(tools);
            }
        }
    }
    // 3. Project-level stageAllowedTools / globalAllowedTools
    if let Some(proj) = project {
        if let Some(ref tp) = proj.tool_permissions {
            if let Some(tools) = tp.get_allowed_tools_for_stage(stage_id) {
                return Some(tools);
            }
        }
    }
    None
}

// ============================================================================
// LLM Connection & Model Settings
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmSettings {
    #[serde(default = "default_llm_provider")]
    pub provider: String,
    #[serde(rename = "apiKey", alias = "api_key", default = "default_llm_api_key")]
    pub api_key: String,
    #[serde(rename = "baseUrl", alias = "base_url", default = "default_llm_base_url")]
    pub base_url: String,
    #[serde(default = "default_llm_model")]
    pub model: String,
    #[serde(default)]
    pub temperature: Option<f32>,
}

fn default_llm_provider() -> String {
    "antigravity".to_string()
}

fn default_llm_api_key() -> String {
    String::new()
}

fn default_llm_base_url() -> String {
    "http://127.0.0.1:8045/v1".to_string()
}

fn default_llm_model() -> String {
    "gemini-3.8-flash-high".to_string()
}

impl Default for LlmSettings {
    fn default() -> Self {
        Self {
            provider: default_llm_provider(),
            api_key: default_llm_api_key(),
            base_url: default_llm_base_url(),
            model: default_llm_model(),
            temperature: Some(0.2),
        }
    }
}

pub fn get_app_config_dir() -> PathBuf {
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(local_app_data).join("com.voimax.cvh")
    } else if let Ok(app_data) = std::env::var("APPDATA") {
        PathBuf::from(app_data).join("com.voimax.cvh")
    } else if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        PathBuf::from(home).join(".cvh")
    } else {
        PathBuf::from(".cvh")
    }
}

pub fn discover_project_root() -> PathBuf {
    let current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // a) Check std::env::current_dir(). If it has .cvh or .git, return it.
    if current.join(".cvh").exists() || current.join(".git").exists() {
        eprintln!(
            "CVH [discover_project_root]: Found .cvh or .git in current_dir: {:?}",
            current
        );
        return current;
    }

    // b) Traverse parent directories (current_dir.ancestors()) looking for .cvh or .git.
    // If found (e.g. going up from src-tauri/target/release to CanbanVoimaxHarness), return that ancestor directory!
    for ancestor in current.ancestors() {
        if ancestor.join(".cvh").exists() || ancestor.join(".git").exists() {
            eprintln!(
                "CVH [discover_project_root]: Found project root in ancestor: {:?}",
                ancestor
            );
            return ancestor.to_path_buf();
        }
    }

    // c) If not found in ancestors, check %LOCALAPPDATA%\com.voimax.cvh\active_project.txt.
    // If it contains a valid directory path that exists, return it.
    let active_project_file = get_app_config_dir().join("active_project.txt");
    if active_project_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&active_project_file) {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                let p = PathBuf::from(trimmed);
                if p.exists() && p.is_dir() {
                    eprintln!(
                        "CVH [discover_project_root]: Using active project from {:?}: {:?}",
                        active_project_file, p
                    );
                    return p;
                }
            }
        }
    }

    // d) If still not found, check if C:\Users\Voimax\Projects\CanbanVoimaxHarness exists; if so, return it.
    let hardcoded_default = PathBuf::from(r"C:\Users\Voimax\Projects\CanbanVoimaxHarness");
    if hardcoded_default.exists() && hardcoded_default.is_dir() {
        eprintln!(
            "CVH [discover_project_root]: Using default path: {:?}",
            hardcoded_default
        );
        return hardcoded_default;
    }

    // e) Otherwise fallback to std::env::current_dir().
    eprintln!(
        "CVH [discover_project_root]: Fallback to current_dir: {:?}",
        current
    );
    current
}

pub async fn persist_active_project(path: &Path) {
    let dir = get_app_config_dir();
    let _ = tokio::fs::create_dir_all(&dir).await;
    let target = dir.join("active_project.txt");
    if let Err(e) = tokio::fs::write(&target, path.to_string_lossy().as_bytes()).await {
        eprintln!(
            "CVH [persist_active_project]: Failed to write active_project.txt: {}",
            e
        );
    } else {
        eprintln!(
            "CVH [persist_active_project]: Saved active project to {:?}",
            target
        );
    }
}

pub fn default_pipeline_config() -> PipelineConfig {
    PipelineConfig {
        id: "default-engineering".to_string(),
        title: "Engineering Pipeline".to_string(),
        tool_permissions: None,
        columns: vec![
            PipelineColumn {
                id: "backlog".to_string(),
                title: "Backlog".to_string(),
                color: Some("#64748b".to_string()),
                icon: Some("Inbox".to_string()),
                persona: None,
                on_enter_script: None,
                context_window: Some(128000),
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: Some(vec![
                    "read_file".to_string(),
                    "glob".to_string(),
                    "grep".to_string(),
                    "web_search".to_string(),
                    "web_fetch".to_string(),
                ]),
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
            },
            PipelineColumn {
                id: "dev".to_string(),
                title: "In Development".to_string(),
                color: Some("#f59e0b".to_string()),
                icon: Some("Code2".to_string()),
                persona: Some("developer".to_string()),
                on_enter_script: None,
                context_window: Some(400000),
                model_override: Some("gemini-3.8-flash-high".to_string()),
                reasoning_effort: None,
                temperature: Some(0.2),
                gatekeeper_sensor: None,
                allowed_tools: Some(vec![
                    "execute_command".to_string(),
                    "read_file".to_string(),
                    "edit_file".to_string(),
                    "write_file".to_string(),
                    "glob".to_string(),
                    "grep".to_string(),
                    "subagent".to_string(),
                ]),
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
            },
            PipelineColumn {
                id: "review".to_string(),
                title: "Review".to_string(),
                color: Some("#8b5cf6".to_string()),
                icon: Some("CheckSquare".to_string()),
                persona: Some("reviewer".to_string()),
                on_enter_script: None,
                context_window: Some(200000),
                model_override: Some("gemini-3.8-flash-high".to_string()),
                reasoning_effort: None,
                temperature: Some(0.1),
                gatekeeper_sensor: None,
                allowed_tools: Some(vec![
                    "execute_command".to_string(),
                    "read_file".to_string(),
                    "glob".to_string(),
                    "grep".to_string(),
                    "subagent".to_string(),
                ]),
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
            },
            PipelineColumn {
                id: "retro".to_string(),
                title: "Retrospective".to_string(),
                color: Some("#ec4899".to_string()),
                icon: Some("History".to_string()),
                persona: Some("retro".to_string()),
                on_enter_script: None,
                context_window: Some(128000),
                model_override: Some("gemini-3.8-flash-high".to_string()),
                reasoning_effort: None,
                temperature: Some(0.2),
                gatekeeper_sensor: None,
                allowed_tools: Some(vec![
                    "read_file".to_string(),
                    "glob".to_string(),
                    "grep".to_string(),
                    "write_file".to_string(),
                    "edit_file".to_string(),
                ]),
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
            },
            PipelineColumn {
                id: "ready".to_string(),
                title: "Ready / Done".to_string(),
                color: Some("#10b981".to_string()),
                icon: Some("CheckCircle2".to_string()),
                persona: None,
                on_enter_script: None,
                context_window: Some(64000),
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: Some(vec![
                    "read_file".to_string(),
                    "glob".to_string(),
                    "grep".to_string(),
                ]),
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
            },
        ],
        boards: None,
        active_board_id: None,
    }
}

pub fn default_board_def() -> BoardDef {
    BoardDef {
        id: "development".to_string(),
        name: "Разработка".to_string(),
        columns: default_pipeline_config().columns,
    }
}

pub fn default_boards_config() -> BoardsConfig {
    BoardsConfig {
        active_board_id: "development".to_string(),
        boards: vec![default_board_def()],
    }
}

pub fn get_board_template_columns(template: &str) -> Vec<PipelineColumn> {
    match template.to_lowercase().as_str() {
        "modeler" | "3d" | "3d-modeler" => vec![
            PipelineColumn {
                id: "concept".to_string(),
                title: "Концепт".to_string(),
                color: Some("#a855f7".to_string()),
                icon: Some("Lightbulb".to_string()),
                persona: Some("concept-artist".to_string()),
                on_enter_script: None,
                context_window: Some(200000),
                model_override: Some("gemini-3.8-flash-high".to_string()),
                reasoning_effort: None,
                temperature: Some(0.4),
                gatekeeper_sensor: None,
                allowed_tools: Some(vec![
                    "web_search".to_string(),
                    "web_fetch".to_string(),
                    "read_file".to_string(),
                ]),
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
            },
            PipelineColumn {
                id: "modeling".to_string(),
                title: "Моделирование".to_string(),
                color: Some("#3b82f6".to_string()),
                icon: Some("Box".to_string()),
                persona: Some("modeler".to_string()),
                on_enter_script: None,
                context_window: Some(300000),
                model_override: Some("gemini-3.8-flash-high".to_string()),
                reasoning_effort: None,
                temperature: Some(0.2),
                gatekeeper_sensor: None,
                allowed_tools: Some(vec![
                    "execute_command".to_string(),
                    "read_file".to_string(),
                    "edit_file".to_string(),
                    "write_file".to_string(),
                    "glob".to_string(),
                    "grep".to_string(),
                ]),
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
            },
            PipelineColumn {
                id: "texturing".to_string(),
                title: "Текстурирование".to_string(),
                color: Some("#f59e0b".to_string()),
                icon: Some("Palette".to_string()),
                persona: Some("texture-artist".to_string()),
                on_enter_script: None,
                context_window: Some(200000),
                model_override: Some("gemini-3.8-flash-high".to_string()),
                reasoning_effort: None,
                temperature: Some(0.2),
                gatekeeper_sensor: None,
                allowed_tools: Some(vec![
                    "execute_command".to_string(),
                    "read_file".to_string(),
                    "edit_file".to_string(),
                    "write_file".to_string(),
                    "glob".to_string(),
                    "grep".to_string(),
                ]),
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
            },
            PipelineColumn {
                id: "ready".to_string(),
                title: "Готово".to_string(),
                color: Some("#10b981".to_string()),
                icon: Some("CheckCircle2".to_string()),
                persona: None,
                on_enter_script: None,
                context_window: Some(64000),
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: Some(vec![
                    "read_file".to_string(),
                    "glob".to_string(),
                    "grep".to_string(),
                ]),
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: Some(true),
            },
        ],
        "kanban" | "simple" => vec![
            PipelineColumn {
                id: "todo".to_string(),
                title: "К выполнению".to_string(),
                color: Some("#64748b".to_string()),
                icon: Some("Inbox".to_string()),
                persona: None,
                on_enter_script: None,
                context_window: Some(128000),
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
            },
            PipelineColumn {
                id: "in_progress".to_string(),
                title: "В работе".to_string(),
                color: Some("#3b82f6".to_string()),
                icon: Some("PlayCircle".to_string()),
                persona: Some("developer".to_string()),
                on_enter_script: None,
                context_window: Some(400000),
                model_override: Some("gemini-3.8-flash-high".to_string()),
                reasoning_effort: None,
                temperature: Some(0.2),
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
            },
            PipelineColumn {
                id: "ready".to_string(),
                title: "Готово".to_string(),
                color: Some("#10b981".to_string()),
                icon: Some("CheckCircle2".to_string()),
                persona: None,
                on_enter_script: None,
                context_window: Some(64000),
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: Some(true),
            },
        ],
        "empty" => vec![
            PipelineColumn {
                id: "backlog".to_string(),
                title: "Бэклог".to_string(),
                color: Some("#64748b".to_string()),
                icon: Some("Inbox".to_string()),
                persona: None,
                on_enter_script: None,
                context_window: Some(128000),
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
            },
            PipelineColumn {
                id: "ready".to_string(),
                title: "Готово".to_string(),
                color: Some("#10b981".to_string()),
                icon: Some("CheckCircle2".to_string()),
                persona: None,
                on_enter_script: None,
                context_window: Some(64000),
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: Some(true),
            },
        ],
        _ => default_pipeline_config().columns,
    }
}

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
    let boards_file = root.join(".cvh").join("boards.json");
    let serialized_boards = serde_json::to_string_pretty(config)
        .context("Failed to serialize BoardsConfig")?;
    atomic_write_file(&boards_file, &serialized_boards).await?;

    let mut pipeline = load_pipeline_config(root).await.unwrap_or_else(|_| default_pipeline_config());
    pipeline.boards = Some(config.boards.clone());
    pipeline.active_board_id = Some(config.active_board_id.clone());
    if let Some(active_board) = config.boards.iter().find(|b| b.id == config.active_board_id) {
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
    // 1. Project-local .cvh/llm.json (if present)
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

    // If repo has .cvh directory, also mirror to .cvh/llm.json
    let repo_cvh = repo_root.join(".cvh");
    if repo_cvh.exists() {
        let local_path = repo_cvh.join("llm.json");
        let _ = atomic_write_file(&local_path, &serialized).await;
    }

    Ok(())
}

/// Automatically scaffold the complete .cvh workspace structure if missing.
/// Creates: .cvh/agents/, .cvh/worktrees/, .cvh/skills/, .cvh/settings/, and default config/pipeline files.
pub async fn scaffold_cvh_project(repo_root: &Path) -> Result<()> {
    let cvh_dir = repo_root.join(".cvh");
    let agents_dir = cvh_dir.join("agents");
    let worktrees_dir = cvh_dir.join("worktrees");
    let skills_dir = cvh_dir.join("skills");
    let settings_dir = cvh_dir.join("settings");
    let hooks_dir = cvh_dir.join("hooks");

    // Initialize pure directory structure without any default agent role files
    tokio::fs::create_dir_all(&cvh_dir).await?;
    tokio::fs::create_dir_all(&agents_dir).await?;
    tokio::fs::create_dir_all(&worktrees_dir).await?;
    tokio::fs::create_dir_all(&skills_dir).await?;
    tokio::fs::create_dir_all(&settings_dir).await?;
    tokio::fs::create_dir_all(&hooks_dir).await?;

    // 1. config.json if not present
    let config_path = cvh_dir.join("config.json");
    if !config_path.exists() {
        let project_name = repo_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("CanbanProject");
        let default_config = serde_json::json!({
            "name": project_name,
            "version": "0.1.0",
            "description": format!("CanbanVoimaxHarness Workspace for {}", project_name),
            "activePipeline": "default-engineering",
            "defaultModel": "gemini-3.8-flash-high",
            "apiBaseUrl": "http://127.0.0.1:8045/v1"
        });
        atomic_write_file(&config_path, &serde_json::to_string_pretty(&default_config)?).await?;
    }

    // 2. pipeline.json if not present
    let pipeline_path = cvh_dir.join("pipeline.json");
    if !pipeline_path.exists() {
        let default_pipeline = default_pipeline_config();
        atomic_write_file(&pipeline_path, &serde_json::to_string_pretty(&default_pipeline)?).await?;
    }

    // Note: agents/ directory is kept intentionally empty. Users create custom personas manually.

    // 3. Default tasks.json if not present
    let tasks_path = cvh_dir.join("tasks.json");
    if !tasks_path.exists() {
        atomic_write_file(&tasks_path, "[]").await?;
    }

    // 4. Default tool policy in settings/ if not present.
    // NOTE: this MUST be tool_policy_path() — the same single file that
    // load_tool_policies()/save_tool_policies() use. A past split-brain
    // (scaffold wrote settings/tool-policy.json while load/save used
    // .cvh/tool_policy.json) left fresh projects with a missing enforced
    // file, so load fell back to all-allowed defaults and silently ignored
    // user disables.
    let tool_policy_path = tool_policy_path(repo_root);
    if !tool_policy_path.exists() {
        let default_tp = ToolPolicyConfig::default();
        atomic_write_file(&tool_policy_path, &serde_json::to_string_pretty(&default_tp)?).await?;
    }

    // 6. Ensure .gitignore excludes .cvh/llm.json, llm.json, and .cvh/worktrees/
    let gitignore_path = repo_root.join(".gitignore");
    if gitignore_path.exists() {
        if let Ok(mut gi_content) = tokio::fs::read_to_string(&gitignore_path).await {
            let mut modified = false;
            if !gi_content.contains(".cvh/llm.json") {
                gi_content.push_str("\n.cvh/llm.json\nllm.json\n");
                modified = true;
            }
            if !gi_content.contains(".cvh/worktrees") {
                gi_content.push_str(".cvh/worktrees/\n");
                modified = true;
            }
            if modified {
                let _ = tokio::fs::write(&gitignore_path, gi_content).await;
            }
        }
    }

    Ok(())
}

// ============================================================================
// Project Registry & Workspace Management
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEntry {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(rename = "rootPath", default)]
    pub root_path: Option<String>,
    #[serde(rename = "lastOpened", default = "chrono::Utc::now")]
    pub last_opened: DateTime<Utc>,
    #[serde(rename = "activeBranch", default)]
    pub active_branch: Option<String>,
    #[serde(rename = "worktreesCount", default)]
    pub worktrees_count: Option<usize>,
}

impl ProjectEntry {
    pub fn from_path(path: &Path) -> Self {
        let path_str = path.to_string_lossy().to_string();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unnamed Project")
            .to_string();
        let id = name.to_lowercase().replace(' ', "-");
        Self {
            id,
            name,
            path: path_str.clone(),
            root_path: Some(path_str),
            last_opened: Utc::now(),
            active_branch: Some("main".to_string()),
            worktrees_count: Some(0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectRegistry {
    #[serde(default)]
    pub projects: Vec<ProjectEntry>,
    #[serde(rename = "activeProjectId", default)]
    pub active_project_id: Option<String>,
}

pub async fn atomic_write_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create parent directory for {:?}", path))?;
    }

    let tmp_path = path.with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));
    tokio::fs::write(&tmp_path, content)
        .await
        .with_context(|| format!("Failed to write temporary file {:?}", tmp_path))?;

    tokio::fs::rename(&tmp_path, path)
        .await
        .with_context(|| format!("Failed to replace destination file {:?} with temporary file {:?}", path, tmp_path))?;

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

pub async fn save_project_config(root_path: impl AsRef<Path>, config: &ProjectConfig) -> Result<()> {
    let config_path = root_path.as_ref().join(".cvh").join("config.json");
    let serialized = serde_json::to_string_pretty(config)
        .context("Failed to serialize ProjectConfig to JSON")?;
    atomic_write_file(&config_path, &serialized).await
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
    let pipeline_path = root.join(".cvh").join("pipeline.json");
    let serialized = serde_json::to_string_pretty(config)
        .context("Failed to serialize PipelineConfig to JSON")?;
    atomic_write_file(&pipeline_path, &serialized).await?;

    if let Some(ref boards) = config.boards {
        let boards_cfg = BoardsConfig {
            active_board_id: config.active_board_id.clone().unwrap_or_else(default_board_id),
            boards: boards.clone(),
        };
        let boards_file = root.join(".cvh").join("boards.json");
        if let Ok(serialized_boards) = serde_json::to_string_pretty(&boards_cfg) {
            let _ = atomic_write_file(&boards_file, &serialized_boards).await;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPolicyConfig {
    #[serde(default = "default_global_tools", rename = "globalEnabled")]
    pub global_enabled: HashMap<String, bool>,
    #[serde(default, rename = "stageOverrides")]
    pub stage_overrides: HashMap<String, HashMap<String, bool>>,
}

pub fn default_global_tools() -> HashMap<String, bool> {
    let mut map = HashMap::new();
    map.insert("execute_command".to_string(), true);
    map.insert("read_file".to_string(), true);
    map.insert("edit_file".to_string(), true);
    map.insert("write_file".to_string(), true);
    map.insert("glob".to_string(), true);
    map.insert("grep".to_string(), true);
    map.insert("web_search".to_string(), true);
    map.insert("web_fetch".to_string(), true);
    map.insert("subagent".to_string(), true);
    map
}

impl Default for ToolPolicyConfig {
    fn default() -> Self {
        let mut stage_overrides = HashMap::new();

        // Review stage: read_file, grep, glob, bash read-only enabled; edit_file, write_file disabled
        let mut review_map = HashMap::new();
        review_map.insert("read_file".to_string(), true);
        review_map.insert("grep".to_string(), true);
        review_map.insert("glob".to_string(), true);
        review_map.insert("execute_command".to_string(), true);
        review_map.insert("edit_file".to_string(), false);
        review_map.insert("write_file".to_string(), false);
        review_map.insert("web_search".to_string(), true);
        review_map.insert("web_fetch".to_string(), true);
        review_map.insert("subagent".to_string(), true);
        stage_overrides.insert("review".to_string(), review_map);

        // Dev stage: write enabled, web_search disabled
        let mut dev_map = HashMap::new();
        dev_map.insert("execute_command".to_string(), true);
        dev_map.insert("read_file".to_string(), true);
        dev_map.insert("edit_file".to_string(), true);
        dev_map.insert("write_file".to_string(), true);
        dev_map.insert("glob".to_string(), true);
        dev_map.insert("grep".to_string(), true);
        dev_map.insert("web_search".to_string(), false);
        dev_map.insert("web_fetch".to_string(), true);
        dev_map.insert("subagent".to_string(), true);
        stage_overrides.insert("dev".to_string(), dev_map);

        Self {
            global_enabled: default_global_tools(),
            stage_overrides,
        }
    }
}

/// Normalized lookup into a policy map: matches the canonical name first,
/// then any SWE-bench alias key (`bash`/`str_replace`) stored in the map.
fn policy_lookup(map: &HashMap<String, bool>, canonical: &str) -> Option<bool> {
    if let Some(&allowed) = map.get(canonical) {
        return Some(allowed);
    }
    for (k, &v) in map.iter() {
        if normalize_tool_name(k) == canonical {
            return Some(v);
        }
    }
    None
}

pub fn is_tool_allowed(
    policy: &ToolPolicyConfig,
    stage_id: Option<&str>,
    tool_name: &str,
) -> bool {
    // Normalize SWE-bench aliases (bash -> execute_command,
    // str_replace -> edit_file) so the schema filter in run_agent_loop and
    // the zero-trust gate in execute_tool evaluate the same canonical name.
    let canonical = normalize_tool_name(tool_name);
    // AND-semantics: effective(tool, stage) = globalEnabled && stageOverride
    // (missing entries default to true). A stage override of `true` can no
    // longer re-enable a globally disabled tool; both layers must allow it.
    let global_ok = policy_lookup(&policy.global_enabled, canonical).unwrap_or(true);
    let stage_ok = match stage_id {
        Some(stage) => policy
            .stage_overrides
            .get(stage)
            .and_then(|stage_map| policy_lookup(stage_map, canonical))
            .unwrap_or(true),
        None => true,
    };
    global_ok && stage_ok
}

/// Canonical on-disk location of the Tool Policy Matrix.
///
/// Single source of truth shared by scaffold, save/load and the agent loop:
/// `<root>/.cvh/settings/tool-policy.json`.
pub fn tool_policy_path(root_path: &Path) -> PathBuf {
    root_path
        .join(".cvh")
        .join("settings")
        .join("tool-policy.json")
}

/// Legacy pre-unification location (`<root>/.cvh/tool_policy.json`).
/// Read-only fallback for projects whose disables were saved before the
/// path unification; new saves always converge to [`tool_policy_path`].
pub fn legacy_tool_policy_path(root_path: &Path) -> PathBuf {
    root_path.join(".cvh").join("tool_policy.json")
}

async fn read_tool_policy_file(policy_path: &Path) -> Result<ToolPolicyConfig> {
    let content = tokio::fs::read_to_string(policy_path)
        .await
        .with_context(|| format!("Failed to read tool policy at {:?}", policy_path))?;

    let config: ToolPolicyConfig = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse tool policy JSON at {:?}", policy_path))?;

    Ok(config)
}

pub async fn load_tool_policies(root_path: impl AsRef<Path>) -> Result<ToolPolicyConfig> {
    let root = root_path.as_ref();
    let canonical = tool_policy_path(root);
    let legacy = legacy_tool_policy_path(root);

    // Prefer the legacy file while it exists: it holds the last actually
    // enforced matrix on pre-unification installs (the canonical settings
    // file there is still the scaffold default). Once the next save runs,
    // state converges to the canonical file only (see save_tool_policies).
    if legacy.exists() {
        return read_tool_policy_file(&legacy).await;
    }
    if canonical.exists() {
        return read_tool_policy_file(&canonical).await;
    }
    Ok(ToolPolicyConfig::default())
}

pub async fn save_tool_policies(root_path: impl AsRef<Path>, config: &ToolPolicyConfig) -> Result<()> {
    let root = root_path.as_ref();
    let canonical = tool_policy_path(root);
    let serialized = serde_json::to_string_pretty(config)
        .context("Failed to serialize ToolPolicyConfig to JSON")?;
    atomic_write_file(&canonical, &serialized).await?;
    // Converge split-brain installs: drop the legacy file so future loads
    // can only see the canonical matrix. Best-effort; a missing file is fine.
    let legacy = legacy_tool_policy_path(root);
    if legacy.exists() {
        let _ = tokio::fs::remove_file(&legacy).await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_settings_serde_camel_and_snake_case() {
        // Test snake_case deserialization
        let json_snake = r#"{
            "provider": "open-router",
            "api_key": "sk-secret-123",
            "base_url": "https://openrouter.ai/api/v1",
            "model": "anthropic/claude-3-7-sonnet",
            "temperature": 0.5
        }"#;
        let s1: LlmSettings = serde_json::from_str(json_snake).expect("Failed to deserialize snake_case");
        assert_eq!(s1.provider, "open-router");
        assert_eq!(s1.api_key, "sk-secret-123");
        assert_eq!(s1.base_url, "https://openrouter.ai/api/v1");
        assert_eq!(s1.model, "anthropic/claude-3-7-sonnet");
        assert_eq!(s1.temperature, Some(0.5));

        // Test camelCase deserialization
        let json_camel = r#"{
            "provider": "antigravity",
            "apiKey": "sk-local-abc",
            "baseUrl": "http://127.0.0.1:19387/api/ai",
            "model": "gemini-3.8-flash-high"
        }"#;
        let s2: LlmSettings = serde_json::from_str(json_camel).expect("Failed to deserialize camelCase");
        assert_eq!(s2.provider, "antigravity");
        assert_eq!(s2.api_key, "sk-local-abc");
        assert_eq!(s2.base_url, "http://127.0.0.1:19387/api/ai");
        assert_eq!(s2.model, "gemini-3.8-flash-high");
        assert_eq!(s2.temperature, None);
    }

    #[tokio::test]
    async fn test_scaffold_cvh_project_pure_empty_structure() {
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base_dir).await.expect("Failed to create test dir in target");

        scaffold_cvh_project(&base_dir).await.expect("Scaffold failed");

        let cvh = base_dir.join(".cvh");
        assert!(cvh.exists());
        assert!(cvh.join("agents").is_dir());
        assert!(cvh.join("worktrees").is_dir());
        assert!(cvh.join("skills").is_dir());
        assert!(cvh.join("settings").is_dir());

        // Verify agents folder is strictly empty (no auto-generated persona files)
        let mut read_agents = tokio::fs::read_dir(cvh.join("agents")).await.expect("Read dir failed");
        let first_entry = read_agents.next_entry().await.expect("Next entry failed");
        assert!(first_entry.is_none(), "Expected agents/ to be completely empty per user mandate");

        let _ = tokio::fs::remove_dir_all(&base_dir).await;
    }

    fn policy_with_dev_exec_disabled() -> ToolPolicyConfig {
        let mut policy = ToolPolicyConfig::default();
        let dev = policy.stage_overrides.entry("dev".to_string()).or_default();
        dev.insert("execute_command".to_string(), false);
        dev.insert("read_file".to_string(), false);
        policy
    }

    #[test]
    fn test_disabled_tool_blocked_for_stage_including_alias() {
        // Regression: a matrix with execute_command=false / read_file=false
        // for a stage must deny both the canonical name and the SWE-bench
        // alias ("bash"), exactly as the execute_tool gate evaluates them.
        let policy = policy_with_dev_exec_disabled();

        assert!(!is_tool_allowed(&policy, Some("dev"), "execute_command"));
        assert!(!is_tool_allowed(&policy, Some("dev"), "bash"));
        assert!(!is_tool_allowed(&policy, Some("dev"), "read_file"));
        // Untouched stage entries keep working (review allows execute_command
        // by default).
        assert!(is_tool_allowed(&policy, Some("review"), "execute_command"));
        // str_replace alias normalizes to edit_file (disabled in review).
        assert!(!is_tool_allowed(&policy, Some("review"), "str_replace"));
    }

    #[test]
    fn test_tool_policy_and_semantics_global_stage() {
        // AND-semantics: effective(tool, stage) = globalEnabled && stageOverride
        // (missing entries default to true). A stage `true` must NOT resurrect
        // a globally disabled tool (the reported bug: default dev/review
        // overrides are almost all `true`, so global-off did nothing).
        let mut policy = ToolPolicyConfig::default();

        // global=false + stage=true -> false (incl. alias normalization).
        policy.global_enabled.insert("read_file".to_string(), false);
        assert!(!is_tool_allowed(&policy, Some("dev"), "read_file"));
        // No stage record at all + global=false -> false.
        assert!(!is_tool_allowed(&policy, Some("no-such-stage"), "read_file"));
        assert!(!is_tool_allowed(&policy, None, "read_file"));

        // global=true + stage=false -> false.
        policy.global_enabled.insert("read_file".to_string(), true);
        policy
            .stage_overrides
            .entry("dev".to_string())
            .or_default()
            .insert("read_file".to_string(), false);
        assert!(!is_tool_allowed(&policy, Some("dev"), "read_file"));

        // Both true -> true.
        policy
            .stage_overrides
            .entry("dev".to_string())
            .or_default()
            .insert("read_file".to_string(), true);
        assert!(is_tool_allowed(&policy, Some("dev"), "read_file"));

        // Alias pair under AND: global execute_command=false + stage bash=true
        // (stored under alias key) -> still false.
        policy.global_enabled.insert("execute_command".to_string(), false);
        let dev = policy.stage_overrides.entry("dev".to_string()).or_default();
        dev.remove("execute_command");
        dev.insert("bash".to_string(), true);
        assert!(!is_tool_allowed(&policy, Some("dev"), "execute_command"));
        assert!(!is_tool_allowed(&policy, Some("dev"), "bash"));
    }

    #[test]
    fn test_tool_permissions_and_semantics() {
        // Same AND-semantics for the legacy allowlist model:
        // effective = global_ok && stage_ok (missing entries default to true).
        use std::collections::HashMap;

        // global=false + stage=true -> false.
        let mut stage_map = HashMap::new();
        stage_map.insert(
            "dev".to_string(),
            vec!["read_file".to_string(), "execute_command".to_string()],
        );
        let perms = ToolPermissions::new(vec!["execute_command".to_string()], stage_map);
        assert!(!perms.is_tool_allowed("dev", "read_file"));
        // global=true + stage=false (absent from stage list) -> false.
        assert!(!perms.is_tool_allowed("dev", "glob"));
        // Both true -> true (incl. bash alias normalization).
        assert!(perms.is_tool_allowed("dev", "execute_command"));
        assert!(perms.is_tool_allowed("dev", "bash"));
        // No stage record + global=false -> false; unrestricted tool -> true.
        assert!(!perms.is_tool_allowed("no-such-stage", "read_file"));
        assert!(perms.is_tool_allowed("no-such-stage", "execute_command"));

        // Empty global + no stages: fail-open default preserved.
        let open = ToolPermissions::new(vec![], HashMap::new());
        assert!(open.is_tool_allowed("dev", "read_file"));
    }

    #[test]
    fn test_tool_policy_paths_unified() {
        // Regression: scaffold, save and load must share ONE canonical file.
        // The split-brain (scaffold -> .cvh/settings/tool-policy.json vs
        // load/save -> .cvh/tool_policy.json) left fresh projects without an
        // enforced file, so loads failed open to all-allowed defaults.
        let root = Path::new("/repo");
        assert_eq!(
            tool_policy_path(root),
            root.join(".cvh").join("settings").join("tool-policy.json")
        );
        assert_ne!(tool_policy_path(root), legacy_tool_policy_path(root));
    }

    #[tokio::test]
    async fn test_tool_policy_save_load_roundtrip_canonical() {
        // Regression: what the matrix saves is what the agent loop loads,
        // from the canonical settings file.
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_policy_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base_dir).await.expect("test dir");

        save_tool_policies(&base_dir, &policy_with_dev_exec_disabled())
            .await
            .expect("save failed");

        assert!(tool_policy_path(&base_dir).exists());
        assert!(!legacy_tool_policy_path(&base_dir).exists());

        let loaded = load_tool_policies(&base_dir).await.expect("load failed");
        assert!(!is_tool_allowed(&loaded, Some("dev"), "execute_command"));
        assert!(!is_tool_allowed(&loaded, Some("dev"), "bash"));
        assert!(!is_tool_allowed(&loaded, Some("dev"), "read_file"));

        let _ = tokio::fs::remove_dir_all(&base_dir).await;
    }

    #[tokio::test]
    async fn test_tool_policy_legacy_file_migrated_on_save() {
        // Regression: pre-unification installs keep their enforced disables:
        // a legacy-only .cvh/tool_policy.json is still honoured on load and
        // converges to the canonical file on the next save.
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_legacy_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base_dir).await.expect("test dir");

        let legacy = legacy_tool_policy_path(&base_dir);
        atomic_write_file(
            &legacy,
            &serde_json::to_string_pretty(&policy_with_dev_exec_disabled()).expect("json"),
        )
        .await
        .expect("legacy seed failed");

        // Load honours the legacy matrix (no silent fail-open to defaults).
        let loaded = load_tool_policies(&base_dir).await.expect("load failed");
        assert!(!is_tool_allowed(&loaded, Some("dev"), "execute_command"));

        // Next save converges to the canonical path and drops the legacy file.
        save_tool_policies(&base_dir, &loaded).await.expect("save failed");
        assert!(tool_policy_path(&base_dir).exists());
        assert!(!legacy_tool_policy_path(&base_dir).exists());
        let reloaded = load_tool_policies(&base_dir).await.expect("reload failed");
        assert!(!is_tool_allowed(&reloaded, Some("dev"), "execute_command"));

        let _ = tokio::fs::remove_dir_all(&base_dir).await;
    }

    #[test]
    fn test_default_boards_config() {
        let cfg = default_boards_config();
        assert_eq!(cfg.active_board_id, "development");
        assert_eq!(cfg.boards.len(), 1);
        assert_eq!(cfg.boards[0].id, "development");
        assert_eq!(cfg.boards[0].name, "Разработка");
        assert_eq!(cfg.boards[0].columns.len(), 5);
    }

    #[test]
    fn test_get_board_template_columns_modeler() {
        let cols = get_board_template_columns("modeler");
        assert_eq!(cols.len(), 4);
        assert_eq!(cols[0].id, "concept");
        assert_eq!(cols[0].persona.as_deref(), Some("concept-artist"));
        assert_eq!(cols[1].id, "modeling");
        assert_eq!(cols[1].persona.as_deref(), Some("modeler"));
        assert_eq!(cols[2].id, "texturing");
        assert_eq!(cols[2].persona.as_deref(), Some("texture-artist"));
        assert_eq!(cols[3].id, "ready");
        assert_eq!(cols[3].terminal, Some(true));
    }

    #[tokio::test]
    async fn test_boards_config_persistence_and_sync() {
        let base_dir = std::env::temp_dir().join(format!("cvh_test_boards_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base_dir).await.expect("test dir");

        let mut cfg = default_boards_config();
        let modeler_cols = get_board_template_columns("modeler");
        cfg.boards.push(BoardDef {
            id: "modeler".to_string(),
            name: "3D Моделирование".to_string(),
            columns: modeler_cols,
        });
        cfg.active_board_id = "modeler".to_string();

        save_boards_config(&base_dir, &cfg).await.expect("save boards");

        // Verify boards.json exists
        assert!(base_dir.join(".cvh").join("boards.json").exists());
        // Verify pipeline.json synchronized with active board
        let pipe = load_pipeline_config(&base_dir).await.expect("load pipeline");
        assert_eq!(pipe.title, "3D Моделирование");
        assert_eq!(pipe.columns.len(), 4);

        // Load boards config back
        let reloaded = load_boards_config(&base_dir).await.expect("load boards");
        assert_eq!(reloaded.active_board_id, "modeler");
        assert_eq!(reloaded.boards.len(), 2);

        let _ = tokio::fs::remove_dir_all(&base_dir).await;
    }

    #[tokio::test]
    async fn test_legacy_pipeline_migration_to_boards() {
        let base_dir = std::env::temp_dir().join(format!("cvh_test_legacy_{}", uuid::Uuid::new_v4()));
        let cvh = base_dir.join(".cvh");
        tokio::fs::create_dir_all(&cvh).await.expect("test dir");

        let legacy_pipeline = serde_json::json!({
            "id": "custom-engineering",
            "title": "Мой пайплайн",
            "columns": [
                {
                    "id": "col1",
                    "title": "Стадия 1",
                    "color": "#64748b",
                    "icon": "Inbox",
                    "persona": null,
                    "contextWindow": 128000,
                    "modelOverride": null,
                    "reasoningEffort": null,
                    "temperature": null,
                    "gatekeeperSensor": null,
                    "allowedTools": null,
                    "hooks": {},
                    "routes": null,
                    "autoRun": null,
                    "terminal": null
                }
            ]
        });

        atomic_write_file(&cvh.join("pipeline.json"), &serde_json::to_string_pretty(&legacy_pipeline).unwrap()).await.unwrap();

        let boards_cfg = load_boards_config(&base_dir).await.expect("load boards from legacy");
        assert_eq!(boards_cfg.active_board_id, "development");
        assert_eq!(boards_cfg.boards.len(), 1);
        assert_eq!(boards_cfg.boards[0].name, "Мой пайплайн");
        assert_eq!(boards_cfg.boards[0].columns.len(), 1);

        let _ = tokio::fs::remove_dir_all(&base_dir).await;
    }
}


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
        "ask_user" => "question",
        "create_tasks" => "create_task",
        "move_task" | "move_task_stage" | "advance_stage" | "next_stage" => "advance_task",
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
    #[serde(rename = "contextWindow", default, skip_serializing_if = "Option::is_none")]
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
    #[serde(rename = "compactorModel", alias = "compactor_model", default = "default_compactor_model")]
    pub compactor_model: String,
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

fn default_compactor_model() -> String {
    "inherit".to_string()
}

impl Default for LlmSettings {
    fn default() -> Self {
        Self {
            provider: default_llm_provider(),
            api_key: default_llm_api_key(),
            base_url: default_llm_base_url(),
            model: default_llm_model(),
            temperature: Some(0.2),
            compactor_model: default_compactor_model(),
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
    let root = discover_project_root_raw();
    let _ = ensure_project_gitignore(&root);
    root
}

fn discover_project_root_raw() -> PathBuf {
    // a) FIRST: Check %LOCALAPPDATA%\com.voimax.cvh\active_project.txt.
    // If this file exists, is not empty, and points to an existing directory,
    // return that path as the active project!
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

    let current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // b) Check std::env::current_dir(). If it has .cvh or .git, return it.
    if current.join(".cvh").exists() || current.join(".git").exists() {
        eprintln!(
            "CVH [discover_project_root]: Found .cvh or .git in current_dir: {:?}",
            current
        );
        return current;
    }

    // c) Traverse parent directories (current_dir.ancestors()) looking for .cvh or .git.
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

    // d) Otherwise fallback to std::env::current_dir().
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
                title: "Ожидание".to_string(),
                color: Some("#64748b".to_string()),
                icon: Some("Inbox".to_string()),
                persona: None,
                on_enter_script: None,
                context_window: None,
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
                context_window: None,
                model_override: None,
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
                context_window: None,
                model_override: None,
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
                context_window: None,
                model_override: None,
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
                context_window: None,
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

pub fn get_canonical_board_columns() -> Vec<PipelineColumn> {
    vec![
        PipelineColumn {
            id: "backlog".to_string(),
            title: "Бэклог".to_string(),
            color: Some("#64748b".to_string()),
            icon: Some("Inbox".to_string()),
            persona: None,
            on_enter_script: None,
            context_window: None,
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
            title: "В разработке".to_string(),
            color: Some("#f59e0b".to_string()),
            icon: Some("Code2".to_string()),
            persona: Some("developer".to_string()),
            on_enter_script: None,
            context_window: None,
            model_override: None,
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
            id: "ready".to_string(),
            title: "Готово".to_string(),
            color: Some("#10b981".to_string()),
            icon: Some("CheckCircle2".to_string()),
            persona: None,
            on_enter_script: None,
            context_window: None,
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
    ]
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
                context_window: None,
                model_override: None,
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
                context_window: None,
                model_override: None,
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
                context_window: None,
                model_override: None,
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
                context_window: None,
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
_ => get_canonical_board_columns(),
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

/// Canonical normalization of Kanban columns:
/// - Enforces that `backlog` (id: "backlog") MUST always exist at index 0.
/// - Enforces that `ready` (id: "ready", terminal: true) MUST always exist at the final index.
/// - Preserves relative order of middle columns.
/// - Duplicate IDs return an error.
pub fn normalize_pipeline_columns(
    columns: Vec<PipelineColumn>,
) -> Result<Vec<PipelineColumn>, String> {
    let mut seen = std::collections::HashSet::new();
    for c in &columns {
        if !seen.insert(c.id.clone()) {
            return Err(format!(
                "Дублирующийся id колонки: '{}' — id колонок должны быть уникальными",
                c.id
            ));
        }
    }
    let mut backlog: Option<PipelineColumn> = None;
    let mut ready: Option<PipelineColumn> = None;
    let mut middle = Vec::with_capacity(columns.len());
    for col in columns {
        if col.id == "backlog" && backlog.is_none() {
            backlog = Some(col);
        } else if col.id == "ready" && ready.is_none() {
            ready = Some(col);
        } else {
            middle.push(col);
        }
    }

    // Invariant: 'backlog' (id: "backlog") MUST always exist at index 0.
    let backlog = backlog.unwrap_or_else(|| PipelineColumn {
        id: "backlog".to_string(),
        title: "Backlog".to_string(),
        color: None,
        icon: None,
        persona: None,
        on_enter_script: None,
        context_window: None,
        model_override: None,
        reasoning_effort: None,
        temperature: None,
        gatekeeper_sensor: None,
        allowed_tools: None,
        hooks: HashMap::new(),
        routes: None,
        auto_run: None,
        terminal: None,
    });

    // Invariant: 'ready' (id: "ready", terminal: true) MUST always exist at the final index.
    let mut ready = ready.unwrap_or_else(|| PipelineColumn {
        id: "ready".to_string(),
        title: "Ready".to_string(),
        color: None,
        icon: None,
        persona: None,
        on_enter_script: None,
        context_window: None,
        model_override: None,
        reasoning_effort: None,
        temperature: None,
        gatekeeper_sensor: None,
        allowed_tools: None,
        hooks: HashMap::new(),
        routes: None,
        auto_run: None,
        terminal: Some(true),
    });
    if ready.terminal != Some(true) {
        ready.terminal = Some(true);
    }

    let mut out = Vec::with_capacity(1 + middle.len() + 1);
    out.push(backlog);
    out.extend(middle);
    out.push(ready);
    Ok(out)
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

pub const DEFAULT_AGENT_SETTING_JSON: &str = r#"{
  "model": "gemini-3.8-flash-high",
  "context_window": 128000,
  "temperature": 0.2,
  "max_tokens": 8192
}"#;

/// Per-agent overrides stored in `.cvh/agents/<role>/setting` (JSON, no
/// extension). All fields optional; a missing/broken file falls back to
/// defaults (see [`load_agent_setting`]).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AgentSetting {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default, alias = "contextWindow")]
    pub context_window: Option<u64>,
    #[serde(default, alias = "maxTokens")]
    pub max_tokens: Option<u64>,
    #[serde(default, alias = "allowedTools")]
    pub allowed_tools: Option<Vec<String>>,
}

/// `<root>/.cvh/agents/<role>`
pub fn agent_dir(repo_root: &Path, role: &str) -> PathBuf {
    repo_root.join(".cvh").join("agents").join(role)
}

/// Persona location (directory-only structure: `<root>/.cvh/agents/<role>/persona.md` or `agent.md`):
///
/// 1. `<root>/.cvh/agents/<role>/persona.md`
/// 2. `<root>/.cvh/agents/<role>/agent.md`
///
/// Defaults to `<root>/.cvh/agents/<role>/persona.md` if neither exists.
/// (NO flat file fallback!)
pub fn agent_persona_path(repo_root: &Path, role: &str) -> PathBuf {
    let base = agent_dir(repo_root, role);
    let persona = base.join("persona.md");
    if persona.is_file() {
        return persona;
    }
    let agent = base.join("agent.md");
    if agent.is_file() {
        return agent;
    }
    persona
}

/// `<root>/.cvh/agents/<role>/rules`
pub fn agent_rules_dir(repo_root: &Path, role: &str) -> PathBuf {
    agent_dir(repo_root, role).join("rules")
}

/// `<root>/.cvh/agents/<role>/hooks`
pub fn agent_hooks_dir(repo_root: &Path, role: &str) -> PathBuf {
    agent_dir(repo_root, role).join("hooks")
}

/// `<root>/.cvh/agents/<role>/setting` (JSON, no extension)
pub fn agent_setting_path(repo_root: &Path, role: &str) -> PathBuf {
    agent_dir(repo_root, role).join("setting")
}

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

/// Intersection check of a tool against the agent-level `allowed_tools`
/// allowlist. `None` (field absent) → no restriction. Alias-aware
/// (`bash`→`execute_command`, `str_replace`→`edit_file`).
pub fn agent_allows_tool(allowed: Option<&[String]>, tool_name: &str) -> bool {
    match allowed {
        None => true,
        Some(list) => {
            let canonical = normalize_tool_name(tool_name);
            list.iter().any(|t| {
                let norm = normalize_tool_name(t);
                norm == canonical
                    || t == tool_name
                    || ((canonical == "subagent_bg" || canonical == "collect_subagent")
                        && (norm == "subagent" || norm == "subagent_bg"))
            })
        }
    }
}

/// Resolve a pipeline `col.hooks` path with per-agent fallback:
/// the configured path as before; if that file does not exist, fall back to
/// `.cvh/agents/<persona>/hooks/<basename>`. With no persona, returns the
/// configured path unchanged (caller checks existence).
pub fn resolve_hook_path(
    repo_root: &Path,
    persona: Option<&str>,
    hook_rel: &str,
) -> PathBuf {
    let primary_buf = PathBuf::from(hook_rel);
    let primary = if primary_buf.is_absolute() {
        primary_buf
    } else {
        repo_root.join(&primary_buf)
    };
    if primary.exists() {
        return primary;
    }
    if let Some(role) = persona {
        let role = role.trim();
        if !role.is_empty() {
            let basename = Path::new(hook_rel)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            if !basename.is_empty() {
                return agent_hooks_dir(repo_root, role).join(&basename);
            }
        }
    }
    primary
}

/// Minimal fresh-project pipeline: exactly two columns — `backlog` (no
/// persona, first) and `ready` (terminal, last). The first agent is created
/// by the user via UI; the scaffold itself seeds no personas.
pub fn minimal_pipeline_config() -> PipelineConfig {
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
                context_window: None,
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
                id: "ready".to_string(),
                title: "Ready / Done".to_string(),
                color: Some("#10b981".to_string()),
                icon: Some("CheckCircle2".to_string()),
                persona: None,
                on_enter_script: None,
                context_window: None,
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
        boards: None,
        active_board_id: None,
    }
}

pub const DEFAULT_CVH_DEV_PERSONA_MD: &str = r#"---
name: cvh-dev
description: Use PROACTIVELY when setting up or repairing boards, pipelines, personas, or hooks.
model: inherit
tools: [read_file, write_file, edit_file, glob, grep, execute_command, web_search, web_fetch, todo_write, subagent, subagent_bg, collect_subagent, question]
scope: gated-writes
activation: on-enter dev runs run_agent(taskId)
---
# Role: CVH-DEV (Project Configuration & Environment Specialist)
Вы отвечаете ИСКЛЮЧИТЕЛЬНО за настройку проекта, правил и архитектуры CanbanVoimaxHarness (.cvh/):
- Настройка досок (.cvh/boards.json) и пайплайнов (.cvh/pipeline.json).
- Создание и редактирование персон, ролей и правил агентов (.cvh/agents/<role>/persona.md, rules/*.md).
- Настройка политик инструментов (.cvh/settings/tool-policy.json) и скриптов перехода (hooks).
Вы НЕ создаёте карточки задач и НЕ управляете рабочими тасками (этим занимается пользователь и пайплайн). Ваша цель — подготовить идеальное окружение для команды разработчиков.
"#;

pub const DEFAULT_RULE_BOARDS_MD: &str = r#"# Guide: Boards and Pipelines Configuration

Authoritative rules for `.cvh/boards.json` and `.cvh/pipeline.json`.

## 1. Boards as Application Control Plane
In CanbanVoimaxHarness, the Kanban board IS the runtime control plane.
Columns represent autonomous stages. Moving a card triggers stage automations and agents.

## 2. Canonical Ordering Invariant
- **Backlog**: id `"backlog"` MUST ALWAYS exist at index 0 (the first column).
- **Ready**: id `"ready"` (with `terminal: true`) MUST ALWAYS exist at the final index.
- **Middle stages**: Preserve relative pipeline order (e.g. `dev` -> `review` -> `qa`).
- Deleting `backlog` or `ready` is strictly prohibited on all boards. If omitted in a payload, the harness automatically re-injects them.

## 3. Configuration Files
- `.cvh/boards.json`:
  ```json
  {
    "activeBoardId": "development",
    "boards": [
      {
        "id": "development",
        "name": "Development",
        "columns": [ ... ]
      }
    ]
  }
  ```
- `.cvh/pipeline.json`:
  Mirrors the active board's columns and lifecycle settings.
  Stage properties: `id`, `title`, `persona`, `gatekeeperSensor`, `allowedTools`, `terminal`, `hooks`, `routes`.

## 4. Persona Mapping Invariant: 1 Persona = Exactly 1 Directory
- **Single Source of Truth**: Exactly 1 persona corresponds to 1 directory under `.cvh/agents/<persona>/`.
- **Decoupled Column IDs and Personas**: Columns define stages (`id`), while `persona` defines the autonomous agent role executing in that stage.
- **No Duplication by Column ID**: Never create duplicate persona directories based on column IDs. For example, if a column has `id: "review"` and `persona: "reviewer"`, create strictly `.cvh/agents/reviewer/`, never `.cvh/agents/review/`. Multiple columns can reference the same persona without duplicating directories.
"#;

pub const DEFAULT_RULE_AGENTS_MD: &str = r#"# Guide: Agent Personas and Modular Rules

Authoritative rules for `.cvh/agents/<role>/`.

## 1. Directory-Only Structure & Persona Invariant
All agent definitions reside in dedicated directories under `.cvh/agents/<role>/`:
```text
.cvh/agents/<role>/
├── persona.md       # Primary persona prompt with YAML frontmatter (or agent.md)
├── setting          # JSON settings ({ "model": "gemini-3.8-flash-high", "context_window": 128000, "temperature": 0.2, "max_tokens": 8192 })
└── rules/           # Modular rule files loaded during zero-turn priming
    ├── rule1.md
    └── ...
```
Flat persona files (`.cvh/agents/<role>.md`) are deprecated and ignored.

### 1.1 Invariant: 1 Persona = Exactly 1 Directory
- **1 Persona = 1 Directory**: Exactly one directory `.cvh/agents/<persona>/` per persona role.
- **No Column ID Duplication**: Never create duplicate persona directories based on column IDs. For example, if a column has `id: "review"` and `persona: "reviewer"`, create strictly `.cvh/agents/reviewer/`, never `.cvh/agents/review/`.
- **Reusable Personas**: Multiple pipeline columns across different boards can reference the same `persona` without directory duplication.

## 2. Persona YAML Frontmatter
Every `persona.md` must start with clean YAML frontmatter:
```yaml
---
name: <role>
description: Use PROACTIVELY when <trigger conditions>
model: inherit
tools: [read_file, glob, grep, execute_command, edit_file, write_file]
scope: gated-writes
activation: on-enter <column-id> runs run_agent(<task-id>)
---
```

## 3. Two-Level Tool Policy Matrix (AND-Semantics)
- Global Switches: `.cvh/settings/tool-policy.json` (`globalEnabled`).
- Stage Overrides: `stageOverrides.<stage_id>` or column `allowedTools`.
- `effective(tool, stage) = globalEnabled[tool] && stageOverride[stage][tool]`.
- If a tool is globally disabled, a stage cannot re-enable it.

## 4. Progressive Disclosure
- Keep `persona.md` concise (≤30-50 lines).
- Offload domain-specific guides to modular files in `.cvh/agents/<role>/rules/*.md`.
"#;

pub const DEFAULT_RULE_HOOKS_MD: &str = r#"# Guide: Transition Scripts and Lifecycle Hooks

Authoritative rules for `.cvh/hooks/` and column lifecycle scripts.

## 1. Lifecycle Triggers
Columns declare transition hooks in their column configuration:
- `hooks.on_enter`: Triggered when a card enters the stage.
- `hooks.on_finish`: Triggered when an agent finishes execution in the stage.
- Built-in hook: `"builtin:create_worktree"` automatically provisions an isolated git worktree upon entering backlog.

## 2. Supported Formats
Hook runner supports `.bat`, `.cmd`, `.ps1`, `.py`, `.sh`, and native binaries.

## 3. Injected Environment Variables
All hook processes receive the full task execution context:
- `CVH_TASK_ID`: Unique task identifier (e.g. `CVH-042`).
- `CVH_TASK_TITLE`: Human-readable task title.
- `CVH_WORKTREE_PATH`: Absolute path to the isolated worktree directory.
- `CVH_BRANCH`: Feature branch name (`feat/<task-slug>`).
- `CVH_STAGE_ID`: Target stage identifier.
- `CVH_VERDICT`: Reviewer verdict when leaving review (`APPROVED`, `APPROVED_WITH_CAVEATS`, `CHANGES_REQUESTED`).

## 4. Gatekeeper Sensors
Columns may define `gatekeeperSensor` (e.g. `cargo check` or `pnpm run build`).
Gatekeeper sensors execute inside `CVH_WORKTREE_PATH` before any card advance. If exit code != 0, transition is blocked.
"#;

/// Automatically scaffold the minimal .cvh workspace structure if missing.
/// Creates: empty `.cvh/agents/`, `.cvh/dialogs/`, `.cvh/worktrees/`,
/// `.cvh/skills/`, `.cvh/settings/`, `.cvh/hooks/` and default
/// config/pipeline/tasks/tool-policy files. Fresh scaffolds seed NO agents:
/// `pipeline.json` holds only `backlog` (no persona, first) + `ready`
/// (terminal, last); the first agent is created by the user via UI.
pub async fn scaffold_cvh_project(repo_root: &Path) -> Result<()> {
    let cvh_dir = repo_root.join(".cvh");
    let agents_dir = cvh_dir.join("agents");
    let dialogs_dir = cvh_dir.join("dialogs");
    let worktrees_dir = cvh_dir.join("worktrees");
    let skills_dir = cvh_dir.join("skills");
    let settings_dir = cvh_dir.join("settings");
    let hooks_dir = cvh_dir.join("hooks");

    tokio::fs::create_dir_all(&cvh_dir).await?;
    tokio::fs::create_dir_all(&agents_dir).await?;
    tokio::fs::create_dir_all(&dialogs_dir).await?;
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

    // 2. pipeline.json if not present (minimal: backlog + ready only).
    // Если корень — git-репо (.git есть), backlog сразу получает
    // hooks {"on_enter": "builtin:create_worktree"}; иначе hooks {}.
    let pipeline_path = cvh_dir.join("pipeline.json");
    if !pipeline_path.exists() {
        let mut minimal_pipeline = minimal_pipeline_config();
        if repo_root.join(".git").exists() {
            if let Some(backlog) = minimal_pipeline.columns.iter_mut().find(|c| c.id == "backlog") {
                backlog.hooks.insert(
                    "on_enter".to_string(),
                    "builtin:create_worktree".to_string(),
                );
            }
        }
        atomic_write_file(&pipeline_path, &serde_json::to_string_pretty(&minimal_pipeline)?).await?;
    }

    // Seed modular cvh-dev router persona, settings, and progressive disclosure rules
    // so any newly scaffolded project immediately has these guides available on disk.
    let cvh_dev_dir = agents_dir.join("cvh-dev");
    let cvh_dev_rules_dir = cvh_dev_dir.join("rules");
    let cvh_dev_hooks_dir = cvh_dev_dir.join("hooks");
    tokio::fs::create_dir_all(&cvh_dev_rules_dir).await?;
    tokio::fs::create_dir_all(&cvh_dev_hooks_dir).await?;

    let persona_path = cvh_dev_dir.join("persona.md");
    if !persona_path.exists() {
        atomic_write_file(&persona_path, DEFAULT_CVH_DEV_PERSONA_MD).await?;
    }
    let setting_path = cvh_dev_dir.join("setting");
    if !setting_path.exists() {
        atomic_write_file(&setting_path, DEFAULT_AGENT_SETTING_JSON).await?;
    }
    let boards_md_path = cvh_dev_rules_dir.join("boards.md");
    if !boards_md_path.exists() {
        atomic_write_file(&boards_md_path, DEFAULT_RULE_BOARDS_MD).await?;
    }
    let agents_md_path = cvh_dev_rules_dir.join("agents.md");
    if !agents_md_path.exists() {
        atomic_write_file(&agents_md_path, DEFAULT_RULE_AGENTS_MD).await?;
    }
    let hooks_md_path = cvh_dev_rules_dir.join("hooks.md");
    if !hooks_md_path.exists() {
        atomic_write_file(&hooks_md_path, DEFAULT_RULE_HOOKS_MD).await?;
    }

    // 3. Default tasks.json if not present
    let tasks_path = cvh_dir.join("tasks.json");
    if !tasks_path.exists() {
        atomic_write_file(&tasks_path, "[]").await?;
    }

    // 4. Default tool policy in settings/ if not present.
    // NOTE: this MUST be tool_policy_path() — the same single file that
    // load_tool_policies()/save_tool_policies() use (single canonical path,
    // no legacy fallbacks).
    let tool_policy_path = tool_policy_path(repo_root);
    if !tool_policy_path.exists() {
        let default_tp = ToolPolicyConfig::default();
        atomic_write_file(&tool_policy_path, &serde_json::to_string_pretty(&default_tp)?).await?;
    }

    // 6. Ensure .gitignore has all local CVH runtime entries
    let _ = ensure_project_gitignore(repo_root);

    Ok(())
}

// ============================================================================
// Automatic .gitignore Synchronization for .cvh Local Runtime
// ============================================================================

pub const CVH_GITIGNORE_HEADER: &str = "# CanbanVoimaxHarness Local Runtime";
pub const CVH_RUNTIME_GITIGNORE_ENTRIES: &[&str] = &[
    ".cvh/harness.db*",
    ".cvh/spill/",
    ".cvh/worktrees/",
    ".cvh/llm.json",
    ".cvh/tasks.json",
];

/// Ensures that `<repo_root>/.gitignore` exists and contains all required
/// local CVH runtime entries:
/// - `.cvh/harness.db*`
/// - `.cvh/spill/`
/// - `.cvh/worktrees/`
/// - `.cvh/llm.json`
/// - `.cvh/tasks.json`
///
/// If any of these entries are missing (and no wildcard `.cvh/` or `.cvh` covers them),
/// appends the missing lines with the clean section header `# CanbanVoimaxHarness Local Runtime`.
/// Never duplicates existing lines or existing section headers.
pub fn ensure_project_gitignore(repo_root: &Path) -> std::io::Result<()> {
    let gitignore_path = repo_root.join(".gitignore");
    if !gitignore_path.exists() {
        if !repo_root.exists() {
            let _ = std::fs::create_dir_all(repo_root);
        }
        let mut content = String::new();
        content.push_str(CVH_GITIGNORE_HEADER);
        content.push('\n');
        for entry in CVH_RUNTIME_GITIGNORE_ENTRIES {
            content.push_str(entry);
            content.push('\n');
        }
        return std::fs::write(&gitignore_path, content);
    }

    let existing_content = std::fs::read_to_string(&gitignore_path)?;

    let lines: Vec<&str> = existing_content.lines().collect();

    // 1. Check if any wildcard `.cvh` or `.cvh/` covers the entire directory
    let is_wildcard_covered = lines.iter().any(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
            return false;
        }
        let norm = trimmed.trim_start_matches('/');
        norm == ".cvh"
            || norm == ".cvh/"
            || norm == ".cvh/*"
            || norm == "**/.cvh"
            || norm == "**/.cvh/"
            || norm == "**/.cvh/*"
    });

    if is_wildcard_covered {
        return Ok(());
    }

    // 2. Determine which runtime entries are missing
    let mut missing_entries = Vec::new();
    for &entry in CVH_RUNTIME_GITIGNORE_ENTRIES {
        let clean_entry = entry.trim_start_matches('/');
        let clean_entry_no_slash = clean_entry.trim_end_matches('/');

        let present = lines.iter().any(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
                return false;
            }
            let norm = trimmed.trim_start_matches('/');
            let norm_no_slash = norm.trim_end_matches('/');

            if norm == clean_entry || norm_no_slash == clean_entry_no_slash {
                return true;
            }

            if let Some(stripped) = norm.strip_prefix("**/") {
                let stripped_no_slash = stripped.trim_end_matches('/');
                if stripped == clean_entry || stripped_no_slash == clean_entry_no_slash {
                    return true;
                }
            }

            if entry.ends_with('/') {
                let dir_wildcard = format!("{}/*", clean_entry_no_slash);
                if norm == dir_wildcard || norm == format!("**/{}", dir_wildcard) {
                    return true;
                }
            }

            if entry == ".cvh/harness.db*" && (norm == ".cvh/harness*" || norm == "**/.cvh/harness*") {
                return true;
            }

            false
        });

        if !present {
            missing_entries.push(entry);
        }
    }

    if missing_entries.is_empty() {
        return Ok(());
    }

    // 3. Append missing lines with clean section header without duplicating
    let has_header = lines.iter().any(|l| l.trim() == CVH_GITIGNORE_HEADER);

    let mut output = existing_content;
    if !has_header {
        if !output.is_empty() && !output.ends_with("\n\n") {
            if output.ends_with('\n') {
                output.push('\n');
            } else {
                output.push_str("\n\n");
            }
        }
        output.push_str(CVH_GITIGNORE_HEADER);
        output.push('\n');
    } else if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }

    for entry in missing_entries {
        output.push_str(entry);
        output.push('\n');
    }

    std::fs::write(&gitignore_path, output)
}

/// Project initialization helper (ensures .gitignore entries and scaffolds CVH structure)
pub async fn init_project_harness(repo_root: &Path) -> Result<()> {
    let _ = ensure_project_gitignore(repo_root);
    scaffold_cvh_project(repo_root).await
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

    if let Some(ref boards) = normalized.boards {
        let boards_cfg = BoardsConfig {
            active_board_id: normalized.active_board_id.clone().unwrap_or_else(default_board_id),
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

pub const CANONICAL_TOOLS: &[&str] = &[
    "execute_command",
    "read_file",
    "edit_file",
    "write_file",
    "glob",
    "grep",
    "web_search",
    "web_fetch",
    "subagent",
    "subagent_bg",
    "collect_subagent",
    "todo_write",
    "advance_task",
    "move_task",
    "reject_task",
    "question",
    "create_task",
    "complete_task",
];

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
    map.insert("subagent_bg".to_string(), true);
    map.insert("todo_write".to_string(), true);
    map.insert("advance_task".to_string(), true);
    map.insert("reject_task".to_string(), true);
    map.insert("question".to_string(), true);
    map.insert("create_task".to_string(), true);
    map.insert("complete_task".to_string(), true);
    map
}

impl Default for ToolPolicyConfig {
    fn default() -> Self {
        let mut stage_overrides = HashMap::new();

        // Review stage: read_file, grep, glob, bash read-only enabled; edit_file, write_file disabled; orchestrator tools disabled
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
        review_map.insert("todo_write".to_string(), true);
        review_map.insert("advance_task".to_string(), true);
        review_map.insert("reject_task".to_string(), true);
        review_map.insert("question".to_string(), true);
        review_map.insert("create_task".to_string(), false);
        review_map.insert("complete_task".to_string(), false);
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
        dev_map.insert("todo_write".to_string(), true);
        dev_map.insert("advance_task".to_string(), true);
        dev_map.insert("reject_task".to_string(), true);
        dev_map.insert("question".to_string(), true);
        dev_map.insert("create_task".to_string(), true);
        dev_map.insert("complete_task".to_string(), true);
        stage_overrides.insert("dev".to_string(), dev_map);

        // Backlog stage: orchestration enabled
        let mut backlog_map = HashMap::new();
        backlog_map.insert("create_task".to_string(), true);
        backlog_map.insert("complete_task".to_string(), true);
        stage_overrides.insert("backlog".to_string(), backlog_map);

        // Orchestrator stage: orchestration enabled
        let mut orch_map = HashMap::new();
        orch_map.insert("create_task".to_string(), true);
        orch_map.insert("complete_task".to_string(), true);
        stage_overrides.insert("orchestrator".to_string(), orch_map);

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

    // Hierarchical Subagent Policy:
    // If checking 'subagent', 'subagent_bg', or 'collect_subagent':
    // The master switch 'subagent' must be allowed first!
    if canonical == "subagent" || canonical == "subagent_bg" || canonical == "collect_subagent" {
        let master_global = policy_lookup(&policy.global_enabled, "subagent").unwrap_or(true);
        if !master_global {
            return false;
        }
        if let Some(stage) = stage_id {
            if let Some(stage_map) = policy.stage_overrides.get(stage) {
                if let Some(master_stage) = policy_lookup(stage_map, "subagent") {
                    if !master_stage {
                        return false;
                    }
                }
            }
        }

        if canonical == "subagent" {
            let sync_global = policy_lookup(&policy.global_enabled, "subagent_sync").unwrap_or(true);
            if !sync_global {
                return false;
            }
            if let Some(stage) = stage_id {
                if let Some(stage_map) = policy.stage_overrides.get(stage) {
                    if let Some(sync_stage) = policy_lookup(stage_map, "subagent_sync") {
                        if !sync_stage {
                            return false;
                        }
                    }
                }
            }
            return true;
        }

        if canonical == "subagent_bg" || canonical == "collect_subagent" {
            let bg_global = policy_lookup(&policy.global_enabled, "subagent_bg").unwrap_or(true);
            if !bg_global {
                return false;
            }
            if let Some(stage) = stage_id {
                if let Some(stage_map) = policy.stage_overrides.get(stage) {
                    if let Some(bg_stage) = policy_lookup(stage_map, "subagent_bg") {
                        if !bg_stage {
                            return false;
                        }
                    }
                }
            }
            return true;
        }
    }

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
    let _ = sync_tool_policies_to_personas(root, config).await;
    Ok(())
}

/// Synchronizes allowed tools from `ToolPolicyConfig` into each agent persona's
/// `.cvh/agents/<role>/persona.md` YAML frontmatter `tools: [...]`.
pub async fn sync_tool_policies_to_personas(
    root_path: impl AsRef<Path>,
    config: &ToolPolicyConfig,
) -> Result<()> {
    let root = root_path.as_ref();
    let agents_dir = root.join(".cvh").join("agents");
    if !agents_dir.exists() {
        return Ok(());
    }

    let mut dir_entries = match tokio::fs::read_dir(&agents_dir).await {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };

    while let Ok(Some(entry)) = dir_entries.next_entry().await {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let role = match entry.file_name().to_str() {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => continue,
        };

        let persona_file = if path.join("persona.md").is_file() {
            path.join("persona.md")
        } else if path.join("agent.md").is_file() {
            path.join("agent.md")
        } else {
            continue;
        };

        let content = match tokio::fs::read_to_string(&persona_file).await {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Determine effective stage/role identifier for policy lookup
        let stage_target: Option<&str> = if config.stage_overrides.contains_key(&role) {
            Some(&role)
        } else if role == "developer" || role == "dev" {
            Some("dev")
        } else if role == "reviewer" || role == "review" {
            Some("review")
        } else if role == "retrospective" || role == "retro" {
            Some("retro")
        } else if role == "orchestrator" {
            Some("orchestrator")
        } else {
            None
        };

        let mut enabled_tools = Vec::new();
        for &tool in CANONICAL_TOOLS {
            // Respect special role restrictions
            if role == "cvh-dev" && !crate::agent::CVH_DEV_TOOLS.contains(&tool) {
                continue;
            }
            if (role == "developer" || role == "dev")
                && (tool == "create_task" || tool == "complete_task")
            {
                // § 2.8 Development Subagents and cvh-dev are STRICTLY FORBIDDEN from calling create_task or complete_task
                continue;
            }
            if is_tool_allowed(config, stage_target, tool) {
                enabled_tools.push(tool);
            }
        }

        let tools_line = format!("tools: [{}]", enabled_tools.join(", "));

        let updated_content = update_frontmatter_tools(&content, &tools_line);
        if updated_content != content {
            let _ = atomic_write_file(&persona_file, &updated_content).await;
        }
    }

    Ok(())
}

pub fn update_frontmatter_tools(content: &str, new_tools_line: &str) -> String {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return format!("---\n{}\n---\n{}", new_tools_line, content);
    }

    let rest = &trimmed[3..];
    if let Some(end_idx) = rest.find("\n---") {
        let frontmatter = &rest[..end_idx];
        let body = &rest[end_idx + 4..];

        let mut lines: Vec<String> = Vec::new();
        let mut in_tools = false;
        let mut replaced = false;

        for line in frontmatter.lines() {
            let line_trim = line.trim();
            if line_trim.starts_with("tools:") {
                lines.push(new_tools_line.to_string());
                replaced = true;
                in_tools = true;
                continue;
            }
            if in_tools {
                if line_trim.starts_with('-') || line_trim.starts_with("  ") {
                    // Skip multiline tools array items
                    continue;
                } else {
                    in_tools = false;
                }
            }
            lines.push(line.to_string());
        }

        if !replaced {
            lines.push(new_tools_line.to_string());
        }

        let new_frontmatter = lines.join("\n");
        format!("---\n{}\n---{}", new_frontmatter.trim(), body)
    } else {
        content.to_string()
    }
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

        // Verify cvh-dev modular rules are seeded
        let cvh_dev = cvh.join("agents").join("cvh-dev");
        assert!(cvh_dev.join("persona.md").is_file());
        assert!(cvh_dev.join("rules").join("boards.md").is_file());
        assert!(cvh_dev.join("rules").join("agents.md").is_file());
        assert!(cvh_dev.join("rules").join("hooks.md").is_file());

        // Ensure no legacy flat persona files exist in agents/
        let mut read_agents = tokio::fs::read_dir(cvh.join("agents")).await.expect("Read dir failed");
        while let Ok(Some(entry)) = read_agents.next_entry().await {
            assert!(entry.path().is_dir(), "Expected only directories in agents/, found: {:?}", entry.path());
        }

        let _ = tokio::fs::remove_dir_all(&base_dir).await;
    }

    #[tokio::test]
    async fn test_scaffold_minimal_backlog_ready_only() {
        // Fresh scaffold minimalism: pipeline.json holds exactly backlog
        // (no persona, first) + ready (terminal, last); agents/ empty;
        // the rest of the scaffold files stay in place.
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_min_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base_dir).await.expect("test dir");

        scaffold_cvh_project(&base_dir).await.expect("Scaffold failed");

        let cvh = base_dir.join(".cvh");
        let pipe_raw = tokio::fs::read_to_string(cvh.join("pipeline.json"))
            .await
            .expect("pipeline.json missing");
        let pipe: PipelineConfig = serde_json::from_str(&pipe_raw).expect("pipeline parse");
        assert_eq!(pipe.columns.len(), 2, "minimal scaffold must have exactly 2 columns");
        assert_eq!(pipe.columns[0].id, "backlog");
        assert_eq!(pipe.columns[0].persona, None, "backlog must have no persona");
        assert_eq!(pipe.columns[1].id, "ready");
        assert_eq!(pipe.columns[1].terminal, Some(true), "ready must be terminal last");

        // cvh-dev seeded with modular rules, no flat personas
        let cvh_dev = cvh.join("agents").join("cvh-dev");
        assert!(cvh_dev.join("persona.md").is_file());
        assert!(cvh_dev.join("rules").join("boards.md").is_file());
        let mut rd = tokio::fs::read_dir(cvh.join("agents")).await.expect("agents dir");
        while let Ok(Some(entry)) = rd.next_entry().await {
            assert!(entry.path().is_dir(), "Expected only directories in agents/, found: {:?}", entry.path());
        }

        // remaining scaffold files/dirs in place
        assert!(cvh.join("dialogs").is_dir());
        assert!(cvh.join("worktrees").is_dir());
        assert!(cvh.join("config.json").is_file());
        let tasks_raw = tokio::fs::read_to_string(cvh.join("tasks.json")).await.expect("tasks.json");
        assert_eq!(tasks_raw.trim(), "[]");
        assert!(tool_policy_path(&base_dir).is_file());

        let _ = tokio::fs::remove_dir_all(&base_dir).await;
    }

    #[tokio::test]
    async fn test_load_agent_setting_broken_falls_back_to_default() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_set_{}", uuid::Uuid::new_v4()));
        let dir = base.join(".cvh").join("agents").join("dev");
        tokio::fs::create_dir_all(&dir).await.expect("test dir");

        // Missing → default.
        let missing = load_agent_setting(&base, "dev").await;
        assert_eq!(missing, AgentSetting::default());

        // Broken JSON → default.
        tokio::fs::write(dir.join("setting"), "{ not valid json {{{").await.expect("seed broken");
        let broken = load_agent_setting(&base, "dev").await;
        assert_eq!(broken, AgentSetting::default());

        // Valid snake_case + camelCase aliases.
        tokio::fs::write(
            dir.join("setting"),
            r#"{"model": "m1", "temperature": 0.7, "contextWindow": 64000, "allowedTools": ["read_file", "bash"]}"#,
        )
        .await
        .expect("seed valid");
        let valid = load_agent_setting(&base, "dev").await;
        assert_eq!(valid.model.as_deref(), Some("m1"));
        assert_eq!(valid.temperature, Some(0.7));
        assert_eq!(valid.context_window, Some(64000));
        assert_eq!(
            valid.allowed_tools,
            Some(vec!["read_file".to_string(), "bash".to_string()])
        );

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[test]
    fn test_agent_allows_tool_intersection() {
        // None → unrestricted.
        assert!(agent_allows_tool(None, "read_file"));
        assert!(agent_allows_tool(None, "bash"));
        // Listed tools pass, others are cut (intersection semantics).
        let allowed = vec!["read_file".to_string(), "execute_command".to_string()];
        assert!(agent_allows_tool(Some(&allowed), "read_file"));
        assert!(agent_allows_tool(Some(&allowed), "execute_command"));
        assert!(agent_allows_tool(Some(&allowed), "bash"), "bash alias must match execute_command");
        assert!(!agent_allows_tool(Some(&allowed), "write_file"));
        assert!(!agent_allows_tool(Some(&allowed), "web_search"));
        // Empty list → nothing allowed.
        let empty: Vec<String> = vec![];
        assert!(!agent_allows_tool(Some(&empty), "read_file"));
    }

    #[test]
    fn test_resolve_hook_path_primary_then_agent_fallback() {
        let base = std::env::temp_dir().join(format!("cvh_hook_{}", uuid::Uuid::new_v4()));
        let hooks = base.join(".cvh").join("hooks");
        let agent_hooks = base.join(".cvh").join("agents").join("dev").join("hooks");
        std::fs::create_dir_all(&hooks).expect("hooks dir");
        std::fs::create_dir_all(&agent_hooks).expect("agent hooks dir");
        std::fs::write(hooks.join("on_enter.bat"), "rem").expect("seed primary");
        std::fs::write(agent_hooks.join("on_enter.bat"), "rem agent").expect("seed agent");

        // Primary exists → primary wins.
        let p1 = resolve_hook_path(&base, Some("dev"), ".cvh/hooks/on_enter.bat");
        assert_eq!(p1, base.join(".cvh").join("hooks").join("on_enter.bat"));

        // Primary missing → agent fallback by basename.
        std::fs::remove_file(hooks.join("on_enter.bat")).expect("remove primary");
        let p2 = resolve_hook_path(&base, Some("dev"), ".cvh/hooks/on_enter.bat");
        assert_eq!(p2, agent_hooks.join("on_enter.bat"));

        // No persona → configured path unchanged.
        let p3 = resolve_hook_path(&base, None, ".cvh/hooks/on_enter.bat");
        assert_eq!(p3, base.join(".cvh").join("hooks").join("on_enter.bat"));

        let _ = std::fs::remove_dir_all(&base);
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
        // ask_user alias normalizes to question (enabled in dev and review by default).
        assert!(is_tool_allowed(&policy, Some("dev"), "question"));
        assert!(is_tool_allowed(&policy, Some("dev"), "ask_user"));
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
    fn test_tool_policy_path_canonical() {
        // Single canonical file shared by scaffold, save and load:
        // `<root>/.cvh/settings/tool-policy.json`. No legacy fallbacks.
        let root = Path::new("/repo");
        assert_eq!(
            tool_policy_path(root),
            root.join(".cvh").join("settings").join("tool-policy.json")
        );
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

        let loaded = load_tool_policies(&base_dir).await.expect("load failed");
        assert!(!is_tool_allowed(&loaded, Some("dev"), "execute_command"));
        assert!(!is_tool_allowed(&loaded, Some("dev"), "bash"));
        assert!(!is_tool_allowed(&loaded, Some("dev"), "read_file"));

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
        assert!(cols.iter().all(|c| c.model_override.is_none()));
        assert_eq!(cols[0].id, "concept");
        assert_eq!(cols[0].persona.as_deref(), Some("concept-artist"));
        assert_eq!(cols[1].id, "modeling");
        assert_eq!(cols[1].persona.as_deref(), Some("modeler"));
        assert_eq!(cols[2].id, "texturing");
        assert_eq!(cols[2].persona.as_deref(), Some("texture-artist"));
        assert_eq!(cols[3].id, "ready");
        assert_eq!(cols[3].terminal, Some(true));
    }

    #[test]
    fn test_get_board_template_columns_canonical_default() {
        let cols = get_board_template_columns("canonical");
        assert_eq!(cols.len(), 3);
        assert!(cols.iter().all(|c| c.model_override.is_none()));
        assert_eq!(cols[0].id, "backlog");
        assert_eq!(cols[0].title, "Бэклог");
        assert_eq!(cols[0].persona, None);
        assert_eq!(cols[1].id, "dev");
        assert_eq!(cols[1].title, "В разработке");
        assert_eq!(cols[1].persona.as_deref(), Some("developer"));
        assert_eq!(cols[2].id, "ready");
        assert_eq!(cols[2].title, "Готово");
        assert_eq!(cols[2].persona, None);
        assert_eq!(cols[2].terminal, Some(true));
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
        // Verify pipeline.json synchronized with active board (normalized with mandatory backlog and ready)
        let pipe = load_pipeline_config(&base_dir).await.expect("load pipeline");
        assert_eq!(pipe.title, "3D Моделирование");
        assert_eq!(pipe.columns.len(), 5);

        // Load boards config back
        let reloaded = load_boards_config(&base_dir).await.expect("load boards");
        assert_eq!(reloaded.active_board_id, "modeler");
        assert_eq!(reloaded.boards.len(), 2);

        let _ = tokio::fs::remove_dir_all(&base_dir).await;
    }

    #[tokio::test]
    async fn test_boards_config_fallback_to_pipeline() {
        // boards.json missing → single-board config derived from pipeline.json
        // columns (kept fallback, not a migration: no files are moved).
        let base_dir = std::env::temp_dir().join(format!("cvh_test_boards_fb_{}", uuid::Uuid::new_v4()));
        let cvh = base_dir.join(".cvh");
        tokio::fs::create_dir_all(&cvh).await.expect("test dir");

        let pipeline_only = serde_json::json!({
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

        atomic_write_file(&cvh.join("pipeline.json"), &serde_json::to_string_pretty(&pipeline_only).unwrap()).await.unwrap();

        let boards_cfg = load_boards_config(&base_dir).await.expect("load boards from pipeline fallback");
        assert_eq!(boards_cfg.active_board_id, "development");
        assert_eq!(boards_cfg.boards.len(), 1);
        assert_eq!(boards_cfg.boards[0].name, "Мой пайплайн");
        assert_eq!(boards_cfg.boards[0].columns.len(), 1);

        let _ = tokio::fs::remove_dir_all(&base_dir).await;
    }

    #[test]
    fn test_ensure_project_gitignore_creates_missing_file() {
        let base_dir = std::env::temp_dir().join(format!("cvh_gi_create_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base_dir).expect("create test dir");

        ensure_project_gitignore(&base_dir).expect("ensure gitignore failed");

        let gi_path = base_dir.join(".gitignore");
        assert!(gi_path.is_file(), ".gitignore must be created if missing");

        let content = std::fs::read_to_string(&gi_path).expect("read gitignore");
        assert!(content.contains(CVH_GITIGNORE_HEADER));
        for entry in CVH_RUNTIME_GITIGNORE_ENTRIES {
            assert!(content.contains(entry), "Missing entry in created gitignore: {}", entry);
        }

        let _ = std::fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn test_ensure_project_gitignore_appends_to_existing_file() {
        let base_dir = std::env::temp_dir().join(format!("cvh_gi_append_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base_dir).expect("create test dir");

        let gi_path = base_dir.join(".gitignore");
        std::fs::write(&gi_path, "/target/\nnode_modules/\n").expect("seed existing");

        ensure_project_gitignore(&base_dir).expect("ensure gitignore failed");

        let content = std::fs::read_to_string(&gi_path).expect("read gitignore");
        assert!(content.starts_with("/target/\nnode_modules/\n"));
        assert!(content.contains(CVH_GITIGNORE_HEADER));
        for entry in CVH_RUNTIME_GITIGNORE_ENTRIES {
            assert!(content.contains(entry), "Missing runtime entry: {}", entry);
        }

        let _ = std::fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn test_ensure_project_gitignore_idempotent_no_duplicates() {
        let base_dir = std::env::temp_dir().join(format!("cvh_gi_idempotent_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base_dir).expect("create test dir");

        ensure_project_gitignore(&base_dir).expect("first run");
        let content_first = std::fs::read_to_string(base_dir.join(".gitignore")).expect("read first");

        ensure_project_gitignore(&base_dir).expect("second run");
        let content_second = std::fs::read_to_string(base_dir.join(".gitignore")).expect("read second");

        assert_eq!(content_first, content_second, "Second run must not modify gitignore");

        let header_count = content_second.matches(CVH_GITIGNORE_HEADER).count();
        assert_eq!(header_count, 1, "Header must appear exactly once");

        for entry in CVH_RUNTIME_GITIGNORE_ENTRIES {
            let entry_count = content_second.matches(entry).count();
            assert_eq!(entry_count, 1, "Entry {} must appear exactly once", entry);
        }

        let _ = std::fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn test_ensure_project_gitignore_wildcard_covered() {
        let base_dir = std::env::temp_dir().join(format!("cvh_gi_wildcard_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base_dir).expect("create test dir");

        let gi_path = base_dir.join(".gitignore");
        let initial_content = "target/\n.cvh/\nnode_modules/\n";
        std::fs::write(&gi_path, initial_content).expect("seed wildcard");

        ensure_project_gitignore(&base_dir).expect("run on wildcard");

        let content = std::fs::read_to_string(&gi_path).expect("read gitignore");
        assert_eq!(content, initial_content, "Wildcard .cvh/ must prevent modifying gitignore");

        let _ = std::fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn test_ensure_project_gitignore_partial_preserves_header_and_entries() {
        let base_dir = std::env::temp_dir().join(format!("cvh_gi_partial_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base_dir).expect("create test dir");

        let gi_path = base_dir.join(".gitignore");
        let initial = format!(
            "{}\n.cvh/worktrees/\n.cvh/llm.json\n",
            CVH_GITIGNORE_HEADER
        );
        std::fs::write(&gi_path, &initial).expect("seed partial");

        ensure_project_gitignore(&base_dir).expect("run on partial");

        let content = std::fs::read_to_string(&gi_path).expect("read gitignore");
        let header_count = content.matches(CVH_GITIGNORE_HEADER).count();
        assert_eq!(header_count, 1, "Existing header must not be duplicated");

        for entry in CVH_RUNTIME_GITIGNORE_ENTRIES {
            assert!(content.contains(entry), "Entry {} must be present", entry);
        }

        let _ = std::fs::remove_dir_all(&base_dir);
    }

    #[tokio::test]
    async fn test_init_project_harness_integration() {
        let base_dir = std::env::temp_dir().join(format!("cvh_init_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base_dir).expect("create test dir");

        init_project_harness(&base_dir).await.expect("init project harness failed");

        let gi_path = base_dir.join(".gitignore");
        assert!(gi_path.is_file(), ".gitignore must exist");
        let content = std::fs::read_to_string(&gi_path).expect("read gitignore");
        assert!(content.contains(".cvh/harness.db*"));
        assert!(content.contains(".cvh/spill/"));
        assert!(content.contains(".cvh/worktrees/"));
        assert!(content.contains(".cvh/llm.json"));
        assert!(content.contains(".cvh/tasks.json"));

        let cvh = base_dir.join(".cvh");
        assert!(cvh.is_dir());
        assert!(cvh.join("agents").is_dir());
        assert!(cvh.join("config.json").is_file());
        assert!(cvh.join("pipeline.json").is_file());

        let _ = std::fs::remove_dir_all(&base_dir);
    }
}


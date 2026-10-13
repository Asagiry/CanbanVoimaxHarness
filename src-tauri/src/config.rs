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
    pub fn is_tool_allowed(&self, stage_id: &str, tool_name: &str) -> bool {
        let canonical = normalize_tool_name(tool_name);

        if let Some(stage_tools) = self.stage_allowed_tools.get(stage_id) {
            return stage_tools.iter().any(|t| {
                let ct = normalize_tool_name(t);
                ct == canonical || t == tool_name
            });
        }

        if !self.global_allowed_tools.is_empty() {
            return self.global_allowed_tools.iter().any(|t| {
                let ct = normalize_tool_name(t);
                ct == canonical || t == tool_name
            });
        }

        // Default: permitted if no restrictions defined
        true
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineColumn {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub persona: Option<String>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub id: String,
    pub title: String,
    #[serde(rename = "toolPermissions", default)]
    pub tool_permissions: Option<ToolPermissions>,
    pub columns: Vec<PipelineColumn>,
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
            },
            PipelineColumn {
                id: "dev".to_string(),
                title: "In Development".to_string(),
                color: Some("#f59e0b".to_string()),
                icon: Some("Code2".to_string()),
                persona: Some("developer".to_string()),
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
            },
            PipelineColumn {
                id: "review".to_string(),
                title: "Review".to_string(),
                color: Some("#8b5cf6".to_string()),
                icon: Some("CheckSquare".to_string()),
                persona: Some("reviewer".to_string()),
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
            },
            PipelineColumn {
                id: "retro".to_string(),
                title: "Retrospective".to_string(),
                color: Some("#ec4899".to_string()),
                icon: Some("History".to_string()),
                persona: Some("retro".to_string()),
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
            },
            PipelineColumn {
                id: "ready".to_string(),
                title: "Ready / Done".to_string(),
                color: Some("#10b981".to_string()),
                icon: Some("CheckCircle2".to_string()),
                persona: None,
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
            },
        ],
    }
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

    // 4. Default tool policy in settings/ if not present
    let tool_policy_path = settings_dir.join("tool-policy.json");
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
    let pipeline_path = root_path.as_ref().join(".cvh").join("pipeline.json");
    let serialized = serde_json::to_string_pretty(config)
        .context("Failed to serialize PipelineConfig to JSON")?;
    atomic_write_file(&pipeline_path, &serialized).await
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

pub fn is_tool_allowed(
    policy: &ToolPolicyConfig,
    stage_id: Option<&str>,
    tool_name: &str,
) -> bool {
    // Stage-level override takes precedence
    if let Some(stage) = stage_id {
        if let Some(stage_map) = policy.stage_overrides.get(stage) {
            if let Some(&allowed) = stage_map.get(tool_name) {
                return allowed;
            }
        }
    }
    // Global switch fallback
    policy.global_enabled.get(tool_name).copied().unwrap_or(true)
}

pub async fn load_tool_policies(root_path: impl AsRef<Path>) -> Result<ToolPolicyConfig> {
    let policy_path = root_path.as_ref().join(".cvh").join("tool_policy.json");
    if !policy_path.exists() {
        return Ok(ToolPolicyConfig::default());
    }

    let content = tokio::fs::read_to_string(&policy_path)
        .await
        .with_context(|| format!("Failed to read tool policy at {:?}", policy_path))?;

    let config: ToolPolicyConfig = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse tool policy JSON at {:?}", policy_path))?;

    Ok(config)
}

pub async fn save_tool_policies(root_path: impl AsRef<Path>, config: &ToolPolicyConfig) -> Result<()> {
    let policy_path = root_path.as_ref().join(".cvh").join("tool_policy.json");
    let serialized = serde_json::to_string_pretty(config)
        .context("Failed to serialize ToolPolicyConfig to JSON")?;
    atomic_write_file(&policy_path, &serialized).await
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
}

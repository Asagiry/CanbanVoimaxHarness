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
        "list_subagents" => "subagent_list",
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
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
    #[serde(rename = "isIntake", default)]
    pub is_intake: Option<bool>,
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

/// Checks if a directory is a build, compiler, distribution, installer, or runtime dependency folder
/// (`bin`, `release`, `debug`, `target`, `dist`, `node_modules`, `installer`).
/// Such directories must NEVER be treated as project roots.
pub fn is_binary_or_target_dir(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    if path_str.trim().is_empty() {
        return false;
    }
    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        let lower = file_name.to_lowercase();
        if matches!(
            lower.as_str(),
            "bin" | "release" | "debug" | "target" | "dist" | "node_modules" | "installer"
        ) {
            return true;
        }
    }
    // Also inspect path components to catch paths inside Release, bin, dist, installer, debug, or node_modules
    for comp in path.components() {
        if let std::path::Component::Normal(c) = comp {
            let s = c.to_string_lossy().to_lowercase();
            if matches!(
                s.as_str(),
                "release" | "bin" | "debug" | "dist" | "installer" | "node_modules"
            ) {
                return true;
            }
            if !cfg!(test) && s == "target" {
                return true;
            }
        }
    }
    false
}

/// Checks if a directory is the compiled binary distribution directory of the running app.
/// If the directory is a git repository (contains `.git`), it is treated as a valid
/// source codebase (even if named CanbanVoimaxHarness) and permitted.
pub fn is_app_own_directory(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    if path_str.trim().is_empty() {
        return false;
    }
    // If the path contains .git, it is a git repository with source code - allow it!
    if path.join(".git").exists() {
        return false;
    }
    // Block only the directory of the running compiled binary (exe_dir) when there is no .git
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            if path == exe_dir || path.starts_with(exe_dir) {
                return true;
            }
        }
    }
    false
}
pub fn discover_project_root() -> PathBuf {
    let root = discover_project_root_raw();
    if !root.as_os_str().is_empty() && root.exists() && !is_binary_or_target_dir(&root) && !is_app_own_directory(&root) {
        let _ = ensure_project_gitignore(&root);
    }
    root
}

fn discover_project_root_raw() -> PathBuf {
    // 0. Check CLI arguments for an explicitly passed project directory (e.g. `cvh D:\projects\my-app`)
    for arg in std::env::args().skip(1) {
        if arg.starts_with('-') {
            continue;
        }
        let p = PathBuf::from(arg.trim());
        if p.exists() && p.is_dir() && !is_binary_or_target_dir(&p) && !is_app_own_directory(&p) {
            let resolved = resolve_actual_repo_root(&p);
            let target = if !resolved.as_os_str().is_empty() && !is_binary_or_target_dir(&resolved) && !is_app_own_directory(&resolved) {
                resolved
            } else {
                p
            };
            eprintln!(
                "CVH [discover_project_root]: Using explicit project from CLI arg: {:?}",
                target
            );
            persist_active_project_sync(&target);
            return target;
        }
    }

    // 1. Check %LOCALAPPDATA%\com.voimax.cvh\active_project.txt for explicitly saved user project:
    let active_project_file = get_app_config_dir().join("active_project.txt");
    if active_project_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&active_project_file) {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                let p = PathBuf::from(trimmed);
                if p.exists() && p.is_dir() && !is_binary_or_target_dir(&p) && !is_app_own_directory(&p) {
                    let resolved = resolve_actual_repo_root(&p);
                    let target = if !resolved.as_os_str().is_empty() && !is_binary_or_target_dir(&resolved) && !is_app_own_directory(&resolved) {
                        resolved
                    } else {
                        p
                    };
                    eprintln!(
                        "CVH [discover_project_root]: Using active project from {:?}: {:?} (target: {:?})",
                        active_project_file, trimmed, target
                    );
                    return target;
                }
            }
        }
    }

    let current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let current_is_bin = is_binary_or_target_dir(&current);
    let current_is_app = is_app_own_directory(&current);

    // 2. If current_dir is a user project (NOT app codebase and NOT binary/target dir)
    // with .cvh or .git:
    if !current_is_bin && !current_is_app && (current.join(".cvh").exists() || current.join(".git").exists()) {
        let resolved = resolve_actual_repo_root(&current);
        if !resolved.as_os_str().is_empty() && !is_binary_or_target_dir(&resolved) && !is_app_own_directory(&resolved) {
            eprintln!(
                "CVH [discover_project_root]: Found .cvh or .git in external current_dir: {:?} (resolved: {:?})",
                current, resolved
            );
            persist_active_project_sync(&resolved);
            return resolved;
        }
    }

    // 3. Traverse ancestors of current_dir only if current is NOT inside the app's own codebase/bin
    if !current_is_bin && !current_is_app {
        for ancestor in current.ancestors() {
            if ancestor == current || is_binary_or_target_dir(ancestor) || is_app_own_directory(ancestor) {
                continue;
            }
            if ancestor.join(".git").exists() || ancestor.join(".cvh").exists() {
                let resolved = resolve_actual_repo_root(ancestor);
                if !resolved.as_os_str().is_empty() && !is_binary_or_target_dir(&resolved) && !is_app_own_directory(&resolved) {
                    eprintln!(
                        "CVH [discover_project_root]: Found repository in ancestor: {:?} (resolved: {:?})",
                        ancestor, resolved
                    );
                    return resolved;
                }
            }
        }
    }

    // 4. Otherwise, no valid external project found: return empty PathBuf::new()
    // so frontend presents the project selection screen on startup.
    eprintln!(
        "CVH [discover_project_root]: No valid project root discovered (current_dir is app codebase, binary, or unselected: {:?}). Returning empty path.",
        current
    );
    PathBuf::new()
}

/// Resolves the actual project root containing `.cvh` when given either the workspace root
/// or an isolated git worktree directory (§ 2.3 & § 2.23).
pub fn resolve_actual_repo_root(path: &Path) -> PathBuf {
    if path.as_os_str().is_empty() || is_binary_or_target_dir(path) || is_app_own_directory(path) {
        return PathBuf::new();
    }
    if path.join(".cvh").exists() {
        return path.to_path_buf();
    }
    // 1. Traverse ancestors looking for .cvh (skipping binary/target dirs and app own codebase)
    for ancestor in path.ancestors() {
        if ancestor != path
            && ancestor.parent().is_some()
            && !is_binary_or_target_dir(ancestor)
            && !is_app_own_directory(ancestor)
            && ancestor.join(".cvh").exists()
        {
            return ancestor.to_path_buf();
        }
    }
    // 2. If .git is a worktree pointer file, follow gitdir to locate parent workspace
    let git_file = path.join(".git");
    if git_file.is_file() {
        if let Ok(content) = std::fs::read_to_string(&git_file) {
            for line in content.lines() {
                if let Some(gitdir) = line.strip_prefix("gitdir:") {
                    let raw_path = PathBuf::from(gitdir.trim());
                    let gitdir_path = if raw_path.is_relative() {
                        path.join(raw_path)
                    } else {
                        raw_path
                    };
                    for ancestor in gitdir_path.ancestors() {
                        if !is_binary_or_target_dir(ancestor)
                            && !is_app_own_directory(ancestor)
                            && ancestor.join(".cvh").exists()
                        {
                            return ancestor.to_path_buf();
                        }
                    }
                }
            }
        }
    }
    if is_binary_or_target_dir(path) || is_app_own_directory(path) {
        PathBuf::new()
    } else {
        path.to_path_buf()
    }
}

pub fn persist_active_project_sync(path: &Path) {
    let dir = get_app_config_dir();
    let _ = std::fs::create_dir_all(&dir);
    let target = dir.join("active_project.txt");
    if let Err(e) = std::fs::write(&target, path.to_string_lossy().as_bytes()) {
        eprintln!(
            "CVH [persist_active_project_sync]: Failed to write active_project.txt: {}",
            e
        );
    } else {
        eprintln!(
            "CVH [persist_active_project_sync]: Saved active project to {:?}",
            target
        );
    }
}

pub async fn persist_active_project(path: &Path) {
    persist_active_project_sync(path);
}

pub fn default_pipeline_config() -> PipelineConfig {
    PipelineConfig {
        id: "default-engineering".to_string(),
        title: "cvh-board".to_string(),
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
                is_intake: Some(true),
            },
            PipelineColumn {
                id: "dev".to_string(),
                title: "CVH Development".to_string(),
                color: Some("#6366f1".to_string()),
                icon: Some("Bot".to_string()),
                persona: Some("cvh-dev".to_string()),
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
                    "web_search".to_string(),
                ]),
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
                is_intake: None,
            },
            PipelineColumn {
                id: "ready".to_string(),
                title: "Ready".to_string(),
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
                is_intake: None,
            },
        ],
        boards: None,
        active_board_id: None,
    }
}

pub fn default_board_def() -> BoardDef {
    BoardDef {
        id: "development".to_string(),
        name: "cvh-board".to_string(),
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
            is_intake: Some(true),
        },
        PipelineColumn {
            id: "dev".to_string(),
            title: "CVH Development".to_string(),
            color: Some("#6366f1".to_string()),
            icon: Some("Bot".to_string()),
            persona: Some("cvh-dev".to_string()),
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
                "web_search".to_string(),
            ]),
            hooks: HashMap::new(),
            routes: None,
            auto_run: None,
            terminal: None,
            is_intake: None,
        },
        PipelineColumn {
            id: "ready".to_string(),
            title: "Ready".to_string(),
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
            is_intake: None,
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
                is_intake: Some(true),
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
                is_intake: None,
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
                is_intake: None,
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
                is_intake: None,
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

/// Normalizes pipeline columns according to dynamic column invariants:
/// - Validates unique column IDs (duplicate IDs return an Err).
/// - If empty, falls back to canonical board columns.
/// - Preserves user column ordering verbatim without injecting synthetic backlog/ready.
/// - Sets column 0 `is_intake = Some(true)` if unset.
/// - Sets final column `terminal = Some(true)`.
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

    if columns.is_empty() {
        return Ok(get_canonical_board_columns());
    }

    let mut out = columns;
    if out[0].is_intake.is_none() {
        out[0].is_intake = Some(true);
    }
    let last_idx = out.len() - 1;
    out[last_idx].terminal = Some(true);

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

pub const DEFAULT_AGENT_SETTING_JSON: &str = r##"{
  "model": "gemini-3.8-flash-high",
  "context_window": 200000,
  "temperature": 0.2,
  "max_tokens": 8192,
  "icon": "Cpu",
  "color": "#6366f1"
}"##;

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
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
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
                    || ((canonical == "subagent_bg" || canonical == "collect_subagent" || canonical == "subagent_list")
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

/// Minimal fresh-project pipeline: 3 columns — `backlog` (no persona, first),
/// `dev` (CVH Development with `cvh-dev` persona, middle), and `ready` (terminal, last).
pub fn minimal_pipeline_config() -> PipelineConfig {
    PipelineConfig {
        id: "default-engineering".to_string(),
        title: "cvh-board".to_string(),
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
                is_intake: Some(true),
            },
            PipelineColumn {
                id: "dev".to_string(),
                title: "CVH Development".to_string(),
                color: Some("#6366f1".to_string()),
                icon: Some("Bot".to_string()),
                persona: Some("cvh-dev".to_string()),
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
                    "web_search".to_string(),
                ]),
                hooks: HashMap::new(),
                routes: None,
                auto_run: None,
                terminal: None,
                is_intake: None,
            },
            PipelineColumn {
                id: "ready".to_string(),
                title: "Ready".to_string(),
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
                is_intake: None,
            },
        ],
        boards: None,
        active_board_id: None,
    }
}

pub const DEFAULT_CVH_DEV_PERSONA_MD: &str = r##"---
name: cvh-dev
description: Use PROACTIVELY when setting up or repairing boards, pipelines, personas, or hooks.
model: inherit
scope: gated-writes
activation: on-enter dev runs run_agent(taskId)
icon: Cpu
color: #6366f1
---
# Role: CVH-DEV (Project Configuration & Environment Specialist)
You are EXCLUSIVELY responsible for configuring and maintaining CanbanVoimaxHarness project infrastructure (.cvh/):
- Configure boards (.cvh/boards.json) and execution pipelines (.cvh/pipeline.json).
- Author and maintain agent personas, roles, and modular rules (.cvh/agents/<role>/persona.md, rules/*.md).
- Configure tool policies (.cvh/settings/tool-policy.json) and stage transition hooks (.cvh/hooks/).
- JIT Modular Rules: Use `load_rule` to dynamically fetch modular domain rules on demand:
  - `load_rule("role-developer")`: Developer behavioral DNA (OpenCode 4 laws, Ponytail, Zero Silent Failures, Zero-TDD, Worktree isolation).
  - `load_rule("role-reviewer")`: Reviewer behavioral DNA (Presumption of guilt, Claude Code convergence, Zero Silent Failures audit, Read-only).
  - `load_rule("role-retro")`: Retro behavioral DNA (Ironclad engine prohibition, LLM-Wiki CLE protocol, rule-sprawl filter).
  - `load_rule("role-planner")`: Planner behavioral DNA (Read-only, developer context protection, be-ambitious "Deeper, not bigger" doctrine).
  - `load_rule("pipeline-setup")`: Master blueprint and verbatim templates for instant repository configuration.
  - `load_rule("boards")`: Multi-board pipeline configuration and column routing invariants.
  - `load_rule("agents")`: Directory-only persona structure and modular rule standards.
  - `load_rule("hooks")`: Worktree lifecycle hooks and gatekeeper sensors.
  - `load_rule("tools")`: Authoritative standard and custom tool catalog.
- Interactive Pipeline Setup Protocol: When instructed to configure a pipeline for a project (e.g. "настрой пайплайн", "настрой cvh"), ALWAYS call `question` to let the operator choose their desired topology (e.g., Full 4-role with Retro, Standard Dev-Review, Lean Solo, or Custom). Never assume or force a mandatory planner. Imbue configured personas with their respective role DNA (`role-developer`, `role-reviewer`, `role-retro`, `role-planner`) and `pipeline-setup.md`.
- Task Registry & History: Inspect board state via `list_tasks` and review prior execution steps via `get_task_history`.
- Strict Role Boundary on Task Cards (§ 2.8, § 2.16): cvh-dev is strictly forbidden from calling `create_task`, `create_tasks`, or `complete_task`. You configure infrastructure and advance stages via `advance_task` / `move_task` or `reject_task`. Task card creation and completion are strictly reserved for Root Orchestrators and human operators.
- Ironclad Completion Rule: After completing all planned tasks, you MUST cease tool calls and provide a terminal findings report, OR invoke `advance_task(summary)` to transition the task to the next stage. NEVER re-publish completed items in `todo_write` or enter recursive planning loops!
- Backlog-to-Ready Invariant: Always maintain `backlog` at index 0 and `ready` (`terminal: true`) at the final index across all boards.
- Board Reconfiguration Invariant: When creating, renaming, or modifying boards in `.cvh/boards.json` or `.cvh/pipeline.json`, NEVER delete the board or column where your current task is running.
"##;

pub const DEFAULT_RULE_ROLE_DEVELOPER_MD: &str = r##"# Rule: Canonical Developer Behavioral Invariants (role-developer.md)

Authoritative behavioral specifications and architectural DNA for the Developer role in CanbanVoimaxHarness.

---

## 1. The OpenCode 4 Laws Against LLM Over-Defensiveness
1. **Prevent Over-Defensiveness**: Do NOT add defensive checks for boundary or concurrency conditions that are mathematically or structurally improbable. Explain probability and only add upon explicit confirmation. Prohibit privately inflating code just to make it "theoretically more stable".
2. **Single Validation at Boundary**: External data (IPC payloads, HTTP requests, CLI flags, file reads) is validated **strictly once at the system boundary**. Internally, rely 100% on the strict type system. Layered redundant null/undefined checks across internal functions and helpers are strictly banned.
3. **Reject Premature Abstraction**: Logic used in a single location must be inlined locally. Casually splitting code into speculative helper functions ahead of time is prohibited.
4. **Control Complexity & Ban Parasitic Try/Catch**: Do NOT use `try/catch` unless handling an expected, recoverable external I/O failure. Never swallow errors into silent fallback logs.

---

## 2. Ponytail Minimalism & Contrastive Diff Self-Audit
- Solve the exact requirement formulated in the task. No opportunistic refactoring of untouched files.
- Before calling `advance_task`, run `git diff` and audit every single changed line:
  - Eliminate stray `console.log`, debugging dumps, temporary files.
  - Eliminate accidental whitespace churn or unrelated formatting changes.
- Ensure all live Ground Truth Sensors pass (compiler, linter, build).

---

## 3. Code Simplifier & Zero Silent Failures (ECC Invariants)
- **Zero Silent Failures**: Never swallow errors via empty `catch {}`, `let _ = ...`, or deceptive fallback defaults (`.catch(() => [])`, `Err(_) => Default::default()`). Distinguish expected absence from failure. Always preserve error causality and diagnostic context.
- **Code Simplifier Refactoring**: Favor early returns (guard clauses) over nested ladders; inline single-use helpers; break complex chains into clear intermediate variables; clarity over cleverness.

---

## 4. Zero-TDD Mandate
- Never author synthetic unit tests with mocks (`*.test.ts`). Verification relies 100% on live Ground Truth Sensors (compiler, linter, build bundle, static audit invariants).

---

## 5. Worktree Isolation
- All feature implementation takes place in dedicated Git worktrees (`worktrees/<task-slug>`). Never touch root directly on multi-stage boards.
"##;

pub const DEFAULT_RULE_ROLE_REVIEWER_MD: &str = r##"# Rule: Canonical Reviewer Behavioral Invariants (role-reviewer.md)

Authoritative behavioral specifications and architectural DNA for the Reviewer role in CanbanVoimaxHarness.

---

## 1. Read-Only Operational Boundary
- Tools `edit_file` and `write_file` are physically stripped from tool schema via Tool Policy.

---

## 2. Cognitive Stance: Presumption of Guilt
- Assume the diff contains hidden defects, async race conditions, or unhandled edge cases until proven innocent. Three-axis review: Invariants, Task Contract, Ponytail.

---

## 3. Anthropic Claude Code Convergence Protocol
1. **Pre-Existing Bugs Immunity**: The reviewer is **strictly forbidden from rejecting (`reject_task`) for bugs or debt existing prior to the task branch**. Pre-existing issues are noted for the backlog, never used to hold the current PR hostage.
2. **Diff-Only Re-Review**: On round 2+ (after developer addresses feedback), audit strictly the new corrective diff, not the entire historical codebase.
3. **Actionable Line References**: Every objection MUST cite `file:line` with exact, unambiguous remediation requirements. Abstract philosophical complaints are forbidden.
4. **Anti-Nitpicking**: If linters and compilers pass, subjective naming preferences and minor formatting cannot be grounds for rejection.

---

## 4. Zero Silent Failures & Simplification Audit
- Strictly reject any code that masks errors behind empty catch blocks, silent fallbacks, or discarded results without context.
- Reject over-abstracted single-use wrapper functions that introduce cognitive overhead without genuine reuse.
"##;

pub const DEFAULT_RULE_ROLE_RETRO_MD: &str = r##"# Rule: Canonical Retrospective Behavioral Invariants (role-retro.md)

Authoritative behavioral specifications and architectural DNA for the Retro role in CanbanVoimaxHarness.

---

## 1. Ironclad Engine Code Prohibition
- The Retro agent is **STRICTLY FORBIDDEN from touching application source code** (`src/`, `src-tauri/`, `server/`). You NEVER write product features or code fixes.

---

## 2. Karpathy's LLM-Wiki Knowledge Ingest & Rule-Sprawl Filter
1. **Ingest (Accumulation via CLE)**:
   - For every task that encountered non-trivial invariants or reviewer corrections, update 1–2 related rule pages using the **Cue — Lesson — Evidence (CLE)** triad:
     - `Cue`: Structural trigger condition (*"When adding IPC commands..."*).
     - `Lesson`: Actionable operational constraint (*"Always validate on boundary..."*).
     - `Evidence`: Concrete task ID and defect trace.
2. **Rule-Sprawl Filter (Mechanical Hardening Over Prose)**:
   - **Banned**: Writing generic prose rules or philosophical essays for one-off bugs.
   - **Mandated**: Convert manual reviewer findings into automated mechanical sensor assertions in `.agents/scripts/audit/` or transition hooks (`.cvh/hooks/`). If an invariant cannot be checked mechanically, record at most 3-4 lines of CLE into an existing rule page. Never create new rule files for single-instance bugs.
"##;

pub const DEFAULT_RULE_ROLE_PLANNER_MD: &str = r##"# Rule: Canonical Planner Behavioral Invariants (role-planner.md)

Authoritative behavioral specifications and architectural DNA for the Planner role in CanbanVoimaxHarness.

---

## 1. Read-Only Scope & Context Protection
- `edit_file` and `write_file` disabled via Tool Policy.
- Provide exact file paths, line numbers, function signatures, and data contracts so the developer never performs blind searches.

---

## 2. "Deeper, Not Bigger" (be-ambitious) Protocol
Turn tasks into deeper versions of themselves, not bigger ones:
1. **Expert Moves**: What a top senior engineer would do that wasn't asked for (reproduce before fixing, check base rates/callers, state trade-offs).
2. **Rawest Material**: Inspect primary sources rather than summaries (raw AST, raw SQLite WAL/schemas, logs, real code instead of assumptions).
3. **Specific Methods & Tools**: Name concrete Rust types, React hooks, Tauri IPC signatures, compiler flags, and profilers.
4. **External Ground Truth Verification**: Specific sensor commands (`pnpm exec tsc --noEmit`, `cargo check`, exit code 0 on reproduction commands) that prove correctness without trusting the model.
"##;

pub const DEFAULT_RULE_BOARDS_MD: &str = r##"# Guide: Boards and Pipelines Configuration (boards.md)

Authoritative rules for `.cvh/boards.json` and `.cvh/pipeline.json`.

---

## 1. The Kanban Board as Application Control Plane
In CanbanVoimaxHarness, the Kanban board is the runtime execution environment.
Columns represent autonomous execution stages. Moving a task card triggers automated hooks and initiates agent execution loops.

---

## 2. Open, Project-Driven Control Plane Architecture
CanbanVoimaxHarness is an open, domain-agnostic desktop agentic IDE and orchestration environment for ANY project type (Software Dev, 3D/Voxel Art, Content, Lean Kanban).
The Kanban board is a dynamic control plane adaptable to arbitrary project workflows and multi-agent topologies.

The ONLY structural invariants across all boards:
1. **Intake Anchor**: First column (index 0, e.g. `backlog`), `persona: null`, `terminal: null`. Tasks wait here until selected for work.
2. **Delivery Anchor**: Final column (e.g. `ready`), `persona: null`, `terminal: true`. Tasks entering this column are finalized deliverables.
3. **Intermediate Execution Stages**: Completely arbitrary and defined by project requirements (any personas, models, tools, gatekeeper sensors, and routing hooks such as `routes.on_approved` / `routes.on_changes_requested`).

Boards are configured in `.cvh/boards.json` and mirrored in `.cvh/pipeline.json`.

---

## 3. Passive Stages Invariant (backlog and ready)
- `backlog`: Intake and accumulation stage. Tasks wait here until selected for work.
- `ready`: Terminal delivery stage (`terminal: true`). Tasks here are fully verified, merged, and completed.
- **Strict Bot Ban**: Both `backlog` and `ready` MUST have `persona: null` and `autoRun: null`. No autonomous agents ever run in backlog or ready.
- **Terminal Exit Ban**: The `ready` stage must NEVER define an `on_leave` hook.

---

## 4. Self-Healing Multi-Board State
- **Orphaned Board Resolution**: If `task.board_id` is missing or references a non-existent board, the runtime reassigns the task to `active_board_id`.
- **Orphaned Column Resolution**: If `task.stage_id` does not exist on the target board, the task is safely moved to the board's intake column (`backlog`).

---

## 5. Ironclad Board Preservation Invariant
- **Strict Board Isolation**: Tasks MUST strictly preserve their `board_id` across `advance_task`, agent loop completion, and UI moves. Cross-board hopping is forbidden.
- **Autonomous Progression Scope**: `advance_task` and agent finalization MUST advance tasks through columns belonging to the task's own board (`task.board_id`). Tasks must never be abducted to another board simply because the user switched tabs or changed `active_board_id`.
- **UI Drag & Drop**: Moving tasks in the UI preserves `task.boardId` unless explicitly re-assigned.


---

## 5. Multi-Stage Pipeline Topologies & Interactive Setup
When creating or restructuring pipelines, refer to `load_rule("pipeline-setup")`.
`cvh-dev` must interactively ask the user via `question` to select among standard topologies:
1. **Standard Peer Review**: `backlog` ➔ `dev` (Developer) ➔ `review` (Reviewer) ➔ `ready`.
2. **Full Cognitive Triad (CarSmash Model)**: `backlog` ➔ `plan` (Planner) ➔ `dev` (Developer) ➔ `review` (Reviewer) ➔ `retro` (Retro) ➔ `ready`.
3. **Lean Solo**: `backlog` ➔ `dev` (Developer) ➔ `ready`.
4. **Custom**: Defined interactively with the user.
"##;

pub const DEFAULT_RULE_AGENTS_MD: &str = r##"# Guide: Agent Personas and Modular Rules (agents.md)

Authoritative rules for `.cvh/agents/<role>/`.

## 1. Directory-Only Structure & Persona Invariant
All agent definitions reside in dedicated directories under `.cvh/agents/<role>/`:
```text
.cvh/agents/<role>/
├── persona.md       # Primary persona prompt with YAML frontmatter
├── setting          # JSON settings ({ "model": "gemini-3.8-flash-high", "temperature": 0.2, ... })
└── rules/           # Role-specific modular rules
    ├── rule1.md
    └── ...
```

### 1.1 Invariant: 1 Persona = Exactly 1 Directory
- **1 Persona = 1 Directory**: Exactly one directory `.cvh/agents/<persona>/` per persona role.
- **No Column ID Duplication**: Never create duplicate persona directories based on column IDs.
- **Role-Specific Isolation**: Each agent persona MUST maintain only the modular rules relevant to its operational domain inside its own `rules/` catalog.

### 1.2 Agent Settings and Customization (`setting`)
Each agent directory `.cvh/agents/<role>/setting` contains JSON configuration without extension:
```json
{
  "model": "gemini-3.8-flash-high",
  "temperature": 0.2,
  "context_window": 200000,
  "max_tokens": 8192,
  "icon": "Cpu",
  "color": "#6366f1"
}
```
- **Priority & Precedence**: Values defined in `setting` take precedence over YAML frontmatter defaults.
- **Color Invariants**: Gray (`#64748b`) and green (`#10b981`) colors are strictly reserved for system stages `backlog` and `ready` and forbidden for agents.


---

## 4. Persona Creation & Behavioral DNA Infusion
When instantiating personas, `cvh-dev` must NEVER generate shallow placeholder prompts.
Refer to `load_rule("pipeline-setup")` for the complete behavioral specifications:
- **Developer**: OpenCode 4 Laws (anti-overengineering), Ponytail minimal diffs, Zero-TDD, contrastive diff self-audit.
- **Reviewer**: Read-only (Tool Policy Level B), Presumption of Guilt, Three-Axis Review, Anthropic Claude Code convergence (immunity to historical pre-existing bugs, diff-only re-review, line-by-line actionable references).
- **Planner**: Read-only, context window protection for developer, subagent explorer, question-based clarification.
- **Retro**: Engine code prohibition (`src/` strictly forbidden), Karpathy LLM-Wiki knowledge ingest, Cue-Lesson-Evidence protocol, automated audit hardening.
"##;

pub const DEFAULT_RULE_HOOKS_MD: &str = r##"# Guide: Lifecycle Hooks and Transition Scripts (hooks.md)

Authoritative rules for `.cvh/hooks/` and column lifecycle scripts.

---

## 1. Hook Invocation Triggers
Columns declare transition hooks in configuration (`boards.json` and `pipeline.json`) within the `hooks` object:
- `hooks.on_enter`: Executes when a task enters the stage.
- `hooks.on_leave`: Executes when a task exits the stage.

All hooks are executable scripts (`.bat`, `.cmd`, `.ps1`, `.sh`, `.py`) located in `.cvh/hooks/` or inside an agent's directory.

---

## 2. Standard Output Protocol for Worktrees & Git Branches
When a hook script prepares or switches a git worktree or branch, it outputs the following key-value pairs to `stdout`:
```text
CVH_WORKTREE=<worktree_relative_or_absolute_path>
CVH_BRANCH=<git_branch_name>
```
The CVH runtime intercepts these lines from hook stdout and writes them directly to the task card's `worktree_path` and `branch` fields in SQLite.

---

## 3. Injected Task Context Environment Variables
Every hook execution receives the task's context via system environment variables:
- `CVH_TASK_ID`: Unique task identifier (e.g. `CVH-042`).
- `CVH_TASK_TITLE`: Title of the task card.
- `CVH_WORKTREE_PATH`: Absolute or relative path to the task's worktree.
- `CVH_BRANCH`: Target git feature branch.
- `CVH_STAGE_ID`: Target stage identifier.
- `CVH_VERDICT`: Verdict emitted during review exit (`APPROVED`, `APPROVED_WITH_CAVEATS`, `CHANGES_REQUESTED`).

---

## 4. Gatekeeper Sensors
Columns may define `gatekeeperSensor` (e.g. `cargo check` or `npm run build`).
Gatekeeper sensors execute inside `CVH_WORKTREE_PATH` before any card advance. If exit code != 0, transition is blocked.

---

## 5. Hook Execution on `advance_task` & Subtask Handover Protocol
During autonomous transitions via `advance_task`:
1. **Hook Sequence Enforcement**:
   - `on_leave` / `on_exit` hook of current stage is executed.
   - `gatekeeperSensor` of target stage is executed.
   - `on_enter` hook of target stage is executed.
   - If any hook/sensor fails, the transition is blocked immediately: NO new subtasks are created, verdict is set to `CHANGES_REQUESTED`, status is kept `running` (if column has persona/autorun), and error details are injected into task dialog and autorun prompt.
2. **Generic Rejection Handover (Zero Card Duplication)**:
   - When a review subtask rejects via `reject_task(reason)`, the reviewing subtask stays in its column with `idle` status and `CHANGES_REQUESTED`, while the developer subtask in `dev` is reactivated with `running` status and rejection details.
   - When the developer calls `advance_task(summary)`, CVH identifies the existing subtask in the target column, marks developer subtask `done`, reactivates the target subtask with `running` status and `handover_prompt`, without generating duplicate subtask cards (`CVH-X.3`).
"##;

pub const DEFAULT_RULE_PIPELINE_SETUP_MD: &str = r##"# Rule: Interactive Pipeline Setup & Persona Behavioral DNA (pipeline-setup.md)

Authoritative, industrial-grade engineering manual for `cvh-dev` when configuring boards, execution pipelines, agent roles, and physical governance policies for any repository.

---

## 1. The Interactive Setup Protocol (Zero-Assumption Architecture)

When an operator instructs `cvh-dev` to configure or optimize a repository (e.g. *"настрой cvh"*, *"настрой пайплайн"*, *"сделай как в CarSmash"*):
1. **Never Unilaterally Impose Fixed Topologies**: Do NOT assume a project wants a 4-role pipeline or a mandatory `planner`. Every codebase has unique velocity and governance needs.
2. **Execute Interactive Discovery via `question`**:
   Invoke the `question` tool to present clear options to the operator:
   ```json
   {
     "text": "Какую топологию пайплайна и уровень контроля вы хотите развернуть для этого проекта?",
     "options": [
       "Standard Peer Review (dev ➔ review ➔ ready) [Рекомендуется для большинства проектов]",
       "Full Cognitive Triad (plan ➔ dev ➔ review ➔ retro ➔ ready) [Модель CarSmash: максимальная автономность и самообучение]",
       "Lean Solo Dev (dev ➔ ready) [Для быстрых прототипов, скриптов и соло-разработки]",
       "Custom (Задать собственные роли и этапы)"
     ]
   }
   ```
3. **Analyze the Project's Technology Stack**:
   Before generating files, inspect the repository root:
   - TypeScript/JavaScript: look for `package.json`, `tsconfig.json`, `vite.config.ts`.
   - Rust: look for `Cargo.toml`, `src-tauri/`.
   - Python: look for `pyproject.toml`, `requirements.txt`, `main.py`.
   - Go/C++/Other: detect native build tools and linters.
   Tailor all gatekeeper sensors and persona instructions to the *actual detected toolchain*.
   - **Leverage Local `docs/ecc/` Rule Catalog**:
     When configuring rules for a specific programming language or framework (e.g. Python, Go, C++, TypeScript, Rust, React, Swift, etc.), do not invent rules from scratch. Use the repository's local catalog at `docs/ecc/rules/<language>/` (and `docs/ecc/agents/`) as an authoritative, battle-tested source for coding style, security invariants, and patterns, adapting them directly into `.cvh/agents/<role>/rules/<topic>.md`.

---

## 2. Modular Behavioral DNA for Generated Personas

When `cvh-dev` instantiates or configures roles under `.cvh/agents/<role>/`, generating shallow, generic prompts is **strictly forbidden**.
**Zero-Context-Pollution Invariant**: Never load a monolithic rule covering all roles! Load ONLY the specific modular rule needed for the stage being configured:
- **For Developer** (`dev`, fast-track, lean solo): `load_rule("role-developer")`
  - OpenCode 4 Laws against over-defensiveness.
  - Ponytail minimalism & contrastive diff self-audit.
  - Code Simplifier tactics & Zero Silent Failures (silent-failure-hunter).
  - Zero-TDD mandate & isolated worktrees.
- **For Reviewer** (`review`, Red Team auditor): `load_rule("role-reviewer")`
  - Read-only operational boundary (Level B tool policy).
  - Presumption of guilt & 3-axis review (Invariants, Contract, Ponytail).
  - Anthropic Claude Code Convergence Protocol (immunity to historical bugs, diff-only re-review, actionable line refs).
  - Zero Silent Failures audit.
- **For Retro** (`retro`, continuous improvement — when requested): `load_rule("role-retro")`
  - Ironclad engine code prohibition (never touches `src/`).
  - Karpathy's LLM-Wiki CLE protocol (Cue — Lesson — Evidence).
  - Rule-Sprawl Filter: mechanical sensor hardening over verbose prose.
- **For Planner** (`plan`, technical architect — when requested): `load_rule("role-planner")`
  - Read-only operational boundary & context window protection.
  - "Deeper, Not Bigger" (`be-ambitious`) protocol: 4 foundational deepening questions.

---

## 3. Physical Gates over Prose Begging (Tool Policy Level B)

Never rely solely on prompt instructions like *"please don't edit files"*.
`cvh-dev` must configure `.cvh/settings/tool-policy.json` with strict Level B stage overrides:
```json
{
  "globalEnabled": {
    "read_file": true,
    "edit_file": true,
    "write_file": true,
    "glob": true,
    "grep": true,
    "execute_command": true,
    "subagent": true,
    "subagent_bg": true,
    "collect_subagent": true,
    "todo_write": true,
    "advance_task": true,
    "reject_task": true,
    "question": true,
    "list_tasks": true,
    "get_task_history": true,
    "load_rule": true
  },
  "stageOverrides": {
    "plan": {
      "edit_file": false,
      "write_file": false
    },
    "review": {
      "edit_file": false,
      "write_file": false,
      "subagent": false,
      "subagent_bg": false
    },
    "retro": {
      "edit_file": true,
      "write_file": true
    }
  }
}
```
Disabled tools are physically stripped from the LLM tool schemas by the CanbanVoimaxHarness Rust engine, eliminating tool hallucinations and saving ~2,400+ prompt tokens per step.

---

## 4. Generation Blueprints & Directory Layout

When the operator approves a pipeline topology, `cvh-dev` generates:

```text
.cvh/
├── boards.json              # Board definitions matching user choice
├── pipeline.json            # Pipeline routing, columns, and hooks
├── settings/
│   └── tool-policy.json     # Two-level tool permissions matrix
├── hooks/
│   └── gatekeeper.bat       # Fast sensor scripts executed on card moves
└── agents/
    ├── <role_1>/
    │   ├── persona.md       # Frontmatter (icon, color, tools) + Core mandate
    │   ├── setting          # { "model": "inherit", "context_window": 200000 }
    │   └── rules/           # Modular JIT rules for the role
    └── <role_2>/
        ...
```

### 4.1 Persona Frontmatter Blueprint
Every `persona.md` generated by `cvh-dev` MUST include valid YAML frontmatter:
```markdown
---
name: <role-id>
description: <Concise 1-line description of authority>
model: inherit
scope: <gated-writes | gated-auditor | read-only | rule-evolution>
activation: on-enter <column_id> runs run_agent(taskId)
icon: <LucideIconName>   # Code2, Wrench, ShieldCheck, RotateCcw, Brain, Cpu
color: <HexColor>        # #0284c7, #f59e0b, #ec4899, #6366f1, #10b981
---
```

### 4.2 Gatekeeper Sensor Blueprint
If the pipeline includes automated transitions, `cvh-dev` generates a fast gatekeeper script in `.cvh/hooks/gatekeeper.bat` (or `.ps1`) tailored to the project's language:
- For Rust: `cargo check --manifest-path src-tauri/Cargo.toml`
- For TypeScript: `pnpm exec tsc --noEmit && pnpm exec oxlint --deny-warnings`
- For Python: `python -m py_compile ... && ruff check`

This ensures that no broken code can advance across stages, guaranteeing high First-Time-Right (FTR) execution across all tasks.
---

## 5. Canonical Verbatim File Templates for Instant Setup

When `cvh-dev` generates agent roles for a project, it MUST write these exact, battle-tested templates:

### 5.1 Template: `.cvh/agents/developer/rules/ponytail.md`
```markdown
# Rule: Ponytail Minimalism & OpenCode Anti-Overengineering (ponytail.md)

Authoritative code craftsmanship and simplicity standard.

## 1. The OpenCode 4 Laws Against LLM Over-Defensiveness
1. **Prevent Over-Defensiveness**: Do NOT add defensive checks for boundary or concurrency conditions that are mathematically or structurally improbable. Explain probability and only add upon explicit confirmation. Prohibit privately inflating code just to make it "theoretically more stable".
2. **Single Validation at Boundary**: External data (IPC payloads, HTTP requests, CLI flags, file reads) is validated **strictly once at the system boundary**. Internally, rely 100% on the strict type system. Layered redundant null/undefined checks across internal functions and helpers are strictly banned.
3. **Reject Premature Abstraction**: Logic used in a single location must be inlined locally. Casually splitting code into speculative helper functions ahead of time is prohibited.
4. **Control Complexity & Ban Parasitic Try/Catch**: Do NOT use `try/catch` unless handling an expected, recoverable external I/O failure. Never swallow errors into silent fallback logs.

## 2. Ponytail Contrastive Diff Self-Audit
- Solve the exact requirement formulated in the task. No opportunistic refactoring of untouched files.
- Before calling `advance_task`, run `git diff` and audit every single changed line:
  - Eliminate stray `console.log`, debugging dumps, temporary files.
  - Eliminate accidental whitespace churn or unrelated formatting changes.
- Ensure all live Ground Truth Sensors pass (compiler, linter, build).
```

### 5.2 Template: `.cvh/agents/reviewer/rules/review-checklist.md`
```markdown
# Rule: Review Checklist & Claude Code Convergence Protocol (review-checklist.md)

Authoritative Red Team code audit protocol.

## 1. Cognitive Stance: Presumption of Guilt
Assume the diff contains hidden defects, async race conditions, or unhandled edge cases until proven innocent.

## 2. Three-Axis Review Framework
1. **Invariants & Domain Safety**: Memory leaks, unmounted timer cleanups, async races, zero-GC hot loops.
2. **Task Contract**: Did the developer fulfill the exact requirements? No silent dropping of edge cases.
3. **Ponytail Craft**: Minimal surgical diff, no dead code, no duplicate abstractions.

## 3. Anthropic Claude Code Convergence Protocol
1. **Pre-Existing Bugs Immunity**: The reviewer is **strictly forbidden from rejecting (`reject_task`) for bugs or debt existing prior to the task branch**. Pre-existing issues are noted for the backlog, never used to hold the current PR hostage.
2. **Diff-Only Re-Review**: On round 2+ (after developer addresses feedback), audit strictly the new corrective diff, not the entire historical codebase.
3. **Actionable Line References**: Every objection MUST cite `file:line` with exact, unambiguous remediation requirements. Abstract philosophical complaints are forbidden.
4. **Anti-Nitpicking**: If linters and compilers pass, subjective naming preferences and minor formatting cannot be grounds for rejection.
```

### 5.3 Template: `.cvh/agents/retro/persona.md` (When Retro Requested)
```markdown
---
name: retro
description: Retrospective verifier, rule evolution specialist, and environment optimizer.
model: inherit
scope: rule-evolution
activation: on-enter retro runs run_agent(taskId)
icon: RotateCcw
color: #ec4899
---
# Role: RETRO (Continuous Improvement & Rule Evolution Specialist)

## 1. Operational Mandate & Engine Code Prohibition
- **IRONCLAD ENGINE CODE PROHIBITION**: The Retro agent is **STRICTLY FORBIDDEN from modifying application source code** (`src/`, `src-tauri/`, `server/`). You NEVER write product features or code fixes.
- **Analysis Scope**: Inspect task trajectory via `get_task_history(task_id, include_tool_outputs=true)`.
- **Target Changes**: You modify ONLY rules (`.cvh/agents/<role>/rules/`), audit sensors (`.agents/scripts/audit/`), or task history ledgers.
- **Completion**: Commit rule updates and advance to `ready` via `advance_task(summary)`.
```

### 5.4 Template: `.cvh/agents/retro/rules/rule-evolution.md` (When Retro Requested)
```markdown
# Rule: Karpathy's LLM-Wiki Knowledge Ingest & Mechanical Linting (rule-evolution.md)

Authoritative protocol for rule evolution and automated invariant hardening.

## 1. Karpathy's LLM-Wiki Protocol
1. **Ingest (Accumulation)**:
   - For every task that encountered non-trivial invariants or reviewer corrections, update 1–2 related rule pages using the **Cue — Lesson — Evidence (CLE)** triad:
     - **`Cue` (Trigger Context)**: Structural condition triggering the rule (*"When adding IPC commands..."*).
     - **`Lesson` (Prescriptive Invariant)**: Actionable operational constraint (*"Always validate on boundary..."*).
     - **`Evidence` (Ground Truth Trace)**: Exact task ID, rejection quote, commit hash.
2. **Query (Zero-Turn Priming)**:
   - Future agents inherit these modular rules automatically via `load_rule` without searching raw history.
3. **Lint (Mechanical Hardening)**:
   - Convert manual reviewer findings into automated mechanical sensor assertions in `.agents/scripts/audit/` or transition scripts. Never leave an invariant as mere prose if it can be verified mechanically.
```
"##;

/// Ensure .cvh/agents/cvh-dev/ exists on disk with persona.md, setting (context_window: 200000),
/// and modular progressive disclosure rules (boards.md, agents.md, hooks.md, pipeline-setup.md).
pub async fn ensure_cvh_dev_scaffold(repo_root: &Path) -> Result<(), anyhow::Error> {
    let cvh_dir = repo_root.join(".cvh");
    let agents_dir = cvh_dir.join("agents");
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
    } else if let Ok(content) = tokio::fs::read_to_string(&setting_path).await {
        // When seeding .cvh/agents/cvh-dev/setting, ensure 'context_window': 200000
        if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&content) {
            let needs_update = val.get("context_window").and_then(|v| v.as_u64()).is_none_or(|w| w < 200000);
            if needs_update {
                val["context_window"] = serde_json::json!(200000);
                if let Ok(updated) = serde_json::to_string_pretty(&val) {
                    let _ = atomic_write_file(&setting_path, &updated).await;
                }
            }
        }
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
    let pipeline_setup_md_path = cvh_dev_rules_dir.join("pipeline-setup.md");
    if !pipeline_setup_md_path.exists() {
        atomic_write_file(&pipeline_setup_md_path, DEFAULT_RULE_PIPELINE_SETUP_MD).await?;
    }
    let role_developer_md_path = cvh_dev_rules_dir.join("role-developer.md");
    if !role_developer_md_path.exists() {
        atomic_write_file(&role_developer_md_path, DEFAULT_RULE_ROLE_DEVELOPER_MD).await?;
    }
    let role_reviewer_md_path = cvh_dev_rules_dir.join("role-reviewer.md");
    if !role_reviewer_md_path.exists() {
        atomic_write_file(&role_reviewer_md_path, DEFAULT_RULE_ROLE_REVIEWER_MD).await?;
    }
    let role_retro_md_path = cvh_dev_rules_dir.join("role-retro.md");
    if !role_retro_md_path.exists() {
        atomic_write_file(&role_retro_md_path, DEFAULT_RULE_ROLE_RETRO_MD).await?;
    }
    let role_planner_md_path = cvh_dev_rules_dir.join("role-planner.md");
    if !role_planner_md_path.exists() {
        atomic_write_file(&role_planner_md_path, DEFAULT_RULE_ROLE_PLANNER_MD).await?;
    }
    let legacy_canonical_roles_path = cvh_dev_rules_dir.join("canonical-roles.md");
    if legacy_canonical_roles_path.exists() {
        let _ = tokio::fs::remove_file(&legacy_canonical_roles_path).await;
    }

    Ok(())
}

/// Automatically scaffold the minimal .cvh workspace structure if missing.
/// Creates: empty `.cvh/agents/`, `.cvh/dialogs/`,
/// `.cvh/skills/`, `.cvh/settings/`, `.cvh/hooks/` and default
/// config/pipeline/tool-policy files. Fresh scaffolds seed NO agents:
/// `pipeline.json` holds only `backlog` (no persona, first) + `ready`
/// (terminal, last); the first agent is created by the user via UI.
pub async fn scaffold_cvh_project(repo_root: &Path) -> Result<()> {
    if repo_root.as_os_str().is_empty() {
        return Err(anyhow::anyhow!("Cannot scaffold .cvh in an empty project root path"));
    }
    if !cfg!(test) && (is_binary_or_target_dir(repo_root) || is_app_own_directory(repo_root)) {
        return Err(anyhow::anyhow!(
            "Refusing to scaffold .cvh in forbidden or binary/target/app directory: {:?}",
            repo_root
        ));
    }
    let cvh_dir = repo_root.join(".cvh");
    let agents_dir = cvh_dir.join("agents");
    let dialogs_dir = cvh_dir.join("dialogs");
    let skills_dir = cvh_dir.join("skills");
    let settings_dir = cvh_dir.join("settings");
    let hooks_dir = cvh_dir.join("hooks");

    tokio::fs::create_dir_all(&cvh_dir).await?;
    tokio::fs::create_dir_all(&agents_dir).await?;
    tokio::fs::create_dir_all(&dialogs_dir).await?;
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

    // 2. pipeline.json if not present (minimal 3 columns: backlog -> dev -> ready).
    let pipeline_path = cvh_dir.join("pipeline.json");
    let initial_columns = if !pipeline_path.exists() {
        let minimal_pipeline = minimal_pipeline_config();
        atomic_write_file(&pipeline_path, &serde_json::to_string_pretty(&minimal_pipeline)?).await?;
        minimal_pipeline.columns
    } else {
        load_pipeline_config(repo_root).await.map(|p| p.columns).unwrap_or_else(|_| minimal_pipeline_config().columns)
    };

    // 2b. boards.json if not present: seed default 3-column board with persona: 'cvh-dev'
    let boards_path = cvh_dir.join("boards.json");
    if !boards_path.exists() {
        let default_board = BoardDef {
            id: "development".to_string(),
            name: "cvh-board".to_string(),
            columns: initial_columns,
        };
        let boards_cfg = BoardsConfig {
            active_board_id: "development".to_string(),
            boards: vec![default_board],
        };
        atomic_write_file(&boards_path, &serde_json::to_string_pretty(&boards_cfg)?).await?;
    }

    // Seed modular cvh-dev router persona, settings, and progressive disclosure rules
    // so any newly scaffolded project immediately has these guides available on disk.
    ensure_cvh_dev_scaffold(repo_root).await?;

    // 3. Default tool policy in settings/ if not present.
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

pub const CVH_GITIGNORE_HEADER: &str = "# CanbanVoimaxHarness Internal Runtime & Database";
pub const CVH_GITIGNORE_LEGACY_HEADER: &str = "# CanbanVoimaxHarness Local Workspace";
pub const CVH_GITIGNORE_ANCIENT_HEADER: &str = "# CanbanVoimaxHarness Local Runtime";
pub const CVH_RUNTIME_GITIGNORE_ENTRIES: &[&str] = &[
    ".cvh/harness.db*",
    ".cvh/worktrees/",
    ".cvh/dialogs/",
    ".cvh/spill/",
    ".cvh/logs/",
    ".cvh/tasks/",
    ".cvh/tasks.json",
    ".cvh/llm.json",
    "**/.cvh/llm.json",
];

/// Ensures that `<repo_root>/.gitignore` exists and contains all required
/// local CVH runtime entries:
/// - `.cvh/`
/// - `.cvh/worktrees/`
/// - `.cvh/llm.json`
/// - `.cvh/harness.db*`
/// - `.cvh/spill/`
/// - `.cvh/tasks.json`
///
/// If any of these entries are missing (and no wildcard `.cvh/` or `.cvh` covers them),
/// appends the missing lines with the clean section header `# CanbanVoimaxHarness Local Workspace`.
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
    let has_header = lines.iter().any(|l| {
        let t = l.trim();
        t == CVH_GITIGNORE_HEADER
            || t == CVH_GITIGNORE_LEGACY_HEADER
            || t == CVH_GITIGNORE_ANCIENT_HEADER
    });

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
    #[serde(rename = "hasProject", default)]
    pub has_project: Option<bool>,
}

impl ProjectEntry {
    pub fn from_path(path: &Path) -> Self {
        let is_valid = !path.as_os_str().is_empty()
            && path.exists()
            && !is_binary_or_target_dir(path)
            && !is_app_own_directory(path);
        let path_str = if is_valid {
            path.to_string_lossy().to_string()
        } else {
            String::new()
        };
        let name = if is_valid {
            path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unnamed Project")
                .to_string()
        } else {
            String::new()
        };
        let id = if is_valid {
            name.to_lowercase().replace(' ', "-")
        } else {
            String::new()
        };
        Self {
            id,
            name,
            path: path_str.clone(),
            root_path: if path_str.is_empty() { None } else { Some(path_str) },
            last_opened: Utc::now(),
            active_branch: if is_valid { Some("main".to_string()) } else { None },
            worktrees_count: Some(0),
            has_project: Some(is_valid),
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
    "subagent_list",
    "todo_write",
    "advance_task",
    "move_task",
    "reject_task",
    "question",
    "create_task",
    "complete_task",
    "list_tasks",
    "get_task_history",
    "load_rule",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPolicyConfig {
    #[serde(default = "default_global_tools", rename = "globalEnabled")]
    pub global_enabled: HashMap<String, bool>,
    #[serde(default, rename = "stageOverrides")]
    pub stage_overrides: HashMap<String, HashMap<String, bool>>,
    #[serde(default = "default_tool_settings", rename = "toolSettings")]
    pub tool_settings: HashMap<String, serde_json::Value>,
    #[serde(default, rename = "toolDescriptions")]
    pub tool_descriptions: HashMap<String, String>,
    #[serde(default, rename = "toolCustomSchemas")]
    pub tool_custom_schemas: HashMap<String, serde_json::Value>,
}

pub fn default_tool_settings() -> HashMap<String, serde_json::Value> {
    let mut map = HashMap::new();
    map.insert("read_file".to_string(), serde_json::json!({ "max_lines": 2000 }));
    map.insert("subagent".to_string(), serde_json::json!({ "max_concurrent": 3 }));
    map.insert("subagent_bg".to_string(), serde_json::json!({ "max_concurrent": 3 }));
    map.insert("todo_write".to_string(), serde_json::json!({ "min_items": 3, "max_items": 20 }));
    map.insert("execute_command".to_string(), serde_json::json!({ "timeout_sec": 180 }));
    map
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
    map.insert("collect_subagent".to_string(), true);
    map.insert("subagent_list".to_string(), true);
    map.insert("todo_write".to_string(), true);
    map.insert("advance_task".to_string(), true);
    map.insert("reject_task".to_string(), true);
    map.insert("question".to_string(), true);
    map.insert("create_task".to_string(), true);
    map.insert("complete_task".to_string(), true);
    map.insert("list_tasks".to_string(), true);
    map.insert("get_task_history".to_string(), true);
    map.insert("load_rule".to_string(), true);
    map
}

impl Default for ToolPolicyConfig {
    fn default() -> Self {
        let mut stage_overrides = HashMap::new();

        // Review stage: read-only code tools; web, subagent, todo_write, navigation enabled; edit/write disabled; task card manipulation disabled (§ 2.8)
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
        review_map.insert("subagent_bg".to_string(), true);
        review_map.insert("collect_subagent".to_string(), true);
        review_map.insert("subagent_list".to_string(), true);
        review_map.insert("todo_write".to_string(), true);
        review_map.insert("advance_task".to_string(), true);
        review_map.insert("reject_task".to_string(), true);
        review_map.insert("question".to_string(), true);
        review_map.insert("create_task".to_string(), false);
        review_map.insert("complete_task".to_string(), false);
        review_map.insert("list_tasks".to_string(), true);
        review_map.insert("get_task_history".to_string(), true);
        review_map.insert("load_rule".to_string(), true);
        stage_overrides.insert("review".to_string(), review_map);

        // Dev stage: code editing, bash, search, fetch, subagent, todo_write enabled; card tools enabled for orchestrator tasks in dev
        let mut dev_map = HashMap::new();
        dev_map.insert("execute_command".to_string(), true);
        dev_map.insert("read_file".to_string(), true);
        dev_map.insert("edit_file".to_string(), true);
        dev_map.insert("write_file".to_string(), true);
        dev_map.insert("glob".to_string(), true);
        dev_map.insert("grep".to_string(), true);
        dev_map.insert("web_search".to_string(), true);
        dev_map.insert("web_fetch".to_string(), true);
        dev_map.insert("subagent".to_string(), true);
        dev_map.insert("subagent_bg".to_string(), true);
        dev_map.insert("collect_subagent".to_string(), true);
        dev_map.insert("subagent_list".to_string(), true);
        dev_map.insert("todo_write".to_string(), true);
        dev_map.insert("advance_task".to_string(), true);
        dev_map.insert("reject_task".to_string(), true);
        dev_map.insert("question".to_string(), true);
        dev_map.insert("create_task".to_string(), true);
        dev_map.insert("complete_task".to_string(), true);
        dev_map.insert("list_tasks".to_string(), true);
        dev_map.insert("get_task_history".to_string(), true);
        dev_map.insert("load_rule".to_string(), true);
        stage_overrides.insert("dev".to_string(), dev_map);

        // Retro stage: audit and ledger logging enabled; search, fetch, subagent, todo enabled; edit/task-creation disabled
        let mut retro_map = HashMap::new();
        retro_map.insert("read_file".to_string(), true);
        retro_map.insert("execute_command".to_string(), true);
        retro_map.insert("write_file".to_string(), true);
        retro_map.insert("glob".to_string(), true);
        retro_map.insert("grep".to_string(), true);
        retro_map.insert("web_search".to_string(), true);
        retro_map.insert("web_fetch".to_string(), true);
        retro_map.insert("subagent".to_string(), true);
        retro_map.insert("subagent_bg".to_string(), true);
        retro_map.insert("collect_subagent".to_string(), true);
        retro_map.insert("subagent_list".to_string(), true);
        retro_map.insert("todo_write".to_string(), true);
        retro_map.insert("advance_task".to_string(), true);
        retro_map.insert("reject_task".to_string(), true);
        retro_map.insert("question".to_string(), true);
        retro_map.insert("create_task".to_string(), false);
        retro_map.insert("complete_task".to_string(), false);
        retro_map.insert("list_tasks".to_string(), true);
        retro_map.insert("get_task_history".to_string(), true);
        retro_map.insert("load_rule".to_string(), true);
        stage_overrides.insert("retro".to_string(), retro_map);

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
            tool_settings: default_tool_settings(),
            tool_descriptions: HashMap::new(),
            tool_custom_schemas: HashMap::new(),
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

    // Hierarchical Subagent Policy per stage:
    // If checking 'subagent', 'subagent_bg', or 'collect_subagent':
    // The master switch 'subagent' for this stage/role controls availability directly.
    if canonical == "subagent" || canonical == "subagent_bg" || canonical == "collect_subagent" || canonical == "subagent_list" {
        if let Some(stage) = stage_id {
            if let Some(stage_map) = policy.stage_overrides.get(stage) {
                if let Some(master_stage) = policy_lookup(stage_map, "subagent") {
                    if !master_stage {
                        return false;
                    }
                }
                if canonical == "subagent" {
                    if let Some(sync_stage) = policy_lookup(stage_map, "subagent_sync") {
                        if !sync_stage {
                            return false;
                        }
                    }
                    return true;
                }
                if canonical == "subagent_bg" || canonical == "collect_subagent" || canonical == "subagent_list" {
                    if let Some(bg_stage) = policy_lookup(stage_map, "subagent_bg") {
                        if !bg_stage {
                            return false;
                        }
                    }
                    return true;
                }
            }
        }
        return true;
    }

    // Single Authoritative Layer: stage_overrides directly dictates tool availability.
    // If the stage/role explicitly configures the tool (true/false), that decision is final.
    match stage_id {
        Some(stage) => {
            if let Some(stage_map) = policy.stage_overrides.get(stage) {
                policy_lookup(stage_map, canonical)
                    .or_else(|| policy_lookup(&policy.global_enabled, canonical))
                    .unwrap_or(true)
            } else {
                policy_lookup(&policy.global_enabled, canonical).unwrap_or(true)
            }
        }
        None => policy_lookup(&policy.global_enabled, canonical).unwrap_or(true),
    }
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
    // Note: Two-way YAML persona frontmatter synchronization is eradicated.
    // Backend tool schemas are injected dynamically from ToolPolicyConfig without touching persona.md.
    Ok(())
}

/// Synchronizes allowed tools from `ToolPolicyConfig` into each agent persona's
/// `.cvh/agents/<role>/persona.md` YAML frontmatter `tools: [...]`.
pub async fn sync_tool_policies_to_personas(
    _root_path: impl AsRef<Path>,
    _config: &ToolPolicyConfig,
) -> Result<()> {
    // Deprecated: Two-way YAML synchronization is eradicated.
    // Tool permissions are governed directly by ToolPolicyConfig.
    Ok(())
}

pub fn strip_frontmatter_tools(content: &str) -> String {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return content.to_string();
    }

    let rest = &trimmed[3..];
    if let Some(end_idx) = rest.find("\n---") {
        let frontmatter = &rest[..end_idx];
        let body = &rest[end_idx + 4..];

        let mut lines: Vec<String> = Vec::new();
        let mut in_tools = false;

        for line in frontmatter.lines() {
            let line_trim = line.trim();
            if line_trim.starts_with("tools:") {
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

        let new_frontmatter = lines.join("\n");
        let new_fm_trimmed = new_frontmatter.trim();
        if new_fm_trimmed.is_empty() {
            body.trim_start_matches('\n').to_string()
        } else {
            format!("---\n{}\n---{}", new_fm_trimmed, body)
        }
    } else {
        content.to_string()
    }
}

pub fn update_frontmatter_tools(content: &str, _new_tools_line: &str) -> String {
    strip_frontmatter_tools(content)
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
        assert!(cvh.join("skills").is_dir());
        assert!(cvh.join("settings").is_dir());

        // Verify cvh-dev modular rules are seeded
        let cvh_dev = cvh.join("agents").join("cvh-dev");
        assert!(cvh_dev.join("persona.md").is_file());
        assert!(cvh_dev.join("rules").join("boards.md").is_file());
        assert!(cvh_dev.join("rules").join("agents.md").is_file());
        assert!(cvh_dev.join("rules").join("hooks.md").is_file());
        assert!(cvh_dev.join("rules").join("pipeline-setup.md").is_file());

        // Ensure no legacy flat persona files exist in agents/
        let mut read_agents = tokio::fs::read_dir(cvh.join("agents")).await.expect("Read dir failed");
        while let Ok(Some(entry)) = read_agents.next_entry().await {
            assert!(entry.path().is_dir(), "Expected only directories in agents/, found: {:?}", entry.path());
        }

        let _ = tokio::fs::remove_dir_all(&base_dir).await;
    }

    #[tokio::test]
    async fn test_ensure_cvh_dev_scaffold_seeding() {
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_dev_scaffold_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base_dir).await.expect("create test dir");

        // Run ensure_cvh_dev_scaffold on empty directory
        ensure_cvh_dev_scaffold(&base_dir).await.expect("ensure_cvh_dev_scaffold failed");

        let cvh_dev = base_dir.join(".cvh").join("agents").join("cvh-dev");
        assert!(cvh_dev.join("persona.md").is_file());
        assert!(cvh_dev.join("setting").is_file());
        assert!(cvh_dev.join("rules").join("boards.md").is_file());
        assert!(cvh_dev.join("rules").join("agents.md").is_file());
        assert!(cvh_dev.join("rules").join("hooks.md").is_file());

        // Check context_window in setting
        let setting_str = tokio::fs::read_to_string(cvh_dev.join("setting")).await.expect("read setting");
        let setting_val: serde_json::Value = serde_json::from_str(&setting_str).expect("parse setting");
        assert_eq!(setting_val.get("context_window").and_then(|v| v.as_u64()), Some(200000));

        // Calling it again is idempotent
        ensure_cvh_dev_scaffold(&base_dir).await.expect("ensure_cvh_dev_scaffold re-run failed");
        assert!(cvh_dev.join("persona.md").is_file());

        let _ = tokio::fs::remove_dir_all(&base_dir).await;
    }

    #[tokio::test]
    async fn test_scaffold_minimal_backlog_ready_only() {
        // Fresh scaffold: pipeline.json holds exactly 3 columns:
        // backlog (no persona, first), dev (cvh-dev persona), ready (terminal, last).
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_min_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base_dir).await.expect("test dir");

        scaffold_cvh_project(&base_dir).await.expect("Scaffold failed");

        let cvh = base_dir.join(".cvh");
        let pipe_raw = tokio::fs::read_to_string(cvh.join("pipeline.json"))
            .await
            .expect("pipeline.json missing");
        let pipe: PipelineConfig = serde_json::from_str(&pipe_raw).expect("pipeline parse");
        assert_eq!(pipe.columns.len(), 3, "minimal scaffold must have exactly 3 columns");
        assert_eq!(pipe.columns[0].id, "backlog");
        assert_eq!(pipe.columns[0].title, "Backlog");
        assert_eq!(pipe.columns[0].persona, None, "backlog must have no persona");
        assert_eq!(pipe.columns[1].id, "dev");
        assert_eq!(pipe.columns[1].title, "CVH Development");
        assert_eq!(pipe.columns[1].persona.as_deref(), Some("cvh-dev"), "dev must have cvh-dev persona");
        assert_eq!(pipe.columns[2].id, "ready");
        assert_eq!(pipe.columns[2].title, "Ready");
        assert_eq!(pipe.columns[2].terminal, Some(true), "ready must be terminal last");

        // Verify boards.json is also created and has 3 columns with cvh-dev
        let boards_raw = tokio::fs::read_to_string(cvh.join("boards.json"))
            .await
            .expect("boards.json missing");
        let boards_cfg: BoardsConfig = serde_json::from_str(&boards_raw).expect("boards parse");
        assert_eq!(boards_cfg.boards.len(), 1);
        assert_eq!(boards_cfg.boards[0].columns.len(), 3);
        assert_eq!(boards_cfg.boards[0].columns[1].persona.as_deref(), Some("cvh-dev"));

        // cvh-dev setting must have context_window 200000
        let cvh_dev = cvh.join("agents").join("cvh-dev");
        let setting_raw = tokio::fs::read_to_string(cvh_dev.join("setting"))
            .await
            .expect("setting missing");
        let parsed_setting: AgentSetting = serde_json::from_str(&setting_raw).expect("setting parse");
        assert_eq!(parsed_setting.context_window, Some(200000));

        // cvh-dev seeded with modular rules, no flat personas
        assert!(cvh_dev.join("persona.md").is_file());
        assert!(cvh_dev.join("rules").join("boards.md").is_file());
        let mut rd = tokio::fs::read_dir(cvh.join("agents")).await.expect("agents dir");
        while let Ok(Some(entry)) = rd.next_entry().await {
            assert!(entry.path().is_dir(), "Expected only directories in agents/, found: {:?}", entry.path());
        }

        // remaining scaffold files/dirs in place
        assert!(cvh.join("dialogs").is_dir());
        assert!(cvh.join("config.json").is_file());
        assert!(!cvh.join("tasks.json").exists(), "tasks.json must not be scaffolded");
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

        // Valid snake_case + camelCase aliases + icon/color.
        tokio::fs::write(
            dir.join("setting"),
            r##"{"model": "m1", "temperature": 0.7, "contextWindow": 64000, "allowedTools": ["read_file", "bash"], "icon": "Sparkles", "color": "#14b8a6"}"##,
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
        assert_eq!(valid.icon.as_deref(), Some("Sparkles"));
        assert_eq!(valid.color.as_deref(), Some("#14b8a6"));

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
        let base = std::path::PathBuf::from("target").join(format!("cvh_hook_{}", uuid::Uuid::new_v4()));
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
    fn test_strip_frontmatter_tools() {
        let content_single = "---\nname: dev\ntools: [read_file, glob]\nmodel: inherit\n---\n# Dev persona";
        let stripped = strip_frontmatter_tools(content_single);
        assert!(!stripped.contains("tools:"));
        assert!(stripped.contains("name: dev"));
        assert!(stripped.contains("model: inherit"));
        assert!(stripped.contains("# Dev persona"));

        let content_multi = "---\nname: dev\ntools:\n  - read_file\n  - glob\nmodel: inherit\n---\n# Dev persona";
        let stripped_multi = strip_frontmatter_tools(content_multi);
        assert!(!stripped_multi.contains("tools:"));
        assert!(!stripped_multi.contains("read_file"));
        assert!(stripped_multi.contains("name: dev"));
        assert!(stripped_multi.contains("model: inherit"));
        assert!(stripped_multi.contains("# Dev persona"));
    }

    #[test]
    fn test_tool_policy_stage_overrides_authoritative() {
        // Single Authoritative Layer: stage_overrides directly dictates tool availability.
        // An explicit stage/role override controls the agent directly.
        let mut policy = ToolPolicyConfig::default();

        // dev explicitly allows read_file
        policy
            .stage_overrides
            .entry("dev".to_string())
            .or_default()
            .insert("read_file".to_string(), true);
        assert!(is_tool_allowed(&policy, Some("dev"), "read_file"));

        // dev explicitly disables read_file
        policy
            .stage_overrides
            .entry("dev".to_string())
            .or_default()
            .insert("read_file".to_string(), false);
        assert!(!is_tool_allowed(&policy, Some("dev"), "read_file"));

        // Fallback for stages with no record: falls back to global_enabled
        policy.global_enabled.insert("glob".to_string(), false);
        assert!(!is_tool_allowed(&policy, Some("no-such-stage"), "glob"));
        assert!(!is_tool_allowed(&policy, None, "glob"));
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
        assert_eq!(cfg.boards[0].name, "cvh-board");
        assert_eq!(cfg.boards[0].columns.len(), 3);
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
        assert_eq!(cols[0].title, "Backlog");
        assert_eq!(cols[0].persona, None);
        assert_eq!(cols[1].id, "dev");
        assert_eq!(cols[1].title, "CVH Development");
        assert_eq!(cols[1].persona.as_deref(), Some("cvh-dev"));
        assert_eq!(cols[2].id, "ready");
        assert_eq!(cols[2].title, "Ready");
        assert_eq!(cols[2].persona, None);
        assert_eq!(cols[2].terminal, Some(true));
    }

    #[tokio::test]
    async fn test_boards_config_persistence_and_sync() {
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_test_boards_{}", uuid::Uuid::new_v4()));
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
        assert_eq!(pipe.columns.len(), 4);

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
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_test_boards_fb_{}", uuid::Uuid::new_v4()));
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
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_gi_create_{}", uuid::Uuid::new_v4()));
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
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_gi_append_{}", uuid::Uuid::new_v4()));
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
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_gi_idempotent_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base_dir).expect("create test dir");

        ensure_project_gitignore(&base_dir).expect("first run");
        let content_first = std::fs::read_to_string(base_dir.join(".gitignore")).expect("read first");

        ensure_project_gitignore(&base_dir).expect("second run");
        let content_second = std::fs::read_to_string(base_dir.join(".gitignore")).expect("read second");

        assert_eq!(content_first, content_second, "Second run must not modify gitignore");

        let header_count = content_second.matches(CVH_GITIGNORE_HEADER).count();
        assert_eq!(header_count, 1, "Header must appear exactly once");

        for entry in CVH_RUNTIME_GITIGNORE_ENTRIES {
            let entry_count = content_second.lines().filter(|l| l.trim() == *entry).count();
            assert_eq!(entry_count, 1, "Entry {} must appear exactly once", entry);
        }

        let _ = std::fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn test_ensure_project_gitignore_wildcard_covered() {
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_gi_wildcard_{}", uuid::Uuid::new_v4()));
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
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_gi_partial_{}", uuid::Uuid::new_v4()));
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
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_init_test_{}", uuid::Uuid::new_v4()));
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

        assert!(content.contains(".cvh/"));
        let cvh = base_dir.join(".cvh");
        assert!(cvh.is_dir());
        assert!(cvh.join("agents").is_dir());
        assert!(cvh.join("config.json").is_file());
        assert!(cvh.join("pipeline.json").is_file());

        let _ = tokio::fs::remove_dir_all(&base_dir).await;
    }

    #[test]
    fn test_is_binary_or_target_dir_protection() {
        // Must reject Release, Release/Windows, Release/Windows/bin, Release/Windows/installer
        assert!(is_binary_or_target_dir(Path::new("Release")));
        assert!(is_binary_or_target_dir(Path::new("release")));
        assert!(is_binary_or_target_dir(Path::new("Release/Windows")));
        assert!(is_binary_or_target_dir(Path::new("Release/Windows/bin")));
        assert!(is_binary_or_target_dir(Path::new("Release/Windows/installer")));
        assert!(is_binary_or_target_dir(Path::new(r"Release\Windows\bin")));
        assert!(is_binary_or_target_dir(Path::new(r"Release\Windows\installer")));
        assert!(is_binary_or_target_dir(Path::new(r"C:\Users\VibeCode\CanbanVoimaxHarness\Release\Windows\bin")));
        assert!(is_binary_or_target_dir(Path::new(r"C:\Users\VibeCode\CanbanVoimaxHarness\Release\Windows\installer")));

        // Standard build and distribution dirs
        assert!(is_binary_or_target_dir(Path::new("target")));
        assert!(is_binary_or_target_dir(Path::new("target/debug")));
        assert!(is_binary_or_target_dir(Path::new("dist")));
        assert!(is_binary_or_target_dir(Path::new("node_modules")));

        // Valid user project paths must NOT be rejected
        assert!(!is_binary_or_target_dir(Path::new(r"C:\projects\my-cool-app")));
        assert!(!is_binary_or_target_dir(Path::new("src")));
    }

    #[test]
    fn test_is_app_own_directory_protection() {
        // Exe directory of the running test binary has no .git, so it should be blocked
        if let Ok(current_exe) = std::env::current_exe() {
            if let Some(exe_dir) = current_exe.parent() {
                assert!(is_app_own_directory(exe_dir));
            }
        }

        // Repositories named CanbanVoimaxHarness or normal user projects must NOT be blocked
        assert!(!is_app_own_directory(Path::new(r"C:\Users\VibeCode\CanbanVoimaxHarness")));
        assert!(!is_app_own_directory(Path::new("CanbanVoimaxHarness")));
        assert!(!is_app_own_directory(Path::new(r"C:\projects\web-dashboard")));

        // A directory containing .git is a source repository and must NEVER be blocked
        let temp_dir = std::env::temp_dir().join(format!("cvh_git_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let _ = std::fs::create_dir_all(temp_dir.join(".git"));
        assert!(!is_app_own_directory(&temp_dir));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_resolve_actual_repo_root_rejects_forbidden_dirs() {
        assert_eq!(resolve_actual_repo_root(Path::new("")), PathBuf::new());
        assert_eq!(resolve_actual_repo_root(Path::new("Release/Windows/bin")), PathBuf::new());
        assert_eq!(resolve_actual_repo_root(Path::new("Release/Windows/installer")), PathBuf::new());
        assert_eq!(resolve_actual_repo_root(Path::new(r"C:\Users\VibeCode\CanbanVoimaxHarness")), PathBuf::from(r"C:\Users\VibeCode\CanbanVoimaxHarness"));
        assert_eq!(resolve_actual_repo_root(Path::new(r"C:\Users\VibeCode\CanbanVoimaxHarness\Release\Windows\bin")), PathBuf::new());
    }
}


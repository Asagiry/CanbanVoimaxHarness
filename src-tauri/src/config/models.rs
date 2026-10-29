//! Data models, serialization structures, and default templates for CanbanVoimaxHarness configuration.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::discovery::{is_app_own_directory, is_binary_or_target_dir};

pub type ColumnDef = PipelineColumn;
pub type PersonaDefinition = AgentSetting;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum HookConfig {
    Script(String),
    Detailed {
        #[serde(alias = "path", alias = "command")]
        script: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
}

impl HookConfig {
    pub fn script(&self) -> &str {
        match self {
            HookConfig::Script(s) => s.as_str(),
            HookConfig::Detailed { script, .. } => script.as_str(),
        }
    }

    pub fn description(&self) -> Option<&str> {
        match self {
            HookConfig::Script(_) => None,
            HookConfig::Detailed { description, .. } => description.as_deref(),
        }
    }
}

impl From<&str> for HookConfig {
    fn from(s: &str) -> Self {
        HookConfig::Script(s.to_string())
    }
}

impl From<String> for HookConfig {
    fn from(s: String) -> Self {
        HookConfig::Script(s)
    }
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
    pub hooks: HashMap<String, HookConfig>,
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

/// `<root>/.cvh/rules` (global shared rules)
pub fn global_rules_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(".cvh").join("rules")
}

/// `<root>/.cvh/agents/<role>/hooks`
pub fn agent_hooks_dir(repo_root: &Path, role: &str) -> PathBuf {
    agent_dir(repo_root, role).join("hooks")
}

/// `<root>/.cvh/agents/<role>/setting` (JSON, no extension)
pub fn agent_setting_path(repo_root: &Path, role: &str) -> PathBuf {
    agent_dir(repo_root, role).join("setting")
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
- Ironclad Completion Rule: Before calling `advance_task`, you MUST mark all TODO items as completed via `todo_write`. After completing all planned tasks, you MUST cease tool calls and provide a terminal findings report, OR invoke `advance_task(summary)` to transition the task to the next stage. NEVER re-publish completed items in `todo_write` or enter recursive planning loops!
- Backlog-to-Ready Invariant: Always maintain `backlog` at index 0 and `ready` (`terminal: true`) at the final index across all boards.
- Board Reconfiguration Invariant: When creating, renaming, or modifying boards in `.cvh/boards.json` or `.cvh/pipeline.json`, NEVER delete the board or column where your current task is running.
"##;

pub const DEFAULT_RULE_ROLE_DEVELOPER_MD: &str = r##"---
description: Behavioral DNA and coding invariants for the Developer role, including Clean Code, Ponytail, and worktree isolation.
---
# Rule: Canonical Developer Behavioral Invariants (role-developer.md)

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

pub const DEFAULT_RULE_ROLE_REVIEWER_MD: &str = r##"---
description: Behavioral DNA and audit invariants for the Reviewer role, enforcing presumption of guilt and 3-axis verification.
---
# Rule: Canonical Reviewer Behavioral Invariants (role-reviewer.md)

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

pub const DEFAULT_RULE_ROLE_RETRO_MD: &str = r##"---
description: Behavioral DNA and post-mortem reflection protocol for the Retrospective role, enforcing engine code prohibition.
---
# Rule: Canonical Retrospective Behavioral Invariants (role-retro.md)

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

pub const DEFAULT_RULE_ROLE_PLANNER_MD: &str = r##"---
description: Behavioral DNA and planning invariants for the Planner role, enforcing read-only analysis and tactical checklist design.
---
# Rule: Canonical Planner Behavioral Invariants (role-planner.md)

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

pub const DEFAULT_RULE_BOARDS_MD: &str = r##"---
description: Multi-board layout configuration, pipeline column schemas, board presets, and routing rules in boards.json.
---
# Guide: Boards and Pipelines Configuration (boards.md)

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

pub const DEFAULT_RULE_HOOKS_MD: &str = r##"---
description: Lifecycle hooks, transition scripts, environment variable bindings, and gatekeeper sensor execution rules.
---
# Guide: Lifecycle Hooks and Transition Scripts (hooks.md)

Authoritative rules for `.cvh/hooks/` and column lifecycle scripts.

---

## 1. Hook Configuration Schema & Invocation Triggers
Columns declare transition hooks in configuration (`boards.json` and `pipeline.json`) within the `hooks` object:
- `hooks.on_enter`: Executes when a task enters the stage.
- `hooks.on_leave`: Executes when a task exits the stage.

### Hook Declaration Formats
Hooks support two schema formats:
1. **Object format (Recommended)** with `script` and `description`:
   ```json
   "hooks": {
     "on_leave": {
       "script": ".cvh/hooks/on_review_finish.bat",
       "description": "Сливает утвержденную feature-ветку в main, отправляет в origin/main и удаляет изолированный worktree"
     }
   }
   ```
   - `script` (aliases: `path`, `command`): Relative path to executable script (`.bat`, `.cmd`, `.ps1`, `.sh`, `.py`).
   - `description`: Informative human-readable explanation displayed when hovering over the Kanban card hook indicator node, clarifying hook purpose and operations to users and agents.
2. **String scalar format (Compact / Legacy)**:
   ```json
   "hooks": {
     "on_leave": ".cvh/hooks/create_worktree.bat"
   }
   ```

All hooks are executable scripts located in `.cvh/hooks/` or inside an agent's directory (`.cvh/agents/<role>/hooks/`).

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

pub const DEFAULT_RULE_PIPELINE_SETUP_MD: &str = r##"---
description: Interactive pipeline setup protocol, persona behavioral DNA infusion, and technology stack detection guidelines.
---
# Rule: Interactive Pipeline Setup & Persona Behavioral DNA (pipeline-setup.md)

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

pub const DEFAULT_RULE_BACKEND_MAP_MD: &str = r##"---
description: Authoritative navigation map and file-by-file router for the Rust Tauri backend (src-tauri/src/).
---
# Backend Navigation Map & File-by-File Router (backend_map.md)

Authoritative directory topology, file mapping table, and subsystem architecture for the Rust 2021 / Tauri v2 backend of **CanbanVoimaxHarness (`src-tauri/src/`)**.

---

## 1. Quick File-by-File Router ("What to modify" -> "Exact file path")

| What you need to build / modify | Exact File Path | Key Invariants & Ceilings |
| :--- | :--- | :--- |
| **Domain Nominal Types & IDs** | `src-tauri/src/domain/ids.rs` | `TaskId`, `StageId`, `BoardId`. Zero raw strings. |
| **Task State Machine & Transitions** | `src-tauri/src/domain/state.rs` | `TaskState` enum. Valid transitions only. |
| **Database Pool & Migrations** | `src-tauri/src/db/pool.rs` | R2D2 pool, WAL mode, foreign keys, 5s timeout. |
| **Task SQLite CRUD & Monotonic IDs** | `src-tauri/src/db/task_repo.rs` | Monotonic ID allocation, subtask cascade. |
| **Dialogue Items & Streaming Chunks** | `src-tauri/src/db/dialog_repo.rs` | Atomic upsert, tool JSON persistence. |
| **Token Usage & TPS Telemetry Repo** | `src-tauri/src/db/telemetry_repo.rs` | Aggregated tokens, non-decaying TPS metrics. |
| **Pipeline & Board Data Models** | `src-tauri/src/config/models.rs` | `BoardDef`, `PipelineConfig`, embedded rules. |
| **Two-Level Tool Policy Matrix** | `src-tauri/src/config/tool_policy.rs` | Strict AND-semantics: global && stage override. |
| **Workspace Root Discovery** | `src-tauri/src/config/discovery.rs` | Multi-tier root discovery, `%LOCALAPPDATA%`. |
| **Scaffolding & Auto-Gitignore** | `src-tauri/src/config/scaffold.rs` | Automatic scaffold, auto-gitignore policies. |
| **Atomic File I/O** | `src-tauri/src/config/io.rs` | `atomic_write_file`, JSON load/save. |
| **Task Stage Transition Pipeline** | `src-tauri/src/commands/tasks/transitions.rs` | `TaskTransitionPipeline`, hooks, gatekeeper sensors. |
| **Task Lifecycle (Create, Delete, Subtask)**| `src-tauri/src/commands/tasks/lifecycle.rs` | Backend-authoritative card lifecycle, auto-unlock. |
| **Agent Runner Trigger & Steer** | `src-tauri/src/commands/tasks/runner.rs` | `run_agent`, `steer_agent`, active task registry. |
| **Task Serialization & Storage** | `src-tauri/src/commands/tasks/storage.rs` | `load_tasks`, `sync_tasks_to_disk`. |
| **Board & Column Self-Healing** | `src-tauri/src/commands/tasks/healing.rs` | Orphan board/column resolution on load. |
| **Generic ReAct Engine Loop** | `src-tauri/src/agent/react_engine.rs` | `ReActEngine<Driver>`, unified streaming. |
| **State-Preserving Context Compactor**| `src-tauri/src/agent/compactor.rs` | `ToolCallSummaryExtractor` (CC=1), 100% ceiling. |
| **Unclosed Tool Call Sanitizer** | `src-tauri/src/agent/sanitizer.rs` | Tag stripping, unclosed JSON repairs. |
| **Non-Decaying TPS Telemetry** | `src-tauri/src/agent/telemetry.rs` | Frozen TPS on generation stop, TTFT split. |
| **5-Tier Fuzzy Text Matcher** | `src-tauri/src/agent/text_match.rs` | CRLF/LF adaptation, whitespace relaxation. |
| **LLM Tool Schemas & Definitions** | `src-tauri/src/agent/tool_schemas.rs` | JSON schemas, dynamic rule enumeration. |
| **File Operations Tools** | `src-tauri/src/agent/tools/file_ops.rs` | `read_file`, `write_file`, `edit_file`, `glob`, `grep`. |
| **Command Execution Tool** | `src-tauri/src/agent/tools/command.rs` | `execute_command` (alias `bash`), timeout, stdio. |
| **Pipeline Navigation Tools** | `src-tauri/src/agent/tools/pipeline.rs` | `advance_task`, `reject_task`, relay handoff. |
| **Planning & Checklist Tools** | `src-tauri/src/agent/tools/planning.rs` | `todo_write`, `question` (ask_user). |
| **Modular Rule Loader Tool** | `src-tauri/src/agent/tools/rules.rs` | `load_rule` (JIT dynamic rule loader). |
| **Subagent Hierarchy Tools** | `src-tauri/src/agent/tools/subagent.rs` | `subagent`, `subagent_bg`, `collect_subagent`. |
| **Task Card Tools (Level 0 Only)** | `src-tauri/src/agent/tools/tasks.rs` | `create_task`, `create_tasks`, `complete_task`. |
| **Web Research Tools** | `src-tauri/src/agent/tools/web.rs` | `web_search`, `web_fetch`. |
| **Git Worktree Manager** | `src-tauri/src/git.rs` | Worktree isolation, prune, branch teardown. |
| **Lifecycle Hooks Runner** | `src-tauri/src/hooks.rs` | Transition scripts, process tree kill on Windows. |
| **Universal LLM Client & SSE Stream** | `src-tauri/src/llm/client.rs` | OpenAI / Gemini streaming, thought extraction. |
| **LLM Configuration & Models** | `src-tauri/src/llm/models.rs` | Model overrides, endpoints, temperature. |
| **Tauri IPC Command Registration** | `src-tauri/src/lib.rs` | Plugin builder, invoke handlers, AppState. |

---

## 2. Deep Subsystem Architectures

### 2.1 Persistence Subsystem (`src-tauri/src/db/`)
- **`pool.rs`**: Implements thread-safe `r2d2::Pool<SqliteConnectionManager>` for `<repo_root>/.cvh/harness.db`. Enforces `PRAGMA journal_mode = WAL`, `PRAGMA synchronous = NORMAL`, `PRAGMA foreign_keys = ON`, and `busy_timeout = 5000ms`. Executes initial migrations for `tasks`, `dialog_items`, and `telemetry`.
- **`task_repo.rs`**: Handles task entity persistence. Manages parent-subtask cascades, monotonic ID allocation (`allocate_next_task_num`), and relational state consistency.
- **`dialog_repo.rs`**: High-concurrency dialogue storage. Atomically upserts thoughts, streaming chunks, tool execution records, and steer events into `dialog_items` table indexed by `(task_id, created_at)`.
- **`telemetry_repo.rs`**: Persists token usage, duration, and generation performance metrics. Provides aggregated stats for reporting.

### 2.2 Configuration & Scaffolding Subsystem (`src-tauri/src/config/`)
- **`models.rs`**: Canonical data contracts (`BoardDef`, `BoardsConfig`, `PipelineConfig`, `ColumnDef`, `AgentSettings`). Defines system directory resolvers (`agent_rules_dir`, `global_rules_dir`, `tool_policy_path`).
- **`tool_policy.rs`**: Two-level tool authorization engine. Enforces strict AND-semantics: `effective(tool, stage) = global_enabled[tool] && stage_override[stage][tool]`. Zero-trust stripping purges disabled tools from LLM schemas.
- **`discovery.rs`**: Four-tier workspace root discovery traversing ancestors, inspecting `.cvh` / `.git`, and persisting active projects to `%LOCALAPPDATA%/com.voimax.cvh/active_project.txt`.
- **`scaffold.rs`**: Initial project initialization (`ensure_cvh_scaffold`, `ensure_cvh_dev_scaffold`). Enforces auto-gitignore entries (`.cvh/llm.json`, `.cvh/worktrees/`) and seeds initial 3-column boards.
- **`io.rs`**: Atomic disk operations. `atomic_write_file` writes via transient temp files and renames to prevent partial corruption.

### 2.3 Task Pipeline & Command Subsystem (`src-tauri/src/commands/tasks/`)
- **`transitions.rs`**: Modular `TaskTransitionPipeline`. Executes stage movement, triggers `on_leave` and `on_enter` scripts, runs `gatekeeperSensor` commands in isolated worktrees, and routes on verdicts (`routes.on_approved`, `routes.on_changes_requested`).
- **`lifecycle.rs`**: Task creation and destruction commands. Enforces monotonic task numbers, parent-subtask waiting locks, and symmetric worktree teardown.
- **`runner.rs`**: Agent execution initiation (`run_agent`, `run_agent_loop`). Resolves active model hierarchy, prepares primed system messages, and tracks active agent runs.
- **`storage.rs`**: Task serialization and synchronization with disk ledgers and frontend subscribers via `tasks:updated`.
- **`healing.rs`**: Autonomous self-healing (`heal_task_board_and_column`, `heal_tasks`). Rescues orphan tasks assigned to non-existent boards or columns without failing.

### 2.4 Domain Layer (`src-tauri/src/domain/`)
- **`ids.rs`**: Strongly typed nominal branded types: `TaskId`, `StageId`, `BoardId`. Prevents primitive obsession and accidental string interchange.
- **`state.rs`**: `TaskState` state machine (`Backlog`, `InProgress`, `Review`, `Done`). Rejects illegal state jumps at compile and runtime.

### 2.5 Agent Execution Engine (`src-tauri/src/agent/`)
- **`react_engine.rs`**: Generic `ReActEngine<Driver>` implementing multi-turn reasoning loops, event streaming (`AgentEvent`), and tool invocation dispatch.
- **`compactor.rs`**: `ToolCallSummaryExtractor` (CC=1) extracting head/tail summaries of large tool outputs without nested branching. Enforces 100% context capacity threshold.
- **`sanitizer.rs`**: Cleans malformed LLM responses, stripping unclosed thinking tags and repairing incomplete tool invocation markers.
- **`tools/`**: Domain-partitioned tool executors conforming to command pattern: `file_ops`, `command`, `pipeline`, `planning`, `rules`, `subagent`, `tasks`, `web`.

---

## 3. Subsystem Ceilings & Architectural Invariants

1. **500-Line Subsystem Ceiling**: Every Rust source file must adhere to Single Responsibility Principle (SRP) and remain under **500 lines** (excluding comprehensive test modules). Any file expanding beyond 500 lines must be decomposed into submodules.
2. **Zero Naked Unwrap**: Never call `.unwrap()` or `.expect()` in production paths. Propagate typed errors with `Result<T, anyhow::Error>` or `thiserror`.
3. **Nominal Branded Types**: Always accept and pass `TaskId`, `StageId`, and `BoardId` across domain boundaries. Never use naked strings for entity keys.
4. **Isolated Worktrees**: All file edits and compiler checks execute inside task worktrees (`worktrees/<slug>`). Never modify the root working directory during task execution.
5. **Windows Process Hygiene**: Shell commands and hook scripts must execute with process tree isolation and explicit timeouts; on Windows, use `taskkill /F /T /PID` on teardown."##;

pub const DEFAULT_RULE_FRONTEND_MAP_MD: &str = r##"---
description: Authoritative navigation map and component router for the React 19 / Tailwind CSS v4 frontend (src/).
---
# Frontend Navigation Map & Component Router (frontend_map.md)

Authoritative directory topology, component mapping table, and modular store architecture for the React 19 / Tailwind CSS v4 frontend of **CanbanVoimaxHarness (`src/`)**.

---

## 1. Quick Component & Action Router ("UI feature / Store action" -> "Exact file path")

| UI Feature / Store Action | Exact File Path | Key Invariants & Ceilings |
| :--- | :--- | :--- |
| **Kanban Card Container & Layout** | `src/components/card/KanbanCard.tsx` | Layout coordinator, memoized with `areCardPropsEqual`. |
| **Tactile Stage Stepper Navigation** | `src/components/card/CardHookNode.tsx` | 1-click stage switching, current stage pulse, checkmarks. |
| **Card Timer & Cadence Badge** | `src/components/card/CardTimerBadge.tsx` | Execution duration, active step timer, tabular-nums. |
| **Card Context Menu & Model Override** | `src/components/card/CardContextMenu.tsx` | Context actions, model selection, subtask deletion. |
| **Card Subtask Indicator Badge** | `src/components/card/CardSubtaskBadge.tsx` | Subtask counter, status indicator, auto-unlock indicator. |
| **Card Tactical Checklist Progress** | `src/components/card/CardTodoProgress.tsx` | `todo_write` visual progress bar and count. |
| **Card Shallow Memo Comparators** | `src/components/card/cardComparators.ts` | `areCardPropsEqual` preventing cascading re-renders. |
| **Kanban Board Container & Drag-Drop** | `src/components/board/KanbanBoard.tsx` | Multi-column DND layout, column rendering. |
| **Kanban Column Container** | `src/components/board/KanbanColumn.tsx` | Stage column, auto-run emitter, column settings. |
| **Multi-Board Tab Header** | `src/components/board/BoardTabsHeader.tsx` | Multi-board tab switching, board creation modal trigger. |
| **Column Context Menu** | `src/components/board/ColumnContextMenu.tsx` | Column settings, persona assignment, auto-run toggle. |
| **Column Hook & Sensor Badges** | `src/components/board/ColumnHookBadge.tsx` | Transition script and gatekeeper sensor status. |
| **Create Task Modal** | `src/components/board/modals/CreateTaskModal.tsx` | Task creation modal with persona & model selection. |
| **Create Column Modal** | `src/components/board/modals/CreateColumnModal.tsx` | Column creation modal with hook configuration. |
| **Create Board Modal** | `src/components/board/CreateBoardModal.tsx` | Board creation modal with preset templates. |
| **Agent Console Drawer (Root Container)**| `src/components/drawer/AgentConsoleDrawer.tsx` | Hoisted fixed overlay, flex bounded (`min-h-0 min-w-0`). |
| **Drawer Header & Action Stepper** | `src/components/drawer/DrawerHeader.tsx` | Task title, tactile stage stepper, abort/run controls. |
| **Activity Stream & Comment Cards** | `src/components/drawer/DrawerActivityStream.tsx` | Step commentary vs prominent terminal response card. |
| **Steer Instruction Prompt Bar** | `src/components/drawer/DrawerSteerBar.tsx` | Interactive steer input bar, optimistic submission. |
| **Proportional Context Gauge & Popover**| `src/components/drawer/ContextGauge.tsx` | Context token gauge, popover breakdown, compact trigger. |
| **Subagents Execution Sidebar** | `src/components/drawer/SubagentsSidebar.tsx` | Subagent hierarchy, active status, subtask logs. |
| **Git Diff Viewer (Unified & Image)** | `src/components/drawer/diff/ImageDiffViewer.tsx` | Visual image diffs, side-by-side and slider modes. |
| **Git Diff File List Sidebar** | `src/components/drawer/diff/DiffFileList.tsx` | Changed files sidebar, line additions/deletions. |
| **Thought Stream Block Card** | `src/components/drawer/stream/ThoughtStreamCard.tsx` | Collapsible reasoning stream card with duration badge. |
| **Steer Directive Card** | `src/components/drawer/stream/SteerDirectiveCard.tsx` | In-stream steer instruction indicator. |
| **Subagent Activity Card** | `src/components/drawer/stream/SubagentActivityCard.tsx` | Child agent delegation summary and report view. |
| **File Operations Tool Card** | `src/components/drawer/tools/FileOpsToolCard.tsx` | `read_file`, `write_file`, `edit_file` diff display. |
| **Shell Command Tool Card** | `src/components/drawer/tools/CommandToolCard.tsx` | Command invocation, exit code, spilled logs link. |
| **Subagent Tool Card** | `src/components/drawer/tools/SubagentToolCard.tsx` | Child agent execution card with progress spinner. |
| **Context Compaction Tool Card** | `src/components/drawer/tools/CompactContextToolCard.tsx` | Optimistic compaction card with before/after stats. |
| **Cortex Explorer Tree** | `src/components/cortex/CortexExplorer.tsx` | File navigation tree for personas, rules, and hooks. |
| **Agent Persona Visual Editor** | `src/components/cortex/AgentSettingVisualEditor.tsx` | Persona prompt editor, settings, icon/color selectors. |
| **Modular Rule Editor Modal** | `src/components/cortex/RuleEditorModal.tsx` | Markdown editor for `.cvh/rules/*.md` and role rules. |
| **Two-Level Tool Policy Modal** | `src/components/settings/ToolPolicyModal.tsx` | Interactive policy matrix: global switches & overrides. |
| **Global Settings Modal** | `src/components/settings/SettingsModal.tsx` | LLM endpoints, active model, and app settings. |
| **Window Titlebar & Controls** | `src/components/layout/WindowControls.tsx` | Tauri window controls (minimize, maximize, close). |
| **Top Navigation Bar** | `src/components/layout/Topbar.tsx` | Board selector, active project, search filter. |
| **Tasks CRUD Store Slice** | `src/stores/slices/tasksCrudSlice.ts` | Backend-authoritative task CRUD, monotonic IDs. |
| **Board Meta Store Slice** | `src/stores/slices/boardMetaSlice.ts` | Active board, board switching, column layout. |
| **Drawer Meta Store Slice** | `src/stores/drawer/slices/drawerMetaSlice.ts` | `activeTaskId`, drawer open/close, view mode. |
| **Drawer Activity Store Slice** | `src/stores/drawer/slices/activitySlice.ts` | `activityFeed`, optimistic tool injection, deduplication. |
| **Drawer Stream Store Slice** | `src/stores/drawer/slices/streamSlice.ts` | Live streaming chunks, thoughts, token telemetry. |
| **Drawer Subagents Store Slice** | `src/stores/drawer/slices/subagentsSlice.ts` | Subagent lifecycle, active subtasks, subagent logs. |
| **Cortex Files Store Slice** | `src/stores/cortex/slices/cortexFilesSlice.ts` | File tree hierarchy, disk file read/write. |
| **Cortex Personas Store Slice** | `src/stores/cortex/slices/personasSlice.ts` | Persona CRUD, settings synchronization. |
| **Cortex Rules Store Slice** | `src/stores/cortex/slices/rulesSlice.ts` | Rule creation, editing, and frontmatter management. |
| **Typed Tauri Event Gateway** | `src/services/tauriEventGateway.ts` | 150ms micro-batching, typed events (zero `listen<any>`).|
| **Nominal Domain Types & Models** | `src/types/domain.ts` | `TaskId`, `StageId`, `BoardId`, `TaskState`. |

---

## 2. Modular Architecture & Data Flow

### 2.1 Card Subsystem (`src/components/card/`)
- **`KanbanCard.tsx`**: Clean, lightweight layout coordinator. Wraps components in `React.memo` with `areCardPropsEqual`. Strictly avoids wholesale subscriptions to `useBoardStore`.
- **`CardHookNode.tsx`**: Renders the tactile stage stepper on every card. Enables 1-click stage advancement, displays pulse animations for active stages, and verifies gatekeeper sensors.
- **`CardTimerBadge.tsx`**: Displays active step time and cumulative execution duration using monospace numerals (`tabular-nums`) to prevent layout shifts.
- **`CardContextMenu.tsx`**: Right-click actions for cards: model overrides, manual compaction, stage moving, subtask inspection, and deletion.

### 2.2 Console Drawer Subsystem (`src/components/drawer/`)
- **`AgentConsoleDrawer.tsx`**: Slide-out execution console. Rendered at root fragment level with `position: fixed` outside any transformed or flex-row containers to prevent clipping. Enforces `min-h-0` and `min-w-0` on scrollable flex children.
- **`DrawerActivityStream.tsx`**: Renders chronological conversation and execution events. Distinguishes intermediate step commentary (`isFinal: false`) from prominent terminal response cards (`isFinal: true`).
- **`ContextGauge.tsx`**: Real-time context window capacity meter. Resolves window capacity from persona settings (`context_window`), proportional segment allocations (System, Tools, Messages), and provides an interactive compaction popover.
- **`diff/ImageDiffViewer.tsx`**: Interactive image diffing component with side-by-side, swipe-slider, and opacity-blend modes.

### 2.3 Store Slices & State Hygiene (`src/stores/`)
- **Backend-Authoritative Lifecycle**: Zustand state mutations occur strictly after Tauri IPC backend confirmation. No optimistic card position updates or client-generated task IDs.
- **`tasksCrudSlice.ts`**: Pure CRUD operations. Auto-unlocks parent tasks when subtasks are deleted or completed.
- **`activitySlice.ts`**: Maintains `activityFeed`. Implements in-place tool deduplication by deterministic `tool_id` to eliminate card flickering upon dialogue reloads.
- **`streamSlice.ts`**: Accumulates streamed thoughts and assistant tokens. Freezes TPS immediately upon generation termination without metric decay.

### 2.4 Typed IPC Event Gateway (`src/services/tauriEventGateway.ts`)
- **Zero Raw `listen<any>`**: Direct invocations of Tauri `listen` in components or stores are strictly prohibited.
- **150ms Micro-Batching**: High-frequency streaming events (`agent:thought`, `agent:stream`) are buffered in a 150ms window before flushing to reactive stores, eliminating UI thread locking during high-speed LLM generation (>100 tok/s).
- **Strongly Typed Payloads**: Emits typed domain events (`AgentThoughtPayload`, `AgentToolCallPayload`, `TaskUpdatedPayload`).

---

## 3. Component Ceilings & UI Invariants

1. **350-Line Component Ceiling**: Every React component is strictly capped at **350 lines**. Multi-concern views must be decomposed into atomic child components or custom hooks.
2. **Selective Zustand Selectors**: Never call `useBoardStore()` or `useDrawerStore()` without selectors. Always use atomic selectors with `useShallow` to isolate re-renders.
3. **Tactile Obsidian Dark Theme**: Deep obsidian palette (`#07090e`, `#0d1117`, `#161b22`, `#30363d`). All form inputs, selects, and dialogs must declare paired dark tokens.
4. **Monospace Numerals**: Timers, token counts, TPS, and cadence indicators must enforce `tabular-nums` and monospace styling.
5. **No Native Browser Tooltips**: Never use native `title="..."` attributes on buttons or interactive controls; use custom dark popovers or self-explanatory iconography."##;


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


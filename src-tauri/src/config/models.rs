//! Data models, serialization structures, and default templates for CanbanVoimaxHarness configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use crate::config::agent_settings::*;
pub use crate::config::pipeline::*;
pub use crate::config::projects::*;
pub use crate::config::prompts::*;

pub type PipelineColumn = PipelineStage;
pub type ColumnDef = PipelineStage;
pub type StageDef = PipelineStage;
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
        "bash" | "pwsh" | "powershell" | "cmd" => "execute_command",
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
pub struct PipelineStage {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default, alias = "role")]
    pub persona: Option<String>,
    #[serde(rename = "wipLimit", alias = "wip_limit", default)]
    pub wip_limit: Option<u32>,
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
    #[serde(rename = "autoRun", default)]
    pub auto_run: Option<bool>,
    #[serde(default)]
    pub terminal: Option<bool>,
    #[serde(rename = "isIntake", default)]
    pub is_intake: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(from = "BoardDefHelper")]
pub struct BoardDef {
    pub id: String,
    pub name: String,
    pub stages: Vec<PipelineStage>,
}

#[derive(Deserialize)]
struct BoardDefHelper {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub stages: Option<Vec<PipelineStage>>,
    #[serde(default)]
    pub columns: Option<Vec<PipelineStage>>,
}

impl From<BoardDefHelper> for BoardDef {
    fn from(h: BoardDefHelper) -> Self {
        Self {
            id: h.id,
            name: h.name,
            stages: h.stages.or(h.columns).unwrap_or_default(),
        }
    }
}

impl BoardDef {
    pub fn columns(&self) -> &[PipelineStage] {
        &self.stages
    }

    pub fn columns_mut(&mut self) -> &mut Vec<PipelineStage> {
        &mut self.stages
    }
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
#[serde(from = "PipelineConfigHelper")]
pub struct PipelineConfig {
    pub id: String,
    pub title: String,
    #[serde(rename = "toolPermissions", default)]
    pub tool_permissions: Option<ToolPermissions>,
    pub stages: Vec<PipelineStage>,
    #[serde(default)]
    pub boards: Option<Vec<BoardDef>>,
    #[serde(rename = "activeBoardId", default)]
    pub active_board_id: Option<String>,
}

#[derive(Deserialize)]
struct PipelineConfigHelper {
    pub id: String,
    pub title: String,
    #[serde(rename = "toolPermissions", default)]
    pub tool_permissions: Option<ToolPermissions>,
    #[serde(default)]
    pub stages: Option<Vec<PipelineStage>>,
    #[serde(default)]
    pub columns: Option<Vec<PipelineStage>>,
    #[serde(default)]
    pub boards: Option<Vec<BoardDef>>,
    #[serde(rename = "activeBoardId", default)]
    pub active_board_id: Option<String>,
}

impl From<PipelineConfigHelper> for PipelineConfig {
    fn from(h: PipelineConfigHelper) -> Self {
        Self {
            id: h.id,
            title: h.title,
            tool_permissions: h.tool_permissions,
            stages: h.stages.or(h.columns).unwrap_or_default(),
            boards: h.boards,
            active_board_id: h.active_board_id,
        }
    }
}

impl PipelineConfig {
    pub fn columns(&self) -> &[PipelineStage] {
        &self.stages
    }

    pub fn columns_mut(&mut self) -> &mut Vec<PipelineStage> {
        &mut self.stages
    }
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
    #[serde(default)]
    pub providers: Vec<crate::config::providers::Provider>,
}

fn default_llm_provider() -> String {
    "OpenCode Go".to_string()
}

fn default_llm_api_key() -> String {
    String::new()
}

fn default_llm_base_url() -> String {
    "https://opencode.ai/inference/go/openai/v1".to_string()
}

fn default_llm_model() -> String {
    "crouton-alpha-free".to_string()
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
            providers: Vec::new(),
        }
    }
}

impl LlmSettings {
    /// Dispatch endpoint for an explicit Provider / Model pair — no
    /// "active provider" gate, no search-by-model. The chosen provider
    /// owns the endpoint; this is what prevents two providers sharing
    /// the same model id from colliding.
    pub fn endpoint_for_pair(&self, provider_id: Option<&str>, model_id: &str) -> (String, Option<String>) {
        let pid = provider_id.map(|s| s.trim()).filter(|s| !s.is_empty());
        if let Some(pid) = pid {
            if let Some(p) = self.providers.iter().find(|p| p.id.eq_ignore_ascii_case(pid)) {
                let key = p.api_key.trim().to_string();
                return (p.base_url.trim().to_string(), if key.is_empty() { None } else { Some(key) });
            }
        }
        // Legacy fallback when no provider was chosen: look up by known model
        // only as a last resort (old tasks without providerId), otherwise flat.
        let needle = model_id.trim();
        if !needle.is_empty() {
            if let Some(p) = self.providers.iter().find(|p| p.models.iter().any(|m| m.id == needle)) {
                let key = p.api_key.trim().to_string();
                return (p.base_url.trim().to_string(), if key.is_empty() { None } else { Some(key) });
            }
        }
        let key = self.api_key.trim().to_string();
        (self.base_url.trim().to_string(), if key.is_empty() { None } else { Some(key) })
    }

    /// Legacy flat endpoint. Prefer `endpoint_for_pair(provider, model)`.
    pub fn endpoint(&self) -> (String, Option<String>) {
        let key = self.api_key.trim().to_string();
        (
            self.base_url.trim().to_string(),
            if key.is_empty() { None } else { Some(key) },
        )
    }

    pub fn ensure_providers(&mut self) {
        if self.providers.is_empty() {
            self.providers.push(crate::config::providers::legacy_provider(
                &self.provider,
                &self.api_key,
                &self.base_url,
                &self.model,
            ));
        }
        self.providers = crate::config::providers::normalize_providers(std::mem::take(&mut self.providers));
        if let Some(first) = self.providers.first().cloned() {
            self.provider = first.name.clone();
            self.api_key = first.api_key.clone();
            self.base_url = first.base_url.clone();
            // Flat model is legacy only; per-model routing uses endpoint_for_model.
            // Keep it if it exists in any catalog, otherwise leave as-is.
            let in_any_catalog = self.providers.iter().any(|p| p.models.iter().any(|m| m.id == self.model));
            if !in_any_catalog {
                if let Some(any_first) = self.providers.iter().flat_map(|p| p.models.iter()).next() {
                    self.model = any_first.id.clone();
                }
            }
        }
    }
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

#[cfg(test)]
#[path = "models_tests.rs"]
mod tests;

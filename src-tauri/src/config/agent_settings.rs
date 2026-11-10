//! Agent settings models, path resolvers, and tool permissions checks.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::config::models::normalize_tool_name;

pub const DEFAULT_AGENT_SETTING_JSON: &str = r##"{
  "model": null,
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
    #[serde(default, alias = "providerId", alias = "provider_id")]
    pub provider_id: Option<String>,
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
    #[serde(default, alias = "reasoningEffort")]
    pub reasoning_effort: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
}

/// Normalize a raw `reasoningEffort` value (`low|medium|high|off`) into the
/// value sent to the LLM payload. `"off"` (and `none`/empty/unknown) means
/// "not set" and maps to `None`, so the field is omitted from the payload
/// (backwards-compatible default).
pub fn normalize_reasoning_effort(value: Option<&str>) -> Option<String> {
    match value.map(|v| v.trim().to_lowercase()).as_deref() {
        Some("low") => Some("low".to_string()),
        Some("medium") => Some("medium".to_string()),
        Some("high") => Some("high".to_string()),
        _ => None,
    }
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

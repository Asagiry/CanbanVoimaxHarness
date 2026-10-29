//! Two-level tool policy matrix, stage overrides, and tool permissions gating.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::io::atomic_write_file;
use crate::config::models::normalize_tool_name;

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


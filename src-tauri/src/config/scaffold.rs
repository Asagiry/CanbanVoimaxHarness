//! Scaffolding and initialization for .cvh project directories and system files.

use anyhow::Result;
use std::path::Path;

use crate::config::discovery::{is_app_own_directory, is_binary_or_target_dir};
use crate::config::io::{atomic_write_file, load_pipeline_config};
use crate::config::models::*;
use crate::config::tool_policy::{tool_policy_path, ToolPolicyConfig};


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

    // Global shared rules directory in .cvh/rules/
    let global_rules_dir = repo_root.join(".cvh").join("rules");
    tokio::fs::create_dir_all(&global_rules_dir).await?;

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
    let rules_dir = cvh_dir.join("rules");

    tokio::fs::create_dir_all(&cvh_dir).await?;
    tokio::fs::create_dir_all(&agents_dir).await?;
    tokio::fs::create_dir_all(&dialogs_dir).await?;
    tokio::fs::create_dir_all(&skills_dir).await?;
    tokio::fs::create_dir_all(&settings_dir).await?;
    tokio::fs::create_dir_all(&hooks_dir).await?;
    tokio::fs::create_dir_all(&rules_dir).await?;

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

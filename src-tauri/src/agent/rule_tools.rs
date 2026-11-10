use crate::llm::ToolDefinition;
use std::path::Path;

/// Extracts the 1-sentence description from YAML frontmatter (`description: ...`).
pub fn extract_frontmatter_description(content: &str) -> Option<String> {
    let trimmed = content.trim_start();
    let rest = trimmed.strip_prefix("---")?;
    let end_idx = rest.find("\n---")?;
    let fm_block = &rest[..end_idx];
    for line in fm_block.lines() {
        let trimmed_line = line.trim();
        if let Some(val) = trimmed_line.strip_prefix("description:") {
            let cleaned = val.trim().trim_matches(|c| c == '\'' || c == '"').trim();
            if !cleaned.is_empty() {
                return Some(cleaned.to_string());
            }
        }
    }
    None
}

/// Discovers available rules for an agent, collecting their stems and optional frontmatter descriptions.
/// Checks `<repo_root>/.cvh/agents/<role>/rules/*.md` and global shared `<repo_root>/.cvh/rules/*.md`.
pub fn get_available_rules_with_descriptions(
    repo_root: &Path,
    role: Option<&str>,
) -> Vec<(String, Option<String>)> {
    let mut rules: Vec<(String, Option<String>)> = Vec::new();
    let mut dirs_to_scan = Vec::new();

    if let Some(r) = role.filter(|s| !s.trim().is_empty()) {
        dirs_to_scan.push(repo_root.join(".cvh").join("agents").join(r).join("rules"));
    }
    dirs_to_scan.push(repo_root.join(".cvh").join("rules"));

    for d in &dirs_to_scan {
        if d.is_dir() {
            if let Ok(entries) = std::fs::read_dir(d) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            let desc = std::fs::read_to_string(&path)
                                .ok()
                                .and_then(|text| extract_frontmatter_description(&text));
                            rules.push((stem.to_string(), desc));
                        }
                    } else if path.is_dir() {
                        if let Ok(sub_entries) = std::fs::read_dir(&path) {
                            for sub in sub_entries.flatten() {
                                let sp = sub.path();
                                if sp.is_file() && sp.extension().and_then(|e| e.to_str()) == Some("md") {
                                    if let Some(stem) = sp.file_stem().and_then(|s| s.to_str()) {
                                        let desc = std::fs::read_to_string(&sp)
                                            .ok()
                                            .and_then(|text| extract_frontmatter_description(&text));
                                        rules.push((stem.to_string(), desc));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if rules.is_empty() {
        let agents_base = repo_root.join(".cvh").join("agents");
        if agents_base.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&agents_base) {
                for entry in entries.flatten() {
                    let r_dir = entry.path().join("rules");
                    if r_dir.is_dir() {
                        if let Ok(r_entries) = std::fs::read_dir(&r_dir) {
                            for re in r_entries.flatten() {
                                let path = re.path();
                                if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
                                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                        let desc = std::fs::read_to_string(&path)
                                            .ok()
                                            .and_then(|text| extract_frontmatter_description(&text));
                                        rules.push((stem.to_string(), desc));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Exclude 'runtime' because runtime.md is automatically primed into Turn 1 for all agents
    rules.retain(|(stem, _)| stem != "runtime");
    rules.sort_by(|a, b| a.0.cmp(&b.0));
    rules.dedup_by(|a, b| a.0 == b.0);
    rules
}

pub fn get_available_rules(repo_root: &Path, role: Option<&str>) -> Vec<String> {
    get_available_rules_with_descriptions(repo_root, role)
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

pub fn get_load_rule_tool(repo_root: &Path, role: Option<&str>) -> ToolDefinition {
    let available_rules = get_available_rules(repo_root, role);
    let rule_prop = if !available_rules.is_empty() {
        serde_json::json!({
            "type": "string",
            "description": "Modular rule topic to load dynamically.",
            "enum": available_rules
        })
    } else {
        serde_json::json!({
            "type": "string",
            "description": "Modular rule topic to load dynamically."
        })
    };

    ToolDefinition::function(
        "load_rule",
        "Dynamically load a modular project standard, architectural invariant, or tech guideline by topic.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "rule": rule_prop
            },
            "required": ["rule"]
        }),
    )
}

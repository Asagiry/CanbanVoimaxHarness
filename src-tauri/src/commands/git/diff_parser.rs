use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Task Git Diff Inspection
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffFile {
    pub path: String,
    pub status: String,
    pub additions: usize,
    pub deletions: usize,
    pub patch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskGitDiff {
    pub files: Vec<DiffFile>,
    #[serde(alias = "total_additions")]
    pub total_additions: usize,
    #[serde(alias = "total_deletions")]
    pub total_deletions: usize,
    pub branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitImageDiff {
    pub path: String,
    pub status: String,
    pub mime_type: String,
    pub current_data_url: Option<String>,
    pub previous_data_url: Option<String>,
    pub current_bytes: Option<usize>,
    pub previous_bytes: Option<usize>,
}

pub fn get_image_mime_type(path: &str) -> &'static str {
    let lower = path.to_lowercase();
    if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else if lower.ends_with(".bmp") {
        "image/bmp"
    } else if lower.ends_with(".ico") {
        "image/x-icon"
    } else if lower.ends_with(".avif") {
        "image/avif"
    } else {
        "application/octet-stream"
    }
}

/// Returns true if a path refers to a user-facing CVH configuration or rule file
/// (e.g. `.cvh/rules/**`, `.cvh/agents/**`, `.cvh/settings/**`, `.cvh/tools/**`,
/// `.cvh/hooks/**`, `.cvh/boards.json`).
/// Runtime artifacts (db, logs, spills, worktrees, dialogs, secrets) return false.
pub fn is_cvh_config_path(path: &str) -> bool {
    let raw = path.replace('\\', "/");
    let norm = raw.trim();
    if norm.is_empty() {
        return false;
    }
    let norm = norm.strip_prefix("./").unwrap_or(norm).trim_start_matches('/');

    if norm.contains("harness.db")
        || norm.starts_with("worktrees/")
        || norm.contains("/worktrees/")
        || norm.ends_with("llm.json")
        || norm.contains("/llm.json")
        || norm.starts_with(".cvh/spill/")
        || norm.contains("/.cvh/spill/")
        || norm.starts_with("spill/")
        || norm.contains("/spill/")
        || norm.starts_with(".cvh/logs/")
        || norm.contains("/.cvh/logs/")
        || norm.starts_with("logs/")
        || norm.contains("/logs/")
        || norm.ends_with(".log")
        || norm.starts_with(".cvh/dialogs/")
        || norm.contains("/.cvh/dialogs/")
        || norm.contains("/dialogs/")
        || norm.ends_with("tasks.json")
        || norm.contains("/tasks.json")
        || norm.starts_with(".cvh/tasks/")
        || norm.contains("/.cvh/tasks/")
        || norm.contains("/tasks/")
        || norm.ends_with(".secret")
        || norm.contains(".secret/")
        || norm.contains("/.secret")
        || norm.ends_with(".key")
        || norm.contains(".key/")
        || norm.contains("/.key")
    {
        return false;
    }

    norm.starts_with(".cvh/rules/")
        || norm.contains("/.cvh/rules/")
        || norm.starts_with(".cvh/agents/")
        || norm.contains("/.cvh/agents/")
        || norm.starts_with(".cvh/settings/")
        || norm.contains("/.cvh/settings/")
        || norm.starts_with(".cvh/tools/")
        || norm.contains("/.cvh/tools/")
        || norm.starts_with(".cvh/hooks/")
        || norm.contains("/.cvh/hooks/")
        || norm == ".cvh/boards.json"
        || norm.ends_with("/.cvh/boards.json")
}

/// Returns true if the file path is an internal CVH runtime, database, secret,
/// or worktree artifact that must be hidden from Git diffs.
/// Configuration files under `.cvh/` (rules, agents, settings, tools, hooks,
/// boards.json) return false so they can be tracked and reviewed.
pub fn is_cvh_path(path: &str) -> bool {
    let norm = path.replace('\\', "/");
    let norm = norm.trim();
    if norm.is_empty() {
        return false;
    }

    // 1. Always hide runtime, secret, database, and log artifacts
    if norm.contains("harness.db")
        || norm.starts_with("worktrees/")
        || norm.contains("/worktrees/")
        || norm.ends_with("llm.json")
        || norm.contains("/llm.json")
        || norm.starts_with(".cvh/spill/")
        || norm.contains("/.cvh/spill/")
        || norm.starts_with("spill/")
        || norm.contains("/spill/")
        || norm.starts_with(".cvh/logs/")
        || norm.contains("/.cvh/logs/")
        || norm.starts_with("logs/")
        || norm.contains("/logs/")
        || norm.ends_with(".log")
        || norm.starts_with(".cvh/dialogs/")
        || norm.contains("/.cvh/dialogs/")
        || norm.contains("/dialogs/")
        || norm.ends_with("tasks.json")
        || norm.contains("/tasks.json")
        || norm.starts_with(".cvh/tasks/")
        || norm.contains("/.cvh/tasks/")
        || norm.contains("/tasks/")
        || norm.ends_with(".secret")
        || norm.contains(".secret/")
        || norm.contains("/.secret")
        || norm.ends_with(".key")
        || norm.contains(".key/")
        || norm.contains("/.key")
    {
        return true;
    }

    // 2. Bare .cvh directory itself is hidden
    if norm == ".cvh" || norm == ".cvh/" || norm.ends_with("/.cvh") || norm.ends_with("/.cvh/") {
        return true;
    }

    // 3. Explicitly allowed CVH configuration files & directories:
    //    .cvh/rules/, .cvh/agents/, .cvh/settings/, .cvh/tools/, .cvh/hooks/,
    //    .cvh/boards.json
    if is_cvh_config_path(&norm) {
        return false;
    }

    // 4. Any other file inside .cvh is considered internal runtime and hidden
    if norm.starts_with(".cvh/") || norm.contains("/.cvh/") {
        return true;
    }

    // 5. Standard workspace files outside .cvh are visible
    false
}

pub fn parse_diff_blocks(diff_output: &str) -> HashMap<String, String> {
    let mut blocks = HashMap::new();
    let mut current_file: Option<String> = None;
    let mut current_patch = String::new();

    for line in diff_output.lines() {
        if line.starts_with("diff --git ") {
            if let Some(file) = current_file.take() {
                blocks.insert(file, current_patch.clone());
            }
            current_patch.clear();

            if let Some(idx) = line.rfind(" b/") {
                let target = &line[idx + 3..];
                let unquoted = target.trim_matches('"');
                if !is_cvh_path(unquoted) {
                    current_file = Some(unquoted.to_string());
                    current_patch.push_str(line);
                    current_patch.push('\n');
                } else {
                    current_file = None;
                }
            } else {
                current_file = None;
            }
        } else if current_file.is_some() {
            current_patch.push_str(line);
            current_patch.push('\n');
        }
    }

    if let Some(file) = current_file {
        blocks.insert(file, current_patch);
    }

    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_cvh_path() {
        // Bare .cvh directory
        assert!(is_cvh_path(".cvh"));
        assert!(is_cvh_path(".cvh/"));
        assert!(is_cvh_path("foo/.cvh"));

        // Runtime database, worktrees, tasks, spill, logs, secrets
        assert!(is_cvh_path(".cvh/tasks.json"));
        assert!(is_cvh_path(".cvh\\tasks.json"));
        assert!(is_cvh_path("worktrees/t-1/.cvh/tasks.json"));
        assert!(is_cvh_path("worktrees/t-1/src/main.rs"));
        assert!(is_cvh_path(".cvh/harness.db"));
        assert!(is_cvh_path(".cvh/harness.db-wal"));
        assert!(is_cvh_path(".cvh/harness.db-shm"));
        assert!(is_cvh_path(".cvh/worktrees/feat-x/file.txt"));
        assert!(is_cvh_path(".cvh/llm.json"));
        assert!(is_cvh_path("foo/.cvh/llm.json"));
        assert!(is_cvh_path(".cvh/spill/exec_output.log"));
        assert!(is_cvh_path(".cvh/logs/debug.log"));
        assert!(is_cvh_path(".cvh/dialogs/cvh-001.json"));
        assert!(is_cvh_path(".cvh/tasks/cvh-001.json"));
        assert!(is_cvh_path(".cvh/secret.key"));
        assert!(is_cvh_path(".cvh/agents/developer/.secret"));
        assert!(is_cvh_path("foo\\.cvh\\bar.txt"));

        // Allowed configuration files must NOT be hidden (!is_cvh_path)
        assert!(!is_cvh_path(".cvh/rules/guidelines.md"));
        assert!(!is_cvh_path(".cvh\\rules\\guidelines.md"));
        assert!(!is_cvh_path(".cvh/agents/developer/persona.md"));
        assert!(!is_cvh_path(".cvh/settings/tool-policy.json"));
        assert!(!is_cvh_path(".cvh/tools/custom.json"));
        assert!(!is_cvh_path(".cvh/hooks/on_enter.bat"));
        assert!(!is_cvh_path(".cvh/boards.json"));
        assert!(!is_cvh_path(".cvh\\boards.json"));
        assert!(!is_cvh_path("subproject/.cvh/boards.json"));

        // Normal codebase files outside .cvh
        assert!(!is_cvh_path("src/main.rs"));
        assert!(!is_cvh_path("something_else.cvh_file"));
        assert!(!is_cvh_path("not_dot_cvh/file.txt"));
    }

    #[test]
    fn test_parse_diff_blocks_excludes_cvh() {
        let diff_text = "\
diff --git a/src/main.rs b/src/main.rs
index 123..456 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
+hello
diff --git a/.cvh/tasks.json b/.cvh/tasks.json
index abc..def 100644
--- a/.cvh/tasks.json
+++ b/.cvh/tasks.json
@@ -1 +1 @@
-old
+new
diff --git a/.cvh/boards.json b/.cvh/boards.json
index 111..222 100644
--- a/.cvh/boards.json
+++ b/.cvh/boards.json
@@ -1 +1 @@
+{\"name\": \"test\"}
diff --git a/src/lib.rs b/src/lib.rs
index 789..012 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1 +1 @@
+pub fn test() {}
";
        let blocks = parse_diff_blocks(diff_text);
        assert!(blocks.contains_key("src/main.rs"));
        assert!(blocks.contains_key("src/lib.rs"));
        assert!(blocks.contains_key(".cvh/boards.json"));
        assert!(!blocks.contains_key(".cvh/tasks.json"));
    }

    #[test]
    fn test_get_image_mime_type() {
        assert_eq!(get_image_mime_type("assets/logo.png"), "image/png");
        assert_eq!(get_image_mime_type("img/hero.JPG"), "image/jpeg");
        assert_eq!(get_image_mime_type("photo.jpeg"), "image/jpeg");
        assert_eq!(get_image_mime_type("banner.webp"), "image/webp");
        assert_eq!(get_image_mime_type("anim.gif"), "image/gif");
        assert_eq!(get_image_mime_type("icons/icon.svg"), "image/svg+xml");
        assert_eq!(get_image_mime_type("bitmap.bmp"), "image/bmp");
        assert_eq!(get_image_mime_type("favicon.ico"), "image/x-icon");
        assert_eq!(get_image_mime_type("nextgen.avif"), "image/avif");
        assert_eq!(get_image_mime_type("data.bin"), "application/octet-stream");
    }
}

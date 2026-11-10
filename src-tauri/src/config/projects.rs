//! Project registry and entry data structures.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::config::discovery::{is_app_own_directory, is_binary_or_target_dir};

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

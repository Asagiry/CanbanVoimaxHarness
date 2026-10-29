//! Workspace root discovery, ancestor traversal, and active project persistence.

use std::path::{Path, PathBuf};

use crate::config::scaffold::ensure_project_gitignore;


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


pub fn discover_project_root_raw() -> PathBuf {
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


/// Canonical alias for discovering the active project root.
pub fn get_active_project_path() -> PathBuf {
    discover_project_root()
}

/// Canonical alias for setting and persisting the active project root.
pub fn set_active_project_path(path: &Path) {
    persist_active_project_sync(path);
}

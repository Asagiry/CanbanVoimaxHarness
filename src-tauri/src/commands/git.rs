use crate::agent::Task;
use crate::commands::state::AppState;
use crate::commands::tasks::load_tasks_from_disk;
use crate::git::{create_worktree, delete_branch, remove_worktree};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tauri::State;
use tokio::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Маркер встроенного создания worktree на старте (on_enter).
/// Значение `col.hooks["on_enter"]`, при котором бэкенд сам вызывает
/// `create_worktree` вместо запуска shell-хука.
pub const BUILTIN_CREATE_WORKTREE_MARKER: &str = "builtin:create_worktree";

/// Явный builtin-маркер в `col.hooks["on_enter"]`.
pub fn is_builtin_worktree_hook(col: &crate::config::PipelineColumn) -> bool {
    col.hooks.get("on_enter").map(|s| s.as_str()) == Some(BUILTIN_CREATE_WORKTREE_MARKER)
}

/// Нужен ли builtin на старте для колонки:
/// явный маркер ИЛИ дефолт бэклога (id == "backlog" без явного on_enter).
pub fn should_builtin_on_enter(col: &crate::config::PipelineColumn) -> bool {
    if is_builtin_worktree_hook(col) {
        return true;
    }
    col.id == "backlog" && !col.hooks.contains_key("on_enter")
}

pub fn task_has_worktree(task: &Task) -> bool {
    task.worktree_path
        .as_deref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
}

/// Парсинг stdout хука формата `CVH_WORKTREE=<path>` (+ опционально
/// `CVH_BRANCH=<branch>`): пишется в task.worktree_path (+branch если пуст).
/// Общая для on_enter и on_finish.
pub fn apply_worktree_from_hook_stdout(task: &mut Task, stdout: &str) {
    for line in stdout.lines() {
        let t = line.trim();
        if let Some(v) = t.strip_prefix("CVH_WORKTREE=") {
            let v = v.trim();
            if !v.is_empty() {
                task.worktree_path = Some(v.to_string());
            }
        } else if let Some(v) = t.strip_prefix("CVH_BRANCH=") {
            let v = v.trim();
            let branch_empty = task
                .branch
                .as_deref()
                .map(|s| s.trim().is_empty())
                .unwrap_or(true);
            if !v.is_empty() && branch_empty {
                task.branch = Some(v.to_string());
            }
        }
    }
}

/// Чистое решение «нужен ли builtin сейчас»: skip-флаг и уже имеющийся
/// worktree (в т.ч. данный shell-хуком через stdout) отключают builtin.
pub fn needs_builtin_worktree(
    col: &crate::config::PipelineColumn,
    task: &Task,
    skip_worktree: bool,
) -> bool {
    if skip_worktree {
        return false;
    }
    if task_has_worktree(task) {
        return false;
    }
    should_builtin_on_enter(col)
}

/// Best-effort builtin: `create_worktree(root, lowercase(task.id), "HEAD")`
/// + branch `feat/<slug>` если пуст. Ошибки только варнят в лог.
pub async fn ensure_builtin_worktree(root: &Path, task: &mut Task) {
    if let Some(ref p) = task.worktree_path {
        let pb = Path::new(p.trim());
        let abs = if pb.is_absolute() {
            pb.to_path_buf()
        } else {
            root.join(pb)
        };
        if abs.exists() {
            return;
        }
    }
    let slug = task.id.to_lowercase();
    let candidates = [
        root.join(".cvh").join("worktrees").join(&slug),
        root.join("worktrees").join(&slug),
    ];
    for cand in &candidates {
        if cand.exists() {
            task.worktree_path = Some(cand.to_string_lossy().to_string());
            if task.branch.as_deref().unwrap_or("").trim().is_empty() {
                task.branch = Some(format!("feat/{}", slug));
            }
            return;
        }
    }

    match create_worktree(root, &slug, "HEAD").await {
        Ok(wt_abs) => {
            task.worktree_path = Some(wt_abs.to_string_lossy().to_string());
            let branch_empty = task
                .branch
                .as_deref()
                .map(|s| s.trim().is_empty())
                .unwrap_or(true);
            if branch_empty {
                task.branch = Some(format!("feat/{}", slug));
            }
        }
        Err(e) => {
            eprintln!(
                "CVH [builtin_worktree]: creation failed for {}: {}",
                task.id, e
            );
            for cand in &candidates {
                if cand.exists() {
                    task.worktree_path = Some(cand.to_string_lossy().to_string());
                    break;
                }
            }
            if task.branch.as_deref().unwrap_or("").trim().is_empty() {
                task.branch = Some(format!("feat/{}", slug));
            }
        }
    }
}

#[tauri::command]
pub async fn cleanup_task_worktree(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let root = state.get_root();
    let task_slug = task_id.to_lowercase();
    remove_worktree(&root, &task_slug)
        .await
        .map_err(|e| format!("Failed to remove worktree for {}: {}", task_id, e))?;
    // Also drop the feature branch created for the task (best-effort: it may
    // already be merged/deleted or never have existed — never fail cleanup).
    let branch = format!("feat/{}", task_slug);
    let _ = delete_branch(&root, &branch, true).await;
    Ok(())
}

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

/// Returns true if the file path is inside or represents the `.cvh` directory.
pub fn is_cvh_path(path: &str) -> bool {
    let norm = path.replace('\\', "/");
    norm == ".cvh" || norm.starts_with(".cvh/") || norm.contains("/.cvh/") || norm.ends_with("/.cvh")
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

#[tauri::command]
pub async fn get_task_git_diff(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<TaskGitDiff, String> {
    let root = state.get_root();

    let task_opt = {
        if let Ok(guard) = state.tasks.lock() {
            guard.iter().find(|t| t.id == task_id).cloned()
        } else {
            None
        }
    };

    let task = match task_opt {
        Some(t) => t,
        None => {
            let disk_tasks = load_tasks_from_disk(&root).await;
            disk_tasks
                .into_iter()
                .find(|t| t.id == task_id)
                .ok_or_else(|| format!("Task {} not found", task_id))?
        }
    };

    let workdir = match &task.worktree_path {
        Some(p) if !p.trim().is_empty() => {
            let pb = Path::new(p.trim());
            let exists = if pb.is_absolute() {
                pb.exists()
            } else {
                root.join(pb).exists()
            };
            if exists {
                if pb.is_absolute() {
                    pb.to_path_buf()
                } else {
                    root.join(pb)
                }
            } else {
                root.clone()
            }
        }
        _ => root.clone(),
    };

    let branch = match crate::git::get_current_branch(&workdir).await {
        Ok(b) if !b.trim().is_empty() && b != "HEAD" => b.trim().to_string(),
        _ => task.branch.unwrap_or_else(|| "main".to_string()),
    };

    let mut status_cmd = Command::new("git");
    status_cmd
        .current_dir(&workdir)
        .args(["status", "--porcelain=v1", "-uall"]);
    #[cfg(windows)]
    status_cmd.creation_flags(CREATE_NO_WINDOW);
    let status_output = status_cmd
        .output()
        .await
        .map_err(|e| format!("Failed to run git status in {:?}: {}", workdir, e))?;

    if !status_output.status.success() {
        let stderr = String::from_utf8_lossy(&status_output.stderr);
        return Err(format!("git status failed in {:?}: {}", workdir, stderr));
    }

    let status_stdout = String::from_utf8_lossy(&status_output.stdout);
    let mut file_entries: Vec<(String, String)> = Vec::new();

    for line in status_stdout.lines() {
        let line = line.trim_end();
        if line.len() < 4 {
            continue;
        }
        let xy = &line[0..2];
        let raw_path = line[3..].trim();
        let target_path = if let Some((_old, new)) = raw_path.split_once(" -> ") {
            new.trim()
        } else {
            raw_path
        };
        let unquoted = target_path.trim_matches('"').to_string();
        if unquoted.is_empty() || is_cvh_path(&unquoted) {
            continue;
        }

        let status = if xy == "??" || xy.contains('A') {
            "A".to_string()
        } else if xy.contains('D') {
            "D".to_string()
        } else {
            "M".to_string()
        };

        file_entries.push((unquoted, status));
    }

    if file_entries.is_empty() {
        return Ok(TaskGitDiff {
            files: Vec::new(),
            total_additions: 0,
            total_deletions: 0,
            branch,
        });
    }

    let mut head_cmd = Command::new("git");
    head_cmd
        .current_dir(&workdir)
        .args(["rev-parse", "--verify", "HEAD"]);
    #[cfg(windows)]
    head_cmd.creation_flags(CREATE_NO_WINDOW);
    let head_exists = head_cmd
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);

    let diff_args = if head_exists {
        vec!["diff", "HEAD"]
    } else {
        vec!["diff"]
    };

    let mut diff_cmd = Command::new("git");
    diff_cmd
        .current_dir(&workdir)
        .args(&diff_args);
    #[cfg(windows)]
    diff_cmd.creation_flags(CREATE_NO_WINDOW);
    let git_diff_out = diff_cmd
        .output()
        .await;

    let mut diff_blocks: HashMap<String, String> = HashMap::new();
    if let Ok(out) = git_diff_out {
        if out.status.code().unwrap_or(1) <= 1 {
            let full_diff = String::from_utf8_lossy(&out.stdout);
            diff_blocks = parse_diff_blocks(&full_diff);
        }
    }

    let mut files: Vec<DiffFile> = Vec::new();
    let mut total_additions: usize = 0;
    let mut total_deletions: usize = 0;

    for (file_path, status) in file_entries {
        // High-performance unified patch resolution:
        // 1. If diff_blocks has the patch (from 'git diff HEAD'), use it immediately.
        // 2. If status is "A" (untracked file) and not in diff_blocks, read file directly in Rust
        //    via tokio::fs without spawning a slow 'git.exe' process (60-3000x faster).
        // 3. If deleted ("D"), generate clean synthetic deletion patch in-memory.
        let patch = if let Some(p) = diff_blocks.remove(&file_path) {
            p
        } else if status == "A" {
            let abs_file = workdir.join(&file_path);
            if let Ok(content) = tokio::fs::read_to_string(&abs_file).await {
                let line_count = content.lines().count();
                let mut p = format!(
                    "diff --git a/{0} b/{0}\nnew file mode 100644\n--- /dev/null\n+++ b/{0}\n@@ -0,0 +1,{1} @@\n",
                    file_path, line_count
                );
                for l in content.lines() {
                    p.push('+');
                    p.push_str(l);
                    p.push('\n');
                }
                p
            } else {
                format!("diff --git a/{0} b/{0}\nnew file mode 100644\nBinary file\n", file_path)
            }
        } else if status == "D" {
            format!("diff --git a/{0} b/{0}\ndeleted file\n--- a/{0}\n+++ /dev/null\n", file_path)
        } else {
            String::new()
        };

        let mut additions: usize = 0;
        let mut deletions: usize = 0;

        for line in patch.lines() {
            if line.starts_with('+') && !line.starts_with("+++") {
                additions += 1;
            } else if line.starts_with('-') && !line.starts_with("---") {
                deletions += 1;
            }
        }

        total_additions += additions;
        total_deletions += deletions;

        files.push(DiffFile {
            path: file_path,
            status,
            additions,
            deletions,
            patch,
        });
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(TaskGitDiff {
        files,
        total_additions,
        total_deletions,
        branch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_cvh_path() {
        assert!(is_cvh_path(".cvh"));
        assert!(is_cvh_path(".cvh/"));
        assert!(is_cvh_path(".cvh/tasks.json"));
        assert!(is_cvh_path(".cvh\\tasks.json"));
        assert!(is_cvh_path("worktrees/t-1/.cvh/tasks.json"));
        assert!(is_cvh_path("foo\\.cvh\\bar.txt"));
        assert!(is_cvh_path("foo/.cvh"));
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
        assert!(!blocks.contains_key(".cvh/tasks.json"));
    }
}

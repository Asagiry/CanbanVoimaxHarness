use crate::agent::Task;
use crate::commands::state::AppState;
use crate::commands::tasks::load_tasks_from_disk;
use std::path::{Path, PathBuf};

pub fn task_has_worktree(task: &Task) -> bool {
    task.worktree_path
        .as_deref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
}

/// Парсинг stdout хука формата `CVH_WORKTREE=<path>` (+ опционально
/// `CVH_BRANCH=<branch>`): пишется в task.worktree_path (+branch если пуст).
/// Используется для on_enter и on_leave.
pub fn apply_worktree_from_hook_stdout(task: &mut Task, stdout: &str) {
    for line in stdout.lines() {
        let t = line.trim();
        if let Some(v) = t.strip_prefix("CVH_WORKTREE=") {
            let v = v.trim();
            if !v.is_empty() {
                task.worktree_path = Some(v.replace('\\', "/"));
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

/// Hierarchically resolves the worktree directory and git branch for a given task or subtask.
///
/// Priority for workdir resolution:
/// 1. `task.worktree_path` (if valid and exists on disk)
/// 2. `parent_task.worktree_path` (if parent exists and path exists on disk)
/// 3. Filesystem heuristics:
///    - `root.join("worktrees").join(&task.id)`
///    - `root.join("worktrees").join(task.id.to_lowercase())`
///    - `root.join("worktrees").join(&parent_id)`
///    - `root.join("worktrees").join(parent_id.to_lowercase())`
///    - `root.join(".cvh/worktrees").join(...)`
/// 4. Fallback to `root` (main repository) if no worktree exists.
///
/// Auto-heals `worktree_path` and `branch` in task/parent_task and persists to SQLite/state.
pub async fn resolve_task_worktree(
    root: &Path,
    task_id: &str,
    state: Option<&AppState>,
) -> Result<(Task, PathBuf, String), String> {
    let mut task_opt = None;
    if let Some(st) = state {
        if let Ok(guard) = st.tasks.lock() {
            task_opt = guard
                .iter()
                .find(|t| crate::commands::task_id_matches(&t.id, task_id))
                .cloned();
        }
    }

    let mut task = match task_opt {
        Some(t) => t,
        None => {
            let disk_tasks = load_tasks_from_disk(root).await;
            match disk_tasks
                .into_iter()
                .find(|t| crate::commands::task_id_matches(&t.id, task_id))
            {
                Some(t) => t,
                None => {
                    crate::logger::cvh_log(
                        Some(root),
                        "WARN",
                        "GIT_DIFF",
                        &format!(
                            "resolve_task_worktree: task '{}' not found in memory or disk",
                            task_id
                        ),
                    );
                    return Err(format!("Task {} not found", task_id));
                }
            }
        }
    };

    let parent_id_candidate: Option<String> = task.parent_id.clone().or_else(|| {
        if task_id.contains('.') {
            task_id.split_once('.').map(|(p, _)| p.to_string())
        } else {
            None
        }
    });

    let mut parent_task: Option<Task> = None;
    if let Some(ref pid) = parent_id_candidate {
        if !crate::commands::task_id_matches(pid, &task.id) {
            if let Some(st) = state {
                if let Ok(guard) = st.tasks.lock() {
                    parent_task = guard
                        .iter()
                        .find(|t| crate::commands::task_id_matches(&t.id, pid))
                        .cloned();
                }
            }
            if parent_task.is_none() {
                let disk_tasks = load_tasks_from_disk(root).await;
                parent_task = disk_tasks
                    .into_iter()
                    .find(|t| crate::commands::task_id_matches(&t.id, pid));
            }
        }
    }

    let check_path_exists = |p_str: &str| -> Option<PathBuf> {
        let trimmed = p_str.trim();
        if trimmed.is_empty() {
            return None;
        }
        let pb = Path::new(trimmed);
        if pb.is_absolute() {
            if pb.exists() {
                Some(pb.to_path_buf())
            } else {
                None
            }
        } else {
            let joined = root.join(pb);
            if joined.exists() {
                Some(joined)
            } else {
                None
            }
        }
    };

    let mut resolved_workdir: Option<PathBuf> = None;
    let mut auto_healed_wt_str: Option<String> = None;

    // Priority 1: Task's own worktree_path
    if let Some(ref wt) = task.worktree_path {
        resolved_workdir = check_path_exists(wt);
        if resolved_workdir.is_some() {
            auto_healed_wt_str = Some(wt.clone());
        }
    }

    // Priority 2: Parent task's worktree_path
    if resolved_workdir.is_none() {
        if let Some(ref pt) = parent_task {
            if let Some(ref pwt) = pt.worktree_path {
                if let Some(found_path) = check_path_exists(pwt) {
                    resolved_workdir = Some(found_path);
                    auto_healed_wt_str = Some(pwt.clone());
                }
            }
        }
    }

    // Priority 3: Filesystem heuristics
    if resolved_workdir.is_none() {
        let mut candidates = Vec::new();
        candidates.push(format!("worktrees/{}", task.id));
        candidates.push(format!("worktrees/{}", task.id.to_lowercase()));
        if let Some(ref pid) = parent_id_candidate {
            candidates.push(format!("worktrees/{}", pid));
            candidates.push(format!("worktrees/{}", pid.to_lowercase()));
        }
        candidates.push(format!(".cvh/worktrees/{}", task.id));
        candidates.push(format!(".cvh/worktrees/{}", task.id.to_lowercase()));
        if let Some(ref pid) = parent_id_candidate {
            candidates.push(format!(".cvh/worktrees/{}", pid));
            candidates.push(format!(".cvh/worktrees/{}", pid.to_lowercase()));
        }

        for rel in candidates {
            if let Some(found) = check_path_exists(&rel) {
                resolved_workdir = Some(found);
                auto_healed_wt_str = Some(rel);
                break;
            }
        }
    }

    let (workdir, is_worktree) = match resolved_workdir {
        Some(wd) => (wd, true),
        None => (root.to_path_buf(), false),
    };

    let mut task_dirty = false;
    if is_worktree {
        if task
            .worktree_path
            .as_deref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            let wt_value = auto_healed_wt_str.clone().unwrap_or_else(|| {
                if let Ok(rel) = workdir.strip_prefix(root) {
                    rel.to_string_lossy().replace('\\', "/")
                } else {
                    workdir.to_string_lossy().replace('\\', "/")
                }
            });
            task.worktree_path = Some(wt_value);
            task_dirty = true;
        }

        if task.parent_id.is_none() && parent_id_candidate.is_some() {
            task.parent_id = parent_id_candidate.clone();
            task_dirty = true;
        }

        if task
            .branch
            .as_deref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            let inherited_branch = parent_task
                .as_ref()
                .and_then(|p| p.branch.clone())
                .or_else(|| {
                    let slug = parent_id_candidate.as_deref().unwrap_or(&task.id);
                    Some(format!("feat/{}", slug))
                });
            if inherited_branch.is_some() {
                task.branch = inherited_branch;
                task_dirty = true;
            }
        }
    }

    if task_dirty {
        let _ = crate::db::db_upsert_task(root, &task);
        if let Some(st) = state {
            if let Ok(mut guard) = st.tasks.lock() {
                if let Some(t) = guard
                    .iter_mut()
                    .find(|t| crate::commands::task_id_matches(&t.id, &task.id))
                {
                    t.worktree_path = task.worktree_path.clone();
                    t.branch = task.branch.clone();
                    t.parent_id = task.parent_id.clone();
                }
            }
        }
    }

    // Auto-heal parent task if it was missing worktree_path
    if is_worktree {
        if let Some(mut pt) = parent_task.clone() {
            if pt
                .worktree_path
                .as_deref()
                .map(|s| s.trim().is_empty())
                .unwrap_or(true)
            {
                if let Some(ref rel_wt) = auto_healed_wt_str {
                    pt.worktree_path = Some(rel_wt.clone());
                    if pt
                        .branch
                        .as_deref()
                        .map(|s| s.trim().is_empty())
                        .unwrap_or(true)
                    {
                        pt.branch = Some(format!("feat/{}", pt.id));
                    }
                    let _ = crate::db::db_upsert_task(root, &pt);
                    if let Some(st) = state {
                        if let Ok(mut guard) = st.tasks.lock() {
                            if let Some(t) = guard
                                .iter_mut()
                                .find(|t| crate::commands::task_id_matches(&t.id, &pt.id))
                            {
                                t.worktree_path = pt.worktree_path.clone();
                                t.branch = pt.branch.clone();
                            }
                        }
                    }
                }
            }
        }
    }

    let branch = if is_worktree {
        if workdir.join(".git").exists() {
            match crate::git::get_current_branch(&workdir).await {
                Ok(b) if !b.trim().is_empty() && b != "HEAD" => b.trim().to_string(),
                _ => task.branch.clone().unwrap_or_else(|| {
                    let slug = parent_id_candidate.as_deref().unwrap_or(&task.id);
                    format!("feat/{}", slug)
                }),
            }
        } else {
            task.branch.clone().unwrap_or_else(|| {
                let slug = parent_id_candidate.as_deref().unwrap_or(&task.id);
                format!("feat/{}", slug)
            })
        }
    } else {
        match crate::git::get_current_branch(&workdir).await {
            Ok(b) if !b.trim().is_empty() && b != "HEAD" => b.trim().to_string(),
            _ => task.branch.clone().unwrap_or_else(|| "main".to_string()),
        }
    };

    Ok((task, workdir, branch))
}

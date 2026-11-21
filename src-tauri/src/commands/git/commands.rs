use super::diff_parser::*;
use super::worktree::*;
use crate::commands::state::AppState;
use std::collections::HashMap;
use tauri::State;
use tokio::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

async fn run_git_stdout(workdir: &std::path::Path, args: &[&str]) -> Option<String> {
    let mut cmd = Command::new("git");
    cmd.current_dir(workdir).args(args);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let out = cmd.output().await.ok()?;
    if out.status.success() {
        Some(crate::agent::decode_output_bytes(&out.stdout).trim().to_string())
    } else {
        None
    }
}

/// Resolves the merge-base commit hash between the task's base branch (or repo default branch)
/// and `HEAD`, so committed branch changes are included in `get_task_git_diff`.
async fn resolve_diff_merge_base(
    workdir: &std::path::Path,
    task: Option<&crate::agent::Task>,
    current_branch: Option<&str>,
) -> Option<String> {
    let mut candidates: Vec<String> = Vec::new();
    if let Some(base) = task.and_then(|t| t.base_branch.as_deref()).map(str::trim).filter(|s| !s.is_empty()) {
        candidates.push(base.to_string());
    }
    for def in ["main", "master", "production", "develop"] {
        if !candidates.iter().any(|c| c == def) {
            candidates.push(def.to_string());
        }
    }

    let head_sha = run_git_stdout(workdir, &["rev-parse", "--verify", "HEAD"]).await?;
    for cand in candidates {
        if current_branch.is_some_and(|b| b == cand || format!("origin/{}", b) == cand) {
            continue;
        }
        if let Some(mb) = run_git_stdout(workdir, &["merge-base", &cand, "HEAD"]).await {
            if !mb.is_empty() && mb != head_sha {
                return Some(mb);
            }
        }
    }
    None
}

#[tauri::command]
pub async fn get_task_git_diff(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<TaskGitDiff, String> {
    let root = state.get_root();
    crate::logger::cvh_log(
        Some(&root),
        "DEBUG",
        "GIT_DIFF",
        &format!("get_task_git_diff called for task_id='{}', root='{}'", task_id, root.display()),
    );

    let (task_item, workdir, branch) = resolve_task_worktree(&root, &task_id, Some(&state)).await?;
    let merge_base = resolve_diff_merge_base(&workdir, Some(&task_item), Some(&branch)).await;

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
        let stderr = crate::agent::decode_output_bytes(&status_output.stderr);
        return Err(format!("git status failed in {:?}: {}", workdir, stderr));
    }

    let status_stdout = crate::agent::decode_output_bytes(&status_output.stdout);
    let mut file_entries_map: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();

    if let Some(ref mb) = merge_base {
        if let Some(ns_stdout) = run_git_stdout(&workdir, &["diff", "--name-status", mb]).await {
            for line in ns_stdout.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() < 2 {
                    continue;
                }
                let st_code = parts[0].trim();
                let raw_path = parts.last().copied().unwrap_or("").trim().trim_matches('"');
                if raw_path.is_empty() || is_cvh_path(raw_path) {
                    continue;
                }
                let status = if st_code.starts_with('A') {
                    "A"
                } else if st_code.starts_with('D') {
                    "D"
                } else {
                    "M"
                };
                file_entries_map
                    .entry(raw_path.to_string())
                    .or_insert_with(|| status.to_string());
            }
        }
    }

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
        if merge_base.is_some() && xy != "??" {
            continue;
        }

        let status = if xy == "??" || xy.contains('A') {
            "A".to_string()
        } else if xy.contains('D') {
            "D".to_string()
        } else {
            "M".to_string()
        };

        // Guard against phantom deletion: if file physically exists on disk, it cannot be "D"
        if status == "D" && workdir.join(&unquoted).exists() {
            continue;
        }

        file_entries_map
            .entry(unquoted)
            .and_modify(|existing| {
                if *existing == "D" {
                    *existing = status.clone();
                }
            })
            .or_insert(status);
    }

    if file_entries_map.is_empty() {
        return Ok(TaskGitDiff {
            files: Vec::new(),
            total_additions: 0,
            total_deletions: 0,
            branch,
        });
    }

    let head_exists = run_git_stdout(&workdir, &["rev-parse", "--verify", "HEAD"]).await.is_some();

    let diff_args: Vec<&str> = if let Some(ref mb) = merge_base {
        vec!["diff", mb.as_str()]
    } else if head_exists {
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
            let full_diff = crate::agent::decode_output_bytes(&out.stdout);
            diff_blocks = parse_diff_blocks(&full_diff);
        }
    }

    let mut files: Vec<DiffFile> = Vec::new();
    let mut total_additions: usize = 0;
    let mut total_deletions: usize = 0;

    for (file_path, status) in file_entries_map {
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
            if workdir.join(&file_path).exists() {
                continue;
            }
            format!("diff --git a/{0} b/{0}\ndeleted file\n--- a/{0}\n+++ /dev/null\n", file_path)
        } else {
            String::new()
        };

        // Filter out phantom "modified" files (e.g. Windows CRLF vs LF stat mismatch or dirty mtime)
        // where git diff proves there are zero textual or binary changes.
        if status == "M" && patch.trim().is_empty() {
            continue;
        }

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

    crate::logger::cvh_log(
        Some(&root),
        "DEBUG",
        "GIT_DIFF",
        &format!("get_task_git_diff: successfully generated diff for task '{}' ({} files, branch: '{}')", task_id, files.len(), branch),
    );

    Ok(TaskGitDiff {
        files,
        total_additions,
        total_deletions,
        branch,
    })
}

#[tauri::command]
pub async fn get_task_image_diff(
    task_id: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<GitImageDiff, String> {
    let root = state.get_root();
    let (_task, workdir, _branch) = resolve_task_worktree(&root, &task_id, Some(&state)).await?;

    let clean_path = path.replace('\\', "/");
    let clean_path = clean_path
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string();

    let mime_type = get_image_mime_type(&clean_path).to_string();

    let abs_file = workdir.join(&clean_path);
    let mut current_bytes = None;
    let mut current_data_url = None;

    if abs_file.exists() {
        if let Ok(bytes) = tokio::fs::read(&abs_file).await {
            use base64::Engine;
            current_bytes = Some(bytes.len());
            current_data_url = Some(format!(
                "data:{};base64,{}",
                mime_type,
                base64::engine::general_purpose::STANDARD.encode(&bytes)
            ));
        }
    }

    let mut previous_bytes = None;
    let mut previous_data_url = None;

    let mut show_cmd = Command::new("git");
    show_cmd
        .current_dir(&workdir)
        .args(["show", &format!("HEAD:{}", clean_path)]);
    #[cfg(windows)]
    show_cmd.creation_flags(CREATE_NO_WINDOW);

    if let Ok(output) = show_cmd.output().await {
        if output.status.success() && !output.stdout.is_empty() {
            use base64::Engine;
            previous_bytes = Some(output.stdout.len());
            previous_data_url = Some(format!(
                "data:{};base64,{}",
                mime_type,
                base64::engine::general_purpose::STANDARD.encode(&output.stdout)
            ));
        }
    }

    let status = match (current_data_url.is_some(), previous_data_url.is_some()) {
        (true, true) => {
            if current_data_url == previous_data_url {
                "U".to_string()
            } else {
                "M".to_string()
            }
        }
        (true, false) => "A".to_string(),
        (false, true) => "D".to_string(),
        (false, false) => "?".to_string(),
    };

    crate::logger::cvh_log(
        Some(&root),
        "DEBUG",
        "GIT_DIFF",
        &format!(
            "get_task_image_diff: task='{}', path='{}', status='{}', curr_bytes={:?}, prev_bytes={:?}",
            task_id, clean_path, status, current_bytes, previous_bytes
        ),
    );

    Ok(GitImageDiff {
        path,
        status,
        mime_type,
        current_data_url,
        previous_data_url,
        current_bytes,
        previous_bytes,
    })
}

/// Lists local and origin remote git branches for a repository root.
/// Returns an empty list if the directory is not a git repo or git fails.
pub async fn list_git_branches_in(root: &std::path::Path) -> Vec<String> {
    let mut cmd = Command::new("git");
    cmd.current_dir(root)
        .args(["for-each-ref", "--format=%(refname:short)", "refs/heads", "refs/remotes/origin"]);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let Ok(output) = cmd.output().await else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    let stdout = crate::agent::decode_output_bytes(&output.stdout);
    let mut seen = std::collections::HashSet::new();
    let mut branches = Vec::new();
    for line in stdout.lines() {
        let b = line.trim();
        if b.is_empty() || b == "origin" || b == "origin/HEAD" {
            continue;
        }
        if seen.insert(b.to_string()) {
            branches.push(b.to_string());
        }
    }
    branches
}

#[tauri::command]
pub async fn list_git_branches(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let root = state.get_root();
    Ok(list_git_branches_in(&root).await)
}

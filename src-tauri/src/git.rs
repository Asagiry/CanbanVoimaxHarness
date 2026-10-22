use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorktreeInfo {
    pub path: String,
    pub head: String,
    pub branch: Option<String>,
}

async fn run_git(repo_root: &Path, args: &[&str]) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_root).args(args);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let output = cmd
        .output()
        .await
        .with_context(|| format!("Failed to execute 'git {}' in {:?}", args.join(" "), repo_root))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !output.status.success() {
        bail!(
            "Git command failed: 'git {}'\nExit status: {:?}\nStderr: {}\nStdout: {}",
            args.join(" "),
            output.status.code(),
            stderr,
            stdout
        );
    }

    Ok(stdout)
}

/// Creates an isolated git worktree at `worktrees/<task_slug>` on branch `feat/<task_slug>`.
/// If branch already exists, checks it out without `-b`.
pub async fn create_worktree(
    repo_root: impl AsRef<Path>,
    task_slug: &str,
    base_branch: &str,
) -> Result<PathBuf> {
    let repo_path = repo_root.as_ref();
    let worktree_rel = PathBuf::from(".cvh").join("worktrees").join(task_slug);
    let worktree_abs = repo_path.join(&worktree_rel);
    let branch_name = format!("feat/{}", task_slug);

    if worktree_abs.exists() {
        return Ok(worktree_abs);
    }

    let alt_worktree_rel = PathBuf::from("worktrees").join(task_slug);
    let alt_worktree_abs = repo_path.join(&alt_worktree_rel);
    if alt_worktree_abs.exists() {
        return Ok(alt_worktree_abs);
    }

    // Check if worktree or branch is already registered in git
    if let Ok(existing_wts) = list_worktrees(repo_path).await {
        for wt in existing_wts {
            if wt.branch.as_deref() == Some(&branch_name) || wt.path.ends_with(task_slug) {
                let p = PathBuf::from(&wt.path);
                if p.exists() {
                    return Ok(p);
                }
            }
        }
    }

    // Check if branch already exists
    let mut branch_cmd = Command::new("git");
    branch_cmd
        .current_dir(repo_path)
        .args(["rev-parse", "--verify", &branch_name]);
    #[cfg(windows)]
    branch_cmd.creation_flags(CREATE_NO_WINDOW);
    let branch_check = branch_cmd.output().await;

    let branch_exists = match branch_check {
        Ok(out) => out.status.success(),
        Err(_) => false,
    };

    let worktree_str = worktree_rel.to_string_lossy().to_string();

    if branch_exists {
        let add_res = run_git(
            repo_path,
            &["worktree", "add", &worktree_str, &branch_name],
        )
        .await;

        if let Err(e) = add_res {
            if worktree_abs.exists() {
                sync_cvh_runtime_state(repo_path, &worktree_abs).await;
                return Ok(worktree_abs);
            }
            if alt_worktree_abs.exists() {
                sync_cvh_runtime_state(repo_path, &alt_worktree_abs).await;
                return Ok(alt_worktree_abs);
            }
            if let Ok(existing_wts) = list_worktrees(repo_path).await {
                for wt in existing_wts {
                    if wt.branch.as_deref() == Some(&branch_name) || wt.path.ends_with(task_slug) {
                        let p = PathBuf::from(&wt.path);
                        if p.exists() {
                            sync_cvh_runtime_state(repo_path, &p).await;
                            return Ok(p);
                        }
                    }
                }
            }
            return Err(e).with_context(|| format!("Failed to attach existing branch '{}' to worktree '{}'", branch_name, worktree_str));
        }
    } else {
        // Verify base branch exists; if not, fallback to "HEAD" so it works on any repo/branch
        let mut base_cmd = Command::new("git");
        base_cmd
            .current_dir(repo_path)
            .args(["rev-parse", "--verify", base_branch]);
        #[cfg(windows)]
        base_cmd.creation_flags(CREATE_NO_WINDOW);
        let base_check = base_cmd.output().await;
        let effective_base = match base_check {
            Ok(out) if out.status.success() => base_branch,
            _ => "HEAD",
        };

        let add_res = run_git(
            repo_path,
            &["worktree", "add", "-b", &branch_name, &worktree_str, effective_base],
        )
        .await;

        if let Err(e) = add_res {
            if worktree_abs.exists() {
                sync_cvh_runtime_state(repo_path, &worktree_abs).await;
                return Ok(worktree_abs);
            }
            if alt_worktree_abs.exists() {
                sync_cvh_runtime_state(repo_path, &alt_worktree_abs).await;
                return Ok(alt_worktree_abs);
            }
            if let Ok(existing_wts) = list_worktrees(repo_path).await {
                for wt in existing_wts {
                    if wt.branch.as_deref() == Some(&branch_name) || wt.path.ends_with(task_slug) {
                        let p = PathBuf::from(&wt.path);
                        if p.exists() {
                            sync_cvh_runtime_state(repo_path, &p).await;
                            return Ok(p);
                        }
                    }
                }
            }
            return Err(e).with_context(|| format!("Failed to create worktree '{}' from branch '{}'", worktree_str, effective_base));
        }
    }

    // The fresh worktree carries the COMMITTED .cvh snapshot, which is stale by
    // definition. Overwrite it with the live runtime-state from <root>/.cvh
    // (best-effort: sync errors only warn, never fail creation).
    sync_cvh_runtime_state(repo_path, &worktree_abs).await;

    Ok(worktree_abs)
}

/// Returns true for secret file names that must never be synced into a worktree:
/// `llm.json`, `*.secret`, `*.key`, `.env*`.
fn is_secret_file_name(name: &str) -> bool {
    name == "llm.json"
        || name.ends_with(".secret")
        || name.ends_with(".key")
        || name.starts_with(".env")
}

/// Best-effort overwrite-copy of a single file. Creates parent dirs as needed.
/// Never fails the caller: errors become `eprintln!` warnings.
async fn copy_file_overwrite(src: &Path, dst: &Path) {
    if let Some(parent) = dst.parent() {
        if let Err(e) = tokio::fs::create_dir_all(parent).await {
            eprintln!(
                "CVH [sync_cvh_runtime_state]: warning: cannot create dir {:?}: {}",
                parent, e
            );
            return;
        }
    }
    if let Err(e) = tokio::fs::copy(src, dst).await {
        eprintln!(
            "CVH [sync_cvh_runtime_state]: warning: cannot copy {:?} -> {:?}: {}",
            src, dst, e
        );
    }
}

/// Copies top-level files of `src_dir` into `dst_dir` when `keep` accepts the
/// file name. Secret files are always skipped. Subdirectories are ignored.
async fn copy_top_level_filtered(src_dir: &Path, dst_dir: &Path, keep: impl Fn(&str) -> bool) {
    if !tokio::fs::try_exists(src_dir).await.unwrap_or(false) {
        return;
    }
    if let Err(e) = tokio::fs::create_dir_all(dst_dir).await {
        eprintln!(
            "CVH [sync_cvh_runtime_state]: warning: cannot create dir {:?}: {}",
            dst_dir, e
        );
        return;
    }
    let mut entries = match tokio::fs::read_dir(src_dir).await {
        Ok(e) => e,
        Err(e) => {
            eprintln!(
                "CVH [sync_cvh_runtime_state]: warning: cannot list {:?}: {}",
                src_dir, e
            );
            return;
        }
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let file_type = match entry.file_type().await {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if !file_type.is_file() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if is_secret_file_name(&name) || !keep(&name) {
            continue;
        }
        copy_file_overwrite(&path, &dst_dir.join(&name)).await;
    }
}

/// Recursively copies `src_dir` into `dst_dir` (overwrite). Secret files are
/// skipped at every level. Errors are warnings only.
async fn copy_dir_recursive_filtered(src_dir: &Path, dst_dir: &Path) {
    if !tokio::fs::try_exists(src_dir).await.unwrap_or(false) {
        return;
    }
    let mut stack = vec![(src_dir.to_path_buf(), dst_dir.to_path_buf())];
    while let Some((current_src, current_dst)) = stack.pop() {
        if let Err(e) = tokio::fs::create_dir_all(&current_dst).await {
            eprintln!(
                "CVH [sync_cvh_runtime_state]: warning: cannot create dir {:?}: {}",
                current_dst, e
            );
            continue;
        }
        let mut entries = match tokio::fs::read_dir(&current_src).await {
            Ok(e) => e,
            Err(e) => {
                eprintln!(
                    "CVH [sync_cvh_runtime_state]: warning: cannot list {:?}: {}",
                    current_src, e
                );
                continue;
            }
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            let file_type = match entry.file_type().await {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            if file_type.is_dir() {
                stack.push((path, current_dst.join(&name)));
            } else if file_type.is_file() {
                if is_secret_file_name(&name) {
                    continue;
                }
                copy_file_overwrite(&path, &current_dst.join(&name)).await;
            }
        }
    }
}

/// Synchronizes the live runtime-state from `<root>/.cvh` into the fresh
/// worktree's `<worktree>/.cvh`, overwriting the stale committed snapshot.
///
/// Copies (if present in root): `pipeline.json`, `boards.json`,
/// `settings/tool-policy.json`, `agents/*.md`, `tools/*.json`, `hooks/` and
/// `skills/` (recursively, minus secrets).
///
/// Never copies: `worktrees/` (nesting!), `dialogs/`, `tasks/<id>/`, `tasks.json`,
/// `spill/`, secrets (`llm.json`, `*.secret`, `*.key`, `.env*`).
///
/// Best-effort: all errors are `eprintln!` warnings, never propagated.
pub async fn sync_cvh_runtime_state(repo_root: &Path, worktree_abs: &Path) {
    let src_cvh = repo_root.join(".cvh");
    if !tokio::fs::try_exists(&src_cvh).await.unwrap_or(false) {
        return;
    }
    let dst_cvh = worktree_abs.join(".cvh");
    if let Err(e) = tokio::fs::create_dir_all(&dst_cvh).await {
        eprintln!(
            "CVH [sync_cvh_runtime_state]: warning: cannot create {:?}: {}",
            dst_cvh, e
        );
        return;
    }

    // Single files (overwrite stale snapshot when root has a live copy).
    for rel in ["pipeline.json", "boards.json"] {
        let src = src_cvh.join(rel);
        if tokio::fs::try_exists(&src).await.unwrap_or(false) {
            if let Some(name) = src.file_name().and_then(|n| n.to_str()) {
                if is_secret_file_name(name) {
                    continue;
                }
            }
            copy_file_overwrite(&src, &dst_cvh.join(rel)).await;
        }
    }

    // Worktrees must be clean and never contain or sync tasks.json.
    let dst_tasks = dst_cvh.join("tasks.json");
    if tokio::fs::try_exists(&dst_tasks).await.unwrap_or(false) {
        let _ = tokio::fs::remove_file(&dst_tasks).await;
    }
    // Canonical Tool Policy Matrix only (not the whole settings/ dir).
    let src_policy = src_cvh.join("settings").join("tool-policy.json");
    if tokio::fs::try_exists(&src_policy).await.unwrap_or(false) {
        copy_file_overwrite(
            &src_policy,
            &dst_cvh.join("settings").join("tool-policy.json"),
        )
        .await;
    }

    // Flat filtered copies.
    copy_top_level_filtered(&src_cvh.join("agents"), &dst_cvh.join("agents"), |n| {
        n.ends_with(".md")
    })
    .await;
    copy_top_level_filtered(&src_cvh.join("tools"), &dst_cvh.join("tools"), |n| {
        n.ends_with(".json")
    })
    .await;

    // Whole-directory copies (minus secrets).
    copy_dir_recursive_filtered(&src_cvh.join("hooks"), &dst_cvh.join("hooks")).await;
    copy_dir_recursive_filtered(&src_cvh.join("skills"), &dst_cvh.join("skills")).await;
    // Deliberately NOT synced: worktrees/, dialogs/, tasks/<id>/, spill/,
    // llm.json and other secrets.
}

/// Removes a worktree forcefully and prunes git worktree metadata.
pub async fn remove_worktree(
    repo_root: impl AsRef<Path>,
    task_slug: &str,
) -> Result<()> {
    let repo_path = repo_root.as_ref();
    let worktree_rel = PathBuf::from(".cvh").join("worktrees").join(task_slug);
    let worktree_str = worktree_rel.to_string_lossy().to_string();

    // Run git worktree remove --force
    let _ = run_git(
        repo_path,
        &["worktree", "remove", "--force", &worktree_str],
    )
    .await;

    // Prune worktree records
    let _ = run_git(repo_path, &["worktree", "prune"]).await;

    // Ensure physical folder is cleaned up if lingering
    let worktree_abs = repo_path.join(&worktree_rel);
    if worktree_abs.exists() {
        let _ = tokio::fs::remove_dir_all(&worktree_abs).await;
    }

    Ok(())
}

/// Prunes git worktree administrative records.
pub async fn prune_worktrees(repo_root: impl AsRef<Path>) -> Result<()> {
    let repo_path = repo_root.as_ref();
    let _ = run_git(repo_path, &["worktree", "prune"]).await;
    Ok(())
}

/// Deletes a feature branch after it has been merged.
pub async fn delete_branch(
    repo_root: impl AsRef<Path>,
    branch: &str,
    force: bool,
) -> Result<()> {
    let repo_path = repo_root.as_ref();
    let flag = if force { "-D" } else { "-d" };
    run_git(repo_path, &["branch", flag, branch])
        .await
        .with_context(|| format!("Failed to delete branch '{}'", branch))?;
    Ok(())
}

/// Lists all active git worktrees.
pub async fn list_worktrees(repo_root: impl AsRef<Path>) -> Result<Vec<WorktreeInfo>> {
    let repo_path = repo_root.as_ref();
    let raw = run_git(repo_path, &["worktree", "list", "--porcelain"])
        .await
        .context("Failed to list git worktrees")?;

    let mut worktrees = Vec::new();
    let mut current_path = String::new();
    let mut current_head = String::new();
    let mut current_branch = None;

    for line in raw.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            current_path = path.trim().to_string();
        } else if let Some(head) = line.strip_prefix("HEAD ") {
            current_head = head.trim().to_string();
        } else if let Some(branch) = line.strip_prefix("branch ") {
            current_branch = Some(branch.trim().replace("refs/heads/", ""));
        } else if line.is_empty() && !current_path.is_empty() {
            worktrees.push(WorktreeInfo {
                path: current_path.clone(),
                head: current_head.clone(),
                branch: current_branch.take(),
            });
            current_path.clear();
            current_head.clear();
        }
    }

    if !current_path.is_empty() {
        worktrees.push(WorktreeInfo {
            path: current_path,
            head: current_head,
            branch: current_branch,
        });
    }

    Ok(worktrees)
}

/// Gets the current active git branch in the repository or worktree.
pub async fn get_current_branch(dir: impl AsRef<Path>) -> Result<String> {
    run_git(dir.as_ref(), &["rev-parse", "--abbrev-ref", "HEAD"])
        .await
        .context("Failed to get current branch")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        std::fs::write(path, content).expect("write file");
    }

    fn read_str(path: &Path) -> String {
        std::fs::read_to_string(path).expect("read file")
    }

    /// Builds `<tmp>/{root,worktree}` with a live root `.cvh` and a stale
    /// worktree `.cvh` snapshot. Returns `(tmp, root, worktree_abs)`.
    fn setup_sync_fixture(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
        let tmp = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!(
                "cvh_worktree_sync_{}_{}",
                tag,
                uuid::Uuid::new_v4()
            ));
        let _ = std::fs::create_dir_all(&tmp);
        let root = tmp.join("root");
        // Worktree abs mimics `<root>/.cvh/worktrees/<slug>` layout.
        let worktree_abs = root.join(".cvh").join("worktrees").join("task-a");
        let src_cvh = root.join(".cvh");
        let dst_cvh = worktree_abs.join(".cvh");

        // --- Live root state ---
        write_file(&src_cvh.join("pipeline.json"), r#"{"live":true}"#);
        write_file(&src_cvh.join("boards.json"), r#"{"boards":[]}"#);
        write_file(&src_cvh.join("tasks.json"), r#"[]"#);
        write_file(
            &src_cvh.join("settings").join("tool-policy.json"),
            r#"{"globalEnabled":{}}"#,
        );
        write_file(&src_cvh.join("agents").join("dev.md"), "# dev live");
        write_file(&src_cvh.join("agents").join("reviewer.md"), "# reviewer live");
        write_file(&src_cvh.join("agents").join("notes.txt"), "not markdown");
        write_file(&src_cvh.join("tools").join("a.json"), r#"{"name":"a"}"#);
        write_file(&src_cvh.join("tools").join("b.json"), r#"{"name":"b"}"#);
        write_file(&src_cvh.join("tools").join("readme.md"), "not json");
        write_file(&src_cvh.join("hooks").join("hook.sh"), "#!/bin/sh\necho hi");
        write_file(
            &src_cvh.join("hooks").join("nested").join("inner.sh"),
            "#!/bin/sh\necho nested",
        );
        write_file(&src_cvh.join("skills").join("skill.md"), "# skill");
        // Excluded live data.
        write_file(&src_cvh.join("worktrees").join("old").join("f.txt"), "wt");
        write_file(&src_cvh.join("dialogs").join("d.json"), "{}");
        write_file(
            &src_cvh.join("tasks").join("CVH-1").join("data.json"),
            "{}",
        );
        write_file(&src_cvh.join("spill").join("log.log"), "log");
        write_file(&src_cvh.join("llm.json"), r#"{"apiKey":"secret"}"#);
        write_file(&src_cvh.join("hooks").join("evil.secret"), "nope");
        write_file(&src_cvh.join("skills").join(".env.local"), "nope");

        // --- Stale worktree snapshot (must be overwritten) ---
        write_file(&dst_cvh.join("pipeline.json"), r#"{"live":false}"#);
        write_file(&dst_cvh.join("boards.json"), r#"{"stale":true}"#);

        (tmp, root, worktree_abs)
    }

    #[tokio::test]
    async fn test_sync_copies_live_state_and_overwrites_stale_snapshot() {
        let (tmp, root, worktree_abs) = setup_sync_fixture("copy");
        let dst_cvh = worktree_abs.join(".cvh");

        sync_cvh_runtime_state(&root, &worktree_abs).await;

        // Overwritten stale files carry the live content now.
        assert_eq!(read_str(&dst_cvh.join("pipeline.json")), r#"{"live":true}"#);
        assert_eq!(read_str(&dst_cvh.join("boards.json")), r#"{"boards":[]}"#);
        assert!(!dst_cvh.join("tasks.json").exists(), "tasks.json must not be synced to worktrees");
        assert_eq!(
            read_str(&dst_cvh.join("settings").join("tool-policy.json")),
            r#"{"globalEnabled":{}}"#
        );
        assert_eq!(read_str(&dst_cvh.join("agents").join("dev.md")), "# dev live");
        assert_eq!(
            read_str(&dst_cvh.join("agents").join("reviewer.md")),
            "# reviewer live"
        );
        assert_eq!(
            read_str(&dst_cvh.join("tools").join("a.json")),
            r#"{"name":"a"}"#
        );
        assert_eq!(
            read_str(&dst_cvh.join("hooks").join("hook.sh")),
            "#!/bin/sh\necho hi"
        );
        assert_eq!(
            read_str(&dst_cvh.join("hooks").join("nested").join("inner.sh")),
            "#!/bin/sh\necho nested"
        );
        assert_eq!(read_str(&dst_cvh.join("skills").join("skill.md")), "# skill");

        // Wrong extensions are not synced.
        assert!(!dst_cvh.join("agents").join("notes.txt").exists());
        assert!(!dst_cvh.join("tools").join("readme.md").exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn test_sync_excludes_worktrees_llm_dialogs_tasks_spill_secrets() {
        let (tmp, root, worktree_abs) = setup_sync_fixture("exclude");
        let dst_cvh = worktree_abs.join(".cvh");

        sync_cvh_runtime_state(&root, &worktree_abs).await;

        assert!(
            !dst_cvh.join("worktrees").exists(),
            "worktrees/ must never be synced (nesting!)"
        );
        assert!(!dst_cvh.join("dialogs").exists());
        assert!(
            !dst_cvh.join("tasks").exists(),
            "tasks/<id>/ must never be synced"
        );
        assert!(!dst_cvh.join("spill").exists());
        assert!(!dst_cvh.join("llm.json").exists());
        assert!(!dst_cvh.join("hooks").join("evil.secret").exists());
        assert!(!dst_cvh.join("skills").join(".env.local").exists());
        // tasks.json must never be synced to worktrees.
        assert!(
            !dst_cvh.join("tasks.json").exists(),
            "tasks.json must never be synced to worktrees"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn test_sync_is_idempotent() {
        let (tmp, root, worktree_abs) = setup_sync_fixture("idem");
        let dst_cvh = worktree_abs.join(".cvh");

        sync_cvh_runtime_state(&root, &worktree_abs).await;
        let first = read_str(&dst_cvh.join("pipeline.json"));
        // Second run must not fail and must leave identical content.
        sync_cvh_runtime_state(&root, &worktree_abs).await;
        let second = read_str(&dst_cvh.join("pipeline.json"));

        assert_eq!(first, r#"{"live":true}"#);
        assert_eq!(first, second);
        assert_eq!(
            read_str(&dst_cvh.join("settings").join("tool-policy.json")),
            r#"{"globalEnabled":{}}"#
        );

        // Missing-source sync is a tolerated no-op, not a crash.
        let empty_root = tmp.join("empty-root");
        let empty_wt = tmp.join("empty-wt");
        std::fs::create_dir_all(&empty_wt).expect("empty wt");
        sync_cvh_runtime_state(&empty_root, &empty_wt).await;

        let _ = std::fs::remove_dir_all(&tmp);
    }
}

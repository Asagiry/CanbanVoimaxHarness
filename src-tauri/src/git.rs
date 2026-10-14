use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorktreeInfo {
    pub path: String,
    pub head: String,
    pub branch: Option<String>,
}

async fn run_git(repo_root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(args)
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

    // Check if branch already exists
    let branch_check = Command::new("git")
        .current_dir(repo_path)
        .args(["rev-parse", "--verify", &branch_name])
        .output()
        .await;

    let branch_exists = match branch_check {
        Ok(out) => out.status.success(),
        Err(_) => false,
    };

    let worktree_str = worktree_rel.to_string_lossy().to_string();

    if branch_exists {
        run_git(
            repo_path,
            &["worktree", "add", &worktree_str, &branch_name],
        )
        .await
        .with_context(|| format!("Failed to attach existing branch '{}' to worktree '{}'", branch_name, worktree_str))?;
    } else {
        // Verify base branch exists; if not, fallback to "HEAD" so it works on any repo/branch
        let base_check = Command::new("git")
            .current_dir(repo_path)
            .args(["rev-parse", "--verify", base_branch])
            .output()
            .await;
        let effective_base = match base_check {
            Ok(out) if out.status.success() => base_branch,
            _ => "HEAD",
        };

        run_git(
            repo_path,
            &["worktree", "add", "-b", &branch_name, &worktree_str, effective_base],
        )
        .await
        .with_context(|| format!("Failed to create worktree '{}' from branch '{}'", worktree_str, effective_base))?;
    }

    // The fresh worktree carries the COMMITTED .cvh snapshot, which is stale by
    // definition. Overwrite it with the live runtime-state from <root>/.cvh
    // (best-effort: sync errors only warn, never fail creation).
    sync_cvh_runtime_state(repo_path, &worktree_abs);

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
fn copy_file_overwrite(src: &Path, dst: &Path) {
    if let Some(parent) = dst.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!(
                "CVH [sync_cvh_runtime_state]: warning: cannot create dir {:?}: {}",
                parent, e
            );
            return;
        }
    }
    if let Err(e) = std::fs::copy(src, dst) {
        eprintln!(
            "CVH [sync_cvh_runtime_state]: warning: cannot copy {:?} -> {:?}: {}",
            src, dst, e
        );
    }
}

/// Copies top-level files of `src_dir` into `dst_dir` when `keep` accepts the
/// file name. Secret files are always skipped. Subdirectories are ignored.
fn copy_top_level_filtered(src_dir: &Path, dst_dir: &Path, keep: impl Fn(&str) -> bool) {
    if !src_dir.is_dir() {
        return;
    }
    if let Err(e) = std::fs::create_dir_all(dst_dir) {
        eprintln!(
            "CVH [sync_cvh_runtime_state]: warning: cannot create dir {:?}: {}",
            dst_dir, e
        );
        return;
    }
    let entries = match std::fs::read_dir(src_dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!(
                "CVH [sync_cvh_runtime_state]: warning: cannot list {:?}: {}",
                src_dir, e
            );
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!(
                    "CVH [sync_cvh_runtime_state]: warning: bad dir entry in {:?}: {}",
                    src_dir, e
                );
                continue;
            }
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if is_secret_file_name(&name) || !keep(&name) {
            continue;
        }
        copy_file_overwrite(&path, &dst_dir.join(&name));
    }
}

/// Recursively copies `src_dir` into `dst_dir` (overwrite). Secret files are
/// skipped at every level. Errors are warnings only.
fn copy_dir_recursive_filtered(src_dir: &Path, dst_dir: &Path) {
    if !src_dir.is_dir() {
        return;
    }
    if let Err(e) = std::fs::create_dir_all(dst_dir) {
        eprintln!(
            "CVH [sync_cvh_runtime_state]: warning: cannot create dir {:?}: {}",
            dst_dir, e
        );
        return;
    }
    let entries = match std::fs::read_dir(src_dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!(
                "CVH [sync_cvh_runtime_state]: warning: cannot list {:?}: {}",
                src_dir, e
            );
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!(
                    "CVH [sync_cvh_runtime_state]: warning: bad dir entry in {:?}: {}",
                    src_dir, e
                );
                continue;
            }
        };
        let src_path = entry.path();
        let name = match src_path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if src_path.is_dir() {
            copy_dir_recursive_filtered(&src_path, &dst_dir.join(&name));
        } else if src_path.is_file() {
            if is_secret_file_name(&name) {
                continue;
            }
            copy_file_overwrite(&src_path, &dst_dir.join(&name));
        }
    }
}

/// Synchronizes the live runtime-state from `<root>/.cvh` into the fresh
/// worktree's `<worktree>/.cvh`, overwriting the stale committed snapshot.
///
/// Copies (if present in root): `pipeline.json`, `boards.json`, `tasks.json`,
/// `settings/tool-policy.json`, `agents/*.md`, `tools/*.json`, `hooks/` and
/// `skills/` (recursively, minus secrets).
///
/// Never copies: `worktrees/` (nesting!), `dialogs/`, `tasks/<id>/`, `spill/`,
/// secrets (`llm.json`, `*.secret`, `*.key`, `.env*`).
///
/// Best-effort: all errors are `eprintln!` warnings, never propagated.
pub fn sync_cvh_runtime_state(repo_root: &Path, worktree_abs: &Path) {
    let src_cvh = repo_root.join(".cvh");
    if !src_cvh.is_dir() {
        return;
    }
    let dst_cvh = worktree_abs.join(".cvh");
    if let Err(e) = std::fs::create_dir_all(&dst_cvh) {
        eprintln!(
            "CVH [sync_cvh_runtime_state]: warning: cannot create {:?}: {}",
            dst_cvh, e
        );
        return;
    }

    // Single files (overwrite stale snapshot when root has a live copy).
    for rel in ["pipeline.json", "boards.json", "tasks.json"] {
        let src = src_cvh.join(rel);
        if src.is_file() {
            if let Some(name) = src.file_name().and_then(|n| n.to_str()) {
                if is_secret_file_name(name) {
                    continue;
                }
            }
            copy_file_overwrite(&src, &dst_cvh.join(rel));
        }
    }
    // Canonical Tool Policy Matrix only (not the whole settings/ dir).
    let src_policy = src_cvh.join("settings").join("tool-policy.json");
    if src_policy.is_file() {
        copy_file_overwrite(
            &src_policy,
            &dst_cvh.join("settings").join("tool-policy.json"),
        );
    }

    // Flat filtered copies.
    copy_top_level_filtered(&src_cvh.join("agents"), &dst_cvh.join("agents"), |n| {
        n.ends_with(".md")
    });
    copy_top_level_filtered(&src_cvh.join("tools"), &dst_cvh.join("tools"), |n| {
        n.ends_with(".json")
    });

    // Whole-directory copies (minus secrets).
    copy_dir_recursive_filtered(&src_cvh.join("hooks"), &dst_cvh.join("hooks"));
    copy_dir_recursive_filtered(&src_cvh.join("skills"), &dst_cvh.join("skills"));
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

/// Merges a feature branch into the target branch.
pub async fn merge_worktree_branch(
    repo_root: impl AsRef<Path>,
    branch: &str,
    target_branch: &str,
) -> Result<String> {
    let repo_path = repo_root.as_ref();

    // 1. Checkout target branch
    run_git(repo_path, &["checkout", target_branch])
        .await
        .with_context(|| format!("Failed to checkout target branch '{}'", target_branch))?;

    // 2. Perform merge
    let commit_msg = format!("merge({}): merge {} into {}", branch, branch, target_branch);
    let merge_output = run_git(
        repo_path,
        &["merge", "--no-ff", branch, "-m", &commit_msg],
    )
    .await
    .with_context(|| format!("Failed to merge branch '{}' into '{}'", branch, target_branch))?;

    Ok(merge_output)
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

/// Gets the latest commit hash (short 7-char or full)
pub async fn get_commit_hash(dir: impl AsRef<Path>, short: bool) -> Result<String> {
    let args: &[&str] = if short {
        &["rev-parse", "--short", "HEAD"]
    } else {
        &["rev-parse", "HEAD"]
    };
    run_git(dir.as_ref(), args).await.context("Failed to get commit hash")
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
        let tmp = std::env::temp_dir().join(format!(
            "cvh_worktree_sync_{}_{}",
            tag,
            uuid::Uuid::new_v4()
        ));
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

    #[test]
    fn test_sync_copies_live_state_and_overwrites_stale_snapshot() {
        let (tmp, root, worktree_abs) = setup_sync_fixture("copy");
        let dst_cvh = worktree_abs.join(".cvh");

        sync_cvh_runtime_state(&root, &worktree_abs);

        // Overwritten stale files carry the live content now.
        assert_eq!(read_str(&dst_cvh.join("pipeline.json")), r#"{"live":true}"#);
        assert_eq!(read_str(&dst_cvh.join("boards.json")), r#"{"boards":[]}"#);
        assert_eq!(read_str(&dst_cvh.join("tasks.json")), r#"[]"#);
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

    #[test]
    fn test_sync_excludes_worktrees_llm_dialogs_tasks_spill_secrets() {
        let (tmp, root, worktree_abs) = setup_sync_fixture("exclude");
        let dst_cvh = worktree_abs.join(".cvh");

        sync_cvh_runtime_state(&root, &worktree_abs);

        assert!(
            !dst_cvh.join("worktrees").exists(),
            "worktrees/ must never be synced (nesting!)"
        );
        assert!(!dst_cvh.join("dialogs").exists());
        assert!(
            !dst_cvh.join("tasks").exists(),
            "tasks/<id>/ must never be synced (tasks.json alone is)"
        );
        assert!(!dst_cvh.join("spill").exists());
        assert!(!dst_cvh.join("llm.json").exists());
        assert!(!dst_cvh.join("hooks").join("evil.secret").exists());
        assert!(!dst_cvh.join("skills").join(".env.local").exists());
        // tasks.json itself IS synced even though tasks/<id>/ is not.
        assert!(dst_cvh.join("tasks.json").is_file());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_sync_is_idempotent() {
        let (tmp, root, worktree_abs) = setup_sync_fixture("idem");
        let dst_cvh = worktree_abs.join(".cvh");

        sync_cvh_runtime_state(&root, &worktree_abs);
        let first = read_str(&dst_cvh.join("pipeline.json"));
        // Second run must not fail and must leave identical content.
        sync_cvh_runtime_state(&root, &worktree_abs);
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
        sync_cvh_runtime_state(&empty_root, &empty_wt);

        let _ = std::fs::remove_dir_all(&tmp);
    }
}

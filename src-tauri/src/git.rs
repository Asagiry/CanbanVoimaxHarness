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
        run_git(
            repo_path,
            &["worktree", "add", "-b", &branch_name, &worktree_str, base_branch],
        )
        .await
        .with_context(|| format!("Failed to create worktree '{}' from branch '{}'", worktree_str, base_branch))?;
    }

    Ok(worktree_abs)
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

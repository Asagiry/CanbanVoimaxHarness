
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub const PROTECTED_CVH_PATHS: &[&str] = &[
    ".cvh/agents/",
    ".cvh/rules/",
    ".cvh/settings/tool-policy.json",
    ".cvh/boards.json",
    ".cvh/hooks/",
    ".cvh/tools/",
    ".cvh/skills/",
];

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
/// Copies (if present in root): `boards.json`,
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
    for rel in ["boards.json"] {
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
    copy_dir_recursive_filtered(&src_cvh.join("rules"), &dst_cvh.join("rules")).await;

    // Agent role directories (agents/<role>/rules/*.md, persona.md, setting, etc.)
    if let Ok(mut entries) = tokio::fs::read_dir(&src_cvh.join("agents")).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(ft) = entry.file_type().await {
                if ft.is_dir() {
                    let dir_name = entry.file_name();
                    copy_dir_recursive_filtered(
                        &entry.path(),
                        &dst_cvh.join("agents").join(dir_name),
                    )
                    .await;
                }
            }
        }
    }
    // Deliberately NOT synced: worktrees/, dialogs/, tasks/<id>/, spill/,
    // llm.json and other secrets.
}

pub async fn sync_cvh_to_all_worktrees(repo_root: &Path) {
    let Ok(wts) = super::list_worktrees(repo_root).await else {
        return;
    };
    let root_canon = repo_root.canonicalize().unwrap_or_else(|_| repo_root.to_path_buf());
    for wt in wts {
        let p = PathBuf::from(&wt.path);
        let canon = p.canonicalize().unwrap_or_else(|_| p.clone());
        if canon == root_canon {
            continue;
        }
        if !p.exists() {
            continue;
        }
        sync_cvh_runtime_state(repo_root, &p).await;
    }
}

pub async fn mark_worktree_cvh_untracked(worktree: &Path) {
    let mut cmd = Command::new("git");
    cmd.current_dir(worktree)
        .args(["update-index", "--assume-unchanged", "--", ".cvh"]);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let _ = cmd.output().await;
}

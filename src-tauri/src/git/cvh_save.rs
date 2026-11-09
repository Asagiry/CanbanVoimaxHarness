use super::run_git;
use crate::commands::git::is_cvh_config_path;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CvhGitStatus {
    pub has_changes: bool,
    pub changed_paths: Vec<String>,
    pub branch: String,
    pub ahead: bool,
}

fn parse_porcelain(output: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for line in output.lines() {
        let line = line.trim_end();
        if line.len() < 4 {
            continue;
        }
        let mut rest = &line[3..];
        if let Some((_, new_path)) = rest.split_once(" -> ") {
            rest = new_path;
        }
        let path = rest.trim().trim_matches('"').to_string();
        if !path.is_empty() && is_cvh_config_path(&path) {
            paths.push(path);
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

pub async fn cvh_git_status_at(root: &Path) -> Result<CvhGitStatus, String> {
    let branch = run_git(root, &["rev-parse", "--abbrev-ref", "HEAD"])
        .await
        .map_err(|e| e.to_string())?;
    let porcelain = run_git(root, &["status", "--porcelain", "--", ".cvh/"])
        .await
        .map_err(|e| e.to_string())?;
    let changed_paths = parse_porcelain(&porcelain);
    let ahead = run_git(root, &["rev-list", "--count", "@{u}..HEAD"])
        .await
        .map(|s| s.trim().parse::<usize>().unwrap_or(0) > 0)
        .unwrap_or(false);
    Ok(CvhGitStatus {
        has_changes: !changed_paths.is_empty(),
        changed_paths,
        branch,
        ahead,
    })
}

pub async fn cvh_git_save_at(root: &Path, message: &str) -> Result<CvhGitStatus, String> {
    let before = cvh_git_status_at(root).await?;
    if !before.has_changes {
        return Ok(before);
    }
    // Stage ONLY the is_cvh_config_path-filtered changed paths explicitly.
    // Never `git add .cvh/`: that would stage runtime artifacts (harness.db,
    // llm.json, logs, spill, dialogs, tasks, secrets) and `git commit` would
    // then commit everything staged, bypassing the exclusion contract.
    let mut add_args: Vec<String> = vec!["add".to_string(), "--".to_string()];
    add_args.extend(before.changed_paths.iter().cloned());
    let add_refs: Vec<&str> = add_args.iter().map(String::as_str).collect();
    run_git(root, &add_refs)
        .await
        .map_err(|e| e.to_string())?;

    // Defense in depth: unstage any non-config path that was already staged
    // before this call (e.g. a tracked runtime file staged by the user), so
    // the commit below can only ever contain config paths.
    let staged = run_git(root, &["diff", "--cached", "--name-only", "--", ".cvh/"])
        .await
        .map_err(|e| e.to_string())?;
    let staged_paths: Vec<String> = staged
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    let non_config: Vec<String> = staged_paths
        .iter()
        .filter(|p| !is_cvh_config_path(p))
        .cloned()
        .collect();
    if !non_config.is_empty() {
        let mut restore_args: Vec<String> =
            vec!["restore".to_string(), "--staged".to_string(), "--".to_string()];
        restore_args.extend(non_config.iter().cloned());
        let restore_refs: Vec<&str> = restore_args.iter().map(String::as_str).collect();
        run_git(root, &restore_refs)
            .await
            .map_err(|e| format!("Failed to unstage non-config paths: {}", e))?;
    }
    let config_staged: Vec<String> = staged_paths
        .iter()
        .filter(|p| is_cvh_config_path(p))
        .cloned()
        .collect();
    if config_staged.is_empty() {
        return cvh_git_status_at(root).await;
    }
    run_git(root, &["commit", "-m", message])
        .await
        .map_err(|e| e.to_string())?;
    run_git(root, &["push", "origin", "HEAD:main"])
        .await
        .map_err(|e| e.to_string())?;
    cvh_git_status_at(root).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp_dir(tag: &str) -> PathBuf {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_save_{}_{}", tag, uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create tmp");
        dir
    }

    fn git(dir: &Path, args: &[&str]) {
        let out = std::process::Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .expect("git");
        assert!(out.status.success(), "git {:?} failed: {:?}", args, String::from_utf8_lossy(&out.stderr));
    }

    fn write(dir: &Path, rel: &str, content: &str) {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).expect("parent");
        std::fs::write(p, content).expect("write");
    }

    fn init_repo(tag: &str) -> PathBuf {
        let dir = tmp_dir(tag);
        git(&dir, &["init", "-b", "main"]);
        git(&dir, &["config", "user.email", "t@t.t"]);
        git(&dir, &["config", "user.name", "t"]);
        write(&dir, ".cvh/rules/x.md", "v1");
        write(&dir, ".cvh/harness.db", "db");
        git(&dir, &["add", "."]);
        git(&dir, &["commit", "-m", "init"]);
        dir
    }

    #[test]
    fn parse_porcelain_filters_config_paths() {
        let out = " M .cvh/rules/x.md\n?? .cvh/harness.db\n M .cvh/boards.json\n D .cvh/agents/dev/rules/a.md\n";
        let paths = parse_porcelain(out);
        assert_eq!(paths, vec![".cvh/agents/dev/rules/a.md", ".cvh/boards.json", ".cvh/rules/x.md"]);
    }

    #[test]
    fn parse_porcelain_includes_setting_and_agent_md() {
        let out = " M .cvh/agents/dev/setting\n M .cvh/agents/dev/agent.md\n M .cvh/agents/dev/persona.md\n";
        let paths = parse_porcelain(out);
        assert_eq!(
            paths,
            vec![
                ".cvh/agents/dev/agent.md",
                ".cvh/agents/dev/persona.md",
                ".cvh/agents/dev/setting"
            ]
        );
    }

    #[tokio::test]
    async fn status_detects_rule_change_and_excludes_db() {
        let dir = init_repo("status");
        write(&dir, ".cvh/rules/x.md", "v2");
        write(&dir, ".cvh/harness.db", "db2");
        let st = cvh_git_status_at(&dir).await.expect("status");
        assert!(st.has_changes);
        assert_eq!(st.changed_paths, vec![".cvh/rules/x.md"]);
        assert_eq!(st.branch, "main");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn save_commits_only_config_paths() {
        let dir = init_repo("save");
        let bare = tmp_dir("bare");
        git(&bare, &["init", "--bare", "-b", "main"]);
        git(&dir, &["remote", "add", "origin", bare.to_str().unwrap()]);
        git(&dir, &["push", "origin", "main"]);
        write(&dir, ".cvh/rules/x.md", "v2");
        write(&dir, ".cvh/harness.db", "db2");
        // Pre-stage the runtime db (simulates a tracked/force-added artifact):
        // save must unstage it and never commit it.
        git(&dir, &["add", "--", ".cvh/harness.db"]);
        let st = cvh_git_save_at(&dir, "chore(cvh): save").await.expect("save");
        assert!(!st.has_changes);
        let log = std::process::Command::new("git")
            .current_dir(&dir)
            .args(["log", "--oneline", "-1"])
            .output()
            .expect("log");
        let log_s = String::from_utf8_lossy(&log.stdout);
        assert!(log_s.contains("chore(cvh): save"), "log: {}", log_s);
        let show = std::process::Command::new("git")
            .current_dir(&dir)
            .args(["show", "--name-only", "--format=", "HEAD"])
            .output()
            .expect("show");
        let show_s = String::from_utf8_lossy(&show.stdout);
        assert!(show_s.contains(".cvh/rules/x.md"), "commit must contain rule: {}", show_s);
        assert!(!show_s.contains("harness.db"), "commit must NOT contain harness.db: {}", show_s);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&bare);
    }
}

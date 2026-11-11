use anyhow::{bail, Result};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::Manager;

use super::guards::{SubagentActiveHandleGuard, TaskStatusGuard};
use crate::agent::{
    find_subagent_report_from_db, find_subagent_session, remove_subagent_receiver, ToolContext,
    SUBAGENT_RECEIVERS, SUBAGENT_SESSIONS,
};

pub async fn execute_collect_subagent(
    ctx: &ToolContext<'_>,
    args: &serde_json::Value,
) -> Result<String> {
    let repo_root = ctx.repo_root;
    let task_id = ctx.task_id;

    let mut ids: Vec<String> = Vec::new();
    if let Some(id) = args.get("subagent_id").and_then(|v| v.as_str()) {
        if !id.trim().is_empty() {
            ids.push(id.trim().to_string());
        }
    }
    if let Some(arr) = args.get("subagent_ids").and_then(|v| v.as_array()) {
        for v in arr {
            if let Some(s) = v.as_str() {
                let trimmed = s.trim();
                if !trimmed.is_empty() && !ids.iter().any(|existing| existing == trimmed) {
                    ids.push(trimmed.to_string());
                }
            }
        }
    }

    if ids.is_empty() {
        bail!("Missing required parameter: provide either 'subagent_id' or 'subagent_ids'");
    }

    let mut collected_reports: Vec<(String, String)> = Vec::new();

    for subagent_id in &ids {
        // 1. Check SUBAGENT_RECEIVERS (in-flight background subagents)
        let rx_opt = {
            let mut reg = SUBAGENT_RECEIVERS.lock().await;
            remove_subagent_receiver(&mut reg, subagent_id)
        };

        if let Some((resolved_id, mut rx)) = rx_opt {
            let guard_id = resolved_id.clone();
            let _active_guard = SubagentActiveHandleGuard {
                app_handle: ctx.app_handle.as_ref(),
                repo_root,
                subagent_id: &guard_id,
            };
            let mut status_guard = task_id.map(|tid| TaskStatusGuard::new(ctx.app_handle.as_ref(), repo_root, tid));
            // Poll parent abort while waiting (D1): bare rx.await hangs forever on
            // panic/abort of the subagent task — mirror execute_subagent's 200ms poll.
            let wait_result = loop {
                tokio::select! {
                    res = &mut rx => break res,
                    _ = tokio::time::sleep(Duration::from_millis(200)) => {
                        if ctx.abort_handle.load(Ordering::SeqCst) {
                            // Parent aborted: cascade-abort the subagent and bail.
                            if let Some((_, session)) = find_subagent_session(task_id, &resolved_id).await {
                                session.abort_handle.store(true, Ordering::SeqCst);
                                if let Some(sender) = session.sender.lock().await.take() {
                                    let _ = sender.send("[Субагент прерван пользователем / Subagent aborted by user]".to_string());
                                }
                            }
                            break Ok("[Субагент прерван пользователем / Subagent aborted by user]".to_string());
                        }
                    }
                }
            };
            match wait_result {
                Ok(report) => {
                    if let Some(ref mut g) = status_guard {
                        g.disarm_to_running();
                    }
                    let mut sessions = SUBAGENT_SESSIONS.lock().await;
                    sessions.remove(&resolved_id);
                    collected_reports.push((resolved_id, report));
                    continue;
                }
                Err(_) => {
                    if let Some(ref mut g) = status_guard {
                        g.disarm_to_running();
                    }
                    // 1. Check SUBAGENT_SESSIONS for completed report or aborted status
                    if let Some((_, session)) = find_subagent_session(task_id, &resolved_id).await {
                        let (report_opt, is_aborted) = {
                            let info = session.retry_info.lock().await;
                            (
                                info.report.clone(),
                                info.status == "aborted" || session.abort_handle.load(Ordering::SeqCst),
                            )
                        };
                        if let Some(rep) = report_opt {
                            let mut sessions = SUBAGENT_SESSIONS.lock().await;
                            sessions.remove(&resolved_id);
                            collected_reports.push((resolved_id, rep));
                            continue;
                        }
                        if is_aborted {
                            let mut sessions = SUBAGENT_SESSIONS.lock().await;
                            sessions.remove(&resolved_id);
                            collected_reports.push((resolved_id, "[Субагент прерван пользователем / Subagent aborted by user]".to_string()));
                            continue;
                        }
                    }

                    // 2. Check SQLite dialog_items for persisted subagent output report
                    if let Some(db_report) = find_subagent_report_from_db(repo_root, task_id, &resolved_id)
                        .or_else(|| find_subagent_report_from_db(repo_root, task_id, subagent_id))
                    {
                        let mut sessions = SUBAGENT_SESSIONS.lock().await;
                        sessions.remove(&resolved_id);
                        collected_reports.push((resolved_id, db_report));
                        continue;
                    }

                    let (is_aborted, paused_err) = if let Some((_, session)) = find_subagent_session(task_id, &resolved_id).await {
                        let info = session.retry_info.lock().await;
                        let aborted = info.status == "aborted" || session.abort_handle.load(Ordering::SeqCst);
                        let paused = if info.status == "paused" {
                            Some(info.last_error.clone().unwrap_or_else(|| "transient error".to_string()))
                        } else {
                            None
                        };
                        (aborted, paused)
                    } else {
                        (false, None)
                    };
                    if let Some(err) = paused_err {
                        bail!("Subagent [{}] paused on transient error: {}", resolved_id, err);
                    }
                    let mut sessions = SUBAGENT_SESSIONS.lock().await;
                    sessions.remove(&resolved_id);
                    if is_aborted {
                        collected_reports.push((resolved_id, "[Субагент прерван пользователем / Subagent aborted by user]".to_string()));
                        continue;
                    }
                    bail!("Subagent [{}] channel closed before completion", resolved_id);
                }
            }
        }

        // 2. Check SUBAGENT_SESSIONS (synchronous or already completed subagents)
        if let Some((resolved_id, session)) = find_subagent_session(task_id, subagent_id).await {
            // Wait if still running without a receiver entry (e.g. second collector or race)
            let mut status_guard: Option<TaskStatusGuard<'_>> = None;
            loop {
                let status_snapshot = {
                    let info = session.retry_info.lock().await;
                    if let Some(ref rep) = info.report {
                        Ok(Some(rep.clone()))
                    } else if info.status == "aborted" || session.abort_handle.load(Ordering::SeqCst) {
                        Ok(Some("[Субагент прерван пользователем / Subagent aborted by user]".to_string()))
                    } else if info.status == "paused" {
                        Err(anyhow::anyhow!(
                            "Subagent [{}] is paused on transient error: {}",
                            resolved_id,
                            info.last_error.as_deref().unwrap_or("Unknown error")
                        ))
                    } else if info.status == "error" {
                        Err(anyhow::anyhow!(
                            "Subagent [{}] failed with error: {}",
                            resolved_id,
                            info.last_error.as_deref().unwrap_or("Unknown error")
                        ))
                    } else if info.status == "running" {
                        Ok(None)
                    } else {
                        Ok(Some(String::new()))
                    }
                };
                match status_snapshot {
                    Ok(Some(rep)) if !rep.is_empty() => {
                        if let Some(ref mut g) = status_guard {
                            g.disarm_to_running();
                        }
                        collected_reports.push((resolved_id.clone(), rep));
                        break;
                    }
                    Ok(Some(_)) => break,
                    Ok(None) => {
                        if status_guard.is_none() {
                            status_guard = task_id.map(|tid| TaskStatusGuard::new(ctx.app_handle.as_ref(), repo_root, tid));
                        }
                        if ctx.abort_handle.load(Ordering::SeqCst) {
                            session.abort_handle.store(true, Ordering::SeqCst);
                            if let Some(ref mut g) = status_guard {
                                g.disarm_to_running();
                            }
                            collected_reports.push((resolved_id.clone(), "[Субагент прерван пользователем / Subagent aborted by user]".to_string()));
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }
                    Err(e) => {
                        if let Some(ref mut g) = status_guard {
                            g.disarm_to_running();
                        }
                        return Err(e);
                    }
                }
            }
            if collected_reports.last().is_some_and(|(id, _)| id == &resolved_id) {
                if let Some(ref app) = ctx.app_handle {
                    if let Some(state_ref) = app.try_state::<crate::commands::AppState>() {
                        state_ref.remove_active_agent(repo_root, &resolved_id);
                    }
                }
                continue;
            }
            if let Some(db_report) = find_subagent_report_from_db(repo_root, task_id, &resolved_id)
                .or_else(|| find_subagent_report_from_db(repo_root, task_id, subagent_id))
            {
                collected_reports.push((resolved_id, db_report));
                continue;
            }
        }

        // 3. Check SQLite dialog_items for persisted subagent output report
        if let Some(db_report) = find_subagent_report_from_db(repo_root, task_id, subagent_id) {
            collected_reports.push((subagent_id.clone(), db_report));
            continue;
        }

        bail!("Subagent [{}] not found in background registry or session history", subagent_id);
    }

    if collected_reports.len() == 1 {
        Ok(collected_reports.remove(0).1)
    } else {
        Ok(collected_reports
            .into_iter()
            .map(|(id, report)| format!("### Subagent [{}]\n\n{}", id, report))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n"))
    }
}

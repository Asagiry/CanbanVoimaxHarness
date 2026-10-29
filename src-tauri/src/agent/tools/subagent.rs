//! Subagent Delegation Tool Handlers (subagent, subagent_bg, collect_subagent, subagent_list)
//! Extracted from agent.rs per ub-mode Clean Code & Command Pattern principles.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use anyhow::{bail, Context, Result};
use tauri::Manager;
use tokio::sync::oneshot;

use crate::agent::{
    are_subagent_prompts_duplicate, find_subagent_report_from_db, find_subagent_session,
    prune_subagent_sessions_cache, remove_subagent_receiver, run_inprocess_subagent,
    SubagentRetryInfo, SubagentSession, ToolContext, SUBAGENT_RECEIVERS, SUBAGENT_SESSIONS,
};

pub async fn execute_subagent(
    ctx: &ToolContext<'_>,
    args: &serde_json::Value,
) -> Result<String> {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .context("Missing required parameter 'title'")?;
    let prompt = args
        .get("prompt")
        .and_then(|v| v.as_str())
        .context("Missing required parameter 'prompt'")?;
    let role = args
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("general");
    let requested_tools: Option<Vec<String>> = args.get("tools").and_then(|v| v.as_array()).map(|arr| {
        arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()
    });

    let parent_id = ctx.task_id.unwrap_or("unknown");
    let abort_handle = &ctx.abort_handle;

    // Deduplication check across sessions:
    // If an identical subagent already completed with report, return it.
    {
        let sessions = SUBAGENT_SESSIONS.lock().await;
        for sess in sessions.values() {
            let info = sess.retry_info.lock().await;
            if info.parent_task_id == parent_id {
                let is_dup = info.role.eq_ignore_ascii_case(role) && are_subagent_prompts_duplicate(&info.prompt, prompt);

                if is_dup && info.status == "finished" {
                    if let Some(ref rep) = info.report {
                        eprintln!("CVH [agent]: Reusing completed subagent report for request (role: '{}')", info.role);
                        return Ok(rep.clone());
                    }
                }
            }
        }
    }

    let subagent_suffix = match ctx.call_id {
        Some(cid) if !cid.trim().is_empty() => cid.trim().to_string(),
        _ => format!(
            "{}-{}",
            chrono::Utc::now().timestamp_millis(),
            uuid::Uuid::new_v4().to_string().chars().take(4).collect::<String>()
        ),
    };
    let subagent_id = format!("{}.sub-{}", parent_id, subagent_suffix);
    let tc_id_opt = ctx.call_id.map(|s| s.to_string());

    let sub_abort_handle = Arc::new(AtomicBool::new(false));
    let sub_join_slot = Arc::new(tokio::sync::Mutex::new(None));

    let (tx, rx) = oneshot::channel::<String>();
    let retry_info = SubagentRetryInfo {
        parent_task_id: parent_id.to_string(),
        subagent_id: subagent_id.clone(),
        tool_call_id: tc_id_opt,
        title: title.to_string(),
        role: role.to_string(),
        prompt: prompt.to_string(),
        requested_tools: requested_tools.clone(),
        ctx: ctx.to_owned(),
        status: "running".to_string(),
        last_error: None,
        report: None,
        is_bg: false,
        duration_ms: None,
        steps: 0,
    };

    let session = SubagentSession {
        sender: Arc::new(tokio::sync::Mutex::new(Some(tx))),
        retry_info: Arc::new(tokio::sync::Mutex::new(retry_info)),
        abort_handle: Arc::clone(&sub_abort_handle),
        join_handle: Arc::clone(&sub_join_slot),
    };

    {
        let mut sessions = SUBAGENT_SESSIONS.lock().await;
        sessions.insert(subagent_id.clone(), session);
    }
    prune_subagent_sessions_cache().await;

    let mut owned_ctx = ctx.to_owned();
    owned_ctx.abort_handle = Arc::clone(&sub_abort_handle);
    let sub_id = subagent_id.clone();
    let title_s = title.to_string();
    let role_s = role.to_string();
    let prompt_s = prompt.to_string();

    let join_h = tokio::spawn(async move {
        let _ = run_inprocess_subagent(
            owned_ctx,
            title_s,
            role_s,
            prompt_s,
            requested_tools,
            Some(sub_id),
        ).await;
    });
    *sub_join_slot.lock().await = Some(join_h);

    // Register into state.active_agents so abort_agent can track/abort it
    if let Some(ref app) = ctx.app_handle {
        if let Some(state_ref) = app.try_state::<crate::commands::AppState>() {
            if let Ok(mut map) = state_ref.active_agents.lock() {
                map.insert(
                    subagent_id.clone(),
                    crate::commands::ActiveAgentHandle {
                        abort_flag: Arc::clone(&sub_abort_handle),
                        join_handle: Arc::clone(&sub_join_slot),
                    },
                );
            }
        }
    }

    // Parent agent blocks synchronously awaiting clean report from channel (Zero-Pollution Invariant!)
    let mut rx = rx;
    let clean_report = loop {
        tokio::select! {
            res = &mut rx => {
                match res {
                    Ok(report) => break report,
                    Err(_) => {
                        if sub_abort_handle.load(Ordering::SeqCst) {
                            return Ok("[Субагент прерван пользователем / Subagent aborted by user]".to_string());
                        }
                        bail!("Subagent channel closed before completion");
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                if abort_handle.load(Ordering::SeqCst) {
                    sub_abort_handle.store(true, Ordering::SeqCst);
                    bail!("Agent execution aborted while waiting for subagent");
                }
                if sub_abort_handle.load(Ordering::SeqCst) {
                    return Ok("[Субагент прерван пользователем / Subagent aborted by user]".to_string());
                }
            }
        }
    };
    Ok(clean_report)
}

pub async fn execute_subagent_bg(
    ctx: &ToolContext<'_>,
    args: &serde_json::Value,
) -> Result<String> {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .context("Missing required parameter 'title'")?;
    let prompt = args
        .get("prompt")
        .and_then(|v| v.as_str())
        .context("Missing required parameter 'prompt'")?;
    let role = args
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("general");
    let requested_tools: Option<Vec<String>> = args.get("tools").and_then(|v| v.as_array()).map(|arr| {
        arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()
    });

    let parent_id = ctx.task_id.unwrap_or("unknown");

    // Deduplication check across sessions: if an identical subagent is already actively running or finished, reuse it.
    {
        let sessions = SUBAGENT_SESSIONS.lock().await;
        for (sid, sess) in sessions.iter() {
            let info = sess.retry_info.lock().await;
            if info.parent_task_id == parent_id
                && (info.status == "running" || info.status == "finished")
            {
                let is_dup = info.role.eq_ignore_ascii_case(role) && are_subagent_prompts_duplicate(&info.prompt, prompt);

                if is_dup {
                    eprintln!("CVH [agent]: Reusing existing subagent {} (role: '{}') for duplicate subagent_bg request", sid, info.role);
                    return Ok(serde_json::to_string(&serde_json::json!({
                        "subagent_id": sid,
                        "status": &info.status,
                        "note": format!("Reused existing {} subagent to prevent duplicate subagent_bg execution", info.role)
                    }))?);
                }
            }
        }
    }

    let subagent_suffix = match ctx.call_id {
        Some(cid) if !cid.trim().is_empty() => cid.trim().to_string(),
        _ => format!(
            "{}-{}",
            chrono::Utc::now().timestamp_millis(),
            uuid::Uuid::new_v4().to_string().chars().take(4).collect::<String>()
        ),
    };
    let subagent_id = format!("{}.sub-{}", parent_id, subagent_suffix);
    let tc_id_opt = ctx.call_id.map(|s| s.to_string());

    let sub_abort_handle = Arc::new(AtomicBool::new(false));
    let sub_join_slot = Arc::new(tokio::sync::Mutex::new(None));

    let (tx, rx) = oneshot::channel::<String>();
    let retry_info = SubagentRetryInfo {
        parent_task_id: parent_id.to_string(),
        subagent_id: subagent_id.clone(),
        tool_call_id: tc_id_opt,
        title: title.to_string(),
        role: role.to_string(),
        prompt: prompt.to_string(),
        requested_tools: requested_tools.clone(),
        ctx: ctx.to_owned(),
        status: "running".to_string(),
        last_error: None,
        report: None,
        is_bg: true,
        duration_ms: None,
        steps: 0,
    };

    let session = SubagentSession {
        sender: Arc::new(tokio::sync::Mutex::new(Some(tx))),
        retry_info: Arc::new(tokio::sync::Mutex::new(retry_info)),
        abort_handle: Arc::clone(&sub_abort_handle),
        join_handle: Arc::clone(&sub_join_slot),
    };

    {
        let mut receivers = SUBAGENT_RECEIVERS.lock().await;
        receivers.insert(subagent_id.clone(), rx);
        let mut sessions = SUBAGENT_SESSIONS.lock().await;
        sessions.insert(subagent_id.clone(), session);
    }
    prune_subagent_sessions_cache().await;

    let mut owned_ctx = ctx.to_owned();
    owned_ctx.abort_handle = Arc::clone(&sub_abort_handle);
    let sub_id = subagent_id.clone();
    let title_s = title.to_string();
    let role_s = role.to_string();
    let prompt_s = prompt.to_string();

    let join_h = tokio::spawn(async move {
        let _ = run_inprocess_subagent(
            owned_ctx,
            title_s,
            role_s,
            prompt_s,
            requested_tools,
            Some(sub_id),
        ).await;
    });
    *sub_join_slot.lock().await = Some(join_h);

    // Register into state.active_agents so abort_agent can track/abort it
    if let Some(ref app) = ctx.app_handle {
        if let Some(state_ref) = app.try_state::<crate::commands::AppState>() {
            if let Ok(mut map) = state_ref.active_agents.lock() {
                map.insert(
                    subagent_id.clone(),
                    crate::commands::ActiveAgentHandle {
                        abort_flag: Arc::clone(&sub_abort_handle),
                        join_handle: Arc::clone(&sub_join_slot),
                    },
                );
            }
        }
    }

    Ok(serde_json::to_string(&serde_json::json!({
        "subagent_id": subagent_id,
        "status": "running"
    }))?)
}

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

        if let Some((resolved_id, rx)) = rx_opt {
            match rx.await {
                Ok(report) => {
                    let mut sessions = SUBAGENT_SESSIONS.lock().await;
                    sessions.remove(&resolved_id);
                    collected_reports.push((resolved_id, report));
                    continue;
                }
                Err(_) => {
                    // 1. Check SUBAGENT_SESSIONS for completed report or aborted status
                    if let Some((_, session)) = find_subagent_session(task_id, &resolved_id).await {
                        let info = session.retry_info.lock().await;
                        if let Some(ref rep) = info.report {
                            let mut sessions = SUBAGENT_SESSIONS.lock().await;
                            sessions.remove(&resolved_id);
                            collected_reports.push((resolved_id, rep.clone()));
                            continue;
                        }
                        if info.status == "aborted" || session.abort_handle.load(Ordering::SeqCst) {
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

                    let is_aborted = if let Some((_, session)) = find_subagent_session(task_id, &resolved_id).await {
                        let info = session.retry_info.lock().await;
                        info.status == "aborted" || session.abort_handle.load(Ordering::SeqCst)
                    } else {
                        false
                    };
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
            let info = session.retry_info.lock().await;
            if let Some(ref rep) = info.report {
                collected_reports.push((resolved_id, rep.clone()));
                continue;
            }
            if info.status == "aborted" || session.abort_handle.load(Ordering::SeqCst) {
                collected_reports.push((resolved_id, "[Субагент прерван пользователем / Subagent aborted by user]".to_string()));
                continue;
            }
            if let Some(db_report) = find_subagent_report_from_db(repo_root, task_id, &resolved_id)
                .or_else(|| find_subagent_report_from_db(repo_root, task_id, subagent_id))
            {
                collected_reports.push((resolved_id, db_report));
                continue;
            }
            if info.status == "error" {
                let err = info.last_error.as_deref().unwrap_or("Unknown error");
                bail!("Subagent [{}] failed with error: {}", resolved_id, err);
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
            .map(|(id, report)| format!("### Subagent [{}]
{}", id, report))
            .collect::<Vec<_>>()
            .join("

---

"))
    }
}

pub async fn execute_subagent_list(
    ctx: &ToolContext<'_>,
    args: &serde_json::Value,
) -> Result<String> {
    let status_filter = args
        .get("status_filter")
        .and_then(|v| v.as_str())
        .unwrap_or("all")
        .trim()
        .to_lowercase();

    let target_task_id = ctx.task_id.unwrap_or("");
    let mut list: Vec<serde_json::Value> = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    // 1. Scan in-memory SUBAGENT_SESSIONS
    {
        let sessions = SUBAGENT_SESSIONS.lock().await;
        for (sid, sess) in sessions.iter() {
            let info = sess.retry_info.lock().await;
            if !target_task_id.is_empty() {
                let matches_parent = info.parent_task_id == target_task_id
                    || sid.starts_with(&format!("{}.", target_task_id))
                    || sid.starts_with(target_task_id);
                if !matches_parent {
                    continue;
                }
            }

            let raw_status = if sess.abort_handle.load(Ordering::SeqCst) || info.status == "aborted" {
                "aborted"
            } else if info.status == "error" {
                "error"
            } else if info.status == "finished" || info.status == "completed" {
                "completed"
            } else if info.status == "running" {
                "running"
            } else {
                info.status.as_str()
            };

            let norm_status = match raw_status {
                "finished" => "completed",
                s => s,
            };

            let matches_filter = match status_filter.as_str() {
                "all" | "" => true,
                "running" => norm_status == "running",
                "completed" => norm_status == "completed",
                "aborted" => norm_status == "aborted",
                "error" => norm_status == "error",
                f => norm_status == f,
            };

            if matches_filter {
                let mut item = serde_json::json!({
                    "subagent_id": &info.subagent_id,
                    "title": &info.title,
                    "role": &info.role,
                    "status": norm_status,
                    "steps": info.steps,
                });
                if let Some(dur) = info.duration_ms {
                    item["duration_ms"] = serde_json::json!(dur.round() as u64);
                }
                list.push(item);
            }
            seen_ids.insert(info.subagent_id.clone());
            seen_ids.insert(sid.clone());
        }
    }

    // 2. Check SQLite dialog_items for persisted subagents not in active sessions
    if !target_task_id.is_empty() {
        if let Ok(dialog_keys) = crate::db::list_subagent_dialog_keys(ctx.repo_root, target_task_id) {
            for key in dialog_keys {
                let sub_id = if key.contains(":sub:") {
                    key.rsplit(":sub:").next().unwrap_or(&key).to_string()
                } else {
                    key.clone()
                };

                if seen_ids.contains(&sub_id) || seen_ids.contains(&key) {
                    continue;
                }

                if let Ok(items) = crate::db::load_dialog_items(ctx.repo_root, &key) {
                    if items.is_empty() {
                        continue;
                    }

                    let mut title = "Subagent".to_string();
                    let mut role = "general".to_string();
                    let mut has_error = false;
                    let mut has_abort = false;
                    let mut steps = 0;
                    let mut duration_ms: Option<f64> = None;

                    for it in &items {
                        if it.item_type == "tool_call" {
                            steps += 1;
                        }
                        if it.item_type == "output" {
                            if let Some(dur) = it.duration_ms {
                                duration_ms = Some(dur);
                            }
                            if let Some(ref c) = it.content {
                                if let Some(idx_sub) = c.find("Subagent [") {
                                    let after = &c[idx_sub + 10..];
                                    if let Some(idx_bracket) = after.find("] '") {
                                        role = after[..idx_bracket].to_string();
                                        let after_title = &after[idx_bracket + 3..];
                                        if let Some(idx_quote) = after_title.find("' report:") {
                                            title = after_title[..idx_quote].to_string();
                                        }
                                    }
                                }
                            }
                        }
                        if it.item_type == "prompt" {
                            if let Some(ref tool_meta) = it.tool {
                                if let Some(t) = tool_meta.get("title").and_then(|v| v.as_str()) {
                                    title = t.to_string();
                                }
                                if let Some(r) = tool_meta.get("role").and_then(|v| v.as_str()) {
                                    role = r.to_string();
                                }
                            }
                        }
                        if it.item_type == "error" {
                            has_error = true;
                            if it.content.as_ref().map(|c| c.contains("aborted") || c.contains("прерван")).unwrap_or(false) {
                                has_abort = true;
                            }
                        }
                    }

                    let norm_status = if has_abort {
                        "aborted"
                    } else if has_error {
                        "error"
                    } else {
                        "completed"
                    };

                    let matches_filter = match status_filter.as_str() {
                        "all" | "" => true,
                        "running" => norm_status == "running",
                        "completed" => norm_status == "completed",
                        "aborted" => norm_status == "aborted",
                        "error" => norm_status == "error",
                        f => norm_status == f,
                    };

                    if matches_filter {
                        let mut item = serde_json::json!({
                            "subagent_id": &sub_id,
                            "title": &title,
                            "role": &role,
                            "status": norm_status,
                            "steps": steps,
                        });
                        if let Some(dur) = duration_ms {
                            item["duration_ms"] = serde_json::json!(dur.round() as u64);
                        }
                        list.push(item);
                    }
                    seen_ids.insert(sub_id);
                    seen_ids.insert(key);
                }
            }
        }
    }

    Ok(serde_json::to_string_pretty(&list)?)
}

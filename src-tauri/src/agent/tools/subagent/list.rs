use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::agent::{SubagentRetryInfo, ToolContext, SUBAGENT_SESSIONS};

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

    // 1. Scan in-memory SUBAGENT_SESSIONS using snapshot pattern
    {
        let session_snapshots: Vec<(String, Arc<tokio::sync::Mutex<SubagentRetryInfo>>, Arc<AtomicBool>)> = {
            let sessions = SUBAGENT_SESSIONS.lock().await;
            sessions
                .iter()
                .map(|(sid, sess)| (sid.clone(), Arc::clone(&sess.retry_info), Arc::clone(&sess.abort_handle)))
                .collect()
        };

        for (sid, retry_info_lock, abort_handle) in session_snapshots {
            let info = retry_info_lock.lock().await;
            if !target_task_id.is_empty() {
                let matches_parent = info.parent_task_id == target_task_id
                    || sid.starts_with(&format!("{}.", target_task_id))
                    || sid.starts_with(target_task_id);
                if !matches_parent {
                    continue;
                }
            }

            let raw_status = if abort_handle.load(Ordering::SeqCst) || info.status == "aborted" {
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
                    "persona": &info.role,
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
                                if let Some(r) = tool_meta.get("persona").or_else(|| tool_meta.get("role")).and_then(|v| v.as_str()) {
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
                            "persona": &role,
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

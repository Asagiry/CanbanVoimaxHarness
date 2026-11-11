use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::Emitter;
use crate::agent::react_engine::{detect_degenerate_command_or_loop, normalize_tool_call_for_circuit_breaker};
use crate::agent::subagent_runtime::{
    are_questions_duplicate, canonical_tool_name, should_abort_tool_batch, OwnedToolContext,
    ToolContext,
};
use crate::agent::tool_dispatch::execute_tool;
use crate::db::ActivityItem;
use crate::llm::{ChatMessage, ToolCall};

pub async fn execute_subagent_turn_tools(
    ctx: &OwnedToolContext,
    role: &str,
    effective_sub_tools: &[String],
    subagent_id: &str,
    parent_id: &str,
    subagent_dialog_id: &str,
    sub_turn: usize,
    turn_tool_calls: &[ToolCall],
    sub_messages: &mut Vec<ChatMessage>,
    recent_tool_calls: &mut Vec<(String, String)>,
    base_url: &str,
    api_key: Option<&str>,
    model: &str,
    reasoning_effort: Option<&str>,
) {
    let mut batch_aborted_by_error: Option<String> = None;
    let mut executed_questions_in_turn: Vec<(String, String, String)> = Vec::new();
    let total_in_batch = turn_tool_calls.len();
    if total_in_batch > 1 {
        for (batch_idx, tc) in turn_tool_calls.iter().enumerate() {
            let batch_info = Some(serde_json::json!({
                "index": batch_idx + 1,
                "total": total_in_batch
            }));
            let tool_item_id = format!("sub-tool-{}", tc.id);
            let tool_exec_queued = serde_json::json!({
                "id": &tc.id,
                "toolName": &tc.function.name,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "durationMs": 0,
                "status": "queued",
                "params": serde_json::from_str(&tc.function.arguments).unwrap_or_else(|_| serde_json::json!({})),
                "batch": &batch_info,
            });
            let tool_act_queued = ActivityItem {
                id: tool_item_id,
                item_type: "tool".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                content: None,
                tool: Some(tool_exec_queued),
                duration_ms: None,
                images: None,
            };
            if let Some(ref app) = ctx.app_handle {
                let _ = app.emit("subagent:activity", serde_json::json!({
                    "parentTaskId": &parent_id,
                    "subagentId": &subagent_id,
                    "activity": &tool_act_queued,
                }));
            }
        }
    }
    for (batch_idx, tc) in turn_tool_calls.iter().enumerate() {
        let batch_info = if total_in_batch > 1 {
            Some(serde_json::json!({
                "index": batch_idx + 1,
                "total": total_in_batch
            }))
        } else {
            None
        };

        if ctx.abort_handle.load(Ordering::SeqCst) {
            break;
        }

        if let Some(ref failed_tool) = batch_aborted_by_error {
            let skip_msg = format!("Skipped: previous tool '{}' in this batch failed", failed_tool);
            let tool_item_id = format!("sub-tool-{}", tc.id);
            let tool_exec_skipped = serde_json::json!({
                "id": &tc.id,
                "toolName": &tc.function.name,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "durationMs": 0,
                "status": "failed",
                "params": serde_json::from_str(&tc.function.arguments).unwrap_or_else(|_| serde_json::json!({})),
                "stdout": &skip_msg,
                "batch": &batch_info,
            });
            let tool_act_skipped = ActivityItem {
                id: tool_item_id,
                item_type: "tool".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                content: None,
                tool: Some(tool_exec_skipped),
                duration_ms: Some(0.0),
                images: None,
            };
            if let Err(e) = crate::db::append_dialog_items(&ctx.repo_root, subagent_dialog_id, std::slice::from_ref(&tool_act_skipped)) {
                eprintln!("CVH [WARN]: Failed to append subagent skipped tool dialog item: {}", e);
            }
            if let Some(ref app) = ctx.app_handle {
                let _ = app.emit("subagent:activity", serde_json::json!({
                    "parentTaskId": &parent_id,
                    "subagentId": &subagent_id,
                    "activity": &tool_act_skipped,
                }));
            }
            sub_messages.push(ChatMessage::tool_response(tc.id.clone(), skip_msg));
            continue;
        }

        let parsed_args: serde_json::Value =
            serde_json::from_str(&tc.function.arguments).unwrap_or_else(|_| serde_json::json!({}));

        if canonical_tool_name(&tc.function.name) == "question" {
            let qtext = parsed_args
                .get("text")
                .or_else(|| parsed_args.get("question"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if !qtext.is_empty() {
                if let Some((prev_id, _, prev_result)) = executed_questions_in_turn
                    .iter()
                    .find(|(_, prev_text, _)| are_questions_duplicate(prev_text, qtext))
                {
                    eprintln!(
                        "CVH [agent]: Deduplicated question call '{}' (subagent turn) in turn {} (matches earlier call '{}')",
                        tc.id, sub_turn, prev_id
                    );
                    let tool_item_id = format!("sub-tool-{}", tc.id);
                    let dedup_done = serde_json::json!({
                        "id": &tc.id,
                        "toolName": &tc.function.name,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "durationMs": 0,
                        "status": "success",
                        "params": &parsed_args,
                        "stdout": prev_result,
                        "batch": &batch_info,
                    });
                    let dedup_act = ActivityItem {
                        id: tool_item_id,
                        item_type: "tool".to_string(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        content: None,
                        tool: Some(dedup_done),
                        duration_ms: Some(0.0),
                        images: None,
                    };
                    if let Err(e) = crate::db::append_dialog_items(&ctx.repo_root, subagent_dialog_id, std::slice::from_ref(&dedup_act)) {
                        eprintln!("CVH [WARN]: Failed to append subagent dedup tool dialog item: {}", e);
                    }
                    if let Some(ref app) = ctx.app_handle {
                        let _ = app.emit("subagent:activity", serde_json::json!({
                            "parentTaskId": &parent_id,
                            "subagentId": &subagent_id,
                            "activity": &dedup_act,
                        }));
                    }
                    sub_messages.push(ChatMessage::tool_response(tc.id.clone(), prev_result.clone()));
                    continue;
                }
            }
        }

        let tool_item_id = format!("sub-tool-{}", tc.id);
        let start_time = chrono::Utc::now().timestamp_millis();
        let tool_exec_running = serde_json::json!({
            "id": &tc.id,
            "toolName": &tc.function.name,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "durationMs": 0,
            "status": "running",
            "params": &parsed_args,
            "batch": &batch_info,
        });
        let tool_act_running = ActivityItem {
            id: tool_item_id.clone(),
            item_type: "tool".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            content: None,
            tool: Some(tool_exec_running),
            duration_ms: None,
            images: None,
        };
        if let Err(e) = crate::db::append_dialog_items(&ctx.repo_root, subagent_dialog_id, std::slice::from_ref(&tool_act_running)) {
            eprintln!("CVH [WARN]: Failed to append subagent running tool dialog item: {}", e);
        }
        if let Some(ref app) = ctx.app_handle {
            let _ = app.emit("subagent:activity", serde_json::json!({
                "parentTaskId": parent_id,
                "subagentId": &subagent_id,
                "activity": &tool_act_running,
            }));
        }

        let sub_tool_ctx = ToolContext {
            repo_root: &ctx.repo_root,
            worktree_dir: &ctx.worktree_dir,
            stage_id: ctx.stage_id.as_deref(),
            role: Some(role),
            abort_handle: Arc::clone(&ctx.abort_handle),
            tool_policy: &ctx.tool_policy,
            task_id: Some(subagent_id),
            agent_allowed_tools: Some(effective_sub_tools),
            app_handle: ctx.app_handle.clone(),
            base_url: Some(base_url),
            api_key,
            model: Some(model),
            temperature: ctx.temperature,
            reasoning_effort,
            call_id: Some(&tc.id),
        };

        let exec_res = if let Some(cb_msg) = detect_degenerate_command_or_loop(
            &tc.function.name,
            &tc.function.arguments,
            recent_tool_calls,
        ) {
            Err(anyhow::anyhow!("{}", cb_msg))
        } else {
            execute_tool(&sub_tool_ctx, &tc.function.name, &parsed_args).await
        };
        recent_tool_calls.push(normalize_tool_call_for_circuit_breaker(
            &tc.function.name,
            &tc.function.arguments,
        ));
        if recent_tool_calls.len() > 16 {
            recent_tool_calls.remove(0);
        }
        if ctx.abort_handle.load(Ordering::SeqCst) {
            let duration_ms = (chrono::Utc::now().timestamp_millis() - start_time) as f64;
            let tool_exec_aborted = serde_json::json!({
                "id": &tc.id,
                "toolName": &tc.function.name,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "durationMs": duration_ms,
                "status": "failed",
                "params": &parsed_args,
                "stderr": "Прервано пользователем",
                "stdout": "[Прервано пользователем / Aborted by user]",
                "batch": &batch_info,
            });
            let tool_act_aborted = ActivityItem {
                id: tool_item_id,
                item_type: "tool".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                content: Some("Прервано пользователем".to_string()),
                tool: Some(tool_exec_aborted),
                duration_ms: Some(duration_ms),
                images: None,
            };
            if let Err(e) = crate::db::append_dialog_items(&ctx.repo_root, subagent_dialog_id, std::slice::from_ref(&tool_act_aborted)) {
                eprintln!("CVH [WARN]: Failed to append subagent aborted tool dialog item: {}", e);
            }
            if let Some(ref app) = ctx.app_handle {
                let _ = app.emit("subagent:activity", serde_json::json!({
                    "parentTaskId": &parent_id,
                    "subagentId": &subagent_id,
                    "activity": &tool_act_aborted,
                }));
            }
            break;
        }
        let duration_ms = (chrono::Utc::now().timestamp_millis() - start_time) as f64;

        let (res_str, is_success) = match exec_res {
            Ok(s) => (s, true),
            Err(e) => (format!("Error executing {}: {:#}", tc.function.name, e), false),
        };

        let tool_exec_done = serde_json::json!({
            "id": &tc.id,
            "toolName": &tc.function.name,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "durationMs": duration_ms,
            "status": if is_success { "success" } else { "failed" },
            "params": &parsed_args,
            "stdout": &res_str,
            "batch": &batch_info,
        });
        let tool_act_done = ActivityItem {
            id: tool_item_id,
            item_type: "tool".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            content: None,
            tool: Some(tool_exec_done),
            duration_ms: Some(duration_ms),
            images: None,
        };
        if let Err(e) = crate::db::append_dialog_items(&ctx.repo_root, subagent_dialog_id, std::slice::from_ref(&tool_act_done)) {
            eprintln!("CVH [WARN]: Failed to append subagent completed tool dialog item: {}", e);
        }
        if let Some(ref app) = ctx.app_handle {
            let _ = app.emit("subagent:activity", serde_json::json!({
                "parentTaskId": &parent_id,
                "subagentId": &subagent_id,
                "activity": &tool_act_done,
            }));
        }

        if should_abort_tool_batch(&tc.function.name, is_success, &res_str) {
            batch_aborted_by_error = Some(tc.function.name.clone());
        }

        if canonical_tool_name(&tc.function.name) == "question" && is_success {
            let qtext = parsed_args
                .get("text")
                .or_else(|| parsed_args.get("question"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if !qtext.is_empty() {
                executed_questions_in_turn.push((tc.id.clone(), qtext, res_str.clone()));
            }
        }

        sub_messages.push(ChatMessage::tool_response(tc.id.clone(), res_str));
    }
}

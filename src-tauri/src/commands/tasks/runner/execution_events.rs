use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager};

use crate::agent::AgentEvent;
use crate::commands::state::AppState;
use crate::commands::tasks::storage::{load_tasks_from_disk, save_tasks_to_disk};

pub fn build_agent_event_sink(
    app: AppHandle,
    tid: String,
    root: PathBuf,
    model: String,
) -> impl Fn(AgentEvent) + Send + Sync + 'static {
    let active_stream_id = std::sync::Mutex::new(None::<String>);
    move |event: AgentEvent| {
        match event {
            AgentEvent::StreamStart { id } => {
                *active_stream_id.lock().unwrap_or_else(|p| p.into_inner()) = Some(id.clone());
                let _ = app.emit("agent:stream_start", serde_json::json!({
                    "taskId": tid, "streamId": id,
                }));
            }
            AgentEvent::StreamRetry { id, delay_secs } => {
                let _ = app.emit("agent:stream_retry", serde_json::json!({
                    "taskId": tid, "streamId": id, "delaySecs": delay_secs,
                }));
                *active_stream_id.lock().unwrap_or_else(|p| p.into_inner()) = None;
            }
            AgentEvent::Token(token) => {
                let payload = serde_json::json!({
                    "taskId": tid,
                    "streamId": active_stream_id.lock().unwrap_or_else(|p| p.into_inner()).clone(),
                    "output": &token,
                    "content": &token,
                    "token": &token
                });
                let _ = app.emit("agent:output", &payload);
            }
            AgentEvent::Output(output) => {
                let payload = serde_json::json!({
                    "taskId": tid,
                    "streamId": active_stream_id.lock().unwrap_or_else(|p| p.into_inner()).clone(),
                    "output": &output,
                    "content": &output,
                    "token": &output
                });
                let _ = app.emit("agent:output", &payload);
            }
            AgentEvent::Thought(thought) => {
                let _ = app.emit(
                    "agent:thought",
                    serde_json::json!({ "taskId": tid, "streamId": active_stream_id.lock().unwrap_or_else(|p| p.into_inner()).clone(), "thought": &thought, "delta": &thought }),
                );
            }
            AgentEvent::ToolQueued { id, name, arguments, batch } => {
                let _ = app.emit(
                    "agent:tool_call",
                    serde_json::json!({
                        "taskId": tid,
                        "id": id,
                        "name": name,
                        "arguments": arguments,
                        "batch": batch,
                        "status": "queued",
                    }),
                );
            }
            AgentEvent::ToolCall { id, name, arguments, batch } => {
                let _ = app.emit(
                    "agent:tool_call",
                    serde_json::json!({
                        "taskId": tid,
                        "id": id,
                        "name": name,
                        "arguments": arguments,
                        "batch": batch,
                        "status": "running",
                    }),
                );

                let parsed_args = if arguments.trim().is_empty() {
                    serde_json::json!({})
                } else {
                    serde_json::from_str::<serde_json::Value>(&arguments)
                        .unwrap_or_else(|_| serde_json::json!({ "raw": &arguments }))
                };
                let mut tool_obj = serde_json::json!({
                    "id": &id,
                    "toolName": &name,
                    "name": &name,
                    "arguments": &arguments,
                    "params": parsed_args,
                    "status": "running",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "durationMs": 0,
                });
                if let Some(ref b) = batch {
                    tool_obj["batch"] = b.clone();
                }
                let tool_item = crate::db::ActivityItem {
                    id: id.clone(),
                    item_type: "tool".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    content: None,
                    tool: Some(tool_obj),
                    duration_ms: None,
                    images: None,
                };
                if let Err(e) = crate::db::append_dialog_items(&root, &tid, std::slice::from_ref(&tool_item)) {
                    eprintln!("CVH [WARN]: Failed to append tool dialog item: {}", e);
                }
            }
            AgentEvent::ToolResult { id, name, result, success, duration_ms, batch } => {
                let sanitized_result = if result.len() > 65536 {
                    let half = 32768;
                    let head_end = result
                        .char_indices()
                        .map(|(i, _)| i)
                        .take_while(|&i| i <= half)
                        .last()
                        .unwrap_or(0);
                    let tail_target = result.len() - half;
                    let tail_start = result
                        .char_indices()
                        .map(|(i, _)| i)
                        .find(|&i| i >= tail_target)
                        .unwrap_or(result.len());
                    format!(
                        "{}\n\n... [IPC payload truncated ({} bytes total). Full output in SQLite] ...\n\n{}",
                        &result[..head_end],
                        result.len(),
                        &result[tail_start..]
                    )
                } else {
                    result.clone()
                };
                let _ = app.emit(
                    "agent:tool_result",
                    serde_json::json!({
                        "taskId": tid,
                        "id": id,
                        "name": name,
                        "result": &sanitized_result,
                        "success": success,
                        "durationMs": duration_ms,
                        "batch": batch,
                    }),
                );

                let status_str = if success { "success" } else { "failed" };
                let mut tool_obj = crate::db::db_get_dialog_tool_json(&root, &tid, &id)
                    .unwrap_or_else(|| serde_json::json!({
                        "id": &id,
                        "toolName": &name,
                        "name": &name,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }));

                if let Some(obj) = tool_obj.as_object_mut() {
                    obj.insert("status".to_string(), serde_json::json!(status_str));
                    obj.insert("stdout".to_string(), serde_json::json!(&result));
                    obj.insert("result".to_string(), serde_json::json!(&result));
                    obj.insert("durationMs".to_string(), serde_json::json!(duration_ms.unwrap_or(0.0)));
                    if !success {
                        obj.insert("stderr".to_string(), serde_json::json!(&result));
                    }
                    if let Some(ref b) = batch {
                        obj.insert("batch".to_string(), b.clone());
                    }
                }

                let tool_json_str = serde_json::to_string(&tool_obj).unwrap_or_default();
                let _ = crate::db::db_update_dialog_tool_result(&root, &tid, &id, &tool_json_str, duration_ms);
            }
            AgentEvent::TurnStart { turn } => {
                let _ = app.emit(
                    "agent:turn_start",
                    serde_json::json!({ "taskId": tid, "turn": turn }),
                );
            }
            AgentEvent::Telemetry {
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cache_creation_tokens,
                cache_hit_rate,
                duration_ms,
                tps,
                ttft_ms,
                system_tokens,
                tool_tokens,
            } => {
                let _ = app.emit(
                    "agent:telemetry",
                    serde_json::json!({
                        "taskId": tid,
                        "inputTokens": input_tokens,
                        "outputTokens": output_tokens,
                        "cacheReadTokens": cache_read_tokens,
                        "cacheCreationTokens": cache_creation_tokens,
                        "cacheHitRate": cache_hit_rate,
                        "durationMs": duration_ms,
                        "tps": tps,
                        "ttftMs": ttft_ms,
                        "systemTokens": system_tokens,
                        "toolTokens": tool_tokens,
                    }),
                );

                let telem_record = crate::db::TelemetryRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    task_id: tid.clone(),
                    model: Some(model.clone()),
                    input_tokens: input_tokens as i64,
                    output_tokens: output_tokens as i64,
                    cached_tokens: cache_read_tokens as i64,
                    tps: tps as f64,
                    ttft_ms,
                    duration_ms,
                    created_at: chrono::Utc::now().timestamp_millis(),
                };
                let _ = crate::db::db_record_telemetry(&root, &telem_record);
            }
            AgentEvent::Compacted {
                saved_chars,
                estimated_saved_tokens,
                evaporated_count,
                tokens_before,
                tokens_after,
            } => {
                let _ = app.emit(
                    "agent:compacted",
                    serde_json::json!({
                        "taskId": tid,
                        "savedChars": saved_chars,
                        "estimatedSavedTokens": estimated_saved_tokens,
                        "saved_tokens": estimated_saved_tokens,
                        "evaporatedCount": evaporated_count,
                        "tokens_before": tokens_before,
                        "tokens_after": tokens_after,
                        "tokensBefore": tokens_before,
                        "tokensAfter": tokens_after,
                    }),
                );
            }
            AgentEvent::Finished { final_response } => {
                let resp_id = format!("resp-{}", uuid::Uuid::new_v4());
                let _ = app.emit(
                    "agent:finished",
                    serde_json::json!({
                        "taskId": tid,
                        "status": "finished",
                        "response": &final_response,
                        "id": &resp_id
                    }),
                );
                if !final_response.trim().is_empty() {
                    let resp_item = crate::db::ActivityItem {
                        id: resp_id,
                        item_type: "output".to_string(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        content: Some(final_response.clone()),
                        tool: None,
                        duration_ms: None,
                        images: None,
                    };
                    if let Err(e) = crate::db::append_dialog_items(&root, &tid, std::slice::from_ref(&resp_item)) {
                        eprintln!("CVH [WARN]: Failed to append response dialog item: {}", e);
                    }
                }
            }
            AgentEvent::TaskUpdated { task_id, todos } => {
                let _ = app.emit(
                    "task:updated",
                    serde_json::json!({
                        "taskId": &task_id,
                        "todos": &todos,
                    }),
                );
                let _ = app.emit(
                    "tasks:updated",
                    serde_json::json!({ "taskId": &task_id }),
                );
            }
            AgentEvent::Aborted => {
                let _ = app.emit(
                    "agent:aborted",
                    serde_json::json!({ "taskId": tid }),
                );
                let _ = app.emit(
                    "agent:status",
                    serde_json::json!({ "taskId": tid, "status": "aborted" }),
                );
            }
            AgentEvent::Error(err) => {
                eprintln!("CVH [agent]: {}", err);
            }
        }
    }
}

pub async fn handle_agent_run_completion(
    result: anyhow::Result<String>,
    tid: String,
    root_task: PathBuf,
    app: AppHandle,
    abort_flag_task: Arc<AtomicBool>,
    session_id_task: String,
) {
    let project_path_str = root_task.to_string_lossy().to_string();

    match &result {
        Ok(final_response) => {
            crate::logger::cvh_log_with_details(
                Some(&root_task),
                "INFO",
                "RUN_AGENT",
                &format!("run_agent: agent loop finished successfully for task '{}'", tid),
                Some(&format!("Final response length: {} chars", final_response.len())),
            );
            let _ = app.emit(
                "agent:status",
                serde_json::json!({ "taskId": tid, "status": "finished", "projectPath": &project_path_str }),
            );

            if let Some(state_ref) = app.try_state::<AppState>() {
                state_ref.remove_agent_context(&root_task, &tid);
            }

            let mut tasks = load_tasks_from_disk(&root_task).await;
            if let Some(task_idx) = tasks.iter().position(|t| t.id == tid) {
                let mut task_changed = false;

                if tasks[task_idx].status.as_deref() != Some("done") {
                    tasks[task_idx].status = Some("idle".to_string());
                    tasks[task_idx].updated_at = Utc::now().timestamp_millis();
                    task_changed = true;
                }

                if task_changed {
                    let _ = crate::db::db_upsert_task(&root_task, &tasks[task_idx]);
                    let _ = save_tasks_to_disk(&root_task, &tasks).await;
                    if let Some(state_ref) = app.try_state::<AppState>() {
                        if state_ref.get_root() == root_task {
                            let mut guard = state_ref.tasks.lock().unwrap_or_else(|p| p.into_inner());
                            *guard = tasks.clone();
                        }
                    }
                    let _ = app.emit("task:updated", serde_json::json!({ "task": &tasks[task_idx], "projectPath": &project_path_str }));
                    let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks, "projectPath": &project_path_str }));
                    let _ = app.emit(
                        "agent:status",
                        serde_json::json!({ "taskId": tid, "status": "idle", "projectPath": &project_path_str }),
                    );
                }
            }
        }
        Err(err) => {
            let is_aborted = abort_flag_task.load(Ordering::SeqCst);
            let err_str = err.to_string();

            if let Some(state_ref) = app.try_state::<AppState>() {
                state_ref.remove_agent_context(&root_task, &tid);
            }

            if is_aborted {
                crate::logger::cvh_log(
                    Some(&root_task),
                    "INFO",
                    "RUN_AGENT",
                    &format!("run_agent: agent loop ABORTED by user for task '{}'", tid),
                );
                eprintln!("CVH [run_agent]: Agent execution aborted by user for task {}", tid);
                let _ = app.emit(
                    "agent:status",
                    serde_json::json!({ "taskId": tid, "status": "idle", "projectPath": &project_path_str }),
                );
                let mut tasks = load_tasks_from_disk(&root_task).await;
                if let Some(task_idx) = tasks.iter().position(|t| t.id == tid) {
                    if tasks[task_idx].status.as_deref() != Some("done") {
                        tasks[task_idx].status = Some("idle".to_string());
                        tasks[task_idx].updated_at = Utc::now().timestamp_millis();
                        let _ = crate::db::db_upsert_task(&root_task, &tasks[task_idx]);
                    }
                    let parent_id = tasks[task_idx].parent_id.clone();
                    if let Some(parent_idx) = tasks.iter().position(|t| {
                        t.waiting_for_subtask_id.as_deref().map(|s| s.eq_ignore_ascii_case(&tid)).unwrap_or(false)
                            || parent_id.as_deref().map(|pid| t.id.eq_ignore_ascii_case(pid) && t.waiting_for_subtask_id.is_some()).unwrap_or(false)
                    }) {
                        tasks[parent_idx].waiting_for_subtask_id = None;
                        if tasks[parent_idx].status.as_deref() == Some("waiting_subtask") {
                            tasks[parent_idx].status = Some("idle".to_string());
                        }
                        tasks[parent_idx].updated_at = Utc::now().timestamp_millis();
                        let _ = crate::db::db_upsert_task(&root_task, &tasks[parent_idx]);
                        let _ = app.emit("task:updated", serde_json::json!({ "task": &tasks[parent_idx], "projectPath": &project_path_str }));
                    }
                    let _ = save_tasks_to_disk(&root_task, &tasks).await;
                    if let Some(state_ref) = app.try_state::<AppState>() {
                        if state_ref.get_root() == root_task {
                            let mut guard = state_ref.tasks.lock().unwrap_or_else(|p| p.into_inner());
                            *guard = tasks.clone();
                        }
                    }
                    let _ = app.emit("task:updated", serde_json::json!({ "task": &tasks[task_idx], "projectPath": &project_path_str }));
                    let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks, "projectPath": &project_path_str }));
                }
            } else {
                eprintln!("CVH [run_agent]: Agent execution failed for task {}: {}", tid, err_str);
                let _ = app.emit(
                    "agent:status",
                    serde_json::json!({ "taskId": tid, "status": "error", "error": &err_str, "projectPath": &project_path_str }),
                );
                let _ = app.emit(
                    "agent:error",
                    serde_json::json!({ "taskId": tid, "error": &err_str, "projectPath": &project_path_str }),
                );

                let mut tasks = load_tasks_from_disk(&root_task).await;
                if let Some(task_idx) = tasks.iter().position(|t| t.id == tid) {
                    tasks[task_idx].status = Some("error".to_string());
                    tasks[task_idx].updated_at = Utc::now().timestamp_millis();
                    let _ = crate::db::db_upsert_task(&root_task, &tasks[task_idx]);
                    let parent_id = tasks[task_idx].parent_id.clone();
                    if let Some(parent_idx) = tasks.iter().position(|t| {
                        t.waiting_for_subtask_id.as_deref().map(|s| s.eq_ignore_ascii_case(&tid)).unwrap_or(false)
                            || parent_id.as_deref().map(|pid| t.id.eq_ignore_ascii_case(pid) && t.waiting_for_subtask_id.is_some()).unwrap_or(false)
                    }) {
                        tasks[parent_idx].waiting_for_subtask_id = None;
                        if tasks[parent_idx].status.as_deref() == Some("waiting_subtask") {
                            tasks[parent_idx].status = Some("idle".to_string());
                        }
                        tasks[parent_idx].updated_at = Utc::now().timestamp_millis();
                        let _ = crate::db::db_upsert_task(&root_task, &tasks[parent_idx]);
                        let _ = app.emit("task:updated", serde_json::json!({ "task": &tasks[parent_idx], "projectPath": &project_path_str }));
                    }
                    let _ = save_tasks_to_disk(&root_task, &tasks).await;
                    if let Some(state_ref) = app.try_state::<AppState>() {
                        if state_ref.get_root() == root_task {
                            let mut guard = state_ref.tasks.lock().unwrap_or_else(|p| p.into_inner());
                            *guard = tasks.clone();
                        }
                    }
                    let _ = app.emit("task:updated", serde_json::json!({ "task": &tasks[task_idx], "projectPath": &project_path_str }));
                    let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks, "projectPath": &project_path_str }));
                }
            }
        }
    }

    if let Some(state_ref) = app.try_state::<AppState>() {
        state_ref.session_registry.update_status(
            &session_id_task,
            crate::agent::session_registry::SessionStatus::Finished,
        );
        if let Some(summary) = state_ref.session_registry.get_session(&session_id_task) {
            let _ = app.emit("agent:session_status", &summary);
        }
        state_ref.remove_active_agent_if_same(&root_task, &tid, &abort_flag_task);
        state_ref.remove_agent_context(&root_task, &tid);
        crate::logger::cvh_log(
            Some(&root_task),
            "INFO",
            "RUN_AGENT",
            &format!("run_agent: active agent handle and context removed for task '{}'", tid),
        );
    }
}

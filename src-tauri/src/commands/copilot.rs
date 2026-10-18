use crate::agent::{execute_tool, get_standard_tools, ThinkingStreamParser, ToolContext};
use crate::commands::state::{AgentEntry, AppState};
use crate::db::ActivityItem;
use crate::llm::{ChatMessage, LlmClient, OpenAIChatRequest, StreamChunk, ToolCall, ToolDefinition};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

pub const COPILOT_DIALOG_TASK_ID: &str = "copilot:meta-chat";

/// Returns the 7 standard architectural tools permitted for CVH Copilot (Project Configuration Specialist),
/// sorted alphabetically for deterministic prefix caching.
pub fn get_copilot_tool_suite() -> Vec<ToolDefinition> {
    const COPILOT_TOOLS: &[&str] = &[
        "read_file",
        "write_file",
        "edit_file",
        "glob",
        "grep",
        "execute_command",
        "web_search",
    ];
    let mut tools: Vec<ToolDefinition> = get_standard_tools()
        .into_iter()
        .filter(|t| COPILOT_TOOLS.contains(&t.function.name.as_str()))
        .collect();
    tools.sort_by(|a, b| a.function.name.cmp(&b.function.name));
    tools
}

/// Pre-compiles the Meta-Architect persona and gathers full project workspace context
/// including boards, pipeline columns, and existing tasks.
pub async fn compile_copilot_system_prompt(repo_root: &Path) -> String {
    let persona_file = repo_root
        .join(".cvh")
        .join("agents")
        .join("cvh-dev")
        .join("persona.md");
    let base_persona = if persona_file.is_file() {
        tokio::fs::read_to_string(&persona_file)
            .await
            .unwrap_or_else(|_| crate::agent::BUILTIN_CVH_DEV_PROMPT.to_string())
    } else {
        crate::agent::BUILTIN_CVH_DEV_PROMPT.to_string()
    };

    // Boards context
    let boards_cfg = crate::config::load_boards_config(repo_root).await.ok();
    let active_board_id = boards_cfg
        .as_ref()
        .map(|b| b.active_board_id.clone())
        .unwrap_or_else(|| "development".to_string());
    let boards_summary = match &boards_cfg {
        Some(cfg) => {
            let mut b_list = Vec::new();
            for b in &cfg.boards {
                let cols: Vec<String> = b.columns.iter().map(|c| c.id.clone()).collect();
                b_list.push(format!(
                    "- Board '{}' (id: '{}', columns: [{}])",
                    b.name,
                    b.id,
                    cols.join(", ")
                ));
            }
            format!(
                "Active Board ID: {}\nAvailable Boards:\n{}",
                active_board_id,
                b_list.join("\n")
            )
        }
        None => "No boards configuration found.".to_string(),
    };

    // Pipeline columns context
    let pipeline_cfg = crate::config::load_pipeline_config(repo_root).await.ok();
    let pipeline_summary = match &pipeline_cfg {
        Some(pipe) => {
            let mut c_list = Vec::new();
            for col in &pipe.columns {
                let persona = col.persona.as_deref().unwrap_or("none");
                let terminal = if col.terminal == Some(true) {
                    " [terminal]"
                } else {
                    ""
                };
                c_list.push(format!(
                    "- Column '{}' (title: '{}', persona: '{}'{})",
                    col.id, col.title, persona, terminal
                ));
            }
            format!("Pipeline '{}':\n{}", pipe.title, c_list.join("\n"))
        }
        None => "No active pipeline configuration found.".to_string(),
    };

    // Existing tasks context
    let tasks = crate::commands::load_tasks_from_disk(repo_root).await;
    let tasks_summary = if tasks.is_empty() {
        "No existing tasks on the board.".to_string()
    } else {
        let mut t_list = Vec::new();
        for t in &tasks {
            let stage = &t.stage_id;
            let board = t.board_id.as_deref().unwrap_or("default");
            let status = t.status.as_deref().unwrap_or("idle");
            t_list.push(format!(
                "- [{}] \"{}\" (stage: {}, board: {}, status: {})",
                t.id, t.title, stage, board, status
            ));
        }
        format!("Current Tasks ({} total):\n{}", tasks.len(), t_list.join("\n"))
    };

    // Optional rules from .cvh/agents/cvh-dev/rules/*.md
    let rules_dir = repo_root
        .join(".cvh")
        .join("agents")
        .join("cvh-dev")
        .join("rules");
    let mut rules_sections = Vec::new();
    if rules_dir.is_dir() {
        if let Ok(mut entries) = tokio::fs::read_dir(&rules_dir).await {
            let mut rule_files = Vec::new();
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
                    let fname = entry.file_name().to_string_lossy().to_string();
                    if let Ok(content) = tokio::fs::read_to_string(&path).await {
                        rule_files.push((fname, content));
                    }
                }
            }
            rule_files.sort_by(|a, b| a.0.cmp(&b.0));
            for (fname, content) in rule_files {
                rules_sections.push(format!("### Rule: {}\n{}", fname, content.trim()));
            }
        }
    }

    let mut system_prompt = format!(
        "{}\n\n## Project Workspace Context\n{}\n\n{}\n\n{}\n\n## Tool Suite\n\
        You have direct access to 7 project configuration and system tools:\n\
        - `read_file`: Inspect file contents with line numbers.\n\
        - `write_file`: Write full content to a file.\n\
        - `edit_file`: Surgically replace text in a file.\n\
        - `glob`: Search files matching a pattern.\n\
        - `grep`: Search for regex patterns across files.\n\
        - `execute_command`: Run terminal commands with output capture.\n\
        - `web_search`: Query external technical documentation and web resources.\n\
        \n\
        Act as the Project Configuration & Environment Specialist for this project. Guide architecture, configure boards, pipelines, agent personas, rules, and tool policies. You do NOT create task cards or manage workflow tickets.",
        base_persona.trim(),
        boards_summary,
        pipeline_summary,
        tasks_summary,
    );
    if !rules_sections.is_empty() {
        system_prompt.push_str("\n\n## Meta-Architect Rules\n");
        system_prompt.push_str(&rules_sections.join("\n\n"));
    }
    system_prompt
}

/// Executes the Copilot ReAct loop with real-time streaming chunks emitted to Tauri.
async fn run_copilot_react_loop(
    repo_root: std::path::PathBuf,
    user_message: String,
    model_override: Option<String>,
    abort_flag: Arc<AtomicBool>,
    app_handle: AppHandle,
) -> Result<String, String> {
    let llm_settings = crate::config::load_llm_settings(&repo_root)
        .await
        .unwrap_or_default();
    let model = model_override
        .filter(|m| !m.trim().is_empty())
        .unwrap_or(llm_settings.model);
    let base_url = llm_settings.base_url;
    let api_key = if llm_settings.api_key.trim().is_empty() {
        None
    } else {
        Some(llm_settings.api_key)
    };
    let temperature = llm_settings.temperature.unwrap_or(0.2);

    let system_prompt = compile_copilot_system_prompt(&repo_root).await;
    let tools = get_copilot_tool_suite();

    // 1. Load prior dialogue history from SQLite for "copilot:meta-chat"
    let history =
        crate::db::load_dialog_items(&repo_root, COPILOT_DIALOG_TASK_ID).unwrap_or_default();

    let mut messages = vec![ChatMessage::system(system_prompt)
        .with_cache_control(crate::llm::CacheControl::ephemeral())];

    for item in history {
        if item.item_type == "user" {
            if let Some(c) = item.content {
                if !c.trim().is_empty() {
                    messages.push(ChatMessage::user(c));
                }
            }
        } else if item.item_type == "assistant" {
            if let Some(c) = item.content {
                if !c.trim().is_empty() {
                    messages.push(ChatMessage::assistant(Some(c), None));
                }
            }
        }
    }

    // 2. Persist new user message into SQLite
    let now_iso = chrono::Local::now().format("%H:%M:%S").to_string();
    let user_activity = ActivityItem {
        id: format!("copilot-user-{}", uuid::Uuid::new_v4()),
        item_type: "user".to_string(),
        timestamp: now_iso,
        content: Some(user_message.clone()),
        tool: None,
        duration_ms: None,
        images: None,
    };
    let _ = crate::db::append_dialog_items(
        &repo_root,
        COPILOT_DIALOG_TASK_ID,
        std::slice::from_ref(&user_activity),
    );

    messages.push(ChatMessage::user(user_message));

    let tool_policy = crate::config::load_tool_policies(&repo_root)
        .await
        .unwrap_or_default();
    let active_tools: Vec<ToolDefinition> = tools
        .into_iter()
        .filter(|t| crate::config::is_tool_allowed(&tool_policy, None, &t.function.name))
        .collect();
    let llm_client = LlmClient::new();

    const MAX_COPILOT_TURNS: usize = 30;
    let mut turn: usize = 0;

    loop {
        turn += 1;
        if turn > MAX_COPILOT_TURNS {
            let msg = format!("Reached maximum turns limit ({}) for copilot session", MAX_COPILOT_TURNS);
            let _ = app_handle.emit(
                "copilot:stream",
                serde_json::json!({
                    "taskId": "copilot",
                    "chunk": &msg,
                    "type": "error"
                }),
            );
            return Err(msg);
        }

        if abort_flag.load(Ordering::SeqCst) {
            let _ = app_handle.emit(
                "copilot:stream",
                serde_json::json!({
                    "taskId": "copilot",
                    "chunk": "Copilot execution aborted by user",
                    "type": "error"
                }),
            );
            return Err("Copilot execution aborted by user".to_string());
        }

        let req = OpenAIChatRequest {
            model: model.clone(),
            messages: messages.clone(),
            temperature: Some(temperature),
            stream: true,
            tools: if active_tools.is_empty() {
                None
            } else {
                Some(active_tools.clone())
            },
            reasoning_effort: None,
            stream_options: Some(crate::llm::StreamOptions { include_usage: true }),
        };

        let (tx, mut rx) = tokio::sync::mpsc::channel::<StreamChunk>(100);
        let client = llm_client.clone();
        let base_url_clone = base_url.clone();
        let api_key_clone = api_key.clone();

        tokio::spawn(async move {
            if let Err(e) = client
                .stream_openai(
                    &base_url_clone,
                    api_key_clone.as_deref(),
                    req,
                    tx.clone(),
                )
                .await
            {
                let err_msg = format!("Failed to dispatch LLM request to {}: {}", base_url_clone, e);
                eprintln!("CVH [copilot]: {}", err_msg);
                let _ = tx.send(StreamChunk::Error(err_msg)).await;
            }
        });

        let mut turn_content = String::new();
        let mut turn_tool_calls: Vec<ToolCall> = Vec::new();
        let mut thinking_parser = ThinkingStreamParser::new();
        let mut received_any_chunk = false;

        while let Some(chunk) = rx.recv().await {
            received_any_chunk = true;
            if abort_flag.load(Ordering::SeqCst) {
                let _ = app_handle.emit(
                    "copilot:stream",
                    serde_json::json!({
                        "taskId": "copilot",
                        "chunk": "Copilot execution aborted by user",
                        "type": "error"
                    }),
                );
                return Err("Copilot execution aborted by user".to_string());
            }

            match chunk {
                StreamChunk::Token(token) => {
                    let parsed = thinking_parser.push(&token);
                    for (is_thought, text) in parsed {
                        if is_thought {
                            let _ = app_handle.emit(
                                "copilot:stream",
                                serde_json::json!({
                                    "taskId": "copilot",
                                    "chunk": text,
                                    "type": "thought"
                                }),
                            );
                        } else {
                            turn_content.push_str(&text);
                            let _ = app_handle.emit(
                                "copilot:stream",
                                serde_json::json!({
                                    "taskId": "copilot",
                                    "chunk": text,
                                    "type": "token"
                                }),
                            );
                        }
                    }
                }
                StreamChunk::Output(output) => {
                    turn_content.push_str(&output);
                    let _ = app_handle.emit(
                        "copilot:stream",
                        serde_json::json!({
                            "taskId": "copilot",
                            "chunk": output,
                            "type": "token"
                        }),
                    );
                }
                StreamChunk::Thought(thought) => {
                    let _ = app_handle.emit(
                        "copilot:stream",
                        serde_json::json!({
                            "taskId": "copilot",
                            "chunk": thought,
                            "type": "thought"
                        }),
                    );
                }
                StreamChunk::ToolCall { id, name, arguments } => {
                    let _ = app_handle.emit(
                        "copilot:stream",
                        serde_json::json!({
                            "taskId": "copilot",
                            "chunk": serde_json::json!({
                                "id": &id,
                                "name": &name,
                                "arguments": &arguments
                            }),
                            "type": "tool_call"
                        }),
                    );
                    turn_tool_calls.push(ToolCall {
                        id,
                        call_type: "function".to_string(),
                        function: crate::llm::FunctionCall { name, arguments },
                    });
                }
                StreamChunk::Usage { .. } => {}
                StreamChunk::Done => break,
                StreamChunk::Error(err) => {
                    let _ = app_handle.emit(
                        "copilot:stream",
                        serde_json::json!({
                            "taskId": "copilot",
                            "chunk": &err,
                            "type": "error"
                        }),
                    );
                    return Err(format!("LLM streaming error: {}", err));
                }
            }
        }

        if !received_any_chunk && !abort_flag.load(Ordering::SeqCst) {
            let err_msg = format!(
                "No stream chunks received from LLM endpoint ({}) - check proxy connection",
                base_url
            );
            let _ = app_handle.emit(
                "copilot:stream",
                serde_json::json!({
                    "taskId": "copilot",
                    "chunk": &err_msg,
                    "type": "error"
                }),
            );
            return Err(err_msg);
        }

        let flushed = thinking_parser.flush();
        for (is_thought, text) in flushed {
            if is_thought {
                let _ = app_handle.emit(
                    "copilot:stream",
                    serde_json::json!({
                        "taskId": "copilot",
                        "chunk": text,
                        "type": "thought"
                    }),
                );
            } else {
                turn_content.push_str(&text);
                let _ = app_handle.emit(
                    "copilot:stream",
                    serde_json::json!({
                        "taskId": "copilot",
                        "chunk": text,
                        "type": "token"
                    }),
                );
            }
        }

        // Terminal response reached when no tool calls were emitted
        if turn_tool_calls.is_empty() {
            let assistant_activity = ActivityItem {
                id: format!("copilot-assistant-{}", uuid::Uuid::new_v4()),
                item_type: "assistant".to_string(),
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                content: Some(turn_content.clone()),
                tool: None,
                duration_ms: None,
                images: None,
            };
            let _ = crate::db::append_dialog_items(
                &repo_root,
                COPILOT_DIALOG_TASK_ID,
                std::slice::from_ref(&assistant_activity),
            );

            let _ = app_handle.emit(
                "copilot:stream",
                serde_json::json!({
                    "taskId": "copilot",
                    "chunk": &turn_content,
                    "type": "finished"
                }),
            );
            return Ok(turn_content);
        }

        // Append assistant tool calls to message history
        messages.push(ChatMessage::assistant(
            if turn_content.is_empty() {
                None
            } else {
                Some(turn_content.clone())
            },
            Some(turn_tool_calls.clone()),
        ));

        let tool_ctx = ToolContext {
            repo_root: &repo_root,
            worktree_dir: &repo_root,
            stage_id: Some("copilot"),
            role: Some("copilot"),
            abort_handle: Arc::clone(&abort_flag),
            tool_policy: &tool_policy,
            task_id: None,
            agent_allowed_tools: None,
            app_handle: Some(app_handle.clone()),
            base_url: Some(&base_url),
            api_key: api_key.as_deref(),
            model: Some(&model),
            temperature: Some(temperature),
            reasoning_effort: None,
            call_id: None,
        };

        // Execute each tool call
        for tc in turn_tool_calls {
            if abort_flag.load(Ordering::SeqCst) {
                let _ = app_handle.emit(
                    "copilot:stream",
                    serde_json::json!({
                        "taskId": "copilot",
                        "chunk": "Copilot execution aborted by user",
                        "type": "error"
                    }),
                );
                return Err("Copilot execution aborted by user".to_string());
            }

            let parsed_args: serde_json::Value =
                serde_json::from_str(&tc.function.arguments).unwrap_or_else(|_| serde_json::json!({}));

            let mut call_tool_ctx = tool_ctx.clone();
            call_tool_ctx.call_id = Some(&tc.id);

            let exec_result = execute_tool(&call_tool_ctx, &tc.function.name, &parsed_args).await;

            if abort_flag.load(Ordering::SeqCst) {
                let _ = app_handle.emit(
                    "copilot:stream",
                    serde_json::json!({
                        "taskId": "copilot",
                        "chunk": "Copilot execution aborted by user",
                        "type": "error"
                    }),
                );
                return Err("Copilot execution aborted by user".to_string());
            }

            let (result_str, success) = match exec_result {
                Ok(out) => (out, true),
                Err(e) => (format!("Error executing {}: {}", tc.function.name, e), false),
            };

            // Emit tool_result event
            let _ = app_handle.emit(
                "copilot:stream",
                serde_json::json!({
                    "taskId": "copilot",
                    "chunk": serde_json::json!({
                        "id": &tc.id,
                        "name": &tc.function.name,
                        "result": &result_str,
                        "success": success
                    }),
                    "type": "tool_result"
                }),
            );

            // Persist tool activity item to SQLite
            let tool_activity = ActivityItem {
                id: format!("copilot-tool-{}", tc.id),
                item_type: "tool".to_string(),
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                content: Some(result_str.clone()),
                tool: Some(serde_json::json!({
                    "id": tc.id,
                    "name": tc.function.name,
                    "arguments": parsed_args,
                    "success": success
                })),
                duration_ms: None,
                images: None,
            };
            let _ = crate::db::append_dialog_items(
                &repo_root,
                COPILOT_DIALOG_TASK_ID,
                std::slice::from_ref(&tool_activity),
            );

            messages.push(ChatMessage::tool_response(tc.id, result_str));
        }
    }
}

/// Tauri IPC command to send a message to CVH Copilot and stream the ReAct loop chunks.
#[tauri::command]
pub async fn send_copilot_message(
    message: String,
    model: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let repo_root = state.get_root();
    let abort_flag = Arc::new(AtomicBool::new(false));
    let join_slot = Arc::new(tokio::sync::Mutex::new(None));

    // Abort and cancel any previous running copilot session
    let prev_opt = {
        let mut guard = state
            .active_copilot
            .lock()
            .map_err(|e| format!("Failed to acquire active_copilot lock: {}", e))?;
        let prev = guard.take();
        *guard = Some(AgentEntry {
            abort_flag: Arc::clone(&abort_flag),
            join_handle: Arc::clone(&join_slot),
        });
        prev
    };

    if let Some(prev) = prev_opt {
        prev.abort_flag.store(true, Ordering::SeqCst);
        let mut handle_guard = prev.join_handle.lock().await;
        if let Some(h) = handle_guard.take() {
            h.abort();
        }
    }

    let (tx_result, rx_result) = tokio::sync::oneshot::channel::<Result<String, String>>();
    let abort_flag_task = Arc::clone(&abort_flag);
    let app_handle_task = app_handle.clone();

    let handle = tokio::spawn(async move {
        let loop_res = run_copilot_react_loop(
            repo_root,
            message,
            model,
            abort_flag_task,
            app_handle_task,
        )
        .await;
        let _ = tx_result.send(loop_res);
    });

    *join_slot.lock().await = Some(handle);

    let active_copilot_slot = Arc::clone(&state.active_copilot);
    let result = match rx_result.await {
        Ok(res) => res,
        Err(_) => Err("Copilot execution aborted".to_string()),
    };

    // Clean up active_copilot entry once task finishes (pointer check)
    if let Ok(mut guard) = active_copilot_slot.lock() {
        if let Some(ref entry) = *guard {
            if Arc::ptr_eq(&entry.abort_flag, &abort_flag) {
                *guard = None;
            }
        }
    }

    result
}

/// Tauri IPC command to abort the running CVH Copilot session.
#[tauri::command]
pub async fn abort_copilot(state: State<'_, AppState>) -> Result<(), String> {
    let entry_opt = {
        let mut guard = state
            .active_copilot
            .lock()
            .map_err(|e| format!("Failed to acquire active_copilot lock: {}", e))?;
        guard.take()
    };

    if let Some(entry) = entry_opt {
        entry.abort_flag.store(true, Ordering::SeqCst);
        let mut handle_guard = entry.join_handle.lock().await;
        if let Some(h) = handle_guard.take() {
            h.abort();
        }
    }
    Ok(())
}

/// Tauri IPC command to load copilot conversation history from SQLite.
#[tauri::command]
pub async fn load_copilot_history(state: State<'_, AppState>) -> Result<Vec<ActivityItem>, String> {
    let root = state.get_root();
    crate::db::load_dialog_items(&root, COPILOT_DIALOG_TASK_ID)
}

/// Tauri IPC command to clear copilot conversation history in SQLite.
#[tauri::command]
pub async fn clear_copilot_history(state: State<'_, AppState>) -> Result<(), String> {
    let root = state.get_root();
    crate::db::clear_dialog_items(&root, COPILOT_DIALOG_TASK_ID)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copilot_tool_suite_has_seven_tools() {
        let tools = get_copilot_tool_suite();
        assert_eq!(tools.len(), 7);
        let names: Vec<String> = tools.into_iter().map(|t| t.function.name).collect();
        assert_eq!(
            names,
            vec![
                "edit_file",
                "execute_command",
                "glob",
                "grep",
                "read_file",
                "web_search",
                "write_file",
            ]
        );
    }

    #[tokio::test]
    async fn test_compile_copilot_system_prompt_structure() {
        let temp_dir = std::env::temp_dir().join(format!("cvh_copilot_prompt_test_{}", uuid::Uuid::new_v4()));
        let cvh_dir = temp_dir.join(".cvh");
        let agents_dir = cvh_dir.join("agents").join("cvh-dev");
        tokio::fs::create_dir_all(&agents_dir).await.expect("create agents dir");

        let persona_content = "# Role Persona: cvh-dev (Custom Architect)\n- Specialized architect persona test.";
        tokio::fs::write(agents_dir.join("persona.md"), persona_content).await.expect("write persona.md");

        let prompt = compile_copilot_system_prompt(&temp_dir).await;
        assert!(prompt.contains("Custom Architect"));
        assert!(prompt.contains("## Project Workspace Context"));
        assert!(prompt.contains("## Tool Suite"));
        assert!(!prompt.contains("`create_task`"));
        assert!(!prompt.contains("`complete_task`"));
        assert!(prompt.contains("`read_file`"));
        assert!(prompt.contains("`write_file`"));

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_copilot_dialog_roundtrip_and_clear() {
        let temp_dir = std::env::temp_dir().join(format!("cvh_copilot_db_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir).await.expect("create temp dir");

        let user_item = ActivityItem {
            id: format!("copilot-user-{}", uuid::Uuid::new_v4()),
            item_type: "user".to_string(),
            timestamp: "12:00:00".to_string(),
            content: Some("Architecture query".to_string()),
            tool: None,
            duration_ms: None,
            images: None,
        };

        crate::db::append_dialog_items(&temp_dir, COPILOT_DIALOG_TASK_ID, &[user_item]).expect("append item");

        let history = crate::db::load_dialog_items(&temp_dir, COPILOT_DIALOG_TASK_ID).expect("load history");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].content.as_deref(), Some("Architecture query"));

        crate::db::clear_dialog_items(&temp_dir, COPILOT_DIALOG_TASK_ID).expect("clear history");
        let history_cleared = crate::db::load_dialog_items(&temp_dir, COPILOT_DIALOG_TASK_ID).expect("load after clear");
        assert!(history_cleared.is_empty());

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_abort_copilot_cancels_task_handle() {
        let abort_flag = Arc::new(AtomicBool::new(false));
        let join_slot = Arc::new(tokio::sync::Mutex::new(None));

        let flag_clone = Arc::clone(&abort_flag);
        let task_handle = tokio::spawn(async move {
            while !flag_clone.load(Ordering::SeqCst) {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        });

        *join_slot.lock().await = Some(task_handle);

        let entry = AgentEntry {
            abort_flag: Arc::clone(&abort_flag),
            join_handle: Arc::clone(&join_slot),
        };

        // Signal abort
        entry.abort_flag.store(true, Ordering::SeqCst);
        let mut h_guard = entry.join_handle.lock().await;
        if let Some(h) = h_guard.take() {
            h.abort();
            let join_res = h.await;
            assert!(join_res.is_err() || abort_flag.load(Ordering::SeqCst));
        }

        assert!(abort_flag.load(Ordering::SeqCst));
    }
}

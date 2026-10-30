use super::*;
use crate::config::{BoardDef, BoardsConfig, PipelineColumn};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub(crate) fn make_test_board(_base: &Path) -> (BoardsConfig, Vec<PipelineColumn>) {
    let b_cols = vec![
        PipelineColumn {
            id: "backlog".to_string(),
            title: "Backlog".to_string(),
            color: None,
            icon: None,
            persona: None,
            on_enter_script: None,
            context_window: None,
            model_override: None,
            reasoning_effort: None,
            temperature: None,
            gatekeeper_sensor: None,
            allowed_tools: None,
            hooks: HashMap::new(),
            routes: None,
            auto_run: None,
            terminal: None,
            is_intake: Some(true),
        },
        PipelineColumn {
            id: "dev".to_string(),
            title: "Dev".to_string(),
            color: None,
            icon: None,
            persona: Some("developer".to_string()),
            on_enter_script: None,
            context_window: None,
            model_override: None,
            reasoning_effort: None,
            temperature: None,
            gatekeeper_sensor: None,
            allowed_tools: None,
            hooks: HashMap::new(),
            routes: None,
            auto_run: None,
            terminal: None,
            is_intake: None,
        },
        PipelineColumn {
            id: "review".to_string(),
            title: "Review".to_string(),
            color: None,
            icon: None,
            persona: Some("reviewer".to_string()),
            on_enter_script: None,
            context_window: None,
            model_override: None,
            reasoning_effort: None,
            temperature: None,
            gatekeeper_sensor: None,
            allowed_tools: None,
            hooks: HashMap::new(),
            routes: None,
            auto_run: None,
            terminal: None,
            is_intake: None,
        },
        PipelineColumn {
            id: "ready".to_string(),
            title: "Ready".to_string(),
            color: None,
            icon: None,
            persona: None,
            on_enter_script: None,
            context_window: None,
            model_override: None,
            reasoning_effort: None,
            temperature: None,
            gatekeeper_sensor: None,
            allowed_tools: None,
            hooks: HashMap::new(),
            routes: None,
            auto_run: None,
            terminal: Some(true),
            is_intake: None,
        },
    ];

    let boards_cfg = BoardsConfig {
        active_board_id: "test-board".to_string(),
        boards: vec![BoardDef {
            id: "test-board".to_string(),
            name: "Test Board".to_string(),
            columns: b_cols.clone(),
        }],
    };

    (boards_cfg, b_cols)
}

pub(crate) fn make_test_task(id: &str, stage_id: &str, board_id: &str) -> Task {
    Task {
        id: id.to_string(),
        title: format!("Task {}", id),
        description: format!("Description for {}", id),
        stage_id: stage_id.to_string(),
        board_id: Some(board_id.to_string()),
        worktree_path: None,
        branch: None,
        verdict: None,
        custom_flow: None,
        status: Some("running".to_string()),
        model: None,
        parent_id: None,
        subtask_index: None,
        waiting_for_subtask_id: None,
        todos: None,
        priority: None,
        tags: None,
        current_tokens: None,
        skip_worktree: None,
        created_at: 0,
        updated_at: 0,
        stage_entered_at: None,
        completed_at: None,
        handover_summary: None,
        attachments: None,
        cover_image: None,
        position: None,
        hook_states: None,
    }
}

pub(crate) async fn start_mock_sse_server(
    sse_payload: &'static str,
    counter: Arc<std::sync::atomic::AtomicUsize>,
) -> (String, tokio::task::JoinHandle<()>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock listener");
    let port = listener.local_addr().expect("local addr").port();
    let base_url = format!("http://127.0.0.1:{}", port);

    let handle = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let req_num = counter.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                let mut data = Vec::new();
                let mut buf = [0u8; 4096];
                let mut content_length = 0;
                let mut header_len = 0;
                loop {
                    let n = socket.read(&mut buf).await.unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    data.extend_from_slice(&buf[..n]);
                    if header_len == 0 {
                        if let Some(pos) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                            header_len = pos + 4;
                            let headers_str = String::from_utf8_lossy(&data[..header_len]);
                            for line in headers_str.lines() {
                                if let Some(val) = line.to_lowercase().strip_prefix("content-length:") {
                                    content_length = val.trim().parse::<usize>().unwrap_or(0);
                                }
                            }
                        }
                    }
                    if header_len > 0 && data.len() >= header_len + content_length {
                        break;
                    }
                }

                let chunks: Vec<&str> = sse_payload
                    .split("data: [DONE]")
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();
                let response_payload = if chunks.len() > 1 {
                    let chunk = chunks.get(req_num).unwrap_or(chunks.last().unwrap());
                    format!("{}\n\ndata: [DONE]\n\n", chunk)
                } else {
                    sse_payload.to_string()
                };

                let body = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n{}",
                    response_payload
                );
                let _ = socket.write_all(body.as_bytes()).await;
                let _ = socket.flush().await;
                let _ = socket.shutdown().await;
            });
        }
    });

    (base_url, handle)
}

mod compactor_tests;
mod engine_tests;
mod session_tests;
mod telemetry_tests;
mod tools_policy_tests;
mod tools_pipeline_tests;
mod tools_fs_tests;
mod tools_cmd_tests;

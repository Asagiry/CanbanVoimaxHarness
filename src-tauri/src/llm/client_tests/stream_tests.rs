use super::fixtures::*;
use crate::llm::client::LlmClient;
use crate::llm::models::*;

#[tokio::test]
async fn incomplete_streams_never_release_tool_calls() {
    let tool = tool_stream_frame("{}");
    let mut cases = Vec::new();
    for reason in ["length", "content_filter", "max_tokens"] {
        cases.push(format!("{tool}data: {}\n\ndata: [DONE]\n\n", serde_json::json!({
            "choices": [{"delta": {}, "finish_reason": reason}], "usage": {"prompt_tokens": 10, "completion_tokens": 4}
        })));
        for ending in ["data: [DONE]\n\n", ""] {
            cases.push(format!("{tool}data: {}\n\ndata: {}\n\n{ending}",
                serde_json::json!({"choices": [{"delta": {}, "finish_reason": reason}]}),
                serde_json::json!({"choices": [], "usage": {"prompt_tokens": 10, "completion_tokens": 4}}),
            ));
        }
    }
    let cases_with_usage = cases.len();
    cases.push(format!("{tool}data: {{\"type\":\"message_delta\",\"delta\":{{\"stop_reason\":\"max_tokens\"}}}}\n\n"));
    cases.push(format!("{tool}data: {{\"stop_reason\":\"max_tokens\"}}\n\n"));
    cases.push(format!("{tool}data: invalid json\n\ndata: [DONE]\n\n"));
    cases.push(format!("{}data: [DONE]\n\n", tool_stream_frame("{\"path\":")));
    cases.push(format!("{tool}data: {}\n\ndata: [DONE]\n\n", serde_json::json!({
        "choices": [{"delta": {"tool_calls": [{"index": 1, "function": {"name": "read_file", "arguments": "{"}}]}}]
    })));
    cases.push(format!("{tool}data: {}\n\ndata: [DONE]\n\n", serde_json::json!({
        "error": {"message": "upstream generation failed", "type": "server_error"}
    })));
    for (index, response) in cases.into_iter().enumerate() {
        let (result, events) = stream_test_response(response).await;
        assert!(result.is_err());
        assert!(events.iter().any(|event| matches!(event, StreamChunk::Error(_))));
        assert!(events.iter().all(|event| !matches!(event, StreamChunk::ToolCall { .. } | StreamChunk::Done)));
        if index < cases_with_usage {
            assert!(events.iter().any(|event| matches!(event, StreamChunk::Usage { input_tokens: 10, output_tokens: 4, .. })));
        }
    }
}

#[tokio::test]
async fn batch_tool_calls_without_index_produce_distinct_calls() {
    let frame = |id: &str, name: &str, args: &str| {
        format!("data: {}\n\n", serde_json::json!({"choices": [{"delta": {"tool_calls": [{
            "id": id, "function": {"name": name, "arguments": args}
        }]}}]}))
    };
    let response = format!(
        "{}{}{}data: {}\n\n",
        frame("call-a", "read_file", "{\"path\":\"a\"}"),
        frame("call-b", "read_file", "{\"path\":\"b\"}"),
        frame("call-c", "read_file", "{\"path\":\"c\"}"),
        serde_json::json!({"choices": [{"delta": {}, "finish_reason": "tool_calls"}]}),
    );
    let (result, events) = stream_test_response(response).await;
    assert!(result.is_ok());
    let calls: Vec<_> = events.iter().filter_map(|event| match event {
        StreamChunk::ToolCall { id, name, arguments } => Some((id.clone(), name.clone(), arguments.clone())),
        _ => None,
    }).collect();
    assert_eq!(calls.len(), 3);
    let ids: std::collections::HashSet<_> = calls.iter().map(|(id, _, _)| id.as_str()).collect();
    assert_eq!(ids.len(), 3);
    assert!(ids.contains("call-a") && ids.contains("call-b") && ids.contains("call-c"));
    assert_eq!(calls.iter().map(|(_, _, args)| args.as_str()).collect::<Vec<_>>(),
        vec!["{\"path\":\"a\"}", "{\"path\":\"b\"}", "{\"path\":\"c\"}"]);
}

#[tokio::test]
async fn stream_accepts_done_or_successful_finish_reason() {
    for reason in ["stop", "tool_calls", "function_call"] {
        let response = format!("{}data: {}\n\n", tool_stream_frame("{}"), serde_json::json!({
            "choices": [{"delta": {}, "finish_reason": reason}]
        }));
        for missing_bytes in [0, 20] {
            let (result, events) = stream_test_http_response("200 OK", response.clone(), missing_bytes).await;
            assert!(result.is_ok());
            assert_eq!(events.iter().filter(|event| matches!(event, StreamChunk::ToolCall { .. })).count(), 1);
            assert!(matches!(events.last(), Some(StreamChunk::Done)));
        }
    }
    let (result, events) = stream_test_response(format!("{}data: [DONE]\n\n", tool_stream_frame("{}"))).await;
    assert!(result.is_ok());
    assert!(matches!(events.last(), Some(StreamChunk::Done)));
}

#[tokio::test]
async fn interrupted_stream_waits_for_retry_until_stopped() {
    for missing_bytes in [0, 20] {
        let body = format!("{}data: {}\n\n", tool_stream_frame("{}"), serde_json::json!({
            "choices": [], "usage": {"prompt_tokens": 10, "completion_tokens": 4}
        }));
        let (result, events) = stream_test_http_response("200 OK", body, missing_bytes).await;
        assert!(result.is_ok());
        let Some(StreamChunk::AttemptStart { id }) = events.first() else { panic!("missing attempt ID") };
        assert!(matches!(events.last(), Some(StreamChunk::Retry { id: retry_id, delay_secs: 5 }) if retry_id == id));
        assert!(events.iter().any(|event| matches!(event, StreamChunk::Usage { input_tokens: 10, output_tokens: 4, .. })));
        assert!(events.iter().all(|event| !matches!(event, StreamChunk::ToolCall { .. } | StreamChunk::Done | StreamChunk::Error(_))));
    }
    let (result, events) = stream_test_http_response("503 Service Unavailable", "offline".into(), 0).await;
    assert!(result.is_ok());
    assert!(matches!(events.last(), Some(StreamChunk::Retry { delay_secs: 5, .. })));
    for status in ["400 Bad Request", "401 Unauthorized", "403 Forbidden", "429 Too Many Requests"] {
        let (result, events) = stream_test_http_response(status, "request denied".into(), 0).await;
        assert!(result.is_err());
        assert!(matches!(events.last(), Some(StreamChunk::Error(_))));
        assert!(events.iter().all(|event| !matches!(event, StreamChunk::Retry { .. })));
    }
}

#[tokio::test]
async fn transport_disconnect_retries_same_request_and_discards_partial_tools() {
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let endpoint = format!("http://opencode.ai:{}/inference/go/openai/v1", address.port());
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for index in 0..2 {
            let (mut socket, _) = listener.accept().await.unwrap();
            requests.push(read_test_request(&mut socket).await);
            let (body, missing_bytes) = if index == 0 {
                (format!("{}data: {}\n\n", tool_stream_frame("{\"path\":\"discarded\"}"), serde_json::json!({
                    "choices": [], "usage": {"prompt_tokens": 10, "completion_tokens": 4}
                })), 20)
            } else {
                (format!("{}data: {}\n\ndata: [DONE]\n\n", tool_stream_frame("{\"path\":\"kept\"}"), serde_json::json!({
                    "choices": [{"delta": {}, "finish_reason": "tool_calls"}],
                    "usage": {"prompt_tokens": 20, "completion_tokens": 6}
                })), 0)
            };
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len() + missing_bytes).as_bytes()).await.unwrap();
        }
        requests
    });
    let client = LlmClient {
        client: reqwest::Client::builder().no_proxy().resolve("opencode.ai", address).build().unwrap(),
        ..LlmClient::new()
    };
    let request = OpenAIChatRequest {
        model: "test".into(), messages: vec![ChatMessage::user("test")], temperature: Some(0.2),
        stream: true, tools: None, reasoning_effort: Some("low".into()),
        stream_options: Some(StreamOptions { include_usage: true }),
    };
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let events_task = tokio::spawn(async move {
        let mut events = Vec::new();
        while let Some(event) = rx.recv().await { events.push(event); }
        events
    });
    tokio::time::timeout(std::time::Duration::from_secs(10), client.stream_openai(&endpoint, Some("test-key"), request, tx))
        .await.unwrap().unwrap();
    let requests = tokio::time::timeout(std::time::Duration::from_secs(2), server).await.unwrap().unwrap();
    let events = tokio::time::timeout(std::time::Duration::from_secs(2), events_task).await.unwrap().unwrap();
    assert_eq!(requests[0], requests[1]);
    assert!(requests[0].0.contains("x-opencode-session:"));
    let attempts: Vec<_> = events.iter().filter_map(|event| match event {
        StreamChunk::AttemptStart { id } => Some(id), _ => None,
    }).collect();
    assert_eq!(attempts.len(), 2);
    assert_ne!(attempts[0], attempts[1]);
    let retry = events.iter().position(|event| matches!(event, StreamChunk::Retry { id, delay_secs: 5 } if id == attempts[0])).unwrap();
    assert!(events[..retry].iter().any(|event| matches!(event, StreamChunk::Usage { input_tokens: 10, output_tokens: 4, .. })));
    let calls: Vec<_> = events.iter().filter_map(|event| match event {
        StreamChunk::ToolCall { arguments, .. } => Some(arguments.as_str()), _ => None,
    }).collect();
    assert_eq!(calls, vec!["{\"path\":\"kept\"}"]);
    assert!(events.iter().all(|event| !matches!(event, StreamChunk::Error(_))));
    assert!(matches!(events.last(), Some(StreamChunk::Done)));
}

#[tokio::test]
async fn opencode_transport_omits_cache_markers_and_preserves_history() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let mut captured = Vec::new();
        for _ in 0..3 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 4096];
            let (headers, body_start, length) = loop {
                let count = socket.read(&mut buffer).await.unwrap();
                assert_ne!(count, 0);
                bytes.extend_from_slice(&buffer[..count]);
                if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                    let headers = String::from_utf8(bytes[..end].to_vec()).unwrap().to_lowercase();
                    let length: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: "))
                        .unwrap().parse().unwrap();
                    break (headers, end + 4, length);
                }
            };
            while bytes.len() < body_start + length {
                let count = socket.read(&mut buffer).await.unwrap();
                assert_ne!(count, 0);
                bytes.extend_from_slice(&buffer[..count]);
            }
            let body: serde_json::Value = serde_json::from_slice(&bytes[body_start..body_start + length]).unwrap();
            captured.push((headers, body));
            let response = "data: [DONE]\n\n";
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", response.len(), response).as_bytes()).await.unwrap();
        }
        captured
    });
    let client = LlmClient {
        client: reqwest::Client::builder().no_proxy().resolve("opencode.ai", address).build().unwrap(),
        ..LlmClient::new()
    };
    let messages = vec![
        ChatMessage::system("system").with_cache_control(CacheControl::ephemeral()),
        ChatMessage::user_with_images("image", vec!["data:image/png;base64,test".into()]).with_cache_control(CacheControl::ephemeral()),
        ChatMessage::assistant(None, Some(vec![ToolCall {
            id: "call-1".into(), call_type: "function".into(),
            function: FunctionCall { name: "read_file".into(), arguments: "{}".into() },
        }])).with_cache_control(CacheControl::ephemeral()),
        ChatMessage::tool_response("call-1", "result").with_cache_control(CacheControl::ephemeral()),
    ];
    for index in 0..3 {
        let mut request = OpenAIChatRequest {
            model: "test-model".into(), messages: messages.clone(), temperature: Some(0.2),
            stream: true, tools: None, reasoning_effort: Some("low".into()),
            stream_options: Some(StreamOptions { include_usage: true }),
        };
        if index == 1 { request.messages.push(ChatMessage::user("continue")); }
        let endpoint = if index < 2 {
            format!("http://opencode.ai:{}/inference/go/openai/v1", address.port())
        } else {
            format!("http://{address}/v1")
        };
        let (tx, _rx) = tokio::sync::mpsc::channel(8);
        tokio::time::timeout(std::time::Duration::from_secs(3), client.stream_openai(&endpoint, None, request, tx))
            .await.unwrap().unwrap();
    }
    let captured = tokio::time::timeout(std::time::Duration::from_secs(3), server).await.unwrap().unwrap();
    for (headers, body) in &captured[..2] {
        assert!(headers.contains("x-opencode-session:"));
        assert!(headers.contains("x-opencode-client: desktop"));
        assert!(headers.contains("user-agent: opencode/latest/2.2.5/desktop"));
        assert!(headers.contains("accept: */*"));
        assert!(body["messages"].as_array().unwrap().iter().all(|message| message.get("cache_control").is_none()));
        assert_eq!(body["stream_options"]["include_usage"], true);
        assert_eq!(body["reasoning_effort"], "low");
    }
    assert_eq!(captured[0].1["messages"].as_array().unwrap(), &captured[1].1["messages"].as_array().unwrap()[..messages.len()]);
    let session = |index: usize| captured[index].0.lines().find(|line| line.starts_with("x-opencode-session:")).unwrap();
    assert_eq!(session(0), session(1));
    let mut expected = serde_json::to_value(&messages).unwrap();
    for message in expected.as_array_mut().unwrap() { message.as_object_mut().unwrap().remove("cache_control"); }
    assert_eq!(captured[0].1["messages"], expected);
    assert_eq!(captured[2].1["messages"], expected);
    assert!(messages.iter().all(|message| message.cache_control.is_some()));
}

#[tokio::test]
async fn cache_stop_closes_pending_http_request() {
    use tokio::io::AsyncReadExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut data = [0; 8192];
        assert!(stream.read(&mut data).await.unwrap() > 0);
        ready_tx.send(()).unwrap();
        loop {
            match stream.read(&mut data).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {},
            }
        }
    });
    let (tx, rx) = tokio::sync::mpsc::channel(8);
    let request = OpenAIChatRequest {
        model: "test".into(), messages: vec![ChatMessage::user("test")], temperature: None,
        stream: true, tools: None, reasoning_effort: None, stream_options: None,
    };
    let client = tokio::spawn(async move { LlmClient::new().stream_openai(&endpoint, None, request, tx).await });
    ready_rx.await.unwrap();
    drop(rx);
    tokio::time::timeout(std::time::Duration::from_secs(2), client).await.unwrap().unwrap().unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), server).await.unwrap().unwrap();
}

#[tokio::test]
async fn test_opencode_responses_api_for_crouton_streams_tools_and_reasoning() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        let (headers, body_start, length) = loop {
            let count = socket.read(&mut buffer).await.unwrap();
            assert_ne!(count, 0);
            bytes.extend_from_slice(&buffer[..count]);
            if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                let headers = String::from_utf8(bytes[..end].to_vec()).unwrap().to_lowercase();
                let length: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: "))
                    .unwrap().parse().unwrap();
                break (headers, end + 4, length);
            }
        };
        while bytes.len() < body_start + length {
            let count = socket.read(&mut buffer).await.unwrap();
            assert_ne!(count, 0);
            bytes.extend_from_slice(&buffer[..count]);
        }
        let body: serde_json::Value = serde_json::from_slice(&bytes[body_start..body_start + length]).unwrap();
        let sse = concat!(
            "data: {\"type\":\"response.output_item.done\",\"item\":{\"type\":\"reasoning\",\"summary\":[{\"type\":\"summary_text\",\"text\":\"Thinking step\"}]}}\n\n",
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"Checking file\"}\n\n",
            "data: {\"type\":\"response.output_item.added\",\"output_index\":1,\"item\":{\"type\":\"function_call\",\"call_id\":\"call_99\",\"name\":\"read_file\"}}\n\n",
            "data: {\"type\":\"response.function_call_arguments.delta\",\"output_index\":1,\"delta\":\"{\\\"path\\\":\\\"src/main.rs\\\"}\"}\n\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"usage\":{\"input_tokens\":23508,\"output_tokens\":42,\"input_tokens_details\":{\"cached_tokens\":20000}}}}\n\n",
        );
        socket.write_all(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            sse.len(), sse
        ).as_bytes()).await.unwrap();
        (headers, body)
    });

    let client = LlmClient {
        client: reqwest::Client::builder().no_proxy().resolve("opencode.ai", address).build().unwrap(),
        ..LlmClient::new()
    };
    let request = OpenAIChatRequest {
        model: "crouton-alpha-free".into(),
        messages: vec![
            ChatMessage::system("sys prompt"),
            ChatMessage::user("hello"),
            ChatMessage::assistant(Some("Reading".into()), Some(vec![ToolCall {
                id: "call_1".into(),
                call_type: "function".into(),
                function: FunctionCall { name: "read_file".into(), arguments: "{\"path\":\"a.rs\"}".into() },
            }])),
            ChatMessage::tool_response("call_1", "file content"),
        ],
        temperature: Some(0.2),
        stream: true,
        tools: None,
        reasoning_effort: Some("medium".into()),
        stream_options: Some(StreamOptions { include_usage: true }),
    };
    let endpoint = format!("http://opencode.ai:{}/inference/go/openai/v1", address.port());
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let events_task = tokio::spawn(async move {
        let mut events = Vec::new();
        while let Some(event) = rx.recv().await { events.push(event); }
        events
    });

    tokio::time::timeout(std::time::Duration::from_secs(3), client.stream_openai(&endpoint, Some("k"), request, tx))
        .await.unwrap().unwrap();
    let (headers, body) = tokio::time::timeout(std::time::Duration::from_secs(3), server).await.unwrap().unwrap();
    let events = tokio::time::timeout(std::time::Duration::from_secs(3), events_task).await.unwrap().unwrap();

    assert!(headers.starts_with("post /inference/go/openai/v1/responses http/1.1"));
    assert!(headers.contains("x-opencode-client: desktop"));
    assert!(headers.contains("user-agent: opencode/latest/2.2.5/desktop"));
    assert_eq!(body["instructions"], "sys prompt");
    assert_eq!(body["reasoning"]["effort"], "medium");
    let input = body["input"].as_array().unwrap();
    assert!(input.iter().any(|item| item["type"] == "function_call" && item["call_id"] == "call_1"));
    assert!(input.iter().any(|item| item["type"] == "function_call_output" && item["call_id"] == "call_1"));

    assert!(events.iter().any(|e| matches!(e, StreamChunk::Thought(t) if t.contains("Thinking step"))));
    assert!(events.iter().any(|e| matches!(e, StreamChunk::Token(t) if t == "Checking file")));
    assert!(events.iter().any(|e| matches!(e, StreamChunk::ToolCall { id, name, arguments, .. } if id == "call_99" && name == "read_file" && arguments == "{\"path\":\"src/main.rs\"}")));
    assert!(events.iter().any(|e| matches!(e, StreamChunk::Usage { input_tokens: 23508, output_tokens: 42, cache_read_tokens: 20000, .. })));
    assert!(matches!(events.last(), Some(StreamChunk::Done)));
}

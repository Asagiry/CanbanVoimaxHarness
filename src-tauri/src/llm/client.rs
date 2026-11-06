use super::models::*;
use anyhow::{bail, Result};
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use reqwest::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct LlmClient {
    client: reqwest::Client,
    session: String,
    prefix: std::sync::Arc<std::sync::Mutex<super::cache::PrefixTracker>>,
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Robust usage extractors: different OpenAI-compatible proxies report cache
/// telemetry under different keys (OpenAI `prompt_tokens_details.cached_tokens`,
/// Responses-API `input_tokens_details.cached_tokens`, top-level `cached_tokens`,
/// Gemini-native `cached_content_token_count`, Anthropic-style
/// `cache_read_input_tokens`). Try each in order; missing keys yield 0.
/// Pure functions so prefix/telemetry behaviour is unit-testable.
pub fn extract_openai_input_tokens(usage: &serde_json::Value) -> u64 {
    if let Some(total) = usage.get("prompt_tokens").and_then(|v| v.as_u64()) {
        return total;
    }
    let input = usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
    if usage.get("input_tokens_details").is_some() {
        return input;
    }
    if usage.get("cache_read_input_tokens").is_some() || usage.get("cache_creation_input_tokens").is_some() {
        return input.saturating_add(extract_openai_cache_read(usage))
            .saturating_add(extract_openai_cache_creation(usage));
    }
    if usage.get("input_tokens").is_none() {
        return usage.get("prompt_cache_miss_tokens").and_then(|v| v.as_u64())
            .map(|miss| miss.saturating_add(extract_openai_cache_read(usage))).unwrap_or(0);
    }
    input
}

pub fn extract_openai_output_tokens(usage: &serde_json::Value) -> u64 {
    usage
        .get("completion_tokens")
        .and_then(|v| v.as_u64())
        .or_else(|| usage.get("output_tokens").and_then(|v| v.as_u64()))
        .unwrap_or(0)
}

pub fn extract_openai_cache_read(usage: &serde_json::Value) -> u64 {
    // Canonical OpenAI Chat Completions shape.
    if let Some(v) = usage
        .get("prompt_tokens_details")
        .and_then(|d| d.get("cached_tokens"))
        .and_then(|v| v.as_u64())
    {
        return v;
    }
    // Responses-API shape.
    if let Some(v) = usage
        .get("input_tokens_details")
        .and_then(|d| d.get("cached_tokens"))
        .and_then(|v| v.as_u64())
    {
        return v;
    }
    // Gemini-native passthrough shapes (in case a proxy forwards them verbatim).
    if let Some(v) = usage
        .get("cached_content_token_count")
        .and_then(|v| v.as_u64())
    {
        return v;
    }
    if let Some(v) = usage
        .get("cache_tokens_details")
        .and_then(|d| d.get("cached_content_token_count"))
        .and_then(|v| v.as_u64())
    {
        return v;
    }
    // Anthropic-style / generic fallbacks some proxies emit.
    if let Some(v) = usage
        .get("cache_read_input_tokens")
        .and_then(|v| v.as_u64())
    {
        return v;
    }
    if let Some(v) = usage
        .get("prompt_cache_hit_tokens")
        .and_then(|v| v.as_u64())
    {
        return v;
    }
    usage
        .get("cached_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
}

pub fn extract_openai_cache_creation(usage: &serde_json::Value) -> u64 {
    if let Some(v) = usage
        .get("prompt_tokens_details")
        .and_then(|d| d.get("cache_creation_tokens"))
        .and_then(|v| v.as_u64())
    {
        return v;
    }
    if let Some(v) = usage
        .get("input_tokens_details")
        .and_then(|d| d.get("cache_write_tokens"))
        .and_then(|v| v.as_u64())
    {
        return v;
    }
    if let Some(v) = usage
        .get("cache_creation_input_tokens")
        .and_then(|v| v.as_u64())
    {
        return v;
    }
    usage
        .get("cache_creation_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
}

impl LlmClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            session: uuid::Uuid::new_v4().to_string(),
            prefix: std::sync::Arc::new(std::sync::Mutex::new(super::cache::PrefixTracker::default())),
        }
    }

    pub fn for_conversation(root: &std::path::Path, task_id: &str) -> Self {
        Self { session: format!("{}:{task_id}", root.display()), ..Self::new() }
    }

    /// Stream an OpenAI-compatible endpoint (OpenAI, Local Antigravity proxy, DeepSeek, Ollama).
    pub async fn stream_openai(
        &self,
        base_url: &str,
        api_key: Option<&str>,
        mut request: OpenAIChatRequest,
        tx: Sender<StreamChunk>,
    ) -> Result<()> {
        // Deterministic Prefix Caching: alphabetically sort tools by name
        if let Some(tools) = &mut request.tools {
            tools.sort_by(|a, b| a.function.name.cmp(&b.function.name));
        }
        let mut headers = super::cache::opencode_headers(base_url, &self.session);
        let is_opencode = headers.contains_key("x-opencode-session");
        if is_opencode {
            for message in &mut request.messages {
                message.cache_control = None;
            }
        }
        self.prefix.lock().map_err(|_| anyhow::anyhow!("Prompt cache tracker lock poisoned"))?
            .observe(&request)?;

        let endpoint = if base_url.ends_with("/chat/completions") {
            base_url.to_string()
        } else if base_url.ends_with('/') {
            format!("{}chat/completions", base_url)
        } else {
            format!("{}/chat/completions", base_url)
        };

        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(key) = api_key {
            if !key.trim().is_empty() {
                if let Ok(val) = HeaderValue::from_str(&format!("Bearer {}", key.trim())) {
                    headers.insert(AUTHORIZATION, val);
                }
            }
        }

        let mut attempt = 0;
        let max_reconnects = if is_opencode { 2 } else { 10 };

        let response = loop {
            let req = self
                .client
                .post(&endpoint)
                .headers(headers.clone())
                .json(&request);

            let response = tokio::select! {
                _ = tx.closed() => return Ok(()),
                response = req.send() => response,
            };
            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        break resp;
                    }

                    let status = resp.status();
                    let text = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Failed to read error body".to_string());
                    let err_summary = if text.len() > 120 {
                        format!("{}...", text.chars().take(120).collect::<String>())
                    } else {
                        text.clone()
                    };
                    let err_msg = format!("HTTP {}: {}", status, err_summary);

                    if status.is_server_error() && attempt < max_reconnects {
                        attempt += 1;
                        let delay_secs = attempt * 10;
                        let notice = format!(
                            "\n[Связь с LLM API недоступна ({}). Повторная попытка {}/{} через {}с...]\n",
                            err_msg, attempt, max_reconnects, delay_secs
                        );
                        eprintln!("CVH [llm_client]: {}", notice);
                        tokio::select! {
                            _ = tx.closed() => return Ok(()),
                            _ = tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs as u64)) => {},
                        }
                        if tx.is_closed() {
                            bail!("Stream channel closed by receiver");
                        }
                        continue;
                    } else {
                        let final_err = format!("OpenAI API error [{}]: {}", status, text);
                        eprintln!("CVH [llm_client]: {}", final_err);
                        let _ = tx.send(StreamChunk::Error(final_err.clone())).await;
                        bail!(final_err);
                    }
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    if attempt < max_reconnects && (!is_opencode || e.is_connect()) {
                        attempt += 1;
                        let delay_secs = attempt * 10;
                        let notice = format!(
                            "\n[Связь с LLM API недоступна ({}). Повторная попытка {}/{} через {}с...]\n",
                            err_msg, attempt, max_reconnects, delay_secs
                        );
                        eprintln!("CVH [llm_client]: {}", notice);
                        tokio::select! {
                            _ = tx.closed() => return Ok(()),
                            _ = tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs as u64)) => {},
                        }
                        if tx.is_closed() {
                            bail!("Stream channel closed by receiver");
                        }
                        continue;
                    } else {
                        let final_err = format!(
                            "Failed to connect to OpenAI endpoint {} after {} attempts: {}",
                            endpoint, attempt, err_msg
                        );
                        eprintln!("CVH [llm_client]: {}", final_err);
                        let _ = tx.send(StreamChunk::Error(final_err.clone())).await;
                        bail!(final_err);
                    }
                }
            }
        };

        let mut stream = response.bytes_stream().eventsource();
        let mut tool_calls_in_progress: HashMap<usize, (String, String, String)> = HashMap::new();
        let mut first_chunk_sent = false;
        let mut pending_usage: Option<StreamChunk> = None;
        let mut terminal_marker = false;
        let mut stream_error = None;

        loop {
            let item = tokio::select! {
                _ = tx.closed() => return Ok(()),
                item = stream.next() => match item {
                    Some(item) => item,
                    None => break,
                },
            };
            match item {
                Ok(event) => {
                    let data = event.data.trim();
                    if data == "[DONE]" {
                        terminal_marker = true;
                        break;
                    }

                    let Ok(payload) = serde_json::from_str::<serde_json::Value>(data) else {
                        stream_error = Some("Invalid JSON in LLM stream".to_string());
                        break;
                    };
                    if ["/stop_reason", "/delta/stop_reason", "/message/stop_reason"].iter()
                        .any(|path| payload.pointer(path).and_then(|value| value.as_str()) == Some("max_tokens"))
                    {
                        stream_error = Some("LLM response incomplete: max_tokens".to_string());
                    }

                    if let Ok(chunk) = serde_json::from_value::<OpenAIChatChunk>(payload) {
                        if !first_chunk_sent {
                            first_chunk_sent = true;
                            let _ = tx.send(StreamChunk::FirstChunk).await;
                        }

                        // Telemetry / Usage delta if available (e.g. DeepSeek / OpenAI prompt cache hit).
                        // Raw usage is logged so a zero cache_read can be attributed to the
                        // proxy payload (missing cached_tokens) rather than to our parser.
                        if let Some(usage) = chunk.usage {
                            let input_tokens = extract_openai_input_tokens(&usage);
                            let output_tokens = extract_openai_output_tokens(&usage);
                            let cache_read = extract_openai_cache_read(&usage);
                            let cache_creation = extract_openai_cache_creation(&usage);
                            eprintln!(
                                "CVH [llm_client] usage raw={} parsed input={} output={} cache_read={} cache_creation={}",
                                usage, input_tokens, output_tokens, cache_read, cache_creation
                            );
                            pending_usage = Some(StreamChunk::Usage {
                                input_tokens,
                                output_tokens,
                                cache_read_tokens: cache_read,
                                cache_creation_tokens: cache_creation,
                            });
                        }

                        if let Some(choices) = chunk.choices {
                            for choice in choices {
                                if let Some(reason) = choice.finish_reason.as_deref().filter(|reason| !reason.is_empty()) {
                                    if matches!(reason, "stop" | "tool_calls" | "function_call") {
                                        terminal_marker = true;
                                    } else {
                                        stream_error = Some(format!("LLM response incomplete: {reason}"));
                                    }
                                }
                                // Content delta
                                if let Some(content) = choice.delta.content {
                                    if !content.is_empty() {
                                        let _ = tx.send(StreamChunk::Token(content)).await;
                                    }
                                }

                                // Reasoning / Thought delta
                                let reasoning = choice
                                    .delta
                                    .reasoning_content
                                    .or(choice.reasoning_content)
                                    .or(choice.delta.thought);
                                if let Some(thought_text) = reasoning {
                                    if !thought_text.is_empty() {
                                        let _ = tx.send(StreamChunk::Thought(thought_text)).await;
                                    }
                                }

                                // Tool calls delta
                                if let Some(tool_calls) = choice.delta.tool_calls {
                                    for tc in tool_calls {
                                        let idx = tc.index.unwrap_or(0);
                                        let entry = tool_calls_in_progress
                                            .entry(idx)
                                            .or_insert_with(|| (String::new(), String::new(), String::new()));

                                        if let Some(id) = tc.id {
                                            entry.0.push_str(&id);
                                        }
                                        if let Some(func) = tc.function {
                                            if let Some(name) = func.name {
                                                entry.1.push_str(&name);
                                            }
                                            if let Some(args) = func.arguments {
                                                entry.2.push_str(&args);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        stream_error = Some("Invalid response shape in LLM stream".to_string());
                    }
                    if stream_error.is_some() {
                        break;
                    }
                }
                Err(err) => {
                    stream_error = Some(format!("SSE stream decode error: {}", err));
                    break;
                }
            }
        }

        if stream_error.is_none() && !terminal_marker {
            stream_error = Some("LLM stream ended before a completion marker".to_string());
        }
        if stream_error.is_none() && tool_calls_in_progress.values().any(|(_, name, args)| {
            name.trim().is_empty() || !serde_json::from_str::<serde_json::Value>(args)
                .is_ok_and(|value| value.is_object())
        }) {
            stream_error = Some("LLM returned an incomplete or invalid tool call".to_string());
        }
        if let Some(err_msg) = stream_error {
            if let Some(usage_chunk) = pending_usage.take() {
                let _ = tx.send(usage_chunk).await;
            }
            let _ = tx.send(StreamChunk::Error(err_msg.clone())).await;
            bail!(err_msg);
        }

        // Flush any completed tool calls
        let mut sorted_indices: Vec<_> = tool_calls_in_progress.keys().copied().collect();
        sorted_indices.sort();
        for idx in sorted_indices {
            if let Some((id, name, args)) = tool_calls_in_progress.remove(&idx) {
                let call_id = if id.is_empty() {
                    format!("call_{}", uuid::Uuid::new_v4().simple())
                } else {
                    id
                };
                let _ = tx
                    .send(StreamChunk::ToolCall {
                        id: call_id,
                        name,
                        arguments: args,
                    })
                    .await;
            }
        }

        // Emit telemetry usage chunk if captured during stream, strictly after all tool calls are flushed
        if let Some(usage_chunk) = pending_usage.take() {
            let _ = tx.send(usage_chunk).await;
        }

        let _ = tx.send(StreamChunk::Done).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn stream_test_response(response: String) -> (Result<()>, Vec<StreamChunk>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let count = socket.read(&mut buffer).await.unwrap();
                assert_ne!(count, 0);
                request.extend_from_slice(&buffer[..count]);
                if let Some(end) = request.windows(4).position(|part| part == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]).to_lowercase();
                    let length: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: "))
                        .unwrap().parse().unwrap();
                    if request.len() >= end + 4 + length { break; }
                }
            }
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", response.len(), response).as_bytes()).await.unwrap();
        });
        let client = LlmClient {
            client: reqwest::Client::builder().no_proxy().build().unwrap(),
            ..LlmClient::new()
        };
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        let request = OpenAIChatRequest {
            model: "test".into(), messages: vec![ChatMessage::user("test")], temperature: None,
            stream: true, tools: None, reasoning_effort: None, stream_options: None,
        };
        let result = tokio::time::timeout(std::time::Duration::from_secs(3), client.stream_openai(&endpoint, None, request, tx))
            .await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(3), server).await.unwrap().unwrap();
        let mut events = Vec::new();
        while let Some(event) = rx.recv().await { events.push(event); }
        (result, events)
    }

    fn tool_stream_frame(arguments: &str) -> String {
        format!("data: {}\n\n", serde_json::json!({"choices": [{"delta": {"tool_calls": [{
            "index": 0, "id": "call-1", "function": {"name": "read_file", "arguments": arguments}
        }]}}]}))
    }

    #[tokio::test]
    async fn incomplete_streams_never_release_tool_calls() {
        let tool = tool_stream_frame("{}");
        let mut cases = Vec::new();
        for reason in ["length", "content_filter", "max_tokens"] {
            cases.push(format!("{tool}data: {}\n\ndata: [DONE]\n\n", serde_json::json!({
                "choices": [{"delta": {}, "finish_reason": reason}], "usage": {"prompt_tokens": 10, "completion_tokens": 4}
            })));
        }
        cases.push(format!("{tool}data: {{\"type\":\"message_delta\",\"delta\":{{\"stop_reason\":\"max_tokens\"}}}}\n\n"));
        cases.push(format!("{tool}data: {{\"stop_reason\":\"max_tokens\"}}\n\n"));
        cases.push(tool.clone());
        cases.push(format!("{}data: [DONE]\n\n", tool_stream_frame("{\"path\":")));
        cases.push(format!("{tool}data: {}\n\ndata: [DONE]\n\n", serde_json::json!({
            "choices": [{"delta": {"tool_calls": [{"index": 1, "function": {"name": "read_file", "arguments": "{"}}]}}]
        })));
        for (index, response) in cases.into_iter().enumerate() {
            let (result, events) = stream_test_response(response).await;
            assert!(result.is_err());
            assert!(events.iter().any(|event| matches!(event, StreamChunk::Error(_))));
            assert!(events.iter().all(|event| !matches!(event, StreamChunk::ToolCall { .. } | StreamChunk::Done)));
            if index < 3 {
                assert!(events.iter().any(|event| matches!(event, StreamChunk::Usage { input_tokens: 10, output_tokens: 4, .. })));
            }
        }
    }

    #[tokio::test]
    async fn stream_accepts_done_or_successful_finish_reason() {
        for reason in ["stop", "tool_calls", "function_call"] {
            let response = format!("{}data: {}\n\n", tool_stream_frame("{}"), serde_json::json!({
                "choices": [{"delta": {}, "finish_reason": reason}]
            }));
            let (result, events) = stream_test_response(response).await;
            assert!(result.is_ok());
            assert_eq!(events.iter().filter(|event| matches!(event, StreamChunk::ToolCall { .. })).count(), 1);
            assert!(matches!(events.last(), Some(StreamChunk::Done)));
        }
        let (result, events) = stream_test_response(format!("{}data: [DONE]\n\n", tool_stream_frame("{}"))).await;
        assert!(result.is_ok());
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
            assert!(headers.contains("user-agent: canbanvoimaxharness/"));
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
        assert_eq!(captured[2].1["messages"], serde_json::to_value(&messages).unwrap());
        assert!(messages.iter().all(|message| message.cache_control.is_some()));
    }

    #[test]
    fn cache_usage_normalizes_provider_accounting() {
        for usage in [
            serde_json::json!({"prompt_tokens": 100000, "prompt_cache_hit_tokens": 99000, "prompt_cache_miss_tokens": 1000}),
            serde_json::json!({"prompt_cache_hit_tokens": 99000, "prompt_cache_miss_tokens": 1000}),
            serde_json::json!({"input_tokens": 700, "cache_read_input_tokens": 99000, "cache_creation_input_tokens": 300}),
            serde_json::json!({"input_tokens": 100000, "input_tokens_details": {"cached_tokens": 99000}}),
        ] {
            let total = extract_openai_input_tokens(&usage);
            let cached = extract_openai_cache_read(&usage);
            assert_eq!(total, 100000);
            assert_eq!(cached, 99000);
            assert_eq!(crate::agent::compute_cache_hit_rate(cached, total), 99.0);
        }
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

    #[test]
    fn test_extract_openai_cache_read_openai_shape() {
        let usage = serde_json::json!({
            "prompt_tokens": 15000,
            "completion_tokens": 100,
            "total_tokens": 15100,
            "prompt_tokens_details": { "cached_tokens": 12000, "audio_tokens": 0 }
        });
        assert_eq!(extract_openai_input_tokens(&usage), 15000);
        assert_eq!(extract_openai_output_tokens(&usage), 100);
        assert_eq!(extract_openai_cache_read(&usage), 12000);
    }

    #[test]
    fn test_extract_openai_cache_read_fallback_shapes() {
        // Responses-API shape.
        let responses = serde_json::json!({
            "input_tokens": 15000,
            "output_tokens": 100,
            "input_tokens_details": { "cached_tokens": 9000 }
        });
        assert_eq!(extract_openai_input_tokens(&responses), 15000);
        assert_eq!(extract_openai_output_tokens(&responses), 100);
        assert_eq!(extract_openai_cache_read(&responses), 9000);

        // Gemini-native passthrough shape.
        let gemini = serde_json::json!({
            "prompt_tokens": 15000,
            "completion_tokens": 100,
            "cached_content_token_count": 7000
        });
        assert_eq!(extract_openai_cache_read(&gemini), 7000);

        // Proxy that drops all cache counters: must parse as 0, not fail.
        let bare = serde_json::json!({
            "prompt_tokens": 15000,
            "completion_tokens": 100,
            "total_tokens": 15100
        });
        assert_eq!(extract_openai_cache_read(&bare), 0);
        assert_eq!(extract_openai_cache_creation(&bare), 0);
    }

    #[test]
    fn test_token_usage_stats_cache_hit_rate_safe_division_by_zero() {
        let mut stats = crate::llm::TokenUsageStats::default();
        stats.compute_hit_rate();
        assert_eq!(stats.cache_hit_rate, 0.0);

        stats.input_tokens = 1000;
        stats.cache_read_input_tokens = 500;
        stats.compute_hit_rate();
        assert!((stats.cache_hit_rate - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_stream_options_serialization() {
        let req = crate::llm::OpenAIChatRequest {
            model: "test-model".to_string(),
            messages: vec![],
            temperature: None,
            stream: true,
            tools: None,
            reasoning_effort: None,
            stream_options: Some(crate::llm::StreamOptions { include_usage: true }),
        };
        let serialized = serde_json::to_string(&req).unwrap();
        assert!(serialized.contains(r#""stream_options":{"include_usage":true}"#));
    }
}

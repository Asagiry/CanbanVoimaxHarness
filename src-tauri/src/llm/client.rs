use super::models::*;
use anyhow::{bail, Result};
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct LlmClient {
    client: reqwest::Client,
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Filter and sort tools deterministically based on allowed tools policy.
/// Disabling unused tools saves prompt tokens and alphabetical sorting guarantees KV prefix cache hits.
pub fn filter_active_tools(
    tools: &[ToolDefinition],
    allowed_tools: Option<&[String]>,
) -> Vec<ToolDefinition> {
    let mut filtered: Vec<ToolDefinition> = if let Some(allowed) = allowed_tools {
        tools
            .iter()
            .filter(|tool| {
                let name = &tool.function.name;
                allowed.iter().any(|a| {
                    let canon_allowed = match a.as_str() {
                        "bash" => "execute_command",
                        "str_replace" => "edit_file",
                        other => other,
                    };
                    let canon_tool = match name.as_str() {
                        "bash" => "execute_command",
                        "str_replace" => "edit_file",
                        other => other,
                    };
                    canon_allowed == canon_tool || a == name
                })
            })
            .cloned()
            .collect()
    } else {
        tools.to_vec()
    };

    // Deterministic Prefix Caching: Alphabetical sorting of tool definitions
    filtered.sort_by(|a, b| a.function.name.cmp(&b.function.name));
    filtered
}

/// Robust usage extractors: different OpenAI-compatible proxies report cache
/// telemetry under different keys (OpenAI `prompt_tokens_details.cached_tokens`,
/// Responses-API `input_tokens_details.cached_tokens`, top-level `cached_tokens`,
/// Gemini-native `cached_content_token_count`, Anthropic-style
/// `cache_read_input_tokens`). Try each in order; missing keys yield 0.
/// Pure functions so prefix/telemetry behaviour is unit-testable.
pub fn extract_openai_input_tokens(usage: &serde_json::Value) -> u64 {
    usage
        .get("prompt_tokens")
        .and_then(|v| v.as_u64())
        .or_else(|| usage.get("input_tokens").and_then(|v| v.as_u64()))
        .unwrap_or(0)
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

/// Converts ToolDefinitions to Anthropic native tools with alphabetical sorting
/// and optional ephemeral prompt caching on the final tool.
pub fn prepare_anthropic_tools(
    tools: &[ToolDefinition],
    enable_cache: bool,
) -> Vec<AnthropicTool> {
    let mut anthropic_tools: Vec<AnthropicTool> = tools
        .iter()
        .map(|t| AnthropicTool {
            name: t.function.name.clone(),
            description: t.function.description.clone(),
            input_schema: t.function.parameters.clone(),
            cache_control: None,
        })
        .collect();

    anthropic_tools.sort_by(|a, b| a.name.cmp(&b.name));

    if enable_cache {
        if let Some(last) = anthropic_tools.last_mut() {
            last.cache_control = Some(serde_json::json!({ "type": "ephemeral" }));
        }
    }

    anthropic_tools
}

/// Prepare Anthropic system prompt with ephemeral prompt cache control.
pub fn prepare_anthropic_system(system_prompt: &str, enable_cache: bool) -> serde_json::Value {
    if enable_cache {
        serde_json::json!([
            {
                "type": "text",
                "text": system_prompt,
                "cache_control": { "type": "ephemeral" }
            }
        ])
    } else {
        serde_json::Value::String(system_prompt.to_string())
    }
}

impl LlmClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .build()
                .expect("Failed to build reqwest Client"),
        }
    }

    /// Stream chat from OpenAI-compatible endpoint with automatic reconnect resilience (alias to stream_openai).
    pub async fn stream_chat(
        &self,
        base_url: &str,
        api_key: Option<&str>,
        request: OpenAIChatRequest,
        tx: Sender<StreamChunk>,
    ) -> Result<()> {
        self.stream_openai(base_url, api_key, request, tx).await
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

        let endpoint = if base_url.ends_with("/chat/completions") {
            base_url.to_string()
        } else if base_url.ends_with('/') {
            format!("{}chat/completions", base_url)
        } else {
            format!("{}/chat/completions", base_url)
        };

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(key) = api_key {
            if !key.trim().is_empty() {
                if let Ok(val) = HeaderValue::from_str(&format!("Bearer {}", key.trim())) {
                    headers.insert(AUTHORIZATION, val);
                }
            }
        }

        let mut attempt = 0;
        let max_reconnects = 10;
        let mut response = None;

        loop {
            let req = self
                .client
                .post(&endpoint)
                .headers(headers.clone())
                .json(&request);

            match req.send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        response = Some(resp);
                        break;
                    }

                    let status = resp.status();
                    let text = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Failed to read error body".to_string());
                    let err_summary = if text.len() > 120 {
                        format!("{}...", &text[..120])
                    } else {
                        text.clone()
                    };
                    let err_msg = format!("HTTP {}: {}", status, err_summary);

                    if status.is_server_error() && attempt < max_reconnects {
                        attempt += 1;
                        let delay_secs = attempt * 10;
                        let notice = format!(
                            "\n[Связь с LLM API недоступна ({}). Повторная попытка {}/10 через {}с...]\n",
                            err_msg, attempt, delay_secs
                        );
                        eprintln!("CVH [llm_client]: {}", notice);
                        let _ = tx.send(StreamChunk::Output(notice)).await;
                        tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs as u64)).await;
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
                    if attempt < max_reconnects {
                        attempt += 1;
                        let delay_secs = attempt * 10;
                        let notice = format!(
                            "\n[Связь с LLM API недоступна ({}). Повторная попытка {}/10 через {}с...]\n",
                            err_msg, attempt, delay_secs
                        );
                        eprintln!("CVH [llm_client]: {}", notice);
                        let _ = tx.send(StreamChunk::Output(notice)).await;
                        tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs as u64)).await;
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
        }

        let response = match response {
            Some(r) => r,
            None => {
                let final_err = format!("Failed to connect to OpenAI endpoint {} after {} attempts", endpoint, max_reconnects);
                let _ = tx.send(StreamChunk::Error(final_err.clone())).await;
                bail!(final_err);
            }
        };

        let mut stream = response.bytes_stream().eventsource();
        let mut tool_calls_in_progress: HashMap<usize, (String, String, String)> = HashMap::new();

        while let Some(item) = stream.next().await {
            match item {
                Ok(event) => {
                    let data = event.data.trim();
                    if data == "[DONE]" {
                        break;
                    }

                    if let Ok(chunk) = serde_json::from_str::<OpenAIChatChunk>(data) {
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
                            let _ = tx.send(StreamChunk::Usage {
                                input_tokens,
                                output_tokens,
                                cache_read_tokens: cache_read,
                                cache_creation_tokens: cache_creation,
                            }).await;
                        }

                        if let Some(choices) = chunk.choices {
                            for choice in choices {
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
                    }
                }
                Err(err) => {
                    let err_msg = format!("SSE stream decode error: {}", err);
                    let _ = tx.send(StreamChunk::Error(err_msg.clone())).await;
                    bail!(err_msg);
                }
            }
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

        let _ = tx.send(StreamChunk::Done).await;
        Ok(())
    }

    /// Stream Anthropic Claude Native API (/v1/messages).
    pub async fn stream_anthropic(
        &self,
        base_url: &str,
        api_key: Option<&str>,
        mut request: AnthropicMessageRequest,
        tx: Sender<StreamChunk>,
    ) -> Result<()> {
        // Deterministic Prefix Caching: alphabetically sort tools by name
        if let Some(tools) = &mut request.tools {
            tools.sort_by(|a, b| a.name.cmp(&b.name));
            // Breakpoint: Inject ephemeral cache_control on the last tool
            if let Some(last_tool) = tools.last_mut() {
                last_tool.cache_control = Some(serde_json::json!({ "type": "ephemeral" }));
            }
        }

        // Breakpoint: Inject ephemeral cache_control on static System prompt
        if let Some(sys_val) = request.system.take() {
            if let Some(sys_str) = sys_val.as_str() {
                request.system = Some(serde_json::json!([
                    {
                        "type": "text",
                        "text": sys_str,
                        "cache_control": { "type": "ephemeral" }
                    }
                ]));
            } else {
                request.system = Some(sys_val);
            }
        }

        // Breakpoint: Inject ephemeral cache_control on initial task prompt (Anchor message)
        if let Some(first_msg) = request.messages.first_mut() {
            if let Some(text) = first_msg.content.as_str() {
                first_msg.content = serde_json::json!([
                    {
                        "type": "text",
                        "text": text,
                        "cache_control": { "type": "ephemeral" }
                    }
                ]);
            }
        }

        let endpoint = if base_url.ends_with("/messages") {
            base_url.to_string()
        } else if base_url.ends_with('/') {
            format!("{}messages", base_url)
        } else {
            format!("{}/messages", base_url)
        };

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            HeaderName::from_static("anthropic-version"),
            HeaderValue::from_static("2023-06-01"),
        );
        if let Some(key) = api_key {
            if !key.trim().is_empty() {
                if let Ok(val) = HeaderValue::from_str(key.trim()) {
                    headers.insert(HeaderName::from_static("x-api-key"), val);
                }
            }
        }

        let mut attempt = 0;
        let max_reconnects = 10;
        let mut response = None;

        loop {
            let req = self
                .client
                .post(&endpoint)
                .headers(headers.clone())
                .json(&request);

            match req.send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        response = Some(resp);
                        break;
                    }

                    let status = resp.status();
                    let text = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Failed to read error body".to_string());
                    let err_summary = if text.len() > 120 {
                        format!("{}...", &text[..120])
                    } else {
                        text.clone()
                    };
                    let err_msg = format!("HTTP {}: {}", status, err_summary);

                    if status.is_server_error() && attempt < max_reconnects {
                        attempt += 1;
                        let delay_secs = attempt * 10;
                        let notice = format!(
                            "\n[Связь с LLM API недоступна ({}). Повторная попытка {}/10 через {}с...]\n",
                            err_msg, attempt, delay_secs
                        );
                        eprintln!("CVH [llm_client]: {}", notice);
                        let _ = tx.send(StreamChunk::Output(notice)).await;
                        tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs as u64)).await;
                        if tx.is_closed() {
                            bail!("Stream channel closed by receiver");
                        }
                        continue;
                    } else {
                        let final_err = format!("Anthropic API error [{}]: {}", status, text);
                        eprintln!("CVH [llm_client]: {}", final_err);
                        let _ = tx.send(StreamChunk::Error(final_err.clone())).await;
                        bail!(final_err);
                    }
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    if attempt < max_reconnects {
                        attempt += 1;
                        let delay_secs = attempt * 10;
                        let notice = format!(
                            "\n[Связь с LLM API недоступна ({}). Повторная попытка {}/10 через {}с...]\n",
                            err_msg, attempt, delay_secs
                        );
                        eprintln!("CVH [llm_client]: {}", notice);
                        let _ = tx.send(StreamChunk::Output(notice)).await;
                        tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs as u64)).await;
                        if tx.is_closed() {
                            bail!("Stream channel closed by receiver");
                        }
                        continue;
                    } else {
                        let final_err = format!(
                            "Failed to connect to Anthropic endpoint {} after {} attempts: {}",
                            endpoint, attempt, err_msg
                        );
                        eprintln!("CVH [llm_client]: {}", final_err);
                        let _ = tx.send(StreamChunk::Error(final_err.clone())).await;
                        bail!(final_err);
                    }
                }
            }
        }

        let response = match response {
            Some(r) => r,
            None => {
                let final_err = format!("Failed to connect to Anthropic endpoint {} after {} attempts", endpoint, max_reconnects);
                let _ = tx.send(StreamChunk::Error(final_err.clone())).await;
                bail!(final_err);
            }
        };

        let mut stream = response.bytes_stream().eventsource();
        let mut tool_blocks: HashMap<usize, (String, String, String)> = HashMap::new();

        while let Some(item) = stream.next().await {
            match item {
                Ok(event) => {
                    let data = event.data.trim();
                    if let Ok(event_obj) = serde_json::from_str::<AnthropicStreamEvent>(data) {
                        match event_obj {
                            AnthropicStreamEvent::MessageStart { message } => {
                                if let Some(usage) = message.get("usage") {
                                    let input_tokens = usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                                    let cache_read = usage.get("cache_read_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                                    let cache_creation = usage.get("cache_creation_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                                    let output_tokens = usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                                    let _ = tx.send(StreamChunk::Usage {
                                        input_tokens,
                                        output_tokens,
                                        cache_read_tokens: cache_read,
                                        cache_creation_tokens: cache_creation,
                                    }).await;
                                }
                            }
                            AnthropicStreamEvent::ContentBlockStart {
                                index,
                                content_block,
                            } => {
                                let block_type = content_block
                                    .get("type")
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("");
                                if block_type == "tool_use" {
                                    let id = content_block
                                        .get("id")
                                        .and_then(|s| s.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let name = content_block
                                        .get("name")
                                        .and_then(|s| s.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    tool_blocks.insert(index, (id, name, String::new()));
                                }
                            }
                            AnthropicStreamEvent::ContentBlockDelta { index, delta } => {
                                let delta_type = delta.get("type").and_then(|t| t.as_str()).unwrap_or("");
                                match delta_type {
                                    "text_delta" => {
                                        if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                            let _ = tx.send(StreamChunk::Token(text.to_string())).await;
                                        }
                                    }
                                    "thinking_delta" => {
                                        if let Some(thinking) = delta.get("thinking").and_then(|t| t.as_str()) {
                                            let _ = tx.send(StreamChunk::Thought(thinking.to_string())).await;
                                        }
                                    }
                                    "input_json_delta" => {
                                        if let Some(json_partial) =
                                            delta.get("partial_json").and_then(|t| t.as_str())
                                        {
                                            if let Some(tool) = tool_blocks.get_mut(&index) {
                                                tool.2.push_str(json_partial);
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            AnthropicStreamEvent::ContentBlockStop { index } => {
                                if let Some((id, name, args)) = tool_blocks.remove(&index) {
                                    let _ = tx
                                        .send(StreamChunk::ToolCall {
                                            id,
                                            name,
                                            arguments: args,
                                        })
                                        .await;
                                }
                            }
                            AnthropicStreamEvent::MessageStop => {
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Err(err) => {
                    let err_msg = format!("Anthropic SSE stream error: {}", err);
                    let _ = tx.send(StreamChunk::Error(err_msg.clone())).await;
                    bail!(err_msg);
                }
            }
        }

        let _ = tx.send(StreamChunk::Done).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

use super::models::*;
use anyhow::{bail, Result};
use eventsource_stream::{Eventsource, EventStreamError};
use futures_util::StreamExt;
use reqwest::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::mpsc::Sender;

use crate::llm::responses_event::handle_responses_sse_event;
pub use crate::llm::usage::*;

static SHARED_LLM_HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .tcp_keepalive(Duration::from_secs(30))
        .build()
        .unwrap_or_default()
});

static SECRET_REDACT_PATTERNS: LazyLock<Vec<(regex::Regex, &'static str)>> = LazyLock::new(|| {
    let specs = [
        (r"(?i)(Bearer\s+)[A-Za-z0-9._~+/=-]{8,}", "${1}[REDACTED]"),
        (r"(?i)((?:api[_-]?key|x-api-key|authorization|secret|password|token)\s*[:=]\s*['\x22]?)[^\s'\x22,;}{]{6,}", "${1}[REDACTED]"),
        (r"([?&](?:api_key|key|token)=)[^&\s]+", "${1}[REDACTED]"),
        (r"\b(?:sk-(?:proj-|ant-)?[A-Za-z0-9_-]{12,}|AIza[A-Za-z0-9_-]{16,}|gh[po]_[A-Za-z0-9]{16,}|AKIA[0-9A-Z]{16}|xox[baprs]-[A-Za-z0-9-]{10,})\b", "[REDACTED]"),
    ];
    specs
        .into_iter()
        .filter_map(|(pat, rep)| regex::Regex::new(pat).ok().map(|re| (re, rep)))
        .collect()
});

/// Masks API keys, Bearer tokens, and secret query/header values from logs and errors.
pub fn redact_secrets(input: &str, extra_secret: Option<&str>) -> String {
    let mut out = input.to_string();
    if let Some(secret) = extra_secret.map(str::trim).filter(|s| s.len() >= 4) {
        out = out.replace(secret, "[REDACTED]");
    }
    for (re, rep) in SECRET_REDACT_PATTERNS.iter() {
        out = re.replace_all(&out, *rep).into_owned();
    }
    out
}

#[derive(Debug, Clone)]
pub struct LlmClient {
    pub client: reqwest::Client,
    pub session: String,
    pub prefix: std::sync::Arc<std::sync::Mutex<super::cache::PrefixTracker>>,
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmClient {
    pub fn new() -> Self {
        Self {
            client: SHARED_LLM_HTTP_CLIENT.clone(),
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
        for message in &mut request.messages {
            message.cache_control = None;
        }
        self.prefix.lock().map_err(|_| anyhow::anyhow!("Prompt cache tracker lock poisoned"))?
            .observe(&request)?;

        let use_responses = super::cache::uses_responses_api(base_url, &request.model);
        let endpoint = if use_responses {
            if base_url.ends_with("/responses") {
                base_url.to_string()
            } else if base_url.ends_with('/') {
                format!("{}responses", base_url)
            } else {
                format!("{}/responses", base_url)
            }
        } else if base_url.ends_with("/chat/completions") {
            base_url.to_string()
        } else if base_url.ends_with('/') {
            format!("{}chat/completions", base_url)
        } else {
            format!("{}/chat/completions", base_url)
        };
        let body_json = if use_responses {
            super::cache::build_responses_payload(&request)
        } else {
            serde_json::to_value(&request)?
        };

        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(key) = api_key {
            if !key.trim().is_empty() {
                if let Ok(val) = HeaderValue::from_str(&format!("Bearer {}", key.trim())) {
                    headers.insert(AUTHORIZATION, val);
                }
            }
        }

        const MAX_STREAM_RETRIES: usize = 4;
        const HEADER_TIMEOUT: Duration = Duration::from_secs(60);
        let mut attempt = 0usize;
        let mut delay_secs = 5_u64;
        loop {
            let id = uuid::Uuid::new_v4().to_string();
            if tx.send(StreamChunk::AttemptStart { id: id.clone() }).await.is_err() {
                return Ok(());
            }
            let req = self
                .client
                .post(&endpoint)
                .headers(headers.clone())
                .json(&body_json);

            let response_res = tokio::select! {
                _ = tx.closed() => return Ok(()),
                res = tokio::time::timeout(HEADER_TIMEOUT, req.send()) => res,
            };
            let retry_reason = match response_res {
                Ok(Ok(resp)) if resp.status().is_success() => {
                    match Self::consume_openai_response(resp, &tx).await? {
                        Some(reason) => reason,
                        None => return Ok(()),
                    }
                }
                Ok(Ok(resp)) => {
                    let status = resp.status();
                    let text = tokio::select! {
                        _ = tx.closed() => return Ok(()),
                        text = resp.text() => text.unwrap_or_else(|_| "Failed to read error body".to_string()),
                    };
                    let redacted_text = redact_secrets(&text, api_key);
                    let err_summary = if redacted_text.chars().count() > 120 {
                        format!("{}...", redacted_text.chars().take(120).collect::<String>())
                    } else {
                        redacted_text.clone()
                    };
                    if status.is_server_error() {
                        format!("HTTP {}: {}", status, err_summary)
                    } else {
                        let final_err = format!("OpenAI API error [{}]: {}", status, redacted_text);
                        eprintln!("CVH [llm_client]: {}", final_err);
                        let _ = tx.send(StreamChunk::Error(final_err.clone())).await;
                        bail!(final_err);
                    }
                }
                Ok(Err(e)) => {
                    let redacted_err = redact_secrets(&e.to_string(), api_key);
                    if e.is_builder() || e.is_redirect() {
                        let final_err = format!("Invalid OpenAI request: {redacted_err}");
                        let _ = tx.send(StreamChunk::Error(final_err.clone())).await;
                        bail!(final_err);
                    }
                    redacted_err
                }
                Err(_) => "HTTP response header timeout (60s)".to_string(),
            };
            let redacted_reason = redact_secrets(&retry_reason, api_key);
            if attempt >= MAX_STREAM_RETRIES {
                let final_err = format!(
                    "OpenAI stream failed after {} attempts: {}",
                    attempt + 1,
                    redacted_reason
                );
                eprintln!("CVH [llm_client]: {}", final_err);
                let _ = tx.send(StreamChunk::Error(final_err.clone())).await;
                bail!(final_err);
            }
            attempt += 1;
            eprintln!(
                "CVH [llm_client]: {} (retry {}/{}). Reconnecting in {}s",
                redacted_reason, attempt, MAX_STREAM_RETRIES, delay_secs
            );
            if tx.send(StreamChunk::Retry { id, delay_secs }).await.is_err() {
                return Ok(());
            }
            tokio::select! {
                _ = tx.closed() => return Ok(()),
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs)) => {},
            }
            delay_secs = delay_secs.saturating_mul(2).min(30);
        }
    }

    async fn consume_openai_response(
        response: reqwest::Response,
        tx: &Sender<StreamChunk>,
    ) -> Result<Option<String>> {
        let mut stream = response.bytes_stream().eventsource();
        let mut tool_calls_in_progress: HashMap<usize, (String, String, String)> = HashMap::new();
        let mut next_tool_call_fallback: usize = 0;
        let mut first_chunk_sent = false;
        let mut streamed_reasoning_delta = false;
        let mut pending_usage: Option<StreamChunk> = None;
        let mut terminal_marker = false;
        let mut stream_error = None;
        let mut transport_error = None;

        loop {
            let item = tokio::select! {
                _ = tx.closed() => return Ok(None),
                res = tokio::time::timeout(Duration::from_secs(60), stream.next()) => match res {
                    Ok(Some(item)) => item,
                    Ok(None) => break,
                    Err(_) => {
                        transport_error = Some("Stream timeout (no data for 60s)".to_string());
                        break;
                    }
                },
            };
            match item {
                Ok(event) => {
                    let data = event.data.trim();
                    if data.is_empty() {
                        continue;
                    }
                    if data == "[DONE]" {
                        terminal_marker = true;
                        break;
                    }

                    let Ok(payload) = serde_json::from_str::<serde_json::Value>(data) else {
                        stream_error = Some("Invalid JSON in LLM stream".to_string());
                        break;
                    };
                    if let Some(error) = payload.get("error").filter(|error| !error.is_null()) {
                        stream_error = Some(format!("LLM stream error: {error}"));
                        break;
                    }
                    if ["/stop_reason", "/delta/stop_reason", "/message/stop_reason"].iter()
                        .any(|path| payload.pointer(path).and_then(|value| value.as_str()) == Some("max_tokens"))
                    {
                        stream_error.get_or_insert_with(|| "LLM response incomplete: max_tokens".to_string());
                    }

                    if let Some(ev_type) = payload
                        .get("type")
                        .and_then(|v| v.as_str())
                        .filter(|t| t.starts_with("response."))
                    {
                        if !first_chunk_sent {
                            first_chunk_sent = true;
                            let _ = tx.send(StreamChunk::FirstChunk).await;
                        }
                        handle_responses_sse_event(
                            ev_type,
                            &payload,
                            tx,
                            &mut tool_calls_in_progress,
                            &mut next_tool_call_fallback,
                            &mut pending_usage,
                            &mut terminal_marker,
                            &mut stream_error,
                            &mut streamed_reasoning_delta,
                        )
                        .await;
                        if stream_error.is_some() {
                            break;
                        }
                        continue;
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
                                        stream_error.get_or_insert_with(|| format!("LLM response incomplete: {reason}"));
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
                                        let idx = resolve_tool_call_index(
                                            &tool_calls_in_progress,
                                            tc.index,
                                            tc.id.as_deref(),
                                            &mut next_tool_call_fallback,
                                        );
                                        if let Some(existing) = tool_calls_in_progress.get(&idx) {
                                            if let Some(new_id) = tc.id.as_deref().filter(|s| !s.is_empty()) {
                                                if !existing.0.is_empty() && existing.0 != new_id {
                                                    let mut fresh = idx;
                                                    while tool_calls_in_progress.contains_key(&fresh) {
                                                        fresh = fresh.saturating_add(1);
                                                    }
                                                    eprintln!(
                                                        "CVH [llm_client] tool call id changed at idx={idx} ({} -> {}); split into idx={fresh}",
                                                        existing.0, new_id
                                                    );
                                                    let entry = tool_calls_in_progress
                                                        .entry(fresh)
                                                        .or_insert_with(|| (String::new(), String::new(), String::new()));
                                                    entry.0.push_str(new_id);
                                                    if let Some(func) = tc.function {
                                                        if let Some(name) = func.name {
                                                            entry.1.push_str(&name);
                                                        }
                                                        if let Some(args) = func.arguments {
                                                            entry.2.push_str(&args);
                                                        }
                                                    }
                                                    continue;
                                                }
                                            }
                                        }
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
                        break;
                    }
                }
                Err(EventStreamError::Transport(err)) => {
                    transport_error = Some(format!("SSE transport error: {err}"));
                    break;
                }
                Err(err) => {
                    stream_error = Some(format!("SSE stream decode error: {}", err));
                    break;
                }
            }
        }

        if stream_error.is_none() && !terminal_marker {
            if let Some(usage_chunk) = pending_usage.take() {
                let _ = tx.send(usage_chunk).await;
            }
            return Ok(Some(transport_error.unwrap_or_else(|| "LLM stream ended before a completion marker".to_string())));
        }
        for (_, _, args) in tool_calls_in_progress.values_mut() {
            if args.trim().is_empty() {
                *args = "{}".to_string();
            }
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
        Ok(None)
    }
}

#[cfg(test)]
#[path = "client_tests/mod.rs"]
mod tests;

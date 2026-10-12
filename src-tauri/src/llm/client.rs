use super::models::*;
use anyhow::{bail, Context, Result};
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
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

impl LlmClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .build()
                .expect("Failed to build reqwest Client"),
        }
    }

    /// Stream an OpenAI-compatible endpoint (OpenAI, Local Antigravity proxy, DeepSeek, Ollama).
    pub async fn stream_openai(
        &self,
        base_url: &str,
        api_key: Option<&str>,
        request: OpenAIChatRequest,
        tx: Sender<StreamChunk>,
    ) -> Result<()> {
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

        let response = self
            .client
            .post(&endpoint)
            .headers(headers)
            .json(&request)
            .send()
            .await
            .with_context(|| format!("Failed to send request to OpenAI endpoint: {}", endpoint))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error body".to_string());
            let err_msg = format!("OpenAI API error [{}]: {}", status, text);
            let _ = tx.send(StreamChunk::Error(err_msg.clone())).await;
            bail!(err_msg);
        }

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
                        if let Some(choices) = chunk.choices {
                            for choice in choices {
                                // Content delta
                                if let Some(content) = choice.delta.content {
                                    if !content.is_empty() {
                                        let _ = tx.send(StreamChunk::Token(content)).await;
                                    }
                                }

                                // Reasoning / Thought delta
                                if let Some(reasoning) = choice.delta.reasoning_content {
                                    if !reasoning.is_empty() {
                                        let _ = tx.send(StreamChunk::Thought(reasoning)).await;
                                    }
                                } else if let Some(thought) = choice.delta.thought {
                                    if !thought.is_empty() {
                                        let _ = tx.send(StreamChunk::Thought(thought)).await;
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
        request: AnthropicMessageRequest,
        tx: Sender<StreamChunk>,
    ) -> Result<()> {
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
            HeaderValue::from_static("anthropic-version"),
            HeaderValue::from_static("2023-06-01"),
        );
        if let Some(key) = api_key {
            if !key.trim().is_empty() {
                if let Ok(val) = HeaderValue::from_str(key.trim()) {
                    headers.insert(HeaderValue::from_static("x-api-key"), val);
                }
            }
        }

        let response = self
            .client
            .post(&endpoint)
            .headers(headers)
            .json(&request)
            .send()
            .await
            .with_context(|| format!("Failed to send request to Anthropic endpoint: {}", endpoint))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error body".to_string());
            let err_msg = format!("Anthropic API error [{}]: {}", status, text);
            let _ = tx.send(StreamChunk::Error(err_msg.clone())).await;
            bail!(err_msg);
        }

        let mut stream = response.bytes_stream().eventsource();
        let mut tool_blocks: HashMap<usize, (String, String, String)> = HashMap::new();

        while let Some(item) = stream.next().await {
            match item {
                Ok(event) => {
                    let data = event.data.trim();
                    if let Ok(event_obj) = serde_json::from_str::<AnthropicStreamEvent>(data) {
                        match event_obj {
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

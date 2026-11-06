use super::{ContentPart, MessageContent, OpenAIChatRequest, Role};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, USER_AGENT};
use std::hash::{Hash, Hasher};

pub const OPENCODE_USER_AGENT: &str = "opencode/latest/2.2.5/desktop";

pub fn opencode_headers(base_url: &str, session: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if !reqwest::Url::parse(base_url).is_ok_and(|url| url.host_str() == Some("opencode.ai")) {
        return headers;
    }
    if let Some(value) = opencode_session_header(base_url, session) {
        if let Ok(header_value) = HeaderValue::from_str(&value) {
            headers.insert("x-opencode-session", header_value);
        }
    }
    headers.insert("x-opencode-client", HeaderValue::from_static("desktop"));
    headers.insert(USER_AGENT, HeaderValue::from_static(OPENCODE_USER_AGENT));
    headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
    headers
}

pub fn opencode_session_header(base_url: &str, session: &str) -> Option<String> {
    if !reqwest::Url::parse(base_url).is_ok_and(|url| url.host_str() == Some("opencode.ai")) {
        return None;
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    session.hash(&mut hasher);
    Some(format!("cvh-{:016x}", hasher.finish()))
}

pub fn uses_responses_api(base_url: &str, model: &str) -> bool {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with("/responses") {
        return true;
    }
    if trimmed.ends_with("/chat/completions") {
        return false;
    }
    let m = model.trim().to_ascii_lowercase();
    m.starts_with("crouton") || m.contains("/crouton") || m.contains("codex")
}

pub fn build_responses_payload(request: &OpenAIChatRequest) -> serde_json::Value {
    let mut system_parts = Vec::new();
    let mut input = Vec::new();

    for msg in &request.messages {
        match msg.role {
            Role::System => {
                if let Some(text) = msg.text_content().filter(|t| !t.is_empty()) {
                    system_parts.push(text.to_string());
                }
            }
            Role::User => match &msg.content {
                Some(MessageContent::Parts(parts)) => {
                    let mapped: Vec<serde_json::Value> = parts
                        .iter()
                        .map(|part| match part {
                            ContentPart::Text { text } => {
                                serde_json::json!({ "type": "input_text", "text": text })
                            }
                            ContentPart::ImageUrl { image_url } => {
                                serde_json::json!({ "type": "input_image", "image_url": image_url.url })
                            }
                        })
                        .collect();
                    input.push(serde_json::json!({ "role": "user", "content": mapped }));
                }
                Some(MessageContent::Text(text)) => {
                    input.push(serde_json::json!({ "role": "user", "content": text }));
                }
                None => {}
            },
            Role::Assistant => {
                if let Some(text) = msg.text_content().filter(|t| !t.is_empty()) {
                    input.push(serde_json::json!({
                        "role": "assistant",
                        "content": [{ "type": "output_text", "text": text }],
                    }));
                }
                if let Some(tool_calls) = &msg.tool_calls {
                    for tc in tool_calls {
                        input.push(serde_json::json!({
                            "type": "function_call",
                            "call_id": tc.id,
                            "name": tc.function.name,
                            "arguments": tc.function.arguments,
                        }));
                    }
                }
            }
            Role::Tool => {
                input.push(serde_json::json!({
                    "type": "function_call_output",
                    "call_id": msg.tool_call_id.as_deref().unwrap_or_default(),
                    "output": msg.text_content().unwrap_or_default(),
                }));
            }
        }
    }

    let effort = request
        .reasoning_effort
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("medium");

    let mut payload = serde_json::json!({
        "model": request.model,
        "input": input,
        "stream": request.stream,
        "reasoning": {
            "effort": effort,
            "summary": "auto",
        },
    });

    if let Some(obj) = payload.as_object_mut() {
        if !system_parts.is_empty() {
            obj.insert(
                "instructions".to_string(),
                serde_json::Value::String(system_parts.join("\n\n")),
            );
        }
        if let Some(temp) = request.temperature {
            obj.insert("temperature".to_string(), serde_json::json!(temp));
        }
        if let Some(tools) = &request.tools {
            let mapped_tools: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "type": t.tool_type,
                        "name": t.function.name,
                        "description": t.function.description,
                        "parameters": t.function.parameters,
                    })
                })
                .collect();
            obj.insert("tools".to_string(), serde_json::Value::Array(mapped_tools));
        }
    }

    payload
}

#[derive(Debug, Default)]
pub struct PrefixTracker {
    model: String,
    tools: String,
    messages: Vec<String>,
}

impl PrefixTracker {
    pub fn observe(&mut self, request: &OpenAIChatRequest) -> Result<(), serde_json::Error> {
        let tools = serde_json::to_string(&request.tools)?;
        let messages: Vec<String> = request.messages.iter()
            .map(serde_json::to_string).collect::<Result<_, _>>()?;
        let anchors_match = self.model == request.model && self.tools == tools;
        let common_messages = if anchors_match {
            self.messages.iter().zip(&messages).take_while(|(old, new)| old == new).count()
        } else {
            0
        };
        let prefix_preserved = anchors_match && !self.messages.is_empty() && common_messages == self.messages.len();
        let common_bytes: usize = messages.iter().take(common_messages).map(String::len).sum();
        eprintln!(
            "CVH [cache] model={} previous_messages={} messages={} anchors_match={} prefix_preserved={} common_messages={} common_bytes={} first_changed_message={:?}",
            request.model, self.messages.len(), messages.len(), anchors_match, prefix_preserved,
            common_messages, common_bytes,
            if common_messages < self.messages.len() { Some(common_messages) } else { None },
        );
        self.model.clone_from(&request.model);
        self.tools = tools;
        self.messages = messages;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{ChatMessage, FunctionCall, ToolCall, ToolDefinition};

    #[test]
    fn cache_routing_is_stable_and_scoped_to_opencode() {
        let first = opencode_headers("https://opencode.ai/inference/go/openai/v1", "project:task");
        assert_eq!(first, opencode_headers("https://opencode.ai/inference/go/openai/v1", "project:task"));
        assert_ne!(first["x-opencode-session"], opencode_headers("https://opencode.ai", "project:child")["x-opencode-session"]);
        assert_eq!(first["x-opencode-client"], "desktop");
        assert_eq!(first[USER_AGENT], OPENCODE_USER_AGENT);
        assert_eq!(first[ACCEPT], "*/*");
        assert!(opencode_headers("https://opencode.ai.example.com/v1", "project:task").is_empty());
    }

    #[test]
    fn responses_payload_preserves_tool_calls_and_outputs() {
        assert!(uses_responses_api("https://opencode.ai/inference/go/openai/v1", "crouton-alpha-free"));
        assert!(!uses_responses_api("https://opencode.ai/inference/go/openai/v1", "beam-free"));

        let req = OpenAIChatRequest {
            model: "crouton-alpha-free".into(),
            messages: vec![
                ChatMessage::system("sys prompt"),
                ChatMessage::user("read file"),
                ChatMessage::assistant(
                    Some("Reading now.".into()),
                    Some(vec![ToolCall {
                        id: "call_1".into(),
                        call_type: "function".into(),
                        function: FunctionCall {
                            name: "read_file".into(),
                            arguments: "{\"file_path\":\"a.rs\"}".into(),
                        },
                    }]),
                ),
                ChatMessage::tool_response("call_1", "fn main() {}"),
            ],
            temperature: Some(0.2),
            stream: true,
            tools: Some(vec![ToolDefinition::function(
                "read_file",
                "Read a file",
                serde_json::json!({"type": "object"}),
            )]),
            reasoning_effort: None,
            stream_options: None,
        };
        let payload = build_responses_payload(&req);
        assert_eq!(payload["instructions"], "sys prompt");
        assert_eq!(payload["reasoning"]["effort"], "medium");
        let input = payload["input"].as_array().unwrap();
        assert_eq!(input.len(), 4);
        assert_eq!(input[2]["type"], "function_call");
        assert_eq!(input[2]["call_id"], "call_1");
        assert_eq!(input[3]["type"], "function_call_output");
        assert_eq!(input[3]["output"], "fn main() {}");
        assert_eq!(payload["tools"][0]["name"], "read_file");
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheControl {
    #[serde(rename = "type")]
    pub cache_type: String,
}

impl CacheControl {
    pub fn ephemeral() -> Self {
        Self {
            cache_type: "ephemeral".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrlObject },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImageUrlObject {
    pub url: String,
}

impl MessageContent {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            MessageContent::Text(s) => Some(s.as_str()),
            MessageContent::Parts(parts) => parts.iter().find_map(|p| match p {
                ContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            }),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            MessageContent::Text(s) => s.len(),
            MessageContent::Parts(parts) => parts
                .iter()
                .map(|p| match p {
                    ContentPart::Text { text } => text.len(),
                    ContentPart::ImageUrl { image_url } => image_url.url.len(),
                })
                .sum(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl std::ops::Deref for MessageContent {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            MessageContent::Text(s) => s.as_str(),
            MessageContent::Parts(parts) => parts
                .iter()
                .find_map(|p| match p {
                    ContentPart::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .unwrap_or(""),
        }
    }
}

impl From<String> for MessageContent {
    fn from(s: String) -> Self {
        MessageContent::Text(s)
    }
}

impl From<&str> for MessageContent {
    fn from(s: &str) -> Self {
        MessageContent::Text(s.to_string())
    }
}

impl From<Vec<ContentPart>> for MessageContent {
    fn from(parts: Vec<ContentPart>) -> Self {
        MessageContent::Parts(parts)
    }
}

impl PartialEq<str> for MessageContent {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == Some(other)
    }
}

impl PartialEq<&str> for MessageContent {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == Some(*other)
    }
}

impl PartialEq<String> for MessageContent {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == Some(other.as_str())
    }
}

impl PartialEq<&String> for MessageContent {
    fn eq(&self, other: &&String) -> bool {
        self.as_str() == Some(other.as_str())
    }
}

impl PartialEq<MessageContent> for str {
    fn eq(&self, other: &MessageContent) -> bool {
        other.as_str() == Some(self)
    }
}

impl PartialEq<MessageContent> for &str {
    fn eq(&self, other: &MessageContent) -> bool {
        other.as_str() == Some(*self)
    }
}

impl PartialEq<MessageContent> for String {
    fn eq(&self, other: &MessageContent) -> bool {
        other.as_str() == Some(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: Role,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<MessageContent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "tool_call_id")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "tool_calls")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "cache_control")]
    pub cache_control: Option<CacheControl>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: Some(MessageContent::Text(content.into())),
            name: None,
            tool_call_id: None,
            tool_calls: None,
            cache_control: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: Some(MessageContent::Text(content.into())),
            name: None,
            tool_call_id: None,
            tool_calls: None,
            cache_control: None,
        }
    }

    pub fn user_with_images(text: impl Into<String>, images: Vec<String>) -> Self {
        let text_str = text.into();
        if images.is_empty() {
            return Self::user(text_str);
        }
        let mut parts = Vec::with_capacity(images.len() + 1);
        if !text_str.is_empty() {
            parts.push(ContentPart::Text { text: text_str });
        }
        for img in images {
            parts.push(ContentPart::ImageUrl {
                image_url: ImageUrlObject { url: img },
            });
        }
        Self {
            role: Role::User,
            content: Some(MessageContent::Parts(parts)),
            name: None,
            tool_call_id: None,
            tool_calls: None,
            cache_control: None,
        }
    }

    pub fn assistant(content: Option<String>, tool_calls: Option<Vec<ToolCall>>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.map(MessageContent::Text),
            name: None,
            tool_call_id: None,
            tool_calls,
            cache_control: None,
        }
    }

    pub fn tool_response(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: Some(MessageContent::Text(content.into())),
            name: None,
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: None,
            cache_control: None,
        }
    }

    pub fn text_content(&self) -> Option<&str> {
        self.content.as_ref().and_then(|c| c.as_str())
    }

    pub fn has_images(&self) -> bool {
        match self.content.as_ref() {
            Some(MessageContent::Parts(parts)) => {
                parts.iter().any(|p| matches!(p, ContentPart::ImageUrl { .. }))
            }
            _ => false,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        self.text_content()
    }

    pub fn with_cache_control(mut self, cc: CacheControl) -> Self {
        self.cache_control = Some(cc);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
}

/// Official alias for OpenAI-compatible tool definition
pub type OpenAITool = ToolDefinition;

impl ToolDefinition {
    pub fn function(name: &str, description: &str, parameters: serde_json::Value) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: name.to_string(),
                description: description.to_string(),
                parameters,
            },
        }
    }

    pub fn name(&self) -> &str {
        &self.function.name
    }
}

/// Token & cache telemetry statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsageStats {
    #[serde(rename = "inputTokens", default)]
    pub input_tokens: u64,
    #[serde(rename = "outputTokens", default)]
    pub output_tokens: u64,
    #[serde(rename = "cacheReadInputTokens", default)]
    pub cache_read_input_tokens: u64,
    #[serde(rename = "cacheCreationInputTokens", default)]
    pub cache_creation_input_tokens: u64,
    #[serde(rename = "cacheHitRate", default)]
    pub cache_hit_rate: f64,
}

impl TokenUsageStats {
    pub fn compute_hit_rate(&mut self) {
        let total = self.input_tokens + self.cache_read_input_tokens;
        if total > 0 {
            self.cache_hit_rate = (self.cache_read_input_tokens as f64) / (total as f64);
        } else {
            self.cache_hit_rate = 0.0;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum StreamChunk {
    FirstChunk,
    Token(String),
    Thought(String),
    Output(String),
    ToolCall {
        id: String,
        name: String,
        arguments: String,
    },
    Usage {
        input_tokens: u64,
        output_tokens: u64,
        cache_read_tokens: u64,
        cache_creation_tokens: u64,
    },
    Done,
    Error(String),
}

// ============================================================================
// OpenAI-Compatible Schemas
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamOptions {
    pub include_usage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "reasoning_effort")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "stream_options")]
    pub stream_options: Option<StreamOptions>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIFunctionDelta {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIToolCallDelta {
    pub index: Option<usize>,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub call_type: Option<String>,
    pub function: Option<OpenAIFunctionDelta>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIDelta {
    pub role: Option<String>,
    pub content: Option<String>,
    #[serde(default)]
    pub reasoning_content: Option<String>,
    #[serde(default)]
    pub thought: Option<String>,
    pub tool_calls: Option<Vec<OpenAIToolCallDelta>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIChoiceDelta {
    pub delta: OpenAIDelta,
    pub finish_reason: Option<String>,
    pub index: Option<usize>,
    #[serde(default)]
    pub reasoning_content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIChatChunk {
    pub id: Option<String>,
    pub choices: Option<Vec<OpenAIChoiceDelta>>,
    pub usage: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_message_serialization() {
        let msg = ChatMessage::user("Hello world");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""content":"Hello world""#));
        let deserialized: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.as_str(), Some("Hello world"));
    }

    #[test]
    fn test_multimodal_message_serialization() {
        let msg = ChatMessage::user_with_images(
            "Look at this capybara",
            vec!["data:image/png;base64,iVBORw0KGgoAAAANSUhEUg==".to_string()],
        );
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#"{"type":"text","text":"Look at this capybara"}"#));
        assert!(json.contains(r#"{"type":"image_url","image_url":{"url":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUg=="}}"#));
        let deserialized: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.as_str(), Some("Look at this capybara"));
    }
}

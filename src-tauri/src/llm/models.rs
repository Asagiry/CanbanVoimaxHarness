use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "tool_call_id")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "tool_calls")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: Some(content.into()),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: Some(content.into()),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn assistant(content: Option<String>, tool_calls: Option<Vec<ToolCall>>) -> Self {
        Self {
            role: Role::Assistant,
            content,
            name: None,
            tool_call_id: None,
            tool_calls,
        }
    }

    pub fn tool_response(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: Some(content.into()),
            name: None,
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: None,
        }
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum StreamChunk {
    Token(String),
    Thought(String),
    ToolCall {
        id: String,
        name: String,
        arguments: String,
    },
    Done,
    Error(String),
}

// ============================================================================
// OpenAI-Compatible Schemas
// ============================================================================

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
    pub reasoning_content: Option<String>,
    pub thought: Option<String>,
    pub tool_calls: Option<Vec<OpenAIToolCallDelta>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIChoiceDelta {
    pub delta: OpenAIDelta,
    pub finish_reason: Option<String>,
    pub index: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIChatChunk {
    pub id: Option<String>,
    pub choices: Option<Vec<OpenAIChoiceDelta>>,
}

// ============================================================================
// Anthropic Native Schemas
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessageRequest {
    pub model: String,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    pub messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<AnthropicTool>>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicStreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: serde_json::Value },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        content_block: serde_json::Value,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        index: usize,
        delta: serde_json::Value,
    },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: serde_json::Value },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(other)]
    Unknown,
}

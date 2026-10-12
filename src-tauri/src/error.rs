use serde::{Serialize, Serializer};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("PermissionDenied: Инструмент '{tool}' отключен политикой безопасности для колонки '{stage}'")]
    PermissionDenied { tool: String, stage: String },

    #[error("ToolExecutionError: {0}")]
    ToolExecution(String),

    #[error("ConfigError: {0}")]
    Config(String),

    #[error("LlmError: {0}")]
    Llm(String),

    #[error("IoError: {0}")]
    Io(#[from] std::io::Error),

    #[error("JsonError: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

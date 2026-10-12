use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(rename = "type")]
    pub provider_type: String,
    #[serde(rename = "baseUrl")]
    pub base_url: String,
    #[serde(rename = "defaultModel", default)]
    pub default_model: Option<String>,
    #[serde(rename = "apiKey", default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "activePipeline", default)]
    pub active_pipeline: Option<String>,
    #[serde(rename = "defaultModel", default)]
    pub default_model: Option<String>,
    #[serde(rename = "apiBaseUrl", default)]
    pub api_base_url: Option<String>,
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineColumn {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub persona: Option<String>,
    #[serde(rename = "contextWindow", default)]
    pub context_window: Option<u64>,
    #[serde(rename = "modelOverride", default)]
    pub model_override: Option<String>,
    #[serde(rename = "reasoningEffort", default)]
    pub reasoning_effort: Option<String>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(rename = "gatekeeperSensor", default)]
    pub gatekeeper_sensor: Option<String>,
    #[serde(default)]
    pub hooks: HashMap<String, String>,
    #[serde(default)]
    pub routes: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub id: String,
    pub title: String,
    pub columns: Vec<PipelineColumn>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEntry {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(rename = "lastOpened")]
    pub last_opened: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectRegistry {
    #[serde(default)]
    pub projects: Vec<ProjectEntry>,
    #[serde(rename = "activeProjectId", default)]
    pub active_project_id: Option<String>,
}

pub async fn atomic_write_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create parent directory for {:?}", path))?;
    }

    let tmp_path = path.with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));
    tokio::fs::write(&tmp_path, content)
        .await
        .with_context(|| format!("Failed to write temporary file {:?}", tmp_path))?;

    tokio::fs::rename(&tmp_path, path)
        .await
        .with_context(|| format!("Failed to replace destination file {:?} with temporary file {:?}", path, tmp_path))?;

    Ok(())
}

pub async fn load_project_config(root_path: impl AsRef<Path>) -> Result<ProjectConfig> {
    let config_path = root_path.as_ref().join(".cvh").join("config.json");
    let content = tokio::fs::read_to_string(&config_path)
        .await
        .with_context(|| format!("Failed to read project config at {:?}", config_path))?;

    let config: ProjectConfig = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse project config JSON at {:?}", config_path))?;

    Ok(config)
}

pub async fn save_project_config(root_path: impl AsRef<Path>, config: &ProjectConfig) -> Result<()> {
    let config_path = root_path.as_ref().join(".cvh").join("config.json");
    let serialized = serde_json::to_string_pretty(config)
        .context("Failed to serialize ProjectConfig to JSON")?;
    atomic_write_file(&config_path, &serialized).await
}

pub async fn load_pipeline_config(root_path: impl AsRef<Path>) -> Result<PipelineConfig> {
    let pipeline_path = root_path.as_ref().join(".cvh").join("pipeline.json");
    let content = tokio::fs::read_to_string(&pipeline_path)
        .await
        .with_context(|| format!("Failed to read pipeline config at {:?}", pipeline_path))?;

    let pipeline: PipelineConfig = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse pipeline config JSON at {:?}", pipeline_path))?;

    Ok(pipeline)
}

pub async fn save_pipeline_config(root_path: impl AsRef<Path>, config: &PipelineConfig) -> Result<()> {
    let pipeline_path = root_path.as_ref().join(".cvh").join("pipeline.json");
    let serialized = serde_json::to_string_pretty(config)
        .context("Failed to serialize PipelineConfig to JSON")?;
    atomic_write_file(&pipeline_path, &serialized).await
}

pub fn get_default_registry_path() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".cvh").join("registry.json")
}

pub async fn load_project_registry(registry_path: Option<PathBuf>) -> Result<ProjectRegistry> {
    let path = registry_path.unwrap_or_else(get_default_registry_path);
    if !path.exists() {
        return Ok(ProjectRegistry::default());
    }

    let content = tokio::fs::read_to_string(&path)
        .await
        .with_context(|| format!("Failed to read project registry at {:?}", path))?;

    let registry: ProjectRegistry = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse project registry at {:?}", path))?;

    Ok(registry)
}

pub async fn save_project_registry(registry_path: Option<PathBuf>, registry: &ProjectRegistry) -> Result<()> {
    let path = registry_path.unwrap_or_else(get_default_registry_path);
    let serialized = serde_json::to_string_pretty(registry)
        .context("Failed to serialize ProjectRegistry to JSON")?;
    atomic_write_file(&path, &serialized).await
}

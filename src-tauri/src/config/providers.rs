//! Provider catalog for LLM settings: named providers with their own API key,
//! base URL, protocol and model catalog. Legacy flat `LlmSettings` fields stay
//! as a deprecated fallback and are migrated into a single `Provider` on load.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProviderProtocol {
    #[default]
    Openai,
    Anthropic,
    Gemini,
    Ollama,
    Custom,
}

impl ProviderProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderProtocol::Openai => "openai",
            ProviderProtocol::Anthropic => "anthropic",
            ProviderProtocol::Gemini => "gemini",
            ProviderProtocol::Ollama => "ollama",
            ProviderProtocol::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderModel {
    pub id: String,
    #[serde(rename = "displayName", alias = "display_name", default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    #[serde(rename = "isCustom", alias = "is_custom", default)]
    pub is_custom: bool,
    #[serde(rename = "apiKey", alias = "api_key", default)]
    pub api_key: String,
    #[serde(rename = "baseUrl", alias = "base_url", default)]
    pub base_url: String,
    #[serde(default)]
    pub protocol: ProviderProtocol,
    #[serde(default)]
    pub models: Vec<ProviderModel>,
}

impl Provider {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            is_custom: false,
            api_key: String::new(),
            base_url: String::new(),
            protocol: ProviderProtocol::default(),
            models: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("Provider id cannot be empty".to_string());
        }
        if self.name.trim().is_empty() {
            return Err("Provider name cannot be empty".to_string());
        }
        let base = self.base_url.trim();
        if !base.is_empty() && reqwest::Url::parse(base).is_err() {
            return Err(format!("Invalid base URL: {}", base));
        }
        for model in &self.models {
            if model.id.trim().is_empty() {
                return Err("Model id cannot be empty".to_string());
            }
        }
        Ok(())
    }
}

pub fn normalize_providers(providers: Vec<Provider>) -> Vec<Provider> {
    let mut result: Vec<Provider> = Vec::new();
    for mut provider in providers {
        // Frontend ids are lowercased (toProviderId); normalize here too so
        // activeProviderId lookups never miss on case and dispatch never
        // silently falls back to providers[0].
        provider.id = provider.id.trim().to_lowercase();
        provider.name = provider.name.trim().to_string();
        provider.base_url = provider.base_url.trim().to_string();
        if provider.id.is_empty() || provider.name.is_empty() {
            continue;
        }
        let mut seen_models = std::collections::HashSet::new();
        provider.models.retain(|m| {
            let id = m.id.trim().to_string();
            if id.is_empty() {
                return false;
            }
            seen_models.insert(id)
        });
        for model in &mut provider.models {
            model.id = model.id.trim().to_string();
            if let Some(display) = &model.display_name {
                model.display_name = Some(display.trim().to_string());
            }
        }
        if let Some(existing) = result.iter_mut().find(|p| p.id == provider.id) {
            for model in provider.models {
                if !existing.models.iter().any(|m| m.id == model.id) {
                    existing.models.push(model);
                }
            }
            continue;
        }
        result.push(provider);
    }
    result
}

pub fn legacy_provider(provider: &str, api_key: &str, base_url: &str, model: &str) -> Provider {
    let name = provider.trim();
    let name = if name.is_empty() { "Default" } else { name };
    let mut result = Provider::new(name.to_lowercase(), name);
    result.api_key = api_key.to_string();
    result.base_url = base_url.to_string();
    result.is_custom = !matches!(
        name,
        "Antigravity Proxy" | "OpenCode Go" | "OpenAI" | "Anthropic" | "Ollama" | "DeepSeek"
    );
    let model = model.trim();
    if !model.is_empty() {
        result.models.push(ProviderModel {
            id: model.to_string(),
            display_name: None,
        });
    }
    result
}

pub fn upsert_provider(providers: &mut Vec<Provider>, provider: Provider) {
    if let Some(existing) = providers.iter_mut().find(|p| p.id == provider.id) {
        *existing = provider;
        return;
    }
    providers.push(provider);
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn protocol_default_is_openai() {
        assert_eq!(ProviderProtocol::default(), ProviderProtocol::Openai);
        assert_eq!(ProviderProtocol::Anthropic.as_str(), "anthropic");
    }

    #[test]
    fn provider_serde_camel_case_roundtrip() {
        let json = r#"{
            "id": "tg",
            "name": "tg",
            "isCustom": true,
            "apiKey": "sk-1",
            "baseUrl": "http://127.0.0.1:8045/v1",
            "protocol": "openai",
            "models": [{"id": "gemini-3.8-flash-high", "displayName": "Gemini 3.8 Flash"}]
        }"#;
        let provider: Provider = serde_json::from_str(json).expect("provider parses");
        assert_eq!(provider.id, "tg");
        assert!(provider.is_custom);
        assert_eq!(provider.api_key, "sk-1");
        assert_eq!(provider.models[0].display_name.as_deref(), Some("Gemini 3.8 Flash"));
        let serialized = serde_json::to_string(&provider).expect("provider serializes");
        assert!(serialized.contains("\"isCustom\":true"));
        assert!(serialized.contains("\"displayName\":\"Gemini 3.8 Flash\""));
    }

    #[test]
    fn provider_validation_at_boundary() {
        let mut provider = Provider::new("p1", "P1");
        assert!(provider.validate().is_ok());

        provider.id = "  ".to_string();
        assert!(provider.validate().is_err());

        provider.id = "p1".to_string();
        provider.base_url = "not a url".to_string();
        assert!(provider.validate().is_err());

        provider.base_url = "http://127.0.0.1:8045/v1".to_string();
        provider.models.push(ProviderModel { id: " ".to_string(), display_name: None });
        assert!(provider.validate().is_err());
    }

    #[test]
    fn normalize_dedups_providers_and_models() {
        let mut dup = Provider::new("tg", "tg");
        dup.models.push(ProviderModel { id: "m1".to_string(), display_name: None });
        dup.models.push(ProviderModel { id: "m1".to_string(), display_name: None });
        let normalized = normalize_providers(vec![Provider::new("tg", "tg"), dup]);
        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0].models.len(), 1);
    }

    #[test]
    fn legacy_provider_migration_keeps_model_and_key() {
        let provider = legacy_provider("Antigravity Proxy", "key", "http://127.0.0.1:8045/v1", "gemini-3.8-flash-high");
        assert_eq!(provider.id, "antigravity proxy");
        assert!(!provider.is_custom);
        assert_eq!(provider.api_key, "key");
        assert_eq!(provider.models.len(), 1);
        assert_eq!(provider.models[0].id, "gemini-3.8-flash-high");

        let custom = legacy_provider("tg", "", "http://x/v1", "");
        assert!(custom.is_custom);
        assert!(custom.models.is_empty());
    }

    #[test]
    fn upsert_replaces_by_id() {
        let mut providers = vec![Provider::new("a", "A")];
        upsert_provider(&mut providers, Provider::new("b", "B"));
        upsert_provider(&mut providers, Provider::new("a", "A2"));
        assert_eq!(providers.len(), 2);
        assert_eq!(providers[0].name, "A2");
    }

    #[test]
    fn normalize_lowercases_ids_so_active_lookup_matches() {
        let mut provider = Provider::new(" OpenCode Go ", "OpenCode Go");
        provider.base_url = " https://opencode.ai/inference/go/openai/v1 ".to_string();
        let normalized = normalize_providers(vec![provider]);
        assert_eq!(normalized[0].id, "opencode go");
        assert_eq!(normalized[0].base_url, "https://opencode.ai/inference/go/openai/v1");
    }
}

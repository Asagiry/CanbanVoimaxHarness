use crate::commands::state::AppState;
use crate::config::{load_llm_settings, save_llm_settings_to_disk, LlmSettings};
use tauri::State;

#[tauri::command]
pub async fn get_llm_settings(state: State<'_, AppState>) -> Result<LlmSettings, String> {
    let root = state.get_root();
    load_llm_settings(&root)
        .await
        .map_err(|e| format!("Failed to load LLM settings: {}", e))
}

#[tauri::command]
pub async fn save_llm_settings(
    settings: LlmSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let root = state.get_root();
    save_llm_settings_to_disk(&root, &settings)
        .await
        .map_err(|e| format!("Failed to save LLM settings: {}", e))
}

#[tauri::command]
pub async fn fetch_available_models(
    base_url: String,
    api_key: Option<String>,
) -> Result<Vec<String>, String> {
    let trimmed_base = base_url.trim().trim_end_matches('/');
    if trimmed_base.is_empty() {
        return Err("base_url cannot be empty".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Prepare candidate endpoints in priority order
    let mut candidates = Vec::new();
    if trimmed_base.ends_with("/models") || trimmed_base.ends_with("/api/tags") {
        candidates.push(trimmed_base.to_string());
    } else if trimmed_base.ends_with("/v1") {
        candidates.push(format!("{}/models", trimmed_base));
        let root = trimmed_base.trim_end_matches("/v1");
        candidates.push(format!("{}/models", root));
        candidates.push(format!("{}/api/tags", root));
    } else {
        candidates.push(format!("{}/models", trimmed_base));
        candidates.push(format!("{}/v1/models", trimmed_base));
        candidates.push(format!("{}/api/tags", trimmed_base));
    }

    let mut last_error = String::new();

    for url in candidates {
        let mut req = client.get(&url);
        if let Some(ref key) = api_key {
            let key_trimmed = key.trim();
            if !key_trimmed.is_empty() {
                req = req.header(
                    reqwest::header::AUTHORIZATION,
                    format!("Bearer {}", key_trimmed),
                );
            }
        }

        match req.send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(json) => {
                            let mut models = Vec::new();

                            // OpenAI format: {"data": [{"id": "model-id"}, ...]}
                            if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                                for item in data {
                                    if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
                                        models.push(id.to_string());
                                    }
                                }
                            }
                            // Ollama format: {"models": [{"name": "model-id"}, ...]}
                            else if let Some(models_arr) = json.get("models").and_then(|m| m.as_array()) {
                                for item in models_arr {
                                    if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                                        models.push(name.to_string());
                                    } else if let Some(model) = item.get("model").and_then(|m| m.as_str()) {
                                        models.push(model.to_string());
                                    }
                                }
                            }
                            // Array format fallback: [{"id": "..."}, ...] or ["model-id", ...]
                            else if let Some(arr) = json.as_array() {
                                for item in arr {
                                    if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
                                        models.push(id.to_string());
                                    } else if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                                        models.push(name.to_string());
                                    } else if let Some(s) = item.as_str() {
                                        models.push(s.to_string());
                                    }
                                }
                            }

                            if !models.is_empty() {
                                models.sort();
                                models.dedup();
                                return Ok(models);
                            } else {
                                last_error = format!("Endpoint {} returned no recognized models in JSON payload", url);
                            }
                        }
                        Err(e) => {
                            last_error = format!("Failed to parse JSON from {}: {}", url, e);
                        }
                    }
                } else {
                    last_error = format!("Endpoint {} returned HTTP status {}", url, resp.status());
                }
            }
            Err(e) => {
                last_error = format!("Failed to connect to {}: {}", url, e);
            }
        }
    }

    Err(format!("Could not fetch models from {}: {}", base_url, last_error))
}

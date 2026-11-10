//! Token and cache usage extractors for OpenAI-compatible and provider-specific responses.

use std::collections::HashMap;

/// Robust usage extractors: different OpenAI-compatible proxies report cache
/// telemetry under different keys (OpenAI `prompt_tokens_details.cached_tokens`,
/// Responses-API `input_tokens_details.cached_tokens`, top-level `cached_tokens`,
/// Gemini-native `cached_content_token_count`, Anthropic-style
/// `cache_read_input_tokens`). Try each in order; missing keys yield 0.
/// Pure functions so prefix/telemetry behaviour is unit-testable.
pub fn extract_openai_input_tokens(usage: &serde_json::Value) -> u64 {
    if let Some(total) = usage.get("prompt_tokens").and_then(|v| v.as_u64()) {
        return total;
    }
    let input = usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
    if usage.get("input_tokens_details").is_some() {
        return input;
    }
    if usage.get("cache_read_input_tokens").is_some() || usage.get("cache_creation_input_tokens").is_some() {
        return input.saturating_add(extract_openai_cache_read(usage))
            .saturating_add(extract_openai_cache_creation(usage));
    }
    if usage.get("input_tokens").is_none() {
        return usage.get("prompt_cache_miss_tokens").and_then(|v| v.as_u64())
            .map(|miss| miss.saturating_add(extract_openai_cache_read(usage))).unwrap_or(0);
    }
    input
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

pub fn resolve_tool_call_index(
    map: &HashMap<usize, (String, String, String)>,
    explicit: Option<usize>,
    id: Option<&str>,
    next_fallback: &mut usize,
) -> usize {
    if let Some(idx) = explicit {
        return idx;
    }
    if let Some(id) = id.filter(|s| !s.is_empty()) {
        if let Some((&idx, _)) = map.iter().find(|(_, (existing, _, _))| existing == id) {
            return idx;
        }
    }
    let idx = *next_fallback;
    *next_fallback = next_fallback.saturating_add(1);
    eprintln!("CVH [llm_client] tool call missing index; assigned fallback idx={idx}");
    idx
}

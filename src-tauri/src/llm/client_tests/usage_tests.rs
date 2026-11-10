use crate::llm::usage::*;

#[test]
fn cache_usage_normalizes_provider_accounting() {
    for usage in [
        serde_json::json!({"prompt_tokens": 100000, "prompt_cache_hit_tokens": 99000, "prompt_cache_miss_tokens": 1000}),
        serde_json::json!({"prompt_cache_hit_tokens": 99000, "prompt_cache_miss_tokens": 1000}),
        serde_json::json!({"input_tokens": 700, "cache_read_input_tokens": 99000, "cache_creation_input_tokens": 300}),
        serde_json::json!({"input_tokens": 100000, "input_tokens_details": {"cached_tokens": 99000}}),
    ] {
        let total = extract_openai_input_tokens(&usage);
        let cached = extract_openai_cache_read(&usage);
        assert_eq!(total, 100000);
        assert_eq!(cached, 99000);
        assert_eq!(crate::agent::compute_cache_hit_rate(cached, total), 99.0);
    }
}

#[test]
fn test_extract_openai_cache_read_openai_shape() {
    let usage = serde_json::json!({
        "prompt_tokens": 15000,
        "completion_tokens": 100,
        "total_tokens": 15100,
        "prompt_tokens_details": { "cached_tokens": 12000, "audio_tokens": 0 }
    });
    assert_eq!(extract_openai_input_tokens(&usage), 15000);
    assert_eq!(extract_openai_output_tokens(&usage), 100);
    assert_eq!(extract_openai_cache_read(&usage), 12000);
}

#[test]
fn test_extract_openai_cache_read_fallback_shapes() {
    // Responses-API shape.
    let responses = serde_json::json!({
        "input_tokens": 15000,
        "output_tokens": 100,
        "input_tokens_details": { "cached_tokens": 9000 }
    });
    assert_eq!(extract_openai_input_tokens(&responses), 15000);
    assert_eq!(extract_openai_output_tokens(&responses), 100);
    assert_eq!(extract_openai_cache_read(&responses), 9000);

    // Gemini-native passthrough shape.
    let gemini = serde_json::json!({
        "prompt_tokens": 15000,
        "completion_tokens": 100,
        "cached_content_token_count": 7000
    });
    assert_eq!(extract_openai_cache_read(&gemini), 7000);

    // Proxy that drops all cache counters: must parse as 0, not fail.
    let bare = serde_json::json!({
        "prompt_tokens": 15000,
        "completion_tokens": 100,
        "total_tokens": 15100
    });
    assert_eq!(extract_openai_cache_read(&bare), 0);
    assert_eq!(extract_openai_cache_creation(&bare), 0);
}

#[test]
fn test_token_usage_stats_cache_hit_rate_safe_division_by_zero() {
    let mut stats = crate::llm::TokenUsageStats::default();
    stats.compute_hit_rate();
    assert_eq!(stats.cache_hit_rate, 0.0);

    stats.input_tokens = 1000;
    stats.cache_read_input_tokens = 500;
    stats.compute_hit_rate();
    assert!((stats.cache_hit_rate - 0.5).abs() < 1e-6);
}

#[test]
fn test_stream_options_serialization() {
    let req = crate::llm::OpenAIChatRequest {
        model: "test-model".to_string(),
        messages: vec![],
        temperature: None,
        stream: true,
        tools: None,
        reasoning_effort: None,
        stream_options: Some(crate::llm::StreamOptions { include_usage: true }),
    };
    let serialized = serde_json::to_string(&req).unwrap();
    assert!(serialized.contains(r#""stream_options":{"include_usage":true}"#));
}

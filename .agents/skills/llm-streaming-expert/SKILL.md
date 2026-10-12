---
name: llm-streaming-expert
description: Specialized LLM API integration and SSE streaming skill for CanbanVoimaxHarness. Covers OpenAI-compatible endpoints (Local Antigravity proxy, DeepSeek, Ollama), Anthropic Claude native streaming, tool schemas, and ReAct loops.
---

# LLM Streaming & ReAct Engine Skill

### Directives:
1. **Universal Provider Architecture**:
   - OpenAI-compatible format: `POST /chat/completions` with `stream: true`. Parse `data: {"choices": [{"delta": ...}]}` chunks.
   - Anthropic format: `POST /messages` with `stream: true`. Parse `content_block_delta`, `thinking_delta`, and `tool_use`.
2. **Zero-Turn Memory Priming**:
   - Construct Turn 0 system context: `SystemMessage = Persona + Project Guidelines + Active Skills + Task Description`.
   - Never instruct agents to read files in early turns; prime memory immediately.
3. **ReAct Tool Execution Loop**:
   - Handle `tool_calls` emitted by LLM.
   - Execute tool in Rust (`read_file`, `write_file`, `edit_file`, `execute_command`, `spawn_subagent`).
   - Append tool results with role `tool` or `user` and continue turn until final text response.
   - Stream individual tokens directly to UI via Tauri event: `app_handle.emit("agent:token", payload)`.
4. **HTTP Header Construction Invariants**:
   - In `reqwest::header::HeaderMap`, header keys must implement `IntoHeaderName`. Use `HeaderName::from_static("anthropic-version")` and `HeaderName::from_static("x-api-key")` or string literals. Never pass `HeaderValue` as the header key.
   - For Anthropic native streaming, include `anthropic-version: 2023-06-01` and `anthropic-beta: prompt-caching-2024-07-31` when ephemeral prompt caching is enabled.

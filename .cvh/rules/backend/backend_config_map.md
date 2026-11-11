---
description: Config, provider APIs, model catalogs, prompts, and cache routing.
---
# Configuration and LLM providers

Tool policy is enforced in three layers (all alias-aware via `normalize_tool_name`): (1) `ToolPolicyConfig::is_tool_allowed` (`tool_policy.rs`) — `stage_overrides[stage][tool]`, default `true` when the stage or tool is absent; `ToolPolicyConfig` has no `global_enabled` field. (2) `ToolPermissions::is_tool_allowed` (`models.rs`) — legacy allowlist with AND-semantics `global && stage`. (3) `agent_allows_tool` (`models.rs`) — agent-level `allowed_tools` allowlist intersection (`None` = no restriction). Disabled tools are excluded from schemas. Config I/O owns atomic writes. Provider adapters own request serialization and cache/session headers.

| File | Responsibility |
| --- | --- |
| `src-tauri/src/config/mod.rs` | Submodule re-exports (`models`, `providers`, `tool_policy`, `discovery`, `scaffold`, `io`, `pipeline`, `projects`, `agent_settings`, `prompts`). |
| `src-tauri/src/config/models.rs` | `BoardDef`, `PipelineConfig`, embedded rules; `LlmSettings` carries `providers` (`#[serde(default)]`, `ensure_providers()` on load/save); `ToolPermissions::is_tool_allowed` and `agent_allows_tool`. |
| `src-tauri/src/config/agent_settings.rs` | Agent setting serializations, persona configuration models, and context window limits. |
| `src-tauri/src/config/pipeline.rs` | Pipeline stage defaults, normalization, and stage template builders (`get_canonical_board_stages`, `get_board_template_stages`). |
| `src-tauri/src/config/projects.rs` | Project and workspace configuration definitions and path normalization. |
| `src-tauri/src/config/prompts/mod.rs` | Prompts module re-exports. |
| `src-tauri/src/config/prompts/roles.rs` | Canonical role persona prompts and system prompt templates. |
| `src-tauri/src/config/prompts/rules.rs` | Static rule templates and system prompt rule compilation. |
| `src-tauri/src/config/providers.rs` | LLM provider catalog under `config/`: `Provider` (`id`, `name`, `is_custom`, `api_key`, `base_url`, `protocol`, `models`); `Provider::validate()` boundary guard; `normalize_providers`; `legacy_provider`. <=500 lines. |
| `src-tauri/src/config/tool_policy.rs` | `ToolPolicyConfig` (`stage_overrides`, `tool_settings`, `tool_descriptions`, `tool_custom_schemas` — no `global_enabled`) and free `is_tool_allowed(policy, stage_id, tool)`: stage override lookup; subagent master-switch hierarchy. |
| `src-tauri/src/config/discovery.rs` | Multi-tier root discovery, `%LOCALAPPDATA%`. |
| `src-tauri/src/config/scaffold.rs` | Automatic scaffold, auto-gitignore policies. |
| `src-tauri/src/config/io.rs` | `atomic_write_file`, atomic JSON load/save. |
| `src-tauri/src/commands/cortex.rs` | Persona/rules file operations, YAML frontmatter. |
| `src-tauri/src/commands/llm.rs` | Provider settings, model catalogs and connection checks; provider CRUD (`list_providers`, `upsert_provider_command`, `delete_provider`) persisted via `save_llm_settings_to_disk`. |
| `src-tauri/src/commands/tools.rs` | Tool policy inspection, stage overrides & matrix updates. |
| `src-tauri/src/llm/mod.rs` | LLM client and models module re-exports. |
| `src-tauri/src/llm/client.rs` | Universal SSE streaming client, retry backoffs, and stream connection management. |
| `src-tauri/src/llm/usage.rs` | Normalized token usage accumulation, prompt caching calculations, and stream chunk mapping. |
| `src-tauri/src/llm/responses_event.rs` | SSE event payload parsing and responses API streaming adapter. |
| `src-tauri/src/llm/cache.rs` | Stable OpenCode session headers and submitted-prefix diagnostics. |
| `src-tauri/src/llm/models.rs` | Model overrides, endpoints, temperature, and request schemas. |

## Related checks

Choose checks for the changed behavior; use the active role's `sensors` rule for execution policy.

- `src-tauri/src/config/tests.rs` — Config tests.
- `src-tauri/src/commands/tests/cortex_tests.rs` — Cortex tests.
- `src-tauri/src/llm/client_tests/mod.rs` — LLM client test module root.
- `src-tauri/src/llm/client_tests/fixtures.rs` — Shared mock/SSE test fixtures for LLM client tests.
- `src-tauri/src/llm/client_tests/stream_tests.rs` — LLM SSE streaming client tests.
- `src-tauri/src/llm/client_tests/usage_tests.rs` — Token usage and cache accounting tests.

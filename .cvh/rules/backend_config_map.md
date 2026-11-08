---
description: Config, provider APIs, model catalogs and cache routing.
---
# Configuration and LLM providers

Tool policy is enforced in three layers (all alias-aware via `normalize_tool_name`): (1) `ToolPolicyConfig::is_tool_allowed` (`tool_policy.rs`) — `stage_overrides[stage][tool]`, default `true` when the stage or tool is absent; `ToolPolicyConfig` has no `global_enabled` field. (2) `ToolPermissions::is_tool_allowed` (`models.rs:64`) — legacy allowlist with AND-semantics `global && stage` (empty global list = no global restriction, missing stage entry = no stage restriction). (3) `agent_allows_tool` (`models.rs:719`) — agent-level `allowed_tools` allowlist intersection (`None` = no restriction). Disabled tools are excluded from schemas. Config I/O owns atomic writes. Provider adapters own request serialization and cache/session headers.

| File | Responsibility |
| --- | --- |
| `src-tauri/src/config/mod.rs` | Submodule re-exports (`models`, `providers`, `tool_policy`, `discovery`, `scaffold`, `io`). |
| `src-tauri/src/config/models.rs` | `BoardDef`, `PipelineConfig`, embedded rules; `LlmSettings` carries `providers` (`#[serde(default)]`, `ensure_providers()` on load/save); `ToolPermissions::is_tool_allowed` (AND `global && stage`) and `agent_allows_tool` (agent allowlist intersection). |
| `src-tauri/src/config/providers.rs` | LLM provider catalog under `config/` (not `llm/`): `Provider` (`id`, `name`, `is_custom`, `api_key`, `base_url`, `protocol: ProviderProtocol`, `models: Vec<ProviderModel{id, display_name?}>`); `Provider::validate()` boundary guard; `normalize_providers` dedups providers/models; `legacy_provider` migrates flat `LlmSettings` into a single provider. ≤500 lines, zero production unwrap. |
| `src-tauri/src/config/tool_policy.rs` | `ToolPolicyConfig` (`stage_overrides`, `tool_settings`, `tool_descriptions`, `tool_custom_schemas` — no `global_enabled`) and free `is_tool_allowed(policy, stage_id, tool)`: stage override lookup, default `true`; subagent master-switch hierarchy. |
| `src-tauri/src/config/discovery.rs` | Multi-tier root discovery, `%LOCALAPPDATA%`. |
| `src-tauri/src/config/scaffold.rs` | Automatic scaffold, auto-gitignore policies. |
| `src-tauri/src/config/io.rs` | `atomic_write_file`, JSON load/save. |
| `src-tauri/src/commands/cortex.rs` | Persona/rules file operations, YAML frontmatter. |
| `src-tauri/src/commands/llm.rs` | Provider settings, model catalogs and connection checks; provider CRUD (`list_providers`, `upsert_provider_command`, `delete_provider`) persisted via `save_llm_settings_to_disk`; `fetch_available_models` signature unchanged. |
| `src-tauri/src/commands/tools.rs` | Tool policy inspection, stage overrides & matrix updates. |
| `src-tauri/src/llm/mod.rs` | LLM client and models module re-exports. |
| `src-tauri/src/llm/client.rs` | Provider requests, streaming parsers and normalized usage. |
| `src-tauri/src/llm/cache.rs` | Stable OpenCode session headers and submitted-prefix diagnostics. |
| `src-tauri/src/llm/models.rs` | Model overrides, endpoints, temperature. |

## Related checks

Choose checks for the changed behavior; use the active role's `sensors` rule for execution policy.

- `src-tauri/src/config/tests.rs` — tests.
- `src-tauri/src/commands/tests/cortex_tests.rs` — cortex tests.

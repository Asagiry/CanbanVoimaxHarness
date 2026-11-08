---
description: Config, provider APIs, model catalogs and cache routing.
---
# Configuration and LLM providers

Tool policy resolves stage override, then global setting, then enabled by default; disabled tools are excluded from schemas. Config I/O owns atomic writes. Provider adapters own request serialization and cache/session headers.

| File | Responsibility |
| --- | --- |
| `src-tauri/src/config/mod.rs` | Submodule re-exports (`models`, `providers`, `tool_policy`, `discovery`, `scaffold`, `io`). |
| `src-tauri/src/config/models.rs` | `BoardDef`, `PipelineConfig`, embedded rules; `LlmSettings` carries `providers` (`#[serde(default)]`, `ensure_providers()` on load/save). |
| `src-tauri/src/config/providers.rs` | LLM provider catalog: `Provider` (`id`, `name`, `is_custom`, `api_key`, `base_url`, `protocol: ProviderProtocol`, `models: Vec<ProviderModel{id, display_name?}>`); `Provider::validate()` boundary guard; `normalize_providers` dedups providers/models; `legacy_provider` migrates flat `LlmSettings` into a single provider. ≤500 lines, zero production unwrap. |
| `src-tauri/src/config/tool_policy.rs` | Single Authoritative Layer: `stage_override ?? global_enabled ?? true`. |
| `src-tauri/src/config/discovery.rs` | Multi-tier root discovery, `%LOCALAPPDATA%`. |
| `src-tauri/src/config/scaffold.rs` | Automatic scaffold, auto-gitignore policies. |
| `src-tauri/src/config/io.rs` | `atomic_write_file`, JSON load/save. |
| `src-tauri/src/commands/cortex.rs` | Persona/rules file operations, YAML frontmatter. |
| `src-tauri/src/commands/llm.rs` | Provider settings, model catalogs and connection checks; provider CRUD (`list_providers`, `upsert_provider_command`, `delete_provider`) persisted via `config/io.rs` (`atomic_write_file`); `fetch_available_models` signature unchanged. |
| `src-tauri/src/commands/tools.rs` | Tool policy inspection, stage overrides & matrix updates. |
| `src-tauri/src/llm/mod.rs` | LLM client and models module re-exports. |
| `src-tauri/src/llm/client.rs` | Provider requests, streaming parsers and normalized usage. |
| `src-tauri/src/llm/cache.rs` | Stable OpenCode session headers and submitted-prefix diagnostics. |
| `src-tauri/src/llm/models.rs` | Model overrides, endpoints, temperature. |

## Related checks

Choose checks for the changed behavior; use the active role's `sensors` rule for execution policy.

- `src-tauri/src/config/tests.rs` — tests.
- `src-tauri/src/commands/tests/cortex_tests.rs` — cortex tests.

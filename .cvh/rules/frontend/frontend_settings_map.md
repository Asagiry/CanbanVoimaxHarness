---
description: Provider settings, tool policies and Cortex editors.
---
# Settings and Cortex

Cortex edits personas, rules and stage settings (also indexed in `frontend_cortex_map`). Settings stores manage provider/model selection; tool policy UI reflects the backend effective policy. For persistence and model APIs, load `backend_config_map`.

| File | Responsibility |
| --- | --- |
| `src/components/settings/SettingsModal.tsx` | Notification settings + provider list/editor shell (LLM tab delegates to `providers/*`). |
| `src/components/settings/providers/ProviderList.tsx` | Provider cards: name, status dot, «Пользовательский» badge, Edit/Delete, «+ Добавить поставщика». Selection syncs the active provider. ≤350 lines. |
| `src/components/settings/providers/ProviderEditor.tsx` | Detail editor: API key, Advanced settings (display name, base URL, API protocol), model catalog, `fetchProviderModels`, reset defaults, Cancel/Apply validation. |
| `src/components/settings/providers/ProviderModelRow.tsx` | Single catalog row: model id + displayName inputs, drag handle, delete. |
| `src/components/settings/providers/ProviderModelPicker.tsx` | Fetch-models picker modal (search, select-all, «добавлена» badge, add-selected). |
| `src/components/settings/providers/useProviderEditorState.ts` | Hook managing active `editingProvider` state for opening, creating custom, and closing provider editors. |
| `src/components/settings/ToolPolicyModal.tsx` | Interactive policy matrix: per-stage/per-agent overrides, and custom tool management. |
| `src/components/settings/tool-policy/CustomToolForm.tsx` | `CustomToolRows` matrix rows and `CustomToolForm` for adding/deleting `.cvh/tools/*.json` custom tools. |
| `src/components/settings/tool-policy/ToolParamsEditor.tsx` | `PolicyToggle` switch component and numeric parameter stepper rows (`TOOL_PARAM_DEFS`). |
| `src/components/settings/tool-policy/ToolSchemaEditor.tsx` | Interactive JSON function schema override editor with format, save, and reset-to-default controls. |
| `src/components/settings/tool-policy/toolPolicyConstants.ts` | `CustomTool` & `AgentMatrixItem` types, `CATEGORY_ORDER`, `buildAgentMatrix`, and `applyAgentToolToggle`. |
| `src/stores/settings/providers.ts` | Provider types, `migrateFlatToProviders`, `catalogModelOptions`/`catalogModelLabel`, normalization and boundary validation. |
| `src/stores/settings/settingsHelpers.ts` | `LlmSettingsState`, `NotificationSettings`, `DEFAULT_SETTINGS`, normalization and persistence helpers for `settingsStore`. |
| `src/stores/settingsStore.ts` | LLM configuration, endpoints, temperature settings; provider CRUD + `fetchProviderModels`; `availableModels` cache. |
| `src/stores/toolPolicyStore.ts` | Re-export facade preserving tool policy store, types, and default constants. |
| `src/stores/toolPolicy/index.ts` | Barrel module composing `useToolPolicyStore` and metadata helpers. |
| `src/stores/toolPolicy/useToolPolicyStore.ts` | Zustand store and Tauri IPC actions for tool policy state and schemas. |
| `src/stores/toolPolicy/toolPolicyTypes.ts` | Tool policy state contracts, parameter definitions, and override shapes. |
| `src/stores/toolPolicy/toolPolicyDefaults.ts` | Default stage/role permissions and parameter values. |
| `src/stores/toolPolicy/toolSchemaUtils.ts` | Shared default tool schema builders, shell presets, and deep customization equality helpers. |
| `src/stores/toolPolicy/builtinTools.ts` | Built-in tool catalog metadata and category definitions. |
| `src/stores/toolPolicy/agentTools.ts` | Effective per-role tool resolution and matrix helper functions. |
| `src/stores/toolPolicy/toolMeta.ts` | Tool display names, descriptions, and grouping lookups. |
Cortex components and stores (`src/components/cortex/*`, `src/stores/cortexStore.ts`, `src/stores/cortex/*`) are indexed solely in `frontend_cortex_map` — do not duplicate them here.
| `src/components/drawer/AgentConsoleDrawer.tsx` | Chat model selector unified on the provider catalog. |
| `src/components/board/modals/CreateTaskModal.tsx` | Model options from the provider catalog with displayName labels. |

## Related checks

Choose checks for the changed behavior; use the active role's `sensors` rule for execution policy.

- `src/stores/__tests__/settingsCache.test.ts` — Provider presets, explicit model selection, and total-input cache ratios.
- `src/stores/__tests__/providers.test.ts` — Provider migration, normalization, validation, upsert/delete fallback.
- `src/components/settings/providers/__tests__/ProviderList.test.ts` — Provider card rendering helpers.

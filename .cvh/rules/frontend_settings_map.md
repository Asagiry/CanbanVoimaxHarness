---
description: Provider settings, tool policies and Cortex editors.
---
# Settings and Cortex

Cortex edits personas, rules and stage settings. Settings stores manage provider/model selection; tool policy UI reflects the backend effective policy. For persistence and model APIs, load `backend_config_map`.

| File | Responsibility |
| --- | --- |
| `src/components/cortex/CortexExplorer.tsx` | File navigation tree for personas, rules, and hooks. |
| `src/components/cortex/AgentSettingVisualEditor.tsx` | Persona prompt editor, settings, icon/color selectors. |
| `src/components/cortex/AgentMindInspector.tsx` | Real-time agent state, current prompt and tools viewer. |
| `src/components/cortex/ColumnStageCard.tsx` | Pipeline stage configuration card in Cortex view. |
| `src/components/cortex/CreateAgentModal.tsx` | Modal dialog for creating new agent personas. |
| `src/components/cortex/EditAgentPersonaModal.tsx` | Modal dialog for updating agent persona metadata. |
| `src/components/cortex/GlobalRulesSection.tsx` | Global rules list, editor triggers, and toggle controls. |
| `src/components/cortex/NewCortexFileForm.tsx` | Inline form for creating new rules or personas. |
| `src/components/cortex/RuleEditorModal.tsx` | Markdown editor for `.cvh/rules/*.md` and role rules. |
| `src/components/cortex/types.ts` | Cortex explorer interfaces, forms, and tree nodes. |
| `src/components/settings/ToolPolicyModal.tsx` | Interactive policy matrix: global switches & overrides. |
| `src/components/settings/SettingsModal.tsx` | LLM endpoints, active model, and app settings. |
| `src/stores/settingsStore.ts` | LLM configuration, endpoints, temperature settings. |
| `src/stores/toolPolicyStore.ts` | Frontend cache for tool policy permissions. |
| `src/stores/cortex/cortexStoreSlice.ts` | File tree, personas CRUD, and rules state slice. |

## Related checks

Choose checks for the changed behavior; use the active role's `sensors` rule for execution policy.

- `src/components/cortex/__tests__/cortexAntigravityDark.test.ts` — Cortex and shared-component theme regression checks.
- `src/stores/__tests__/settingsCache.test.ts` — Provider presets, explicit model selection, and total-input cache ratios.

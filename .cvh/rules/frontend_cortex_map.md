---
description: Cortex explorer, agent persona editors, stage cards, rule editors and Cortex store slices.
---
# Cortex explorer and persona/rule editors

Cortex inspects and edits agent personas (`.cvh/agents/<role>/persona.md`), role settings (`.cvh/agents/<role>/setting`), modular rules (`.cvh/rules/*.md` and `.cvh/agents/<role>/rules/*.md`), and stage transition hooks (`.cvh/hooks/*`). For provider settings and tool policies, load `frontend_settings_map`.

| File | Responsibility |
| --- | --- |
| `src/components/cortex/CortexExplorer.tsx` | File navigation tree for pipeline stages, personas, rules, and hooks. |
| `src/components/cortex/AgentSettingVisualEditor.tsx` | Persona prompt editor, settings, icon/color selectors, and provider catalog model selector. |
| `src/components/cortex/AgentMindInspector.tsx` | Real-time agent state, compiled system prompt, and active tools viewer. |
| `src/components/cortex/ColumnStageCard.tsx` | Pipeline stage configuration card in Cortex view. |
| `src/components/cortex/CreateAgentModal.tsx` | Modal dialog for creating new agent personas. |
| `src/components/cortex/EditAgentPersonaModal.tsx` | Modal dialog for updating agent persona metadata, icon, and color. |
| `src/components/cortex/GlobalRulesSection.tsx` | Global shared rules list (`.cvh/rules/*.md`), editor triggers, and controls. |
| `src/components/cortex/NewCortexFileForm.tsx` | Inline form for creating new role rules or stage hooks with live filename preview. |
| `src/components/cortex/RuleEditorModal.tsx` | Markdown and script editor for global rules, role rules, and hook scripts. |
| `src/components/cortex/types.ts` | Cortex explorer component props, form state, and tree node interfaces. |
| `src/stores/cortexStore.ts` | Composed Zustand Cortex store (`useCortexStore`) and helper/type re-exports. |
| `src/stores/cortex/cortexStoreSlice.ts` | Main Cortex Zustand slice combining stage tree expansion, persona actions, and file CRUD. |
| `src/stores/cortex/cortexFileOps.ts` | File loading (`openFileAction`), global rule deletion, saving, and rule reordering. |
| `src/stores/cortex/cortexTemplates.ts` | Persona/setting template actions and Cortex rule/hook/global rule creation actions. |
| `src/stores/cortex/cortexSync.ts` | Stage-to-column synchronization (`syncStagesFromColumnsAction`) and persona CRUD/color/icon IPC sync. |
| `src/stores/cortex/cortexHelpers.ts` | Path resolution (`cortexAgentDir`), rule/hook/setting/persona templates, command tokenization, and script path extraction. |
| `src/stores/cortex/cortexConstants.ts` | `DEFAULT_CORTEX_STAGES` fallback tree and recognized `SCRIPT_EXTENSIONS`. |
| `src/stores/cortex/cortexListeners.ts` | DEPRECATED stub (`export {}`; Tauri events centralized in `src/services/tauriEventGateway.ts`). Removal plan: delete file + barrel re-export in `src/stores/cortex/index.ts` + import in `src/stores/cortexStore.ts` in one diff; do not add new imports. |
| `src/stores/cortex/types.ts` | `AgentPersonaInfo`, `CortexHookMoment`, `CortexHookExt`, and Cortex slice state/action contracts. |
| `src/stores/cortex/index.ts` | Barrel re-exports for Cortex store types, constants, helpers, and slices. |

## Related checks

Choose checks for the changed behavior; use the active role's `sensors` rule for execution policy.

- `src/components/cortex/__tests__/AgentSettingReasoning.test.ts` — Reasoning effort and agent setting editor checks.
- `src/stores/__tests__/cortexStore.test.ts` — Cortex file tree, hook resolution, and persona synchronization tests.

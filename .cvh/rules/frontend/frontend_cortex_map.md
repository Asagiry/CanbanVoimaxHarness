---
description: Cortex explorer, agent persona editors, stage cards, rule editors and Cortex store slices.
---
# Cortex explorer and persona/rule editors

Cortex inspects and edits agent personas (`.cvh/agents/<role>/persona.md`), role settings (`.cvh/agents/<role>/setting`), modular rules (`.cvh/rules/*.md` and `.cvh/agents/<role>/rules/*.md`), and stage transition hooks (`.cvh/hooks/*`). All components and store modules strictly observe the <=350 lines ceiling. For provider settings and tool policies, load `frontend_settings_map`.

| File | Responsibility |
| --- | --- |
| `src/components/cortex/CortexExplorer.tsx` | File navigation tree for pipeline stages, personas, rules, and hooks. |
| `src/components/cortex/CvhSaveButton.tsx` | Button triggering Git save and commit for Cortex modifications. |
| `src/components/cortex/DirtyDot.tsx` | Visual indicator for unsaved/dirty configuration files in Cortex tree. |
| `src/components/cortex/AgentSettingVisualEditor.tsx` | Public facade re-exporting persona visual setting editor. |
| `src/components/cortex/settingEditor/CustomizationSection.tsx` | Persona prompt editing, agent identity, icon, and theme color pickers. |
| `src/components/cortex/settingEditor/ModelParametersSection.tsx` | Model selector, temperature, reasoning effort, and provider overrides. |
| `src/components/cortex/settingEditor/ContextLimitsSection.tsx` | Context window size inputs and compaction trigger thresholds. |
| `src/components/cortex/settingEditor/settingSchema.ts` | Schema definitions and validation for agent settings form fields. |
| `src/components/cortex/settingEditor/index.ts` | Barrel re-exports for agent setting editor sections. |
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
| `src/stores/cortex/cortexGitSave.ts` | Automated Git save, stage committing, and synchronization for `.cvh/` edits. |
| `src/stores/cortex/cortexTemplates.ts` | Persona/setting template actions and Cortex rule/hook/global rule creation actions. |
| `src/stores/cortex/cortexSync.ts` | Stage-to-column synchronization (`syncStagesFromColumnsAction`) and persona CRUD/color/icon IPC sync. |
| `src/stores/cortex/cortexHelpers.ts` | Path resolution (`cortexAgentDir`), rule/hook/setting/persona templates, command tokenization, and script path extraction. |
| `src/stores/cortex/cortexConstants.ts` | `DEFAULT_CORTEX_STAGES` fallback tree and recognized `SCRIPT_EXTENSIONS`. |
| `src/stores/cortex/types.ts` | `AgentPersonaInfo`, `CortexHookMoment`, `CortexHookExt`, and Cortex slice state/action contracts. |
| `src/stores/cortex/index.ts` | Barrel re-exports for Cortex store types, constants, helpers, and slices. |

## Related checks

Choose checks for the changed behavior; use the active role's `sensors` rule for execution policy.

- `src/components/cortex/__tests__/AgentSettingReasoning.test.ts` — Reasoning effort and agent setting editor checks.
- `src/components/cortex/__tests__/CvhSaveButton.test.tsx` — Cortex Git save button rendering and action tests.
- `src/stores/__tests__/cortexStore.test.ts` — Cortex file tree, hook resolution, and persona synchronization tests.

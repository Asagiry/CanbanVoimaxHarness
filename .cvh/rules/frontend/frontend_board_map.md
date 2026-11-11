---
description: Board, cards, stage actions, task stores, and DND coordination.
---
# Board and cards

Task lifecycle and IDs are backend-authoritative. Card components use focused store selectors; `cardComparators.ts` owns card memo equality. All components and slices strictly observe the <=350 lines ceiling.

| File | Responsibility |
| --- | --- |
| `src/components/card/KanbanCard.tsx` | Card layout coordinator, memoized with `areCardPropsEqual`. |
| `src/components/card/cardFamily.ts` | Subtask family relationships, root task resolution, and child aggregation. |
| `src/components/card/cardModel.ts` | Pure card model transformers, badge calculations, and status predicates. |
| `src/components/card/CardHookNode.tsx` | Stage stepper, stage switching and running state. |
| `src/components/card/CardHookPopover.tsx` | Hover/focus popover showing stage hook scripts and descriptions. |
| `src/components/card/useCardHooks.ts` | Card hook node left/right routing (source vs target) and hook state selection. |
| `src/components/card/CardTimerBadge.tsx` | Execution duration, active step timer, tabular-nums; exports `CardTimerBadge`, `CardTimeFooter`, `formatCardDate`, `formatStageDuration`. |
| `src/components/card/CardContextMenu.tsx` | Context actions, model selection, subtask deletion. |
| `src/components/card/TaskContextMenu.tsx` | Re-export of `CardContextMenu.tsx` (backward-compatible alias, no separate responsibility). |
| `src/components/card/CardSubtaskBadge.tsx` | Subtask counter/status; exports `CardWaitingSubtaskBadge`, `CardSubtaskProgressBar`, `CardSubtaskBadge`. |
| `src/components/card/CardTodoProgress.tsx` | Task checklist progress and item count. |
| `src/components/card/CardTodoPopover.tsx` | Expandable checklist popover for `CardTodoProgress`. |
| `src/components/card/cardComparators.ts` | `areCardPropsEqual` preventing cascading re-renders. |
| `src/components/board/KanbanBoard.tsx` | Multi-column DND layout, column rendering. |
| `src/components/board/useBoardDragDrop.ts` | Drag-and-drop sensor handling, drop animations, and task reordering. |
| `src/components/board/useBoardTaskFilter.ts` | Live board task filtering, search query handling, and foreign task exclusion. |
| `src/components/board/KanbanColumn.tsx` | Stage column, auto-run emitter, column settings. |
| `src/components/board/BoardTabsHeader.tsx` | Multi-board tab switching, board creation modal trigger. |
| `src/components/board/StageContextMenu.tsx` | Public facade re-exporting stage settings and context menu views. |
| `src/components/board/stageMenu/StageMainMenu.tsx` | Primary stage context menu view: rename, persona, script, delete actions. |
| `src/components/board/stageMenu/StageRenameView.tsx` | In-menu stage title editor with validation and Enter-to-save. |
| `src/components/board/stageMenu/StagePersonaView.tsx` | In-menu persona assignment and agent model selector. |
| `src/components/board/stageMenu/StageScriptView.tsx` | In-menu script configuration for on_enter/on_leave hooks. |
| `src/components/board/stageMenu/pipelineConfig.ts` | Pipeline stage helper predicates and icon resolvers. |
| `src/components/board/stageMenu/index.ts` | Barrel re-exports for stage menu components. |
| `src/components/board/StageHookBadge.tsx` | Transition script and gatekeeper sensor status. |
| `src/components/board/ColumnContextMenu.tsx` | Re-export of `StageContextMenu.tsx` (backward-compatible alias, no separate responsibility). |
| `src/components/board/ColumnHookBadge.tsx` | Re-export of `StageHookBadge.tsx` (backward-compatible alias, no separate responsibility). |
| `src/components/board/modals/CreateTaskModal.tsx` | Task creation modal with persona and catalog model selection. |
| `src/components/board/modals/TaskAttachmentPicker.tsx` | Image attachment dropzone, paste handling, and thumbnail gallery for `CreateTaskModal`. |
| `src/components/board/modals/CreateColumnModal.tsx` | Column creation modal with hook configuration. |
| `src/components/board/modals/StageAppearancePicker.tsx` | Stage icon and color picker extracted from `CreateColumnModal`. |
| `src/components/board/CreateBoardModal.tsx` | Board creation modal with preset templates. |
| `src/components/board/modals/ImageLightboxModal.tsx` | Re-export of the canonical drawer `ImageLightboxModal.tsx` (no separate responsibility). |
| `src/hooks/useContextMenuPosition.ts` | Viewport-clamped positioning and outside-click/Escape dismissal for context menus. |
| `src/stores/boardStore.ts` | Composed Zustand board store (`useBoardStore`) and domain re-exports. |
| `src/stores/board/boardConstants.ts` | `CANONICAL_BOARD_STAGES`, `CANONICAL_BOARD_COLUMNS`, and `DEFAULT_BOARD`. |
| `src/stores/board/boardHelpers.ts` | Task position sorting, terminal stage detection (`isDoneStageDef`), completion sorting. |
| `src/stores/board/hookStorage.ts` | `localStorage` hydration and persistence for task hook execution states (`cvh_task_hook_states`). |
| `src/stores/board/taskNormalizer.ts` | Dynamic persona mapping for terminal/working stages, foreign-project filtering (`isTaskForeign`). |
| `src/stores/board/taskSync.ts` | `syncTasksBackend` payload serialization and deduplicated `triggerRunAgent` invocation. |
| `src/stores/slices/tasksCrudSlice.ts` | Public facade re-exporting decomposed task CRUD slices. |
| `src/stores/slices/tasks/tasksCoreSlice.ts` | Core task retrieval, task normalization, and task list state. |
| `src/stores/slices/tasks/tasksCreateSlice.ts` | Task and subtask creation actions and initial stage assignment. |
| `src/stores/slices/tasks/tasksMoveSlice.ts` | Stage transitions, drag reorder, and optimistic position updates. |
| `src/stores/slices/tasks/tasksMutateSlice.ts` | Task updating, description edits, task deletion, and status mutations. |
| `src/stores/slices/tasks/index.ts` | Barrel re-exports for task store slices. |
| `src/stores/slices/boardMetaSlice.ts` | Active board, board switching, stage/column layout management. |
| `src/stores/slices/taskTimersSlice.ts` | Active task step timers and execution stopwatch. |
| `src/stores/slices/taskHooksSlice.ts` | Per-task `on_enter`/`on_leave` hook start, finish, and clear state transitions. |
| `src/stores/slices/taskQuestionsSlice.ts` | Root-family deduplicated interactive clarifying question state (`activeQuestions`). |
| `src/stores/slices/types.ts` | Slice state and action interfaces (`BoardState`, `BoardMetaSlice`, `TasksCrudSlice`, `TaskHooksSlice`, `TaskQuestionsSlice`). |

## Related checks

Choose checks for the changed behavior; use the active role's `sensors` rule for execution policy.

- `src/components/board/__tests__/StageContextMenu.test.tsx` — Stage context menu interactions.
- `src/components/card/__tests__/KanbanCard.test.tsx` — Kanban card rendering and layout.
- `src/stores/__tests__/tasksCrudSlice.test.ts` — Task CRUD and stage transition slice tests.
- `src/stores/__tests__/boardMetaSlice.test.ts` — Board metadata, stage update, and column order slice tests.

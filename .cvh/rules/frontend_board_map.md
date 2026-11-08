---
description: Board, cards, stage actions and task stores.
---
# Board and cards

Task lifecycle and IDs are backend-authoritative. Card components use focused store selectors; `cardComparators.ts` owns card memo equality.

| File | Responsibility |
| --- | --- |
| `src/components/card/KanbanCard.tsx` | Card layout coordinator, memoized with `areCardPropsEqual`. |
| `src/components/card/CardHookNode.tsx` | Stage stepper, stage switching and running state. |
| `src/components/card/useCardHooks.ts` | Card hook node left/right routing (source vs target) and hook state selection. |
| `src/components/card/CardTimerBadge.tsx` | Execution duration, active step timer, tabular-nums. |
| `src/components/card/CardContextMenu.tsx` | Context actions, model selection, subtask deletion. |
| `src/components/card/TaskContextMenu.tsx` | Backward-compatible alias for `CardContextMenu.tsx`. |
| `src/components/card/CardSubtaskBadge.tsx` | Subtask counter, status indicator, auto-unlock indicator. |
| `src/components/card/CardTodoProgress.tsx` | Task checklist progress and item count. |
| `src/components/card/cardComparators.ts` | `areCardPropsEqual` preventing cascading re-renders. |
| `src/components/board/KanbanBoard.tsx` | Multi-column DND layout, column rendering. |
| `src/components/board/KanbanColumn.tsx` | Stage column, auto-run emitter, column settings. |
| `src/components/board/BoardTabsHeader.tsx` | Multi-board tab switching, board creation modal trigger. |
| `src/components/board/StageContextMenu.tsx` | Stage settings, persona assignment, entry/leave hook scripts, rename/delete. |
| `src/components/board/StageHookBadge.tsx` | Transition script and gatekeeper sensor status. |
| `src/components/board/ColumnContextMenu.tsx` | Backward-compatible alias for `StageContextMenu.tsx`. |
| `src/components/board/ColumnHookBadge.tsx` | Backward-compatible alias for `StageHookBadge.tsx`. |
| `src/components/board/modals/CreateTaskModal.tsx` | Task creation modal with persona and catalog model selection. |
| `src/components/board/modals/CreateColumnModal.tsx` | Column creation modal with hook configuration. |
| `src/components/board/CreateBoardModal.tsx` | Board creation modal with preset templates. |
| `src/components/board/modals/ImageLightboxModal.tsx` | Full-screen image preview for board attachments. |
| `src/hooks/useContextMenuPosition.ts` | Viewport-clamped positioning and outside-click/Escape dismissal for context menus. |
| `src/stores/boardStore.ts` | Composed Zustand board store (`useBoardStore`) and domain re-exports. |
| `src/stores/board/boardConstants.ts` | `CANONICAL_BOARD_STAGES`, `CANONICAL_BOARD_COLUMNS`, and `DEFAULT_BOARD`. |
| `src/stores/board/boardHelpers.ts` | Task position sorting, terminal stage detection (`isDoneStageDef`), completion sorting. |
| `src/stores/board/hookStorage.ts` | `localStorage` hydration and persistence for task hook execution states (`cvh_task_hook_states`). |
| `src/stores/board/taskNormalizer.ts` | Dynamic persona mapping for terminal/working stages, foreign-project filtering (`isTaskForeign`). |
| `src/stores/board/taskSync.ts` | `syncTasksBackend` payload serialization and deduplicated `triggerRunAgent` invocation. |
| `src/stores/slices/tasksCrudSlice.ts` | Backend-authoritative task CRUD, stage transitions, and monotonic IDs. |
| `src/stores/slices/boardMetaSlice.ts` | Active board, board switching, stage/column layout management. |
| `src/stores/slices/taskTimersSlice.ts` | Active task step timers and execution stopwatch. |
| `src/stores/slices/taskHooksSlice.ts` | Per-task `on_enter`/`on_leave` hook start, finish, and clear state transitions. |
| `src/stores/slices/taskQuestionsSlice.ts` | Root-family deduplicated interactive clarifying question state (`activeQuestions`). |
| `src/stores/slices/types.ts` | Slice state and action interfaces (`BoardState`, `BoardMetaSlice`, `TasksCrudSlice`, `TaskHooksSlice`, `TaskQuestionsSlice`). |

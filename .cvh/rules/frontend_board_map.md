---
description: Board, cards, stage actions and task stores.
---
# Board and cards

Task lifecycle and IDs are backend-authoritative. Card components use focused store selectors; `cardComparators.ts` owns card memo equality.

| File | Responsibility |
| --- | --- |
| `src/components/card/KanbanCard.tsx` | Card layout coordinator, memoized with `areCardPropsEqual`. |
| `src/components/card/CardHookNode.tsx` | Stage stepper, stage switching and running state. |
| `src/components/card/CardTimerBadge.tsx` | Execution duration, active step timer, tabular-nums. |
| `src/components/card/StageDurationsPopover.tsx` | Per-stage active duration breakdown, animated running indicator, total active time. |
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
| `src/components/board/modals/CreateTaskModal.tsx` | Task creation modal with persona & model selection. |
| `src/components/board/modals/CreateColumnModal.tsx` | Column creation modal with hook configuration. |
| `src/components/board/CreateBoardModal.tsx` | Board creation modal with preset templates. |
| `src/components/board/modals/ImageLightboxModal.tsx` | Full-screen image preview for board attachments. |
| `src/stores/board/taskNormalizer.ts` | Dynamic persona mapping for terminal/working stages. |
| `src/stores/slices/tasksCrudSlice.ts` | Backend-authoritative task CRUD, monotonic IDs. |
| `src/stores/slices/boardMetaSlice.ts` | Active board, board switching, column layout. |
| `src/stores/slices/taskTimersSlice.ts` | Active task step timers and execution stopwatch. |

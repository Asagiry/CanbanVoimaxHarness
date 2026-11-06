import { create } from 'zustand';
import { createBoardMetaSlice } from './slices/boardMetaSlice';
import { createTaskTimersSlice } from './slices/taskTimersSlice';
import { createTasksCrudSlice } from './slices/tasksCrudSlice';
import { BoardState } from './slices/types';

// Ub-mode Re-exports for zero breaking changes across codebase
export {
  CANONICAL_BOARD_STAGES,
  CANONICAL_BOARD_COLUMNS,
  DEFAULT_BOARD,
} from './board/boardConstants';

export {
  sortTasksByPosition,
  isDoneStageDef,
  isDoneColumnDef,
  getTaskCompletionTime,
  sortDoneTasksByCompletionDateDesc,
} from './board/boardHelpers';

export {
  HOOK_STATES_KEY,
  loadCachedHookStates,
  saveCachedHookStates,
  persistCachedHookStates,
} from './board/hookStorage';

export {
  normalizeKanbanTask,
  mapBackendTaskToKanban,
  isTaskForeign,
} from './board/taskNormalizer';

export {
  syncTasksBackend,
  triggerRunAgent,
} from './board/taskSync';

export type {
  ActiveQuestionInfo,
  BoardMetaSlice,
  TaskTimersSlice,
  TasksCrudSlice,
  BoardState,
} from './slices/types';

export const useBoardStore = create<BoardState>()((...a) => ({
  ...createBoardMetaSlice(...a),
  ...createTaskTimersSlice(...a),
  ...createTasksCrudSlice(...a),
}));

import { create } from 'zustand';
import { createBoardMetaSlice } from './slices/boardMetaSlice';
import { createTaskTimersSlice } from './slices/taskTimersSlice';
import { createTasksCrudSlice } from './slices/tasksCrudSlice';
import { BoardState } from './slices/types';

// Ub-mode Re-exports for zero breaking changes across codebase
export {
  CANONICAL_BOARD_COLUMNS,
  DEFAULT_BOARD,
} from './board/boardConstants';

export {
  sortTasksByPosition,
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

let boardListenersInitialized = false;

export async function initBoardEventListeners(): Promise<() => void> {
  if (boardListenersInitialized) return () => {};
  try {
    const { listen } = await import('@tauri-apps/api/event');
    const uBoards = await listen('boards:updated', async () => {
      await useBoardStore.getState().loadBoards();
      await useBoardStore.getState().loadTasks();
    });
    const uPipeline = await listen('pipeline:updated', async () => {
      await useBoardStore.getState().loadBoards();
      await useBoardStore.getState().loadTasks();
    });
    const uHookStart = await listen<{ taskId?: string; stageId?: string; hookType?: 'on_enter' | 'on_leave'; hookName?: string; description?: string }>('hook:start', (event) => {
      const { taskId, stageId, hookType, hookName, description } = event.payload || {};
      if (taskId && stageId && hookType && hookName) {
        useBoardStore.getState().setHookStart({ taskId, stageId, hookType, hookName, description });
      }
    });
    const uHookFinish = await listen<{ taskId?: string; stageId?: string; hookType?: 'on_enter' | 'on_leave'; success?: boolean; exitCode?: number | null; stdout?: string; stderr?: string; error?: string }>('hook:finish', (event) => {
      const { taskId, stageId, hookType, success, exitCode, stdout, stderr, error } = event.payload || {};
      if (taskId && stageId && hookType) {
        useBoardStore.getState().setHookFinish({
          taskId,
          stageId,
          hookType,
          success: Boolean(success),
          exitCode,
          stdout,
          stderr,
          error,
        });
      }
    });
    boardListenersInitialized = true;
    return () => {
      uBoards();
      uPipeline();
      uHookStart();
      uHookFinish();
      boardListenersInitialized = false;
    };
  } catch (e) {
    console.info('initBoardEventListeners not active in web mode:', e);
    return () => {};
  }
}

import { StateCreator } from 'zustand';
import { KanbanTask, BackendTaskDTO, getHookScript } from '../../../types';
import { BoardState, TasksCrudCoreSlice } from '../types';
import { sortTasksByPosition, isDoneColumnDef } from '../../board/boardHelpers';
import { mapBackendTaskToKanban } from '../../board/taskNormalizer';
import { triggerRunAgent } from '../../board/taskSync';
import { logEvent } from '../../../lib/logger';

export type TasksMoveSlice = Pick<
  TasksCrudCoreSlice,
  'moveTask' | 'reorderTask' | 'promoteTaskToNextStage'
>;

export const createTasksMoveSlice: StateCreator<BoardState, [], [], TasksMoveSlice> = (set, get) => ({
  moveTask: async (taskId, targetColumnId, options) => {
    const { tasks, columns, activeBoardId } = get();
    const task = tasks.find(
      (t) => t.id === taskId || t.id.trim().toLowerCase() === taskId.trim().toLowerCase()
    );
    const targetStageId = (targetColumnId || '').trim();
    if (!task) {
      console.warn(`[moveTask] Task "${taskId}" not found in local store`);
      return;
    }
    const resolvedTaskId = task.id;
    if (!targetStageId) {
      console.warn(`[moveTask] Invalid empty targetStageId for task "${resolvedTaskId}"`);
      return;
    }

    if (!options?.force && task.stageId === targetStageId) {
      return;
    }

    const targetColDef = columns.find((c) => c.id === targetStageId);
    console.info(`[moveTask] Moving task "${resolvedTaskId}" from "${task.stageId}" to "${targetStageId}" (board: ${activeBoardId})`);
    logEvent(
      'INFO',
      'BOARD_STORE',
      `moveTask: starting move for "${resolvedTaskId}" from "${task.stageId}" to "${targetStageId}"`,
      JSON.stringify({ activeBoardId, originalStatus: task.status, force: options?.force })
    );

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const backendTask = await invoke<BackendTaskDTO>('move_task_stage', {
        taskId: resolvedTaskId,
        targetStageId,
        boardId: task.boardId || activeBoardId,
      });
      if (backendTask && typeof backendTask.id === 'string') {
        const mapped = mapBackendTaskToKanban(backendTask);
        if (!mapped) return;
        const prev = get().tasks.find((t) => t.id === resolvedTaskId || t.id.trim().toLowerCase() === resolvedTaskId.trim().toLowerCase()) ?? task;
        const sourceColId = options?.sourceStageId ?? options?.sourceColumnId ?? prev.stageId;
        const sourceCol = columns.find((c) => c.id === sourceColId);
        const sourceHasLeaveHook = Boolean(
          getHookScript(sourceCol?.hooks?.['on_leave']) ||
          getHookScript(sourceCol?.hooks?.['on_exit']) ||
          getHookScript(sourceCol?.hooks?.['on_finish'])
        );
        const targetHasEnterHook = Boolean(getHookScript(targetColDef?.hooks?.['on_enter']));

        let nextTaskHookStates = mapped.hookStates
          ? { ...mapped.hookStates }
          : (prev.hookStates ? { ...prev.hookStates } : undefined);
        if (!sourceHasLeaveHook && nextTaskHookStates?.on_leave) {
          const cleaned = { ...nextTaskHookStates };
          delete cleaned.on_leave;
          nextTaskHookStates = Object.keys(cleaned).length > 0 ? cleaned : undefined;
          get().clearHookState(resolvedTaskId, 'on_leave');
        }
        if (!targetHasEnterHook && nextTaskHookStates?.on_enter) {
          const cleaned = { ...nextTaskHookStates };
          delete cleaned.on_enter;
          nextTaskHookStates = Object.keys(cleaned).length > 0 ? cleaned : undefined;
          get().clearHookState(resolvedTaskId, 'on_enter');
        }

        const sourceColIndex = columns.findIndex((c) => c.id === sourceColId);
        const targetColIndex = columns.findIndex((c) => c.id === targetStageId);
        const isRework =
          sourceColIndex >= 0 && targetColIndex >= 0 && targetColIndex < sourceColIndex;
        const currentDuration =
          typeof prev.activeStageDuration === 'number' && Number.isFinite(prev.activeStageDuration)
            ? prev.activeStageDuration
            : 0;
        const prevStageDurations =
          prev.stageDurations && typeof prev.stageDurations === 'object'
            ? prev.stageDurations
            : {};
        const merged: KanbanTask = {
          ...mapped,
          stageId: targetStageId,
          columnId: targetStageId,
          previousStageId: sourceColId,
          hookStates: nextTaskHookStates,
          boardId: mapped.boardId || prev.boardId || task.boardId || activeBoardId || 'development',
          persona: targetColDef?.persona || prev.persona,
          verdict: backendTask.verdict ? mapped.verdict : prev.verdict,
          verdictNote: prev.verdictNote,
          priority: prev.priority,
          attachments: mapped.attachments ?? prev.attachments,
          handoverSummary: mapped.handoverSummary ?? prev.handoverSummary,
          todos: mapped.todos !== undefined ? mapped.todos : prev.todos,
          stageEnteredAt: backendTask.stageEnteredAt || backendTask.stage_entered_at || Date.now(),
          updatedAt: Date.now(),
          activeStageDuration: 0,
          stageDurations: {
            ...prevStageDurations,
            [prev.stageId]: Math.max(prevStageDurations[prev.stageId] || 0, currentDuration),
          },
          totalElapsed: prev.totalElapsed,
          isTimerRunning: mapped.status === 'running',
          returnsCount: isRework ? prev.returnsCount + 1 : prev.returnsCount,
        };
        const isTerminalMove = isDoneColumnDef(targetColDef) || targetStageId === 'ready';
        if (isTerminalMove) {
          merged.waitingForSubtaskId = undefined;
          merged.status = 'done';
          merged.completedAt = Date.now();
          const sumOfStageDurations = Object.values(merged.stageDurations || {}).reduce(
            (acc, val) => acc + (typeof val === 'number' && Number.isFinite(val) ? val : 0),
            0
          );
          const activeElapsed =
            typeof merged.totalElapsed === 'number' && merged.totalElapsed > 0
              ? merged.totalElapsed
              : sumOfStageDurations;
          merged.totalElapsed = merged.createdAt
            ? Math.max(0, Math.floor((merged.completedAt - merged.createdAt) / 1000))
            : activeElapsed;
        } else {
          merged.completedAt = undefined;
        }
        const updatedTasks = get()
          .tasks.map((t) =>
            t.id === resolvedTaskId || t.id.trim().toLowerCase() === resolvedTaskId.trim().toLowerCase() ? merged : t
          )
          .sort(sortTasksByPosition);
        set({ tasks: updatedTasks });
        logEvent(
          'INFO',
          'BOARD_STORE',
          `moveTask: backend move_task_stage succeeded for "${resolvedTaskId}"`,
          JSON.stringify({ newStage: merged.stageId, newStatus: merged.status, boardId: merged.boardId })
        );

        if (merged.boardId && merged.boardId !== get().activeBoardId) {
          const boardExists = get().boards.some((b) => b.id === merged.boardId);
          if (boardExists) {
            await get().setActiveBoard(merged.boardId);
          }
        }

        const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
        if (!isTauri) {
          const colDef = targetColDef ?? columns.find((c) => c.id === merged.stageId);
          if (colDef?.autoRun === true || (colDef?.autoRun !== false && Boolean(colDef?.persona))) {
            await triggerRunAgent(resolvedTaskId, undefined, { skipSync: true });
          }
        }
        return;
      }
      throw new Error(`move_task_stage returned empty or invalid task for id "${resolvedTaskId}"`);
    } catch (err: unknown) {
      console.error(`[moveTask] Backend move_task_stage failed for task "${resolvedTaskId}" -> stage "${targetStageId}":`, err);
      const msg = typeof err === 'string' ? err : err instanceof Error ? err.message : JSON.stringify(err);
      logEvent(
        'ERROR',
        'BOARD_STORE',
        `moveTask: backend move_task_stage REJECTED for "${resolvedTaskId}" -> "${targetStageId}"`,
        msg
      );
      throw (err instanceof Error ? err : new Error(msg));
    }
  },

  reorderTask: async (taskId, targetStageId, newIndex) => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const backendTask = await invoke<BackendTaskDTO>('reorder_task', {
        taskId,
        targetStageId,
        newIndex,
      });
      if (backendTask && typeof backendTask.id === 'string') {
        const mapped = mapBackendTaskToKanban(backendTask);
        if (!mapped) return;
        const { tasks, columns } = get();
        const targetColDef = columns.find((c) => c.id === targetStageId);
        const isTerminalMove = isDoneColumnDef(targetColDef) || targetStageId === 'ready';
        if (isTerminalMove) {
          mapped.waitingForSubtaskId = undefined;
          mapped.status = 'done';
          mapped.completedAt = Date.now();
          if (mapped.createdAt) {
            mapped.totalElapsed = Math.max(0, Math.floor((mapped.completedAt - mapped.createdAt) / 1000));
          }
        } else {
          mapped.completedAt = undefined;
        }
        const prev = tasks.find((t) => t.id === mapped.id);
        const preservedTodos = mapped.todos !== undefined ? mapped.todos : prev?.todos;
        const updated = tasks.map((t) => (t.id === mapped.id ? { ...t, ...mapped, todos: preservedTodos } : t)).sort(sortTasksByPosition);
        set({ tasks: updated });

        if (mapped.boardId && mapped.boardId !== get().activeBoardId) {
          const boardExists = get().boards.some((b) => b.id === mapped.boardId);
          if (boardExists) {
            await get().setActiveBoard(mapped.boardId);
          }
        }

        if (targetColDef?.autoRun === true || (targetColDef?.autoRun !== false && Boolean(targetColDef?.persona))) {
          const prevTask = tasks.find((t) => t.id === taskId);
          if (prevTask && prevTask.stageId !== targetStageId) {
            await triggerRunAgent(taskId);
          }
        }
      }
    } catch (err) {
      console.error(`[reorderTask] Backend reorder_task failed for task "${taskId}":`, err);
      const msg = typeof err === 'string' ? err : err instanceof Error ? err.message : JSON.stringify(err);
      logEvent('ERROR', 'BOARD_STORE', `reorderTask: backend reorder_task failed for "${taskId}"`, msg);
      throw (err instanceof Error ? err : new Error(msg));
    }
  },

  promoteTaskToNextStage: (taskId) => {
    const { tasks, columns, moveTask } = get();
    const task = tasks.find((t) => t.id === taskId);
    if (!task) return;

    if (task.customFlow && task.customFlow.length > 0) {
      const currentIndexInFlow = task.customFlow.indexOf(task.stageId);
      if (currentIndexInFlow >= 0 && currentIndexInFlow < task.customFlow.length - 1) {
        const nextStage = task.customFlow[currentIndexInFlow + 1];
        if (columns.some((c) => c.id === nextStage)) {
          moveTask(taskId, nextStage);
          return;
        }
      } else if (currentIndexInFlow >= 0) {
        return;
      }
    }

    const currentIndex = columns.findIndex((c) => c.id === task.stageId);
    if (currentIndex >= 0 && currentIndex < columns.length - 1) {
      const nextCol = columns[currentIndex + 1];
      moveTask(taskId, nextCol.id);
    }
  },
});

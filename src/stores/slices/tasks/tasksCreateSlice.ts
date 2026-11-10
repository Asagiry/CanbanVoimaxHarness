import { StateCreator } from 'zustand';
import { TaskStatus, BackendTaskDTO } from '../../../types';
import { BoardState, TasksCrudCoreSlice } from '../types';
import { sortTasksByPosition } from '../../board/boardHelpers';
import { mapBackendTaskToKanban } from '../../board/taskNormalizer';
import { triggerRunAgent } from '../../board/taskSync';
import { logEvent } from '../../../lib/logger';

export type TasksCreateSlice = Pick<TasksCrudCoreSlice, 'addTask' | 'createSubtask'>;

export const createTasksCreateSlice: StateCreator<BoardState, [], [], TasksCreateSlice> = (set, get) => ({
  addTask: async (input) => {
    const state = get();
    const intakeCol = state.columns.find((c) => c.isIntake);
    const targetStageId =
      input.stageId ||
      input.columnId ||
      intakeCol?.id ||
      (state.columns.some((c) => c.id === 'backlog') ? 'backlog' : (state.columns[0]?.id ?? 'backlog'));
    const boardId = input.boardId || state.activeBoardId || 'development';
    const title = input.title.trim();
    if (!title) return null;
    const description = input.description?.trim() || '';

    logEvent(
      'INFO',
      'BOARD_STORE',
      `addTask: initiated for title="${title}"`,
      JSON.stringify({ targetStageId, boardId, model: input.model })
    );

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const backendTask = await invoke<BackendTaskDTO>('create_task', {
        title,
        description,
        stageId: targetStageId,
        boardId,
        model: input.model || undefined,
        attachments: input.attachments && input.attachments.length > 0 ? input.attachments : undefined,
      });

      if (backendTask && typeof backendTask.id === 'string') {
        const confirmed = mapBackendTaskToKanban({
          ...backendTask,
          stageEnteredAt: backendTask.stageEnteredAt || backendTask.stage_entered_at || Date.now(),
        });
        if (!confirmed) return null;
        const currentTasks = get().tasks;
        const nextTasks = [confirmed, ...currentTasks.filter((t) => t.id !== confirmed.id)].sort(sortTasksByPosition);

        set({ tasks: nextTasks });

        logEvent(
          'INFO',
          'BOARD_STORE',
          `addTask: backend create_task confirmed for "${confirmed.id}"`,
          JSON.stringify({ stageId: confirmed.stageId, boardId: confirmed.boardId, worktree: confirmed.worktreePath })
        );

        const colDef = get().columns.find((c) => c.id === confirmed.stageId);
        if (colDef?.autoRun ?? !!colDef?.persona) {
          await triggerRunAgent(confirmed.id);
        }
        return confirmed;
      }
      return null;
    } catch (err) {
      const errMsg = typeof err === 'string' ? err : err instanceof Error ? err.message : JSON.stringify(err);
      console.error('[addTask] Backend create_task failed:', err);
      logEvent('ERROR', 'BOARD_STORE', 'addTask: backend create_task failed', errMsg);
      alert(`Не удалось создать задачу:\n${errMsg}`);
      return null;
    }
  },

  createSubtask: async (parentId: string, targetStageId?: string) => {
    const { tasks, columns } = get();
    const parent = tasks.find((t) => t.id === parentId);
    if (!parent) return null;

    const parentColIndex = columns.findIndex((c) => c.id === parent.stageId);
    const targetCol = targetStageId
      ? columns.find((c) => c.id === targetStageId)
      : parentColIndex >= 0 && parentColIndex + 1 < columns.length
      ? columns[parentColIndex + 1]
      : null;

    if (!targetCol) return null;

    logEvent(
      'INFO',
      'BOARD_STORE',
      `createSubtask: initiated for parentId="${parentId}" targetStageId="${targetCol.id}"`
    );

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const backendSubtask = await invoke<BackendTaskDTO>('create_subtask', {
        parentId,
        targetStageId: targetCol.id,
        title: null,
      });

      if (backendSubtask && typeof backendSubtask.id === 'string') {
        const mapped = mapBackendTaskToKanban({
          ...backendSubtask,
          stageEnteredAt: backendSubtask.stageEnteredAt || backendSubtask.stage_entered_at || Date.now(),
        });
        if (!mapped) return null;
        const currentTasks = get().tasks;
        const updatedTasks = currentTasks.map((t) => {
          if (t.id === parentId) {
            return {
              ...t,
              status: 'idle' as TaskStatus,
              waitingForSubtaskId: mapped.id,
              updatedAt: Date.now(),
            };
          }
          return t;
        });
        const finalTasks = [...updatedTasks.filter((t) => t.id !== mapped.id), mapped].sort(sortTasksByPosition);
        set({ tasks: finalTasks });

        logEvent(
          'INFO',
          'BOARD_STORE',
          `createSubtask: backend create_subtask confirmed for "${mapped.id}" (parent: "${parentId}")`,
          JSON.stringify({ stageId: mapped.stageId, status: mapped.status })
        );

        if (targetCol.autoRun ?? !!targetCol.persona) {
          await triggerRunAgent(mapped.id);
        }
        return mapped;
      }
      return null;
    } catch (err) {
      const errMsg = typeof err === 'string' ? err : err instanceof Error ? err.message : JSON.stringify(err);
      console.error(`[createSubtask] Backend create_subtask failed for parent "${parentId}":`, err);
      logEvent('ERROR', 'BOARD_STORE', `createSubtask: backend create_subtask failed for parent "${parentId}"`, errMsg);
      alert(`Не удалось создать подзадачу:\n${errMsg}`);
      return null;
    }
  },
});

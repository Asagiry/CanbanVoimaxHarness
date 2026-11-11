import { StateCreator } from 'zustand';
import { TaskStatus } from '../../../types';
import { BoardState, TasksCrudCoreSlice } from '../types';
import { CANONICAL_BOARD_COLUMNS } from '../../board/boardConstants';
import { syncTasksBackend, triggerRunAgent } from '../../board/taskSync';
import { saveCachedHookStates } from '../../board/hookStorage';
import { useDrawerStore } from '../../drawerStore';
import { logEvent } from '../../../lib/logger';

export type TasksMutateSlice = Pick<
  TasksCrudCoreSlice,
  'updateTask' | 'updateTaskStatus' | 'deleteTask' | 'runTaskAgent' | 'setTaskVerdict'
>;

export const createTasksMutateSlice: StateCreator<BoardState, [], [], TasksMutateSlice> = (set, get) => ({
  updateTask: (taskId, partial) => {
    const prevTask = get().tasks.find((t) => t.id === taskId);
    set((state) => {
      const updatedTasks = state.tasks.map((t) => (t.id === taskId ? { ...t, ...partial } : t));
      return { tasks: updatedTasks, boardError: null };
    });
    void (async () => {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('update_task', {
          taskId,
          model: partial.model !== undefined ? partial.model : null,
          providerId: partial.providerId !== undefined ? partial.providerId : null,
          reasoningEffort: partial.reasoningEffort !== undefined ? partial.reasoningEffort : null,
          title: partial.title ?? null,
          description: partial.description ?? null,
          status: partial.status ?? null,
          verdict: partial.verdict ?? null,
        });
      } catch (err) {
        const errMsg = typeof err === 'string' ? err : err instanceof Error ? err.message : String(err);
        console.error(`[updateTask] Backend update_task failed for "${taskId}", rolling back:`, err);
        logEvent('ERROR', 'BOARD_STORE', `updateTask: backend update_task failed for "${taskId}"`, errMsg);
        if (prevTask) {
          set((state) => ({
            tasks: state.tasks.map((t) => (t.id === taskId ? { ...t, ...prevTask } : t)),
            boardError: errMsg,
          }));
        } else {
          set({ boardError: errMsg });
        }
      }
    })();
  },

  updateTaskStatus: (taskId, status) => {
    set((state) => {
      const updatedTasks = state.tasks.map((t) => {
        if (t.id !== taskId) return t;
        if (status === 'idle' && t.status === 'done') {
          return t;
        }
        const isTimerRunning = status === 'running' ? t.isTimerRunning : false;
        return { ...t, status, isTimerRunning };
      });
      return { tasks: updatedTasks };
    });
  },

  deleteTask: async (taskId) => {
    const prevTasks = get().tasks;
    const target = prevTasks.find(
      (t) => t.id === taskId || t.id.trim().toLowerCase() === taskId.trim().toLowerCase()
    );
    const resolvedId = target ? target.id : taskId;

    const toDelete = new Set<string>([resolvedId, taskId]);
    let grew = true;
    while (grew) {
      grew = false;
      for (const t of prevTasks) {
        if (toDelete.has(t.id)) continue;
        if ((t.parentId && toDelete.has(t.parentId)) || [...toDelete].some((id) => t.id.startsWith(`${id}.`))) {
          toDelete.add(t.id);
          grew = true;
        }
      }
    }
    const ids = [...toDelete];

    logEvent('INFO', 'BOARD_STORE', `deleteTask: requesting backend delete for "${resolvedId}"`, JSON.stringify(ids));

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('delete_task', { taskId: resolvedId });
    } catch (err) {
      const errMsg = typeof err === 'string' ? err : err instanceof Error ? err.message : JSON.stringify(err);
      console.error(`[deleteTask] Backend delete_task failed for "${resolvedId}":`, err);
      logEvent('ERROR', 'BOARD_STORE', `deleteTask: backend delete_task failed for "${resolvedId}"`, errMsg);
      set({ boardError: `Не удалось удалить задачу: ${errMsg}` });
      return;
    }

    // Close drawer and clear local logs/drafts ONLY after backend delete_task succeeds
    const drawerActiveTaskId = useDrawerStore.getState().activeTaskId;
    if (drawerActiveTaskId && toDelete.has(drawerActiveTaskId)) {
      useDrawerStore.getState().closeDrawer();
    }

    for (const id of ids) {
      try {
        await useDrawerStore.getState().clearTaskLogs(id);
      } catch (logErr) {
        console.warn('Failed to clear task logs for deleted task:', id, logErr);
      }
      try {
        useDrawerStore.getState().clearTaskDraft(id);
      } catch (draftErr) {
        console.warn('Failed to clear task draft for deleted task:', id, draftErr);
      }
    }

    const currentTasks = get().tasks;
    const updatedTasks = currentTasks
      .filter((t) => !toDelete.has(t.id))
      .map((t) => {
        if (t.waitingForSubtaskId && toDelete.has(t.waitingForSubtaskId)) {
          return {
            ...t,
            waitingForSubtaskId: undefined,
            status: t.status === 'waiting_subtask' ? ('idle' as TaskStatus) : t.status,
          };
        }
        return t;
      });

    const prevSelectedTaskId = get().selectedTaskId;
    const nextHookStates = { ...get().hookStates };
    for (const id of ids) {
      delete nextHookStates[id];
    }
    saveCachedHookStates(nextHookStates);
    set({
      tasks: updatedTasks,
      hookStates: nextHookStates,
      selectedTaskId: toDelete.has(prevSelectedTaskId ?? '') ? null : prevSelectedTaskId,
      boardError: null,
    });
  },

  runTaskAgent: async (taskId, prompt, options) => {
    const { tasks, boards, activeBoardId, columns: activeColumns, moveTask } = get();
    let task = tasks.find(
      (t) => t.id === taskId || t.id.trim().toLowerCase() === taskId.trim().toLowerCase()
    );
    if (!task) {
      await get().loadTasks();
      task = get().tasks.find(
        (t) => t.id === taskId || t.id.trim().toLowerCase() === taskId.trim().toLowerCase()
      );
      if (!task) {
        console.warn(`[runTaskAgent] Task "${taskId}" not found in local store`);
        logEvent('WARN', 'BOARD_STORE', `runTaskAgent: Task "${taskId}" not found in local store`);
        return;
      }
    }
    const resolvedTaskId = task.id;
    logEvent(
      'INFO',
      'BOARD_STORE',
      `runTaskAgent: requested for "${resolvedTaskId}"`,
      JSON.stringify({ stageId: task.stageId, boardId: task.boardId, status: task.status })
    );

    const effectiveBoardId = task.boardId || activeBoardId;
    const taskBoard = boards.find((b) => b.id === effectiveBoardId);
    const targetColumns = (taskBoard?.stages && taskBoard.stages.length > 0)
      ? taskBoard.stages
      : (taskBoard?.columns && taskBoard.columns.length > 0)
      ? taskBoard.columns
      : (activeColumns && activeColumns.length > 0 ? activeColumns : CANONICAL_BOARD_COLUMNS);
    const taskCol = targetColumns.find(
      (c) => c.id === task.stageId || (task.stageId && c.id.toLowerCase() === task.stageId.toLowerCase())
    );

    if (taskCol?.terminal || taskCol?.id === 'ready' || taskCol?.id === 'done') {
      logEvent('INFO', 'BOARD_STORE', `runTaskAgent: task "${resolvedTaskId}" is in terminal stage "${task.stageId}", skipping`);
      return;
    } else if (taskCol?.persona || taskCol?.autoRun) {
      logEvent('INFO', 'BOARD_STORE', `runTaskAgent: task "${resolvedTaskId}" is in agent stage "${taskCol.id}", starting agent`);
      const updatedTasks = get().tasks.map((t) =>
        t.id === resolvedTaskId || t.id.trim().toLowerCase() === resolvedTaskId.trim().toLowerCase()
          ? { ...t, status: 'running' as TaskStatus, isTimerRunning: true, updatedAt: Date.now() }
          : t
      );
      set({ tasks: updatedTasks });
      await triggerRunAgent(resolvedTaskId, prompt, options);
    } else if (taskCol?.isIntake || taskCol?.id === 'backlog') {
      const firstAgentCol = targetColumns.find((c) => c.persona || c.autoRun);
      if (firstAgentCol) {
        logEvent('INFO', 'BOARD_STORE', `runTaskAgent: task "${resolvedTaskId}" in intake stage "${task.stageId}", moving to "${firstAgentCol.id}"`);
        await moveTask(resolvedTaskId, firstAgentCol.id);
      } else {
        logEvent('WARN', 'BOARD_STORE', `runTaskAgent: no agent stage found for task "${resolvedTaskId}"`);
      }
    } else {
      logEvent('WARN', 'BOARD_STORE', `runTaskAgent: task "${resolvedTaskId}" sits in stage "${task.stageId}" without autoRun/persona. No move performed.`);
    }
  },

  setTaskVerdict: (taskId, verdict, note) => {
    set((state) => {
      const updatedTasks = state.tasks.map((t) =>
        t.id === taskId ? { ...t, verdict, verdictNote: note ?? t.verdictNote } : t
      );
      syncTasksBackend(updatedTasks);
      return { tasks: updatedTasks };
    });
  },
});

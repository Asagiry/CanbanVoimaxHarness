import { StateCreator } from 'zustand';
import {
  KanbanTask,
  TaskStatus,
  TaskHookStates,
  BackendTaskDTO,
  isBackendTaskDTO,
  getHookScript,
} from '../../types';
import { BoardState, TasksCrudSlice } from './types';
import { sortTasksByPosition, isDoneColumnDef } from '../board/boardHelpers';
import { CANONICAL_BOARD_COLUMNS } from '../board/boardConstants';
import { loadCachedHookStates, saveCachedHookStates } from '../board/hookStorage';
import { mapBackendTaskToKanban } from '../board/taskNormalizer';
import { syncTasksBackend, triggerRunAgent } from '../board/taskSync';
import { useDrawerStore } from '../drawerStore';
import { logEvent } from '../../lib/logger';

export const createTasksCrudSlice: StateCreator<
  BoardState,
  [],
  [],
  TasksCrudSlice
> = (set, get) => ({
  tasks: [],
  selectedTaskId: null,
  hoveredTaskId: null,
  searchQuery: '',
  isLoadingTasks: false,
  hookStates: loadCachedHookStates(),
  activeQuestions: {},

  setActiveQuestion: (payload) => {
    set((state) => ({
      activeQuestions: {
        ...state.activeQuestions,
        [payload.taskId]: payload,
      },
    }));
  },

  clearActiveQuestion: (taskId) => {
    set((state) => {
      if (!state.activeQuestions[taskId]) return state;
      const next = { ...state.activeQuestions };
      delete next[taskId];
      return { activeQuestions: next };
    });
  },

  setHookStart: ({ taskId, stageId, hookType, hookName, description }) => {
    set((state) => {
      const prev = state.hookStates[taskId] || {};
      const nextTaskHookState: TaskHookStates = {
        ...prev,
        [hookType]: {
          hookName,
          stageId,
          hookType,
          status: 'running',
          startedAt: Date.now(),
          stdout: '',
          stderr: '',
          description,
        },
      };
      const nextHookStates = {
        ...state.hookStates,
        [taskId]: nextTaskHookState,
      };
      saveCachedHookStates(nextHookStates);
      const nextTasks = state.tasks.map((t) => {
        if (t.id === taskId) {
          return { ...t, hookStates: nextTaskHookState };
        }
        return t;
      });
      return {
        hookStates: nextHookStates,
        tasks: nextTasks,
      };
    });
  },

  setHookFinish: ({ taskId, stageId, hookType, success, exitCode, stdout, stderr, error }) => {
    set((state) => {
      const prev = state.hookStates[taskId] || {};
      const existing = prev[hookType];
      const startedAt = existing?.startedAt ?? Date.now();
      const durationMs = Date.now() - startedAt;
      const nextTaskHookState: TaskHookStates = {
        ...prev,
        [hookType]: {
          hookName: existing?.hookName || hookType,
          stageId,
          hookType,
          status: success ? 'success' : 'failed',
          startedAt,
          durationMs,
          exitCode: exitCode ?? null,
          stdout: stdout || '',
          stderr: stderr || '',
          error,
          description: existing?.description,
        },
      };
      const nextHookStates = {
        ...state.hookStates,
        [taskId]: nextTaskHookState,
      };
      saveCachedHookStates(nextHookStates);
      const nextTasks = state.tasks.map((t) => {
        if (t.id === taskId) {
          return { ...t, hookStates: nextTaskHookState };
        }
        return t;
      });
      return {
        hookStates: nextHookStates,
        tasks: nextTasks,
      };
    });
  },

  clearHookState: (taskId, hookType) => {
    set((state) => {
      if (!state.hookStates[taskId]) return state;
      if (!hookType) {
        const next = { ...state.hookStates };
        delete next[taskId];
        saveCachedHookStates(next);
        const nextTasks = state.tasks.map((t) => {
          if (t.id === taskId) {
            const copy = { ...t };
            delete copy.hookStates;
            return copy;
          }
          return t;
        });
        return { hookStates: next, tasks: nextTasks };
      }
      const prev = state.hookStates[taskId];
      const nextTaskHooks = { ...prev };
      delete nextTaskHooks[hookType];
      const nextHookStates = {
        ...state.hookStates,
        [taskId]: nextTaskHooks,
      };
      saveCachedHookStates(nextHookStates);
      const nextTasks = state.tasks.map((t) => {
        if (t.id === taskId) {
          return { ...t, hookStates: nextTaskHooks };
        }
        return t;
      });
      return {
        hookStates: nextHookStates,
        tasks: nextTasks,
      };
    });
  },

  setTasks: (tasks) => {
    set({ tasks });
  },

  setSelectedTaskId: (id) => set({ selectedTaskId: id }),

  setHoveredTaskId: (id) => set({ hoveredTaskId: id }),

  setSearchQuery: (query) => set({ searchQuery: query }),

  loadTasks: async () => {
    set({ isLoadingTasks: true });
    logEvent('DEBUG', 'BOARD_STORE', 'loadTasks: invoking list_tasks from backend');
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const rawTasks = await invoke<BackendTaskDTO[]>('list_tasks');
      logEvent('INFO', 'BOARD_STORE', `loadTasks: received ${Array.isArray(rawTasks) ? rawTasks.length : 0} tasks from backend`);
      if (Array.isArray(rawTasks)) {
        const prevTasks = get().tasks;
        const mapped: KanbanTask[] = [];
        for (const raw of rawTasks) {
          if (!isBackendTaskDTO(raw)) continue;
          const prev = prevTasks.find((p) => p.id === raw.id);
          const rawCol = raw.stageId || raw.stage_id || raw.columnId;
          if (prev && (!rawCol || prev.columnId === rawCol)) {
            if (prev.stageEnteredAt && !raw.stageEnteredAt && !raw.stage_entered_at) {
              raw.stageEnteredAt = prev.stageEnteredAt;
            }
            if (prev.stageDurations && !raw.stageDurations) {
              raw.stageDurations = prev.stageDurations;
            }
          }
          const task = mapBackendTaskToKanban(raw);
          if (task) {
            if (prev?.todos && (task.todos === undefined || task.todos.length === 0)) {
              task.todos = prev.todos;
            }
            mapped.push(task);
          }
        }
        mapped.sort(sortTasksByPosition);
        const prevHookStates = get().hookStates;
        const nextHookStates: Record<string, TaskHookStates> = { ...prevHookStates };
        for (const t of mapped) {
          if (t.hookStates) {
            nextHookStates[t.id] = {
              ...nextHookStates[t.id],
              ...t.hookStates,
            };
          }
        }
        saveCachedHookStates(nextHookStates);
        set({ tasks: mapped, hookStates: nextHookStates, isTauriConnected: true, isLoadingTasks: false });
        return;
      }
    } catch (err) {
      console.info('Tauri list_tasks not available or returned empty:', err);
    }

    set({ isLoadingTasks: false });
  },

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
      console.warn(`[moveTask] Invalid empty targetColumnId for task "${resolvedTaskId}"`);
      return;
    }

    if (!options?.force && task.columnId === targetStageId) {
      return;
    }

    const targetColDef = columns.find((c) => c.id === targetStageId);
    console.info(`[moveTask] Moving task "${resolvedTaskId}" from "${task.columnId}" to "${targetStageId}" (board: ${activeBoardId})`);
    logEvent(
      'INFO',
      'BOARD_STORE',
      `moveTask: starting move for "${resolvedTaskId}" from "${task.columnId}" to "${targetStageId}"`,
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
        const sourceColId = options?.sourceColumnId ?? prev.columnId;
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
            [prev.columnId]: (prevStageDurations[prev.columnId] || 0) + currentDuration,
          },
          totalElapsed: prev.totalElapsed,
          isTimerRunning: mapped.status === 'running',
          returnsCount: isRework ? prev.returnsCount + 1 : prev.returnsCount,
        };
        const isTerminalMove = isDoneColumnDef(targetColDef) || targetStageId === 'ready';
        if (isTerminalMove) {
          merged.waitingForSubtaskId = undefined;
          merged.status = 'done';
          merged.completedAt = merged.completedAt || Date.now();
          if (merged.createdAt) {
            merged.totalElapsed = Math.max(0, Math.floor((merged.completedAt - merged.createdAt) / 1000));
          }
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
          JSON.stringify({ newStage: merged.columnId, newStatus: merged.status, boardId: merged.boardId })
        );

        if (merged.boardId && merged.boardId !== get().activeBoardId) {
          const boardExists = get().boards.some((b) => b.id === merged.boardId);
          if (boardExists) {
            await get().setActiveBoard(merged.boardId);
          }
        }

        const colDef = targetColDef ?? columns.find((c) => c.id === merged.columnId);
        if (colDef?.autoRun === true || (colDef?.autoRun !== false && Boolean(colDef?.persona))) {
          await triggerRunAgent(resolvedTaskId);
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
          mapped.completedAt = mapped.completedAt || Date.now();
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
          if (prevTask && prevTask.columnId !== targetStageId) {
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

  updateTask: (taskId, partial) => {
    set((state) => {
      const updatedTasks = state.tasks.map((t) => (t.id === taskId ? { ...t, ...partial } : t));
      return { tasks: updatedTasks };
    });
    void (async () => {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('update_task', {
          taskId,
          model: partial.model !== undefined ? partial.model : null,
          title: partial.title ?? null,
          description: partial.description ?? null,
          status: partial.status ?? null,
          verdict: partial.verdict ?? null,
        });
      } catch {
        const tasks = get().tasks;
        await syncTasksBackend(tasks);
      }
    })();
  },

  updateTaskStatus: (taskId, status) => {
    set((state) => {
      const updatedTasks = state.tasks.map((t) => {
        if (t.id !== taskId) return t;
        if (status === 'idle' && (t.status === 'waiting_subtask' || t.status === 'done')) {
          return t;
        }
        const isTimerRunning = status === 'running' ? t.isTimerRunning : false;
        return { ...t, status, isTimerRunning };
      });
      return { tasks: updatedTasks };
    });
  },

  upsertBackendTask: (rawTask: BackendTaskDTO | unknown) => {
    if (!rawTask || typeof rawTask !== 'object') return;
    const rawDto = rawTask as BackendTaskDTO;
    const prev = get().tasks.find((p) => p.id === rawDto.id);
    const rawCol = rawDto.stageId || rawDto.stage_id || rawDto.columnId;
    if (prev && (!rawCol || prev.columnId === rawCol)) {
      if (prev.stageEnteredAt && !rawDto.stageEnteredAt && !rawDto.stage_entered_at) {
        rawDto.stageEnteredAt = prev.stageEnteredAt;
      }
      if (prev.stageDurations && !rawDto.stageDurations) {
        rawDto.stageDurations = prev.stageDurations;
      }
    }
    const mapped = mapBackendTaskToKanban(rawDto);
    if (!mapped) return;
    const { columns } = get();
    set((state) => {
      let nextHookStates = state.hookStates;
      if (mapped.hookStates) {
        nextHookStates = {
          ...state.hookStates,
          [mapped.id]: {
            ...state.hookStates[mapped.id],
            ...mapped.hookStates,
          },
        };
        saveCachedHookStates(nextHookStates);
      }
      const exists = state.tasks.some((t) => t.id === mapped.id);
      const updatedTasks = exists
        ? state.tasks.map((t) => {
            if (t.id === mapped.id) {
              const preservedTodos = mapped.todos !== undefined ? mapped.todos : t.todos;
              const mergedHookStates = mapped.hookStates
                ? { ...t.hookStates, ...mapped.hookStates }
                : t.hookStates;
              const preservedStageEnteredAt = mapped.stageEnteredAt || t.stageEnteredAt || t.createdAt;
              const preservedStageDurations = mapped.stageDurations && Object.keys(mapped.stageDurations).length > 0
                ? mapped.stageDurations
                : t.stageDurations;
              return {
                ...t,
                ...mapped,
                stageEnteredAt: preservedStageEnteredAt,
                stageDurations: preservedStageDurations,
                todos: preservedTodos,
                hookStates: mergedHookStates,
              };
            }
            return t;
          })
        : [...state.tasks, mapped];

      const finalTasks = mapped.parentId
        ? updatedTasks.map((t) => {
            if (t.id === mapped.parentId) {
              const mappedCol = columns.find((c) => c.id === mapped.columnId);
              const isSubDone = mapped.status === 'done' || mapped.columnId === 'ready' || mappedCol?.terminal === true;
              return {
                ...t,
                waitingForSubtaskId: isSubDone ? undefined : mapped.id,
                status: isSubDone
                  ? (t.status === 'waiting_subtask' ? ('idle' as TaskStatus) : t.status)
                  : ('waiting_subtask' as TaskStatus),
              };
            }
            return t;
          })
        : updatedTasks.map((t) => {
            const tCol = columns.find((c) => c.id === t.columnId);
            if (t.id === mapped.id && (t.columnId === 'ready' || t.status === 'done' || tCol?.terminal === true)) {
              return {
                ...t,
                waitingForSubtaskId: undefined,
                status: t.status === 'waiting_subtask' ? ('done' as TaskStatus) : t.status,
              };
            }
            return t;
          });

      return { tasks: finalTasks.sort(sortTasksByPosition), hookStates: nextHookStates };
    });
  },

  setBackendTasks: (rawTasks: (BackendTaskDTO | unknown)[]) => {
    if (!Array.isArray(rawTasks)) return;
    const prevTasks = get().tasks;
    const { columns } = get();
    const mapped: KanbanTask[] = [];
    for (const raw of rawTasks) {
      if (!isBackendTaskDTO(raw)) continue;
      const prev = prevTasks.find((p) => p.id === raw.id);
      const rawCol = raw.stageId || raw.stage_id || raw.columnId;
      if (prev && (!rawCol || prev.columnId === rawCol)) {
        if (prev.stageEnteredAt && !raw.stageEnteredAt && !raw.stage_entered_at) {
          raw.stageEnteredAt = prev.stageEnteredAt;
        }
        if (prev.stageDurations && !raw.stageDurations) {
          raw.stageDurations = prev.stageDurations;
        }
      }
      const task = mapBackendTaskToKanban(raw);
      if (task) {
        if (prev?.todos && (task.todos === undefined || task.todos.length === 0)) {
          task.todos = prev.todos;
        }
        mapped.push(task);
      }
    }

    const cleanedTasks = mapped.map((t) => {
      const colDef = columns.find((c) => c.id === t.columnId);
      const isTerminal = t.columnId === 'ready' || t.status === 'done' || colDef?.terminal === true;
      if (isTerminal) {
        if (t.waitingForSubtaskId || t.status === 'waiting_subtask') {
          return { ...t, waitingForSubtaskId: undefined, status: 'done' as TaskStatus };
        }
        return t;
      }
      if (t.waitingForSubtaskId) {
        const sub = mapped.find((s) => s.id === t.waitingForSubtaskId);
        const subColDef = sub ? columns.find((c) => c.id === sub.columnId) : undefined;
        if (!sub || sub.status === 'done' || sub.columnId === 'ready' || subColDef?.terminal === true) {
          return {
            ...t,
            waitingForSubtaskId: undefined,
            status: t.status === 'waiting_subtask' ? ('idle' as TaskStatus) : t.status,
          };
        }
      }
      return t;
    });

    const prevHookStates = get().hookStates;
    const nextHookStates: Record<string, TaskHookStates> = { ...prevHookStates };
    for (const t of mapped) {
      if (t.hookStates) {
        nextHookStates[t.id] = {
          ...nextHookStates[t.id],
          ...t.hookStates,
        };
      }
    }
    saveCachedHookStates(nextHookStates);

    set({ tasks: cleanedTasks.sort(sortTasksByPosition), hookStates: nextHookStates });
  },

  addTask: async (input) => {
    const state = get();
    const intakeCol = state.columns.find((c) => c.isIntake);
    const targetColumnId =
      input.stageId ||
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
      JSON.stringify({ targetColumnId, boardId, model: input.model })
    );

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const backendTask = await invoke<BackendTaskDTO>('create_task', {
        title,
        description,
        stageId: targetColumnId,
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
          JSON.stringify({ columnId: confirmed.columnId, boardId: confirmed.boardId, worktree: confirmed.worktreePath })
        );

        const colDef = get().columns.find((c) => c.id === confirmed.columnId);
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

  createSubtask: async (parentId: string, targetColumnId?: string) => {
    const { tasks, columns } = get();
    const parent = tasks.find((t) => t.id === parentId);
    if (!parent) return null;

    const parentColIndex = columns.findIndex((c) => c.id === parent.columnId);
    const targetCol = targetColumnId
      ? columns.find((c) => c.id === targetColumnId)
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
              status: 'waiting_subtask' as TaskStatus,
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
          JSON.stringify({ columnId: mapped.columnId, status: mapped.status })
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

    // Invariant § 2.25: Unconditional drawer auto-close and memory eviction BEFORE backend IPC
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
    }

    logEvent('INFO', 'BOARD_STORE', `deleteTask: requesting backend delete for "${resolvedId}"`, JSON.stringify(ids));

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('delete_task', { taskId: resolvedId });
    } catch (err) {
      const errMsg = typeof err === 'string' ? err : err instanceof Error ? err.message : JSON.stringify(err);
      console.error(`[deleteTask] Backend delete_task failed for "${resolvedId}":`, err);
      logEvent('ERROR', 'BOARD_STORE', `deleteTask: backend delete_task failed for "${resolvedId}"`, errMsg);
      alert(`Не удалось удалить задачу:\n${errMsg}`);
      return;
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
    });
  },

  runTaskAgent: async (taskId, prompt) => {
    const { tasks, boards, activeBoardId, columns: activeColumns, moveTask } = get();
    const task = tasks.find(
      (t) => t.id === taskId || t.id.trim().toLowerCase() === taskId.trim().toLowerCase()
    );
    if (!task) {
      console.warn(`[runTaskAgent] Task "${taskId}" not found in local store`);
      logEvent('WARN', 'BOARD_STORE', `runTaskAgent: Task "${taskId}" not found in local store`);
      return;
    }
    const resolvedTaskId = task.id;
    logEvent(
      'INFO',
      'BOARD_STORE',
      `runTaskAgent: requested for "${resolvedTaskId}"`,
      JSON.stringify({ columnId: task.columnId, boardId: task.boardId, status: task.status })
    );

    await syncTasksBackend(get().tasks);

    const effectiveBoardId = task.boardId || activeBoardId;
    const taskBoard = boards.find((b) => b.id === effectiveBoardId);
    const targetColumns = (taskBoard?.columns && taskBoard.columns.length > 0)
      ? taskBoard.columns
      : (activeColumns && activeColumns.length > 0 ? activeColumns : CANONICAL_BOARD_COLUMNS);
    const taskCol = targetColumns.find(
      (c) => c.id === task.columnId || (task.columnId && c.id.toLowerCase() === task.columnId.toLowerCase())
    );

    if (taskCol?.terminal || taskCol?.id === 'ready' || taskCol?.id === 'done') {
      logEvent('INFO', 'BOARD_STORE', `runTaskAgent: task "${resolvedTaskId}" is in terminal column "${task.columnId}", skipping`);
      return;
    } else if (taskCol?.persona || taskCol?.autoRun) {
      logEvent('INFO', 'BOARD_STORE', `runTaskAgent: task "${resolvedTaskId}" is in agent column "${taskCol.id}", starting agent`);
      const updatedTasks = get().tasks.map((t) =>
        t.id === resolvedTaskId || t.id.trim().toLowerCase() === resolvedTaskId.trim().toLowerCase()
          ? { ...t, status: 'running' as TaskStatus, isTimerRunning: true, updatedAt: Date.now() }
          : t
      );
      set({ tasks: updatedTasks });
      await syncTasksBackend(updatedTasks);
      await triggerRunAgent(resolvedTaskId, prompt);
    } else if (taskCol?.isIntake || taskCol?.id === 'backlog') {
      const firstAgentCol = targetColumns.find((c) => c.persona || c.autoRun);
      if (firstAgentCol) {
        logEvent('INFO', 'BOARD_STORE', `runTaskAgent: task "${resolvedTaskId}" in intake column "${task.columnId}", moving to "${firstAgentCol.id}"`);
        await moveTask(resolvedTaskId, firstAgentCol.id);
      } else {
        logEvent('WARN', 'BOARD_STORE', `runTaskAgent: no agent column found for task "${resolvedTaskId}"`);
      }
    } else {
      logEvent('WARN', 'BOARD_STORE', `runTaskAgent: task "${resolvedTaskId}" sits in column "${task.columnId}" without autoRun/persona. No move performed.`);
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

  promoteTaskToNextStage: (taskId) => {
    const { tasks, columns, moveTask } = get();
    const task = tasks.find((t) => t.id === taskId);
    if (!task) return;

    if (task.customFlow && task.customFlow.length > 0) {
      const currentIndexInFlow = task.customFlow.indexOf(task.columnId);
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

    const currentIndex = columns.findIndex((c) => c.id === task.columnId);
    if (currentIndex >= 0 && currentIndex < columns.length - 1) {
      const nextCol = columns[currentIndex + 1];
      moveTask(taskId, nextCol.id);
    }
  },
});

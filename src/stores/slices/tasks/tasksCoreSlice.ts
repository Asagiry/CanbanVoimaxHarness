import { StateCreator } from 'zustand';
import { KanbanTask, TaskHookStates, TaskStatus, BackendTaskDTO, isBackendTaskDTO } from '../../../types';
import { BoardState, TasksCrudCoreSlice } from '../types';
import { sortTasksByPosition } from '../../board/boardHelpers';
import { mapBackendTaskToKanban, isTaskForeign } from '../../board/taskNormalizer';
import { saveCachedHookStates } from '../../board/hookStorage';
import { useProjectStore } from '../../projectStore';
import { useAgentSessionStore } from '../../agentSessionStore';
import { logEvent } from '../../../lib/logger';

export type TasksCoreSlice = Pick<
  TasksCrudCoreSlice,
  | 'tasks'
  | 'selectedTaskId'
  | 'hoveredTaskId'
  | 'searchQuery'
  | 'isLoadingTasks'
  | 'setTasks'
  | 'setSelectedTaskId'
  | 'setHoveredTaskId'
  | 'setSearchQuery'
  | 'loadTasks'
  | 'upsertBackendTask'
  | 'setBackendTasks'
>;

export const createTasksCoreSlice: StateCreator<BoardState, [], [], TasksCoreSlice> = (set, get) => ({
  tasks: [],
  selectedTaskId: null,
  hoveredTaskId: null,
  searchQuery: '',
  isLoadingTasks: false,
  setTasks: (tasks) => set({ tasks }),
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
        let activeRoot: string | undefined;
        try {
          const { useProjectStore } = await import('../../projectStore');
          activeRoot = useProjectStore.getState().activeProject?.rootPath;
        } catch {
          // ignore in standalone test environments
        }

        const mapped: KanbanTask[] = [];
        for (const raw of rawTasks) {
          if (!isBackendTaskDTO(raw)) continue;
          if (activeRoot && isTaskForeign(raw, activeRoot)) {
            continue;
          }
          const task = mapBackendTaskToKanban(raw);
          if (task) {
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
        const liveIds = new Set(mapped.filter((t) => t.status !== 'done' && t.stageId !== 'ready' && t.stageId !== 'done').map((t) => t.id));
        const prevQuestions = get().activeQuestions;
        const prunedQuestions: Record<string, (typeof prevQuestions)[string]> = {};
        for (const [k, v] of Object.entries(prevQuestions)) {
          if (liveIds.has(k)) prunedQuestions[k] = v;
        }
        set({ tasks: mapped, hookStates: nextHookStates, activeQuestions: prunedQuestions, isTauriConnected: true, isLoadingTasks: false });
        return;
      }
    } catch (err) {
      console.info('Tauri list_tasks not available or returned empty:', err);
    }

    set({ tasks: [], isLoadingTasks: false });
  },

  upsertBackendTask: (rawTask: BackendTaskDTO | unknown) => {
    if (!rawTask || typeof rawTask !== 'object') return;
    const rawDto = rawTask as BackendTaskDTO;
    let activeRoot: string | undefined;
    try {
      activeRoot = useProjectStore.getState().activeProject?.rootPath;
    } catch {
      // ignore
    }
    if (activeRoot && isTaskForeign(rawDto, activeRoot)) {
      return;
    }
    const prev = get().tasks.find((p) => p.id === rawDto.id);
    const rawStage = rawDto.stageId || rawDto.stage_id || rawDto.columnId;
    if (prev && (!rawStage || prev.stageId === rawStage)) {
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
      const preserveRunning = (incoming: TaskHookStates | undefined): TaskHookStates | undefined => {
        const live = state.hookStates[mapped.id];
        if (!live) return incoming;
        const next: TaskHookStates = { ...incoming };
        if (live.on_enter?.status === 'running') next.on_enter = live.on_enter;
        if (live.on_leave?.status === 'running') next.on_leave = live.on_leave;
        if (
          (live.on_enter?.status === 'success' || live.on_enter?.status === 'failed') &&
          live.on_enter.stageId === mapped.stageId &&
          !next.on_enter
        ) {
          next.on_enter = live.on_enter;
        }
        if (
          (live.on_leave?.status === 'success' || live.on_leave?.status === 'failed') &&
          live.on_leave.stageId === mapped.stageId &&
          !next.on_leave
        ) {
          next.on_leave = live.on_leave;
        }
        return next;
      };
      let nextHookStates = state.hookStates;
      if (mapped.hookStates) {
        nextHookStates = {
          ...state.hookStates,
          [mapped.id]: preserveRunning({
            ...state.hookStates[mapped.id],
            ...mapped.hookStates,
          }) as TaskHookStates,
        };
        saveCachedHookStates(nextHookStates);
      }
      const exists = state.tasks.some((t) => t.id === mapped.id);
      const updatedTasks = exists
        ? state.tasks.map((t) => {
            if (t.id === mapped.id) {
              const preservedTodos = mapped.todos !== undefined ? mapped.todos : t.todos;
              const mergedHookStates = mapped.hookStates
                ? preserveRunning({ ...t.hookStates, ...mapped.hookStates })
                : t.hookStates;
              const preservedStageEnteredAt = mapped.stageEnteredAt || t.stageEnteredAt || t.createdAt;
              const preservedStageDurations = mapped.stageDurations && Object.keys(mapped.stageDurations).length > 0
                ? mapped.stageDurations
                : t.stageDurations;
              const sessionRunning = (() => { try { return useAgentSessionStore.getState().isSessionRunning(mapped.id); } catch { return false; } })();
              const preservedStatus = ((t.status === 'running' && mapped.status === 'idle') || (sessionRunning && mapped.status === 'idle')) ? 'running' : mapped.status;
              return {
                ...t,
                ...mapped,
                status: preservedStatus,
                isTimerRunning: preservedStatus === 'running' ? true : mapped.isTimerRunning,
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
              const mappedCol = columns.find((c) => c.id === mapped.stageId);
              const isSubDone = mapped.status === 'done' || mapped.stageId === 'ready' || mappedCol?.terminal === true;
              const isActiveSub = mapped.status === 'running' || mapped.status === 'idle';
              const parentStatus = t.status === 'waiting_subtask' ? ('idle' as TaskStatus) : t.status;
              if (isSubDone || !isActiveSub) {
                return { ...t, waitingForSubtaskId: undefined, status: parentStatus };
              }
              return { ...t, waitingForSubtaskId: mapped.id, status: parentStatus };
            }
            return t;
          })
        : updatedTasks.map((t) => {
            const tCol = columns.find((c) => c.id === t.stageId);
            if (t.id === mapped.id && (t.stageId === 'ready' || t.stageId === 'done' || tCol?.terminal === true)) {
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
    let activeRoot: string | undefined;
    try {
      activeRoot = useProjectStore.getState().activeProject?.rootPath;
    } catch {
      // ignore
    }
    const prevTasks = get().tasks;
    const { columns } = get();
    const mapped: KanbanTask[] = [];
    for (const raw of rawTasks) {
      if (!isBackendTaskDTO(raw)) continue;
      if (activeRoot && isTaskForeign(raw, activeRoot)) continue;
      const prev = prevTasks.find((p) => p.id === raw.id);
      const rawStage = raw.stageId || raw.stage_id || raw.columnId;
      if (prev && (!rawStage || prev.stageId === rawStage)) {
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
        const sessionRunning = (() => { try { return useAgentSessionStore.getState().isSessionRunning(task.id); } catch { return false; } })();
        if ((prev?.status === 'running' || sessionRunning) && task.status === 'idle') {
          task.status = 'running';
          task.isTimerRunning = true;
        }
        mapped.push(task);
      }
    }

    const cleanedTasks = mapped.map((t) => {
      const colDef = columns.find((c) => c.id === t.stageId);
      const isTerminal = t.stageId === 'ready' || t.stageId === 'done' || colDef?.terminal === true;
      if (isTerminal) {
        if (t.waitingForSubtaskId || t.status === 'waiting_subtask') {
          return { ...t, waitingForSubtaskId: undefined, status: 'done' as TaskStatus };
        }
        return t;
      }
      let status = t.status;
      if (status === 'waiting_subtask') {
        status = 'idle';
      }
      if (t.waitingForSubtaskId) {
        const sub = mapped.find((s) => s.id === t.waitingForSubtaskId);
        const subColDef = sub ? columns.find((c) => c.id === sub.stageId) : undefined;
        if (!sub || sub.status === 'done' || sub.stageId === 'ready' || subColDef?.terminal === true) {
          return {
            ...t,
            waitingForSubtaskId: undefined,
            status: 'idle' as TaskStatus,
          };
        }
      }
      return status !== t.status ? { ...t, status } : t;
    });

    const prevHookStates = get().hookStates;
    const nextHookStates: Record<string, TaskHookStates> = { ...prevHookStates };
    for (const t of mapped) {
      if (t.hookStates) {
        const merged: TaskHookStates = {
          ...nextHookStates[t.id],
          ...t.hookStates,
        };
        if (nextHookStates[t.id]?.on_enter?.status === 'running') merged.on_enter = nextHookStates[t.id].on_enter;
        if (nextHookStates[t.id]?.on_leave?.status === 'running') merged.on_leave = nextHookStates[t.id].on_leave;
        if (
          (nextHookStates[t.id]?.on_enter?.status === 'success' ||
            nextHookStates[t.id]?.on_enter?.status === 'failed') &&
          nextHookStates[t.id]?.on_enter?.stageId === t.stageId &&
          !merged.on_enter
        ) {
          merged.on_enter = nextHookStates[t.id].on_enter;
        }
        if (
          (nextHookStates[t.id]?.on_leave?.status === 'success' ||
            nextHookStates[t.id]?.on_leave?.status === 'failed') &&
          nextHookStates[t.id]?.on_leave?.stageId === t.stageId &&
          !merged.on_leave
        ) {
          merged.on_leave = nextHookStates[t.id].on_leave;
        }
        nextHookStates[t.id] = merged;
      }
    }
    saveCachedHookStates(nextHookStates);

    const liveIds = new Set(cleanedTasks.filter((t) => t.status !== 'done' && t.stageId !== 'ready' && t.stageId !== 'done').map((t) => t.id));
    const prevQuestions = get().activeQuestions;
    const prunedQuestions: Record<string, (typeof prevQuestions)[string]> = {};
    for (const [k, v] of Object.entries(prevQuestions)) {
      if (liveIds.has(k)) prunedQuestions[k] = v;
    }

    set({ tasks: cleanedTasks.sort(sortTasksByPosition), hookStates: nextHookStates, activeQuestions: prunedQuestions });
  },
});

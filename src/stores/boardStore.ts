import { create } from 'zustand';
import { KanbanTask, PipelineColumn, TaskStatus, TaskVerdict, TaskPriority } from '../types';
import { INITIAL_COLUMNS } from './mockData';
import { useProjectStore } from './projectStore';

interface BoardState {
  columns: PipelineColumn[];
  tasks: KanbanTask[];
  selectedTaskId: string | null;
  searchQuery: string;
  isTauriConnected: boolean;
  isLoadingTasks: boolean;

  // Actions
  setColumns: (columns: PipelineColumn[]) => void;
  setTasks: (tasks: KanbanTask[]) => void;
  setSelectedTaskId: (id: string | null) => void;
  setSearchQuery: (query: string) => void;

  loadColumns: () => Promise<void>;
  loadTasks: () => Promise<void>;
  moveTask: (taskId: string, targetColumnId: string) => Promise<void>;
  updateTask: (taskId: string, partial: Partial<KanbanTask>) => void;
  addTask: (task: KanbanTask) => Promise<void>;
  deleteTask: (taskId: string) => void;
  setTaskVerdict: (taskId: string, verdict: TaskVerdict, note?: string) => void;
  toggleTimer: (taskId: string) => void;
  tickTimers: () => void;
  promoteTaskToNextStage: (taskId: string) => void;
  runTaskAgent: (taskId: string) => Promise<void>;
}

export function getTasksStorageKey(): string {
  try {
    const activeProject = useProjectStore.getState().activeProject;
    const storageKey = `cvh_tasks_${activeProject?.id || 'default'}`;
    return storageKey;
  } catch {
    return 'cvh_tasks_default';
  }
}

function persistTasksToLocalStorage(tasks: KanbanTask[]): void {
  try {
    if (typeof window !== 'undefined' && window.localStorage) {
      window.localStorage.setItem(getTasksStorageKey(), JSON.stringify(tasks));
    }
  } catch (err) {
    console.warn('Failed to write tasks backup to localStorage:', err);
  }
}

function loadTasksFromLocalStorage(): KanbanTask[] {
  try {
    if (typeof window !== 'undefined' && window.localStorage) {
      const raw = window.localStorage.getItem(getTasksStorageKey());
      if (raw) {
        const parsed = JSON.parse(raw);
        if (Array.isArray(parsed) && parsed.length > 0) {
          return parsed;
        }
      }
    }
  } catch (err) {
    console.warn('Failed to read tasks backup from localStorage:', err);
  }
  return [];
}

export async function syncTasksBackend(tasks: KanbanTask[]): Promise<void> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    // NOTE: send ONLY canonical camelCase keys. Sending both stageId+columnId
    // makes Rust serde fail with "duplicate field `stageId`" and save_tasks
    // silently never persists (this broke run_agent with "Task not found").
    const backendPayload = tasks.map((t) => ({
      id: t.id,
      title: t.title,
      description: t.description || '',
      stageId: t.columnId,
      worktreePath: t.worktreePath,
      branch: t.branch,
      verdict: t.verdict === 'NONE' ? null : t.verdict,
      customFlow: t.customFlow,
      status: t.status,
      model: t.model,
      createdAt: Date.now(),
      updatedAt: Date.now(),
    }));
    await invoke('save_tasks', { tasks: backendPayload });
  } catch (err) {
    console.info('Backend save_tasks not available or in web mode:', err);
  }
}

async function triggerRunAgent(taskId: string, prompt?: string): Promise<void> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('run_agent', { taskId, prompt: prompt || null });
  } catch (err) {
    console.info('run_agent invoke failed or in web mode:', err);
  }
}

function mapBackendTaskToKanban(t: any): KanbanTask {
  // Fully generic: resolve everything through the current board columns.
  // (defined outside create(), so columns come from getState()).
  const columns = useBoardStore.getState().columns;
  const rawId: string | undefined = t.stageId || t.stage_id;
  const knownCol = columns.find((c) => c.id === rawId);
  const columnId = knownCol?.id ?? columns[0]?.id ?? rawId ?? 'backlog';
  const col = columns.find((c) => c.id === columnId);

  // terminal → done, persona column → running, otherwise idle
  const status: TaskStatus = col?.terminal
    ? 'done'
    : col?.persona
    ? 'running'
    : 'idle';

  return {
    id: t.id,
    title: t.title,
    description: t.description || undefined,
    priority: (t.priority as TaskPriority) || 'high',
    columnId,
    persona: col?.persona ?? 'developer',
    branch: t.branch || `feat/${t.id.toLowerCase()}`,
    worktreePath: t.worktreePath || t.worktree_path || `worktrees/${t.id.toLowerCase()}`,
    status,
    activeStageDuration: t.activeStageDuration || 0,
    stageDurations: t.stageDurations || {},
    totalElapsed: t.totalElapsed || 0,
    isTimerRunning: status === 'running',
    returnsCount: t.returnsCount || 0,
    fastTrack: t.fastTrack,
    customFlow: t.customFlow || t.custom_flow,
    verdict: (t.verdict as TaskVerdict) || 'NONE',
    verdictNote: t.verdictNote,
    model: t.model || undefined,
  };
}

export const useBoardStore = create<BoardState>((set, get) => ({
  columns: INITIAL_COLUMNS,
  tasks: [], // Initialized with empty array - NO mock tasks!
  selectedTaskId: null,
  searchQuery: '',
  isTauriConnected: false,
  isLoadingTasks: false,

  setColumns: (columns) => set({ columns }),
  setTasks: (tasks) => {
    persistTasksToLocalStorage(tasks);
    set({ tasks });
  },
  setSelectedTaskId: (id) => set({ selectedTaskId: id }),
  setSearchQuery: (query) => set({ searchQuery: query }),

  loadColumns: async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      // Backend get_pipeline returns PipelineConfig { id, title, columns }
      const pipeline = await invoke<{ id: string; title: string; columns: any[] }>('get_pipeline');
      const rawCols = pipeline?.columns;
      if (Array.isArray(rawCols) && rawCols.length > 0) {
        const mapped: PipelineColumn[] = rawCols.map((c: any) => ({
          id: c.id,
          title: c.title,
          color: c.color ?? '#64748b',
          icon: c.icon ?? 'Inbox',
          persona: c.persona ?? null,
          autoRun: c.autoRun,
          terminal: c.terminal,
          contextWindow: c.contextWindow ?? c.context_window ?? 128000,
          modelOverride: c.modelOverride ?? c.model_override ?? null,
          temperature: c.temperature,
          reasoningEffort: c.reasoningEffort ?? c.reasoning_effort,
          gatekeeperSensor: c.gatekeeperSensor ?? c.gatekeeper_sensor ?? null,
          wipLimit: c.wipLimit,
          hooks: c.hooks ?? {},
          routes: c.routes ?? undefined,
        }));
        set({ columns: mapped });
      }
    } catch (err) {
      console.info('Tauri get_pipeline not available or in web mode:', err);
    }
  },

  loadTasks: async () => {
    set({ isLoadingTasks: true });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const rawTasks = await invoke<any[]>('list_tasks');
      if (Array.isArray(rawTasks)) {
        if (rawTasks.length > 0) {
          const mapped = rawTasks.map(mapBackendTaskToKanban);
          set({ tasks: mapped, isTauriConnected: true, isLoadingTasks: false });
          persistTasksToLocalStorage(mapped);
          return;
        } else {
          // Backend has empty tasks array on disk: check per-project localStorage backup
          const backupTasks = loadTasksFromLocalStorage();
          if (backupTasks.length > 0) {
            set({ tasks: backupTasks, isTauriConnected: true, isLoadingTasks: false });
            await syncTasksBackend(backupTasks);
            return;
          }
          set({ tasks: [], isTauriConnected: true, isLoadingTasks: false });
          return;
        }
      }
    } catch (err) {
      console.info('Tauri list_tasks not available or returned empty:', err);
    }

    // Fallback: restore from localStorage backup
    const backupTasks = loadTasksFromLocalStorage();
    if (backupTasks.length > 0) {
      set({ tasks: backupTasks, isLoadingTasks: false });
      await syncTasksBackend(backupTasks);
      return;
    }

    set({ tasks: [], isLoadingTasks: false });
  },

  moveTask: async (taskId, targetColumnId) => {
    const { tasks, columns } = get();
    const task = tasks.find((t) => t.id === taskId);
    if (!task || task.columnId === targetColumnId) return;

    // Generic rework detection: board order only, no hardcoded stages
    const sourceColIndex = columns.findIndex((c) => c.id === task.columnId);
    const targetColIndex = columns.findIndex((c) => c.id === targetColumnId);

    // Check if task is being returned / reworked backwards
    const isRework =
      sourceColIndex >= 0 && targetColIndex >= 0 && targetColIndex < sourceColIndex;
    const newReturnsCount = isRework ? task.returnsCount + 1 : task.returnsCount;

    // Freeze current stage duration
    const currentStage = task.columnId;
    const currentDuration = task.activeStageDuration;
    const updatedStageDurations = {
      ...task.stageDurations,
      [currentStage]: (task.stageDurations[currentStage] || 0) + currentDuration,
    };

    // NOTE: fastTrack (noReview/noRetro) is intentionally ignored —
    // the task always lands in the requested target column.

    const targetColDef = columns.find((c) => c.id === targetColumnId);
    // terminal → done, persona column → running, otherwise idle
    const newStatus: TaskStatus = targetColDef?.terminal
      ? 'done'
      : targetColDef?.persona
      ? 'running'
      : 'idle';

    const updatedTasks = tasks.map((t) => {
      if (t.id !== taskId) return t;
      return {
        ...t,
        columnId: targetColumnId,
        persona: targetColDef?.persona || t.persona,
        status: newStatus,
        activeStageDuration: 0,
        stageDurations: updatedStageDurations,
        isTimerRunning: newStatus === 'running',
        returnsCount: newReturnsCount,
      };
    });

    set({ tasks: updatedTasks });
    persistTasksToLocalStorage(updatedTasks);
    await syncTasksBackend(updatedTasks);

    // Column = agent: autoRun triggers run_agent on entry (default: !!persona)
    if (targetColDef?.autoRun ?? !!targetColDef?.persona) {
      const targetTask = updatedTasks.find((t) => t.id === taskId);
      await triggerRunAgent(taskId, targetTask?.description);
    }
  },

  updateTask: (taskId, partial) => {
    set((state) => {
      const updatedTasks = state.tasks.map((t) => (t.id === taskId ? { ...t, ...partial } : t));
      persistTasksToLocalStorage(updatedTasks);
      syncTasksBackend(updatedTasks);
      return { tasks: updatedTasks };
    });
  },

  addTask: async (task) => {
    const updatedTasks = [task, ...get().tasks];
    set({ tasks: updatedTasks });
    persistTasksToLocalStorage(updatedTasks);
    await syncTasksBackend(updatedTasks);

    const targetColDef = get().columns.find((c) => c.id === task.columnId);
    if (targetColDef?.autoRun ?? !!targetColDef?.persona) {
      await triggerRunAgent(task.id, task.description);
    }
  },

  deleteTask: (taskId) => {
    const updatedTasks = get().tasks.filter((t) => t.id !== taskId);
    set({
      tasks: updatedTasks,
      selectedTaskId: get().selectedTaskId === taskId ? null : get().selectedTaskId,
    });
    persistTasksToLocalStorage(updatedTasks);
    syncTasksBackend(updatedTasks);
  },

  runTaskAgent: async (taskId) => {
    const { tasks, columns, moveTask, updateTask } = get();
    const task = tasks.find((t) => t.id === taskId);
    if (!task) return;

    const taskCol = columns.find((c) => c.id === task.columnId);
    if (taskCol?.persona) {
      updateTask(taskId, { status: 'running', isTimerRunning: true });
      await syncTasksBackend(get().tasks);
      await triggerRunAgent(taskId, task.description);
    } else {
      // Task sits in a non-agent column: move to the FIRST column with a persona
      // (moveTask auto-runs the agent on entry).
      const firstAgentCol = columns.find((c) => c.persona);
      if (firstAgentCol) {
        await moveTask(taskId, firstAgentCol.id);
      }
    }
  },

  setTaskVerdict: (taskId, verdict, note) => {
    set((state) => {
      const updatedTasks = state.tasks.map((t) =>
        t.id === taskId ? { ...t, verdict, verdictNote: note ?? t.verdictNote } : t
      );
      persistTasksToLocalStorage(updatedTasks);
      syncTasksBackend(updatedTasks);
      return { tasks: updatedTasks };
    });
  },

  toggleTimer: (taskId) => {
    set((state) => ({
      tasks: state.tasks.map((t) =>
        t.id === taskId ? { ...t, isTimerRunning: !t.isTimerRunning } : t
      ),
    }));
  },

  tickTimers: () => {
    set((state) => ({
      tasks: state.tasks.map((task) => {
        if (!task.isTimerRunning || task.status === 'done' || task.status === 'paused') {
          return task;
        }
        return {
          ...task,
          activeStageDuration: task.activeStageDuration + 1,
          totalElapsed: task.totalElapsed + 1,
        };
      }),
    }));
  },

  promoteTaskToNextStage: (taskId) => {
    const { tasks, columns, moveTask } = get();
    const task = tasks.find((t) => t.id === taskId);
    if (!task) return;

    // Custom pipeline flow priority (only if valid against the board)
    if (task.customFlow && task.customFlow.length > 0) {
      const currentIndexInFlow = task.customFlow.indexOf(task.columnId);
      if (currentIndexInFlow >= 0 && currentIndexInFlow < task.customFlow.length - 1) {
        const nextStage = task.customFlow[currentIndexInFlow + 1];
        if (columns.some((c) => c.id === nextStage)) {
          moveTask(taskId, nextStage);
          return;
        }
        // Invalid flow target → fall through to board order
      } else if (currentIndexInFlow >= 0) {
        // Already at the end of a valid custom flow: nowhere to promote
        return;
      }
      // Current column not in customFlow → fall through to board order
    }

    // Default sequential promotion: next column on the board
    const currentIndex = columns.findIndex((c) => c.id === task.columnId);
    if (currentIndex >= 0 && currentIndex < columns.length - 1) {
      const nextCol = columns[currentIndex + 1];
      moveTask(taskId, nextCol.id);
    }
  },
}));

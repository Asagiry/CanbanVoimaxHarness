import { create } from 'zustand';
import { KanbanTask, PipelineColumn, TaskStatus, TaskVerdict, TaskPriority } from '../types';
import { INITIAL_COLUMNS } from './mockData';

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

  loadTasks: () => Promise<void>;
  moveTask: (taskId: string, targetColumnId: string) => void;
  updateTask: (taskId: string, partial: Partial<KanbanTask>) => void;
  addTask: (task: KanbanTask) => void;
  deleteTask: (taskId: string) => void;
  setTaskVerdict: (taskId: string, verdict: TaskVerdict, note?: string) => void;
  toggleTimer: (taskId: string) => void;
  tickTimers: () => void;
  promoteTaskToNextStage: (taskId: string) => void;
  runTaskAgent: (taskId: string) => Promise<void>;
}

// Stage sequence order to compute rework returns
const STAGE_ORDER: Record<string, number> = {
  backlog: 0,
  dev: 1,
  review: 2,
  retro: 3,
  ready: 4,
};

const TASKS_STORAGE_KEY = 'cvh_tasks_backup';

function persistTasksToLocalStorage(tasks: KanbanTask[]): void {
  try {
    if (typeof window !== 'undefined' && window.localStorage) {
      window.localStorage.setItem(TASKS_STORAGE_KEY, JSON.stringify(tasks));
    }
  } catch (err) {
    console.warn('Failed to write tasks backup to localStorage:', err);
  }
}

function loadTasksFromLocalStorage(): KanbanTask[] {
  try {
    if (typeof window !== 'undefined' && window.localStorage) {
      const raw = window.localStorage.getItem(TASKS_STORAGE_KEY);
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

async function syncTasksBackend(tasks: KanbanTask[]): Promise<void> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const backendPayload = tasks.map((t) => ({
      id: t.id,
      title: t.title,
      description: t.description || '',
      stageId: t.columnId,
      columnId: t.columnId,
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

async function triggerRunAgent(taskId: string): Promise<void> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('run_agent', { taskId });
  } catch (err) {
    console.info('run_agent invoke failed or in web mode:', err);
  }
}

function mapBackendTaskToKanban(t: any): KanbanTask {
  return {
    id: t.id,
    title: t.title,
    description: t.description || undefined,
    priority: (t.priority as TaskPriority) || 'high',
    columnId: t.stageId || t.stage_id || 'backlog',
    persona: t.persona || (t.stageId === 'dev' ? 'developer' : t.stageId === 'review' ? 'reviewer' : 'artisan'),
    branch: t.branch || `feat/${t.id.toLowerCase()}`,
    worktreePath: t.worktreePath || t.worktree_path || `worktrees/${t.id.toLowerCase()}`,
    status: (t.status as TaskStatus) || (t.stageId === 'dev' ? 'running' : 'idle'),
    activeStageDuration: t.activeStageDuration || 0,
    stageDurations: t.stageDurations || {},
    totalElapsed: t.totalElapsed || 0,
    isTimerRunning: t.isTimerRunning ?? (t.stageId === 'dev'),
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

  loadTasks: async () => {
    set({ isLoadingTasks: true });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const rawTasks = await invoke<any[]>('list_tasks');
      if (Array.isArray(rawTasks) && rawTasks.length > 0) {
        const mapped = rawTasks.map(mapBackendTaskToKanban);
        set({ tasks: mapped, isTauriConnected: true, isLoadingTasks: false });
        persistTasksToLocalStorage(mapped);
        return;
      }
    } catch (err) {
      console.info('Tauri list_tasks not available or returned empty:', err);
    }

    // Fallback: restore from localStorage backup
    const backupTasks = loadTasksFromLocalStorage();
    if (backupTasks.length > 0) {
      set({ tasks: backupTasks, isLoadingTasks: false });
      syncTasksBackend(backupTasks);
      return;
    }

    set({ tasks: [], isLoadingTasks: false });
  },

  moveTask: (taskId, targetColumnId) => {
    const { tasks, columns } = get();
    const task = tasks.find((t) => t.id === taskId);
    if (!task || task.columnId === targetColumnId) return;

    const sourceColIndex = STAGE_ORDER[task.columnId] ?? 0;
    const targetColIndex = STAGE_ORDER[targetColumnId] ?? 0;

    // Check if task is being returned / reworked backwards
    const isRework = targetColIndex < sourceColIndex || (targetColumnId === 'dev' && task.columnId === 'review');
    const newReturnsCount = isRework ? task.returnsCount + 1 : task.returnsCount;

    // Freeze current stage duration
    const currentStage = task.columnId;
    const currentDuration = task.activeStageDuration;
    const updatedStageDurations = {
      ...task.stageDurations,
      [currentStage]: (task.stageDurations[currentStage] || 0) + currentDuration,
    };

    // Fast-Track checks
    let effectiveTargetCol = targetColumnId;
    if (task.fastTrack?.noReview && targetColumnId === 'review') {
      effectiveTargetCol = task.fastTrack?.noRetro ? 'ready' : 'retro';
    } else if (task.fastTrack?.noRetro && targetColumnId === 'retro') {
      effectiveTargetCol = 'ready';
    }

    const targetColDef = columns.find((c) => c.id === effectiveTargetCol);
    const newStatus: TaskStatus =
      effectiveTargetCol === 'ready'
        ? 'done'
        : effectiveTargetCol === 'dev' || effectiveTargetCol === 'retro'
        ? 'running'
        : effectiveTargetCol === 'review'
        ? 'waiting_review'
        : 'idle';

    const updatedTasks = tasks.map((t) => {
      if (t.id !== taskId) return t;
      return {
        ...t,
        columnId: effectiveTargetCol,
        persona: targetColDef?.persona || t.persona,
        status: newStatus,
        activeStageDuration: 0,
        stageDurations: updatedStageDurations,
        isTimerRunning: newStatus === 'running' || newStatus === 'waiting_review',
        returnsCount: newReturnsCount,
      };
    });

    set({ tasks: updatedTasks });
    persistTasksToLocalStorage(updatedTasks);
    syncTasksBackend(updatedTasks);

    if (effectiveTargetCol === 'dev') {
      triggerRunAgent(taskId);
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

  addTask: (task) => {
    const updatedTasks = [task, ...get().tasks];
    set({ tasks: updatedTasks });
    persistTasksToLocalStorage(updatedTasks);
    syncTasksBackend(updatedTasks);

    if (task.columnId === 'dev') {
      triggerRunAgent(task.id);
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
    const { tasks, moveTask, updateTask } = get();
    const task = tasks.find((t) => t.id === taskId);
    if (!task) return;

    if (task.columnId !== 'dev') {
      moveTask(taskId, 'dev');
    } else {
      updateTask(taskId, { status: 'running', isTimerRunning: true });
      await triggerRunAgent(taskId);
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

    // Custom pipeline flow priority
    if (task.customFlow && task.customFlow.length > 0) {
      if (task.columnId === 'backlog') {
        moveTask(taskId, task.customFlow[0]);
        return;
      }
      const currentIndexInFlow = task.customFlow.indexOf(task.columnId);
      if (currentIndexInFlow >= 0 && currentIndexInFlow < task.customFlow.length - 1) {
        const nextStage = task.customFlow[currentIndexInFlow + 1];
        moveTask(taskId, nextStage);
        return;
      } else if (currentIndexInFlow === -1) {
        const currentOrder = STAGE_ORDER[task.columnId] ?? 0;
        const nextStage = task.customFlow.find((s) => (STAGE_ORDER[s] ?? 0) > currentOrder) || task.customFlow[0];
        moveTask(taskId, nextStage);
        return;
      }
      return;
    }

    // Default sequential promotion
    const currentIndex = columns.findIndex((c) => c.id === task.columnId);
    if (currentIndex >= 0 && currentIndex < columns.length - 1) {
      const nextCol = columns[currentIndex + 1];
      moveTask(taskId, nextCol.id);
    }
  },
}));

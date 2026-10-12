import { create } from 'zustand';
import { KanbanTask, PipelineColumn, TaskStatus, TaskVerdict } from '../types';
import { INITIAL_COLUMNS, INITIAL_TASKS } from './mockData';

interface BoardState {
  columns: PipelineColumn[];
  tasks: KanbanTask[];
  selectedTaskId: string | null;
  searchQuery: string;
  isTauriConnected: boolean;

  // Actions
  setColumns: (columns: PipelineColumn[]) => void;
  setTasks: (tasks: KanbanTask[]) => void;
  setSelectedTaskId: (id: string | null) => void;
  setSearchQuery: (query: string) => void;

  moveTask: (taskId: string, targetColumnId: string) => void;
  updateTask: (taskId: string, partial: Partial<KanbanTask>) => void;
  addTask: (task: KanbanTask) => void;
  deleteTask: (taskId: string) => void;
  setTaskVerdict: (taskId: string, verdict: TaskVerdict, note?: string) => void;
  toggleTimer: (taskId: string) => void;
  tickTimers: () => void;
  promoteTaskToNextStage: (taskId: string) => void;
}

// Stage sequence order to compute rework returns
const STAGE_ORDER: Record<string, number> = {
  backlog: 0,
  todo: 1,
  dev: 2,
  review: 3,
  retro: 4,
  ready: 5,
};

export const useBoardStore = create<BoardState>((set, get) => ({
  columns: INITIAL_COLUMNS,
  tasks: INITIAL_TASKS,
  selectedTaskId: null,
  searchQuery: '',
  isTauriConnected: false,

  setColumns: (columns) => set({ columns }),
  setTasks: (tasks) => set({ tasks }),
  setSelectedTaskId: (id) => set({ selectedTaskId: id }),
  setSearchQuery: (query) => set({ searchQuery: query }),

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
  },

  updateTask: (taskId, partial) => {
    set((state) => ({
      tasks: state.tasks.map((t) => (t.id === taskId ? { ...t, ...partial } : t)),
    }));
  },

  addTask: (task) => {
    set((state) => ({ tasks: [task, ...state.tasks] }));
  },

  deleteTask: (taskId) => {
    set((state) => ({
      tasks: state.tasks.filter((t) => t.id !== taskId),
      selectedTaskId: state.selectedTaskId === taskId ? null : state.selectedTaskId,
    }));
  },

  setTaskVerdict: (taskId, verdict, note) => {
    set((state) => ({
      tasks: state.tasks.map((t) =>
        t.id === taskId ? { ...t, verdict, verdictNote: note ?? t.verdictNote } : t
      ),
    }));
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

    const currentIndex = columns.findIndex((c) => c.id === task.columnId);
    if (currentIndex >= 0 && currentIndex < columns.length - 1) {
      const nextCol = columns[currentIndex + 1];
      moveTask(taskId, nextCol.id);
    }
  },
}));

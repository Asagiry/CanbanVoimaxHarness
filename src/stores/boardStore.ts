import { create } from 'zustand';
import { KanbanTask, PipelineColumn, PipelineConfig, TaskStatus, TaskVerdict, TaskPriority, BoardDef } from '../types';
import { INITIAL_COLUMNS } from './mockData';
import { useProjectStore } from './projectStore';

export const DEFAULT_BOARD: BoardDef = {
  id: 'development',
  name: 'Разработка',
  columns: INITIAL_COLUMNS,
};

interface BoardState {
  columns: PipelineColumn[];
  tasks: KanbanTask[];
  selectedTaskId: string | null;
  hoveredTaskId: string | null;
  searchQuery: string;
  isTauriConnected: boolean;
  isLoadingTasks: boolean;

  boards: BoardDef[];
  activeBoardId: string;
  isLoadingBoards: boolean;

  // Actions
  setColumns: (columns: PipelineColumn[]) => void;
  setTasks: (tasks: KanbanTask[]) => void;
  setSelectedTaskId: (id: string | null) => void;
  setHoveredTaskId: (id: string | null) => void;
  setSearchQuery: (query: string) => void;

  setBoards: (boards: BoardDef[]) => void;
  setActiveBoard: (boardId: string) => Promise<void>;
  loadBoards: () => Promise<void>;
  createBoard: (name: string, template?: string) => Promise<BoardDef | null>;
  saveBoard: (board: BoardDef) => Promise<void>;
  deleteBoard: (boardId: string) => Promise<void>;
  saveCurrentBoard: (columns: PipelineColumn[]) => Promise<void>;
  updateColumn: (columnId: string, partial: Partial<PipelineColumn>) => Promise<void>;

  loadColumns: () => Promise<void>;
  loadTasks: () => Promise<void>;
  moveTask: (taskId: string, targetColumnId: string) => Promise<void>;
  updateTask: (taskId: string, partial: Partial<KanbanTask>) => void;
  addTask: (task: KanbanTask) => Promise<void>;
  createSubtask: (parentId: string, targetColumnId?: string) => Promise<KanbanTask | null>;
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
    const activeBoardId = useBoardStore.getState().activeBoardId || 'development';
    // NOTE: send ONLY canonical camelCase keys. Sending both stageId+columnId
    // makes Rust serde fail with "duplicate field `stageId`" and save_tasks
    // silently never persists (this broke run_agent with "Task not found").
    const backendPayload = tasks.map((t) => ({
      id: t.id,
      title: t.title,
      description: t.description || '',
      stageId: t.columnId,
      boardId: t.boardId || activeBoardId,
      worktreePath: t.worktreePath,
      branch: t.branch,
      verdict: t.verdict === 'NONE' ? null : t.verdict,
      customFlow: t.customFlow,
      status: t.status,
      model: t.model,
      parentId: t.parentId ?? null,
      subtaskIndex: t.subtaskIndex ?? null,
      waitingForSubtaskId: t.waitingForSubtaskId ?? null,
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

  // Respect existing status or infer based on column
  const rawStatus = (t.status as TaskStatus) || undefined;
  const status: TaskStatus = rawStatus ?? (col?.terminal
    ? 'done'
    : col?.persona
    ? 'running'
    : 'idle');

  return {
    id: t.id,
    title: t.title,
    description: t.description || undefined,
    priority: (t.priority as TaskPriority) || 'high',
    columnId,
    boardId: t.boardId || t.board_id || 'development',
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
    parentId: t.parentId ?? t.parent_id ?? undefined,
    subtaskIndex: t.subtaskIndex ?? t.subtask_index ?? undefined,
    waitingForSubtaskId: t.waitingForSubtaskId ?? t.waiting_for_subtask_id ?? undefined,
  };
}

export const useBoardStore = create<BoardState>((set, get) => ({
  columns: INITIAL_COLUMNS,
  tasks: [], // Initialized with empty array - NO mock tasks!
  selectedTaskId: null,
  hoveredTaskId: null,
  searchQuery: '',
  isTauriConnected: false,
  isLoadingTasks: false,

  boards: [DEFAULT_BOARD],
  activeBoardId: 'development',
  isLoadingBoards: false,

  setColumns: (columns) => {
    const { activeBoardId, boards } = get();
    const updatedBoards = boards.map((b) => (b.id === activeBoardId ? { ...b, columns } : b));
    set({ columns, boards: updatedBoards });
  },
  setTasks: (tasks) => {
    persistTasksToLocalStorage(tasks);
    set({ tasks });
  },
  setSelectedTaskId: (id) => set({ selectedTaskId: id }),
  setHoveredTaskId: (id) => set({ hoveredTaskId: id }),
  setSearchQuery: (query) => set({ searchQuery: query }),
  setBoards: (boards) => set({ boards }),

  setActiveBoard: async (boardId: string) => {
    const { boards } = get();
    const targetBoard = boards.find((b) => b.id === boardId);
    if (!targetBoard) return;
    set({
      activeBoardId: boardId,
      columns: targetBoard.columns,
    });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('set_active_board', { boardId });
    } catch (err) {
      console.info('set_active_board IPC failed or in web mode:', err);
    }
  },

  loadBoards: async () => {
    set({ isLoadingBoards: true });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const [rawBoards, rawActiveId] = await Promise.all([
        invoke<BoardDef[]>('list_boards'),
        invoke<string>('get_active_board_id').catch(() => 'development'),
      ]);
      if (Array.isArray(rawBoards) && rawBoards.length > 0) {
        const activeId = rawBoards.some((b) => b.id === rawActiveId)
          ? rawActiveId
          : rawBoards[0].id;
        const currentBoard = rawBoards.find((b) => b.id === activeId) ?? rawBoards[0];
        set({
          boards: rawBoards,
          activeBoardId: activeId,
          columns: currentBoard.columns,
          isLoadingBoards: false,
        });
        return;
      }
    } catch (err) {
      console.info('loadBoards IPC not available or in web mode:', err);
    }

    const current = get();
    if (current.boards.length > 0) {
      const curBoard = current.boards.find((b) => b.id === current.activeBoardId) ?? current.boards[0];
      set({ columns: curBoard.columns, isLoadingBoards: false });
    } else {
      set({
        boards: [DEFAULT_BOARD],
        activeBoardId: 'development',
        columns: INITIAL_COLUMNS,
        isLoadingBoards: false,
      });
    }
  },

  createBoard: async (name: string, template?: string) => {
    const trimmed = name.trim();
    if (!trimmed) return null;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const newBoard = await invoke<BoardDef>('create_board', {
        name: trimmed,
        template: template || null,
      });
      if (newBoard) {
        set((state) => ({
          boards: [...state.boards, newBoard],
          activeBoardId: newBoard.id,
          columns: newBoard.columns,
        }));
        return newBoard;
      }
    } catch (err) {
      console.info('create_board IPC failed, falling back to local state:', err);
    }

    const slug = trimmed.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '') || 'board';
    const uniqueId = `${slug}-${Date.now().toString(36)}`;
    const newBoard: BoardDef = {
      id: uniqueId,
      name: trimmed,
      columns: INITIAL_COLUMNS,
    };
    set((state) => ({
      boards: [...state.boards, newBoard],
      activeBoardId: uniqueId,
      columns: newBoard.columns,
    }));
    return newBoard;
  },

  saveBoard: async (board: BoardDef) => {
    set((state) => {
      const nextBoards = state.boards.map((b) => (b.id === board.id ? board : b));
      const isCurrent = state.activeBoardId === board.id;
      return {
        boards: nextBoards,
        columns: isCurrent ? board.columns : state.columns,
      };
    });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_board', { board });
    } catch (err) {
      console.info('save_board IPC failed or in web mode:', err);
    }
  },

  deleteBoard: async (boardId: string) => {
    const { boards, activeBoardId, tasks } = get();
    if (boards.length <= 1) return;

    const filtered = boards.filter((b) => b.id !== boardId);
    const nextActiveId = activeBoardId === boardId ? filtered[0].id : activeBoardId;
    const nextBoard = filtered.find((b) => b.id === nextActiveId) ?? filtered[0];

    const updatedTasks = tasks.map((t) =>
      (t.boardId || 'development') === boardId ? { ...t, boardId: nextActiveId } : t
    );

    set({
      boards: filtered,
      activeBoardId: nextActiveId,
      columns: nextBoard.columns,
      tasks: updatedTasks,
    });
    persistTasksToLocalStorage(updatedTasks);

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('delete_board', { boardId });
    } catch (err) {
      console.info('delete_board IPC failed or in web mode:', err);
    }
  },

  saveCurrentBoard: async (columns: PipelineColumn[]) => {
    const { activeBoardId, boards } = get();
    const current = boards.find((b) => b.id === activeBoardId);
    if (!current) {
      set({ columns });
      return;
    }
    const updated: BoardDef = { ...current, columns };
    await get().saveBoard(updated);
  },

  updateColumn: async (columnId: string, partial: Partial<PipelineColumn>) => {
    const { columns, activeBoardId, boards } = get();
    const nextColumns = columns.map((c) => (c.id === columnId ? { ...c, ...partial } : c));
    const updatedBoards = boards.map((b) => (b.id === activeBoardId ? { ...b, columns: nextColumns } : b));
    set({ columns: nextColumns, boards: updatedBoards });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      let cfg: PipelineConfig | null = null;
      try {
        cfg = await invoke<PipelineConfig>('get_pipeline');
      } catch {
        cfg = null;
      }
      const pipeline: PipelineConfig = {
        id: cfg?.id ?? 'default-engineering',
        title: cfg?.title ?? 'Инженерный пайплайн',
        columns: nextColumns,
      };
      await invoke('save_pipeline', { pipeline });
      await get().saveCurrentBoard(nextColumns);
    } catch (err) {
      console.info('updateColumn failed or in web mode:', err);
    }
  },

  loadColumns: async () => {
    await get().loadBoards();
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
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('move_task_stage', {
        taskId,
        targetStageId: targetColumnId,
      });
    } catch (err) {
      console.info('Backend move_task_stage failed or in web mode:', err);
    }
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
    const activeBoardId = get().activeBoardId || 'development';
    const taskWithBoard: KanbanTask = {
      ...task,
      boardId: task.boardId || activeBoardId,
    };
    const updatedTasks = [taskWithBoard, ...get().tasks];
    set({ tasks: updatedTasks });
    persistTasksToLocalStorage(updatedTasks);
    await syncTasksBackend(updatedTasks);

    const targetColDef = get().columns.find((c) => c.id === taskWithBoard.columnId);
    if (targetColDef?.autoRun ?? !!targetColDef?.persona) {
      await triggerRunAgent(taskWithBoard.id, taskWithBoard.description);
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

    // Calculate subtask index
    const existingSubtasks = tasks.filter(
      (t) => t.parentId === parentId || t.id.startsWith(`${parentId}.`)
    );
    const nextIndex =
      existingSubtasks.reduce((max, t) => {
        const parsed = t.subtaskIndex ?? parseInt(t.id.split('.').pop() || '0', 10);
        return Math.max(max, isNaN(parsed) ? 0 : parsed);
      }, 0) + 1;
    const subtaskId = `${parentId}.${nextIndex}`;

    // Try Tauri backend IPC first
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const backendSubtask = await invoke<any>('create_subtask', {
        parentId,
        targetStageId: targetCol.id,
        title: null,
      });
      if (backendSubtask) {
        await get().loadTasks();
        return get().tasks.find((t) => t.id === subtaskId) || null;
      }
    } catch (err) {
      console.info('create_subtask invoke fallback to client state:', err);
    }

    // Web mode / client fallback
    const subtaskStatus: TaskStatus = targetCol.terminal
      ? 'done'
      : targetCol.persona
      ? 'running'
      : 'idle';

    const newSubtask: KanbanTask = {
      id: subtaskId,
      title: `[${targetCol.title}] ${parent.title}`,
      description: `### Подзадача этапа: ${targetCol.title}\nРодительская задача: ${parentId}\n\n${parent.description || ''}`,
      priority: parent.priority,
      columnId: targetCol.id,
      persona: targetCol.persona || 'reviewer',
      branch: parent.branch,
      worktreePath: parent.worktreePath,
      status: subtaskStatus,
      activeStageDuration: 0,
      stageDurations: {},
      totalElapsed: 0,
      isTimerRunning: subtaskStatus === 'running',
      returnsCount: 0,
      verdict: 'NONE',
      model: targetCol.modelOverride || parent.model,
      parentId,
      subtaskIndex: nextIndex,
    };

    const updatedTasks = tasks.map((t) => {
      if (t.id === parentId) {
        return {
          ...t,
          status: 'waiting_subtask' as TaskStatus,
          waitingForSubtaskId: subtaskId,
        };
      }
      return t;
    });

    const finalTasks = [...updatedTasks, newSubtask];
    set({ tasks: finalTasks });
    persistTasksToLocalStorage(finalTasks);
    await syncTasksBackend(finalTasks);

    if (targetCol.autoRun ?? !!targetCol.persona) {
      await triggerRunAgent(subtaskId, newSubtask.description);
    }

    return newSubtask;
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

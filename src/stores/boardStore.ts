import { create } from 'zustand';
import { KanbanTask, PipelineColumn, PipelineConfig, TaskStatus, TaskVerdict, TaskPriority, BoardDef } from '../types';
import { INITIAL_COLUMNS } from './mockData';
import { useProjectStore } from './projectStore';
import { useDrawerStore } from './drawerStore';

export const CANONICAL_BOARD_COLUMNS: PipelineColumn[] = [
  {
    id: 'backlog',
    title: 'Ожидание',
    color: '#64748b',
    icon: 'Inbox',
    persona: null,
    contextWindow: 128000,
    modelOverride: null,
    wipLimit: 10,
    hooks: {},
  },
  {
    id: 'dev',
    title: 'В разработке',
    color: '#f59e0b',
    icon: 'Code2',
    persona: 'developer',
    contextWindow: 400000,
    modelOverride: 'gemini-3.8-flash-high',
    temperature: 0.2,
    wipLimit: 3,
    hooks: {
      on_enter: '.cvh/hooks/on_enter_dev.bat',
    },
    gatekeeperSensor: 'cargo check',
  },
  {
    id: 'ready',
    title: 'Готово',
    color: '#10b981',
    icon: 'CheckCircle2',
    persona: null,
    terminal: true,
    contextWindow: 64000,
    modelOverride: null,
    hooks: {
      on_enter: '.cvh/hooks/on_finish_done.bat',
    },
    gatekeeperSensor: 'cargo check & npm run build',
  },
];

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
  updateTaskStatus: (taskId: string, status: TaskStatus) => void;
  addTask: (input: {
    title: string;
    description?: string;
    stageId?: string;
    boardId?: string;
    skipWorktree?: boolean;
    model?: string;
  }) => Promise<KanbanTask | null>;
  createSubtask: (parentId: string, targetColumnId?: string) => Promise<KanbanTask | null>;
  deleteTask: (taskId: string) => Promise<void>;
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

export function getTaskSeqStorageKey(): string {
  try {
    const activeProject = useProjectStore.getState().activeProject;
    return `cvh_task_seq_${activeProject?.id || 'default'}`;
  } catch {
    return 'cvh_task_seq_default';
  }
}

function getSubtaskSeqStorageKey(parentId: string): string {
  return `${getTaskSeqStorageKey()}_sub_${parentId}`;
}

/** Numeric suffix of a top-level id: CVH-012 -> 12. Subtask CVH-012.3 -> base 12. */
export function parseTopLevelTaskNum(id: string): number {
  const m = /^CVH-(\d+)/i.exec(id.trim());
  if (!m) return 0;
  const n = parseInt(m[1], 10);
  return Number.isFinite(n) ? n : 0;
}

function readSeq(key: string): number {
  try {
    if (typeof window !== 'undefined' && window.localStorage) {
      const raw = window.localStorage.getItem(key);
      const n = raw == null ? 0 : parseInt(raw, 10);
      return Number.isFinite(n) && n > 0 ? n : 0;
    }
  } catch {
    // ignore — treat as empty counter
  }
  return 0;
}

function writeSeq(key: string, value: number): void {
  try {
    if (typeof window !== 'undefined' && window.localStorage) {
      window.localStorage.setItem(key, String(value));
    }
  } catch (err) {
    console.warn('Failed to write task seq to localStorage:', err);
  }
}

function maxTaskNum(tasks: KanbanTask[]): number {
  let max = 0;
  for (const t of tasks) {
    const n = parseTopLevelTaskNum(t.id);
    if (n > max) max = n;
  }
  return max;
}

/**
 * Raise the persistent counter floor so it never falls below existing tasks.
 * Only ever raises — never resets, so deleted IDs are never re-issued.
 */
export function syncTaskSeqWithTasks(tasks: KanbanTask[]): void {
  const key = getTaskSeqStorageKey();
  writeSeq(key, Math.max(readSeq(key), maxTaskNum(tasks), tasks.length));
}

/**
 * Allocate the next monotonic top-level task number and persist the
 * incremented counter. next = max(counter, max numeric suffix, tasks.length) + 1.
 */
export function allocateNextTaskNum(tasks: KanbanTask[]): number {
  const key = getTaskSeqStorageKey();
  const next = Math.max(readSeq(key), maxTaskNum(tasks), tasks.length) + 1;
  writeSeq(key, next);
  return next;
}

export function formatTaskId(num: number): string {
  return `CVH-${String(num).padStart(3, '0')}`;
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

function normalizeKanbanTask(raw: any): KanbanTask | null {
  if (!raw || typeof raw.id !== 'string' || raw.id.trim() === '') return null;
  const { boards, activeBoardId, columns: currentColumns } = useBoardStore.getState();

  // Dynamic board resolution:
  // If a task's boardId is missing or doesn't match any board, fallback to activeBoardId (or first board).
  const rawBoardId = typeof raw.boardId === 'string' ? raw.boardId.trim() : (typeof raw.board_id === 'string' ? raw.board_id.trim() : '');
  const hasMatchingBoard = rawBoardId !== '' && boards.some((b) => b.id === rawBoardId);
  const boardId = hasMatchingBoard
    ? rawBoardId
    : (activeBoardId || boards[0]?.id || 'development');

  // Determine target board's columns to validate columnId
  const targetBoard = boards.find((b) => b.id === boardId);
  const targetColumns = targetBoard?.columns && targetBoard.columns.length > 0
    ? targetBoard.columns
    : (currentColumns && currentColumns.length > 0 ? currentColumns : INITIAL_COLUMNS);

  const rawId: string | undefined = raw.stageId || raw.stage_id || raw.columnId;
  const knownCol = targetColumns.find((c) => c.id === rawId);
  // If columnId doesn't match any column in the target board, fallback to the first column (backlog)
  const columnId = knownCol?.id ?? targetColumns[0]?.id ?? 'backlog';
  const col = targetColumns.find((c) => c.id === columnId);
  const rawStatus = (raw.status as TaskStatus) || undefined;
  const status: TaskStatus = rawStatus ?? (col?.terminal
    ? 'done'
    : col?.persona
    ? 'running'
    : 'idle');
  const stageDurations: Record<string, number> =
    raw.stageDurations && typeof raw.stageDurations === 'object' && !Array.isArray(raw.stageDurations)
      ? raw.stageDurations
      : {};
  const createdAt = typeof raw.createdAt === 'number' && Number.isFinite(raw.createdAt)
    ? raw.createdAt
    : (typeof raw.created_at === 'number' && Number.isFinite(raw.created_at) ? raw.created_at : Date.now());
  const updatedAt = typeof raw.updatedAt === 'number' && Number.isFinite(raw.updatedAt)
    ? raw.updatedAt
    : (typeof raw.updated_at === 'number' && Number.isFinite(raw.updated_at) ? raw.updated_at : Date.now());
  const stageEnteredAt = typeof raw.stageEnteredAt === 'number' && Number.isFinite(raw.stageEnteredAt)
    ? raw.stageEnteredAt
    : (typeof raw.stage_entered_at === 'number' && Number.isFinite(raw.stage_entered_at)
      ? raw.stage_entered_at
      : undefined);

  const stageStart = stageEnteredAt || updatedAt || createdAt;
  const isDoneTask = status === 'done';
  const computedStageDuration = isDoneTask
    ? (typeof raw.activeStageDuration === 'number' && Number.isFinite(raw.activeStageDuration) ? raw.activeStageDuration : 0)
    : (stageStart ? Math.max(0, Math.floor((Date.now() - stageStart) / 1000)) : 0);

  const activeStageDuration = typeof raw.activeStageDuration === 'number' && Number.isFinite(raw.activeStageDuration) && raw.activeStageDuration > 0
    ? raw.activeStageDuration
    : computedStageDuration;

  const computedTotalElapsed = isDoneTask
    ? (typeof raw.totalElapsed === 'number' && Number.isFinite(raw.totalElapsed) ? raw.totalElapsed : 0)
    : (createdAt ? Math.max(0, Math.floor((Date.now() - createdAt) / 1000)) : 0);

  const totalElapsed = typeof raw.totalElapsed === 'number' && Number.isFinite(raw.totalElapsed) && raw.totalElapsed > 0
    ? raw.totalElapsed
    : computedTotalElapsed;

  const returnsCount = typeof raw.returnsCount === 'number' && Number.isFinite(raw.returnsCount)
    ? raw.returnsCount
    : 0;
  return {
    id: raw.id,
    title: typeof raw.title === 'string' && raw.title.trim() !== '' ? raw.title : raw.id,
    description: typeof raw.description === 'string' && raw.description !== '' ? raw.description : undefined,
    attachments: Array.isArray(raw.attachments) ? raw.attachments.filter((a: unknown) => typeof a === 'string') : undefined,
    createdAt,
    updatedAt,
    stageEnteredAt,
    priority: (raw.priority as TaskPriority) || 'high',
    columnId,
    boardId,
    persona: col?.persona ?? (typeof raw.persona === 'string' ? raw.persona : 'developer'),
    branch: raw.branch || `feat/${raw.id.toLowerCase()}`,
    worktreePath: raw.worktreePath || raw.worktree_path || `worktrees/${raw.id.toLowerCase()}`,
    status,
    activeStageDuration,
    stageDurations,
    totalElapsed,
    isTimerRunning: typeof raw.isTimerRunning === 'boolean' ? raw.isTimerRunning : status === 'running',
    returnsCount,
    fastTrack: raw.fastTrack,
    customFlow: raw.customFlow || raw.custom_flow,
    verdict: (raw.verdict as TaskVerdict) || 'NONE',
    verdictNote: raw.verdictNote,
    model: raw.model || undefined,
    parentId: raw.parentId ?? raw.parent_id ?? undefined,
    subtaskIndex: raw.subtaskIndex ?? raw.subtask_index ?? undefined,
    waitingForSubtaskId: raw.waitingForSubtaskId ?? raw.waiting_for_subtask_id ?? undefined,
    todos: Array.isArray(raw.todos)
      ? raw.todos.map((item: any, idx: number) => ({
          id: typeof item?.id === 'string' && item.id.trim() !== '' ? item.id : `todo-${idx + 1}`,
          content: typeof item?.content === 'string' ? item.content : '',
          status: item?.status === 'completed' || item?.status === 'in_progress' ? item.status : 'pending',
        }))
      : undefined,
    handoverSummary: typeof raw.handoverSummary === 'string'
      ? raw.handoverSummary
      : (typeof raw.handover_summary === 'string' ? raw.handover_summary : undefined),
  };
}

function loadTasksFromLocalStorage(): KanbanTask[] {
  try {
    if (typeof window !== 'undefined' && window.localStorage) {
      const raw = window.localStorage.getItem(getTasksStorageKey());
      if (raw) {
        const parsed = JSON.parse(raw);
        if (Array.isArray(parsed) && parsed.length > 0) {
          const normalized = parsed
            .map((t) => {
              try {
                return normalizeKanbanTask(t);
              } catch {
                return null;
              }
            })
            .filter((t): t is KanbanTask => t !== null);
          return normalized;
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
    const { activeBoardId, boards } = useBoardStore.getState();
    const fallbackBoardId = activeBoardId || boards[0]?.id || 'development';
    // NOTE: send ONLY canonical camelCase keys. Sending both stageId+columnId
    // makes Rust serde fail with "duplicate field `stageId`" and save_tasks
    // silently never persists (this broke run_agent with "Task not found").
    const backendPayload = tasks.map((t) => {
      const hasBoard = boards.some((b) => b.id === t.boardId);
      const effectiveBoardId = hasBoard ? t.boardId : fallbackBoardId;
      return {
        id: t.id,
        title: t.title,
        description: t.description || '',
        stageId: t.columnId,
        boardId: effectiveBoardId,
        worktreePath: t.worktreePath,
        branch: t.branch,
        verdict: t.verdict === 'NONE' ? null : t.verdict,
        customFlow: t.customFlow,
        status: t.status,
        model: t.model,
        parentId: t.parentId ?? null,
        subtaskIndex: t.subtaskIndex ?? null,
        waitingForSubtaskId: t.waitingForSubtaskId ?? null,
        // NOTE: todos MUST be included — otherwise every sync wipes the plan
        // the agent saved via todo_write (backend save_tasks replaces the file).
        todos: Array.isArray(t.todos)
          ? t.todos.map((td: any) => ({
              id: String(td?.id ?? ''),
              content: String(td?.content ?? ''),
              status: td?.status === 'completed' || td?.status === 'in_progress' ? td.status : 'pending',
            }))
          : null,
        createdAt: t.createdAt || Date.now(),
        updatedAt: Date.now(),
      };
    });
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
  // normalizeKanbanTask fills defaults for legacy tasks created before
  // cover/attachments fields existed (localStorage / .cvh/tasks.json).
  const normalized = normalizeKanbanTask(t);
  if (normalized) return normalized;
  // Крайний фолбэк: запись без id — собрать безопасную задачу-заглушку,
  // чтобы рендер никогда не падал с TypeError.
  const { columns, boards, activeBoardId } = useBoardStore.getState();
  const fallbackId = `unknown-${Date.now().toString(36)}`;
  const fallbackCol = columns[0]?.id ?? 'backlog';
  const fallbackBoardId = activeBoardId || boards[0]?.id || 'development';
  return {
    id: fallbackId,
    title: fallbackId,
    description: undefined,
    attachments: undefined,
    createdAt: Date.now(),
    updatedAt: Date.now(),
    priority: 'high' as TaskPriority,
    columnId: fallbackCol,
    boardId: fallbackBoardId,
    persona: columns.find((c) => c.id === fallbackCol)?.persona ?? 'developer',
    branch: `feat/${fallbackId.toLowerCase()}`,
    worktreePath: `worktrees/${fallbackId.toLowerCase()}`,
    status: 'idle' as TaskStatus,
    activeStageDuration: 0,
    stageDurations: {},
    totalElapsed: 0,
    isTimerRunning: false,
    returnsCount: 0,
    fastTrack: undefined,
    customFlow: undefined,
    verdict: 'NONE' as TaskVerdict,
    verdictNote: undefined,
    model: undefined,
    parentId: undefined,
    subtaskIndex: undefined,
    waitingForSubtaskId: undefined,
    handoverSummary: undefined,
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

        // Dynamic Self-Healing: Verify all tasks in store against the loaded boards
        const currentTasks = get().tasks;
        const healedTasks = currentTasks.map((t) => {
          const hasBoard = rawBoards.some((b) => b.id === t.boardId);
          const effectiveBoardId = hasBoard ? t.boardId : activeId;
          const targetBoard = rawBoards.find((b) => b.id === effectiveBoardId) ?? currentBoard;
          const colExists = targetBoard.columns.some((c) => c.id === t.columnId);
          const effectiveColId = colExists ? t.columnId : (targetBoard.columns[0]?.id ?? 'backlog');
          if (effectiveBoardId !== t.boardId || effectiveColId !== t.columnId) {
            return { ...t, boardId: effectiveBoardId, columnId: effectiveColId };
          }
          return t;
        });

        set({
          boards: rawBoards,
          activeBoardId: activeId,
          columns: currentBoard.columns,
          tasks: healedTasks,
          isLoadingBoards: false,
        });
        persistTasksToLocalStorage(healedTasks);
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
        template: template || 'canonical',
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
      columns: CANONICAL_BOARD_COLUMNS,
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
      // Heal tasks on this board whose columns may have been removed
      const nextTasks = state.tasks.map((t) => {
        if (t.boardId === board.id) {
          const colExists = board.columns.some((c) => c.id === t.columnId);
          if (!colExists) {
            return { ...t, columnId: board.columns[0]?.id ?? 'backlog' };
          }
        }
        return t;
      });
      return {
        boards: nextBoards,
        columns: isCurrent ? board.columns : state.columns,
        tasks: nextTasks,
      };
    });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_board', { board });
    } catch (err) {
      console.info('save_board IPC failed or in web mode:', err);
      throw err instanceof Error ? err : new Error(String(err));
    }
  },

  deleteBoard: async (boardId: string) => {
    const { boards, activeBoardId, tasks } = get();
    if (boards.length <= 1) return;

    const filtered = boards.filter((b) => b.id !== boardId);
    const nextActiveId = activeBoardId === boardId ? filtered[0].id : activeBoardId;
    const nextBoard = filtered.find((b) => b.id === nextActiveId) ?? filtered[0];

    const updatedTasks = tasks.map((t) => {
      const isTarget = (t.boardId || 'development') === boardId;
      if (!isTarget) return t;
      const targetCols = nextBoard.columns;
      const colExists = targetCols.some((c) => c.id === t.columnId);
      const healedCol = colExists ? t.columnId : (targetCols[0]?.id ?? 'backlog');
      return { ...t, boardId: nextActiveId, columnId: healedCol };
    });

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
          const prevTasks = get().tasks;
          const mapped = rawTasks.map((raw) => {
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
            return mapBackendTaskToKanban(raw);
          });
          set({ tasks: mapped, isTauriConnected: true, isLoadingTasks: false });
          persistTasksToLocalStorage(mapped);
          syncTaskSeqWithTasks(mapped);
          return;
        } else {
          // Backend has empty tasks array on disk: check per-project localStorage backup
          const backupTasks = loadTasksFromLocalStorage();
          if (backupTasks.length > 0) {
            set({ tasks: backupTasks, isTauriConnected: true, isLoadingTasks: false });
            syncTaskSeqWithTasks(backupTasks);
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
      syncTaskSeqWithTasks(backupTasks);
      await syncTasksBackend(backupTasks);
      return;
    }

    set({ tasks: [], isLoadingTasks: false });
  },

  moveTask: async (taskId, targetColumnId) => {
    const { tasks, columns } = get();
    const task = tasks.find((t) => t.id === taskId);
    const targetStageId = (targetColumnId || '').trim();
    if (!task) {
      console.warn(`[moveTask] Task "${taskId}" not found in local store`);
      return;
    }
    if (!targetStageId) {
      console.warn(`[moveTask] Invalid empty targetColumnId for task "${taskId}"`);
      return;
    }
    if (task.columnId === targetStageId) return;

    const targetColDef = columns.find((c) => c.id === targetStageId);
    console.info(`[moveTask] Moving task "${taskId}" from "${task.columnId}" to "${targetStageId}"`);

    // Preferred path: backend owns the move (gatekeeper sensor, target on_enter
    // hooks, source on_finish hooks + worktree allocation). Local state is
    // updated from the returned Task so hook-assigned worktreePath/branch
    // survive. `verdict` is optional in move_task_stage and omitted here.
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const backendTask = await invoke<any>('move_task_stage', {
        taskId,
        targetStageId,
      });
      if (backendTask && typeof backendTask.id === 'string') {
        const mapped = mapBackendTaskToKanban(backendTask);
        const prev = get().tasks.find((t) => t.id === taskId) ?? task;
        // Local-only timer/UX bookkeeping (backend has no timer fields).
        const sourceColIndex = columns.findIndex((c) => c.id === prev.columnId);
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
          persona: targetColDef?.persona || prev.persona,
          verdict: backendTask.verdict ? mapped.verdict : prev.verdict,
          verdictNote: prev.verdictNote,
          priority: prev.priority,
          fastTrack: prev.fastTrack,
          attachments: mapped.attachments ?? prev.attachments,
          handoverSummary: mapped.handoverSummary ?? prev.handoverSummary,
          stageEnteredAt: Date.now(),
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
        const updatedTasks = get().tasks.map((t) => (t.id === taskId ? merged : t));
        set({ tasks: updatedTasks });
        persistTasksToLocalStorage(updatedTasks);
        // NOTE: no syncTasksBackend here — backend already persisted the move;
        // pushing local state via save_tasks would clobber hook-assigned worktree.
        if (targetColDef?.autoRun ?? !!targetColDef?.persona) {
          await triggerRunAgent(taskId, merged.description);
        }
        return;
      }
      throw new Error(`move_task_stage returned empty or invalid task for id "${taskId}"`);
    } catch (err: unknown) {
      console.error(`[moveTask] Backend move_task_stage failed for task "${taskId}" -> stage "${targetStageId}":`, err);
      const msg = typeof err === 'string' ? err : err instanceof Error ? err.message : JSON.stringify(err);
      alert(`Перемещение задачи отклонено:\n${msg}`);
      // Откатить состояние / синхронизировать карточки с бэкенда
      const { loadTasks } = get();
      await loadTasks();
      return;
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

  updateTaskStatus: (taskId, status) => {
    set((state) => {
      const updatedTasks = state.tasks.map((t) => {
        if (t.id !== taskId) return t;
        const isTimerRunning = status === 'running' ? t.isTimerRunning : false;
        return { ...t, status, isTimerRunning };
      });
      persistTasksToLocalStorage(updatedTasks);
      syncTasksBackend(updatedTasks);
      return { tasks: updatedTasks };
    });
  },

  addTask: async (input) => {
    const state = get();
    const targetColumnId =
      input.stageId ||
      (state.columns.some((c) => c.id === 'backlog') ? 'backlog' : (state.columns[0]?.id ?? 'backlog'));
    const boardId = input.boardId || state.activeBoardId || 'development';
    const skipWorktree = input.skipWorktree ?? false;
    const title = input.title.trim();
    if (!title) return null;
    const description = input.description?.trim() || '';

    // Preferred path: backend owns creation (IDs, on_enter hooks + worktree).
    // Backend already persisted to disk — syncTasksBackend MUST NOT be called
    // here (it would clobber hook-assigned worktreePath/branch via save_tasks).
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const backendTask = await invoke<any>('create_task', {
        title,
        description,
        stageId: targetColumnId,
        boardId: boardId,
        skipWorktree,
      });
      if (backendTask && typeof backendTask.id === 'string') {
        const mapped = mapBackendTaskToKanban(backendTask);
        const merged: KanbanTask = {
          ...mapped,
          model: mapped.model || input.model || undefined,
        };
        const updatedTasks = [merged, ...get().tasks];
        set({ tasks: updatedTasks });
        persistTasksToLocalStorage(updatedTasks);
        // Keep the monotonic counter ahead of the backend-issued id.
        syncTaskSeqWithTasks(updatedTasks);
        const targetColDef = get().columns.find((c) => c.id === merged.columnId);
        if (targetColDef?.autoRun ?? !!targetColDef?.persona) {
          await triggerRunAgent(merged.id, merged.description);
        }
        return merged;
      }
      throw new Error('create_task returned empty task');
    } catch (err) {
      console.info('Backend create_task failed, falling back to local creation:', err);
    }

    // Fallback path (web mode / IPC error): local ID + save_tasks.
    const fb = get();
    const paddedId = formatTaskId(allocateNextTaskNum(fb.tasks));
    const fbTargetCol = fb.columns.find((c) => c.id === targetColumnId);
    const status: TaskStatus = fbTargetCol?.terminal
      ? 'done'
      : fbTargetCol?.persona
        ? 'running'
        : 'idle';
    const fallbackTask: KanbanTask = {
      id: paddedId,
      title,
      description: description || undefined,
      priority: 'high',
      columnId: targetColumnId,
      boardId,
      persona: fbTargetCol?.persona ?? 'developer',
      branch: `feat/${paddedId.toLowerCase()}`,
      worktreePath: `.cvh/worktrees/${paddedId.toLowerCase()}`,
      status,
      createdAt: Date.now(),
      updatedAt: Date.now(),
      stageEnteredAt: Date.now(),
      activeStageDuration: 0,
      stageDurations: {},
      totalElapsed: 0,
      isTimerRunning: status === 'running',
      returnsCount: 0,
      verdict: 'NONE',
      model: input.model,
    };
    const finalTasks = [fallbackTask, ...fb.tasks];
    set({ tasks: finalTasks });
    persistTasksToLocalStorage(finalTasks);
    syncTaskSeqWithTasks(finalTasks);
    await syncTasksBackend(finalTasks);

    const fbTargetDef = get().columns.find((c) => c.id === fallbackTask.columnId);
    if (fbTargetDef?.autoRun ?? !!fbTargetDef?.persona) {
      await triggerRunAgent(fallbackTask.id, fallbackTask.description);
    }
    return fallbackTask;
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

    // Calculate subtask index — monotonic per parent: never re-issue an index
    // of a deleted subtask (its dialog history may still be referenced).
    const existingSubtasks = tasks.filter(
      (t) => t.parentId === parentId || t.id.startsWith(`${parentId}.`)
    );
    const siblingMax = existingSubtasks.reduce((max, t) => {
      const parsed = t.subtaskIndex ?? parseInt(t.id.split('.').pop() || '0', 10);
      return Math.max(max, isNaN(parsed) ? 0 : parsed);
    }, 0);
    const subSeqKey = getSubtaskSeqStorageKey(parentId);
    const nextIndex = Math.max(siblingMax + 1, readSeq(subSeqKey) + 1);
    const subtaskId = `${parentId}.${nextIndex}`;

    // Try Tauri backend IPC first
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const backendSubtask = await invoke<any>('create_subtask', {
        parentId,
        targetStageId: targetCol.id,
        title: null,
      });
      if (backendSubtask && typeof backendSubtask.id === 'string') {
        // Backend computes its own index (max+1 over disk tasks): adopt the
        // real id and raise our per-parent counter to it so later indexes
        // never collide with it.
        const usedIdx = parseInt(backendSubtask.id.split('.').pop() || '0', 10);
        writeSeq(subSeqKey, Number.isFinite(usedIdx) && usedIdx > 0 ? Math.max(usedIdx, nextIndex) : nextIndex);
        await get().loadTasks();
        return get().tasks.find((t) => t.id === backendSubtask.id) || null;
      }
      if (backendSubtask) {
        writeSeq(subSeqKey, nextIndex);
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
      createdAt: Date.now(),
      updatedAt: Date.now(),
      stageEnteredAt: Date.now(),
      activeStageDuration: 0,
      stageDurations: {},
      totalElapsed: 0,
      isTimerRunning: subtaskStatus === 'running',
      returnsCount: 0,
      verdict: 'NONE',
      model: targetCol.modelOverride || parent.model,
      parentId,
      subtaskIndex: nextIndex,
      handoverSummary: undefined,
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
    writeSeq(getSubtaskSeqStorageKey(parentId), nextIndex);
    await syncTasksBackend(finalTasks);

    if (targetCol.autoRun ?? !!targetCol.persona) {
      await triggerRunAgent(subtaskId, newSubtask.description);
    }

    return newSubtask;
  },

  deleteTask: async (taskId) => {
    const { tasks } = get();
    const target = tasks.find((t) => t.id === taskId);
    // Subtasks (CVH-012.1) share the parent's worktree/branch — only
    // top-level tasks own git resources.
    const isSubtask = !!target?.parentId || taskId.includes('.');

    // Cascade: the task plus all its descendants (via parentId or id prefix).
    const toDelete = new Set<string>([taskId]);
    let grew = true;
    while (grew) {
      grew = false;
      for (const t of tasks) {
        if (toDelete.has(t.id)) continue;
        if ((t.parentId && toDelete.has(t.parentId)) || [...toDelete].some((id) => t.id.startsWith(`${id}.`))) {
          toDelete.add(t.id);
          grew = true;
        }
      }
    }
    const ids = [...toDelete];

    // (a) Worktree + branch cleanup (best-effort, never blocks deletion).
    if (!isSubtask) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('cleanup_task_worktree', { taskId });
      } catch (err) {
        console.info('cleanup_task_worktree invoke error or web mode:', err);
      }
    }

    // (б) Dialog history cleanup via the drawer store action:
    // invoke clear_task_dialog (.cvh/harness.db SQLite table dialog_items).
    for (const id of ids) {
      try {
        await useDrawerStore.getState().clearTaskLogs(id);
      } catch (err) {
        console.info('clearTaskLogs error for task:', id, err);
      }
    }

    // (в) Remove from store + persist + sync.
    const updatedTasks = get().tasks.filter((t) => !toDelete.has(t.id));
    set({
      tasks: updatedTasks,
      selectedTaskId: toDelete.has(get().selectedTaskId ?? '') ? null : get().selectedTaskId,
    });
    persistTasksToLocalStorage(updatedTasks);
    await syncTasksBackend(updatedTasks);
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
    const now = Date.now();
    set((state) => ({
      tasks: state.tasks.map((task) => {
        if (task.status === 'done' || task.status === 'paused') {
          return task;
        }
        const stageStart = task.stageEnteredAt || task.updatedAt || task.createdAt;
        const activeStageDuration = stageStart
          ? Math.max(0, Math.floor((now - stageStart) / 1000))
          : (typeof task.activeStageDuration === 'number' && Number.isFinite(task.activeStageDuration) ? task.activeStageDuration + 1 : 1);

        const totalElapsed = task.createdAt
          ? Math.max(0, Math.floor((now - task.createdAt) / 1000))
          : (typeof task.totalElapsed === 'number' && Number.isFinite(task.totalElapsed) ? task.totalElapsed + 1 : 1);

        return {
          ...task,
          activeStageDuration,
          totalElapsed,
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

let boardListenersInitialized = false;
export async function initBoardEventListeners(): Promise<() => void> {
  if (boardListenersInitialized) return () => {};
  try {
    const { listen } = await import('@tauri-apps/api/event');
    const uBoards = await listen<any>('boards:updated', async () => {
      await useBoardStore.getState().loadBoards();
      await useBoardStore.getState().loadTasks();
    });
    const uPipeline = await listen<any>('pipeline:updated', async () => {
      await useBoardStore.getState().loadBoards();
      await useBoardStore.getState().loadTasks();
    });
    boardListenersInitialized = true;
    return () => {
      uBoards();
      uPipeline();
      boardListenersInitialized = false;
    };
  } catch (e) {
    console.info('initBoardEventListeners not active in web mode:', e);
    return () => {};
  }
}

import { create } from 'zustand';
import { KanbanTask, PipelineColumn, PipelineConfig, TaskStatus, TaskVerdict, TaskPriority, BoardDef } from '../types';
import { useProjectStore } from './projectStore';
import { useDrawerStore } from './drawerStore';
import { logEvent } from '../lib/logger';



const CANONICAL_BOARD_COLUMNS: PipelineColumn[] = [
  {
    id: 'backlog',
    title: 'Backlog',
    color: '#64748b',
    icon: 'Inbox',
    persona: null,
    modelOverride: null,
    wipLimit: 10,
    hooks: {},
    gatekeeperSensor: null,
  },
  {
    id: 'dev',
    title: 'CVH Development',
    color: '#6366f1',
    icon: 'Bot',
    persona: 'cvh-dev',
    modelOverride: 'gemini-3.8-flash-high',
    temperature: 0.2,
    wipLimit: 3,
    hooks: {},
    gatekeeperSensor: null,
  },
  {
    id: 'ready',
    title: 'Ready',
    color: '#10b981',
    icon: 'CheckCircle2',
    persona: null,
    terminal: true,
    modelOverride: null,
    hooks: {},
    gatekeeperSensor: null,
  },
];

const DEFAULT_BOARD: BoardDef = {
  id: 'development',
  name: 'cvh-board',
  columns: CANONICAL_BOARD_COLUMNS,
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
  moveTask: (taskId: string, targetColumnId: string, options?: { force?: boolean; sourceColumnId?: string }) => Promise<void>;
  updateTask: (taskId: string, partial: Partial<KanbanTask>) => void;
  updateTaskStatus: (taskId: string, status: TaskStatus) => void;
  upsertBackendTask: (rawTask: any) => void;
  setBackendTasks: (rawTasks: any[]) => void;
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

function getTasksStorageKey(): string {
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
    : (currentColumns && currentColumns.length > 0 ? currentColumns : CANONICAL_BOARD_COLUMNS);

  const rawId: string | undefined = raw.stageId || raw.stage_id || raw.columnId;

  // ZERO INVISIBLE TASKS (§ 2.18): Ensure columnId and boardId are mutually consistent.
  // If rawId does not exist in targetBoard, but exists in active board columns, heal boardId to activeBoardId.
  let effectiveBoardId = boardId;
  let effectiveColumns = targetColumns;
  let knownCol = effectiveColumns.find((c) => c.id === rawId || c.id.toLowerCase() === rawId?.toLowerCase());

  if (!knownCol && activeBoardId && activeBoardId !== boardId) {
    const activeBoardDef = boards.find((b) => b.id === activeBoardId);
    const activeCols = activeBoardDef?.columns && activeBoardDef.columns.length > 0 ? activeBoardDef.columns : currentColumns;
    const colInActive = activeCols.find((c) => c.id === rawId || c.id.toLowerCase() === rawId?.toLowerCase());
    if (colInActive) {
      effectiveBoardId = activeBoardId;
      effectiveColumns = activeCols;
      knownCol = colInActive;
    }
  }

  // Alias matching fallback (dev <-> development, backlog <-> waiting, ready <-> done)
  const matchedCol = knownCol ?? effectiveColumns.find((c) => {
    if (!rawId) return false;
    const cid = c.id.toLowerCase();
    const rid = rawId.toLowerCase();
    if ((rid === 'dev' || rid === 'development') && (cid === 'dev' || cid === 'development')) return true;
    if ((rid === 'backlog' || rid === 'waiting' || rid === 'ожидание') && (cid === 'backlog' || cid === 'waiting' || cid === 'ожидание')) return true;
    if ((rid === 'ready' || rid === 'done' || rid === 'готово') && (cid === 'ready' || cid === 'done' || cid === 'готово')) return true;
    return false;
  });

  const columnId = matchedCol?.id ?? (rawId && effectiveColumns.some((c) => c.id === rawId) ? rawId : (effectiveColumns[0]?.id ?? 'backlog'));
  const col = effectiveColumns.find((c) => c.id === columnId);
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

  const skipWorktree = Boolean(raw.skipWorktree ?? raw.skip_worktree ?? false);
  const branch = skipWorktree
    ? (typeof raw.branch === 'string' && raw.branch.trim() !== '' ? raw.branch : undefined)
    : (typeof raw.branch === 'string' && raw.branch.trim() !== '' ? raw.branch : `feat/${raw.id.toLowerCase()}`);
  const worktreePath = skipWorktree
    ? (typeof raw.worktreePath === 'string' && raw.worktreePath.trim() !== ''
        ? raw.worktreePath
        : (typeof raw.worktree_path === 'string' && raw.worktree_path.trim() !== '' ? raw.worktree_path : undefined))
    : (raw.worktreePath || raw.worktree_path || `worktrees/${raw.id.toLowerCase()}`);

  const rawParentId = raw.parentId ?? raw.parent_id ?? undefined;
  const rawTitle = typeof raw.title === 'string' ? raw.title : '';
  const isReviewSubtask = Boolean(
    rawParentId ||
    rawTitle.toLowerCase().includes('ревью') ||
    rawTitle.toLowerCase().includes('review')
  );
  const fallbackPersona = isReviewSubtask ? 'reviewer' : 'developer';
  const rawPersona = typeof raw.persona === 'string' && raw.persona.trim() !== '' ? raw.persona : undefined;
  const effectivePersona = (col?.persona && !isReviewSubtask)
    ? col.persona
    : (rawPersona ?? (col?.persona || fallbackPersona));

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
    boardId: effectiveBoardId,
    persona: effectivePersona,
    branch,
    worktreePath,
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
    currentTokens: typeof raw.currentTokens === 'number'
      ? raw.currentTokens
      : (typeof raw.current_tokens === 'number' ? raw.current_tokens : undefined),
    skipWorktree,
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
    logEvent(
      'DEBUG',
      'BOARD_STORE',
      `syncTasksBackend: syncing ${tasks.length} tasks to save_tasks`,
      JSON.stringify(tasks.map((t) => t.id))
    );
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
        currentTokens: typeof t.currentTokens === 'number' ? t.currentTokens : null,
        createdAt: t.createdAt || Date.now(),
        updatedAt: Date.now(),
        skipWorktree: t.skipWorktree ?? false,
      };
    });
    await invoke('save_tasks', { tasks: backendPayload });
  } catch (err) {
    console.info('Backend save_tasks not available or in web mode:', err);
    logEvent('WARN', 'BOARD_STORE', 'syncTasksBackend: save_tasks failed or in web mode', String(err));
  }
}

async function triggerRunAgent(taskId: string, prompt?: string): Promise<void> {
  // Ensure the task is guaranteed to be saved and known to the backend before running agent (§ Requirement 4)
  const currentTasks = useBoardStore.getState().tasks;
  const taskToRun = currentTasks.find(
    (t) => t.id === taskId || t.id.trim().toLowerCase() === taskId.trim().toLowerCase()
  );
  if (taskToRun) {
    await syncTasksBackend(currentTasks);
  }

  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('run_agent', { taskId, prompt: prompt || null });
  } catch (err) {
    const errMsg = String(err);
    logEvent('ERROR', 'BOARD_STORE', `triggerRunAgent: run_agent failed for task "${taskId}"`, errMsg);
    console.error(`[triggerRunAgent] run_agent failed for task "${taskId}":`, err);

    // Safely update the task in boardStore: revert status from 'running' to 'idle' (or 'error'), set isTimerRunning: false
    const tasksAfterErr = useBoardStore.getState().tasks;
    const reverted = tasksAfterErr.map((t) =>
      t.id === taskId || t.id.trim().toLowerCase() === taskId.trim().toLowerCase()
        ? {
            ...t,
            status: 'idle' as TaskStatus,
            isTimerRunning: false,
            updatedAt: Date.now(),
          }
        : t
    );
    useBoardStore.setState({ tasks: reverted });
    persistTasksToLocalStorage(reverted);
    useDrawerStore.getState().setActiveAgentTask(taskId, false);
    void syncTasksBackend(reverted);

    // Display user notification / alert so the user understands why the agent did not start
    alert(`Не удалось запустить агента для задачи "${taskId}":\n${errMsg}`);
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
  columns: CANONICAL_BOARD_COLUMNS,
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
        columns: CANONICAL_BOARD_COLUMNS,
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
    logEvent('DEBUG', 'BOARD_STORE', 'loadTasks: invoking list_tasks from backend');
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const rawTasks = await invoke<any[]>('list_tasks');
      logEvent('INFO', 'BOARD_STORE', `loadTasks: received ${Array.isArray(rawTasks) ? rawTasks.length : 0} tasks from backend`);
      if (Array.isArray(rawTasks)) {
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
        return;
      }
    } catch (err) {
      console.info('Tauri list_tasks not available or returned empty:', err);
    }

    // Fallback: restore from localStorage backup
    const backupTasks = loadTasksFromLocalStorage();
    if (backupTasks.length > 0) {
      set({ tasks: backupTasks, isLoadingTasks: false });
      return;
    }

    set({ tasks: [], isLoadingTasks: false });
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

    // ZERO INVISIBLE TASKS (§ 2.18 & § 2.22):
    // Move task with target active board id.
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const backendTask = await invoke<any>('move_task_stage', {
        taskId: resolvedTaskId,
        targetStageId,
        boardId: activeBoardId,
      });
      if (backendTask && typeof backendTask.id === 'string') {
        const mapped = mapBackendTaskToKanban(backendTask);
        const prev = get().tasks.find((t) => t.id === resolvedTaskId || t.id.trim().toLowerCase() === resolvedTaskId.trim().toLowerCase()) ?? task;
        // Local-only timer/UX bookkeeping (backend has no timer fields).
        const sourceColId = options?.sourceColumnId ?? prev.columnId;
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
          boardId: activeBoardId || mapped.boardId || prev.boardId || 'development',
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
        const updatedTasks = get().tasks.map((t) =>
          t.id === resolvedTaskId || t.id.trim().toLowerCase() === resolvedTaskId.trim().toLowerCase() ? merged : t
        );
        set({ tasks: updatedTasks });
        persistTasksToLocalStorage(updatedTasks);
        logEvent(
          'INFO',
          'BOARD_STORE',
          `moveTask: backend move_task_stage succeeded for "${resolvedTaskId}"`,
          JSON.stringify({ newStage: merged.columnId, newStatus: merged.status, boardId: merged.boardId })
        );

        // ZERO INVISIBLE TASKS (§ 2.18): If the task moved to a stage on another board,
        // seamlessly switch to that board so the task remains visible to the operator.
        if (merged.boardId && merged.boardId !== get().activeBoardId) {
          const boardExists = get().boards.some((b) => b.id === merged.boardId);
          if (boardExists) {
            await get().setActiveBoard(merged.boardId);
          }
        }

        // Reliable agent autorun upon entering a stage with a persona (§ 2.2 & § 2.32)
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
      // Re-throw so caller (handleDragEnd) can cleanly revert local UI state!
      throw (err instanceof Error ? err : new Error(msg));
    }
  },

  updateTask: (taskId, partial) => {
    set((state) => {
      const updatedTasks = state.tasks.map((t) => (t.id === taskId ? { ...t, ...partial } : t));
      persistTasksToLocalStorage(updatedTasks);
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
        // Never overwrite waiting_subtask or done when requested status is idle!
        if (status === 'idle' && (t.status === 'waiting_subtask' || t.status === 'done')) {
          return t;
        }
        const isTimerRunning = status === 'running' ? t.isTimerRunning : false;
        return { ...t, status, isTimerRunning };
      });
      persistTasksToLocalStorage(updatedTasks);
      // NOTE: NEVER call syncTasksBackend here! Backend owns agent status transitions.
      return { tasks: updatedTasks };
    });
  },

  upsertBackendTask: (rawTask: any) => {
    if (!rawTask || typeof rawTask !== 'object') return;
    const mapped = mapBackendTaskToKanban(rawTask);
    set((state) => {
      const exists = state.tasks.some((t) => t.id === mapped.id);
      const updatedTasks = exists
        ? state.tasks.map((t) => (t.id === mapped.id ? { ...t, ...mapped } : t))
        : [...state.tasks, mapped];

      // Relational symmetry: if this is a subtask, ensure parent task reflects waitingForSubtaskId
      const finalTasks = mapped.parentId
        ? updatedTasks.map((t) => {
            if (t.id === mapped.parentId) {
              const isSubDone = mapped.status === 'done' || mapped.columnId === 'ready';
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
        : updatedTasks;

      persistTasksToLocalStorage(finalTasks);
      return { tasks: finalTasks };
    });
  },

  setBackendTasks: (rawTasks: any[]) => {
    if (!Array.isArray(rawTasks)) return;
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
    set({ tasks: mapped });
    persistTasksToLocalStorage(mapped);
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

    logEvent(
      'INFO',
      'BOARD_STORE',
      `addTask: initiated for title="${title}"`,
      JSON.stringify({ targetColumnId, boardId, skipWorktree, model: input.model })
    );

    // BACKEND-AUTHORITATIVE TASK CREATION:
    // No client ID generation and no optimistic insertion into tasks before backend response.
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const backendTask = await invoke<any>('create_task', {
        title,
        description,
        stageId: targetColumnId,
        boardId,
        skipWorktree,
        model: input.model || undefined,
      });

      if (backendTask && typeof backendTask.id === 'string') {
        const confirmed = mapBackendTaskToKanban(backendTask);
        const currentTasks = get().tasks;
        const nextTasks = [confirmed, ...currentTasks.filter((t) => t.id !== confirmed.id)];

        set({ tasks: nextTasks });
        persistTasksToLocalStorage(nextTasks);

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

    // BACKEND-AUTHORITATIVE SUBTASK CREATION:
    // Call backend create_subtask without optimistic local insertions.
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const backendSubtask = await invoke<any>('create_subtask', {
        parentId,
        targetStageId: targetCol.id,
        title: null,
      });

      if (backendSubtask && typeof backendSubtask.id === 'string') {
        const mapped = mapBackendTaskToKanban(backendSubtask);
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
        const finalTasks = [...updatedTasks.filter((t) => t.id !== mapped.id), mapped];
        set({ tasks: finalTasks });
        persistTasksToLocalStorage(finalTasks);

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

    // Cascade calculation: target task plus all descendants
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

    // BACKEND-AUTHORITATIVE TASK DELETION:
    // Call backend delete_task first. Update store ONLY after backend confirmation.
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

    // Backend confirmed deletion. Now update store state.
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
    set({
      tasks: updatedTasks,
      selectedTaskId: toDelete.has(prevSelectedTaskId ?? '') ? null : prevSelectedTaskId,
    });
    persistTasksToLocalStorage(updatedTasks);

    // Close drawer if active task is deleted
    const drawerActiveTaskId = useDrawerStore.getState().activeTaskId;
    if (drawerActiveTaskId && toDelete.has(drawerActiveTaskId)) {
      useDrawerStore.getState().closeDrawer();
    }

    // ALWAYS clear logs and SQLite dialogue for every deleted ID in drawer store
    for (const id of ids) {
      try {
        await useDrawerStore.getState().clearTaskLogs(id);
      } catch (logErr) {
        console.warn('Failed to clear task logs for deleted task:', id, logErr);
      }
    }
  },

  runTaskAgent: async (taskId) => {
    const { tasks, columns, moveTask } = get();
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

    // Ensure the task is guaranteed to be saved and known to the backend before running agent (§ Requirement 4)
    await syncTasksBackend(get().tasks);

    const taskCol = columns.find((c) => c.id === task.columnId);
    if (taskCol?.persona || taskCol?.autoRun) {
      logEvent('INFO', 'BOARD_STORE', `runTaskAgent: task "${resolvedTaskId}" is in agent column "${taskCol.id}", starting agent`);
      const updatedTasks = get().tasks.map((t) =>
        t.id === resolvedTaskId || t.id.trim().toLowerCase() === resolvedTaskId.trim().toLowerCase()
          ? { ...t, status: 'running' as TaskStatus, isTimerRunning: true, updatedAt: Date.now() }
          : t
      );
      set({ tasks: updatedTasks });
      persistTasksToLocalStorage(updatedTasks);
      await syncTasksBackend(updatedTasks);
      await triggerRunAgent(resolvedTaskId);
    } else {
      // Task sits in a non-agent column: move to the FIRST column with a persona or autoRun
      // (moveTask auto-runs the agent on entry).
      const firstAgentCol = columns.find((c) => c.persona || c.autoRun);
      if (firstAgentCol) {
        logEvent('INFO', 'BOARD_STORE', `runTaskAgent: task "${resolvedTaskId}" in non-agent column "${task.columnId}", moving to "${firstAgentCol.id}"`);
        await moveTask(resolvedTaskId, firstAgentCol.id);
      } else {
        logEvent('WARN', 'BOARD_STORE', `runTaskAgent: no agent column found for task "${resolvedTaskId}"`);
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

import {
  PipelineColumn,
  KanbanTask,
  BoardDef,
  TaskHookStates,
  TaskStatus,
  TaskVerdict,
  BackendTaskDTO,
} from '../../types';

export interface ActiveQuestionInfo {
  taskId: string;
  questionId: string;
  text: string;
  timestamp: number;
}

export interface BoardMetaSlice {
  boards: BoardDef[];
  activeBoardId: string;
  columns: PipelineColumn[];
  isLoadingBoards: boolean;
  isBoardsLoaded: boolean;
  isTauriConnected: boolean;

  setBoards: (boards: BoardDef[]) => void;
  setActiveBoard: (boardId: string) => Promise<void>;
  loadBoards: () => Promise<void>;
  createBoard: (name: string, template?: string) => Promise<BoardDef | null>;
  saveBoard: (board: BoardDef) => Promise<void>;
  deleteBoard: (boardId: string) => Promise<void>;
  saveCurrentBoard: (columns: PipelineColumn[]) => Promise<void>;
  setColumns: (columns: PipelineColumn[]) => void;
  updateColumn: (columnId: string, partial: Partial<PipelineColumn>) => Promise<void>;
  loadColumns: () => Promise<void>;
  addColumn?: (column: PipelineColumn) => Promise<void>;
  deleteColumn?: (columnId: string) => Promise<void>;
  reorderColumns?: (columns: PipelineColumn[]) => Promise<void>;
}

export interface TaskTimersSlice {
  toggleTimer: (taskId: string) => void;
  tickTimers: () => void;
  startTaskTimer: (taskId: string) => void;
  stopTaskTimer: (taskId: string) => void;
  resetTaskTimer: (taskId: string) => void;
}

export interface TasksCrudSlice {
  tasks: KanbanTask[];
  selectedTaskId: string | null;
  hoveredTaskId: string | null;
  searchQuery: string;
  isLoadingTasks: boolean;
  hookStates: Record<string, TaskHookStates>;
  activeQuestions: Record<string, ActiveQuestionInfo>;

  setActiveQuestion: (payload: ActiveQuestionInfo) => void;
  clearActiveQuestion: (taskId: string) => void;
  setHookStart: (payload: {
    taskId: string;
    stageId: string;
    hookType: 'on_enter' | 'on_leave';
    hookName: string;
    description?: string;
  }) => void;
  setHookFinish: (payload: {
    taskId: string;
    stageId: string;
    hookType: 'on_enter' | 'on_leave';
    success: boolean;
    exitCode?: number | null;
    stdout?: string;
    stderr?: string;
    error?: string;
  }) => void;
  clearHookState: (taskId: string, hookType?: 'on_enter' | 'on_leave') => void;
  setTasks: (tasks: KanbanTask[]) => void;
  setSelectedTaskId: (id: string | null) => void;
  setHoveredTaskId: (id: string | null) => void;
  setSearchQuery: (query: string) => void;

  loadTasks: () => Promise<void>;
  moveTask: (
    taskId: string,
    targetColumnId: string,
    options?: { force?: boolean; sourceColumnId?: string }
  ) => Promise<void>;
  reorderTask: (taskId: string, targetStageId: string, newIndex: number) => Promise<void>;
  updateTask: (taskId: string, partial: Partial<KanbanTask>) => void;
  updateTaskStatus: (taskId: string, status: TaskStatus) => void;
  upsertBackendTask: (rawTask: BackendTaskDTO | unknown) => void;
  setBackendTasks: (rawTasks: (BackendTaskDTO | unknown)[]) => void;
  addTask: (input: {
    title: string;
    description?: string;
    stageId?: string;
    boardId?: string;
    model?: string;
    attachments?: string[];
  }) => Promise<KanbanTask | null>;
  createSubtask: (parentId: string, targetColumnId?: string) => Promise<KanbanTask | null>;
  deleteTask: (taskId: string) => Promise<void>;
  setTaskVerdict: (taskId: string, verdict: TaskVerdict, note?: string) => void;
  promoteTaskToNextStage: (taskId: string) => void;
  runTaskAgent: (taskId: string, prompt?: string) => Promise<void>;
}

export type BoardState = BoardMetaSlice & TaskTimersSlice & TasksCrudSlice;

import {
  PipelineStage,
  PipelineColumn,
  KanbanTask,
  BoardDef,
  TaskHookStates,
  TaskStatus,
  TaskVerdict,
  BackendTaskDTO,
  TaskInput,
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
  stages: PipelineStage[];
  /** @deprecated Use stages instead */
  readonly columns: PipelineColumn[];
  isLoadingBoards: boolean;
  isBoardsLoaded: boolean;
  isTauriConnected: boolean;

  setBoards: (boards: BoardDef[]) => void;
  setActiveBoard: (boardId: string) => Promise<void>;
  loadBoards: () => Promise<void>;
  createBoard: (name: string, template?: string) => Promise<BoardDef | null>;
  saveBoard: (board: BoardDef) => Promise<void>;
  deleteBoard: (boardId: string) => Promise<void>;
  saveCurrentBoard: (stages: PipelineStage[]) => Promise<void>;
  setStages: (stages: PipelineStage[]) => void;
  /** @deprecated Use setStages instead */
  setColumns: (columns: PipelineColumn[]) => void;
  updateStage: (stageId: string, partial: Partial<PipelineStage>) => Promise<void>;
  /** @deprecated Use updateStage instead */
  updateColumn: (columnId: string, partial: Partial<PipelineColumn>) => Promise<void>;
  loadStages: () => Promise<void>;
  /** @deprecated Use loadStages instead */
  loadColumns: () => Promise<void>;
  addStage: (stage: PipelineStage) => Promise<void>;
  /** @deprecated Use addStage instead */
  addColumn: (column: PipelineColumn) => Promise<void>;
  deleteStage: (stageId: string) => Promise<void>;
  /** @deprecated Use deleteStage instead */
  deleteColumn: (columnId: string) => Promise<void>;
  reorderStages: (stages: PipelineStage[]) => Promise<void>;
  /** @deprecated Use reorderStages instead */
  reorderColumns: (columns: PipelineColumn[]) => Promise<void>;
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
    targetStageId: string,
    options?: { force?: boolean; sourceStageId?: string; sourceColumnId?: string }
  ) => Promise<void>;
  reorderTask: (taskId: string, targetStageId: string, newIndex: number) => Promise<void>;
  updateTask: (taskId: string, partial: Partial<KanbanTask>) => void;
  updateTaskStatus: (taskId: string, status: TaskStatus) => void;
  upsertBackendTask: (rawTask: BackendTaskDTO | unknown) => void;
  setBackendTasks: (rawTasks: (BackendTaskDTO | unknown)[]) => void;
  addTask: (input: TaskInput) => Promise<KanbanTask | null>;
  createSubtask: (parentId: string, targetStageId?: string) => Promise<KanbanTask | null>;
  deleteTask: (taskId: string) => Promise<void>;
  setTaskVerdict: (taskId: string, verdict: TaskVerdict, note?: string) => void;
  promoteTaskToNextStage: (taskId: string) => void;
  runTaskAgent: (taskId: string, prompt?: string, options?: { skipSync?: boolean }) => Promise<void>;
}

export type BoardState = BoardMetaSlice & TaskTimersSlice & TasksCrudSlice;

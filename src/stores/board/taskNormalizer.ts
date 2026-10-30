import {
  BackendTaskDTO,
  isBackendTaskDTO,
  KanbanTask,
  PipelineColumn,
  TaskPriority,
  TaskStatus,
  TaskVerdict,
  BoardDef,
} from '../../types';
import { CANONICAL_BOARD_COLUMNS, DEFAULT_BOARD } from './boardConstants';
import { logEvent } from '../../lib/logger';
import { useBoardStore } from '../boardStore';

/**
 * Detects if a task belongs to a foreign project outside the current active project root.
 * Tasks with absolute worktree paths that do not start with projectRoot are flagged as foreign.
 */
export function isTaskForeign(
  task: { worktreePath?: string; worktree_path?: string },
  projectRoot?: string
): boolean {
  if (!projectRoot || typeof projectRoot !== 'string') return false;
  const wt = (task.worktreePath || task.worktree_path || '').trim();
  if (!wt) return false;
  // Absolute path check (Windows drive letter C:\ or UNC \\ or Unix /)
  const isAbs = /^(?:[a-zA-Z]:[\\/]|\\\\|\/)/.test(wt);
  if (!isAbs) return false;

  const normWt = wt.replace(/\\/g, '/').toLowerCase().replace(/\/+$/, '');
  const normRoot = projectRoot.replace(/\\/g, '/').toLowerCase().replace(/\/+$/, '');

  return !normWt.startsWith(normRoot + '/') && normWt !== normRoot;
}

export function normalizeKanbanTask(
  raw: BackendTaskDTO | unknown,
  context?: {
    boards?: BoardDef[];
    activeBoardId?: string;
    columns?: PipelineColumn[];
    tasks?: KanbanTask[];
    activeProjectRoot?: string;
  }
): KanbanTask | null {
  if (!isBackendTaskDTO(raw)) return null;

  if (context?.activeProjectRoot && isTaskForeign(raw, context.activeProjectRoot)) {
    return null;
  }

  const storeState = typeof useBoardStore?.getState === 'function' ? useBoardStore.getState() : undefined;
  const boards = context?.boards ?? storeState?.boards ?? [DEFAULT_BOARD];
  const activeBoardId = context?.activeBoardId ?? storeState?.activeBoardId ?? 'development';
  const currentColumns = context?.columns ?? storeState?.columns ?? CANONICAL_BOARD_COLUMNS;
  const existingTasks = context?.tasks ?? storeState?.tasks ?? [];

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
      : createdAt);
  const completedAt = typeof raw.completedAt === 'number' && Number.isFinite(raw.completedAt)
    ? raw.completedAt
    : (typeof raw.completed_at === 'number' && Number.isFinite(raw.completed_at) ? raw.completed_at : undefined);

  const stageStart = stageEnteredAt || createdAt;
  const isDoneTask = status === 'done';
  const finishTime = completedAt || (isDoneTask ? updatedAt : undefined);
  const computedStageDuration = isDoneTask
    ? (typeof raw.activeStageDuration === 'number' && Number.isFinite(raw.activeStageDuration) && raw.activeStageDuration > 0
        ? raw.activeStageDuration
        : (stageStart && finishTime ? Math.max(0, Math.floor((finishTime - stageStart) / 1000)) : 0))
    : (stageStart ? Math.max(0, Math.floor((Date.now() - stageStart) / 1000)) : 0);

  const activeStageDuration = typeof raw.activeStageDuration === 'number' && Number.isFinite(raw.activeStageDuration) && raw.activeStageDuration > 0
    ? raw.activeStageDuration
    : computedStageDuration;

  const computedTotalElapsed = isDoneTask
    ? (typeof raw.totalElapsed === 'number' && Number.isFinite(raw.totalElapsed) && raw.totalElapsed > 0
        ? raw.totalElapsed
        : (createdAt && finishTime ? Math.max(0, Math.floor((finishTime - createdAt) / 1000)) : 0))
    : (createdAt ? Math.max(0, Math.floor((Date.now() - createdAt) / 1000)) : 0);

  const totalElapsed = typeof raw.totalElapsed === 'number' && Number.isFinite(raw.totalElapsed) && raw.totalElapsed > 0
    ? raw.totalElapsed
    : computedTotalElapsed;

  const returnsCount = typeof raw.returnsCount === 'number' && Number.isFinite(raw.returnsCount)
    ? raw.returnsCount
    : 0;

  const rawParentId = (typeof raw.parentId === 'string' && raw.parentId.trim() !== '')
    ? raw.parentId.trim()
    : (typeof raw.parent_id === 'string' && raw.parent_id.trim() !== ''
      ? raw.parent_id.trim()
      : (typeof raw.id === 'string' && raw.id.includes('.') ? raw.id.split('.')[0] : undefined));

  const parentTask = rawParentId ? existingTasks.find((t) => t.id === rawParentId) : undefined;

  const skipWorktree = Boolean(
    raw.skipWorktree ?? raw.skip_worktree ?? raw.skipBacklogHook ?? raw.skip_backlog_hook ?? parentTask?.skipWorktree ?? false
  );
  const branch = (typeof raw.branch === 'string' && raw.branch.trim() !== '')
    ? raw.branch.trim()
    : parentTask?.branch;
  const worktreePath = (typeof raw.worktreePath === 'string' && raw.worktreePath.trim() !== '')
    ? raw.worktreePath.trim()
    : ((typeof raw.worktree_path === 'string' && raw.worktree_path.trim() !== '')
      ? raw.worktree_path.trim()
      : parentTask?.worktreePath);
  const rawPersona = typeof raw.persona === 'string' && raw.persona.trim() !== '' ? raw.persona.trim() : undefined;
  const workingCols = effectiveColumns.filter(
    (c) => !c.isIntake && c.id !== 'backlog' && !c.terminal && c.id !== 'ready' && c.id !== 'done'
  );

  const prevStageId = typeof raw.previousStageId === 'string' && raw.previousStageId.trim() !== ''
    ? raw.previousStageId.trim()
    : (typeof raw.previous_stage_id === 'string' && raw.previous_stage_id.trim() !== ''
      ? raw.previous_stage_id.trim()
      : (typeof raw.fromStage === 'string' && raw.fromStage.trim() !== ''
        ? raw.fromStage.trim()
        : (typeof raw.from_stage === 'string' && raw.from_stage.trim() !== '' ? raw.from_stage.trim() : undefined)));
  const prevCol = prevStageId ? effectiveColumns.find((c) => c.id === prevStageId) : undefined;

  const isTerminalCol = Boolean(col?.terminal || columnId === 'ready' || columnId === 'done');

  let effectivePersona: string;
  if (col?.persona) {
    effectivePersona = col.persona;
  } else if (isTerminalCol && !rawParentId) {
    // Root parent task in terminal column represents the first working pipeline stage (e.g. planner)
    effectivePersona = workingCols[0]?.persona ?? rawPersona ?? 'general';
  } else if (isTerminalCol && rawParentId) {
    // Child subtask in terminal column (if ever moved there) resolves to its previous stage persona or raw persona
    effectivePersona = prevCol?.persona ?? rawPersona ?? workingCols[0]?.persona ?? 'general';
  } else {
    // Columns without explicit persona (e.g. intake / backlog)
    effectivePersona = rawPersona ?? workingCols[0]?.persona ?? 'general';
  }

  const isTerminalTask = columnId === 'ready' || status === 'done';
  const effectiveWaitingForSubtaskId = isTerminalTask
    ? undefined
    : (raw.waitingForSubtaskId ?? raw.waiting_for_subtask_id ?? undefined);

  return {
    id: raw.id,
    title: typeof raw.title === 'string' && raw.title.trim() !== '' ? raw.title : raw.id,
    description: typeof raw.description === 'string' && raw.description !== '' ? raw.description : undefined,
    attachments: Array.isArray(raw.attachments) ? raw.attachments.filter((a: unknown) => typeof a === 'string') : undefined,
    createdAt,
    updatedAt,
    completedAt,
    stageEnteredAt,
    priority: (raw.priority as TaskPriority) || 'high',
    columnId,
    boardId: effectiveBoardId,
    persona: effectivePersona,
    branch,
    worktreePath,
    status: isTerminalTask && status === 'waiting_subtask' ? ('done' as TaskStatus) : status,
    activeStageDuration,
    stageDurations,
    totalElapsed,
    isTimerRunning: typeof raw.isTimerRunning === 'boolean' ? raw.isTimerRunning : status === 'running',
    returnsCount,
    customFlow: raw.customFlow || raw.custom_flow,
    verdict: (raw.verdict as TaskVerdict) || 'NONE',
    verdictNote: raw.verdictNote,
    model: raw.model || undefined,
    parentId: raw.parentId ?? raw.parent_id ?? undefined,
    subtaskIndex: raw.subtaskIndex ?? raw.subtask_index ?? undefined,
    waitingForSubtaskId: effectiveWaitingForSubtaskId,
    todos: Array.isArray(raw.todos)
      ? raw.todos.map((item, idx: number) => ({
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
    position: typeof raw.position === 'number' && Number.isFinite(raw.position) ? raw.position : undefined,
    previousStageId: typeof raw.previousStageId === 'string' && raw.previousStageId.trim() !== ''
      ? raw.previousStageId.trim()
      : (typeof raw.previous_stage_id === 'string' && raw.previous_stage_id.trim() !== ''
        ? raw.previous_stage_id.trim()
        : (typeof raw.fromStage === 'string' && raw.fromStage.trim() !== ''
          ? raw.fromStage.trim()
          : (typeof raw.from_stage === 'string' && raw.from_stage.trim() !== '' ? raw.from_stage.trim() : undefined))),
    hookStates: (raw.hookStates || raw.hook_states) && typeof (raw.hookStates || raw.hook_states) === 'object'
      ? (raw.hookStates || raw.hook_states)
      : undefined,
  };
}

export function mapBackendTaskToKanban(
  t: BackendTaskDTO | unknown,
  context?: {
    boards?: BoardDef[];
    activeBoardId?: string;
    columns?: PipelineColumn[];
    tasks?: KanbanTask[];
    activeProjectRoot?: string;
  }
): KanbanTask | null {
  const normalized = normalizeKanbanTask(t, context);
  if (normalized) return normalized;

  logEvent(
    'WARN',
    'BOARD_STORE',
    'Dropped invalid backend task payload without valid ID',
    typeof t === 'object' && t !== null ? JSON.stringify(t).slice(0, 300) : String(t)
  );
  return null;
}

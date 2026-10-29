import { KanbanTask, TodoItem } from '../../types';

export interface KanbanCardProps {
  task: KanbanTask;
  index: number;
}

export function areTodosEqual(a?: TodoItem[], b?: TodoItem[]): boolean {
  if (a === b) return true;
  if (!a || !b) return false;
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    const itemA = a[i];
    const itemB = b[i];
    if (!itemA || !itemB) return false;
    if (
      itemA.id !== itemB.id ||
      itemA.status !== itemB.status ||
      itemA.content !== itemB.content
    ) {
      return false;
    }
  }
  return true;
}

export function areCardPropsEqual(prev: KanbanCardProps, next: KanbanCardProps): boolean {
  if (prev.index !== next.index) return false;
  if (prev.task === next.task) return true;

  const a = prev.task;
  const b = next.task;

  return (
    a.id === b.id &&
    a.title === b.title &&
    a.description === b.description &&
    a.columnId === b.columnId &&
    a.boardId === b.boardId &&
    a.status === b.status &&
    a.priority === b.priority &&
    a.branch === b.branch &&
    a.worktreePath === b.worktreePath &&
    a.persona === b.persona &&
    a.verdict === b.verdict &&
    a.verdictNote === b.verdictNote &&
    a.model === b.model &&
    a.parentId === b.parentId &&
    a.subtaskIndex === b.subtaskIndex &&
    a.waitingForSubtaskId === b.waitingForSubtaskId &&
    a.handoverSummary === b.handoverSummary &&
    a.activeStageDuration === b.activeStageDuration &&
    a.totalElapsed === b.totalElapsed &&
    a.completedAt === b.completedAt &&
    a.stageEnteredAt === b.stageEnteredAt &&
    a.updatedAt === b.updatedAt &&
    a.createdAt === b.createdAt &&
    a.previousStageId === b.previousStageId &&
    a.isTimerRunning === b.isTimerRunning &&
    a.returnsCount === b.returnsCount &&
    a.attachments === b.attachments &&
    a.hookStates === b.hookStates &&
    areTodosEqual(a.todos, b.todos)
  );
}

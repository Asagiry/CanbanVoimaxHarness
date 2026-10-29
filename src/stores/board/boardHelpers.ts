import { KanbanTask } from '../../types';

export function sortTasksByPosition(a: KanbanTask, b: KanbanTask): number {
  const posA = typeof a.position === 'number' && Number.isFinite(a.position) ? a.position : 0;
  const posB = typeof b.position === 'number' && Number.isFinite(b.position) ? b.position : 0;
  if (posA !== posB) {
    return posA - posB;
  }
  return (a.createdAt || 0) - (b.createdAt || 0);
}

export function isDoneColumnDef(col?: { id: string; title?: string; terminal?: boolean } | null): boolean {
  if (!col) return false;
  return Boolean(
    col.terminal ||
    col.id === 'ready' ||
    col.title?.toLowerCase().includes('готов') ||
    col.title?.toLowerCase().includes('done')
  );
}

export function getTaskCompletionTime(task: KanbanTask): number {
  return task.completedAt || task.stageEnteredAt || task.updatedAt || task.createdAt || 0;
}

export function sortDoneTasksByCompletionDateDesc(a: KanbanTask, b: KanbanTask): number {
  const timeA = getTaskCompletionTime(a);
  const timeB = getTaskCompletionTime(b);
  if (timeB !== timeA) {
    return timeB - timeA;
  }
  return (b.createdAt || 0) - (a.createdAt || 0);
}

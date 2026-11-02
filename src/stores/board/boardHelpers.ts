import { KanbanTask } from '../../types';

export function sortTasksByPosition(a: KanbanTask, b: KanbanTask): number {
  const posA = typeof a.position === 'number' && Number.isFinite(a.position) ? a.position : 0;
  const posB = typeof b.position === 'number' && Number.isFinite(b.position) ? b.position : 0;
  if (posA !== posB) {
    return posA - posB;
  }
  return (a.createdAt || 0) - (b.createdAt || 0);
}

export function isDoneStageDef(stage?: { id: string; title?: string; terminal?: boolean } | null): boolean {
  if (!stage) return false;
  return Boolean(
    stage.terminal ||
    stage.id === 'ready' ||
    stage.title?.toLowerCase().includes('готов') ||
    stage.title?.toLowerCase().includes('done')
  );
}

export const isDoneColumnDef = isDoneStageDef;

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

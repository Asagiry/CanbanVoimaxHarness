import type { KanbanTask } from '../../types';

function rootOf(id: string): string {
  return id.split('.')[0].toLowerCase();
}

export function getRootTaskId(task: Pick<KanbanTask, 'id' | 'parentId'>): string {
  return task.parentId ?? task.id.split('.')[0];
}

export function isSameFamily(aId: string, bId: string): boolean {
  return rootOf(aId) === rootOf(bId);
}

export function isFamilyHighlighted(
  task: Pick<KanbanTask, 'id' | 'parentId'>,
  hoveredTaskId: string | null | undefined
): boolean {
  if (!hoveredTaskId) return false;
  return isSameFamily(task.id, hoveredTaskId);
}

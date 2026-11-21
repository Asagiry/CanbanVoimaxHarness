import { describe, it, expect } from 'vitest';
import { toUnfilteredIndex } from '../useBoardDragDrop';
import type { KanbanTask } from '../../../types';

const tasks = (...ids: string[]) => ids.map((id) => ({ id }) as KanbanTask);

describe('toUnfilteredIndex', () => {
  const full = tasks('a', 'b', 'c', 'd', 'e');

  it('returns the drop index unchanged when no search filter is active', () => {
    expect(toUnfilteredIndex(full, null, 3)).toBe(3);
  });

  it('places the card before the visible card it was dropped onto', () => {
    expect(toUnfilteredIndex(tasks('b', 'd'), full, 1)).toBe(3);
    expect(toUnfilteredIndex(tasks('b', 'd'), full, 0)).toBe(1);
  });

  it('places the card right after the last visible card when dropped at the end', () => {
    expect(toUnfilteredIndex(tasks('b', 'd'), full, 2)).toBe(4);
  });

  it('appends to the column when no card matches the search', () => {
    expect(toUnfilteredIndex([], full, 0)).toBe(5);
  });
});

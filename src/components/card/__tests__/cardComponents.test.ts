import { describe, it, expect } from 'vitest';
import { areTodosEqual, areCardPropsEqual, KanbanCardProps } from '../cardComparators';
import { formatCardDate, formatStageDuration, formatTimerMMSS } from '../CardTimerBadge';
import { KanbanTask } from '../../../types';

const createSampleTask = (overrides?: Partial<KanbanTask>): KanbanTask => ({
  id: 'CVH-001',
  title: 'Task A',
  description: 'Sample description',
  priority: 'medium',
  columnId: 'dev',
  boardId: 'development',
  persona: 'cvh-dev',
  status: 'idle',
  activeStageDuration: 0,
  stageDurations: {},
  totalElapsed: 0,
  isTimerRunning: false,
  returnsCount: 0,
  createdAt: 1000,
  updatedAt: 1000,
  ...overrides,
});

describe('Card Component Logic & Comparators', () => {
  describe('areTodosEqual', () => {
    it('returns true for identical todo items', () => {
      const a = [{ id: '1', content: 'Do X', status: 'pending' as const }];
      const b = [{ id: '1', content: 'Do X', status: 'pending' as const }];
      expect(areTodosEqual(a, b)).toBe(true);
    });

    it('returns false when status or content differs', () => {
      const a = [{ id: '1', content: 'Do X', status: 'pending' as const }];
      const b = [{ id: '1', content: 'Do X', status: 'completed' as const }];
      expect(areTodosEqual(a, b)).toBe(false);
    });
  });

  describe('areCardPropsEqual', () => {
    it('detects equal props correctly', () => {
      const task = createSampleTask({ id: 'CVH-001', title: 'Task A' });
      const p1: KanbanCardProps = { task, index: 0 };
      const p2: KanbanCardProps = { task: { ...task }, index: 0 };
      expect(areCardPropsEqual(p1, p2)).toBe(true);
    });

    it('detects different index as unequal', () => {
      const task = createSampleTask({ id: 'CVH-001', title: 'Task A' });
      expect(areCardPropsEqual({ task, index: 0 }, { task, index: 1 })).toBe(false);
    });
  });

  describe('CardTimerBadge formatters', () => {
    it('formats MM:SS correctly', () => {
      expect(formatTimerMMSS(65)).toBe('01:05');
      expect(formatTimerMMSS(3665)).toBe('1:01:05');
    });

    it('formats stage duration in en and ru', () => {
      expect(formatStageDuration(45, 'en')).toBe('45s');
      expect(formatStageDuration(45, 'ru')).toBe('45с');
      expect(formatStageDuration(120, 'en')).toBe('2m');
      expect(formatStageDuration(120, 'ru')).toBe('2м');
    });

    it('formats card date', () => {
      expect(formatCardDate(undefined)).toBe('');
      const ts = new Date('2026-05-10T14:30:00').getTime();
      const formatted = formatCardDate(ts);
      expect(formatted).toContain('10.05');
    });
  });
});

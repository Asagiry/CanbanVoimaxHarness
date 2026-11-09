import { describe, it, expect } from 'vitest';
import {
  isOldActivity,
  isAutoCollapsible,
  resolveIsCollapsed,
  applyToggle,
  activityCollapseKey,
  OLD_ACTIVITY_COLLAPSE_THRESHOLD,
} from '../useActivityScroll';

const empty = { collapsedOutputs: new Set<string>(), expandedOverrides: new Set<string>() };

describe('isOldActivity', () => {
  it('keeps threshold at 10', () => {
    expect(OLD_ACTIVITY_COLLAPSE_THRESHOLD).toBe(10);
  });

  it('marks idx < total - 10 as old', () => {
    expect(isOldActivity(0, 50)).toBe(true);
    expect(isOldActivity(39, 50)).toBe(true);
    expect(isOldActivity(40, 50)).toBe(false);
    expect(isOldActivity(49, 50)).toBe(false);
  });
});

describe('isAutoCollapsible', () => {
  it('is true for tool activity', () => {
    expect(isAutoCollapsible({ type: 'tool' })).toBe(true);
    expect(isAutoCollapsible('tool')).toBe(true);
  });

  it('is false for thought and output', () => {
    expect(isAutoCollapsible({ type: 'thought' })).toBe(false);
    expect(isAutoCollapsible({ type: 'output', isFinal: true })).toBe(false);
    expect(isAutoCollapsible({ type: 'output', isFinal: false })).toBe(false);
    expect(isAutoCollapsible('thought')).toBe(false);
  });

  it('is false for all other types', () => {
    for (const type of [
      'question',
      'compacted',
      'system',
      'error',
      'user',
      'steer',
      'summary',
      'reject',
      'agent',
      'prompt',
    ] as const) {
      expect(isAutoCollapsible({ type })).toBe(false);
    }
    expect(isAutoCollapsible(undefined)).toBe(false);
    expect(isAutoCollapsible(null)).toBe(false);
  });
});

describe('resolveIsCollapsed', () => {
  it('collapses tools by default regardless of index or age', () => {
    expect(resolveIsCollapsed({ id: 't1', idx: 0, total: 50, activity: { type: 'tool' }, ...empty })).toBe(true);
    expect(resolveIsCollapsed({ id: 't2', idx: 40, total: 50, activity: { type: 'tool' }, ...empty })).toBe(true);
    expect(resolveIsCollapsed({ id: 't3', idx: 49, total: 50, activity: { type: 'tool' }, ...empty })).toBe(true);
    expect(resolveIsCollapsed({ id: 't4', idx: 0, total: 1, activity: 'tool', ...empty })).toBe(true);
  });

  it('keeps thought, agent explanation, and answers expanded by default', () => {
    expect(resolveIsCollapsed({ id: 'th1', idx: 0, total: 50, activity: { type: 'thought' }, ...empty })).toBe(false);
    expect(resolveIsCollapsed({ id: 'o1', idx: 0, total: 50, activity: { type: 'output', isFinal: true }, ...empty })).toBe(false);
    expect(resolveIsCollapsed({ id: 'o2', idx: 0, total: 50, activity: { type: 'output', isFinal: false }, ...empty })).toBe(false);
    expect(resolveIsCollapsed({ id: 'o3', idx: 49, total: 50, activity: { type: 'output' }, ...empty })).toBe(false);
  });

  it('does not auto-collapse other types at idx=0 with total=50', () => {
    for (const type of ['question', 'compacted', 'system', 'error', 'user', 'steer', 'summary', 'reject', 'agent'] as const) {
      expect(resolveIsCollapsed({ id: 'x', idx: 0, total: 50, activity: { type }, ...empty })).toBe(false);
    }
  });

  it('manual collapsedOutputs wins for thought', () => {
    expect(
      resolveIsCollapsed({
        id: 'th1',
        idx: 0,
        total: 50,
        activity: { type: 'thought' },
        collapsedOutputs: new Set(['th1']),
        expandedOverrides: new Set(),
      })
    ).toBe(true);
  });

  it('expandedOverrides wins over auto-collapse for tool', () => {
    expect(
      resolveIsCollapsed({
        id: 't1',
        idx: 0,
        total: 50,
        activity: { type: 'tool' },
        collapsedOutputs: new Set(),
        expandedOverrides: new Set(['t1']),
      })
    ).toBe(false);
  });

  it('expandedOverrides wins over collapsedOutputs', () => {
    expect(
      resolveIsCollapsed({
        id: 't1',
        idx: 0,
        total: 50,
        activity: { type: 'tool' },
        collapsedOutputs: new Set(['t1']),
        expandedOverrides: new Set(['t1']),
      })
    ).toBe(false);
  });
});

describe('applyToggle', () => {
  it('manual toggle of a tool collapses it, second toggle expands it', () => {
    const first = applyToggle(false, 't1', new Set(), new Set());
    expect(first.collapsedOutputs.has('t1')).toBe(true);
    expect(first.expandedOverrides.has('t1')).toBe(false);
    expect(resolveIsCollapsed({ id: 't1', idx: 0, total: 50, activity: { type: 'tool' }, ...first })).toBe(true);

    const second = applyToggle(true, 't1', first.collapsedOutputs, first.expandedOverrides);
    expect(second.collapsedOutputs.has('t1')).toBe(false);
    expect(second.expandedOverrides.has('t1')).toBe(true);
    expect(resolveIsCollapsed({ id: 't1', idx: 0, total: 50, activity: { type: 'tool' }, ...second })).toBe(false);
  });

  it('toggling an auto-collapsed tool expands it and stays expanded', () => {
    const next = applyToggle(true, 't1', new Set(), new Set());
    expect(resolveIsCollapsed({ id: 't1', idx: 0, total: 50, activity: { type: 'tool' }, ...next })).toBe(false);
  });

  it('rapid double-toggle returns to original state (no stale closure)', () => {
    const first = applyToggle(false, 't1', new Set(), new Set());
    const second = applyToggle(true, 't1', first.collapsedOutputs, first.expandedOverrides);
    expect(resolveIsCollapsed({ id: 't1', idx: 0, total: 50, activity: { type: 'tool' }, ...second })).toBe(false);
  });

  it('two tools sharing activity id but different tool ids toggle independently via composite key', () => {
    const keyA = activityCollapseKey({ id: 'shared', type: 'tool', tool: { id: 'call-a' } } as never, 0);
    const keyB = activityCollapseKey({ id: 'shared', type: 'tool', tool: { id: 'call-b' } } as never, 1);
    expect(keyA).not.toBe(keyB);
    const afterA = applyToggle(false, keyA, new Set(), new Set());
    const afterB = applyToggle(true, keyB, afterA.collapsedOutputs, afterA.expandedOverrides);
    expect(resolveIsCollapsed({ id: keyA, idx: 0, total: 50, activity: { type: 'tool' }, ...afterB })).toBe(true);
    expect(resolveIsCollapsed({ id: keyB, idx: 49, total: 50, activity: { type: 'tool' }, ...afterB })).toBe(false);
  });

  it('task switch reset (empty sets) restores defaults: old tool collapsed, old thought expanded', () => {
    const reset = { collapsedOutputs: new Set<string>(), expandedOverrides: new Set<string>() };
    expect(resolveIsCollapsed({ id: 't1', idx: 0, total: 50, activity: { type: 'tool' }, ...reset })).toBe(true);
    expect(resolveIsCollapsed({ id: 'th1', idx: 0, total: 50, activity: { type: 'thought' }, ...reset })).toBe(false);
  });
});

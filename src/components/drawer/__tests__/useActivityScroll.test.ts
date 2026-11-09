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
  it('is true only for thought and output', () => {
    expect(isAutoCollapsible({ type: 'thought' })).toBe(true);
    expect(isAutoCollapsible({ type: 'output' })).toBe(true);
    expect(isAutoCollapsible('thought')).toBe(true);
    expect(isAutoCollapsible('output')).toBe(true);
  });

  it('is false for tool and all other types', () => {
    for (const type of [
      'tool',
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
  it('does not auto-collapse a tool at idx=0 with total=50', () => {
    expect(resolveIsCollapsed({ id: 't1', idx: 0, total: 50, activity: { type: 'tool' }, ...empty })).toBe(false);
  });

  it('auto-collapses thought and output at idx=0 with total=50', () => {
    expect(resolveIsCollapsed({ id: 'th1', idx: 0, total: 50, activity: { type: 'thought' }, ...empty })).toBe(true);
    expect(resolveIsCollapsed({ id: 'o1', idx: 0, total: 50, activity: { type: 'output' }, ...empty })).toBe(true);
  });

  it('does not auto-collapse recent thought/output', () => {
    expect(resolveIsCollapsed({ id: 'th1', idx: 49, total: 50, activity: { type: 'thought' }, ...empty })).toBe(false);
    expect(resolveIsCollapsed({ id: 'o1', idx: 40, total: 50, activity: { type: 'output' }, ...empty })).toBe(false);
  });

  it('does not auto-collapse other types at idx=0 with total=50', () => {
    for (const type of ['question', 'compacted', 'system', 'error', 'user', 'steer', 'summary', 'reject', 'agent'] as const) {
      expect(resolveIsCollapsed({ id: 'x', idx: 0, total: 50, activity: { type }, ...empty })).toBe(false);
    }
  });

  it('manual collapsedOutputs wins for tools', () => {
    expect(
      resolveIsCollapsed({
        id: 't1',
        idx: 0,
        total: 50,
        activity: { type: 'tool' },
        collapsedOutputs: new Set(['t1']),
        expandedOverrides: new Set(),
      })
    ).toBe(true);
  });

  it('expandedOverrides wins over auto-collapse for thought/output', () => {
    expect(
      resolveIsCollapsed({
        id: 'th1',
        idx: 0,
        total: 50,
        activity: { type: 'thought' },
        collapsedOutputs: new Set(),
        expandedOverrides: new Set(['th1']),
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

  it('toggling an auto-collapsed thought expands it and stays expanded', () => {
    const next = applyToggle(true, 'th1', new Set(), new Set());
    expect(resolveIsCollapsed({ id: 'th1', idx: 0, total: 50, activity: { type: 'thought' }, ...next })).toBe(false);
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
    expect(resolveIsCollapsed({ id: keyA, idx: 0, total: 50, activity: { type: 'tool' }, ...afterA })).toBe(true);
    expect(resolveIsCollapsed({ id: keyB, idx: 1, total: 50, activity: { type: 'tool' }, ...afterA })).toBe(false);
  });

  it('task switch reset (empty sets) restores defaults: tool expanded, old thought collapsed', () => {
    const reset = { collapsedOutputs: new Set<string>(), expandedOverrides: new Set<string>() };
    expect(resolveIsCollapsed({ id: 't1', idx: 0, total: 50, activity: { type: 'tool' }, ...reset })).toBe(false);
    expect(resolveIsCollapsed({ id: 'th1', idx: 0, total: 50, activity: { type: 'thought' }, ...reset })).toBe(true);
  });
});

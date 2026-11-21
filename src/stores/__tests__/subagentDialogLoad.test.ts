import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useDrawerStore } from '../drawerStore';
import type { DrawerActivityItem } from '../drawer/types';

const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const item = (id: string, content: string): DrawerActivityItem =>
  ({ id, type: 'output', content, timestamp: new Date().toISOString() }) as DrawerActivityItem;

describe('loadSubagentDialog', () => {
  const subId = 'CVH-1.sub-a';

  beforeEach(() => {
    vi.clearAllMocks();
    useDrawerStore.setState({ activityFeed: {}, toolIdToSubagentId: {}, subagentIdToToolId: {} });
  });

  it('keeps live events that arrive while the snapshot is loading', async () => {
    useDrawerStore.setState({ activityFeed: { [subId]: [item('stale', 'stale in-memory copy')] } });
    mockInvoke.mockImplementation(async () => {
      useDrawerStore.setState((s) => ({
        activityFeed: { ...s.activityFeed, [subId]: [...(s.activityFeed[subId] || []), item('live', 'arrived during load')] },
      }));
      return [item('persisted', 'from database')];
    });

    await useDrawerStore.getState().loadSubagentDialog('CVH-1', subId);

    expect(useDrawerStore.getState().activityFeed[subId].map((it) => it.id)).toEqual(['persisted', 'live']);
  });
});

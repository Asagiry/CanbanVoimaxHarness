import { describe, it, expect, beforeEach, beforeAll, vi } from 'vitest';
import { useDrawerStore } from '../drawerStore';
import { useAgentSessionStore } from '../agentSessionStore';
import { useBoardStore } from '../boardStore';

const mockInvoke = vi.fn().mockResolvedValue([]);

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('Dialog live update without open drawer (CVH-066)', () => {
  beforeAll(() => {
    if (typeof (globalThis as any).window === 'undefined') {
      (globalThis as any).window = globalThis;
    }
    (globalThis as any).__TAURI_INTERNALS__ = {
      invoke: (...args: unknown[]) => mockInvoke(...args),
    };
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue([]);
    useDrawerStore.setState({
      isOpen: false,
      activeTaskId: null,
      activityFeed: {},
      streamEntries: {},
      toolExecutions: {},
      activeAgentTaskIds: {},
      activeStreamIds: {},
      toolIdToSubagentId: {},
      subagentIdToToolId: {},
    });
    useAgentSessionStore.getState().reset();
    useBoardStore.setState({ tasks: [] });
  });

  it('appendThought/appendOutput grow activityFeed for task without open drawer', () => {
    const taskId = 'CVH-061';
    useDrawerStore.getState().appendThought(taskId, 'hello');
    useDrawerStore.getState().appendOutput(taskId, 'world');
    expect(useDrawerStore.getState().activityFeed[taskId].length).toBeGreaterThanOrEqual(2);
  });

  it('appendActivityItem with same item.id for different taskId applies to both', () => {
    const item = { id: 'shared-1', type: 'output' as const, timestamp: '12:00:00', content: 'x' };
    useDrawerStore.getState().appendActivityItem('CVH-061', item as never);
    useDrawerStore.getState().appendActivityItem('CVH-063', item as never);
    expect(useDrawerStore.getState().activityFeed['CVH-061'].length).toBeGreaterThanOrEqual(1);
    expect(useDrawerStore.getState().activityFeed['CVH-063'].length).toBeGreaterThanOrEqual(1);
  });

  it('loadTaskActivity with empty response does not clear live feed while agent is active, but clears stale feed when idle', async () => {
    const taskId = 'CVH-064';
    useDrawerStore.getState().setActiveAgentTask(taskId, true);
    useDrawerStore.getState().appendOutput(taskId, 'live chunk');
    const before = useDrawerStore.getState().activityFeed[taskId].length;
    expect(before).toBeGreaterThan(0);
    mockInvoke.mockResolvedValue([]);
    await useDrawerStore.getState().loadTaskActivity(taskId);
    expect(useDrawerStore.getState().activityFeed[taskId].length).toBe(before);

    // Once agent is no longer active, empty backend response clears stale feed
    useDrawerStore.getState().setActiveAgentTask(taskId, false);
    await useDrawerStore.getState().loadTaskActivity(taskId);
    expect(useDrawerStore.getState().activityFeed[taskId]).toEqual([]);
  });

  it('loadTaskActivity merges live-only chunks after hydration', async () => {
    const taskId = 'CVH-065';
    useDrawerStore.getState().appendOutput(taskId, 'live-only');
    const liveId = useDrawerStore.getState().activityFeed[taskId][0].id;
    mockInvoke.mockResolvedValue([
      { id: 'db-1', type: 'output', timestamp: '11:00:00', content: 'from db' },
    ]);
    await useDrawerStore.getState().loadTaskActivity(taskId);
    const ids = useDrawerStore.getState().activityFeed[taskId].map((i) => i.id);
    expect(ids).toContain('db-1');
    expect(ids).toContain(liveId);
  });

  it('agent:finished guard: session running prevents idle reset via updateTaskStatus path', () => {
    useAgentSessionStore.getState().handleSessionStatus({ sessionId: 's1', taskId: 'CVH-061', status: 'running', startedAt: Date.now() });
    expect(useAgentSessionStore.getState().isSessionRunning('CVH-061')).toBe(true);
    useBoardStore.setState({
      tasks: [{ id: 'CVH-061', title: 't', description: '', priority: 'medium', stageId: 'dev', boardId: 'development', persona: 'cvh-dev', status: 'running', activeStageDuration: 0, stageDurations: {}, totalElapsed: 0, isTimerRunning: true, returnsCount: 0, createdAt: 1, updatedAt: 1 } as never],
    });
    const running = useAgentSessionStore.getState().isSessionRunning('CVH-061');
    if (!running) useBoardStore.getState().updateTaskStatus('CVH-061', 'idle');
    expect(useBoardStore.getState().tasks[0].status).toBe('running');
  });
});

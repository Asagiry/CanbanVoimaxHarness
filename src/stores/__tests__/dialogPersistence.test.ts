import { describe, it, expect, beforeEach, beforeAll, vi } from 'vitest';
import { useDrawerStore } from '../drawerStore';
import type { DrawerActivityItem } from '../drawer/types';

const mockInvoke = vi.fn().mockResolvedValue([]);

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('Dialog Persistence & Stream Unification (CVH-051)', () => {
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
      toolIdToSubagentId: {},
    });
  });

  it('deletes from all client collections only after persistence succeeds', async () => {
    const taskId = 'DELETE-1';
    const tool = { id: 'call-1', toolName: 'read_file', status: 'success' as const, timestamp: '', durationMs: 0, params: {} };
    useDrawerStore.setState({
      activityFeed: { [taskId]: [{ id: 'call-1', type: 'tool', timestamp: '', tool }] },
      streamEntries: { [taskId]: { thoughts: [], outputs: [{ id: 'call-1', type: 'output', timestamp: '', content: 'result' }] } },
      toolExecutions: { [taskId]: [tool] },
    });
    mockInvoke.mockImplementation(async (command) => { if (command === 'delete_dialog_item') throw new Error('disk failure'); return []; });
    await expect(useDrawerStore.getState().deleteDialogItem!(taskId, 'call-1')).rejects.toThrow('disk failure');
    expect(useDrawerStore.getState().activityFeed[taskId]).toHaveLength(1);
    mockInvoke.mockResolvedValue([]);
    await useDrawerStore.getState().deleteDialogItem!(taskId, 'call-1');
    expect(useDrawerStore.getState().activityFeed[taskId]).toEqual([]);
    expect(useDrawerStore.getState().streamEntries[taskId].outputs).toEqual([]);
    expect(useDrawerStore.getState().toolExecutions[taskId]).toEqual([]);
  });

  it('removes linked question and tool records returned by the backend', async () => {
    const tool = { id: 'call-1', toolName: 'question', status: 'success' as const, timestamp: '', durationMs: 0, params: {} };
    useDrawerStore.setState({
      activityFeed: { TASK: [{ id: 'q-1', type: 'question', timestamp: '', tool: { ...tool, id: 'question-1' } }, { id: 'call-1', type: 'tool', timestamp: '', tool }] },
      toolExecutions: { TASK: [tool] },
    });
    mockInvoke.mockImplementation(async (command) => command === 'delete_dialog_item' ? ['q-1', 'question-1', 'call-1'] : []);
    await useDrawerStore.getState().deleteDialogItem!('TASK', 'q-1');
    expect(useDrawerStore.getState().activityFeed.TASK).toEqual([]);
    expect(useDrawerStore.getState().toolExecutions.TASK).toEqual([]);
  });

  it('refuses deletion while running and uses the subagent dialogue when stopped', async () => {
    useDrawerStore.setState({
      activeTaskId: 'PARENT', toolIdToSubagentId: { alias: 'child' }, activeAgentTaskIds: { child: true },
      activityFeed: { child: [], alias: [{ id: 'out-1', type: 'output', timestamp: '', content: 'answer' }] },
    });
    await expect(useDrawerStore.getState().deleteDialogItem!('child', 'out-1')).rejects.toThrow('Stop the agent');
    expect(mockInvoke).not.toHaveBeenCalled();
    useDrawerStore.setState({ activeAgentTaskIds: {} });
    const items = useDrawerStore.getState().activityFeed.alias;
    await useDrawerStore.getState().deleteDialogItem!('child', 'out-1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_dialog_item', { taskId: 'PARENT:sub:child', itemId: 'out-1', items });
    expect(useDrawerStore.getState().activityFeed.alias).toEqual([]);
  });

  it('markFinalResponse marks isFinal without duplicating streamed text into a double response', () => {
    const store = useDrawerStore.getState();
    const taskId = 'CVH-051-TEST';

    // 1. Simulate tokens being streamed
    store.appendOutput(taskId, 'Here is the comprehensive report:\n1. Done\n2. Tested');

    const feedBefore = useDrawerStore.getState().activityFeed[taskId];
    expect(feedBefore).toHaveLength(1);
    expect(feedBefore[0].content).toBe('Here is the comprehensive report:\n1. Done\n2. Tested');
    expect(feedBefore[0].isFinal).toBeUndefined();

    // 2. Mark final response with the complete response
    store.markFinalResponse(taskId, 'Here is the comprehensive report:\n1. Done\n2. Tested');

    const feedAfter = useDrawerStore.getState().activityFeed[taskId];
    // Must remain exactly 1 card without repeating the content
    expect(feedAfter).toHaveLength(1);
    expect(feedAfter[0].content).toBe('Here is the comprehensive report:\n1. Done\n2. Tested');
    expect(feedAfter[0].isFinal).toBe(true);
  });

  it('appendActivityItem cleanly adds and deduplicates task:dialog items from backend', () => {
    const store = useDrawerStore.getState();
    const taskId = 'CVH-051-TEST';

    const summaryItem: DrawerActivityItem = {
      id: 'summary-uuid-1',
      type: 'summary',
      timestamp: '12:00:00',
      content: '### Саммари этапа: done',
    };

    store.appendActivityItem(taskId, summaryItem);
    expect(useDrawerStore.getState().activityFeed[taskId]).toHaveLength(1);

    // Re-appending same item or item with identical content & type must deduplicate
    const duplicateSummary: DrawerActivityItem = {
      id: 'summary-uuid-2',
      type: 'summary',
      timestamp: '12:00:01',
      content: '### Саммари этапа: done',
    };
    store.appendActivityItem(taskId, duplicateSummary);

    const feed = useDrawerStore.getState().activityFeed[taskId];
    expect(feed).toHaveLength(1);
    expect(feed[0].content).toBe('### Саммари этапа: done');
  });

  it('loadTaskDialog deduplicates duplicate output and summary items loaded from SQLite', async () => {
    const store = useDrawerStore.getState();
    const taskId = 'CVH-051-TEST';

    mockInvoke.mockResolvedValueOnce([
      {
        id: 'out-1',
        type: 'output',
        timestamp: '12:00:00',
        content: 'Task output result',
      },
      {
        id: 'out-2',
        type: 'output',
        timestamp: '12:00:01',
        content: 'Task output result', // duplicate from race
      },
      {
        id: 'sum-1',
        type: 'summary',
        timestamp: '12:00:02',
        content: 'Handover complete',
      },
    ]);

    await store.loadTaskDialog(taskId);

    const feed = useDrawerStore.getState().activityFeed[taskId];
    expect(feed).toHaveLength(2);
    expect(feed[0].type).toBe('output');
    expect(feed[0].content).toBe('Task output result');
    expect(feed[1].type).toBe('summary');
    expect(feed[1].content).toBe('Handover complete');
    expect(mockInvoke).toHaveBeenCalledWith('load_task_activity', { taskId });
  });

  it('loadTaskActivity, saveTaskActivity, and clearTaskActivity invoke canonical IPC commands (CVH-054)', async () => {
    const store = useDrawerStore.getState();
    const taskId = 'CVH-054-TEST';

    mockInvoke.mockResolvedValueOnce([]);
    await store.loadTaskActivity(taskId);
    expect(mockInvoke).toHaveBeenCalledWith('load_task_activity', { taskId });

    useDrawerStore.setState({
      activityFeed: {
        [taskId]: [
          {
            id: 'act-1',
            type: 'output',
            timestamp: '12:00:00',
            content: 'Canonical activity save',
          },
        ],
      },
    });

    await store.saveTaskActivity(taskId);
    expect(mockInvoke).toHaveBeenCalledWith('save_task_activity', expect.objectContaining({
      taskId,
      items: expect.arrayContaining([expect.objectContaining({ id: 'act-1' })]),
    }));

    await store.clearTaskActivity(taskId);
    expect(mockInvoke).toHaveBeenCalledWith('clear_task_activity', { taskId });
    expect(useDrawerStore.getState().activityFeed[taskId]).toEqual([]);
  });
});

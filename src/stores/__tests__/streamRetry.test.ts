import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useDrawerStore } from '../drawerStore';
import { TextBufferManager, TEXT_FLUSH_MS } from '../../services/tauriEventGateway';
import { initSubagentListeners } from '../drawer/subagentListeners';
import type { DrawerActivityItem } from '../drawer/types';
import { shouldSuppressExplanation } from '../../components/drawer/stream/streamHelpers';

const eventHandlers = vi.hoisted(() => new Map<string, (event: { payload: unknown }) => void>());
const mockInvoke = vi.hoisted(() => vi.fn().mockResolvedValue([]));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (event: string, handler: (event: { payload: unknown }) => void) => {
    eventHandlers.set(event, handler);
    return () => eventHandlers.delete(event);
  }),
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mockInvoke }));

describe('stream attempt rollback', () => {
  let buffers: TextBufferManager;
  const taskId = 'CVH-069.1';
  const tool: DrawerActivityItem = {
    id: 'finished-tool', type: 'tool', timestamp: '10:00:00',
    tool: { id: 'finished-tool', toolName: 'read_file', timestamp: '10:00:00', durationMs: 12, status: 'success', params: {} },
  };

  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-11-06T10:00:00Z'));
    useDrawerStore.setState({
      activityFeed: {}, streamEntries: {}, activeStreamIds: {}, connectionWait: {},
      streamRollbackVersions: {},
      activeAgentTaskIds: {}, thinkingStartTime: {}, generationStartTime: {},
      generationEndTime: {}, streamedTurnTokens: {}, finalTps: {},
      toolIdToSubagentId: {}, subagentIdToToolId: {}, subagents: {},
    });
    mockInvoke.mockReset().mockResolvedValue([]);
    buffers = new TextBufferManager();
  });

  afterEach(() => {
    buffers.dispose();
    vi.clearAllTimers();
    vi.useRealTimers();
  });

  it('preserves channel order and uses one stable entry per channel across interleaved chunks', () => {
    buffers.startStream(taskId, 'attempt-a');
    buffers.queueOutput(taskId, 'first ', 'attempt-a');
    buffers.queueThought(taskId, 'reason ', 'attempt-a');
    buffers.queueOutput(taskId, 'second', 'attempt-a');
    buffers.queueThought(taskId, 'again', 'attempt-a');
    vi.advanceTimersByTime(TEXT_FLUSH_MS);

    expect(useDrawerStore.getState().activityFeed[taskId].map(({ id, content }) => ({ id, content }))).toEqual([
      { id: 'attempt-a:output', content: 'first second' },
      { id: 'attempt-a:thought', content: 'reason again' },
    ]);
    expect(useDrawerStore.getState().streamEntries[taskId].outputs[0].id).toBe('attempt-a:output');
  });

  it('keeps neighboring completed turns and tools while removing visible and queued failed chunks', () => {
    buffers.startStream(taskId, 'completed');
    buffers.queueOutput(taskId, 'Completed answer', 'completed');
    buffers.queueThought(taskId, 'Completed reasoning', 'completed');
    buffers.flushTask(taskId);
    useDrawerStore.getState().appendActivityItem(taskId, tool);
    const completed = useDrawerStore.getState().activityFeed[taskId];

    buffers.startStream(taskId, 'failed');
    buffers.queueOutput(taskId, 'Partial answer', 'failed');
    buffers.queueThought(taskId, 'Partial reasoning', 'failed');
    buffers.flushTask(taskId);
    buffers.queueOutput(taskId, 'not yet painted', 'failed');
    buffers.queueThought('other-task', 'Unrelated', 'other-attempt');
    buffers.retryStream(taskId, 'failed', 5);
    vi.advanceTimersByTime(TEXT_FLUSH_MS);

    const state = useDrawerStore.getState();
    expect(state.activityFeed[taskId]).toEqual(completed);
    expect(state.streamEntries[taskId].thoughts.map((entry) => entry.id)).toEqual(['completed:thought']);
    expect(state.streamEntries[taskId].outputs.map((entry) => entry.id)).toEqual(['completed:output']);
    expect(state.activityFeed['other-task'][0].content).toBe('Unrelated');
    expect(state.connectionWait[taskId]).toEqual({ streamId: 'failed', delaySecs: 5 });
    expect(state.activeAgentTaskIds[taskId]).toBe(true);
  });

  it('ignores stale chunks/retry events and clears waiting at the next attempt', () => {
    buffers.startStream(taskId, 'failed');
    buffers.retryStream(taskId, 'failed', 10);
    buffers.queueOutput(taskId, 'late failed chunk', 'failed');
    buffers.startStream(taskId, 'next');
    buffers.queueOutput(taskId, 'next answer', 'next');
    buffers.retryStream(taskId, 'failed', 10);
    buffers.queueThought(taskId, 'late failed reasoning', 'failed');
    vi.advanceTimersByTime(TEXT_FLUSH_MS);

    const state = useDrawerStore.getState();
    expect(state.connectionWait[taskId]).toBeNull();
    expect(state.activeStreamIds[taskId]).toBe('next');
    expect(state.activityFeed[taskId].map((item) => item.id)).toEqual(['next:output']);
    expect(state.streamedTurnTokens[taskId]).toBe(Math.round('next answer'.length / 3.8));
  });

  it('does not merge adjacent successful responses from different attempts', () => {
    buffers.startStream(taskId, 'first');
    buffers.queueOutput(taskId, 'Same answer', 'first');
    buffers.startStream(taskId, 'second');
    buffers.queueOutput(taskId, 'Same answer', 'second');
    buffers.flushTask(taskId);
    expect(useDrawerStore.getState().activityFeed[taskId].map((item) => item.id)).toEqual(['first:output', 'second:output']);
    const feed = useDrawerStore.getState().activityFeed[taskId];
    expect(shouldSuppressExplanation(feed[1], 1, feed)).toBe(false);
  });

  it('preserves identical stable outputs during history reload and generic activity updates', async () => {
    const first: DrawerActivityItem = { id: 'first:output', type: 'output', timestamp: '10:00:00', content: 'Same answer' };
    const second = { ...first, id: 'second:output' };
    mockInvoke.mockResolvedValueOnce([first, second]);
    await useDrawerStore.getState().loadTaskActivity(taskId);
    expect(useDrawerStore.getState().activityFeed[taskId].map((item) => item.id)).toEqual(['first:output', 'second:output']);
    useDrawerStore.getState().appendActivityItem(taskId, { ...second, id: 'third:output' });
    expect(useDrawerStore.getState().activityFeed[taskId]).toHaveLength(3);
  });

  it('reloads history if a database read finishes after its attempt was rolled back', async () => {
    let resolveOldRead!: (items: DrawerActivityItem[]) => void;
    let notifyReadStarted!: () => void;
    const readStarted = new Promise<void>((resolve) => { notifyReadStarted = resolve; });
    mockInvoke.mockImplementationOnce(() => {
      notifyReadStarted();
      return new Promise<DrawerActivityItem[]>((resolve) => { resolveOldRead = resolve; });
    }).mockResolvedValueOnce([tool]);
    buffers.startStream(taskId, 'failed');
    const loading = useDrawerStore.getState().loadTaskActivity(taskId);
    await readStarted;
    buffers.retryStream(taskId, 'failed', 5);
    buffers.startStream(taskId, 'next');
    resolveOldRead([tool, { id: 'failed:output', type: 'output', timestamp: '10:00:01', content: 'Stale partial' }]);
    await loading;
    expect(useDrawerStore.getState().activityFeed[taskId].map((item) => item.id)).toEqual(['finished-tool']);
    expect(mockInvoke).toHaveBeenCalledTimes(2);
  });

  it('clears waiting on Stop and still flushes completed chunks if another terminal listener runs first', () => {
    buffers.startStream(taskId, 'failed');
    buffers.retryStream(taskId, 'failed', 10);
    useDrawerStore.getState().setActiveAgentTask(taskId, false);
    expect(useDrawerStore.getState().connectionWait[taskId]).toBeNull();

    buffers.startStream(taskId, 'finished');
    buffers.queueOutput(taskId, 'Final answer', 'finished');
    useDrawerStore.getState().setActiveAgentTask(taskId, false);
    buffers.flushTask(taskId);
    expect(useDrawerStore.getState().activityFeed[taskId][0].content).toBe('Final answer');
    expect(useDrawerStore.getState().activeAgentTaskIds[taskId]).toBe(false);
  });

  it('rolls back subagent snapshots and their tool aliases without counting retry chunks as steps', async () => {
    await initSubagentListeners(useDrawerStore);
    useDrawerStore.getState().registerSubagentMapping('subagent-tool', taskId);
    useDrawerStore.getState().registerSubagent('CVH-069', {
      id: taskId, parentTaskId: 'CVH-069', title: 'Worker', role: 'developer', prompt: '', status: 'running', stepsCount: 1,
    });
    useDrawerStore.setState({ activityFeed: { [taskId]: [tool], 'subagent-tool': [tool] } });
    buffers.startStream(taskId, 'sub-attempt');
    const activity = { id: 'sub-attempt:thought', type: 'thought', timestamp: '10:00:01', content: 'Partial' };
    eventHandlers.get('subagent:activity')!({ payload: { subagentId: taskId, activity } });
    eventHandlers.get('subagent:activity')!({ payload: { subagentId: taskId, activity: { ...activity, content: 'Partial update' } } });
    expect(useDrawerStore.getState().activityFeed[taskId]).toHaveLength(2);
    expect(useDrawerStore.getState().subagents['CVH-069'][0].stepsCount).toBe(1);

    buffers.retryStream(taskId, 'sub-attempt', 5);
    eventHandlers.get('subagent:activity')!({ payload: { subagentId: taskId, activity } });
    expect(useDrawerStore.getState().activityFeed[taskId]).toEqual([tool]);
    expect(useDrawerStore.getState().activityFeed['subagent-tool']).toEqual([tool]);
  });

  it('continues to accept legacy chunks without stream IDs', () => {
    buffers.queueThought(taskId, 'Legacy thought');
    buffers.queueOutput(taskId, 'Legacy output');
    buffers.flushTask(taskId);
    expect(useDrawerStore.getState().activityFeed[taskId].map((item) => item.content)).toEqual(['Legacy thought', 'Legacy output']);
  });

  it.each(['user', 'steer'] as const)('keeps intentional repeated %s submissions distinct and upserts their backend confirmation', async (type) => {
    const store = useDrawerStore.getState();
    store.appendOutput(taskId, 'Continue', type);
    store.appendOutput(taskId, 'Continue', type);
    const optimistic = useDrawerStore.getState().activityFeed[taskId];
    expect(optimistic).toHaveLength(2);
    expect(optimistic[0].id).not.toBe(optimistic[1].id);
    expect(optimistic.map((item) => item.content)).toEqual(['Continue', 'Continue']);

    store.appendActivityItem(taskId, { ...optimistic[1], type: 'steer' });
    expect(useDrawerStore.getState().activityFeed[taskId]).toHaveLength(2);
    mockInvoke.mockResolvedValueOnce(optimistic);
    await store.loadTaskActivity(taskId);
    expect(useDrawerStore.getState().activityFeed[taskId].map((item) => item.id)).toEqual(optimistic.map((item) => item.id));
  });
});

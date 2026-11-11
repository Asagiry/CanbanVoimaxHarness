import { describe, it, expect, beforeEach, vi } from 'vitest';
import '@tauri-apps/api/event';
import '@tauri-apps/api/core';
import { TextBufferManager, setupTauriEventGateway } from '../tauriEventGateway';
import { initSubagentListeners } from '../../stores/drawer/subagentListeners';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import { useAgentSessionStore } from '../../stores/agentSessionStore';
import { useCortexStore } from '../../stores/cortexStore';
import { CANONICAL_BOARD_COLUMNS, DEFAULT_BOARD } from '../../stores/board/boardConstants';

const { mockListen, mockInvoke } = vi.hoisted(() => ({
  mockListen: vi.fn(),
  mockInvoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => mockListen(...args),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('tauriEventGateway & subagentListeners', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useDrawerStore.getState().resetProjectContext();
    useAgentSessionStore.getState().reset();
    useBoardStore.setState({
      boards: [DEFAULT_BOARD],
      activeBoardId: 'development',
      stages: CANONICAL_BOARD_COLUMNS,
      columns: CANONICAL_BOARD_COLUMNS,
      tasks: [],
      isLoadingTasks: false,
      isLoadingBoards: false,
    });
  });

  it('does not leak subagent listeners across StrictMode double-mount when mount 1 aborts before listen resolves', async () => {
    const resolvers: Array<() => void> = [];
    const unlistenSpies: Array<ReturnType<typeof vi.fn>> = [];

    mockListen.mockImplementation(() => {
      const spy = vi.fn();
      unlistenSpies.push(spy);
      return new Promise<() => void>((resolve) => {
        resolvers.push(() => resolve(spy));
      });
    });

    const controller1 = new AbortController();
    const p1 = initSubagentListeners(useDrawerStore, controller1.signal);
    await vi.waitFor(() => {
      expect(resolvers.length).toBe(1);
    });
    controller1.abort();

    const controller2 = new AbortController();
    const p2 = initSubagentListeners(useDrawerStore, controller2.signal);

    let settled = false;
    const both = Promise.all([p1, p2]).then((res) => {
      settled = true;
      return res;
    });

    while (!settled) {
      while (resolvers.length > 0) {
        resolvers.shift()?.();
      }
      await new Promise((r) => setTimeout(r, 0));
    }

    const [cleanup1, cleanup2] = await both;

    // 1 listener from mount 1 (aborted on resolve) + 6 listeners from mount 2
    expect(unlistenSpies.length).toBeGreaterThanOrEqual(6);
    expect(unlistenSpies[0]).toHaveBeenCalledTimes(1);

    cleanup1();
    cleanup2();
    for (const spy of unlistenSpies) {
      expect(spy).toHaveBeenCalledTimes(1);
    }
  });

  it('setupTauriEventGateway disposes TextBufferManager without flushing stale chunks', async () => {
    vi.useFakeTimers();
    const manager = new TextBufferManager();
    manager.queueOutput('TASK-1', 'stale chunk');
    manager.dispose();

    vi.advanceTimersByTime(500);
    expect(useDrawerStore.getState().activityFeed['TASK-1']).toBeUndefined();
    vi.useRealTimers();
  });

  it('subagent:finished does not label running tools as user-aborted', async () => {
    const handlers = new Map<string, (e: { payload: unknown }) => void>();
    mockListen.mockImplementation(async (event: string, cb: (e: { payload: unknown }) => void) => {
      handlers.set(event, cb);
      return vi.fn();
    });

    const cleanup = await initSubagentListeners(useDrawerStore);
    useDrawerStore.getState().addToolExecution('sub-1', {
      id: 'tool-1',
      toolName: 'read_file',
      timestamp: '10:00:00',
      durationMs: 0,
      status: 'running',
      params: {},
    });

    handlers.get('subagent:finished')?.({ payload: { subagentId: 'sub-1', response: 'Done' } });
    const tool = useDrawerStore.getState().toolExecutions['sub-1']?.[0];
    expect(tool?.status).toBe('failed');
    expect(tool?.stderr).not.toContain('Прервано пользователем');
    cleanup();
  });

  it('routes task:deleted, agent:aborted, agent:steer, agent:compacted and queues project:switched during load', async () => {
    const handlers = new Map<string, (e: { payload: unknown }) => void>();
    mockListen.mockImplementation(async (event: string, cb: (e: { payload: unknown }) => void) => {
      handlers.set(event, cb);
      return vi.fn();
    });
    mockInvoke.mockResolvedValue([]);

    const dispose = setupTauriEventGateway();
    await vi.waitFor(() => {
      expect(handlers.has('project:switched')).toBe(true);
      expect(handlers.has('task:deleted')).toBe(true);
      expect(handlers.has('agent:aborted')).toBe(true);
      expect(handlers.has('agent:steer')).toBe(true);
      expect(handlers.has('agent:compacted')).toBe(true);
    });

    useBoardStore.setState({
      tasks: [
        {
          id: 'CVH-9',
          title: 'T9',
          priority: 'medium',
          stageId: 'dev',
          boardId: 'development',
          persona: 'cvh-dev',
          status: 'running',
          activeStageDuration: 0,
          stageDurations: {},
          totalElapsed: 0,
          isTimerRunning: true,
          returnsCount: 0,
        },
      ],
    });
    useDrawerStore.setState({ activeTaskId: 'CVH-9', isOpen: true });

    handlers.get('agent:compacted')?.({ payload: { taskId: 'CVH-9', tokensAfter: 1200 } });
    expect(useDrawerStore.getState().tokenOverrides['CVH-9']).toBe(1200);

    handlers.get('agent:steer')?.({ payload: { taskId: 'CVH-9', content: 'Focus on tests' } });
    expect(useDrawerStore.getState().activityFeed['CVH-9']?.some((i) => i.content === 'Focus on tests')).toBe(true);

    handlers.get('agent:aborted')?.({ payload: { taskId: 'CVH-9' } });
    expect(useBoardStore.getState().tasks[0].status).toBe('idle');

    handlers.get('task:deleted')?.({ payload: { taskId: 'CVH-9', deletedIds: ['CVH-9'] } });
    expect(useBoardStore.getState().tasks).toEqual([]);
    expect(useDrawerStore.getState().isOpen).toBe(false);

    // Project switch arrives while isLoadingTasks is true -> must wait and then reset + reload
    const loadBoardsSpy = vi.spyOn(useBoardStore.getState(), 'loadBoards').mockResolvedValue();
    const loadTasksSpy = vi.spyOn(useBoardStore.getState(), 'loadTasks').mockResolvedValue();
    const loadPersonasSpy = vi.spyOn(useCortexStore.getState(), 'loadAgentPersonas').mockResolvedValue();
    const loadRulesSpy = vi.spyOn(useCortexStore.getState(), 'loadGlobalRules').mockResolvedValue();
    useAgentSessionStore.getState().handleSessionStatus({
      sessionId: 'sess-1',
      taskId: 'OLD-1',
      status: 'running',
      startedAt: 1,
    });
    useBoardStore.setState({ isLoadingTasks: true });

    handlers.get('project:switched')?.({ payload: {} });
    expect(loadBoardsSpy).not.toHaveBeenCalled();

    useBoardStore.setState({ isLoadingTasks: false });
    await vi.waitFor(() => {
      expect(loadBoardsSpy).toHaveBeenCalledTimes(1);
      expect(loadTasksSpy).toHaveBeenCalledTimes(1);
      expect(loadPersonasSpy).toHaveBeenCalledTimes(1);
      expect(loadRulesSpy).toHaveBeenCalledTimes(1);
    });
    expect(useAgentSessionStore.getState().sessions).toEqual({});

    dispose();
  });
});

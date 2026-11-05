import { describe, it, expect, beforeEach, beforeAll, vi } from 'vitest';
import { useDrawerStore } from '../drawerStore';
import type { DrawerActivityItem } from '../drawer/types';

const mockInvoke = vi.fn().mockResolvedValue([]);

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('Tool Time Tracking Pipeline (CVH-061)', () => {
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
    });
  });

  it('addToolExecution automatically sets createdAt timestamp', () => {
    const store = useDrawerStore.getState();
    const taskId = 'CVH-061-TEST';
    const before = Date.now();

    store.addToolExecution(taskId, {
      id: 'tool-call-1',
      toolName: 'read_file',
      timestamp: '12:00:00',
      durationMs: 0,
      status: 'running',
      params: { file_path: 'test.rs' },
    });

    const tools = useDrawerStore.getState().toolExecutions[taskId];
    expect(tools).toHaveLength(1);
    expect(tools[0].createdAt).toBeGreaterThanOrEqual(before);
    expect(tools[0].createdAt).toBeLessThanOrEqual(Date.now());
  });

  it('updateToolExecution records backend durationMs when provided', () => {
    const store = useDrawerStore.getState();
    const taskId = 'CVH-061-TEST';

    store.addToolExecution(taskId, {
      id: 'tool-call-2',
      toolName: 'execute_command',
      timestamp: '12:00:00',
      durationMs: 0,
      status: 'running',
      params: { command: 'cargo check' },
    });

    store.updateToolExecution(taskId, 'tool-call-2', {
      status: 'success',
      durationMs: 1420.5,
      stdout: 'Finished check',
    });

    const tool = useDrawerStore.getState().toolExecutions[taskId][0];
    expect(tool.status).toBe('success');
    expect(tool.durationMs).toBe(1420.5);

    const feedItem = useDrawerStore.getState().activityFeed[taskId][0];
    expect(feedItem.durationMs).toBe(1420.5);
    expect(feedItem.tool?.durationMs).toBe(1420.5);
  });

  it('updateToolExecution falls back to elapsed time from createdAt when durationMs is omitted', () => {
    const store = useDrawerStore.getState();
    const taskId = 'CVH-061-TEST';
    const pastTime = Date.now() - 3500;

    store.addToolExecution(taskId, {
      id: 'tool-call-3',
      toolName: 'glob',
      timestamp: '12:00:00',
      durationMs: 0,
      status: 'running',
      params: { pattern: '**/*.rs' },
      createdAt: pastTime,
    });

    store.updateToolExecution(taskId, 'tool-call-3', {
      status: 'success',
    });

    const tool = useDrawerStore.getState().toolExecutions[taskId][0];
    expect(tool.durationMs).toBeGreaterThanOrEqual(3400);

    const feedItem = useDrawerStore.getState().activityFeed[taskId][0];
    expect(feedItem.durationMs).toBeGreaterThanOrEqual(3400);
  });

  it('loadTaskActivity computes durations from SQLite items and populates toolExecutions', async () => {
    const taskId = 'CVH-061-TEST';
    const mockItems: DrawerActivityItem[] = [
      {
        id: 't-1',
        type: 'tool',
        timestamp: '2026-03-30T10:00:00.000Z',
        durationMs: 1250,
        tool: {
          id: 't-1',
          toolName: 'grep',
          timestamp: '2026-03-30T10:00:00.000Z',
          durationMs: 1250,
          status: 'success',
          params: {},
        },
      },
      {
        id: 't-2',
        type: 'tool',
        timestamp: '2026-03-30T10:00:02.000Z',
        tool: {
          id: 't-2',
          toolName: 'read_file',
          timestamp: '2026-03-30T10:00:02.000Z',
          durationMs: 0,
          status: 'success',
          params: {},
        },
      },
      {
        id: 'out-1',
        type: 'output',
        timestamp: '2026-03-30T10:00:05.000Z',
        content: 'Done',
      },
    ];

    mockInvoke.mockResolvedValueOnce(mockItems);

    await useDrawerStore.getState().loadTaskActivity(taskId);

    const tools = useDrawerStore.getState().toolExecutions[taskId];
    expect(tools).toHaveLength(2);

    // t-1 had explicit durationMs 1250
    const tool1 = tools.find((t) => t.id === 't-1');
    expect(tool1?.durationMs).toBe(1250);

    // t-2 had durationMs 0, so computed duration from timestamps (10:00:02 to 10:00:05 = 3000ms)
    const tool2 = tools.find((t) => t.id === 't-2');
    expect(tool2?.durationMs).toBe(3000);
  });
});

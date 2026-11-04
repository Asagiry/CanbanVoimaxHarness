import { describe, it, expect, beforeEach, beforeAll, vi } from 'vitest';
import { useDrawerStore } from '../drawerStore';
import type { SubagentInfo } from '../../types';
import type { DrawerActivityItem } from '../drawer/types';

const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('Subagent Duration & Timestamp Preservation (CVH-030)', () => {
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
    useDrawerStore.setState({
      isOpen: false,
      activeTaskId: null,
      subagents: {},
      activityFeed: {},
      toolExecutions: {},
      toolIdToSubagentId: {},
      subagentIdToToolId: {},
    });
  });

  it('loadSubagents preserves earliest createdAt and does not overwrite with later now() timestamp', async () => {
    const parentTaskId = 'CVH-030';
    const subId = 'CVH-030.sub-call_test1';
    const initialCreatedAt = Date.now() - 60000; // 60s ago
    const initialFinishedAt = Date.now() - 10000; // 10s ago

    // Pre-populate store with an existing subagent having initialCreatedAt
    useDrawerStore.setState({
      subagents: {
        [parentTaskId]: [
          {
            id: subId,
            parentTaskId,
            toolCallId: 'call_test1',
            title: 'Initial Subagent',
            role: 'developer',
            prompt: 'Test prompt',
            status: 'running',
            createdAt: initialCreatedAt,
            finishedAt: initialFinishedAt,
            stepsCount: 15,
          },
        ],
      },
    });

    // Mock backend returning a refreshed item with a newer createdAt (simulate reopening drawer)
    const refreshedCreatedAt = Date.now();
    const loadedFromBackend: SubagentInfo[] = [
      {
        id: subId,
        parentTaskId,
        toolCallId: 'call_test1',
        title: 'Initial Subagent',
        role: 'developer',
        prompt: 'Test prompt',
        status: 'running',
        createdAt: refreshedCreatedAt,
        stepsCount: 20,
      },
    ];
    mockInvoke.mockResolvedValueOnce(loadedFromBackend);

    await useDrawerStore.getState().loadSubagents(parentTaskId);

    const subs = useDrawerStore.getState().subagents[parentTaskId];
    expect(subs).toHaveLength(1);
    // CRITICAL: createdAt must be the EARLIEST (initialCreatedAt), not overwritten by refreshedCreatedAt
    expect(subs[0].createdAt).toBe(initialCreatedAt);
    // finishedAt must be preserved
    expect(subs[0].finishedAt).toBe(initialFinishedAt);
    // stepsCount updated
    expect(subs[0].stepsCount).toBe(20);
  });

  it('loadTaskActivity extracts and registers toolIdToSubagentId from subagent_bg tool executions', async () => {
    const taskId = 'CVH-030-PARENT';
    const toolCallId = 'call_bg_42';
    const subagentId = 'CVH-030-PARENT.sub-call_bg_42';

    const activityItems: DrawerActivityItem[] = [
      {
        id: 'act-tool-1',
        type: 'tool',
        timestamp: '2025-05-18T10:00:00Z',
        tool: {
          id: toolCallId,
          toolName: 'subagent_bg',
          params: {
            title: 'Background Auditor',
            role: 'reviewer',
            prompt: 'Audit architecture',
          },
          status: 'success',
          timestamp: '2025-05-18T10:00:00Z',
          durationMs: 120,
          stdout: JSON.stringify({ subagent_id: subagentId }),
        },
      },
    ];
    mockInvoke.mockResolvedValueOnce(activityItems);

    await useDrawerStore.getState().loadTaskActivity(taskId);

    const mapping = useDrawerStore.getState().toolIdToSubagentId;
    expect(mapping[toolCallId]).toBe(subagentId);
  });
});

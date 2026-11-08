import { describe, it, expect, beforeEach, beforeAll, vi } from 'vitest';
import { useDrawerStore } from '../drawerStore';
import { parseDiscoveredSubagents } from '../../components/drawer/tools/SubagentToolCard';

const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('Subagent prompt resolution (CVH-058)', () => {
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
      activeSubagentId: null,
      activeSubagentTitle: null,
      activeSubagentRole: null,
      activeSubagentPrompt: null,
      subagents: {},
      activityFeed: {},
      toolExecutions: {},
      toolIdToSubagentId: {},
      subagentIdToToolId: {},
    });
  });

  it('openSubagent with empty prompt resolves SubagentInfo.prompt from store', () => {
    const parentTaskId = 'CVH-058';
    const subId = 'CVH-058.sub-call_1';
    useDrawerStore.setState({
      activeTaskId: parentTaskId,
      subagents: {
        [parentTaskId]: [
          {
            id: subId,
            parentTaskId,
            toolCallId: 'call_1',
            title: 'Subagent #1',
            role: 'developer',
            prompt: 'Implement the feature',
            status: 'finished',
            stepsCount: 3,
          },
        ],
      },
    });

    useDrawerStore.getState().openSubagent(subId, 'Subagent #1', 'developer', '');

    expect(useDrawerStore.getState().activeSubagentPrompt).toBe('Implement the feature');
  });

  it('openSubagent with empty prompt resolves spawn tool params.prompt', () => {
    const parentTaskId = 'CVH-058';
    const toolId = 'call_bg_1';
    const subId = 'CVH-058.sub-call_bg_1';
    useDrawerStore.setState({
      activeTaskId: parentTaskId,
      toolIdToSubagentId: { [toolId]: subId },
      subagentIdToToolId: { [subId]: toolId },
      toolExecutions: {
        [parentTaskId]: [
          {
            id: toolId,
            toolName: 'subagent_bg',
            params: { title: 'Bg task', role: 'reviewer', prompt: 'Review the code' },
            status: 'success',
            durationMs: 120,
            timestamp: '2025-05-18T10:00:00Z',
          },
        ],
      },
    });

    useDrawerStore.getState().openSubagent(subId, 'Bg task', 'reviewer', '');

    expect(useDrawerStore.getState().activeSubagentPrompt).toBe('Review the code');
  });

  it('openSubagent keeps explicit non-empty prompt', () => {
    useDrawerStore.setState({ activeTaskId: 'CVH-058' });
    useDrawerStore.getState().openSubagent('sub-1', 'T', 'general', 'Explicit prompt');
    expect(useDrawerStore.getState().activeSubagentPrompt).toBe('Explicit prompt');
  });

  it('parseDiscoveredSubagents preserves prompt from stdout JSON', () => {
    const stdout = JSON.stringify([
      { id: 'sub-1', title: 'One', role: 'developer', status: 'finished', prompt: 'Do thing one' },
    ]);
    const result = parseDiscoveredSubagents(stdout, 'CVH-058', {}, new Map(), {}, (k: any) => String(k));
    expect(result).toHaveLength(1);
    expect(result[0].prompt).toBe('Do thing one');
  });

  it('parseDiscoveredSubagents preserves prompt from store fallback', () => {
    const result = parseDiscoveredSubagents(
      null,
      'CVH-058',
      {
        'CVH-058': [
          {
            id: 'sub-2',
            parentTaskId: 'CVH-058',
            title: 'Two',
            role: 'reviewer',
            prompt: 'Review thing two',
            status: 'finished',
            stepsCount: 1,
          },
        ],
      },
      new Map(),
      {},
      (k: any) => String(k)
    );
    expect(result).toHaveLength(1);
    expect(result[0].prompt).toBe('Review thing two');
  });
});

import { describe, it, expect, beforeEach, beforeAll, vi } from 'vitest';
import React from 'react';
import { renderToString } from 'react-dom/server';
import SubagentsSidebar from '../SubagentsSidebar';
import GitDiffSidebar from '../GitDiffSidebar';
import CortexExplorer from '../../cortex/CortexExplorer';
import AgentConsoleDrawer from '../AgentConsoleDrawer';
import { useDrawerStore } from '../../../stores/drawerStore';
import { useBoardStore } from '../../../stores/boardStore';
import { useAgentSessionStore } from '../../../stores/agentSessionStore';
import { useCortexStore } from '../../../stores/cortexStore';

// Mock Tauri IPC APIs
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

vi.mock('@tauri-apps/plugin-notification', () => ({
  isPermissionGranted: vi.fn().mockResolvedValue(true),
  requestPermission: vi.fn().mockResolvedValue('granted'),
  sendNotification: vi.fn(),
}));

vi.mock('../../../services/tauriEventGateway', () => ({
  tauriEventGateway: {
    onAgentEvent: vi.fn().mockResolvedValue(() => {}),
  },
}));

class ReactDispatcherSimulator {
  render<T>(componentFn: () => T): T {
    const internals = (React as any).__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;
    const prevDispatcher = internals.H;

    const mockDispatcher = {
      useSyncExternalStore: (_subscribe: any, getSnapshot: any) => getSnapshot(),
      useState: (initial: any) => {
        const val = typeof initial === 'function' ? initial() : initial;
        return [val, () => {}];
      },
      useMemo: (factory: any) => factory(),
      useEffect: () => {},
      useCallback: (fn: any) => fn,
      useRef: (initial: any) => ({ current: initial }),
      useContext: (ctx: any) => ctx?._currentValue,
      useDebugValue: () => {},
      useId: () => 'test-id',
      useLayoutEffect: () => {},
      useInsertionEffect: () => {},
      useTransition: () => [false, (fn: any) => fn()],
      useDeferredValue: (val: any) => val,
      useActionState: (action: any, initialState: any) => [initialState, action, false],
      useOptimistic: (passthrough: any) => [passthrough, () => {}],
    };

    internals.H = mockDispatcher;
    try {
      return componentFn();
    } finally {
      internals.H = prevDispatcher;
    }
  }
}

describe('Column Roundings and Visible Endings Invariant', () => {
  beforeAll(() => {
    (globalThis as any).alert = vi.fn();
    if (typeof (globalThis as any).window === 'undefined') {
      (globalThis as any).window = globalThis;
    }
    (globalThis as any).window.addEventListener = vi.fn();
    (globalThis as any).window.removeEventListener = vi.fn();
    (globalThis as any).window.localStorage = {
      getItem: vi.fn(),
      setItem: vi.fn(),
      removeItem: vi.fn(),
    };
  });

  beforeEach(() => {
    vi.clearAllMocks();

    useDrawerStore.setState({
      isOpen: true,
      activeTaskId: 'CVH-057',
      activityFeed: {},
      streamEntries: {},
      toolExecutions: {},
      activeSubagentId: null,
      activeAgentTaskIds: {},
    });

    useBoardStore.setState({
      tasks: [
        {
          id: 'CVH-057',
          title: 'Fix Column Roundings',
          stageId: 'dev',
          boardId: 'development',
          persona: 'cvh-dev',
          status: 'idle',
          priority: 'high',
          createdAt: Date.now(),
          updatedAt: Date.now(),
          activeStageDuration: 0,
          stageDurations: {},
          totalElapsed: 0,
          isTimerRunning: false,
          returnsCount: 0,
          verdict: 'NONE',
        },
      ],
      columns: [
        { id: 'backlog', title: 'Backlog', color: '#10b981', icon: 'Inbox', persona: null, modelOverride: null, isIntake: true },
        { id: 'dev', title: 'Dev', color: '#3b82f6', icon: 'Code', persona: 'cvh-dev', modelOverride: null },
        { id: 'ready', title: 'Ready', color: '#6b7280', icon: 'Check', persona: null, modelOverride: null, terminal: true },
      ],
      activeBoardId: 'development',
    });

    useAgentSessionStore.setState({
      sessions: {},
      isSyncing: false,
    });

    useCortexStore.setState({
      stages: [],
      agentStages: [],
      agentPersonas: [],
      globalRules: [],
      focusedStageId: null,
      activeFile: null,
    });
  });

  it('1. SubagentsSidebar renders with rounded-xl, full border, my-1.5, and h-[calc(100%-0.75rem)]', () => {
    const html = renderToString(
      <SubagentsSidebar
        taskId="CVH-057"
        taskTitle="Fix Column Roundings"
        isCollapsed={false}
        onToggleCollapse={() => {}}
      />
    );

    // Assert full rounded corners, full border, and balanced vertical margin
    expect(html).toContain('rounded-xl');
    expect(html).toContain('border border-[#ebebeb] dark:border-[#21262d]');
    expect(html).toContain('my-1.5 ml-1.5');
    expect(html).toContain('h-[calc(100%-0.75rem)]');

    // Assert obsolete cut-off styles are absent
    expect(html).not.toContain('rounded-tl-xl');
    expect(html).not.toContain('rounded-tr-xl');
    expect(html).not.toContain('border-t border-x');
    expect(html).not.toContain('h-[calc(100%-0.25rem)]');
  });

  it('2. GitDiffSidebar renders with rounded-xl, full border, my-1.5, and h-[calc(100%-0.75rem)]', () => {
    const html = renderToString(
      <GitDiffSidebar
        taskId="CVH-057"
        isCollapsed={false}
        onToggleCollapse={() => {}}
      />
    );

    // Assert full rounded corners, full border, and balanced vertical margin
    expect(html).toContain('rounded-xl');
    expect(html).toContain('border border-[#ebebeb] dark:border-[#21262d]');
    expect(html).toContain('my-1.5 mr-1.5');
    expect(html).toContain('h-[calc(100%-0.75rem)]');

    // Assert obsolete cut-off styles are absent
    expect(html).not.toContain('rounded-tl-xl');
    expect(html).not.toContain('rounded-tr-xl');
    expect(html).not.toContain('border-t border-x');
    expect(html).not.toContain('h-[calc(100%-0.25rem)]');
  });

  it('3. CortexExplorer renders with rounded-xl, full border, margins, and supports className prop', () => {
    const htmlDefault = renderToString(<CortexExplorer />);

    expect(htmlDefault).toContain('rounded-xl');
    expect(htmlDefault).toContain('border border-[#ebebeb] dark:border-[#21262d]');
    expect(htmlDefault).toContain('my-1.5 ml-1.5 mr-1.5');
    expect(htmlDefault).toContain('h-[calc(100%-0.75rem)]');
    expect(htmlDefault).not.toContain('border-r border-[#ebebeb]');

    const htmlCustom = renderToString(<CortexExplorer className="test-explorer-class" />);
    expect(htmlCustom).toContain('test-explorer-class');
  });

  it('4. AgentConsoleDrawer central column renders with rounded-xl, full border, and my-1.5 mx-1.5', () => {
    const simulator = new ReactDispatcherSimulator();
    const tree = simulator.render(() => AgentConsoleDrawer());
    const html = renderToString(tree);

    // Central column container assertions
    expect(html).toContain('rounded-xl border border-[#ebebeb] dark:border-[#21262d] shadow-2xs my-1.5 mx-1.5');

    // Obsolete cut-off classes are absent
    expect(html).not.toContain('rounded-tl-xl rounded-tr-xl border-t border-x border-[#ebebeb] dark:border-[#21262d] shadow-2xs mt-1 mx-1.5');
  });
});

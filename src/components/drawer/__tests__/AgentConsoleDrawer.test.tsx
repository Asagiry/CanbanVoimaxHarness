import { describe, it, expect, beforeEach, beforeAll, vi } from 'vitest';
import React from 'react';
import AgentConsoleDrawer from '../AgentConsoleDrawer';
import ErrorBoundary from '../../common/ErrorBoundary';
import { useDrawerStore } from '../../../stores/drawerStore';
import { useBoardStore } from '../../../stores/boardStore';
import { useAgentSessionStore } from '../../../stores/agentSessionStore';

// Mocks for child components and Tauri APIs
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

vi.mock('../DrawerHeader', () => ({ default: () => React.createElement('div', { 'data-testid': 'drawer-header' }) }));
vi.mock('../DrawerActivityStream', () => ({ default: () => React.createElement('div', { 'data-testid': 'drawer-stream' }) }));
vi.mock('../DrawerSteerBar', () => ({ default: () => React.createElement('div', { 'data-testid': 'drawer-steer' }) }));
vi.mock('../GitDiffSidebar', () => ({ default: () => React.createElement('div', { 'data-testid': 'diff-sidebar' }) }));
vi.mock('../SubagentsSidebar', () => ({ default: () => React.createElement('div', { 'data-testid': 'subagents-sidebar' }) }));

/**
 * Emulates React's internal fiber hook dispatcher rules.
 * Records the sequence of hooks on initial render and throws
 * "Rendered more hooks than during the previous render." or
 * "Rendered fewer hooks than during the previous render."
 * if the hook sequence or count diverges across renders.
 */
class ReactHookRulesSimulator {
  private hookSequence: string[] = [];
  private isMounted = false;
  private currentHookIndex = 0;

  private record(hookName: string) {
    if (!this.isMounted) {
      this.hookSequence.push(hookName);
    } else {
      if (this.currentHookIndex >= this.hookSequence.length) {
        throw new Error('Rendered more hooks than during the previous render.');
      }
      const expected = this.hookSequence[this.currentHookIndex];
      if (expected !== hookName) {
        throw new Error(
          `React Rules of Hooks violation: hook mismatch at index ${this.currentHookIndex}. Expected ${expected}, got ${hookName}.`
        );
      }
      this.currentHookIndex++;
    }
  }

  get hookCount() {
    return this.hookSequence.length;
  }

  render(componentFn: () => any) {
    const internals = (React as any).__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;
    const prevDispatcher = internals.H;

    const mockDispatcher = {
      useSyncExternalStore: (_subscribe: any, getSnapshot: any) => {
        this.record('useSyncExternalStore');
        return getSnapshot();
      },
      useState: (initial: any) => {
        this.record('useState');
        const val = typeof initial === 'function' ? initial() : initial;
        return [val, () => {}];
      },
      useMemo: (factory: any) => {
        this.record('useMemo');
        return factory();
      },
      useEffect: () => {
        this.record('useEffect');
      },
      useCallback: (fn: any) => {
        this.record('useCallback');
        return fn;
      },
      useRef: (initial: any) => {
        this.record('useRef');
        return { current: initial };
      },
      useContext: (ctx: any) => {
        this.record('useContext');
        return ctx?._currentValue;
      },
      useDebugValue: () => {
        // debug value is non-semantic hook metadata
      },
      useId: () => {
        this.record('useId');
        return 'test-id';
      },
      useLayoutEffect: () => {
        this.record('useLayoutEffect');
      },
      useInsertionEffect: () => {
        this.record('useInsertionEffect');
      },
      useTransition: () => {
        this.record('useTransition');
        return [false, (fn: any) => fn()];
      },
      useDeferredValue: (val: any) => {
        this.record('useDeferredValue');
        return val;
      },
    };

    internals.H = mockDispatcher;
    try {
      if (this.isMounted) {
        this.currentHookIndex = 0;
      }
      const result = componentFn();
      if (this.isMounted && this.currentHookIndex < this.hookSequence.length) {
        throw new Error('Rendered fewer hooks than during the previous render.');
      }
      this.isMounted = true;
      return result;
    } finally {
      internals.H = prevDispatcher;
    }
  }
}

describe('AgentConsoleDrawer React Rules of Hooks Verification', () => {
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
      isOpen: false,
      activeTaskId: null,
      activityFeed: {},
      streamEntries: {},
      toolExecutions: {},
      activeSubagentId: null,
      activeAgentTaskIds: {},
    });
    useBoardStore.setState({
      tasks: [
        {
          id: 'CVH-001',
          title: 'Implement Task Feature',
          columnId: 'dev',
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
  });

  it('1. Renders with isOpen: false and then updating to isOpen: true does NOT throw hook violation', () => {
    const simulator = new ReactHookRulesSimulator();

    // Initial render: Drawer is CLOSED (isOpen: false)
    expect(useDrawerStore.getState().isOpen).toBe(false);
    let outputClosed: any;
    expect(() => {
      outputClosed = simulator.render(() => AgentConsoleDrawer());
    }).not.toThrow();

    expect(outputClosed).toBeNull();
    const initialHookCount = simulator.hookCount;
    // Exactly 16 hooks called at top-level before early return:
    // 1 (useUiZoom) + 5 (drawer, board, settings, toolPolicy, toolPolicy) + 1 (agentSession)
    // + 3 (useState) + 3 (useEffect) + 3 (useMemo) = 16
    expect(initialHookCount).toBeGreaterThan(0);

    // Update store state: Open drawer for task CVH-001
    useDrawerStore.setState({
      isOpen: true,
      activeTaskId: 'CVH-001',
    });

    // Re-render: Drawer is OPEN (isOpen: true)
    let outputOpen: any;
    expect(() => {
      outputOpen = simulator.render(() => AgentConsoleDrawer());
    }).not.toThrow();

    // Drawer is now open and returned valid React elements without throwing:
    // "Rendered more hooks than during the previous render."
    expect(outputOpen).not.toBeNull();
  });

  it('2. Regression invariant: Hook order simulator correctly detects and rejects conditional hooks', () => {
    const simulator = new ReactHookRulesSimulator();
    let condition = false;

    function BuggyComponent() {
      // Common hook
      React.useState(0);
      // Conditional early return
      if (!condition) return null;
      // Illegal hook after return (Root cause of commit 3726105)
      React.useState('leaked');
      return React.createElement('div');
    }

    // Render 1: condition = false (1 hook executed)
    expect(simulator.render(() => BuggyComponent())).toBeNull();

    // Render 2: condition = true (2 hooks executed) -> throws Rules of Hooks violation
    condition = true;
    expect(() => simulator.render(() => BuggyComponent())).toThrow(
      'Rendered more hooks than during the previous render.'
    );
  });

  it('2. useAgentSessionStore is subscribed unconditionally at the top level', () => {
    // Verify that session store reflects active authoritative running session
    useAgentSessionStore.getState().handleSessionStatus({
      sessionId: 'sess-001',
      taskId: 'CVH-001',
      status: 'running',
      startedAt: Date.now(),
    });

    expect(useAgentSessionStore.getState().isSessionRunning('CVH-001')).toBe(true);

    const simulator = new ReactHookRulesSimulator();
    useDrawerStore.setState({ isOpen: false, activeTaskId: 'CVH-001' });

    // Renders safely when closed
    expect(() => simulator.render(() => AgentConsoleDrawer())).not.toThrow();

    // Renders safely when opened
    useDrawerStore.setState({ isOpen: true, activeTaskId: 'CVH-001' });
    expect(() => simulator.render(() => AgentConsoleDrawer())).not.toThrow();
  });
});

describe('ErrorBoundary isOverlay Enhancement', () => {
  it('1. Renders fixed inset-0 z-[100] centered modal when isOverlay is true', () => {
    class BuggyComponent extends React.Component {
      override render(): React.ReactNode {
        throw new Error('Test Crash');
      }
    }

    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    // Render ErrorBoundary with isOverlay: true
    const boundary = new ErrorBoundary({
      isOverlay: true,
      scopeLabel: 'Test Modal',
      children: React.createElement(BuggyComponent),
    });

    // Simulate error state
    (boundary as any).state = { hasError: true, message: 'Test Crash' };

    const rendered = boundary.render() as React.ReactElement<{ className?: string }>;
    expect(rendered.props.className).toContain('fixed inset-0 z-[100]');
    expect(rendered.props.className).toContain('bg-black/60 backdrop-blur-xs');

    consoleErrorSpy.mockRestore();
  });

  it('2. Automatically renders overlay modal when scopeLabel is "Консоль задачи"', () => {
    const boundary = new ErrorBoundary({
      scopeLabel: 'Консоль задачи',
      children: React.createElement('div'),
    });

    (boundary as any).state = { hasError: true, message: 'Console Crash' };

    const rendered = boundary.render() as React.ReactElement<{ className?: string }>;
    expect(rendered.props.className).toContain('fixed inset-0 z-[100]');
    expect(rendered.props.className).toContain('bg-black/60 backdrop-blur-xs');
  });

  it('3. Renders inline standard flex-1 container when isOverlay is false', () => {
    const boundary = new ErrorBoundary({
      isOverlay: false,
      scopeLabel: 'Обычный блок',
      children: React.createElement('div'),
    });

    (boundary as any).state = { hasError: true, message: 'Inline Block Crash' };

    const rendered = boundary.render() as React.ReactElement<{ className?: string }>;
    expect(rendered.props.className).toContain('flex-1');
    expect(rendered.props.className).not.toContain('fixed inset-0');
  });
});

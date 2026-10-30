import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import DrawerHeader, { DrawerHeaderProps } from '../DrawerHeader';
import { KanbanTask, PipelineColumn } from '../../../types';
import { useDrawerStore } from '../../../stores/drawerStore';
import { useBoardStore } from '../../../stores/boardStore';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

vi.mock('../layout/WindowControls', () => ({
  default: () => React.createElement('div', { 'data-testid': 'window-controls' }),
  toggleMaximizeWindow: vi.fn(),
}));

class ReactDispatcherSimulator {
  render<T>(componentFn: () => T): T {
    const internals = (React as any).__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;
    const prevDispatcher = internals.H;

    const mockDispatcher = {
      useSyncExternalStore: (_subscribe: any, getSnapshot: any) => getSnapshot(),
      useState: (initial: any) => [typeof initial === 'function' ? initial() : initial, () => {}],
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

function findStepper(node: any): any {
  if (!node || typeof node !== 'object') return null;
  if (typeof node.props?.className === 'string' && node.props.className.includes('max-w-2xl')) {
    return node;
  }
  if (Array.isArray(node.props?.children)) {
    for (const child of node.props.children) {
      const found = findStepper(child);
      if (found) return found;
    }
  } else if (node.props?.children) {
    return findStepper(node.props.children);
  }
  return null;
}

describe('DrawerHeader Stage Breadcrumb Navigation', () => {
  const simulator = new ReactDispatcherSimulator();

  const columns: PipelineColumn[] = [
    { id: 'backlog', title: 'Backlog', color: '#64748b', icon: 'Inbox', persona: null, isIntake: true, modelOverride: null },
    { id: 'plan', title: 'Planning', color: '#0284c7', icon: 'Compass', persona: 'planner', modelOverride: null },
    { id: 'dev', title: 'Developer', color: '#6366f1', icon: 'Code', persona: 'developer', modelOverride: null },
    { id: 'review', title: 'Review', color: '#a855f7', icon: 'Eye', persona: 'reviewer', modelOverride: null },
    { id: 'ready', title: 'Ready', color: '#10b981', icon: 'Check', persona: null, terminal: true, modelOverride: null },
  ];

  const rootTaskInReady: KanbanTask = {
    id: 'CVH-040',
    title: 'Root Feature Task',
    columnId: 'ready',
    status: 'done',
    persona: 'planner',
    priority: 'high',
    activeStageDuration: 0,
    stageDurations: {},
    totalElapsed: 120,
    isTimerRunning: false,
    returnsCount: 0,
    verdict: 'NONE',
  };

  const devSubtaskDone: KanbanTask = {
    id: 'CVH-040.1',
    parentId: 'CVH-040',
    title: 'CVH-040 (Developer)',
    columnId: 'dev',
    status: 'done',
    persona: 'developer',
    priority: 'high',
    activeStageDuration: 0,
    stageDurations: {},
    totalElapsed: 60,
    isTimerRunning: false,
    returnsCount: 0,
    verdict: 'NONE',
  };

  const reviewSubtaskDone: KanbanTask = {
    id: 'CVH-040.2',
    parentId: 'CVH-040',
    title: 'CVH-040 (Review)',
    columnId: 'review',
    status: 'done',
    persona: 'reviewer',
    priority: 'high',
    activeStageDuration: 0,
    stageDurations: {},
    totalElapsed: 40,
    isTimerRunning: false,
    returnsCount: 0,
    verdict: 'NONE',
  };

  beforeEach(() => {
    useDrawerStore.setState({
      activeTaskId: 'CVH-040',
      activeSubagentId: null,
      activeSubagentTitle: null,
      activeSubagentRole: null,
      subagents: {},
      toolExecutions: {},
      toolIdToSubagentId: {},
    });
    useBoardStore.setState({
      tasks: [rootTaskInReady, devSubtaskDone, reviewSubtaskDone],
      columns,
    });
  });

  const renderDrawerHeader = (props: Partial<DrawerHeaderProps>) => {
    const fullProps: DrawerHeaderProps = {
      task: rootTaskInReady,
      columns,
      subtasks: [devSubtaskDone, reviewSubtaskDone],
      rootTask: rootTaskInReady,
      activeTaskId: 'CVH-040',
      onOpenDrawer: vi.fn(),
      onOpenToolPolicy: vi.fn(),
      onOpenSettings: vi.fn(),
      ...props,
    };
    return simulator.render(() => DrawerHeader(fullProps));
  };

  it('renders stages stepper with Planning active and subtasks passed when viewing root task in ready', () => {
    const headerElement = renderDrawerHeader({
      task: rootTaskInReady,
      activeTaskId: 'CVH-040',
    });

    expect(headerElement).toBeDefined();

    // Verify stage stepper container exists
    const stepperWrapper = findStepper(headerElement);
    expect(stepperWrapper).not.toBeNull();

    // The workingColumns are [plan, dev, review]
    const stageItems = stepperWrapper.props.children;
    expect(stageItems).toHaveLength(3);

    // 1. Planning button (idx 0):
    // Since activeTaskId === rootTask.id ('CVH-040'), Planning is current
    const planItem = stageItems[0];
    // For idx 0, ChevronRight is not rendered, so children is the button directly
    const actualPlanButton = planItem.props.children[1] ?? planItem.props.children[0] ?? planItem.props.children;
    expect(actualPlanButton.props.disabled).toBe(true); // Current stage is disabled to prevent self-click
    expect(actualPlanButton.props['aria-label']).toContain('Planning');

    // 2. Developer button (idx 1):
    // Subtask is done, targetTaskId is 'CVH-040.1', isCurrent is false, isPassed is true
    // For idx > 0, children[0] is ChevronRight and children[1] is the button
    const devButton = stageItems[1].props.children[1];
    expect(devButton.props.disabled).toBe(false); // Clickable to navigate to subtask!

    // 3. Review button (idx 2):
    // Subtask is done, targetTaskId is 'CVH-040.2', isCurrent is false, isPassed is true
    const reviewButton = stageItems[2].props.children[1];
    expect(reviewButton.props.disabled).toBe(false);
  });

  it('allows clicking Planning to switch back to root task when viewing developer subtask', () => {
    const onOpenDrawerMock = vi.fn();
    const headerElement = renderDrawerHeader({
      task: devSubtaskDone,
      activeTaskId: 'CVH-040.1',
      onOpenDrawer: onOpenDrawerMock,
    });

    const stepperWrapper = findStepper(headerElement);
    expect(stepperWrapper).not.toBeNull();
    const stageItems = stepperWrapper.props.children;

    // Planning button (idx 0):
    // targetTaskId should be 'CVH-040', isCurrent is false, isPassed is true, disabled is false!
    const planButton = stageItems[0].props.children[1] ?? stageItems[0].props.children[0] ?? stageItems[0].props.children;
    expect(planButton.props.disabled).toBe(false);

    // Simulate clicking on Planning button
    planButton.props.onClick({ stopPropagation: vi.fn() });
    expect(onOpenDrawerMock).toHaveBeenCalledWith('CVH-040');

    // Developer button (idx 1):
    // Current active task is CVH-040.1, so Developer button is disabled (already current)
    const devButton = stageItems[1].props.children[1];
    expect(devButton.props.disabled).toBe(true);
  });
});

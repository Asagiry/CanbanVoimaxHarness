import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderToString } from 'react-dom/server';
import { CardTimerBadge, CardTimeFooter, calculateTotalElapsed } from '../CardTimerBadge';
import { KanbanTask } from '../../../types';
import { useAgentSessionStore } from '../../../stores/agentSessionStore';

const createSampleTask = (overrides?: Partial<KanbanTask>): KanbanTask => ({
  id: 'CVH-001',
  title: 'Task A',
  description: 'Sample description',
  priority: 'medium',
  stageId: 'dev',
  boardId: 'development',
  persona: 'cvh-dev',
  status: 'idle',
  activeStageDuration: 0,
  stageDurations: {},
  totalElapsed: 0,
  isTimerRunning: false,
  returnsCount: 0,
  createdAt: 1000,
  updatedAt: 1000,
  ...overrides,
});

describe('CardTimerBadge component rendering', () => {
  it('renders green Done badge when task status is done without duration', () => {
    const task = createSampleTask({ status: 'done', totalElapsed: 0, stageDurations: {} });
    const html = renderToString(<CardTimerBadge task={task} />);

    expect(html).toContain('Done');
    expect(html).toContain('text-[#15803d]');
  });

  it('renders Done in MM:SS when task status is done with totalElapsed', () => {
    const task = createSampleTask({ status: 'done', totalElapsed: 952 });
    const html = renderToString(<CardTimerBadge task={task} />);

    expect(html).toContain('Done in 15:52');
    expect(html).toContain('text-[#15803d]');
  });

  it('renders Done in MM:SS from sum of stageDurations if totalElapsed is zero', () => {
    const task = createSampleTask({
      status: 'done',
      totalElapsed: 0,
      stageDurations: { plan: 65, dev: 60 },
    });
    const html = renderToString(<CardTimerBadge task={task} />);

    expect(html).toContain('Done in 02:05');
  });

  it('renders Done in MM:SS from timestamps fallback when totalElapsed and stageDurations are empty', () => {
    const task = createSampleTask({
      status: 'done',
      totalElapsed: 0,
      stageDurations: {},
      createdAt: 10000,
      completedAt: 839000, // 829 seconds = 13m 49s
    });
    const html = renderToString(<CardTimerBadge task={task} />);

    expect(html).toContain('Done in 13:49');
    expect(html).toContain('text-[#15803d]');
  });

  it('renders green Done badge when task has waitingForSubtaskId and status is idle', () => {
    const task = createSampleTask({ status: 'idle', waitingForSubtaskId: 'CVH-001.1' });
    const html = renderToString(<CardTimerBadge task={task} />);

    expect(html).toContain('Done');
    expect(html).toContain('text-[#15803d]');
  });

  it('renders Done in MM:SS for planner parent done+waiting with completedAt', () => {
    const task = createSampleTask({
      status: 'done',
      waitingForSubtaskId: 'CVH-102.1',
      createdAt: 10000,
      completedAt: 839000,
      stageEnteredAt: 10000,
      totalElapsed: 0,
      stageDurations: {},
    });
    const html = renderToString(<CardTimerBadge task={task} />);

    expect(html).toContain('Done in 13:49');
    expect(html).not.toContain('In Progress');
  });

  it('renders Done in MM:SS for idle+waiting stale parent with completedAt', () => {
    const task = createSampleTask({
      status: 'idle',
      waitingForSubtaskId: 'CVH-102.1',
      createdAt: 10000,
      completedAt: 839000,
      stageEnteredAt: 10000,
      totalElapsed: 0,
      stageDurations: {},
    });
    const html = renderToString(<CardTimerBadge task={task} />);

    expect(html).toContain('Done in 13:49');
    expect(html).not.toContain('In Progress');
  });

  it('renders green Done badge when isDone is explicitly passed true', () => {
    const task = createSampleTask({ status: 'idle' });
    const html = renderToString(<CardTimerBadge task={task} isDone={true} />);

    expect(html).toContain('Done');
    expect(html).toContain('text-[#15803d]');
  });

  it('renders green Running badge with timer when task status is running', () => {
    const task = createSampleTask({ status: 'running', stageEnteredAt: Date.now() - 125_000 });
    const html = renderToString(<CardTimerBadge task={task} />);

    expect(html).toContain('In Progress');
    expect(html).toContain('02:05');
  });

  it('renders neutral Idle badge when task is idle without waiting subtask', () => {
    useAgentSessionStore.getState().reset();
    const task = createSampleTask({ status: 'idle' });
    const html = renderToString(<CardTimerBadge task={task} isDone={false} />);

    expect(html).toContain('Idle');
    expect(html).toContain('text-[#666666]');
  });

  it('renders In Progress when status idle but session running', () => {
    useAgentSessionStore.getState().handleSessionStatus({ sessionId: 's1', taskId: 'CVH-001', status: 'running', startedAt: Date.now() });
    const task = createSampleTask({ status: 'idle', stageEnteredAt: Date.now() - 65_000 });
    const html = renderToString(<CardTimerBadge task={task} isDone={false} />);
    expect(html).toContain('In Progress');
    useAgentSessionStore.getState().reset();
  });
});

describe('calculateTotalElapsed function', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(100_000);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('calculates total elapsed from createdAt to Date.now() when not done', () => {
    const task = createSampleTask({ createdAt: 40_000, completedAt: undefined });
    // Date.now() = 100_000, createdAt = 40_000 -> 60_000ms = 60s
    expect(calculateTotalElapsed(task, false, false)).toBe(60);
  });

  it('calculates total elapsed from createdAt to completedAt when isDone is true', () => {
    const task = createSampleTask({ createdAt: 10_000, completedAt: 85_000 });
    // 85_000 - 10_000 = 75_000ms = 75s
    expect(calculateTotalElapsed(task, false, true)).toBe(75);
  });

  it('falls back to updatedAt when completedAt is missing and isDone is true', () => {
    const task = createSampleTask({ createdAt: 10_000, updatedAt: 50_000, completedAt: undefined });
    // 50_000 - 10_000 = 40_000ms = 40s
    expect(calculateTotalElapsed(task, false, true)).toBe(40);
  });

  it('falls back to task.totalElapsed if createdAt is missing', () => {
    const task = createSampleTask({ createdAt: undefined, totalElapsed: 120 });
    expect(calculateTotalElapsed(task, false, false)).toBe(120);
    expect(calculateTotalElapsed(task, false, true)).toBe(120);
  });
});

describe('CardTimeFooter component rendering', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(1_000_000);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('renders running {time} for in-progress / working task on board', () => {
    // Created 125 seconds before Date.now() (1_000_000 - 125_000 = 875_000) -> 2m 5s
    const task = createSampleTask({
      stageId: 'plan',
      status: 'idle',
      createdAt: 875_000,
    });
    const html = renderToString(<CardTimeFooter task={task} isDone={false} />);

    expect(html).toContain('running 2m 5s');
    expect(html).not.toContain('took');
  });

  it('renders took {time} only when isDone={true} (terminal column) with completedAt - createdAt', () => {
    const task = createSampleTask({
      stageId: 'ready',
      status: 'done',
      createdAt: 100_000,
      completedAt: 250_000, // 150 seconds = 2m 30s
    });
    const html = renderToString(<CardTimeFooter task={task} isDone={true} />);

    expect(html).toContain('took 2m 30s');
    expect(html).not.toContain('running');
  });

  it('renders only created {date} for idle task in backlog without running indicator', () => {
    const task = createSampleTask({
      stageId: 'backlog',
      status: 'idle',
      isTimerRunning: false,
      createdAt: 875_000,
    });
    const html = renderToString(<CardTimeFooter task={task} isDone={false} />);

    expect(html).toContain('created');
    expect(html).not.toContain('running');
    expect(html).not.toContain('took');
    expect(html).not.toContain('·');
  });

  it('renders running {time} for actively running task in backlog', () => {
    const task = createSampleTask({
      stageId: 'backlog',
      status: 'running',
      isTimerRunning: true,
      createdAt: 875_000, // 125 seconds = 2m 5s
    });
    const html = renderToString(<CardTimeFooter task={task} isDone={false} />);

    expect(html).toContain('running 2m 5s');
  });
});

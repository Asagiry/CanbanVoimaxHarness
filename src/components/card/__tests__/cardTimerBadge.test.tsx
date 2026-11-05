import { describe, it, expect } from 'vitest';
import { renderToString } from 'react-dom/server';
import { CardTimerBadge } from '../CardTimerBadge';
import { KanbanTask } from '../../../types';

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
    const task = createSampleTask({ status: 'idle' });
    const html = renderToString(<CardTimerBadge task={task} isDone={false} />);

    expect(html).toContain('Idle');
    expect(html).toContain('text-[#666666]');
  });
});

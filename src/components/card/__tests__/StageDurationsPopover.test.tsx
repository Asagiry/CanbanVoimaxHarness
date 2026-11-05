import { describe, it, expect, beforeEach } from 'vitest';
import { renderToString } from 'react-dom/server';
import { StageDurationsPopover } from '../StageDurationsPopover';
import { useBoardStore } from '../../../stores/boardStore';
import { KanbanTask, PipelineColumn } from '../../../types';

const columns: PipelineColumn[] = [
  { id: 'plan', title: 'Planning', color: '#0284c7', icon: 'Compass', persona: 'planner', modelOverride: null },
  { id: 'dev', title: 'Development', color: '#6366f1', icon: 'Code', persona: 'developer', modelOverride: null },
  { id: 'review', title: 'Review', color: '#a855f7', icon: 'Eye', persona: 'reviewer', modelOverride: null },
];

const createSampleTask = (overrides?: Partial<KanbanTask>): KanbanTask => ({
  id: 'CVH-001',
  title: 'Task A',
  priority: 'high',
  stageId: 'dev',
  boardId: 'development',
  persona: 'developer',
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

describe('StageDurationsPopover', () => {
  beforeEach(() => {
    useBoardStore.setState({ columns, tasks: [] });
  });

  const dummyRect = {
    top: 100,
    bottom: 120,
    left: 200,
    right: 300,
    width: 100,
    height: 20,
    x: 200,
    y: 100,
    toJSON: () => {},
  } as DOMRect;

  it('renders visited stages and formatted active durations', () => {
    const task = createSampleTask({
      stageDurations: { plan: 65, dev: 952 },
      totalElapsed: 1017,
      status: 'done',
    });

    const html = renderToString(
      <StageDurationsPopover
        task={task}
        isOpen={true}
        triggerRect={dummyRect}
        columns={columns}
      />
    );

    expect(html).toContain('Time in columns');
    expect(html).toContain('Planning');
    expect(html).toContain('01:05');
    expect(html).toContain('Development');
    expect(html).toContain('15:52');
    expect(html).toContain('Total active time');
    expect(html).toContain('16:57');
  });

  it('aggregates subtask stage durations for parent tasks', () => {
    const parentTask = createSampleTask({
      id: 'CVH-057',
      stageId: 'dev',
      stageDurations: { plan: 65 },
      status: 'waiting_subtask',
    });

    const childTask = createSampleTask({
      id: 'CVH-057.1',
      parentId: 'CVH-057',
      stageId: 'dev',
      activeStageDuration: 120,
      stageDurations: { dev: 120 },
      status: 'done',
    });

    const html = renderToString(
      <StageDurationsPopover
        task={parentTask}
        isOpen={true}
        triggerRect={dummyRect}
        columns={columns}
        allTasks={[parentTask, childTask]}
      />
    );

    expect(html).toContain('Planning');
    expect(html).toContain('01:05');
    expect(html).toContain('Development');
    expect(html).toContain('02:00');
  });

  it('returns null when isOpen is false', () => {
    const task = createSampleTask({ stageDurations: { plan: 65 } });
    const html = renderToString(
      <StageDurationsPopover
        task={task}
        isOpen={false}
        triggerRect={dummyRect}
      />
    );

    expect(html).toBe('');
  });
});

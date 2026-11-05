import { describe, it, expect } from 'vitest';
import { isDoneStageDef } from '../../../stores/board/boardHelpers';
import type { KanbanTask, PipelineStage } from '../../../types';

describe('KanbanColumn & KanbanBoard subtask visibility invariants', () => {
  const columns: PipelineStage[] = [
    { id: 'backlog', title: 'Backlog', color: '#64748b', isIntake: true, icon: 'Inbox', persona: null, modelOverride: null },
    { id: 'plan', title: 'Planning', color: '#3b82f6', persona: 'planner', icon: 'Compass', modelOverride: null },
    { id: 'dev', title: 'Developer', color: '#0284c7', persona: 'developer', icon: 'Wrench', modelOverride: null },
    { id: 'review', title: 'Review', color: '#a855f7', persona: 'reviewer', icon: 'Eye', modelOverride: null },
    { id: 'ready', title: 'Ready', color: '#10b981', terminal: true, icon: 'Check', persona: null, modelOverride: null },
  ];

  function createTestTask(overrides: Partial<KanbanTask>): KanbanTask {
    return {
      id: 'task-test',
      title: 'Test',
      description: 'Test description',
      priority: 'medium',
      stageId: 'backlog',
      boardId: 'development',
      persona: 'cvh-dev',
      status: 'idle',
      activeStageDuration: 0,
      stageDurations: {},
      totalElapsed: 0,
      isTimerRunning: false,
      returnsCount: 0,
      createdAt: Date.now(),
      updatedAt: Date.now(),
      ...overrides,
    };
  }

  function computeDoneParentIds(tasks: KanbanTask[], stages: PipelineStage[]): Set<string> {
    const set = new Set<string>();
    for (const t of tasks) {
      const col = stages.find((c) => c.id === t.stageId);
      const isTerminal = isDoneStageDef(col) || t.stageId === 'ready' || t.stageId === 'done';
      if (isTerminal) {
        set.add(t.id);
      }
    }
    return set;
  }

  function getDisplayTasksForColumn(columnId: string, tasks: KanbanTask[], doneParentIds: Set<string>): KanbanTask[] {
    const col = columns.find((c) => c.id === columnId);
    const isDoneColumn = isDoneStageDef(col);
    const columnTasks = tasks.filter((t) => t.stageId === columnId);

    if (isDoneColumn) {
      return columnTasks.filter((t) => !t.parentId);
    }

    return columnTasks.filter((t) => {
      if (!t.parentId) return true;
      if (doneParentIds && doneParentIds.has(t.parentId)) return false;
      return true;
    });
  }

  it('MUST show subtask in Developer column when parent planner stage is marked status="done"', () => {
    // Parent task in "plan" column, marked status: 'done' because planner finished and spawned subtask
    const parentTask = createTestTask({
      id: 'CVH-052',
      title: 'Hooks',
      stageId: 'plan',
      status: 'done',
      waitingForSubtaskId: 'CVH-052.1',
      boardId: 'development',
      createdAt: Date.now() - 10000,
    });

    // Active subtask in "dev" column
    const subtask = createTestTask({
      id: 'CVH-052.1',
      title: 'Hooks (Developer)',
      parentId: 'CVH-052',
      stageId: 'dev',
      status: 'running',
      boardId: 'development',
      createdAt: Date.now() - 5000,
    });

    const tasks = [parentTask, subtask];
    const doneParentIds = computeDoneParentIds(tasks, columns);

    // INVARIANT 1: Parent is in 'plan' (non-terminal), so its ID must NOT be in doneParentIds!
    expect(doneParentIds.has('CVH-052')).toBe(false);

    // INVARIANT 2: Subtask in 'dev' MUST be visible!
    const devDisplayTasks = getDisplayTasksForColumn('dev', tasks, doneParentIds);
    expect(devDisplayTasks.some((t) => t.id === 'CVH-052.1')).toBe(true);
  });

  it('MUST collapse subtask only when parent moves to terminal stage (ready/done)', () => {
    const parentTaskInReady = createTestTask({
      id: 'CVH-050',
      title: 'Test',
      stageId: 'ready',
      status: 'done',
      boardId: 'development',
      createdAt: Date.now() - 20000,
      completedAt: Date.now() - 1000,
    });

    const finishedSubtask = createTestTask({
      id: 'CVH-050.1',
      title: 'Test (Developer)',
      parentId: 'CVH-050',
      stageId: 'dev',
      status: 'done',
      boardId: 'development',
      createdAt: Date.now() - 15000,
    });

    const tasks = [parentTaskInReady, finishedSubtask];
    const doneParentIds = computeDoneParentIds(tasks, columns);

    // Parent is in terminal 'ready' column, so it is in doneParentIds
    expect(doneParentIds.has('CVH-050')).toBe(true);

    // Subtask in dev column should now be collapsed (hidden from active dev column)
    const devDisplayTasks = getDisplayTasksForColumn('dev', tasks, doneParentIds);
    expect(devDisplayTasks.some((t) => t.id === 'CVH-050.1')).toBe(false);

    // And ready column shows only the root parent
    const readyDisplayTasks = getDisplayTasksForColumn('ready', tasks, doneParentIds);
    expect(readyDisplayTasks.map((t) => t.id)).toEqual(['CVH-050']);
  });
});

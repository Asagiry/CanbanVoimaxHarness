import { describe, it, expect, beforeEach } from 'vitest';
import { useBoardStore } from '../boardStore';
import { KanbanTask } from '../../types';

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

describe('taskTimersSlice active duration counting', () => {
  beforeEach(() => {
    useBoardStore.getState().setTasks([]);
  });

  it('increments activeStageDuration, stageDurations[stageId], and totalElapsed for running tasks', () => {
    const runningTask = createSampleTask({
      id: 'CVH-001',
      stageId: 'dev',
      status: 'running',
      activeStageDuration: 10,
      stageDurations: { plan: 30, dev: 10 },
      totalElapsed: 40,
    });
    useBoardStore.getState().setTasks([runningTask]);

    useBoardStore.getState().tickTimers();

    const updated = useBoardStore.getState().tasks.find((t) => t.id === 'CVH-001');
    expect(updated?.activeStageDuration).toBe(11);
    expect(updated?.stageDurations.dev).toBe(11);
    expect(updated?.stageDurations.plan).toBe(30);
    expect(updated?.totalElapsed).toBe(41);
  });

  it('does NOT accumulate time for idle, done, paused, waiting or blocked tasks', () => {
    const idleTask = createSampleTask({ id: 'CVH-001', status: 'idle', totalElapsed: 5, activeStageDuration: 5 });
    const doneTask = createSampleTask({ id: 'CVH-002', status: 'done', totalElapsed: 20, activeStageDuration: 20 });
    const pausedTask = createSampleTask({ id: 'CVH-003', status: 'paused', totalElapsed: 15, activeStageDuration: 15 });
    const waitingSubtask = createSampleTask({ id: 'CVH-004', status: 'waiting_subtask', totalElapsed: 8, activeStageDuration: 8 });
    const waitingReview = createSampleTask({ id: 'CVH-005', status: 'waiting_review', totalElapsed: 12, activeStageDuration: 12 });
    const blockedTask = createSampleTask({ id: 'CVH-006', status: 'blocked', totalElapsed: 9, activeStageDuration: 9 });

    useBoardStore.getState().setTasks([
      idleTask,
      doneTask,
      pausedTask,
      waitingSubtask,
      waitingReview,
      blockedTask,
    ]);

    const initialTasks = useBoardStore.getState().tasks;
    useBoardStore.getState().tickTimers();

    // Since no tasks are running, state tasks reference should be preserved
    expect(useBoardStore.getState().tasks).toBe(initialTasks);

    const t1 = useBoardStore.getState().tasks.find((t) => t.id === 'CVH-001');
    const t2 = useBoardStore.getState().tasks.find((t) => t.id === 'CVH-002');
    const t3 = useBoardStore.getState().tasks.find((t) => t.id === 'CVH-003');
    const t4 = useBoardStore.getState().tasks.find((t) => t.id === 'CVH-004');
    expect(t1?.totalElapsed).toBe(5);
    expect(t2?.totalElapsed).toBe(20);
    expect(t3?.totalElapsed).toBe(15);
    expect(t4?.totalElapsed).toBe(8);
  });

  it('startTaskTimer and stopTaskTimer toggle isTimerRunning', () => {
    const task = createSampleTask({ id: 'CVH-001', isTimerRunning: false });
    useBoardStore.getState().setTasks([task]);

    useBoardStore.getState().startTaskTimer('CVH-001');
    expect(useBoardStore.getState().tasks[0].isTimerRunning).toBe(true);

    useBoardStore.getState().stopTaskTimer('CVH-001');
    expect(useBoardStore.getState().tasks[0].isTimerRunning).toBe(false);
  });

  it('resetTaskTimer sets activeStageDuration to 0 and stops timer', () => {
    const task = createSampleTask({ id: 'CVH-001', activeStageDuration: 50, isTimerRunning: true });
    useBoardStore.getState().setTasks([task]);

    useBoardStore.getState().resetTaskTimer('CVH-001');
    expect(useBoardStore.getState().tasks[0].activeStageDuration).toBe(0);
    expect(useBoardStore.getState().tasks[0].isTimerRunning).toBe(false);
  });
});

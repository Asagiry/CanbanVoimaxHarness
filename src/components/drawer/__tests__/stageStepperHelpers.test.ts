import { describe, it, expect, beforeEach } from 'vitest';
import {
  resolveStageTask,
  resolveStageDuration,
  resolveStageStatus,
} from '../stageStepperHelpers';
import type { KanbanTask, PipelineColumn } from '../../../types';
import { useAgentSessionStore } from '../../../stores/agentSessionStore';

describe('stageStepperHelpers', () => {
  const workingColumns: PipelineColumn[] = [
    { id: 'plan', title: 'Planning', color: '#0284c7', icon: 'Compass', persona: 'planner', modelOverride: null },
    { id: 'dev', title: 'Developer', color: '#6366f1', icon: 'Code', persona: 'developer', modelOverride: null },
    { id: 'review', title: 'Review', color: '#a855f7', icon: 'Eye', persona: 'reviewer', modelOverride: null },
  ];

  const rootTask: KanbanTask = {
    id: 'CVH-042',
    title: 'Root Feature Task',
    stageId: 'plan',
    status: 'done',
    persona: 'planner',
    priority: 'high',
    activeStageDuration: 0,
    stageDurations: { plan: 109 },
    totalElapsed: 124,
    isTimerRunning: false,
    returnsCount: 0,
    verdict: 'NONE',
  };

  const devSubtaskRunning: KanbanTask = {
    id: 'CVH-042.1',
    parentId: 'CVH-042',
    title: 'CVH-042 (Developer)',
    stageId: 'dev',
    status: 'running',
    persona: 'developer',
    priority: 'high',
    activeStageDuration: 15,
    stageDurations: { dev: 15 },
    totalElapsed: 124,
    isTimerRunning: true,
    returnsCount: 0,
    verdict: 'NONE',
  };

  beforeEach(() => {
    useAgentSessionStore.getState().reset();
  });

  describe('resolveStageTask', () => {
    it('resolves rootTask for planning stage when viewing developer subtask', () => {
      const task = resolveStageTask('plan', workingColumns, devSubtaskRunning, rootTask, [devSubtaskRunning]);
      expect(task).toBe(rootTask);
      expect(task?.id).toBe('CVH-042');
    });

    it('resolves matching subtask for developer stage', () => {
      const task = resolveStageTask('dev', workingColumns, devSubtaskRunning, rootTask, [devSubtaskRunning]);
      expect(task).toBe(devSubtaskRunning);
      expect(task?.id).toBe('CVH-042.1');
    });

    it('returns null for future unstarted stages', () => {
      const task = resolveStageTask('review', workingColumns, devSubtaskRunning, rootTask, [devSubtaskRunning]);
      expect(task).toBeNull();
    });
  });

  describe('resolveStageDuration', () => {
    it('preserves Planning stage duration (109s) when viewing developer subtask', () => {
      const duration = resolveStageDuration('plan', devSubtaskRunning, rootTask, [devSubtaskRunning], true);
      expect(duration).toBe(109);
    });

    it('displays Developer own stage duration (15s) instead of totalElapsed (124s)', () => {
      const duration = resolveStageDuration('dev', devSubtaskRunning, rootTask, [devSubtaskRunning], false);
      expect(duration).toBe(15);
      expect(duration).not.toBe(124);
    });

    it('returns 0 for unvisited stages', () => {
      const duration = resolveStageDuration('review', devSubtaskRunning, rootTask, [devSubtaskRunning], false);
      expect(duration).toBe(0);
    });
  });

  describe('resolveStageStatus', () => {
    it('marks Planning as passed and NOT running when Developer is running', () => {
      const planStatus = resolveStageStatus(
        'plan',
        0,
        1,
        rootTask,
        devSubtaskRunning,
        rootTask,
        [devSubtaskRunning],
        'CVH-042.1',
        true,
        workingColumns
      );

      expect(planStatus.isPassed).toBe(true);
      expect(planStatus.isColRunning).toBe(false);
      expect(planStatus.isCurrent).toBe(false);
    });

    it('marks Developer as running and NOT passed when Developer is active', () => {
      const devStatus = resolveStageStatus(
        'dev',
        1,
        1,
        devSubtaskRunning,
        devSubtaskRunning,
        rootTask,
        [devSubtaskRunning],
        'CVH-042.1',
        true,
        workingColumns
      );

      expect(devStatus.isColRunning).toBe(true);
      expect(devStatus.isPassed).toBe(false);
      expect(devStatus.isCurrent).toBe(true);
    });

    it('does not mark completed subtask as running even if drawer is running', () => {
      const devDone: KanbanTask = { ...devSubtaskRunning, status: 'done', isTimerRunning: false };
      const devStatus = resolveStageStatus(
        'dev',
        1,
        1,
        devDone,
        devDone,
        rootTask,
        [devDone],
        'CVH-042.1',
        true,
        workingColumns
      );

      expect(devStatus.isColRunning).toBe(false);
      expect(devStatus.isPassed).toBe(true);
    });
  });
});

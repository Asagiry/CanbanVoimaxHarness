import { describe, it, expect, beforeEach } from 'vitest';
import React from 'react';
import { renderToString } from 'react-dom/server';
import { useCardHooks, CardHooksResult } from '../useCardHooks';
import { KanbanTask, PipelineColumn, HookExecutionState } from '../../../types';
import { useBoardStore } from '../../../stores/boardStore';

const devColumn: PipelineColumn = {
  id: 'dev',
  title: 'Developer',
  color: '#3b82f6',
  icon: 'code',
  persona: 'dev-agent',
  autoRun: true,
  terminal: false,
  isIntake: false,
  modelOverride: null,
  hooks: {},
};

const reviewColumn: PipelineColumn = {
  id: 'review',
  title: 'Review',
  color: '#f59e0b',
  icon: 'search',
  persona: 'reviewer',
  autoRun: true,
  terminal: false,
  isIntake: false,
  modelOverride: null,
  hooks: {
    on_enter: { script: '.cvh/hooks/gatekeeper_sensor.bat', description: 'Gatekeeper sensors' },
  },
};

const columns: PipelineColumn[] = [devColumn, reviewColumn];

function createTask(overrides?: Partial<KanbanTask>): KanbanTask {
  return {
    id: 'CVH-074.1',
    title: 'Test task',
    description: 'desc',
    priority: 'medium',
    stageId: 'dev',
    boardId: 'dev',
    persona: 'dev-agent',
    status: 'idle',
    activeStageDuration: 0,
    stageDurations: {},
    totalElapsed: 0,
    isTimerRunning: false,
    returnsCount: 0,
    createdAt: 1000,
    updatedAt: 1000,
    ...overrides,
  };
}

function capture(
  task: KanbanTask,
  column: PipelineColumn | undefined,
  cols: PipelineColumn[] = columns
): CardHooksResult {
  let captured: CardHooksResult | null = null;
  function Harness() {
    captured = useCardHooks(task, column, cols);
    return React.createElement('div', null);
  }
  renderToString(React.createElement(Harness));
  if (!captured) throw new Error('useCardHooks was not invoked');
  return captured;
}

describe('useCardHooks left/right routing', () => {
  beforeEach(() => {
    useBoardStore.setState({ hookStates: {} });
  });

  it('a) running on_enter into next column goes to RIGHT node with target stage', () => {
    const hookState: HookExecutionState = {
      hookName: '.cvh/hooks/gatekeeper_sensor.bat',
      stageId: 'review',
      hookType: 'on_enter',
      status: 'running',
      startedAt: Date.now() - 44500,
      durationMs: 44500,
    };
    const task = createTask({ stageId: 'dev', hookStates: { on_enter: hookState } });

    const h = capture(task, devColumn);

    expect(h.isEnterRunningTarget).toBe(true);
    expect(h.hasRightHook).toBe(true);
    expect(h.effectiveRightState?.status).toBe('running');
    expect(h.effectiveRightHookType).toBe('on_enter');
    expect(h.effectiveRightHookName).toBe('.cvh/hooks/gatekeeper_sensor.bat');
    expect(h.targetStageTitle).toBe('Review');
    expect(h.targetEnterHookDescription).toBe('Gatekeeper sensors');
    expect(h.effectiveLeftState?.status).not.toBe('running');
    expect(h.hasLeftHook).toBe(false);
  });

  it('b) on_enter succeeded in its own column surfaces as LEFT (current stage entry)', () => {
    const hookState: HookExecutionState = {
      hookName: '.cvh/hooks/gatekeeper_sensor.bat',
      stageId: 'review',
      hookType: 'on_enter',
      status: 'success',
      startedAt: Date.now() - 50000,
      durationMs: 44500,
    };
    const task = createTask({ stageId: 'review', hookStates: { on_enter: hookState } });

    const h = capture(task, reviewColumn);

    expect(h.isEnterRunningTarget).toBe(false);
    expect(h.hasLeftHook).toBe(true);
    expect(h.effectiveLeftState?.status).toBe('success');
    expect(h.effectiveLeftHookType).toBe('on_enter');
    expect(h.effectiveLeftHookName).toBe('.cvh/hooks/gatekeeper_sensor.bat');
    expect(h.hasRightHook).toBe(false);
  });

  it('c) running on_leave in current column stays on RIGHT node', () => {
    const hookState: HookExecutionState = {
      hookName: '.cvh/hooks/on_leave_dev.bat',
      stageId: 'dev',
      hookType: 'on_leave',
      status: 'running',
      startedAt: Date.now() - 1000,
    };
    const task = createTask({ stageId: 'dev', hookStates: { on_leave: hookState } });

    const h = capture(task, devColumn);

    expect(h.hasRightHook).toBe(true);
    expect(h.effectiveRightState?.status).toBe('running');
    expect(h.effectiveRightHookType).toBe('on_leave');
    expect(h.isEnterRunningTarget).toBe(false);
  });

  it('d) transition on_leave from previous column stays on LEFT', () => {
    const hookState: HookExecutionState = {
      hookName: '.cvh/hooks/on_leave_dev.bat',
      stageId: 'dev',
      hookType: 'on_leave',
      status: 'success',
      startedAt: 1005000,
      durationMs: 500,
    };
    const task = createTask({
      stageId: 'review',
      previousStageId: 'dev',
      createdAt: 1000000,
      stageEnteredAt: 1005100,
      hookStates: { on_leave: hookState },
    });

    const h = capture(task, reviewColumn);

    expect(h.isTransitionHookFromResult).toBe(true);
    expect(h.hasLeftHook).toBe(true);
    expect(h.effectiveLeftState?.hookName).toBe('.cvh/hooks/on_leave_dev.bat');
    expect(h.effectiveLeftHookType).toBe('on_leave');
  });

  it('e) stale finished on_enter targeting another column does not pop a node', () => {
    const stale: HookExecutionState = {
      hookName: '.cvh/hooks/gatekeeper_sensor.bat',
      stageId: 'review',
      hookType: 'on_enter',
      status: 'success',
      startedAt: Date.now(),
      durationMs: 1000,
    };
    const task = createTask({ stageId: 'dev', hookStates: { on_enter: stale } });

    const h = capture(task, devColumn);

    expect(h.isEnterRunningTarget).toBe(false);
    expect(h.hasLeftHook).toBe(false);
    expect(h.hasRightHook).toBe(false);
  });
});

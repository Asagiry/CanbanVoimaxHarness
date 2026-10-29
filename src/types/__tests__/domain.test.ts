import { describe, it, expect } from 'vitest';
import {
  toTaskId,
  isTaskId,
  toStageId,
  isStageId,
  toBoardId,
  isBoardId,
  DomainValidationError,
  TaskState,
  TaskStates,
  isTaskState,
  toTaskState,
  isIdleState,
  isRunningState,
  isWaitingSubtaskState,
  isDoneState,
  isTerminalStage,
  isIntakeStage,
  CANONICAL_TERMINAL_STAGES,
  CANONICAL_INTAKE_STAGES,
  type TaskId,
  type StageId,
  type BoardId,
} from '../domain';

describe('Domain Value Objects & Types', () => {
  describe('TaskId', () => {
    it('normalizes lowercase strings to UPPERCASE and trims whitespace', () => {
      const taskId: TaskId = toTaskId('  cvh-001  ');
      expect(taskId).toBe('CVH-001');
    });

    it('handles mixed-case strings with valid separators', () => {
      expect(toTaskId('Task_42:sub.1')).toBe('TASK_42:SUB.1');
      expect(toTaskId('abc-123')).toBe('ABC-123');
      expect(toTaskId('PROJECT:task-999_A.b')).toBe('PROJECT:TASK-999_A.B');
    });

    it('accepts digits, dashes, underscores, colons, and dots', () => {
      const valid = 'CVH-123_456:789.0';
      expect(toTaskId(valid)).toBe(valid);
    });

    it('throws DomainValidationError on empty or whitespace-only strings', () => {
      expect(() => toTaskId('')).toThrow(DomainValidationError);
      expect(() => toTaskId('   ')).toThrow(DomainValidationError);
    });

    it('throws DomainValidationError on spaces inside the ID', () => {
      expect(() => toTaskId('CVH 001')).toThrow(DomainValidationError);
      expect(() => toTaskId('CVH	001')).toThrow(DomainValidationError);
    });

    it('throws DomainValidationError on prohibited special characters', () => {
      expect(() => toTaskId('CVH#001')).toThrow(DomainValidationError);
      expect(() => toTaskId('CVH@001')).toThrow(DomainValidationError);
      expect(() => toTaskId('CVH/001')).toThrow(DomainValidationError);
      expect(() => toTaskId('CVH$001')).toThrow(DomainValidationError);
      expect(() => toTaskId('CVH!001')).toThrow(DomainValidationError);
    });

    it('throws DomainValidationError on non-string inputs', () => {
      expect(() => toTaskId(null as unknown as string)).toThrow(DomainValidationError);
      expect(() => toTaskId(undefined as unknown as string)).toThrow(DomainValidationError);
      expect(() => toTaskId(123 as unknown as string)).toThrow(DomainValidationError);
    });

    it('isTaskId correctly acts as a type guard', () => {
      expect(isTaskId('CVH-001')).toBe(true);
      expect(isTaskId('TASK:42.1')).toBe(true);
      expect(isTaskId('cvh-001')).toBe(false); // lowercase is not normalized TaskId
      expect(isTaskId(' CVH-001 ')).toBe(false); // untrimmed is not normalized TaskId
      expect(isTaskId('CVH 001')).toBe(false);
      expect(isTaskId('')).toBe(false);
      expect(isTaskId(null)).toBe(false);
      expect(isTaskId(123)).toBe(false);
    });
  });

  describe('StageId', () => {
    it('normalizes uppercase and mixed-case strings to lowercase and trims whitespace', () => {
      const stageId: StageId = toStageId('  BACKLOG  ');
      expect(stageId).toBe('backlog');

      expect(toStageId('ReView')).toBe('review');
      expect(toStageId('  Dev ')).toBe('dev');
    });

    it('throws DomainValidationError on empty or whitespace-only strings', () => {
      expect(() => toStageId('')).toThrow(DomainValidationError);
      expect(() => toStageId('   ')).toThrow(DomainValidationError);
    });

    it('throws DomainValidationError on non-string inputs', () => {
      expect(() => toStageId(null as unknown as string)).toThrow(DomainValidationError);
      expect(() => toStageId(123 as unknown as string)).toThrow(DomainValidationError);
    });

    it('isStageId validates lowercase non-empty strings', () => {
      expect(isStageId('backlog')).toBe(true);
      expect(isStageId('dev')).toBe(true);
      expect(isStageId('DEV')).toBe(false);
      expect(isStageId(' dev ')).toBe(false);
      expect(isStageId('')).toBe(false);
      expect(isStageId(null)).toBe(false);
    });
  });

  describe('BoardId', () => {
    it('normalizes uppercase and mixed-case strings to lowercase and trims whitespace', () => {
      const boardId: BoardId = toBoardId('  DEVELOPMENT  ');
      expect(boardId).toBe('development');

      expect(toBoardId('KanBan')).toBe('kanban');
      expect(toBoardId(' 3D_Modeler ')).toBe('3d_modeler');
    });

    it('throws DomainValidationError on empty or whitespace-only strings', () => {
      expect(() => toBoardId('')).toThrow(DomainValidationError);
      expect(() => toBoardId('   ')).toThrow(DomainValidationError);
    });

    it('throws DomainValidationError on non-string inputs', () => {
      expect(() => toBoardId(null as unknown as string)).toThrow(DomainValidationError);
      expect(() => toBoardId(undefined as unknown as string)).toThrow(DomainValidationError);
    });

    it('isBoardId validates lowercase non-empty strings', () => {
      expect(isBoardId('development')).toBe(true);
      expect(isBoardId('DEVELOPMENT')).toBe(false);
      expect(isBoardId(' development ')).toBe(false);
      expect(isBoardId('')).toBe(false);
      expect(isBoardId(null)).toBe(false);
    });
  });

  describe('Stage Predicates (isTerminalStage, isIntakeStage)', () => {
    it('exposes canonical stage constants', () => {
      expect(CANONICAL_TERMINAL_STAGES).toContain('ready');
      expect(CANONICAL_TERMINAL_STAGES).toContain('done');
      expect(CANONICAL_INTAKE_STAGES).toContain('backlog');
    });

    it('isTerminalStage correctly identifies terminal stage strings', () => {
      expect(isTerminalStage('ready')).toBe(true);
      expect(isTerminalStage('done')).toBe(true);
      expect(isTerminalStage('READY')).toBe(true);
      expect(isTerminalStage('  Done  ')).toBe(true);
      expect(isTerminalStage(toStageId('ready'))).toBe(true);

      expect(isTerminalStage('dev')).toBe(false);
      expect(isTerminalStage('backlog')).toBe(false);
      expect(isTerminalStage('review')).toBe(false);
    });

    it('isTerminalStage correctly handles StageLike objects', () => {
      expect(isTerminalStage({ id: 'custom-finish', terminal: true })).toBe(true);
      expect(isTerminalStage({ id: 'ready', terminal: false })).toBe(true);
      expect(isTerminalStage({ id: 'done' })).toBe(true);
      expect(isTerminalStage({ id: 'dev', terminal: false })).toBe(false);
      expect(isTerminalStage({ id: 'backlog' })).toBe(false);
    });

    it('isTerminalStage safely returns false for nullish or invalid values', () => {
      expect(isTerminalStage(null)).toBe(false);
      expect(isTerminalStage(undefined)).toBe(false);
      expect(isTerminalStage('')).toBe(false);
      expect(isTerminalStage({})).toBe(false);
    });

    it('isIntakeStage correctly identifies intake stage strings', () => {
      expect(isIntakeStage('backlog')).toBe(true);
      expect(isIntakeStage('BACKLOG')).toBe(true);
      expect(isIntakeStage('  backlog  ')).toBe(true);
      expect(isIntakeStage(toStageId('backlog'))).toBe(true);

      expect(isIntakeStage('ready')).toBe(false);
      expect(isIntakeStage('dev')).toBe(false);
      expect(isIntakeStage('review')).toBe(false);
    });

    it('isIntakeStage correctly handles StageLike objects', () => {
      expect(isIntakeStage({ id: 'inbox', isIntake: true })).toBe(true);
      expect(isIntakeStage({ id: 'backlog' })).toBe(true);
      expect(isIntakeStage({ id: 'dev', isIntake: false })).toBe(false);
    });

    it('isIntakeStage safely returns false for nullish or invalid values', () => {
      expect(isIntakeStage(null)).toBe(false);
      expect(isIntakeStage(undefined)).toBe(false);
      expect(isIntakeStage('')).toBe(false);
      expect(isIntakeStage({})).toBe(false);
    });
  });

  describe('TaskState Discriminated Union & Factories', () => {
    it('validates idle state', () => {
      const state: TaskState = { status: 'idle' };
      expect(isTaskState(state)).toBe(true);
      expect(isIdleState(state)).toBe(true);
      expect(isRunningState(state)).toBe(false);
      expect(toTaskState(state)).toEqual({ status: 'idle' });
      expect(TaskStates.idle()).toEqual({ status: 'idle' });
    });

    it('validates running state with and without model', () => {
      const stateNoModel: TaskState = { status: 'running' };
      const stateWithModel: TaskState = { status: 'running', model: 'claude-3-7-sonnet' };

      expect(isTaskState(stateNoModel)).toBe(true);
      expect(isTaskState(stateWithModel)).toBe(true);
      expect(isRunningState(stateNoModel)).toBe(true);
      expect(isRunningState(stateWithModel)).toBe(true);

      expect(TaskStates.running()).toEqual({ status: 'running' });
      expect(TaskStates.running(' deepseek-chat ')).toEqual({ status: 'running', model: 'deepseek-chat' });
    });

    it('validates waiting_subtask state and requires non-empty waitingForSubtaskId', () => {
      const validState: TaskState = {
        status: 'waiting_subtask',
        waitingForSubtaskId: 'CVH-001.1',
      };
      expect(isTaskState(validState)).toBe(true);
      expect(isWaitingSubtaskState(validState)).toBe(true);
      expect(toTaskState(validState)).toEqual(validState);

      const created = TaskStates.waitingSubtask(' CVH-002.1 ');
      expect(created).toEqual({ status: 'waiting_subtask', waitingForSubtaskId: 'CVH-002.1' });

      // Invalid: missing or empty waitingForSubtaskId
      expect(isTaskState({ status: 'waiting_subtask' })).toBe(false);
      expect(isTaskState({ status: 'waiting_subtask', waitingForSubtaskId: '' })).toBe(false);
      expect(isTaskState({ status: 'waiting_subtask', waitingForSubtaskId: '   ' })).toBe(false);
      expect(isTaskState({ status: 'waiting_subtask', waitingForSubtaskId: 123 })).toBe(false);

      expect(() => TaskStates.waitingSubtask('')).toThrow(DomainValidationError);
      expect(() => TaskStates.waitingSubtask('   ')).toThrow(DomainValidationError);
    });

    it('validates done state with and without verdict', () => {
      const stateNoVerdict: TaskState = { status: 'done' };
      const stateWithVerdict: TaskState = { status: 'done', verdict: 'APPROVED' };

      expect(isTaskState(stateNoVerdict)).toBe(true);
      expect(isTaskState(stateWithVerdict)).toBe(true);
      expect(isDoneState(stateNoVerdict)).toBe(true);
      expect(isDoneState(stateWithVerdict)).toBe(true);

      expect(TaskStates.done()).toEqual({ status: 'done' });
      expect(TaskStates.done(' APPROVED ')).toEqual({ status: 'done', verdict: 'APPROVED' });
    });

    it('rejects invalid TaskState candidates', () => {
      expect(isTaskState(null)).toBe(false);
      expect(isTaskState(undefined)).toBe(false);
      expect(isTaskState('idle')).toBe(false);
      expect(isTaskState(123)).toBe(false);
      expect(isTaskState({})).toBe(false);
      expect(isTaskState({ status: 'unknown_status' })).toBe(false);
      expect(isTaskState({ status: 'running', model: 123 })).toBe(false);
      expect(isTaskState({ status: 'done', verdict: 123 })).toBe(false);

      expect(() => toTaskState({ status: 'invalid' })).toThrow(DomainValidationError);
      expect(() => toTaskState(null)).toThrow(DomainValidationError);
    });
  });
});

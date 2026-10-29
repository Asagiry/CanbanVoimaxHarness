/**
 * Core Domain Layer Value Objects, Branded Types, and Type Factories.
 * Implements Uncle Bob's Clean Architecture & DDD principles:
 * - Make illegal states unrepresentable
 * - Validate once at boundary, trust inside
 * - Zero runtime overhead for nominal branded types
 */

/**
 * Nominal branding symbol used to enforce compile-time type boundaries.
 */
declare const __brand: unique symbol;

export type Brand<T, B extends string> = T & { readonly [__brand]: B };

/**
 * Branded nominal type for Task identifiers (e.g. "CVH-001", "TASK:42.1").
 * Guarantees uppercase, trimmed, and validated against ^[A-Z0-9-_.:]+$.
 */
export type TaskId = Brand<string, 'TaskId'>;

/**
 * Branded nominal type for Stage identifiers (e.g. "backlog", "dev", "review", "ready").
 * Guarantees lowercase and trimmed.
 */
export type StageId = Brand<string, 'StageId'>;

/**
 * Branded nominal type for Board identifiers (e.g. "development", "kanban").
 * Guarantees lowercase and trimmed.
 */
export type BoardId = Brand<string, 'BoardId'>;

/**
 * Domain validation error thrown when input fails value object constraints.
 */
export class DomainValidationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'DomainValidationError';
    Object.setPrototypeOf(this, DomainValidationError.prototype);
  }
}

export const TASK_ID_REGEX = /^[A-Z0-9\-_.:]+$/;
export const CANONICAL_TERMINAL_STAGES = Object.freeze(['ready', 'done'] as const);
export const CANONICAL_INTAKE_STAGES = Object.freeze(['backlog'] as const);

/**
 * Factory with validation for TaskId.
 * Normalizes input string to UPPERCASE, trims whitespace, and validates against ^[A-Z0-9-_.:]+$.
 */
export function toTaskId(raw: string): TaskId {
  if (typeof raw !== 'string') {
    throw new DomainValidationError(`Expected string for TaskId, received ${typeof raw}`);
  }
  const normalized = raw.trim().toUpperCase();
  if (!normalized || !TASK_ID_REGEX.test(normalized)) {
    throw new DomainValidationError(
      `Invalid TaskId: "${raw}". Normalized value "${normalized}" does not match ^[A-Z0-9-_.:]+$`
    );
  }
  return normalized as TaskId;
}

/**
 * Type guard checking if an unknown value is a valid TaskId.
 */
export function isTaskId(raw: unknown): raw is TaskId {
  return typeof raw === 'string' && TASK_ID_REGEX.test(raw);
}

/**
 * Factory with validation for StageId.
 * Normalizes input string to lowercase and trims whitespace.
 */
export function toStageId(raw: string): StageId {
  if (typeof raw !== 'string') {
    throw new DomainValidationError(`Expected string for StageId, received ${typeof raw}`);
  }
  const normalized = raw.trim().toLowerCase();
  if (!normalized) {
    throw new DomainValidationError('Invalid StageId: cannot be empty');
  }
  return normalized as StageId;
}

/**
 * Type guard checking if an unknown value is a valid StageId.
 */
export function isStageId(raw: unknown): raw is StageId {
  return typeof raw === 'string' && raw.trim().length > 0 && raw === raw.trim().toLowerCase();
}

/**
 * Factory with validation for BoardId.
 * Normalizes input string to lowercase and trims whitespace.
 */
export function toBoardId(raw: string): BoardId {
  if (typeof raw !== 'string') {
    throw new DomainValidationError(`Expected string for BoardId, received ${typeof raw}`);
  }
  const normalized = raw.trim().toLowerCase();
  if (!normalized) {
    throw new DomainValidationError('Invalid BoardId: cannot be empty');
  }
  return normalized as BoardId;
}

/**
 * Type guard checking if an unknown value is a valid BoardId.
 */
export function isBoardId(raw: unknown): raw is BoardId {
  return typeof raw === 'string' && raw.trim().length > 0 && raw === raw.trim().toLowerCase();
}

/**
 * Discriminated union of task execution states.
 * Guarantees that illegal states (e.g. waiting_subtask without waitingForSubtaskId) are unrepresentable.
 */
export type TaskState =
  | { status: 'idle' }
  | { status: 'running'; model?: string }
  | { status: 'waiting_subtask'; waitingForSubtaskId: string }
  | { status: 'done'; verdict?: string };

export type TaskStateStatus = TaskState['status'];

/**
 * Factory constructors for creating valid TaskState instances.
 */
export const TaskStates = {
  idle: (): TaskState => ({ status: 'idle' }),
  running: (model?: string): TaskState => (model && model.trim() ? { status: 'running', model: model.trim() } : { status: 'running' }),
  waitingSubtask: (waitingForSubtaskId: string): TaskState => {
    if (typeof waitingForSubtaskId !== 'string' || !waitingForSubtaskId.trim()) {
      throw new DomainValidationError('waitingForSubtaskId must be a non-empty string');
    }
    return { status: 'waiting_subtask', waitingForSubtaskId: waitingForSubtaskId.trim() };
  },
  done: (verdict?: string): TaskState => (verdict && verdict.trim() ? { status: 'done', verdict: verdict.trim() } : { status: 'done' }),
} as const;

/**
 * Type guard validating that a candidate object is a valid TaskState.
 */
export function isTaskState(candidate: unknown): candidate is TaskState {
  if (!candidate || typeof candidate !== 'object') return false;
  const state = candidate as Record<string, unknown>;
  switch (state.status) {
    case 'idle':
      return true;
    case 'running':
      return state.model === undefined || (typeof state.model === 'string' && state.model.trim().length > 0);
    case 'waiting_subtask':
      return typeof state.waitingForSubtaskId === 'string' && state.waitingForSubtaskId.trim().length > 0;
    case 'done':
      return state.verdict === undefined || typeof state.verdict === 'string';
    default:
      return false;
  }
}

/**
 * Validates and converts an unknown candidate into a TaskState.
 * Throws DomainValidationError if the candidate is invalid.
 */
export function toTaskState(candidate: unknown): TaskState {
  if (!isTaskState(candidate)) {
    throw new DomainValidationError(`Invalid TaskState candidate: ${JSON.stringify(candidate)}`);
  }
  return candidate;
}

/**
 * Specialized type predicates for TaskState.
 */
export function isIdleState(state: TaskState): state is { status: 'idle' } {
  return state.status === 'idle';
}

export function isRunningState(state: TaskState): state is { status: 'running'; model?: string } {
  return state.status === 'running';
}

export function isWaitingSubtaskState(
  state: TaskState
): state is { status: 'waiting_subtask'; waitingForSubtaskId: string } {
  return state.status === 'waiting_subtask';
}

export function isDoneState(state: TaskState): state is { status: 'done'; verdict?: string } {
  return state.status === 'done';
}

/**
 * Structural contract for stage-like objects (e.g. PipelineColumn).
 */
export interface StageLike {
  id?: string;
  terminal?: boolean;
  isIntake?: boolean;
}

/**
 * Predicate checking if a stage identifier or column configuration is a terminal/sink stage.
 * Matches canonical terminal stages ("ready", "done") or columns explicitly marked with terminal: true.
 */
export function isTerminalStage(stage: string | StageLike | null | undefined): boolean {
  if (!stage) return false;
  if (typeof stage === 'string') {
    const normalized = stage.trim().toLowerCase();
    return CANONICAL_TERMINAL_STAGES.includes(normalized as (typeof CANONICAL_TERMINAL_STAGES)[number]);
  }
  if (stage.terminal === true) return true;
  if (typeof stage.id === 'string') {
    return isTerminalStage(stage.id);
  }
  return false;
}

/**
 * Predicate checking if a stage identifier or column configuration is an intake stage.
 * Matches canonical intake stages ("backlog") or columns explicitly marked with isIntake: true.
 */
export function isIntakeStage(stage: string | StageLike | null | undefined): boolean {
  if (!stage) return false;
  if (typeof stage === 'string') {
    const normalized = stage.trim().toLowerCase();
    return CANONICAL_INTAKE_STAGES.includes(normalized as (typeof CANONICAL_INTAKE_STAGES)[number]);
  }
  if (stage.isIntake === true) return true;
  if (typeof stage.id === 'string') {
    return isIntakeStage(stage.id);
  }
  return false;
}

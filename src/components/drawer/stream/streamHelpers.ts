import type { DrawerActivityItem } from '../../../stores/drawerStore';
import { tryExtractTodos, type ExtractedTodosResult } from '../../../lib/markdown';
import { filterDisplayActivities, resolveActivityToolName } from './activityFilter';

export { filterDisplayActivities };

/**
 * Clean prompt helper: strips legacy appended [USER STEER] or [USER]: blocks for pure prompt display
 */
export function getCleanPrompt(description?: string, title?: string): string {
  const raw = (description || title || '').trim();
  const steerIdx = raw.search(/\n\s*\[USER(?:\s+STEER)?\]:/i);
  if (steerIdx !== -1) {
    return raw.slice(0, steerIdx).trim();
  }
  return raw;
}

/**
 * Evaluates whether an activity item represents the true final assistant response
 * in a turn vs intermediate step commentary preceding tool calls.
 */
export function isFinalResponseItem(
  activity: DrawerActivityItem,
  idx: number,
  activities: DrawerActivityItem[],
  isRunning: boolean
): boolean {
  if (activity.type !== 'output' && (activity.type as string) !== 'summary') return false;

  let turnHasEnded = false;
  let hasInterveningToolsOrOutputs = false;

  for (let i = idx + 1; i < activities.length; i++) {
    const next = activities[i];
    if (next.type === 'user' || next.type === 'steer' || next.type === 'prompt') {
      turnHasEnded = true;
      break;
    }
    if (
      next.type === 'tool' ||
      Boolean(next.tool) ||
      next.type === 'output' ||
      (next.type as string) === 'summary'
    ) {
      hasInterveningToolsOrOutputs = true;
      break;
    }
  }

  if (hasInterveningToolsOrOutputs) {
    return false;
  }

  if (activity.isFinal) {
    return true;
  }

  if (!turnHasEnded && isRunning) {
    return false;
  }

  if (turnHasEnded || !isRunning) {
    return true;
  }

  return false;
}

/**
 * Detects whether a todo_write tool call is within a small window of the given activity.
 */
export function hasNearbyTodoWrite(idx: number, activities: DrawerActivityItem[]): boolean {
  const windowSize = 3;
  const start = Math.max(0, idx - windowSize);
  const end = Math.min(activities.length - 1, idx + windowSize);
  for (let i = start; i <= end; i++) {
    if (i === idx) continue;
    const act = activities[i];
    if (act?.type === 'tool' && resolveActivityToolName(act.tool) === 'todo_write') {
      return true;
    }
  }
  return false;
}

/**
 * Determines whether an intermediate explanation should be suppressed
 * (e.g., duplicated identical repeated messages).
 */
export function shouldSuppressExplanation(
  activity: DrawerActivityItem,
  idx: number,
  activities: DrawerActivityItem[]
): boolean {
  const explanationText = (activity?.content || '').trim();
  if (!explanationText) return true;
  if (activity.id.endsWith(':output')) return false;
  const expLower = explanationText.toLowerCase();

  for (let i = idx - 1; i >= 0; i--) {
    const act = activities[i];
    if (act?.type === 'output' || !act?.type) {
      const prevText = (act.content || '').trim();
      if (prevText) {
        if (prevText === explanationText || prevText.toLowerCase() === expLower) {
          return true;
        }
        break;
      }
    }
  }
  for (let i = idx - 1; i >= Math.max(0, idx - 5); i--) {
    const act = activities[i];
    if (act?.type === 'output' || !act?.type) {
      const prevText = (act.content || '').trim();
      if (prevText && (prevText === explanationText || prevText.toLowerCase() === expLower)) {
        return true;
      }
    }
  }

  return false;
}

export { computeActivityDurations } from '../../../stores/drawer/drawerHelpers';

export interface ActivityFlag {
  isFinal: boolean;
  nearbyTodo: boolean;
  suppressed: boolean;
  todoData: ExtractedTodosResult | null;
}

export function computeActivityFlags(
  activities: DrawerActivityItem[],
  isRunning: boolean
): ActivityFlag[] {
  const list = activities ?? [];
  const n = list.length;
  const flags: ActivityFlag[] = Array.from({ length: n });

  const isTodoWrite = (act: DrawerActivityItem | undefined): boolean =>
    Boolean(act && act.type === 'tool' && resolveActivityToolName(act.tool) === 'todo_write');

  const isTurnEnd = (act: DrawerActivityItem | undefined): boolean =>
    Boolean(act && (act.type === 'user' || act.type === 'steer' || act.type === 'prompt'));

  const isIntervening = (act: DrawerActivityItem | undefined): boolean =>
    Boolean(
      act &&
        (act.type === 'tool' ||
          Boolean(act.tool) ||
          act.type === 'output' ||
          (act.type as string) === 'summary')
    );

  const nextTurnEnd = Array.from({ length: n + 1 }, () => false);
  const nextIntervening = Array.from({ length: n + 1 }, () => false);
  for (let i = n - 1; i >= 0; i--) {
    nextTurnEnd[i] = nextTurnEnd[i + 1] || isTurnEnd(list[i]);
    nextIntervening[i] = nextIntervening[i + 1] || isIntervening(list[i]);
  }

  const todoIndices: number[] = [];
  for (let i = 0; i < n; i++) if (isTodoWrite(list[i])) todoIndices.push(i);

  const nearbyTodo = Array.from({ length: n }, () => false);
  for (const ti of todoIndices) {
    for (let i = Math.max(0, ti - 3); i <= Math.min(n - 1, ti + 3); i++) {
      if (i !== ti) nearbyTodo[i] = true;
    }
  }

  let prevOutputText: string | null = null;
  let prevOutputLower: string | null = null;
  let prevOutputIdx = -1;

  for (let idx = 0; idx < n; idx++) {
    const activity = list[idx];

    let isFinal = false;
    if (activity.type === 'output' || (activity.type as string) === 'summary') {
      const hasIntervening = idx + 1 < n ? nextIntervening[idx + 1] : false;
      const turnHasEnded = idx + 1 < n ? nextTurnEnd[idx + 1] : false;
      if (!hasIntervening) {
        if (activity.isFinal) isFinal = true;
        else if (!turnHasEnded && isRunning) isFinal = false;
        else if (turnHasEnded || !isRunning) isFinal = true;
      }
    }

    let suppressed = false;
    const explanationText = (activity?.content || '').trim();
    if (!explanationText) {
      suppressed = true;
    } else if (activity.id.endsWith(':output')) {
      suppressed = false;
    } else {
      const expLower = explanationText.toLowerCase();
      if (
        prevOutputText !== null &&
        prevOutputIdx >= idx - 5 &&
        (prevOutputText === explanationText || prevOutputLower === expLower)
      ) {
        suppressed = true;
      }
    }

    const todoData = explanationText ? tryExtractTodos(explanationText) : null;

    flags[idx] = { isFinal, nearbyTodo: nearbyTodo[idx], suppressed, todoData };

    if (activity.type === 'output' || !activity.type) {
      const prevText = (activity.content || '').trim();
      if (prevText) {
        prevOutputText = prevText;
        prevOutputLower = prevText.toLowerCase();
        prevOutputIdx = idx;
      }
    }
  }

  return flags;
}

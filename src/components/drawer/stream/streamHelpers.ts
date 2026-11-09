import type { DrawerActivityItem } from '../../../stores/drawerStore';
import { tryExtractTodos, type ExtractedTodosResult } from '../../../lib/markdown';

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
      (next as any).tool ||
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
    if (act?.type === 'tool') {
      const tName = act.tool?.toolName || (act.tool as any)?.name || (act.tool as any)?.tool;
      if (tName === 'todo_write') return true;
    }
  }
  return false;
}

/**
 * Determines whether an intermediate explanation should be suppressed
 * (e.g., duplicated identical repeated messages).
 * Note: Summary text from advance_task / move_task / reject_task represents
 * the agent's final response and must never be suppressed.
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

  // Suppress repeated duplicate Agent Explanation cards if content is identical to previous agent message
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

/**
 * Deduplicates question activities, tool executions, and filters legacy foreign stage summaries for root tasks.
 */
export function filterDisplayActivities(
  unifiedActivities: DrawerActivityItem[],
  currentTaskId?: string
): DrawerActivityItem[] {
  const list = unifiedActivities || [];
  if (list.length === 0) return list;

  const isRootTask = Boolean(currentTaskId && !currentTaskId.includes('.'));
  const foreignSummaryRegex = /Саммари этапа \(передано на \S+\.(?:[2-9]|\d{2,})/i;

  const questionItems = list.filter((a) => a?.type === 'question');
  const questionSignatures = new Set<string>();
  if (questionItems.length > 0) {
    for (const q of questionItems) {
      if (q.id) questionSignatures.add(q.id);
      if (q.id?.startsWith('q-')) questionSignatures.add(q.id.replace(/^q-/, ''));
      const qParams = q.tool?.params as Record<string, unknown> | undefined;
      const qId = qParams?.questionId as string | undefined;
      if (qId) questionSignatures.add(qId);
      const callId = qParams?.callId as string | undefined;
      if (callId) questionSignatures.add(callId);
      if (q.tool?.id) questionSignatures.add(q.tool.id);
      if (q.content?.trim()) questionSignatures.add(`txt:${q.content.trim()}`);
      if (typeof qParams?.text === 'string' && qParams.text.trim()) {
        questionSignatures.add(`txt:${qParams.text.trim()}`);
      }
    }
  }

  const seenQuestionIds = new Set<string>();
  const seenHandoverSteers = new Set<string>();

  const filteredQuestions = list.filter((activity) => {
    if (activity?.type === 'steer' && activity?.content?.includes('[ОТЧЕТ ОБ ИСПРАВЛЕНИЯХ / HANDOVER SUMMARY]')) {
      const key = activity.content.trim();
      if (seenHandoverSteers.has(key)) return false;
      seenHandoverSteers.add(key);
    }
    if (isRootTask && (activity?.type as string) === 'summary') {
      const content = activity?.content || '';
      if (
        content.includes('Саммари финализации задачи') ||
        foreignSummaryRegex.test(content)
      ) {
        return false;
      }
    }

    if (activity?.type === 'question') {
      const qParams = activity.tool?.params as Record<string, unknown> | undefined;
      const qId =
        (qParams?.questionId as string | undefined) ||
        activity.tool?.id ||
        activity.id;
      if (qId) {
        if (seenQuestionIds.has(qId)) return false;
        seenQuestionIds.add(qId);
      }
      return true;
    }

    if (
      activity?.type === 'tool' &&
      (activity.tool?.toolName === 'question' || activity.tool?.toolName === 'ask_user')
    ) {
      const tParams = activity.tool?.params as Record<string, unknown> | undefined;
      const tId = activity.tool?.id || activity.id;
      const qId = (tParams?.questionId as string | undefined) || '';
      const callId = (tParams?.callId as string | undefined) || '';
      const text = (
        (typeof tParams?.text === 'string' ? tParams.text : '') ||
        activity.content ||
        ''
      ).trim();

      if (
        (tId && (questionSignatures.has(tId) || questionSignatures.has(`q-${tId}`))) ||
        (qId && questionSignatures.has(qId)) ||
        (callId && questionSignatures.has(callId)) ||
        (text && questionSignatures.has(`txt:${text}`))
      ) {
        return false;
      }
    }
    return true;
  });

  // Second pass: deduplicate identical adjacent / near-identical tool calls
  const result: DrawerActivityItem[] = [];
  for (let i = 0; i < filteredQuestions.length; i++) {
    const act = filteredQuestions[i];
    if (act?.type === 'tool' && act.tool) {
      const tName = act.tool.toolName || (act.tool as any)?.name;

      const isBatchStep = Boolean(act.tool?.batch && (act.tool.batch.total ?? 0) > 1);

      // Smart Todo deduplication: if multiple todo_write snapshots share the same completed count
      // (e.g. duplicate startup runs or zero-progress updates), keep only the latest checklist snapshot
      if ((tName === 'todo_write' || tName === 'write_todos') && !isBatchStep) {
        const actParams = (act.tool?.params as Record<string, unknown> | undefined)?.todos;
        let actTodos: unknown[] = [];
        if (Array.isArray(actParams)) {
          actTodos = actParams;
        } else if (typeof actParams === 'string') {
          try {
            const p = JSON.parse(actParams);
            if (Array.isArray(p)) actTodos = p;
          } catch {
            // ignore
          }
        }
        const actCompleted = actTodos.filter((t: any) => t?.status === 'completed').length;

        const existingTodoIdx = result.findIndex((prev) => {
          if (prev?.type !== 'tool' || !prev.tool) return false;
          if (prev.tool.batch && (prev.tool.batch.total ?? 0) > 1) return false;
          const pName = prev.tool.toolName || (prev.tool as any)?.name;
          if (pName !== 'todo_write' && pName !== 'write_todos') return false;
          const prevParams = (prev.tool?.params as Record<string, unknown> | undefined)?.todos;
          let prevTodos: unknown[] = [];
          if (Array.isArray(prevParams)) {
            prevTodos = prevParams;
          } else if (typeof prevParams === 'string') {
            try {
              const p = JSON.parse(prevParams);
              if (Array.isArray(p)) prevTodos = p;
            } catch {
              // ignore
            }
          }
          const prevCompleted = prevTodos.filter((t: any) => t?.status === 'completed').length;
          return prevCompleted === actCompleted;
        });

        if (existingTodoIdx !== -1) {
          continue;
        }
      }

      if (!isBatchStep) {
        const isDuplicate = result.slice(-3).some((prev) => {
          if (prev?.type !== 'tool' || !prev.tool) return false;
          const prevName = prev.tool.toolName || (prev.tool as any)?.name;
          if (!prevName || prevName !== tName) return false;
          return JSON.stringify(prev.tool.params) === JSON.stringify(act.tool?.params);
        });
        if (isDuplicate) {
          continue;
        }
      }
    }
    result.push(act);
  }

  return result;
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

  const isTodoWrite = (act: DrawerActivityItem | undefined): boolean => {
    if (!act || act.type !== 'tool') return false;
    const tName = act.tool?.toolName || (act.tool as any)?.name || (act.tool as any)?.tool;
    return tName === 'todo_write';
  };

  const isTurnEnd = (act: DrawerActivityItem | undefined): boolean =>
    !!act && (act.type === 'user' || act.type === 'steer' || act.type === 'prompt');

  const isIntervening = (act: DrawerActivityItem | undefined): boolean =>
    !!act &&
    (act.type === 'tool' ||
      !!(act as any).tool ||
      act.type === 'output' ||
      (act.type as string) === 'summary');

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

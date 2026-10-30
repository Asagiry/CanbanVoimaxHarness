import type { DrawerActivityItem } from '../../../stores/drawerStore';

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
 * Deduplicates question activities and tool executions according to § 2.20 invariant.
 */
export function filterDisplayActivities(unifiedActivities: DrawerActivityItem[]): DrawerActivityItem[] {
  const list = unifiedActivities || [];
  const questionItems = list.filter((a) => a?.type === 'question');
  if (questionItems.length === 0) return list;

  const questionSignatures = new Set<string>();
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

  const seenQuestionIds = new Set<string>();

  return list.filter((activity) => {
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
}

/**
 * Pre-computes elapsed durations for activity items based on timestamps or explicit durationMs.
 */
export function computeActivityDurations(
  activities: DrawerActivityItem[],
  isRunning: boolean
): (number | undefined)[] {
  const list = activities || [];
  const res: (number | undefined)[] = Array.from({ length: list.length });
  for (let i = 0; i < list.length; i++) {
    const act = list[i];
    if (act?.durationMs && act.durationMs > 0) {
      res[i] = act.durationMs;
    } else if (act?.tool?.durationMs && act.tool.durationMs > 0) {
      res[i] = act.tool.durationMs;
    } else {
      const curTime = act?.timestamp ? new Date(act.timestamp).getTime() : NaN;
      if (!isNaN(curTime) && curTime > 0) {
        let nextTime = NaN;
        for (let j = i + 1; j < list.length; j++) {
          const t = list[j]?.timestamp ? new Date(list[j].timestamp).getTime() : NaN;
          if (!isNaN(t) && t > curTime) {
            nextTime = t;
            break;
          }
        }
        if (!isNaN(nextTime) && nextTime >= curTime) {
          const diff = nextTime - curTime;
          if (diff > 50 && diff < 3600000) {
            res[i] = diff;
          }
        } else if (i === list.length - 1 && isRunning) {
          const diff = Date.now() - curTime;
          if (diff > 50 && diff < 3600000) {
            res[i] = diff;
          }
        }
      }
    }
  }
  return res;
}

import type { DrawerActivityItem } from '../../../stores/drawerStore';
import type { KanbanTask } from '../../../types';
import type { Language } from '../../../i18n/types';

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

export interface ColumnSummaryInfo {
  id: string;
  title: string;
  persona?: string | null;
}

const SUMMARY_LABELS: Record<
  Language,
  {
    planner: string;
    developer: string;
    reviewer: string;
    retro: string;
    final: string;
    stage: (stage: string) => string;
    default: string;
  }
> = {
  ru: {
    planner: 'Саммари планера',
    developer: 'Саммари девелопера',
    reviewer: 'Саммари ревьюера',
    retro: 'Саммари ретро',
    final: 'Финальное саммари',
    stage: (s) => `Саммари этапа: ${s}`,
    default: 'Саммари этапа',
  },
  en: {
    planner: 'Planner Summary',
    developer: 'Developer Summary',
    reviewer: 'Reviewer Summary',
    retro: 'Retro Summary',
    final: 'Final Summary',
    stage: (s) => `Stage Summary: ${s}`,
    default: 'Stage Summary',
  },
};

export function classifyStageRole(raw?: string): 'planner' | 'developer' | 'reviewer' | 'retro' | 'final' | null {
  if (!raw) return null;
  const s = raw.toLowerCase().trim();
  if (s === 'plan' || s === 'planner' || s === 'planning' || s.includes('план') || s.includes('planner') || s.includes('planning')) {
    return 'planner';
  }
  if (s === 'dev' || s === 'developer' || s === 'cvh-dev' || s === 'development' || s.includes('девелоп') || s.includes('разработ') || s.includes('develop')) {
    return 'developer';
  }
  if (s === 'review' || s === 'reviewer' || s.includes('ревью') || s.includes('review')) {
    return 'reviewer';
  }
  if (s === 'retro' || s === 'retrospective' || s.includes('ретро') || s.includes('retro')) {
    return 'retro';
  }
  if (s === 'ready' || s === 'done' || s.includes('готов') || s.includes('финализац') || s.includes('final')) {
    return 'final';
  }
  return null;
}

/**
 * Strips trailing parenthesized stage name from task or subtask title
 * (e.g. "Task (Planning)" with stage "Planning" -> "Task")
 */
export function stripDuplicateStageInTitle(title: string | undefined, stageTitle?: string): string {
  if (!title) return '';
  if (!stageTitle || !stageTitle.trim()) return title.trim();
  const escaped = stageTitle.trim().replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const re = new RegExp(`\\s*\\(${escaped}\\)\\s*$`, 'i');
  return title.replace(re, '').trim();
}

/**
 * Strips leading redundant markdown headings from summary content such as
 * "### Саммари этапа..." or "### Саммари финализации..."
 */
export function getCleanSummaryContent(content?: string): string {
  if (!content) return '';
  const headerRegex = /^\s*#{1,6}\s*(?:\[?(?:Саммари\s+(?:этапа|финализации)|Stage\s+Summary|Handover\s+Summary)[^\]\n\r]*\]?:?[^\n\r]*)(?:\r?\n)+/i;
  return content.replace(headerRegex, '').replace(/^\s*(?:\r?\n)+/, '');
}

/**
 * Resolves deslopped dynamic stage summary label (e.g. "Саммари планера", "Developer Summary")
 */
export function getStageSummaryLabel(
  activity: DrawerActivityItem,
  task: KanbanTask,
  subtasks?: KanbanTask[],
  columns?: ColumnSummaryInfo[],
  lang: Language = 'ru'
): string {
  const labels = SUMMARY_LABELS[lang] || SUMMARY_LABELS.ru;
  const content = activity?.content || '';

  if (/###\s*Саммари\s+финализации/i.test(content)) return labels.final;

  const stdMatch = content.match(/###\s*Саммари\s+этапа:\s*([^\n\r]+)/i);
  if (stdMatch?.[1]) {
    const rawStage = stdMatch[1].trim();
    const role = classifyStageRole(rawStage);
    if (role) return labels[role];
    if (rawStage) return labels.stage(rawStage);
  }

  const legacyMatch = content.match(/###\s*Саммари\s+этапа\s+([a-zA-Z0-9_-]+):/i);
  if (legacyMatch?.[1]) {
    const stageId = legacyMatch[1].trim();
    const role = classifyStageRole(stageId);
    if (role) return labels[role];
    const col = columns?.find((c) => c.id.toLowerCase() === stageId.toLowerCase());
    if (col) {
      const colRole = classifyStageRole(col.title) || classifyStageRole(col.persona || '');
      if (colRole) return labels[colRole];
      return labels.stage(col.title);
    }
  }

  const handoverMatch = content.match(/передано на\s+([A-Za-z0-9_.-]+)\s+([^)]*)/i);
  if (handoverMatch) {
    const targetSubId = handoverMatch[1].trim();
    const targetTitle = handoverMatch[2].trim();
    if (subtasks && subtasks.length > 0) {
      const sorted = [...subtasks].sort((a, b) => (a.subtaskIndex ?? 0) - (b.subtaskIndex ?? 0));
      const idx = sorted.findIndex((s) => s.id === targetSubId);
      if (idx > 0) {
        const prevSub = sorted[idx - 1];
        const prevRole = classifyStageRole(prevSub.stageId as string) || classifyStageRole(prevSub.persona);
        if (prevRole) return labels[prevRole];
        const col = columns?.find((c) => c.id === prevSub.stageId);
        if (col) return labels.stage(col.title);
      }
    }
    const targetRole = classifyStageRole(targetTitle);
    if (targetRole === 'developer') return labels.planner;
    if (targetRole === 'reviewer') return labels.developer;
    if (targetRole === 'retro') return labels.reviewer;
    if (targetRole === 'final') return labels.reviewer;
  }

  if (activity?.tool) {
    const params = (activity.tool.params as Record<string, unknown>) || {};
    const sourceParam = (params.source_stage || params.from_stage || '') as string;
    const role = classifyStageRole(sourceParam);
    if (role) return labels[role];
  }

  if (task.parentId || (task.id && task.id.includes('.'))) {
    const role = classifyStageRole(task.stageId as string) || classifyStageRole(task.persona);
    if (role) return labels[role];
    const col = columns?.find((c) => c.id === task.stageId);
    if (col) {
      const colRole = classifyStageRole(col.title) || classifyStageRole(col.persona || '');
      if (colRole) return labels[colRole];
      return labels.stage(col.title);
    }
  }

  if (subtasks && subtasks.length > 0) {
    for (const sub of subtasks) {
      if (sub.handoverSummary && content && (content.includes(sub.handoverSummary) || sub.handoverSummary.includes(content.trim()))) {
        const role = classifyStageRole(sub.stageId as string) || classifyStageRole(sub.persona);
        if (role) return labels[role];
        const col = columns?.find((c) => c.id === sub.stageId);
        if (col) return labels.stage(col.title);
      }
    }
  }

  if (task.stageId) {
    const role = classifyStageRole(task.stageId as string) || classifyStageRole(task.persona);
    if (role) return labels[role];
    const col = columns?.find((c) => c.id === task.stageId);
    if (col) return labels.stage(col.title);
  }

  return labels.default;
}

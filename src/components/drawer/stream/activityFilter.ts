import type { DrawerActivityItem } from '../../../stores/drawerStore';
import type { ToolExecution } from '../../../types';

export function resolveActivityToolName(tool: ToolExecution | undefined): string | undefined {
  if (!tool) return undefined;
  const legacy = tool as unknown as { name?: string; tool?: string };
  return tool.toolName || legacy.name || legacy.tool;
}

function parseTodosList(raw: unknown): unknown[] {
  if (Array.isArray(raw)) return raw;
  if (typeof raw === 'string') {
    try {
      const parsed: unknown = JSON.parse(raw);
      if (Array.isArray(parsed)) return parsed;
    } catch (err) {
      console.debug('[activityFilter] Failed to parse todos JSON string:', err);
    }
  }
  return [];
}

function countCompletedTodos(items: unknown[]): number {
  return items.filter(
    (entry) => entry && typeof entry === 'object' && (entry as { status?: unknown }).status === 'completed'
  ).length;
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

  const result: DrawerActivityItem[] = [];
  for (let i = 0; i < filteredQuestions.length; i++) {
    const act = filteredQuestions[i];
    if (act?.type === 'tool' && act.tool) {
      const tName = resolveActivityToolName(act.tool);
      const isBatchStep = Boolean(act.tool?.batch && (act.tool.batch.total ?? 0) > 1);

      if ((tName === 'todo_write' || tName === 'write_todos') && !isBatchStep) {
        const actParams = (act.tool?.params as Record<string, unknown> | undefined)?.todos;
        const actCompleted = countCompletedTodos(parseTodosList(actParams));

        const existingTodoIdx = result.findIndex((prev) => {
          if (prev?.type !== 'tool' || !prev.tool) return false;
          if (prev.tool.batch && (prev.tool.batch.total ?? 0) > 1) return false;
          const pName = resolveActivityToolName(prev.tool);
          if (pName !== 'todo_write' && pName !== 'write_todos') return false;
          const prevParams = (prev.tool?.params as Record<string, unknown> | undefined)?.todos;
          return countCompletedTodos(parseTodosList(prevParams)) === actCompleted;
        });

        if (existingTodoIdx !== -1) {
          continue;
        }
      }

      if (!isBatchStep) {
        const isDuplicate = result.slice(-3).some((prev) => {
          if (prev?.type !== 'tool' || !prev.tool) return false;
          const prevName = resolveActivityToolName(prev.tool);
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

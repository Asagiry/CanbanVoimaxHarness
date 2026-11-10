import { ToolExecution, LiveStreamEntry } from '../../../types';
import { DrawerActivityItem } from '../types';
import { computeActivityDurations } from '../drawerHelpers';
import { capActivityFeed, capToolExecutions } from './windowing';
import { extractSubagentIdsFromTool } from './toolMerge';

/**
 * Deduplicates questions, adjacent identical steer/user/output/summary items, and duplicate tool calls loaded from SQLite.
 */
export function deduplicateLoadedActivityItems(
  items: DrawerActivityItem[]
): DrawerActivityItem[] {
  const dedupedItems: DrawerActivityItem[] = [];
  const loadedQuestionIds = new Set<string>();
  const loadedQuestionTexts = new Set<string>();

  for (const item of items) {
    if (item.type === 'question') {
      const qParams = item.tool?.params as Record<string, unknown> | undefined;
      const qId = (qParams?.questionId as string) || item.tool?.id || item.id;
      if (qId) loadedQuestionIds.add(qId);
      if (item.id?.startsWith('q-')) loadedQuestionIds.add(item.id.replace(/^q-/, ''));
      const callId = qParams?.callId as string | undefined;
      if (callId) loadedQuestionIds.add(callId);
      if (item.content?.trim()) loadedQuestionTexts.add(item.content.trim());
      if (typeof qParams?.text === 'string' && qParams.text.trim()) {
        loadedQuestionTexts.add(qParams.text.trim());
      }
    }
  }

  for (const item of items) {
    if (
      item.type === 'tool' &&
      (item.tool?.toolName === 'question' || item.tool?.toolName === 'ask_user')
    ) {
      const tParams = item.tool?.params as Record<string, unknown> | undefined;
      const tId = item.tool?.id || item.id;
      const qId = tParams?.questionId as string | undefined;
      const callId = tParams?.callId as string | undefined;
      const txt = (
        (typeof tParams?.text === 'string' ? tParams.text : '') ||
        item.content ||
        ''
      ).trim();

      if (
        (tId && (loadedQuestionIds.has(tId) || loadedQuestionIds.has(`q-${tId}`))) ||
        (qId && loadedQuestionIds.has(qId)) ||
        (callId && loadedQuestionIds.has(callId)) ||
        (txt && loadedQuestionTexts.has(txt))
      ) {
        continue;
      }
    }

    const prev = dedupedItems[dedupedItems.length - 1];

    if (
      item.type === 'tool' &&
      item.tool &&
      dedupedItems.slice(-3).some((d) => {
        if (d.type !== 'tool' || !d.tool) return false;
        const dName = d.tool.toolName || (d.tool as { name?: string }).name;
        const iName = item.tool?.toolName || (item.tool as { name?: string })?.name;
        if (!dName || dName !== iName) return false;
        return JSON.stringify(d.tool.params) === JSON.stringify(item.tool?.params);
      })
    ) {
      continue;
    }

    if (
      prev &&
      (item.type === 'steer' || item.type === 'user') &&
      (prev.type === 'steer' || prev.type === 'user') &&
      item.content?.trim() === prev.content?.trim() &&
      (item.id === prev.id ||
        (item.id.startsWith('out-') !== prev.id.startsWith('out-') &&
          ((Boolean(item.timestamp) && item.timestamp === prev.timestamp) ||
            Boolean(
              item.createdAt &&
                prev.createdAt &&
                Math.abs(item.createdAt - prev.createdAt) <= 5000
            ))))
    ) {
      if (item.type === 'steer') {
        prev.type = 'steer';
      }
      if (item.images && item.images.length > 0) {
        prev.images = item.images;
      }
      continue;
    }

    if (
      prev &&
      !item.id.endsWith(':output') &&
      !prev.id.endsWith(':output') &&
      (item.type === 'output' || (item.type as string) === 'summary') &&
      (prev.type === 'output' || (prev.type as string) === 'summary') &&
      item.content?.trim() &&
      prev.content?.trim() &&
      item.content.trim() === prev.content.trim()
    ) {
      if ((item.type as string) === 'summary') {
        prev.type = 'summary';
      }
      if (item.isFinal) {
        prev.isFinal = true;
      }
      continue;
    }

    if (item.type === 'output' && !item.id.endsWith(':output') && item.content?.trim()) {
      const itemContent = item.content.trim();
      const isDuplicateSummary = dedupedItems
        .slice(-3)
        .some(
          (d) =>
            (d.type as string) === 'summary' &&
            d.content?.trim() &&
            (d.content.trim() === itemContent ||
              d.content.includes(itemContent) ||
              itemContent.includes(d.content.trim()))
        );
      if (isDuplicateSummary) {
        continue;
      }
    }
    dedupedItems.push(item);
  }

  return dedupedItems;
}

/**
 * Computes durations and derives `tools`, `thoughts`, and `outputs` collections from deduplicated SQLite activity items.
 */
export function hydrateLoadedActivityState(
  dedupedItems: DrawerActivityItem[],
  registerSubagentMapping: (toolId: string, subagentId: string) => void
): {
  feed: DrawerActivityItem[];
  tools: ToolExecution[];
  thoughts: LiveStreamEntry[];
  outputs: LiveStreamEntry[];
} {
  const computedDurations = computeActivityDurations(dedupedItems, false);
  const tools: ToolExecution[] = [];
  const thoughts: LiveStreamEntry[] = [];
  const outputs: LiveStreamEntry[] = [];

  for (let i = 0; i < dedupedItems.length; i++) {
    const item = dedupedItems[i];
    const computedDur = computedDurations[i];
    if (item.type === 'tool' && item.tool) {
      const rawToolDur = item.tool.durationMs ?? (item.tool as { duration_ms?: number }).duration_ms;
      const rawItemDur = item.durationMs ?? (item as { duration_ms?: number }).duration_ms;
      const dur =
        typeof rawToolDur === 'number' && rawToolDur > 0
          ? rawToolDur
          : typeof rawItemDur === 'number' && rawItemDur > 0
            ? rawItemDur
            : typeof computedDur === 'number' && computedDur > 0
              ? computedDur
              : 0;

      const toolWithDuration: ToolExecution = {
        ...item.tool,
        durationMs: dur,
        createdAt:
          item.tool.createdAt ||
          item.createdAt ||
          (item.timestamp && !isNaN(new Date(item.timestamp).getTime())
            ? new Date(item.timestamp).getTime()
            : undefined),
      };
      item.tool = toolWithDuration;
      item.durationMs = dur > 0 ? dur : undefined;

      tools.unshift(toolWithDuration);
      if (item.tool.toolName === 'subagent' || item.tool.toolName === 'subagent_bg') {
        extractSubagentIdsFromTool(
          item.tool.id,
          item.tool.params,
          item.tool.stdout,
          registerSubagentMapping,
          true
        );
      }
    } else if (item.type === 'thought') {
      const rawItemDur = item.durationMs ?? (item as { duration_ms?: number }).duration_ms;
      const dur =
        typeof rawItemDur === 'number' && rawItemDur > 0
          ? rawItemDur
          : typeof computedDur === 'number' && computedDur > 0
            ? computedDur
            : undefined;
      item.durationMs = dur;
      thoughts.push({
        id: item.id,
        timestamp: item.timestamp,
        type: 'thought',
        content: item.content || '',
        durationMs: dur,
        createdAt: item.createdAt,
      });
    } else {
      outputs.push({
        id: item.id,
        timestamp: item.timestamp,
        type: (item.type === 'steer' ||
        item.type === 'system' ||
        item.type === 'error' ||
        item.type === 'user'
          ? item.type
          : 'output') as LiveStreamEntry['type'],
        content: item.content || '',
      });
    }
  }

  return {
    feed: capActivityFeed(dedupedItems),
    tools: capToolExecutions(tools),
    thoughts,
    outputs,
  };
}

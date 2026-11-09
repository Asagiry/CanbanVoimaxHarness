import { ToolExecution, LiveStreamEntry } from '../../types';
import { DrawerActivityItem, DrawerState } from './types';
import { computeActivityDurations } from './drawerHelpers';
import { findMatchingQuestionIndexForTool } from './activityQuestionHelpers';

export const MAX_ACTIVITY_ITEMS = 500;
export const MAX_TOOL_EXECUTIONS = 500;

/**
 * Rolling window cap for `activityFeed`.
 * Never evicts `feed[0]` when `feed[0]?.type === 'prompt'` (initial task objective card),
 * keeping it plus the most recent `maxItems - 1` items when `feed.length > maxItems`.
 */
export function capActivityFeed(
  feed: DrawerActivityItem[],
  maxItems = MAX_ACTIVITY_ITEMS
): DrawerActivityItem[] {
  if (feed.length <= maxItems) return feed;
  if (feed[0]?.type === 'prompt') {
    return [feed[0], ...feed.slice(-(maxItems - 1))];
  }
  return feed.slice(-maxItems);
}

/**
 * Rolling window cap for `toolExecutions` (stored newest-first).
 * Keeps the latest `maxItems` entries when `tools.length > maxItems`.
 */
export function capToolExecutions(
  tools: ToolExecution[],
  maxItems = MAX_TOOL_EXECUTIONS
): ToolExecution[] {
  if (tools.length <= maxItems) return tools;
  return tools.slice(0, maxItems);
}

/**
 * Extracts `subagent_id` from a tool's params or stdout and invokes `registerSubagentMapping`.
 */
export function extractSubagentIdsFromTool(
  toolId: string,
  params: unknown,
  stdout: string | undefined,
  registerSubagentMapping: (toolId: string, subagentId: string) => void,
  preferParamsOnly = false
): void {
  const pSubId = (params as Record<string, unknown> | undefined)?.subagent_id;
  if (typeof pSubId === 'string' && pSubId) {
    registerSubagentMapping(toolId, pSubId);
    if (preferParamsOnly) return;
  }
  if (stdout) {
    try {
      const parsed = JSON.parse(stdout);
      if (parsed?.subagent_id) registerSubagentMapping(toolId, parsed.subagent_id);
    } catch {
      // ignore non-JSON stdout
    }
    const match = stdout.match(/\[Subagent ID:\s*([^\s\]]+)\]/);
    if (match && match[1]) registerSubagentMapping(toolId, match[1]);
  }
}

/**
 * Merges or appends the final assistant response into `activityFeed` without duplicating streamed output.
 */
export function mergeFinalResponseIntoFeed(
  feed: DrawerActivityItem[],
  response?: string,
  timeStr = new Date().toLocaleTimeString('en-US', { hour12: false })
): DrawerActivityItem[] {
  const updatedFeed = [...feed];

  let lastOutputIdx = -1;
  for (let i = updatedFeed.length - 1; i >= 0; i--) {
    if (updatedFeed[i].type === 'output' || (updatedFeed[i].type as string) === 'summary') {
      lastOutputIdx = i;
      break;
    }
  }

  const hasInterveningTools =
    lastOutputIdx !== -1 &&
    updatedFeed
      .slice(lastOutputIdx + 1)
      .some((it) => it.type === 'tool' || Boolean((it as { tool?: unknown }).tool));

  const resp = (response || '').trim();

  if (lastOutputIdx !== -1 && !hasInterveningTools) {
    const lastOutput = updatedFeed[lastOutputIdx];
    const content = (lastOutput.content || '').trim();
    if (resp && !content) {
      updatedFeed[lastOutputIdx] = {
        ...lastOutput,
        content: response!,
        isFinal: true,
      };
    } else {
      updatedFeed[lastOutputIdx] = {
        ...lastOutput,
        isFinal: true,
      };
    }
  } else if (resp.length > 0) {
    const alreadyExists = updatedFeed.some(
      (it) =>
        (it.type === 'output' || (it.type as string) === 'summary') &&
        (it.content || '').trim() === resp
    );
    if (!alreadyExists) {
      updatedFeed.push({
        id: `out-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
        type: 'output',
        timestamp: timeStr,
        content: response!,
        isFinal: true,
      });
    }
  }

  return capActivityFeed(updatedFeed);
}

/**
 * Merges a new tool execution into `activityFeed` and `toolExecutions`, deduplicating question tools.
 */
export function mergeToolExecutionIntoState(
  existingFeed: DrawerActivityItem[],
  existingTools: ToolExecution[],
  toolWithCreatedAt: ToolExecution
): { updatedFeed: DrawerActivityItem[]; updatedTools: ToolExecution[] } {
  if (toolWithCreatedAt.toolName === 'question' || toolWithCreatedAt.toolName === 'ask_user') {
    const qIndex = findMatchingQuestionIndexForTool(existingFeed, toolWithCreatedAt);
    if (qIndex !== -1) {
      const prev = existingFeed[qIndex];
      const updatedFeed = [...existingFeed];
      updatedFeed[qIndex] = {
        ...prev,
        tool: {
          ...prev.tool,
          ...toolWithCreatedAt,
          id: prev.tool?.id || toolWithCreatedAt.id,
          toolName: 'question',
          params: {
            ...(toolWithCreatedAt.params as Record<string, unknown>),
            ...(prev.tool?.params as Record<string, unknown>),
            callId: toolWithCreatedAt.id,
          },
        },
      };

      const toolExists = existingTools.some((t) => t.id === toolWithCreatedAt.id);
      const updatedTools = toolExists
        ? existingTools.map((t) =>
            t.id === toolWithCreatedAt.id ? { ...t, ...toolWithCreatedAt } : t
          )
        : [toolWithCreatedAt, ...existingTools];

      return {
        updatedFeed: capActivityFeed(updatedFeed),
        updatedTools: capToolExecutions(updatedTools),
      };
    }
  }

  const feedItem: DrawerActivityItem = {
    id: toolWithCreatedAt.id,
    type: 'tool',
    timestamp: toolWithCreatedAt.timestamp,
    tool: toolWithCreatedAt,
    createdAt: toolWithCreatedAt.createdAt,
    durationMs: toolWithCreatedAt.durationMs || undefined,
  };

  const existingMatch = existingFeed.find(
    (item) => item.id === toolWithCreatedAt.id || item.tool?.id === toolWithCreatedAt.id
  );
  const isCollision =
    Boolean(existingMatch) &&
    existingMatch!.tool?.id !== undefined &&
    existingMatch!.tool!.id !== toolWithCreatedAt.id;
  const updatedFeed = isCollision
    ? [
        ...existingFeed,
        {
          ...feedItem,
          id: toolWithCreatedAt.batch
            ? `${toolWithCreatedAt.id}::batch-${toolWithCreatedAt.batch.index}`
            : `${toolWithCreatedAt.id}::${existingFeed.length}`,
        },
      ]
    : existingMatch
      ? existingFeed.map((item) =>
          item.id === toolWithCreatedAt.id || item.tool?.id === toolWithCreatedAt.id
            ? { ...item, tool: { ...item.tool, ...toolWithCreatedAt } }
            : item
        )
      : [...existingFeed, feedItem];

  const toolExists = existingTools.some((t) => t.id === toolWithCreatedAt.id);
  const updatedTools = toolExists
    ? existingTools.map((t) =>
        t.id === toolWithCreatedAt.id ? { ...t, ...toolWithCreatedAt } : t
      )
    : [toolWithCreatedAt, ...existingTools];

  return {
    updatedFeed: capActivityFeed(updatedFeed),
    updatedTools: capToolExecutions(updatedTools),
  };
}

/**
 * Updates an existing tool execution in both `toolExecutions` and `activityFeed`.
 */
export function applyToolExecutionUpdate(
  existingFeed: DrawerActivityItem[],
  existingTools: ToolExecution[],
  toolId: string,
  partial: Partial<ToolExecution>
): { updatedFeed: DrawerActivityItem[]; updatedTools: ToolExecution[] } {
  const existingTool = existingTools.find((t) => t.id === toolId);

  let durationMs =
    typeof partial.durationMs === 'number' && Number.isFinite(partial.durationMs)
      ? partial.durationMs
      : undefined;

  if (durationMs === undefined && existingTool?.createdAt) {
    durationMs = Math.max(0, Date.now() - existingTool.createdAt);
  }

  const mergedPartial: Partial<ToolExecution> = {
    ...partial,
    ...(typeof durationMs === 'number' && Number.isFinite(durationMs) ? { durationMs } : {}),
  };

  const updatedTools = existingTools.map((t) =>
    t.id === toolId ? { ...t, ...mergedPartial } : t
  );

  const updatedFeed = existingFeed.map((item) => {
    if (
      (item.type === 'tool' || item.type === 'question') &&
      (item.id === toolId ||
        item.tool?.id === toolId ||
        (item.tool?.params as Record<string, unknown> | undefined)?.callId === toolId ||
        (item.tool?.params as Record<string, unknown> | undefined)?.questionId === toolId)
    ) {
      return {
        ...item,
        ...(typeof durationMs === 'number' && Number.isFinite(durationMs) ? { durationMs } : {}),
        tool: item.tool ? { ...item.tool, ...mergedPartial } : undefined,
      };
    }
    return item;
  });

  return {
    updatedFeed: capActivityFeed(updatedFeed),
    updatedTools: capToolExecutions(updatedTools),
  };
}

/**
 * Appends or merges an activity item into `activityFeed` with duplicate suppression and rolling window cap.
 */
export function mergeActivityItemIntoFeed(
  feed: DrawerActivityItem[],
  item: DrawerActivityItem,
  isStreamedItem = false
): DrawerActivityItem[] {
  const idx = feed.findIndex((f) => f.id === item.id);
  let updatedFeed: DrawerActivityItem[];
  if (idx !== -1) {
    updatedFeed = [...feed];
    updatedFeed[idx] = { ...feed[idx], ...item };
  } else {
    const isHandoverSteer =
      item.type === 'steer' && item.content?.includes('[ОТЧЕТ ОБ ИСПРАВЛЕНИЯХ / HANDOVER SUMMARY]');
    let recentUserSteerIdx = -1;
    if ((item.type === 'user' || item.type === 'steer') && item.content?.trim()) {
      const targetText = item.content.trim();
      const start = Math.max(0, feed.length - 2);
      for (let i = feed.length - 1; i >= start; i--) {
        const candidate = feed[i];
        if (
          (candidate.type === 'user' || candidate.type === 'steer') &&
          candidate.content?.trim() === targetText &&
          candidate.id.startsWith('out-') !== item.id.startsWith('out-')
        ) {
          const sameTimestamp = Boolean(
            candidate.timestamp && item.timestamp && candidate.timestamp === item.timestamp
          );
          const timeDelta =
            candidate.createdAt && item.createdAt
              ? Math.abs(item.createdAt - candidate.createdAt)
              : candidate.createdAt
                ? Math.abs(Date.now() - candidate.createdAt)
                : 0;
          if (sameTimestamp || timeDelta <= 5000) {
            recentUserSteerIdx = i;
            break;
          }
        }
      }
    }
    const duplicateIdx =
      recentUserSteerIdx !== -1
        ? recentUserSteerIdx
        : isStreamedItem || item.type === 'user' || (item.type === 'steer' && !isHandoverSteer)
          ? -1
          : feed.findIndex(
              (f) =>
                !/:(thought|output)$/.test(f.id) &&
                f.type === item.type &&
                f.content &&
                item.content &&
                f.content.trim() === item.content.trim()
            );
    if (duplicateIdx !== -1) {
      updatedFeed = [...feed];
      updatedFeed[duplicateIdx] = { ...feed[duplicateIdx], ...item };
    } else {
      updatedFeed = [...feed, item];
    }
  }
  return capActivityFeed(updatedFeed);
}

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

/**
 * Evaporates verbose stdout/stderr from older client tool executions while keeping the 2 most recent intact.
 */
export function evaporateToolExecutionLogs(tools: ToolExecution[]): {
  updatedTools: ToolExecution[];
  evaporatedCount: number;
  savedTokens: number;
} {
  let evaporatedCount = 0;
  let savedChars = 0;

  const updatedTools = tools.map((tool, idx) => {
    if (idx < 2) return tool;

    let newStdout = tool.stdout;
    let newStderr = tool.stderr;

    if (tool.stdout && tool.stdout.length > 500) {
      const origLen = tool.stdout.length;
      const placeholder = `[Output evaporated: ${origLen} chars truncated. Exit code: ${tool.exitCode ?? 0}. Tool: ${tool.toolName}]`;
      savedChars += Math.max(0, origLen - placeholder.length);
      newStdout = placeholder;
      evaporatedCount += 1;
    }

    if (tool.stderr && tool.stderr.length > 500) {
      const origLen = tool.stderr.length;
      const placeholder = `[Output evaporated: ${origLen} chars truncated. Exit code: ${tool.exitCode ?? 1}. Tool: ${tool.toolName}]`;
      savedChars += Math.max(0, origLen - placeholder.length);
      newStderr = placeholder;
      evaporatedCount += 1;
    }

    return {
      ...tool,
      stdout: newStdout,
      stderr: newStderr,
    };
  });

  const savedTokens = Math.max(0, Math.round(savedChars / 4));
  return { updatedTools, evaporatedCount, savedTokens };
}

/**
 * Builds the cleared state slice for a given taskId when `clearTaskActivity` is invoked.
 */
export function buildClearedTaskActivityState(
  state: DrawerState,
  taskId: string
): Partial<DrawerState> {
  return {
    connectionWait: { ...state.connectionWait, [taskId]: null },
    activeStreamIds: { ...state.activeStreamIds, [taskId]: null },
    activityFeed: { ...state.activityFeed, [taskId]: [] },
    streamEntries: { ...state.streamEntries, [taskId]: { thoughts: [], outputs: [] } },
    toolExecutions: { ...state.toolExecutions, [taskId]: [] },
    finalTps: { ...state.finalTps, [taskId]: null },
    generationStartTime: { ...state.generationStartTime, [taskId]: null },
    generationEndTime: { ...state.generationEndTime, [taskId]: null },
    streamedTurnTokens: { ...state.streamedTurnTokens, [taskId]: 0 },
    taskStartTime: { ...state.taskStartTime, [taskId]: null },
    taskEndTime: { ...state.taskEndTime, [taskId]: null },
    cumulativeOutputTokens: { ...state.cumulativeOutputTokens, [taskId]: 0 },
    activeAgentTaskIds: { ...state.activeAgentTaskIds, [taskId]: false },
    cumulativeGenerationSec: { ...state.cumulativeGenerationSec, [taskId]: 0 },
    subagents: { ...state.subagents, [taskId]: [] },
  };
}

import { ToolExecution } from '../../../types';
import { DrawerActivityItem } from '../types';
import { findMatchingQuestionIndexForTool } from '../activityQuestionHelpers';
import { capActivityFeed, capToolExecutions } from './windowing';

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

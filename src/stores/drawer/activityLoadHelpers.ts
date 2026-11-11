import { DrawerActivityItem, DrawerState } from './types';
import { useBoardStore } from '../boardStore';
import { deduplicateLoadedActivityItems, hydrateLoadedActivityState } from './activityDedup';

export const MAX_LOAD_ROLLBACK_RETRIES = 3;

export function buildLoadedTaskActivityPatch(
  state: DrawerState,
  taskId: string,
  items: DrawerActivityItem[],
  registerSubagentMapping: (toolId: string, subagentId: string) => void,
): Partial<DrawerState> {
  let compactedTokens: number | undefined;
  for (let i = items.length - 1; i >= 0; i--) {
    const tData = items[i].type === 'compacted' ? (items[i].tool as { tokensAfter?: number }) : undefined;
    if (typeof tData?.tokensAfter === 'number') {
      compactedTokens = tData.tokensAfter;
      break;
    }
  }

  try {
    const boardTask = useBoardStore.getState().tasks.find((t) => t.id === taskId);
    if (compactedTokens === undefined && typeof boardTask?.currentTokens === 'number') {
      compactedTokens = boardTask.currentTokens;
    }
  } catch (err) {
    console.debug('buildLoadedTaskActivityPatch: boardStore unavailable:', err);
  }

  if (items.length > 0) {
    const currentFeed = state.activityFeed[taskId] || [];
    const hasCompacted = items.some((it) => it.type === 'compacted');
    const newOverrides =
      compactedTokens !== undefined
        ? { ...state.tokenOverrides, [taskId]: compactedTokens }
        : state.tokenOverrides;
    if (
      currentFeed.length === 0 ||
      items.length >= currentFeed.length ||
      hasCompacted ||
      items[items.length - 1]?.id !== currentFeed[currentFeed.length - 1]?.id
    ) {
      const dedupedItems = deduplicateLoadedActivityItems(items);
      const { feed, tools, thoughts, outputs } = hydrateLoadedActivityState(dedupedItems, registerSubagentMapping);
      const loadedIds = new Set(feed.map((it) => it.id));
      const liveOnly = currentFeed.filter((it) => !loadedIds.has(it.id));
      const mergedFeed = liveOnly.length > 0 ? [...feed, ...liveOnly] : feed;
      const mergedThoughts = [...thoughts];
      const mergedOutputs = [...outputs];
      for (const it of liveOnly) {
        if (it.type === 'thought') mergedThoughts.push(it as never);
        else if (it.type === 'output') mergedOutputs.push(it as never);
      }
      const liveToolIds = new Set(tools.map((t) => t.id));
      const liveTools = (state.toolExecutions[taskId] || []).filter((t) => !liveToolIds.has(t.id));
      return {
        activityFeed: { ...state.activityFeed, [taskId]: mergedFeed },
        toolExecutions: { ...state.toolExecutions, [taskId]: [...tools, ...liveTools] },
        streamEntries: { ...state.streamEntries, [taskId]: { thoughts: mergedThoughts, outputs: mergedOutputs } },
        tokenOverrides: newOverrides,
      };
    }
    return { tokenOverrides: newOverrides };
  }

  // Only keep live in-memory items when an agent is actively running for this task;
  // otherwise an empty backend result is authoritative and clears stale items.
  if (state.activeAgentTaskIds[taskId]) {
    return {};
  }

  return {
    activityFeed: { ...state.activityFeed, [taskId]: [] },
    streamEntries: { ...state.streamEntries, [taskId]: { thoughts: [], outputs: [] } },
    toolExecutions: { ...state.toolExecutions, [taskId]: [] },
    tokenOverrides: { ...state.tokenOverrides, [taskId]: undefined },
    cumulativeOutputTokens: { ...state.cumulativeOutputTokens, [taskId]: 0 },
    cumulativeGenerationSec: { ...state.cumulativeGenerationSec, [taskId]: 0 },
  };
}

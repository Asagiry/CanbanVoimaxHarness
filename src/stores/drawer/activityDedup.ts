export {
  MAX_ACTIVITY_ITEMS,
  MAX_TOOL_EXECUTIONS,
  capActivityFeed,
  capToolExecutions,
  evaporateToolExecutionLogs,
  buildClearedTaskActivityState,
} from './dedup/windowing';

export { mergeFinalResponseIntoFeed, mergeActivityItemIntoFeed } from './dedup/feedMerge';

export {
  extractSubagentIdsFromTool,
  mergeToolExecutionIntoState,
  applyToolExecutionUpdate,
} from './dedup/toolMerge';

export { deduplicateLoadedActivityItems, hydrateLoadedActivityState } from './dedup/hydration';

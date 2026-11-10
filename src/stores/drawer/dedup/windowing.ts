import { ToolExecution } from '../../../types';
import { DrawerActivityItem, DrawerState } from '../types';

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

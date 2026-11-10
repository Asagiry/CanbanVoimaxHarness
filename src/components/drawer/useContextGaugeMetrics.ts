import { useEffect, useMemo, useRef } from 'react';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import { UseContextGaugeMetricsOptions } from './metrics/types';
import {
  computeGaugeGeometry,
  computeIncrementalTextChars,
  computeTokenBreakdown,
  createTextCharsCache,
  parseCompactedFromFeed,
} from './metrics/tokenMetrics';
import {
  computeSpeedAndTiming,
  computeTokenDisplaySummary,
  computeToolTimeSec,
  computeTurnsAndSteps,
} from './metrics/telemetryMetrics';
import {
  useCompactionActions,
  usePersonaContextLimit,
  useTelemetrySummarySync,
} from './metrics/useGaugeEffects';

export type { UseContextGaugeMetricsOptions };

export function useContextGaugeMetrics({
  taskId,
  persona,
  turns,
  contextWindow,
  stageLimit = 128000,
  currentEstimatedTokens,
  onCompact,
}: UseContextGaugeMetricsOptions) {
  const resolvedPersonaLimit = usePersonaContextLimit(taskId, persona, contextWindow);
  const effectiveLimit = contextWindow || resolvedPersonaLimit || stageLimit;

  const thoughts = useDrawerStore((s) => s.streamEntries[taskId]?.thoughts || (s.toolIdToSubagentId[taskId] ? s.streamEntries[s.toolIdToSubagentId[taskId]]?.thoughts : undefined));
  const outputs = useDrawerStore((s) => s.streamEntries[taskId]?.outputs || (s.toolIdToSubagentId[taskId] ? s.streamEntries[s.toolIdToSubagentId[taskId]]?.outputs : undefined));
  const tools = useDrawerStore((s) => s.toolExecutions[taskId] || (s.toolIdToSubagentId[taskId] ? s.toolExecutions[s.toolIdToSubagentId[taskId]] : undefined));
  const feed = useDrawerStore((s) => s.activityFeed[taskId] || (s.toolIdToSubagentId[taskId] ? s.activityFeed[s.toolIdToSubagentId[taskId]] : undefined));
  const tokenOverride = useDrawerStore((s) => s.tokenOverrides[taskId] ?? (s.toolIdToSubagentId[taskId] ? s.tokenOverrides[s.toolIdToSubagentId[taskId]] : undefined));
  const taskCurrentTokens = useBoardStore((s) => s.tasks.find((t) => t.id === taskId)?.currentTokens);
  const telemetry = useDrawerStore((s) => s.taskTelemetry[taskId] || (s.toolIdToSubagentId[taskId] ? s.taskTelemetry[s.toolIdToSubagentId[taskId]] : undefined) || (s.subagentIdToToolId[taskId] ? s.taskTelemetry[s.subagentIdToToolId[taskId]] : undefined));
  const telemetrySummary = useDrawerStore((s) => s.taskTelemetrySummary[taskId] || (s.toolIdToSubagentId[taskId] ? s.taskTelemetrySummary[s.toolIdToSubagentId[taskId]] : undefined) || (s.subagentIdToToolId[taskId] ? s.taskTelemetrySummary[s.subagentIdToToolId[taskId]] : undefined));
  const setTaskTelemetrySummary = useDrawerStore((s) => s.setTaskTelemetrySummary);
  const thinkingStart = useDrawerStore((s) => s.thinkingStartTime[taskId] ?? null);
  const genStart = useDrawerStore((s) => s.generationStartTime[taskId] ?? null);
  const genEnd = useDrawerStore((s) => s.generationEndTime[taskId] ?? null);
  const streamedTurnTokens = useDrawerStore((s) => s.streamedTurnTokens[taskId] ?? (s.toolIdToSubagentId[taskId] ? s.streamedTurnTokens[s.toolIdToSubagentId[taskId]] : undefined) ?? (s.subagentIdToToolId[taskId] ? s.streamedTurnTokens[s.subagentIdToToolId[taskId]] : undefined) ?? 0);
  const frozenTps = useDrawerStore((s) => s.finalTps[taskId] ?? (s.toolIdToSubagentId[taskId] ? s.finalTps[s.toolIdToSubagentId[taskId]] : undefined) ?? (s.subagentIdToToolId[taskId] ? s.finalTps[s.subagentIdToToolId[taskId]] : undefined) ?? null);
  const cumulativeOutputTokens = useDrawerStore((s) => s.cumulativeOutputTokens[taskId] ?? 0);
  const cumulativeGenerationSec = useDrawerStore((s) => s.cumulativeGenerationSec[taskId] ?? 0);
  const setTaskTokens = useDrawerStore((s) => s.setTaskTokens);
  const evaporateClientToolLogs = useDrawerStore((s) => s.evaporateClientToolLogs);
  const isTaskRunning = useBoardStore((s) => s.tasks.find((t) => t.id === taskId)?.status === 'running');
  // NOTE: both hooks must run unconditionally — `||` short-circuit would violate Rules of Hooks
  const isDrawerAgentRunning = useDrawerStore((s) => Boolean(s.activeAgentTaskIds[taskId] || (s.toolIdToSubagentId[taskId] && s.activeAgentTaskIds[s.toolIdToSubagentId[taskId]])));
  const isAgentRunning = isTaskRunning || isDrawerAgentRunning;

  const { latestCompactedTokens, postCompactionTokens } = useMemo(() => parseCompactedFromFeed(feed), [feed]);
  const baseCompacted = tokenOverride ?? taskCurrentTokens ?? latestCompactedTokens;
  const resolvedCompactTokens = baseCompacted !== undefined ? baseCompacted + postCompactionTokens : undefined;

  useEffect(() => {
    if (resolvedCompactTokens !== undefined && tokenOverride === undefined) {
      setTaskTokens?.(taskId, resolvedCompactTokens);
    }
  }, [taskId, resolvedCompactTokens, tokenOverride, setTaskTokens]);

  const textCache = useRef(createTextCharsCache());

  const { totalUsedTokens, systemPromptTokens, toolDefTokens, messageTokens, isEstimate } = useMemo(() => {
    const thoughtsArr = thoughts ?? [];
    const outputsArr = outputs ?? [];
    const toolsArr = tools ?? [];
    const textChars = computeIncrementalTextChars(thoughtsArr, outputsArr, textCache.current);
    return computeTokenBreakdown({
      taskId,
      persona,
      currentEstimatedTokens,
      resolvedCompactTokens,
      tokenOverride,
      telemetry,
      streamedTurnTokens,
      textChars,
      thoughtsLen: thoughtsArr.length,
      outputsLen: outputsArr.length,
      tools: toolsArr,
      feedLen: feed?.length ?? 0,
    });
  }, [
    taskId, persona, currentEstimatedTokens, resolvedCompactTokens, tokenOverride,
    telemetry, streamedTurnTokens, thoughts, outputs, tools, feed,
  ]);

  const {
    percent,
    systemPct,
    toolPct,
    msgPct,
    usedTokensK,
    maxTokensK,
    radius,
    circumference,
    strokeDashoffset,
    miniCircumference,
    miniStrokeDashoffset,
    strokeColor,
  } = computeGaugeGeometry(
    { totalUsedTokens, systemPromptTokens, toolDefTokens, messageTokens },
    effectiveLimit
  );

  const { effectiveTurns, stepsCount } = useMemo(
    () => computeTurnsAndSteps({ taskId, turns, tools, thoughts, outputs }),
    [taskId, turns, tools, thoughts, outputs]
  );

  const { tpsValue, llmTimeSec, ttftMs, safeCacheHitRate } = computeSpeedAndTiming({
    telemetry,
    thinkingStart,
    genStart,
    genEnd,
    streamedTurnTokens,
    frozenTps,
    cumulativeOutputTokens,
    cumulativeGenerationSec,
  });

  const toolTimeSec = useMemo(
    () => computeToolTimeSec(tools, feed, isAgentRunning),
    [tools, feed, isAgentRunning]
  );

  const displaySummary = computeTokenDisplaySummary({
    telemetry,
    telemetrySummary,
    totalUsedTokens,
    streamedTurnTokens,
    cumulativeOutputTokens,
    safeCacheHitRate,
  });

  useTelemetrySummarySync(taskId, setTaskTelemetrySummary);

  const { isCompacting, savingsBadge, handleCompactClick } = useCompactionActions({
    taskId,
    onCompact,
    setTaskTokens,
    evaporateClientToolLogs,
  });

  return {
    effectiveLimit,
    percent,
    usedTokensK,
    maxTokensK,
    systemPct,
    toolPct,
    msgPct,
    systemPromptTokens,
    toolDefTokens,
    messageTokens,
    isEstimate,
    effectiveTurns,
    stepsCount,
    llmTimeSec,
    toolTimeSec,
    ttftMs,
    tpsValue,
    ...displaySummary,
    isCompacting,
    savingsBadge,
    handleCompactClick,
    strokeColor,
    radius,
    circumference,
    strokeDashoffset,
    miniCircumference,
    miniStrokeDashoffset,
  };
}

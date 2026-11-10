import { DrawerActivityItem, TaskTelemetry } from '../../../stores/drawerStore';
import { TaskTelemetrySummary } from '../../../stores/drawer/types';
import { computeActivityDurations } from '../../../stores/drawer/drawerHelpers';
import { LiveStreamEntry, ToolExecution } from '../../../types';
import { TokenDisplaySummary } from './types';

export function computeTurnsAndSteps(params: {
  taskId: string;
  turns?: number;
  tools?: ToolExecution[];
  thoughts?: LiveStreamEntry[];
  outputs?: LiveStreamEntry[];
}): { effectiveTurns: number; stepsCount: number } {
  const { taskId, turns, tools, thoughts, outputs } = params;
  const calculatedTurns = Math.max(
    (tools?.length || 0) + (outputs?.length || 0),
    thoughts?.length || 0,
    0
  );
  const isSubagentTask =
    taskId.includes('.sub-') || taskId.includes(':sub:') || taskId.includes('.');
  const effectiveTurns = turns !== undefined && !isSubagentTask ? turns : calculatedTurns;
  const stepsCount = tools?.length || 0;
  return { effectiveTurns, stepsCount };
}

export function computeToolTimeSec(
  tools: ToolExecution[] | undefined,
  feed: DrawerActivityItem[] | undefined,
  isAgentRunning: boolean
): number {
  let sumMs = 0;
  const toolList = tools ?? [];

  for (const t of toolList) {
    if (!t) continue;
    if (t.status === 'running' && t.createdAt) {
      sumMs += Math.max(0, Date.now() - t.createdAt);
    } else if (typeof t.durationMs === 'number' && t.durationMs > 0) {
      sumMs += t.durationMs;
    }
  }

  // Fallback: If tools sum is 0 (e.g. legacy tasks or before store update), calculate from activity feed
  if (sumMs === 0 && feed && feed.length > 0) {
    const computed = computeActivityDurations(feed, isAgentRunning);
    for (let i = 0; i < feed.length; i++) {
      const item = feed[i];
      if (item && (item.type === 'tool' || item.tool)) {
        const rawToolDur =
          item.tool?.durationMs ?? (item.tool as unknown as { duration_ms?: unknown } | undefined)?.duration_ms;
        const rawItemDur =
          item.durationMs ?? (item as unknown as { duration_ms?: unknown }).duration_ms;
        const dur =
          typeof rawToolDur === 'number' && rawToolDur > 0
            ? rawToolDur
            : typeof rawItemDur === 'number' && rawItemDur > 0
              ? rawItemDur
              : computed[i] || 0;
        sumMs += dur;
      }
    }
  }

  return sumMs / 1000;
}

export function computeSpeedAndTiming(params: {
  telemetry?: TaskTelemetry;
  thinkingStart: number | null;
  genStart: number | null;
  genEnd: number | null;
  streamedTurnTokens: number;
  frozenTps: number | null;
  cumulativeOutputTokens: number;
  cumulativeGenerationSec: number;
}): {
  tpsValue: number | null;
  llmTimeSec: number;
  ttftMs: number | null;
  safeCacheHitRate: number;
} {
  const {
    telemetry,
    thinkingStart,
    genStart,
    genEnd,
    streamedTurnTokens,
    frozenTps,
    cumulativeOutputTokens,
    cumulativeGenerationSec,
  } = params;

  const totalInputTokens = telemetry?.inputTokens || 0;
  const safeCacheHitRate = telemetry
    ? typeof telemetry.cacheHitRate === 'number' &&
      Number.isFinite(telemetry.cacheHitRate) &&
      telemetry.cacheHitRate >= 0
      ? telemetry.cacheHitRate
      : totalInputTokens > 0
        ? ((telemetry.cacheReadTokens || 0) / totalInputTokens) * 100
        : 0
    : 0;

  const activeGenStart = genStart ?? thinkingStart;
  const elapsedSec =
    activeGenStart != null
      ? Math.max(0.01, ((genEnd ?? Date.now()) - activeGenStart) / 1000)
      : null;
  const turnOutputTokens =
    telemetry?.outputTokens && telemetry.outputTokens > 0
      ? telemetry.outputTokens
      : streamedTurnTokens;
  const liveTps =
    elapsedSec != null && elapsedSec >= 0.1 && turnOutputTokens > 0
      ? turnOutputTokens / elapsedSec
      : null;
  const lastTurnTps = frozenTps != null ? frozenTps : liveTps;

  const avgTps =
    cumulativeGenerationSec > 0 && cumulativeOutputTokens > 0
      ? cumulativeOutputTokens / cumulativeGenerationSec
      : lastTurnTps;

  const tpsValue = avgTps != null ? avgTps : lastTurnTps;
  const llmTimeSec = cumulativeGenerationSec + (elapsedSec ?? 0);
  const ttftMs = telemetry?.ttftMs ?? null;

  return { tpsValue, llmTimeSec, ttftMs, safeCacheHitRate };
}

export function computeTokenDisplaySummary(params: {
  telemetry?: TaskTelemetry;
  telemetrySummary?: TaskTelemetrySummary;
  totalUsedTokens: number;
  streamedTurnTokens: number;
  cumulativeOutputTokens: number;
  safeCacheHitRate: number;
}): TokenDisplaySummary {
  const {
    telemetry,
    telemetrySummary,
    totalUsedTokens,
    streamedTurnTokens,
    cumulativeOutputTokens,
    safeCacheHitRate,
  } = params;

  const inputTokens = telemetry?.inputTokens ?? totalUsedTokens;
  const cachedInput = Math.min(telemetry?.cacheReadTokens || 0, inputTokens);
  const uncachedInput = Math.max(0, inputTokens - cachedInput);
  const outputTokens =
    telemetry?.outputTokens && telemetry.outputTokens > 0
      ? telemetry.outputTokens
      : streamedTurnTokens > 0
        ? streamedTurnTokens
        : cumulativeOutputTokens || 0;
  const totalTokens = uncachedInput + cachedInput + outputTokens;

  const cumulativeInputTokens = telemetrySummary?.totalInputTokens ?? 0;
  const cumulativeCachedTokens = Math.min(
    telemetrySummary?.totalCachedTokens ?? 0,
    cumulativeInputTokens
  );
  const liveStreamOutput = streamedTurnTokens > 0 ? streamedTurnTokens : 0;
  const cumulativeOutputTokensTotal = (telemetrySummary?.totalOutputTokens ?? 0) + liveStreamOutput;
  const cumulativeTotalTokens =
    (telemetrySummary?.totalTokens ?? cumulativeInputTokens + (telemetrySummary?.totalOutputTokens ?? 0)) +
    liveStreamOutput;
  const cumulativeUncachedInput = Math.max(0, cumulativeInputTokens - cumulativeCachedTokens);
  const cumulativeCacheHitRate =
    telemetrySummary?.cacheHitRate ??
    (cumulativeInputTokens > 0 ? (cumulativeCachedTokens / cumulativeInputTokens) * 100 : 0);
  const cumulativeTurnCount = telemetrySummary?.turnCount ?? 0;
  const hasCumulative = (telemetrySummary?.totalTokens ?? 0) > 0 || cumulativeInputTokens > 0;

  return {
    cacheHitRate: hasCumulative ? cumulativeCacheHitRate : safeCacheHitRate,
    uncachedInput: hasCumulative ? cumulativeUncachedInput : uncachedInput,
    cachedInput: hasCumulative ? cumulativeCachedTokens : cachedInput,
    outputTokens: hasCumulative ? cumulativeOutputTokensTotal : outputTokens,
    totalTokens: hasCumulative ? cumulativeTotalTokens : totalTokens,
    cumulativeInputTokens,
    cumulativeCachedTokens,
    cumulativeOutputTokens: cumulativeOutputTokensTotal,
    cumulativeTotalTokens,
    cumulativeUncachedInput,
    cumulativeCacheHitRate,
    cumulativeTurnCount,
    hasCumulative,
  };
}

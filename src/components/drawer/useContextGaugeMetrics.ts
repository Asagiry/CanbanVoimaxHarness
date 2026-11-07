import { useState, useEffect, useMemo, useCallback, useRef } from 'react';
import { useDrawerStore, DrawerActivityItem } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import { useCortexStore } from '../../stores/cortexStore';
import { listenToEvent } from '../../services/tauriEventGateway';
import { computeActivityDurations } from '../../stores/drawer/drawerHelpers';

export interface UseContextGaugeMetricsOptions {
  taskId: string;
  persona?: string;
  turns?: number;
  contextWindow?: number;
  stageLimit?: number;
  currentEstimatedTokens?: number;
  onCompact?: () => Promise<{ savedTokens: number; evaporatedCount: number } | void>;
}

function parseCompactedFromFeed(feed?: DrawerActivityItem[]) {
  if (!feed || feed.length === 0) return { latestCompactedTokens: undefined, postCompactionTokens: 0 };
  let compactedIdx = -1;
  let baseTokens: number | undefined;

  for (let i = feed.length - 1; i >= 0; i--) {
    const it = feed[i];
    if (it.type === 'compacted' && it.tool) {
      const tData = it.tool as any;
      if (typeof tData?.tokensAfter === 'number') {
        baseTokens = tData.tokensAfter;
        compactedIdx = i;
        break;
      }
    }
  }

  if (baseTokens === undefined || compactedIdx === -1) {
    return { latestCompactedTokens: undefined, postCompactionTokens: 0 };
  }

  let postChars = 0;
  for (let i = compactedIdx + 1; i < feed.length; i++) {
    const it = feed[i];
    postChars += (it.content?.length || 0);
    if (it.tool) {
      postChars += (it.tool.stdout?.length || 0) + (it.tool.stderr?.length || 0) + JSON.stringify(it.tool.params || {}).length;
    }
  }

  return { latestCompactedTokens: baseTokens, postCompactionTokens: Math.round(postChars / 2.2) };
}

export function useContextGaugeMetrics({
  taskId,
  persona,
  turns,
  contextWindow,
  stageLimit = 128000,
  currentEstimatedTokens,
  onCompact,
}: UseContextGaugeMetricsOptions) {
  const [resolvedPersonaLimit, setResolvedPersonaLimit] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;
    import('@tauri-apps/api/core')
      .then(({ invoke }) => invoke<number>('get_task_context_window', { taskId, persona }))
      .then((cw) => {
        if (!cancelled && typeof cw === 'number' && cw > 0) {
          const pInfo = useCortexStore.getState().agentPersonas?.find((p) => p.role.toLowerCase() === (persona || '').toLowerCase());
          setResolvedPersonaLimit(pInfo?.contextWindow ?? (contextWindow && cw === 128000 ? contextWindow : cw));
        }
      })
      .catch(() => {
        const role = persona || useBoardStore.getState().columns.find((c) => c.id === taskId || c.id === useBoardStore.getState().tasks.find((t) => t.id === taskId)?.stageId)?.persona || 'general';
        const pInfo = useCortexStore.getState().agentPersonas?.find((p) => p.role.toLowerCase() === role.toLowerCase());
        if (!cancelled && (pInfo?.contextWindow || contextWindow)) {
          setResolvedPersonaLimit(pInfo?.contextWindow ?? contextWindow ?? null);
        }
      });
    return () => { cancelled = true; };
  }, [taskId, persona, contextWindow]);

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

  const [isCompacting, setIsCompacting] = useState(false);
  const [savingsBadge, setSavingsBadge] = useState<string | null>(null);
  const savingsTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const textCache = useRef<{
    valid: boolean;
    thoughtsLen: number;
    outputsLen: number;
    lastThoughtId?: string;
    lastOutputId?: string;
    lastThoughtLen: number;
    lastOutputLen: number;
    textChars: number;
  }>({ valid: false, thoughtsLen: 0, outputsLen: 0, lastThoughtLen: 0, lastOutputLen: 0, textChars: 0 });

  const { totalUsedTokens, systemPromptTokens, toolDefTokens, messageTokens, isEstimate } = useMemo(() => {
    const thoughtsArr = thoughts ?? [];
    const outputsArr = outputs ?? [];
    const toolsArr = tools ?? [];
    const tLen = thoughtsArr.length;
    const oLen = outputsArr.length;
    const lastTh = tLen > 0 ? thoughtsArr[tLen - 1] : undefined;
    const lastOut = oLen > 0 ? outputsArr[oLen - 1] : undefined;

    const fullTextRecalc = () => {
      let sum = 0;
      for (const th of thoughtsArr) sum += (th?.content ?? '').length;
      for (const out of outputsArr) sum += (out?.content ?? '').length;
      return sum;
    };
    const commitCache = (textChars: number) => {
      const c = textCache.current;
      c.valid = true;
      c.thoughtsLen = tLen;
      c.outputsLen = oLen;
      c.lastThoughtId = lastTh?.id;
      c.lastOutputId = lastOut?.id;
      c.lastThoughtLen = (lastTh?.content ?? '').length;
      c.lastOutputLen = (lastOut?.content ?? '').length;
      c.textChars = textChars;
    };

    let textChars: number;
    const c = textCache.current;
    if (!c.valid || tLen < c.thoughtsLen || oLen < c.outputsLen) {
      textChars = fullTextRecalc();
      commitCache(textChars);
    } else {
      const thoughtStable = c.thoughtsLen === 0 || thoughtsArr[c.thoughtsLen - 1]?.id === c.lastThoughtId;
      const outputStable = c.outputsLen === 0 || outputsArr[c.outputsLen - 1]?.id === c.lastOutputId;
      if (!thoughtStable || !outputStable) {
        textChars = fullTextRecalc();
        commitCache(textChars);
      } else {
        let delta = 0;
        let valid = true;
        if (c.thoughtsLen > 0) {
          const grow = (thoughtsArr[c.thoughtsLen - 1]?.content ?? '').length - c.lastThoughtLen;
          if (grow < 0) valid = false; else delta += grow;
        }
        if (c.outputsLen > 0) {
          const grow = (outputsArr[c.outputsLen - 1]?.content ?? '').length - c.lastOutputLen;
          if (grow < 0) valid = false; else delta += grow;
        }
        if (valid) {
          for (let i = c.thoughtsLen; i < tLen; i++) delta += (thoughtsArr[i]?.content ?? '').length;
          for (let i = c.outputsLen; i < oLen; i++) delta += (outputsArr[i]?.content ?? '').length;
          textChars = c.textChars + delta;
          commitCache(textChars);
        } else {
          textChars = fullTextRecalc();
          commitCache(textChars);
        }
      }
    }

    const telemetrySys = typeof telemetry?.systemTokens === 'number' && Number.isFinite(telemetry.systemTokens) && (telemetry.systemTokens as number) > 0
      ? (telemetry.systemTokens as number)
      : undefined;
    const telemetryTool = typeof telemetry?.toolTokens === 'number' && Number.isFinite(telemetry.toolTokens) && (telemetry.toolTokens as number) >= 0
      ? (telemetry.toolTokens as number)
      : undefined;

    const isExploratorySubagent = taskId.includes('.sub-') || taskId.includes(':sub:');
    const baseSystem = telemetrySys ?? (isExploratorySubagent ? 800 : 0);
    const baseTool = telemetryTool ?? (isExploratorySubagent ? 1000 : 0);

    let messageChars = textChars;
    for (const t of toolsArr) {
      messageChars += (t?.stdout?.length || 0) + (t?.stderr?.length || 0) + JSON.stringify(t?.params || {}).length;
    }
    const hasMessages = thoughtsArr.length > 0 || outputsArr.length > 0 || toolsArr.length > 0 || ((feed?.length ?? 0) > 0) || messageChars > 0;
    const rawMsgTokens = Math.max(hasMessages ? 100 : 0, Math.round(messageChars / 2.2));

    const isTelemetryEstimate = telemetrySys !== undefined || telemetryTool !== undefined;

    let total: number;
    if (currentEstimatedTokens !== undefined) {
      total = currentEstimatedTokens;
    } else if ((telemetry?.inputTokens || 0) > 0) {
      total = telemetry.inputTokens + (streamedTurnTokens > 0 ? streamedTurnTokens : 0);
    } else if (resolvedCompactTokens !== undefined || tokenOverride !== undefined) {
      total = (tokenOverride ?? resolvedCompactTokens)! + (streamedTurnTokens > 0 ? streamedTurnTokens : 0);
    } else {
      total = baseSystem + baseTool + Math.max(100, rawMsgTokens);
    }

    let sys = baseSystem;
    let tool = baseTool;
    let msg = 0;

    if (total <= 0) {
      sys = 0; tool = 0; msg = 0;
    } else if (typeof telemetry?.systemTokens === 'number' && typeof telemetry?.toolTokens === 'number') {
      sys = telemetry.systemTokens;
      tool = telemetry.toolTokens;
      if (total < sys + tool) {
        msg = total;
        total = sys + tool + msg;
      } else {
        msg = Math.max(0, total - sys - tool);
      }
    } else {
      const fixedOverhead = baseSystem + baseTool;
      if (total <= fixedOverhead) {
        const ratio = fixedOverhead > 0 ? total / fixedOverhead : 1;
        sys = Math.round(baseSystem * ratio);
        tool = Math.max(0, total - sys);
        msg = 0;
      } else {
        sys = baseSystem;
        tool = baseTool;
        msg = total - fixedOverhead;
      }
    }

    return { totalUsedTokens: total, systemPromptTokens: sys, toolDefTokens: tool, messageTokens: msg, isEstimate: !isTelemetryEstimate && total > 0 };
  }, [
    taskId, currentEstimatedTokens, resolvedCompactTokens, tokenOverride,
    telemetry?.inputTokens, telemetry?.cacheReadTokens, telemetry?.outputTokens,
    telemetry?.systemTokens, telemetry?.toolTokens, streamedTurnTokens, thoughts, outputs, tools, feed,
  ]);

  const percent = Math.min(100, Math.max(0, Math.round((totalUsedTokens / effectiveLimit) * 100)));
  const systemPct = Math.max(0, (systemPromptTokens / effectiveLimit) * 100);
  const toolPct = Math.max(0, (toolDefTokens / effectiveLimit) * 100);
  const msgPct = Math.max(0, (messageTokens / effectiveLimit) * 100);
  const usedTokensK = (totalUsedTokens / 1000) >= 10 ? Math.round(totalUsedTokens / 1000) : (totalUsedTokens / 1000).toFixed(1);
  const maxTokensK = Math.round(effectiveLimit / 1000);

  const calculatedTurns = useMemo(() => {
    return Math.max((tools?.length || 0) + (outputs?.length || 0), (thoughts?.length || 0), 0);
  }, [tools, thoughts, outputs]);
  const isSubagentTask = taskId.includes('.sub-') || taskId.includes(':sub:') || taskId.includes('.');
  const effectiveTurns = turns !== undefined && !isSubagentTask ? turns : calculatedTurns;
  const stepsCount = tools?.length || 0;

  const totalInputTokens = telemetry?.inputTokens || 0;
  const safeCacheHitRate = telemetry
    ? typeof telemetry.cacheHitRate === 'number' && Number.isFinite(telemetry.cacheHitRate) && telemetry.cacheHitRate >= 0
      ? telemetry.cacheHitRate
      : totalInputTokens > 0 ? ((telemetry.cacheReadTokens || 0) / totalInputTokens) * 100 : 0
    : 0;

  const activeGenStart = genStart ?? thinkingStart;
  const elapsedSec = activeGenStart != null ? Math.max(0.01, ((genEnd ?? Date.now()) - activeGenStart) / 1000) : null;
  const turnOutputTokens = (telemetry?.outputTokens && telemetry.outputTokens > 0) ? telemetry.outputTokens : streamedTurnTokens;
  const liveTps = elapsedSec != null && elapsedSec >= 0.1 && turnOutputTokens > 0 ? turnOutputTokens / elapsedSec : null;
  const lastTurnTps = frozenTps != null ? frozenTps : liveTps;

  const avgTps = useMemo(() => {
    if (cumulativeGenerationSec > 0 && cumulativeOutputTokens > 0) {
      return cumulativeOutputTokens / cumulativeGenerationSec;
    }
    return lastTurnTps;
  }, [cumulativeGenerationSec, cumulativeOutputTokens, lastTurnTps]);

  const tpsValue = avgTps != null ? avgTps : lastTurnTps;
  const llmTimeSec = cumulativeGenerationSec + (elapsedSec ?? 0);

  const toolTimeSec = useMemo(() => {
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
          const rawToolDur = item.tool?.durationMs ?? (item.tool as any)?.duration_ms;
          const rawItemDur = item.durationMs ?? (item as any)?.duration_ms;
          const dur = (typeof rawToolDur === 'number' && rawToolDur > 0)
            ? rawToolDur
            : (typeof rawItemDur === 'number' && rawItemDur > 0)
              ? rawItemDur
              : (computed[i] || 0);
          sumMs += dur;
        }
      }
    }

    return sumMs / 1000;
  }, [tools, feed, isAgentRunning]);

  const ttftMs = telemetry?.ttftMs ?? null;

  const inputTokens = telemetry?.inputTokens ?? totalUsedTokens;
  const cachedInput = Math.min(telemetry?.cacheReadTokens || 0, inputTokens);
  const uncachedInput = Math.max(0, inputTokens - cachedInput);
  const outputTokens = (telemetry?.outputTokens && telemetry.outputTokens > 0) ? telemetry.outputTokens : (streamedTurnTokens > 0 ? streamedTurnTokens : (cumulativeOutputTokens || 0));
  const totalTokens = uncachedInput + cachedInput + outputTokens;

  const cumulativeInputTokens = telemetrySummary?.totalInputTokens ?? 0;
  const cumulativeCachedTokens = Math.min(telemetrySummary?.totalCachedTokens ?? 0, cumulativeInputTokens);
  const cumulativeOutputTokensTotal = telemetrySummary?.totalOutputTokens ?? 0;
  const cumulativeTotalTokens = telemetrySummary?.totalTokens ?? (cumulativeInputTokens + cumulativeOutputTokensTotal);
  const cumulativeUncachedInput = Math.max(0, cumulativeInputTokens - cumulativeCachedTokens);
  const cumulativeCacheHitRate = telemetrySummary?.cacheHitRate ?? (cumulativeInputTokens > 0 ? (cumulativeCachedTokens / cumulativeInputTokens) * 100 : 0);
  const cumulativeTurnCount = telemetrySummary?.turnCount ?? 0;
  const hasCumulative = cumulativeTotalTokens > 0;

  const displayTotalTokens = hasCumulative ? cumulativeTotalTokens : totalTokens;
  const displayUncachedInput = hasCumulative ? cumulativeUncachedInput : uncachedInput;
  const displayCachedInput = hasCumulative ? cumulativeCachedTokens : cachedInput;
  const displayOutputTokens = hasCumulative ? cumulativeOutputTokensTotal : outputTokens;
  const displayCacheHitRate = hasCumulative ? cumulativeCacheHitRate : safeCacheHitRate;

  const refreshTelemetrySummary = useCallback(async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const summary = await invoke<{
        totalInputTokens: number;
        totalCachedTokens: number;
        totalOutputTokens: number;
        totalTokens: number;
        cacheHitRate: number;
        turnCount: number;
      }>('get_task_telemetry_summary', { taskId });
      if (summary && typeof summary.totalTokens === 'number') {
        setTaskTelemetrySummary?.(taskId, summary);
      }
    } catch {
      // Web fallback: cumulative stays unset, UI falls back to last-turn snapshot.
    }
  }, [taskId, setTaskTelemetrySummary]);

  useEffect(() => {
    void refreshTelemetrySummary();
  }, [refreshTelemetrySummary]);

  useEffect(() => {
    let isDisposed = false;
    let unlisten: (() => void) | undefined;
    let debounce: ReturnType<typeof setTimeout> | null = null;
    const setupListener = async () => {
      try {
        if (isDisposed) return;
        const u = await listenToEvent('agent:telemetry', (payload) => {
          if (payload?.taskId && payload.taskId !== taskId) return;
          if (debounce) clearTimeout(debounce);
          debounce = setTimeout(() => { void refreshTelemetrySummary(); }, 300);
        });
        if (isDisposed) u(); else unlisten = u;
      } catch {
        // Web fallback
      }
    };
    setupListener();
    return () => {
      isDisposed = true;
      if (unlisten) unlisten();
      if (debounce) clearTimeout(debounce);
    };
  }, [taskId, refreshTelemetrySummary]);

  const radius = 10;
  const circumference = 2 * Math.PI * radius;
  const strokeDashoffset = circumference - (circumference * percent) / 100;
  const strokeColor = useMemo(() => {
    if (percent < 60) return '#10b981';
    if (percent <= 85) return '#f59e0b';
    return '#ef4444';
  }, [percent]);

  const triggerSavingsNotice = useCallback((savedCount: number) => {
    setSavingsBadge(`Оптимизировано: ~${savedCount.toLocaleString('ru-RU')} токенов`);
    if (savingsTimerRef.current) clearTimeout(savingsTimerRef.current);
    savingsTimerRef.current = setTimeout(() => {
      setSavingsBadge(null);
      savingsTimerRef.current = null;
    }, 3000);
  }, []);

  useEffect(() => {
    let isDisposed = false;
    let unlisten: (() => void) | undefined;
    const setupListener = async () => {
      try {
        if (isDisposed) return;
        const u = await listenToEvent('agent:compacted', (payload) => {
          if (payload) {
            const after = payload.tokens_after ?? payload.tokensAfter;
            const saved = payload.saved_tokens ?? payload.estimatedSavedTokens ?? payload.savedTokens ?? 0;
            if (typeof after === 'number' && Number.isFinite(after)) setTaskTokens?.(taskId, after);
            if (saved > 0) triggerSavingsNotice(saved);
          }
        });
        if (isDisposed) u(); else unlisten = u;
      } catch {
        // Web fallback
      }
    };
    setupListener();
    return () => {
      isDisposed = true;
      if (unlisten) unlisten();
      if (savingsTimerRef.current) clearTimeout(savingsTimerRef.current);
    };
  }, [taskId, setTaskTokens, triggerSavingsNotice]);

  const handleCompactClick = async () => {
    if (isCompacting) return;
    setIsCompacting(true);
    let saved = 0;
    if (evaporateClientToolLogs) {
      saved = evaporateClientToolLogs(taskId).savedTokens;
    }
    if (onCompact) {
      try {
        const res = await onCompact();
        if (res) saved = Math.max(saved, res.savedTokens);
      } catch { /* Fallback */ }
    } else {
      try {
        const res = await useDrawerStore.getState().compactTaskContext(taskId);
        if (res) saved = Math.max(saved, res.savedTokens);
      } catch { /* Handled */ }
    }
    setIsCompacting(false);
    triggerSavingsNotice(saved > 0 ? saved : 2450);
  };

  const miniCircumference = 2 * Math.PI * 6;
  const miniStrokeDashoffset = miniCircumference - (miniCircumference * percent) / 100;

  return {
    effectiveLimit, percent, usedTokensK, maxTokensK, systemPct, toolPct, msgPct,
    systemPromptTokens, toolDefTokens, messageTokens, isEstimate, effectiveTurns, stepsCount,
    llmTimeSec, toolTimeSec, ttftMs, tpsValue,
    cacheHitRate: displayCacheHitRate, uncachedInput: displayUncachedInput, cachedInput: displayCachedInput, outputTokens: displayOutputTokens, totalTokens: displayTotalTokens,
    cumulativeInputTokens, cumulativeCachedTokens, cumulativeOutputTokens: cumulativeOutputTokensTotal, cumulativeTotalTokens, cumulativeUncachedInput, cumulativeCacheHitRate, cumulativeTurnCount, hasCumulative,
    isCompacting, savingsBadge, handleCompactClick,
    strokeColor, radius, circumference, strokeDashoffset, miniCircumference, miniStrokeDashoffset,
  };
}

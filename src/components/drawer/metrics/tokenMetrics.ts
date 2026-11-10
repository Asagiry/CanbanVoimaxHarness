import { DrawerActivityItem, TaskTelemetry } from '../../../stores/drawerStore';
import { LiveStreamEntry, ToolExecution } from '../../../types';
import {
  CompactedFeedTokens,
  GaugeRingGeometry,
  TextCharsCache,
  TokenBreakdown,
} from './types';

export function parseCompactedFromFeed(feed?: DrawerActivityItem[]): CompactedFeedTokens {
  if (!feed || feed.length === 0) {
    return { latestCompactedTokens: undefined, postCompactionTokens: 0 };
  }

  let compactedIdx = -1;
  let baseTokens: number | undefined;

  for (let i = feed.length - 1; i >= 0; i--) {
    const it = feed[i];
    if (it.type === 'compacted' && it.tool) {
      const tData = it.tool as unknown as { tokensAfter?: unknown };
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
    postChars += it.content?.length || 0;
    if (it.tool) {
      postChars +=
        (it.tool.stdout?.length || 0) +
        (it.tool.stderr?.length || 0) +
        JSON.stringify(it.tool.params || {}).length;
    }
  }

  return {
    latestCompactedTokens: baseTokens,
    postCompactionTokens: Math.round(postChars / 2.2),
  };
}

export function createTextCharsCache(): TextCharsCache {
  return {
    valid: false,
    thoughtsLen: 0,
    outputsLen: 0,
    lastThoughtLen: 0,
    lastOutputLen: 0,
    textChars: 0,
  };
}

export function computeIncrementalTextChars(
  thoughtsArr: LiveStreamEntry[],
  outputsArr: LiveStreamEntry[],
  cache: TextCharsCache
): number {
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
    cache.valid = true;
    cache.thoughtsLen = tLen;
    cache.outputsLen = oLen;
    cache.lastThoughtId = lastTh?.id;
    cache.lastOutputId = lastOut?.id;
    cache.lastThoughtLen = (lastTh?.content ?? '').length;
    cache.lastOutputLen = (lastOut?.content ?? '').length;
    cache.textChars = textChars;
  };

  if (!cache.valid || tLen < cache.thoughtsLen || oLen < cache.outputsLen) {
    const textChars = fullTextRecalc();
    commitCache(textChars);
    return textChars;
  }

  const thoughtStable =
    cache.thoughtsLen === 0 || thoughtsArr[cache.thoughtsLen - 1]?.id === cache.lastThoughtId;
  const outputStable =
    cache.outputsLen === 0 || outputsArr[cache.outputsLen - 1]?.id === cache.lastOutputId;

  if (!thoughtStable || !outputStable) {
    const textChars = fullTextRecalc();
    commitCache(textChars);
    return textChars;
  }

  let delta = 0;
  let valid = true;
  if (cache.thoughtsLen > 0) {
    const grow = (thoughtsArr[cache.thoughtsLen - 1]?.content ?? '').length - cache.lastThoughtLen;
    if (grow < 0) valid = false;
    else delta += grow;
  }
  if (cache.outputsLen > 0) {
    const grow = (outputsArr[cache.outputsLen - 1]?.content ?? '').length - cache.lastOutputLen;
    if (grow < 0) valid = false;
    else delta += grow;
  }

  if (valid) {
    for (let i = cache.thoughtsLen; i < tLen; i++) delta += (thoughtsArr[i]?.content ?? '').length;
    for (let i = cache.outputsLen; i < oLen; i++) delta += (outputsArr[i]?.content ?? '').length;
    const textChars = cache.textChars + delta;
    commitCache(textChars);
    return textChars;
  }

  const textChars = fullTextRecalc();
  commitCache(textChars);
  return textChars;
}

export interface ComputeTokenBreakdownParams {
  taskId: string;
  persona?: string;
  currentEstimatedTokens?: number;
  resolvedCompactTokens?: number;
  tokenOverride?: number;
  telemetry?: TaskTelemetry;
  streamedTurnTokens: number;
  textChars: number;
  thoughtsLen: number;
  outputsLen: number;
  tools: ToolExecution[];
  feedLen: number;
}

export function computeTokenBreakdown({
  taskId,
  persona,
  currentEstimatedTokens,
  resolvedCompactTokens,
  tokenOverride,
  telemetry,
  streamedTurnTokens,
  textChars,
  thoughtsLen,
  outputsLen,
  tools,
  feedLen,
}: ComputeTokenBreakdownParams): TokenBreakdown {
  const telemetrySys =
    typeof telemetry?.systemTokens === 'number' &&
    Number.isFinite(telemetry.systemTokens) &&
    telemetry.systemTokens > 0
      ? telemetry.systemTokens
      : undefined;
  const telemetryTool =
    typeof telemetry?.toolTokens === 'number' &&
    Number.isFinite(telemetry.toolTokens) &&
    telemetry.toolTokens >= 0
      ? telemetry.toolTokens
      : undefined;

  const isExploratorySubagent = taskId.includes('.sub-') || taskId.includes(':sub:');
  const baseSystem = telemetrySys ?? (isExploratorySubagent ? 800 : persona ? 1800 : 1200);
  const baseTool = telemetryTool ?? (isExploratorySubagent ? 1000 : 2200);

  let messageChars = textChars;
  for (const t of tools) {
    messageChars +=
      (t?.stdout?.length || 0) +
      (t?.stderr?.length || 0) +
      JSON.stringify(t?.params || {}).length;
  }

  const hasMessages =
    thoughtsLen > 0 || outputsLen > 0 || tools.length > 0 || feedLen > 0 || messageChars > 0;
  const rawMsgTokens = Math.max(hasMessages ? 100 : 0, Math.round(messageChars / 2.2));
  const isTelemetryEstimate = telemetrySys !== undefined || telemetryTool !== undefined;

  let total: number;
  if (currentEstimatedTokens !== undefined) {
    total = currentEstimatedTokens;
  } else if ((telemetry?.inputTokens || 0) > 0) {
    total = telemetry!.inputTokens + (streamedTurnTokens > 0 ? streamedTurnTokens : 0);
  } else if (resolvedCompactTokens !== undefined || tokenOverride !== undefined) {
    total = (tokenOverride ?? resolvedCompactTokens)! + (streamedTurnTokens > 0 ? streamedTurnTokens : 0);
  } else {
    total = baseSystem + baseTool + Math.max(100, rawMsgTokens);
  }

  let sys = baseSystem;
  let tool = baseTool;
  let msg = 0;

  if (total <= 0) {
    sys = 0;
    tool = 0;
    msg = 0;
  } else if (
    typeof telemetry?.systemTokens === 'number' &&
    typeof telemetry?.toolTokens === 'number'
  ) {
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

  return {
    totalUsedTokens: total,
    systemPromptTokens: sys,
    toolDefTokens: tool,
    messageTokens: msg,
    isEstimate: !isTelemetryEstimate && total > 0,
  };
}

export function computeGaugeGeometry(
  breakdown: Pick<
    TokenBreakdown,
    'totalUsedTokens' | 'systemPromptTokens' | 'toolDefTokens' | 'messageTokens'
  >,
  effectiveLimit: number
): GaugeRingGeometry {
  const { totalUsedTokens, systemPromptTokens, toolDefTokens, messageTokens } = breakdown;
  const percent = Math.min(100, Math.max(0, Math.round((totalUsedTokens / effectiveLimit) * 100)));
  const systemPct = Math.max(0, (systemPromptTokens / effectiveLimit) * 100);
  const toolPct = Math.max(0, (toolDefTokens / effectiveLimit) * 100);
  const msgPct = Math.max(0, (messageTokens / effectiveLimit) * 100);
  const usedTokensK =
    totalUsedTokens / 1000 >= 10
      ? Math.round(totalUsedTokens / 1000)
      : (totalUsedTokens / 1000).toFixed(1);
  const maxTokensK = Math.round(effectiveLimit / 1000);

  const radius = 10;
  const circumference = 2 * Math.PI * radius;
  const strokeDashoffset = circumference - (circumference * percent) / 100;
  const miniCircumference = 2 * Math.PI * 6;
  const miniStrokeDashoffset = miniCircumference - (miniCircumference * percent) / 100;
  const strokeColor = percent < 60 ? '#10b981' : percent <= 85 ? '#f59e0b' : '#ef4444';

  return {
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
  };
}

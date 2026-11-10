export interface UseContextGaugeMetricsOptions {
  taskId: string;
  persona?: string;
  turns?: number;
  contextWindow?: number;
  stageLimit?: number;
  currentEstimatedTokens?: number;
  onCompact?: () => Promise<{ savedTokens: number; evaporatedCount: number } | void>;
}

export interface CompactedFeedTokens {
  latestCompactedTokens: number | undefined;
  postCompactionTokens: number;
}

export interface TextCharsCache {
  valid: boolean;
  thoughtsLen: number;
  outputsLen: number;
  lastThoughtId?: string;
  lastOutputId?: string;
  lastThoughtLen: number;
  lastOutputLen: number;
  textChars: number;
}

export interface TokenBreakdown {
  totalUsedTokens: number;
  systemPromptTokens: number;
  toolDefTokens: number;
  messageTokens: number;
  isEstimate: boolean;
}

export interface GaugeRingGeometry {
  percent: number;
  systemPct: number;
  toolPct: number;
  msgPct: number;
  usedTokensK: string | number;
  maxTokensK: number;
  radius: number;
  circumference: number;
  strokeDashoffset: number;
  miniCircumference: number;
  miniStrokeDashoffset: number;
  strokeColor: string;
}

export interface TokenDisplaySummary {
  cacheHitRate: number;
  uncachedInput: number;
  cachedInput: number;
  outputTokens: number;
  totalTokens: number;
  cumulativeInputTokens: number;
  cumulativeCachedTokens: number;
  cumulativeOutputTokens: number;
  cumulativeTotalTokens: number;
  cumulativeUncachedInput: number;
  cumulativeCacheHitRate: number;
  cumulativeTurnCount: number;
  hasCumulative: boolean;
}

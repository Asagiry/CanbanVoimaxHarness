import { Gauge, Database } from 'lucide-react';
import { t, useLanguageStore } from '../../i18n';
import { formatTps, formatCompactTokens, pluralize } from './telemetryFormatters';
import { ContextPopover } from './ContextPopover';
import { SessionStatsPopover } from './SessionStatsPopover';
import { TokenUsagePopover } from './TokenUsagePopover';
import { useContextGaugeMetrics } from './useContextGaugeMetrics';

export interface ContextGaugeProps {
  taskId: string;
  persona?: string;
  turns?: number;
  contextWindow?: number;
  stageLimit?: number;
  currentEstimatedTokens?: number;
  onCompact?: () => Promise<{ savedTokens: number; evaporatedCount: number } | void>;
  variant?: 'status-bar' | 'header-ring';
}

export default function ContextGauge({
  taskId,
  persona,
  turns,
  contextWindow,
  stageLimit = 128000,
  currentEstimatedTokens,
  onCompact,
  variant = 'status-bar',
}: ContextGaugeProps) {
  const metrics = useContextGaugeMetrics({
    taskId,
    persona,
    turns,
    contextWindow,
    stageLimit,
    currentEstimatedTokens,
    onCompact,
  });

  const lang = useLanguageStore((s) => s.language);

  if (variant === 'header-ring') {
    return (
      <div className="relative group inline-flex items-center">
        <div className="relative w-7 h-7 flex items-center justify-center cursor-pointer select-none rounded-full hover:ring-2 hover:ring-[#ebebeb] dark:hover:ring-[#21262d] transition">
          <svg className="w-7 h-7 -rotate-90 transform" viewBox="0 0 28 28">
            <circle
              cx="14"
              cy="14"
              r={metrics.radius}
              stroke="#e5e7eb"
              className="dark:stroke-[#21262d]"
              strokeWidth="2.5"
              fill="none"
            />
            <circle
              cx="14"
              cy="14"
              r={metrics.radius}
              stroke={metrics.strokeColor}
              strokeWidth="2.5"
              strokeDasharray={metrics.circumference}
              strokeDashoffset={metrics.strokeDashoffset}
              strokeLinecap="round"
              fill="none"
              className="transition-all duration-300"
            />
          </svg>
          <span className="absolute inset-0 flex items-center justify-center text-[8.5px] font-mono font-semibold text-[#171717] dark:text-[#f0f6fc] tabular-nums">
            {metrics.percent}%
          </span>
        </div>
      </div>
    );
  }

  const turnWord = pluralize(
    metrics.effectiveTurns,
    t('telemetry.turnOne'),
    t('telemetry.turnFew'),
    t('telemetry.turnMany'),
    lang
  );
  const stepWord = pluralize(
    metrics.stepsCount,
    t('telemetry.stepOne'),
    t('telemetry.stepFew'),
    t('telemetry.stepMany'),
    lang
  );

  const turnsStepsText =
    metrics.stepsCount > 0
      ? `${metrics.effectiveTurns} ${turnWord} ${metrics.stepsCount} ${stepWord}`
      : `${metrics.effectiveTurns} ${turnWord}`;

  const tpsText = formatTps(metrics.tpsValue);
  const compactTokens = formatCompactTokens(metrics.totalTokens);
  const cacheHitRateRounded = Math.round(Math.max(0, Math.min(100, metrics.cacheHitRate)));

  return (
    <div className="font-mono text-xs text-[#8f8f8f] dark:text-[#8b949e] flex items-center justify-center gap-2 py-1.5 select-none">
      {/* Speedometer Pill: Session statistics */}
      <div className="relative group inline-flex items-center">
        <div className="flex items-center gap-1.5 px-2 py-1 rounded-md cursor-pointer transition border border-transparent hover:border-[#d0d7de] dark:hover:border-[#30363d] hover:bg-[#ebebeb]/50 dark:hover:bg-[#161b22] text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc]">
          <Gauge className="w-3.5 h-3.5 shrink-0" />
          <span className="tabular-nums">
            {turnsStepsText} · {tpsText}
          </span>
        </div>
        <SessionStatsPopover
          llmTimeSec={metrics.llmTimeSec}
          toolTimeSec={metrics.toolTimeSec}
          ttftMs={metrics.ttftMs}
          tpsValue={metrics.tpsValue}
        />
      </div>

      {/* Token Usage Pill: Token usage */}
      <div className="relative group inline-flex items-center">
        <div className="flex items-center gap-1.5 px-2 py-1 rounded-md cursor-pointer transition border border-transparent hover:border-[#d0d7de] dark:hover:border-[#30363d] hover:bg-[#ebebeb]/50 dark:hover:bg-[#161b22] text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc]">
          <Database className="w-3.5 h-3.5 shrink-0" />
          <span className="tabular-nums">
            {compactTokens} · {t('telemetry.cacheLabel')} {cacheHitRateRounded}%
          </span>
        </div>
        <TokenUsagePopover
          cacheHitRate={metrics.cacheHitRate}
          uncachedInput={metrics.uncachedInput}
          cachedInput={metrics.cachedInput}
          outputTokens={metrics.outputTokens}
          totalTokens={metrics.totalTokens}
        />
      </div>

      {/* Context Ring Pill: Segment breakdown & compaction */}
      <div className="relative group inline-flex items-center">
        <div className="flex items-center gap-1.5 px-2 py-1 rounded-md cursor-pointer transition border border-transparent hover:border-[#d0d7de] dark:hover:border-[#30363d] hover:bg-[#ebebeb]/50 dark:hover:bg-[#161b22] text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc]">
          <div className="relative w-3.5 h-3.5 flex items-center justify-center shrink-0">
            <svg className="w-3.5 h-3.5 -rotate-90 transform" viewBox="0 0 16 16">
              <circle
                cx="8"
                cy="8"
                r="6"
                stroke="#ebebeb"
                className="dark:stroke-[#21262d]"
                strokeWidth="2"
                fill="none"
              />
              <circle
                cx="8"
                cy="8"
                r="6"
                stroke={metrics.strokeColor}
                strokeWidth="2"
                strokeDasharray={metrics.miniCircumference}
                strokeDashoffset={metrics.miniStrokeDashoffset}
                strokeLinecap="round"
                fill="none"
                className="transition-all duration-300"
              />
            </svg>
          </div>
          <span className="tabular-nums">{metrics.percent}%</span>
        </div>
        <ContextPopover
          percent={metrics.percent}
          usedTokensK={metrics.usedTokensK}
          maxTokensK={metrics.maxTokensK}
          systemPct={metrics.systemPct}
          toolPct={metrics.toolPct}
          msgPct={metrics.msgPct}
          systemPromptTokens={metrics.systemPromptTokens}
          toolDefTokens={metrics.toolDefTokens}
          messageTokens={metrics.messageTokens}
          isCompacting={metrics.isCompacting}
          savingsBadge={metrics.savingsBadge}
          onCompactClick={metrics.handleCompactClick}
        />
      </div>
    </div>
  );
}

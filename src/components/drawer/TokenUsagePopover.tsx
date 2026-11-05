import { Database } from 'lucide-react';
import { t } from '../../i18n';
import { formatTokensWithUnit } from './telemetryFormatters';

export interface TokenUsagePopoverProps {
  cacheHitRate: number;
  uncachedInput: number;
  cachedInput: number;
  outputTokens: number;
  totalTokens: number;
}

export function TokenUsagePopover({
  cacheHitRate,
  uncachedInput,
  cachedInput,
  outputTokens,
  totalTokens,
}: TokenUsagePopoverProps) {
  const cacheHitFormatted = `${Math.round(Math.max(0, Math.min(100, cacheHitRate)))}%`;

  return (
    <div className="absolute left-1/2 -translate-x-1/2 bottom-full mb-2.5 z-50 invisible opacity-0 group-hover:visible group-hover:opacity-100 transition-all duration-150 ease-out pointer-events-auto">
      <div className="absolute -bottom-3 left-0 right-0 h-3" />

      <div className="w-72 rounded-lg border border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114] p-3.5 shadow-lg text-[#171717] dark:text-[#f4f4f5] select-none font-sans">
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center gap-1.5 text-xs font-semibold text-[#171717] dark:text-[#f4f4f5]">
            <Database className="w-3.5 h-3.5 shrink-0" />
            <span>{t('telemetry.tokenUsage')}</span>
          </div>
          <span className="font-mono text-xs font-semibold text-[#171717] dark:text-[#f4f4f5] tabular-nums">
            {formatTokensWithUnit(totalTokens)}
          </span>
        </div>

        <div className="border-b border-[#ebebeb] dark:border-[#212126] mb-2.5" />

        <div className="space-y-1.5 text-xs">
          <div className="flex items-center justify-between">
            <span className="text-[#666666] dark:text-[#71717a]">
              {t('telemetry.cacheHit')}
            </span>
            <span className="font-mono tabular-nums text-[#171717] dark:text-[#f4f4f5]">
              {cacheHitFormatted}
            </span>
          </div>

          <div className="flex items-center justify-between">
            <span className="text-[#666666] dark:text-[#71717a]">
              {t('telemetry.uncachedInput')}
            </span>
            <span className="font-mono tabular-nums text-[#171717] dark:text-[#f4f4f5]">
              {formatTokensWithUnit(uncachedInput)}
            </span>
          </div>

          <div className="flex items-center justify-between">
            <span className="text-[#666666] dark:text-[#71717a]">
              {t('telemetry.cachedInput')}
            </span>
            <span className="font-mono tabular-nums text-[#171717] dark:text-[#f4f4f5]">
              {formatTokensWithUnit(cachedInput)}
            </span>
          </div>

          <div className="flex items-center justify-between">
            <span className="text-[#666666] dark:text-[#71717a]">
              {t('telemetry.output')}
            </span>
            <span className="font-mono tabular-nums text-[#171717] dark:text-[#f4f4f5]">
              {formatTokensWithUnit(outputTokens)}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}

export default TokenUsagePopover;

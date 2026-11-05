import { Gauge } from 'lucide-react';
import { t } from '../../i18n';
import {
  formatDurationCompact,
  formatTtft,
  formatTps,
} from './telemetryFormatters';

export interface SessionStatsPopoverProps {
  llmTimeSec: number;
  toolTimeSec: number;
  ttftMs?: number | null;
  tpsValue?: number | null;
}

export function SessionStatsPopover({
  llmTimeSec,
  toolTimeSec,
  ttftMs,
  tpsValue,
}: SessionStatsPopoverProps) {
  return (
    <div className="absolute left-1/2 -translate-x-1/2 bottom-full mb-2.5 z-50 invisible opacity-0 group-hover:visible group-hover:opacity-100 transition-all duration-150 ease-out pointer-events-auto">
      <div className="absolute -bottom-3 left-0 right-0 h-3" />

      <div className="w-72 rounded-lg border border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114] p-3.5 shadow-lg text-[#171717] dark:text-[#f4f4f5] select-none font-sans">
        <div className="flex items-center gap-1.5 text-xs font-semibold text-[#171717] dark:text-[#f4f4f5] mb-2">
          <Gauge className="w-3.5 h-3.5 shrink-0" />
          <span>{t('telemetry.sessionStatistics')}</span>
        </div>

        <div className="border-b border-[#ebebeb] dark:border-[#212126] mb-2.5" />

        <div className="space-y-1.5 text-xs">
          <div className="flex items-center justify-between">
            <span className="text-[#666666] dark:text-[#71717a]">
              {t('telemetry.llmTime')}
            </span>
            <span className="font-mono tabular-nums text-[#171717] dark:text-[#f4f4f5]">
              {formatDurationCompact(llmTimeSec)}
            </span>
          </div>

          <div className="flex items-center justify-between">
            <span className="text-[#666666] dark:text-[#71717a]">
              {t('telemetry.toolTime')}
            </span>
            <span className="font-mono tabular-nums text-[#171717] dark:text-[#f4f4f5]">
              {formatDurationCompact(toolTimeSec)}
            </span>
          </div>

          <div className="flex items-center justify-between">
            <span className="text-[#666666] dark:text-[#71717a]">
              {t('telemetry.avgTtft')}
            </span>
            <span className="font-mono tabular-nums text-[#171717] dark:text-[#f4f4f5]">
              {formatTtft(ttftMs)}
            </span>
          </div>

          <div className="flex items-center justify-between">
            <span className="text-[#666666] dark:text-[#71717a]">
              {t('telemetry.tps')}
            </span>
            <span className="font-mono tabular-nums text-[#171717] dark:text-[#f4f4f5]">
              {formatTps(tpsValue)}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}

export default SessionStatsPopover;

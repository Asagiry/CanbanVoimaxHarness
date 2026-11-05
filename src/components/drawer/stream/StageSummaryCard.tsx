import { CheckCircle2, RotateCcw } from 'lucide-react';
import type { DrawerActivityItem } from '../../../stores/drawerStore';
import { useTranslation } from '../../../stores/languageStore';
import { MarkdownContent } from './MarkdownContent';

export interface StageSummaryCardProps {
  activity: DrawerActivityItem;
  durationMs?: number;
  onImageClick?: (src: string, alt?: string) => void;
}

export default function StageSummaryCard({
  activity,
  durationMs,
  onImageClick,
}: StageSummaryCardProps) {
  const { t } = useTranslation();
  const isReject = (activity?.type as string) === 'reject';

  if (isReject) {
    return (
      <div className="animate-silk-entrance motion-reduce:animate-none rounded-lg border border-[#e4e4e7] dark:border-[#21262d] bg-white dark:bg-[#0d1117] shadow-xs overflow-hidden my-2.5 min-w-0">
        <div className="flex items-center justify-between px-3.5 py-2.5 bg-[#f8f8f9] dark:bg-[#161b22] border-b border-[#f4f4f5] dark:border-[#21262d] border-l-2 border-l-amber-500 dark:border-l-amber-400 select-none">
          <div className="flex items-center space-x-2 text-[#171717] dark:text-[#f0f6fc]">
            <RotateCcw className="h-4 w-4 text-amber-600 dark:text-amber-400 shrink-0" />
            <span className="text-xs font-semibold font-sans">
              {t('stageSummary.rejectTitle')}
            </span>
          </div>
          {durationMs ? (
            <span className="text-[10px] font-mono text-[#71717a] dark:text-[#8b949e]">
              {Math.round(durationMs / 1000)}s
            </span>
          ) : null}
        </div>
        <div className="p-3.5 text-sm leading-relaxed text-[#27272a] dark:text-[#d4d4d8] select-text min-w-0 break-words">
          <MarkdownContent content={activity?.content || ''} onImageClick={onImageClick} />
        </div>
      </div>
    );
  }

  return (
    <div className="animate-silk-entrance motion-reduce:animate-none rounded-lg border border-[#e4e4e7] dark:border-[#21262d] bg-white dark:bg-[#0d1117] shadow-xs overflow-hidden my-2.5 min-w-0">
      <div className="flex items-center justify-between px-3.5 py-2.5 bg-[#f8f8f9] dark:bg-[#161b22] border-b border-[#f4f4f5] dark:border-[#21262d] border-l-2 border-l-emerald-600 dark:border-l-emerald-400 select-none">
        <div className="flex items-center space-x-2 text-[#171717] dark:text-[#f0f6fc]">
          <CheckCircle2 className="h-4 w-4 text-emerald-600 dark:text-emerald-400 shrink-0" />
          <span className="text-xs font-semibold font-sans">
            {t('stageSummary.summaryTitle')}
          </span>
        </div>
        {durationMs ? (
          <span className="text-[10px] font-mono text-[#71717a] dark:text-[#8b949e]">
            {Math.round(durationMs / 1000)}s
          </span>
        ) : null}
      </div>
      <div className="p-3.5 text-sm leading-relaxed text-[#27272a] dark:text-[#d4d4d8] select-text min-w-0 break-words">
        <MarkdownContent content={activity?.content || ''} onImageClick={onImageClick} />
      </div>
    </div>
  );
}

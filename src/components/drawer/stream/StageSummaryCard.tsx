import { useMemo } from 'react';
import type { DrawerActivityItem } from '../../../stores/drawerStore';
import type { KanbanTask, PipelineStage } from '../../../types';
import { useTranslation } from '../../../stores/languageStore';
import { CheckCircle2 } from 'lucide-react';
import { MarkdownContent } from './MarkdownContent';
import { getStageSummaryLabel, getCleanSummaryContent, type ColumnSummaryInfo } from './streamHelpers';

export interface StageSummaryCardProps {
  activity: DrawerActivityItem;
  durationMs?: number;
  task: KanbanTask;
  subtasks?: KanbanTask[];
  columns?: ColumnSummaryInfo[] | PipelineStage[];
  onImageClick?: (src: string, alt?: string) => void;
}

export function StageSummaryCard({
  activity,
  durationMs,
  task,
  subtasks,
  columns,
  onImageClick,
}: StageSummaryCardProps) {
  const { language } = useTranslation();

  const label = useMemo(() => {
    return getStageSummaryLabel(activity, task, subtasks, columns as ColumnSummaryInfo[] | undefined, language);
  }, [activity, task, subtasks, columns, language]);

  const cleanContent = useMemo(() => {
    return getCleanSummaryContent(activity?.content);
  }, [activity?.content]);

  const effectiveDurationMs = durationMs ?? activity?.durationMs;

  return (
    <div className="animate-silk-entrance motion-reduce:animate-none rounded-lg border border-emerald-500/30 dark:border-emerald-500/30 bg-emerald-50/50 dark:bg-emerald-950/20 shadow-xs overflow-hidden my-2.5">
      <div className="flex items-center justify-between px-3.5 py-2.5 bg-emerald-100/60 dark:bg-emerald-900/30 border-b border-emerald-200/60 dark:border-emerald-800/40 select-none">
        <div className="flex items-center space-x-2 text-emerald-900 dark:text-emerald-200 min-w-0">
          <CheckCircle2 className="h-4 w-4 text-emerald-600 dark:text-emerald-400 shrink-0" />
          <span className="text-xs font-semibold font-sans truncate">
            {label}
          </span>
        </div>
        {effectiveDurationMs ? (
          <span className="text-[10px] font-mono text-emerald-700 dark:text-emerald-300 shrink-0 ml-2 tabular-nums">
            {Math.round(effectiveDurationMs / 1000)}s
          </span>
        ) : null}
      </div>
      <div className="p-3.5 text-sm leading-relaxed text-[#27272a] dark:text-[#d4d4d8] select-text">
        <MarkdownContent content={cleanContent} onImageClick={onImageClick} />
      </div>
    </div>
  );
}

export default StageSummaryCard;

import type { KanbanTask } from '../../../types';
import { useTranslation } from '../../../stores/languageStore';
import { Target, Paperclip, ZoomIn } from 'lucide-react';
import { MarkdownContent } from './MarkdownContent';
import { getCleanPrompt } from './streamHelpers';

export interface TaskObjectiveCardProps {
  task: KanbanTask;
  effectiveAttachments?: string[];
  onImageClick?: (src: string, alt?: string) => void;
}

export function TaskObjectiveCard({
  task,
  effectiveAttachments,
  onImageClick,
}: TaskObjectiveCardProps) {
  const { t } = useTranslation();

  return (
    <div className="rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] p-4 shadow-xs">
      <div className="flex items-center justify-between pb-2 mb-2.5 border-b border-[#ebebeb] dark:border-[#21262d]">
        <div className="flex items-center space-x-2">
          <Target className="h-4 w-4 text-[#171717] dark:text-[#f0f6fc]" />
          <span className="text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] font-sans">
            {t('drawer.taskObjective')}
          </span>
        </div>
        <span className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#8b949e]">
          {t('drawer.taskObjective')}
        </span>
      </div>
      <div className="text-sm text-[#171717] dark:text-[#f0f6fc] leading-relaxed select-text font-sans">
        <MarkdownContent
          content={getCleanPrompt(task.description, task.title)}
          onImageClick={onImageClick}
        />
      </div>

      {/* Task Attachments Gallery */}
      {effectiveAttachments && effectiveAttachments.length > 0 && (
        <div className="mt-3 pt-3 border-t border-[#ebebeb] dark:border-[#21262d]">
          <div className="text-[11px] font-mono font-medium text-[#666666] dark:text-[#8b949e] mb-2 flex items-center gap-1.5">
            <Paperclip className="h-3 w-3" />
            <span>{t('newTask.attachmentsLabel')} ({effectiveAttachments.length})</span>
          </div>
          <div className="flex flex-wrap gap-2">
            {effectiveAttachments.map((att, idx) => (
              <div
                key={idx}
                onClick={() => onImageClick?.(att, `Attachment ${idx + 1}`)}
                className="relative group rounded-md overflow-hidden border border-[#ebebeb] dark:border-[#30363d] bg-[#fafafa] dark:bg-[#161b22] cursor-pointer hover:border-[#171717] dark:hover:border-[#f0f6fc] transition shadow-2xs"
              >
                <img
                  src={att}
                  alt={`Attachment ${idx + 1}`}
                  className="h-20 w-28 object-cover transition-transform group-hover:scale-105"
                />
                <div className="absolute inset-0 bg-black/0 group-hover:bg-black/20 flex items-center justify-center transition-opacity opacity-0 group-hover:opacity-100">
                  <ZoomIn className="h-4 w-4 text-white drop-shadow-md" />
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

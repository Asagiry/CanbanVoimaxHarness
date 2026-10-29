import { useState, memo } from 'react';
import { GitFork, ChevronDown, MessageSquare } from 'lucide-react';
import { KanbanTask } from '../../types';
import { useTranslation } from '../../stores/languageStore';
import { useBoardStore } from '../../stores/boardStore';
import { useDrawerStore } from '../../stores/drawerStore';
import { useShallow } from 'zustand/react/shallow';
import { formatStageDuration } from './CardTimerBadge';

export interface CardWaitingSubtaskBadgeProps {
  taskId: string;
  waitingForSubtaskId?: string;
  onOpenDrawer?: (id: string) => void;
}

export const CardWaitingSubtaskBadge = memo(function CardWaitingSubtaskBadge({
  waitingForSubtaskId,
  onOpenDrawer,
}: CardWaitingSubtaskBadgeProps) {
  const { t } = useTranslation();
  const openDrawer = useDrawerStore((s) => s.openDrawer);

  const handleClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (waitingForSubtaskId) {
      if (onOpenDrawer) {
        onOpenDrawer(waitingForSubtaskId);
      } else {
        openDrawer(waitingForSubtaskId);
      }
    }
  };

  return (
    <button
      type="button"
      onClick={handleClick}
      className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#d97706] dark:text-[#f59e0b] bg-[#fefce8] dark:bg-[#161b22] border border-[#fef08a] dark:border-[#30363d] hover:border-[#f59e0b] transition cursor-pointer"
      aria-label={waitingForSubtaskId ? `${t('status.waitingSubtask')}: ${waitingForSubtaskId}` : undefined}
    >
      <span className="h-1.5 w-1.5 rounded-full bg-[#f59e0b]" />
      <span>{t('status.waitingSubtask')}</span>
    </button>
  );
});

export interface CardSubtaskProgressBarProps {
  completedCount: number;
  totalCount: number;
}

export const CardSubtaskProgressBar = memo(function CardSubtaskProgressBar({
  completedCount,
  totalCount,
}: CardSubtaskProgressBarProps) {
  if (totalCount <= 0) return null;
  const percentage = Math.min(100, Math.round((completedCount / totalCount) * 100));

  return (
    <div className="w-full mt-1.5 mb-1 flex items-center gap-1.5">
      <div className="flex-1 h-1 bg-[#e5e7eb] dark:bg-[#21262d] rounded-full overflow-hidden">
        <div
          className="h-full bg-[#8b5cf6] transition-all duration-300 rounded-full"
          style={{ width: `${percentage}%` }}
        />
      </div>
      <span className="text-[9px] font-mono tabular-nums text-[#8f8f8f] dark:text-[#8b949e]">
        {completedCount}/{totalCount}
      </span>
    </div>
  );
});

export interface CardSubtaskBadgeProps {
  task: KanbanTask;
  subtasks: KanbanTask[];
  isDone?: boolean;
}

export const CardSubtaskBadge = memo(function CardSubtaskBadge({
  task: _task,
  subtasks,
  isDone = false,
}: CardSubtaskBadgeProps) {
  const { language, t } = useTranslation();
  const openDrawer = useDrawerStore((s) => s.openDrawer);
  const columns = useBoardStore(useShallow((s) => s.columns));
  const [isSubtasksOpen, setIsSubtasksOpen] = useState(false);

  if (!isDone || subtasks.length === 0) {
    return null;
  }

  return (
    <div className="mt-2.5 pt-2 border-t border-[#ebebeb] dark:border-[#21262d]">
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          setIsSubtasksOpen(!isSubtasksOpen);
        }}
        className="flex items-center justify-between w-full text-[10px] font-mono font-medium text-[#4d4d4d] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] transition cursor-pointer py-1 px-1 rounded hover:bg-[#f4f4f5] dark:hover:bg-[#21262d]"
        aria-label={t('card.expandSubtasks')}
      >
        <span className="flex items-center gap-1.5">
          <GitFork className="h-3 w-3 text-[#8b5cf6]" />
          <span>{t('card.completedSubtasks', { count: subtasks.length })}</span>
        </span>
        <ChevronDown
          className={`h-3 w-3 transition-transform duration-150 ${
            isSubtasksOpen ? 'rotate-180' : ''
          }`}
        />
      </button>

      {isSubtasksOpen && (
        <div className="mt-1 space-y-1">
          {subtasks.map((sub) => {
            const subId = sub?.id ?? '';
            const subSuffix =
              sub?.subtaskIndex != null
                ? `.${sub.subtaskIndex}`
                : subId.split('.').pop()
                ? `.${subId.split('.').pop()}`
                : '';
            const subCol = columns.find((c) => c.id === sub?.columnId);
            const stageTitle = subCol?.title || sub?.columnId || '';

            const subFinish = sub.completedAt || (sub.status === 'done' ? sub.updatedAt : undefined);
            const subDuration =
              sub.totalElapsed && sub.totalElapsed > 0
                ? sub.totalElapsed
                : sub.createdAt && subFinish
                ? Math.max(0, Math.floor((subFinish - sub.createdAt) / 1000))
                : 0;

            return (
              <div
                key={subId || Math.random().toString(36)}
                onClick={(e) => {
                  e.stopPropagation();
                  if (subId) openDrawer(subId);
                }}
                className="flex items-center justify-between p-1.5 rounded bg-[#f8f9fa] dark:bg-[#0d1117] hover:bg-[#f1f3f5] dark:hover:bg-[#21262d] border border-[#e9ecef] dark:border-[#30363d] cursor-pointer transition text-[10px] font-mono group"
                aria-label={t('card.openSubtaskDialog', { id: sub.id })}
              >
                <div className="flex items-center gap-1.5 truncate min-w-0">
                  <span className="font-semibold text-[#8b5cf6] shrink-0">
                    {subSuffix} {stageTitle}
                  </span>
                  <span className="text-[#666666] dark:text-[#8b949e] truncate">
                    {(sub?.title ?? '').replace(/^\[.*?\]\s*/, '')}
                  </span>
                </div>
                <div className="flex items-center gap-1.5 shrink-0 ml-2">
                  {subDuration > 0 && (
                    <span className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#8b949e] tabular-nums">
                      {formatStageDuration(subDuration, language)}
                    </span>
                  )}
                  {sub?.verdict && sub.verdict !== 'NONE' ? (
                    <span
                      className={`px-1 py-0.2 rounded text-[9px] font-mono ${
                        sub?.verdict === 'APPROVED'
                          ? 'bg-[#f0fdf4] dark:bg-[#161b22] text-[#15803d] dark:text-[#7ee787] border border-[#bbf7d0] dark:border-[#238636]'
                          : sub?.verdict === 'APPROVED_WITH_CAVEATS'
                          ? 'bg-[#fffbeb] dark:bg-[#161b22] text-[#b45309] dark:text-[#facc15] border border-[#fde68a] dark:border-[#713f12]'
                          : 'bg-[#fef2f2] dark:bg-[#161b22] text-[#b91c1c] dark:text-[#ff7b72] border border-[#fecaca] dark:border-[#da3633]'
                      }`}
                    >
                      {sub?.verdict === 'APPROVED'
                        ? 'OK'
                        : sub?.verdict === 'APPROVED_WITH_CAVEATS'
                        ? 'CAVEATS'
                        : 'REWORK'}
                    </span>
                  ) : (
                    <span className="px-1 py-0.2 rounded text-[9px] font-mono bg-[#f4f4f5] dark:bg-[#161b22] text-[#666666] dark:text-[#8b949e] border border-[#ebebeb] dark:border-[#30363d]">
                      Готово
                    </span>
                  )}
                  <MessageSquare className="h-3 w-3 text-[#8f8f8f] dark:text-[#8b949e] group-hover:text-[#171717] dark:group-hover:text-[#f0f6fc] transition" />
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
});

export default CardSubtaskBadge;

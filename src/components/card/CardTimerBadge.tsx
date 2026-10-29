import { memo, useMemo, useCallback } from 'react';
import { Paperclip } from 'lucide-react';
import { KanbanTask, TaskStatus } from '../../types';
import { useTranslation, Language } from '../../stores/languageStore';
import { useBoardStore } from '../../stores/boardStore';
import { useDrawerStore } from '../../stores/drawerStore';
import { useShallow } from 'zustand/react/shallow';

export function formatCardDate(ts?: number): string {
  if (!ts || !Number.isFinite(ts)) return '';
  const d = new Date(ts);
  const dd = String(d.getDate()).padStart(2, '0');
  const mm = String(d.getMonth() + 1).padStart(2, '0');
  const hh = String(d.getHours()).padStart(2, '0');
  const min = String(d.getMinutes()).padStart(2, '0');
  return `${dd}.${mm} ${hh}:${min}`;
}

export function formatStageDuration(totalSeconds: number, lang: Language = 'en'): string {
  const s = Math.max(0, Math.floor(totalSeconds));
  const sUnit = lang === 'ru' ? 'с' : 's';
  const mUnit = lang === 'ru' ? 'м' : 'm';
  const hUnit = lang === 'ru' ? 'ч' : 'h';
  const dUnit = lang === 'ru' ? 'д' : 'd';
  if (s < 60) return `${s}${sUnit}`;
  const minutesTotal = Math.floor(s / 60);
  const sec = s % 60;
  if (minutesTotal < 60) {
    return sec > 0 ? `${minutesTotal}${mUnit} ${sec}${sUnit}` : `${minutesTotal}${mUnit}`;
  }
  const hoursTotal = Math.floor(minutesTotal / 60);
  const min = minutesTotal % 60;
  if (hoursTotal < 24) {
    return min > 0 ? `${hoursTotal}${hUnit} ${min}${mUnit}` : `${hoursTotal}${hUnit}`;
  }
  const days = Math.floor(hoursTotal / 24);
  const h = hoursTotal % 24;
  return h > 0 ? `${days}${dUnit} ${h}${hUnit}` : `${days}${dUnit}`;
}

export function formatTimerMMSS(totalSeconds: number): string {
  const s = Math.max(0, Math.floor(totalSeconds));
  const hrs = Math.floor(s / 3600);
  const mins = Math.floor((s % 3600) / 60);
  const secs = s % 60;
  if (hrs > 0) {
    return `${hrs}:${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
  }
  return `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
}

export function calculateStageDuration(task: KanbanTask, isRunning: boolean): number {
  const stageStart = task.stageEnteredAt || task.createdAt;
  return isRunning && stageStart
    ? Math.max(0, Math.floor((Date.now() - stageStart) / 1000))
    : (task.activeStageDuration ?? 0);
}

export function calculateTotalElapsed(task: KanbanTask, isRunning: boolean, isDone: boolean): number {
  const finishTime = task.completedAt || (isDone ? task.updatedAt : undefined);
  if (isDone) {
    return task.totalElapsed > 0
      ? task.totalElapsed
      : (task.createdAt && finishTime ? Math.max(0, Math.floor((finishTime - task.createdAt) / 1000)) : 0);
  }
  if (isRunning) {
    return task.createdAt ? Math.max(0, Math.floor((Date.now() - task.createdAt) / 1000)) : (task.totalElapsed ?? 0);
  }
  return task.totalElapsed ?? 0;
}

export interface CardTimerBadgeProps {
  task: KanbanTask;
  isDone?: boolean;
}

export const CardTimerBadge = memo(function CardTimerBadge({
  task,
  isDone = false,
}: CardTimerBadgeProps) {
  const { t } = useTranslation();
  const openDrawer = useDrawerStore((s) => s.openDrawer);
  const columns = useBoardStore(useShallow((s) => s.columns));

  const status: TaskStatus = task.status;
  const isRunning = status === 'running';
  const currentStageDuration = calculateStageDuration(task, isRunning);

  // Check waiting subtask state reactively (Invariant § 2.22 & Defect 12)
  const waitingSub = useBoardStore(
    useCallback(
      (s) => (task.waitingForSubtaskId ? s.tasks.find((t) => t.id === task.waitingForSubtaskId) : undefined),
      [task.waitingForSubtaskId]
    )
  );

  const waitingSubCol = useMemo(() => {
    if (!waitingSub) return null;
    return columns.find((c) => c.id === waitingSub.columnId) || null;
  }, [waitingSub, columns]);

  const isSubActive = waitingSub
    ? waitingSub.status !== 'done' && waitingSub.columnId !== 'ready' && !waitingSubCol?.terminal
    : true;

  if (!isDone && status !== 'done' && isSubActive && (task.waitingForSubtaskId || status === 'waiting_subtask')) {
    return (
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          if (task.waitingForSubtaskId) openDrawer(task.waitingForSubtaskId);
        }}
        className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#d97706] dark:text-[#f59e0b] bg-[#fefce8] dark:bg-[#161b22] border border-[#fef08a] dark:border-[#30363d] hover:border-[#f59e0b] transition cursor-pointer"
        aria-label={task.waitingForSubtaskId ? `${t('status.waitingSubtask')}: ${task.waitingForSubtaskId}` : undefined}
      >
        <span className="h-1.5 w-1.5 rounded-full bg-[#f59e0b]" />
        <span>{t('status.waitingSubtask')}</span>
      </button>
    );
  }

  switch (status) {
    case 'running':
      return (
        <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono tabular-nums text-[#15803d] dark:text-[#4ade80] bg-[#f0fdf4] dark:bg-[#161b22] border border-[#bbf7d0] dark:border-[#30363d]">
          <span className="h-1.5 w-1.5 rounded-full bg-[#15803d] dark:bg-[#4ade80]" />
          <span>{t('status.running')} {formatTimerMMSS(currentStageDuration)}</span>
        </span>
      );
    case 'paused':
      return (
        <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#b45309] dark:text-[#fbbf24] bg-[#fefce8] dark:bg-[#161b22] border border-[#fef08a] dark:border-[#30363d]">
          <span className="h-1.5 w-1.5 rounded-full bg-[#b45309] dark:bg-[#fbbf24]" />
          <span>{t('status.paused')}</span>
        </span>
      );
    case 'waiting_review':
      return (
        <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#7e22ce] dark:text-[#c084fc] bg-[#faf5ff] dark:bg-[#161b22] border border-[#e9d5ff] dark:border-[#30363d]">
          <span>{t('status.waitingReview')}</span>
        </span>
      );
    case 'done':
      return (
        <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#15803d] dark:text-[#4ade80] bg-[#f0fdf4] dark:bg-[#161b22] border border-[#bbf7d0] dark:border-[#30363d]">
          <span>{t('status.done')}</span>
        </span>
      );
    case 'blocked':
      return (
        <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#b91c1c] dark:text-[#f87171] bg-[#fef2f2] dark:bg-[#161b22] border border-[#fecaca] dark:border-[#30363d]">
          <span>{t('status.blocked')}</span>
        </span>
      );
    case 'idle':
    default:
      return (
        <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#666666] dark:text-[#8b949e] bg-[#f4f4f5] dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#21262d]">
          <span>{t('status.idle')}</span>
        </span>
      );
  }
});

export interface CardTimeFooterProps {
  task: KanbanTask;
  isDone?: boolean;
}

export const CardTimeFooter = memo(function CardTimeFooter({
  task,
  isDone = false,
}: CardTimeFooterProps) {
  const { language, t } = useTranslation();
  const isRunning = task.status === 'running';
  const currentTotalElapsed = calculateTotalElapsed(task, isRunning, isDone);

  if (!task.createdAt && currentTotalElapsed <= 0 && (!task.attachments || task.attachments.length === 0)) {
    return null;
  }

  return (
    <div className="mt-1 flex items-center justify-between gap-1 font-mono text-[10px] text-[#8f8f8f] dark:text-[#8b949e] tabular-nums whitespace-nowrap">
      <div className="flex items-center gap-1 min-w-0 truncate">
        {task.createdAt ? (
          <span>{t('card.created', { date: formatCardDate(task.createdAt) })}</span>
        ) : null}
        {task.createdAt && (currentTotalElapsed > 0 || isDone) ? <span>·</span> : null}
        {isDone ? (
          <span className="tabular-nums">
            {t('card.completedDuration', {
              time: currentTotalElapsed > 0
                ? formatStageDuration(currentTotalElapsed, language)
                : `<1${language === 'ru' ? 'с' : 's'}`,
            })}
          </span>
        ) : isRunning && currentTotalElapsed > 0 ? (
          <span className="tabular-nums">
            {t('card.elapsed', { time: formatStageDuration(currentTotalElapsed, language) })}
          </span>
        ) : currentTotalElapsed > 0 ? (
          <span className="tabular-nums">
            {formatStageDuration(currentTotalElapsed, language)}
          </span>
        ) : null}
      </div>
      {task.attachments && task.attachments.length > 0 && (
        <span
          className="inline-flex items-center gap-0.5 text-[10px] font-mono text-[#666666] dark:text-[#8b949e] shrink-0 tabular-nums"
          aria-label={t('newTask.attachmentsLabel')}
        >
          <Paperclip className="h-2.5 w-2.5 shrink-0" />
          <span className="tabular-nums">{task.attachments.length}</span>
        </span>
      )}
    </div>
  );
});

export default CardTimerBadge;

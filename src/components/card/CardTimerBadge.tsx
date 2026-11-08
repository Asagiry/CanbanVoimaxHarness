import { memo, useState, useEffect } from 'react';
import { Paperclip } from 'lucide-react';
import { KanbanTask, TaskStatus } from '../../types';
import { useTranslation, Language } from '../../stores/languageStore';
import { useAgentSessionStore } from '../../stores/agentSessionStore';

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

export function calculateTotalElapsed(task: KanbanTask, _isRunning: boolean, isDone: boolean): number {
  const finishTime = task.completedAt || (isDone ? task.updatedAt : undefined);
  if (isDone) {
    if (task.createdAt && finishTime) {
      return Math.max(0, Math.floor((finishTime - task.createdAt) / 1000));
    }
    return task.totalElapsed ?? 0;
  }
  if (task.createdAt) {
    return Math.max(0, Math.floor((Date.now() - task.createdAt) / 1000));
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
  const isSessionRunning = useAgentSessionStore((s) => s.isSessionRunning(task.id));
  const hasActiveSession = useAgentSessionStore((s) => s.hasActiveSession(task.id));

  const status: TaskStatus = task.status;
  const isRunning = status === 'running' || isSessionRunning;

  const [, setTick] = useState(0);
  useEffect(() => {
    if (!isRunning) return;
    const interval = setInterval(() => {
      setTick((prev) => prev + 1);
    }, 1000);
    return () => clearInterval(interval);
  }, [isRunning]);

  const currentStageDuration = calculateStageDuration(task, isRunning);

  if (isRunning) {
    return (
      <span
        className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono tabular-nums text-[#15803d] dark:text-[#4ade80] bg-[#f0fdf4] dark:bg-[#17171b] border border-[#bbf7d0] dark:border-[#2c2c33]"
      >
        <span className="h-1.5 w-1.5 rounded-full bg-[#15803d] dark:bg-[#4ade80]" />
        <span>{t('status.running')} {formatTimerMMSS(currentStageDuration)}</span>
      </span>
    );
  }

  if (hasActiveSession && (status === 'paused' || status === 'waiting_subagent')) {
    return (
      <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#d97706] dark:text-[#f59e0b] bg-[#fefce8] dark:bg-[#17171b] border border-[#fef08a] dark:border-[#2c2c33]">
        <span className="h-1.5 w-1.5 rounded-full bg-[#f59e0b] animate-pulse" />
        <span>{t('status.waitingSubagent')}</span>
      </span>
    );
  }

  if (status === 'done' || isDone || Boolean(task.waitingForSubtaskId)) {
    const isCompleted = status === 'done' || isDone;
    const sumDurations = task.stageDurations
      ? Object.values(task.stageDurations).reduce((acc, v) => acc + (typeof v === 'number' && Number.isFinite(v) ? v : 0), 0)
      : 0;
    const computedElapsed = calculateTotalElapsed(task, false, isCompleted);
    const totalActiveSec = typeof task.totalElapsed === 'number' && task.totalElapsed > 0
      ? task.totalElapsed
      : (sumDurations > 0 ? sumDurations : computedElapsed);

    const doneLabel = isCompleted && totalActiveSec > 0
      ? t('status.doneWithDuration', { time: formatTimerMMSS(totalActiveSec) })
      : t('status.done');

    return (
      <span
        className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono tabular-nums text-[#15803d] dark:text-[#4ade80] bg-[#f0fdf4] dark:bg-[#17171b] border border-[#bbf7d0] dark:border-[#2c2c33]"
      >
        <span>{doneLabel}</span>
      </span>
    );
  }

  switch (status) {
    case 'waiting_subagent':
      return (
        <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#d97706] dark:text-[#f59e0b] bg-[#fefce8] dark:bg-[#17171b] border border-[#fef08a] dark:border-[#2c2c33]">
          <span className="h-1.5 w-1.5 rounded-full bg-[#f59e0b] animate-pulse" />
          <span>{t('status.waitingSubagent')}</span>
        </span>
      );
    case 'paused':
      return (
        <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#b45309] dark:text-[#fbbf24] bg-[#fefce8] dark:bg-[#17171b] border border-[#fef08a] dark:border-[#2c2c33]">
          <span className="h-1.5 w-1.5 rounded-full bg-[#b45309] dark:bg-[#fbbf24]" />
          <span>{t('status.paused')}</span>
        </span>
      );
    case 'waiting_review':
      return (
        <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#7e22ce] dark:text-[#c084fc] bg-[#faf5ff] dark:bg-[#17171b] border border-[#e9d5ff] dark:border-[#2c2c33]">
          <span>{t('status.waitingReview')}</span>
        </span>
      );
    case 'blocked':
      return (
        <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#b91c1c] dark:text-[#f87171] bg-[#fef2f2] dark:bg-[#17171b] border border-[#fecaca] dark:border-[#2c2c33]">
          <span>{t('status.blocked')}</span>
        </span>
      );
    case 'idle':
    default:
      return (
        <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#666666] dark:text-[#71717a] bg-[#f4f4f5] dark:bg-[#111114] border border-[#ebebeb] dark:border-[#212126]">
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
  const isBacklogIdle = !isDone && task.stageId === 'backlog' && task.status !== 'running' && !task.isTimerRunning;
  const shouldTick = !isDone && Boolean(task.createdAt) && !isBacklogIdle;
  const [, setTick] = useState(0);

  useEffect(() => {
    if (!shouldTick) return;
    const timer = setInterval(() => {
      setTick((prev) => prev + 1);
    }, 1000);
    return () => clearInterval(timer);
  }, [shouldTick]);

  const isRunning = task.status === 'running';
  const currentTotalElapsed = calculateTotalElapsed(task, isRunning, isDone);

  if (!task.createdAt && currentTotalElapsed <= 0 && (!task.attachments || task.attachments.length === 0)) {
    return null;
  }

  return (
    <div className="mt-1 flex items-center justify-between gap-1 font-mono text-[10px] text-[#8f8f8f] dark:text-[#71717a] tabular-nums whitespace-nowrap">
      <div className="flex items-center gap-1 min-w-0 truncate">
        {task.createdAt ? (
          <span>{t('card.created', { date: formatCardDate(task.createdAt) })}</span>
        ) : null}
        {task.createdAt && !isBacklogIdle ? <span>·</span> : null}
        {isDone ? (
          <span className="tabular-nums">
            {t('card.completedDuration', {
              time: currentTotalElapsed > 0
                ? formatStageDuration(currentTotalElapsed, language)
                : `<1${language === 'ru' ? 'с' : 's'}`,
            })}
          </span>
        ) : !isBacklogIdle ? (
          <span className="tabular-nums">
            {t('card.elapsed', { time: formatStageDuration(currentTotalElapsed, language) })}
          </span>
        ) : null}
      </div>
      {task.attachments && task.attachments.length > 0 && (
        <span
          className="inline-flex items-center gap-0.5 text-[10px] font-mono text-[#666666] dark:text-[#71717a] shrink-0 tabular-nums"
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

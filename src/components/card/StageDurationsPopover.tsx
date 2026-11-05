import { memo, useLayoutEffect, useRef, useState, useMemo } from 'react';
import { createPortal } from 'react-dom';
import { KanbanTask, PipelineColumn } from '../../types';
import { useBoardStore } from '../../stores/boardStore';
import { useTranslation } from '../../stores/languageStore';
import { formatTimerMMSS } from './CardTimerBadge';

export interface StageDurationsPopoverProps {
  task: KanbanTask;
  isOpen: boolean;
  triggerRect: DOMRect | null;
  columns?: PipelineColumn[];
  allTasks?: KanbanTask[];
  onMouseEnter?: () => void;
  onMouseLeave?: () => void;
}

interface StageItem {
  id: string;
  title: string;
  duration: number;
  isRunning: boolean;
}

const POPOVER_WIDTH = 230;
const VIEWPORT_MARGIN = 8;

export const StageDurationsPopover = memo(function StageDurationsPopover({
  task,
  isOpen,
  triggerRect,
  columns: propColumns,
  allTasks: propTasks,
  onMouseEnter,
  onMouseLeave,
}: StageDurationsPopoverProps) {
  const popoverRef = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<{ top: number; left: number }>({ top: 0, left: 0 });
  const storeColumns = useBoardStore((s) => s.columns);
  const storeStages = useBoardStore((s) => s.stages);
  const storeTasks = useBoardStore((s) => s.tasks);
  const { t } = useTranslation();

  const columns = propColumns || (storeColumns && storeColumns.length > 0 ? storeColumns : storeStages) || [];
  const allTasks = propTasks || storeTasks || [];

  const isRunning = task.status === 'running' || task.isTimerRunning;

  const { stageItems, totalActiveSec } = useMemo(() => {
    const rawDurations = task.stageDurations || {};
    const aggregated: Record<string, number> = { ...rawDurations };

    const subtasks = allTasks.filter((sub) => sub.parentId === task.id);
    for (const sub of subtasks) {
      if (sub.stageDurations && typeof sub.stageDurations === 'object') {
        for (const [sId, dur] of Object.entries(sub.stageDurations)) {
          if (typeof dur === 'number' && dur > 0) {
            aggregated[sId] = (aggregated[sId] || 0) + dur;
          }
        }
      } else if (sub.stageId) {
        const subSec = sub.totalElapsed || sub.activeStageDuration || 0;
        if (subSec > 0) {
          aggregated[sub.stageId] = (aggregated[sub.stageId] || 0) + subSec;
        }
      }
    }

    if (isRunning && task.stageId) {
      const liveCurrent = task.activeStageDuration || 0;
      aggregated[task.stageId] = Math.max(aggregated[task.stageId] || 0, liveCurrent);
    }

    const items: StageItem[] = [];
    const visitedColIds = new Set<string>();

    for (const col of columns) {
      const dur = aggregated[col.id] || 0;
      const isColRunning = isRunning && task.stageId === col.id;
      if (dur > 0 || isColRunning) {
        items.push({
          id: col.id,
          title: col.title,
          duration: dur,
          isRunning: isColRunning,
        });
        visitedColIds.add(col.id);
      }
    }

    for (const [sId, dur] of Object.entries(aggregated)) {
      if (!visitedColIds.has(sId) && typeof dur === 'number' && dur > 0) {
        items.push({
          id: sId,
          title: sId,
          duration: dur,
          isRunning: isRunning && task.stageId === sId,
        });
      }
    }

    const sumSec = items.reduce((acc, it) => acc + it.duration, 0);
    const totalSec = typeof task.totalElapsed === 'number' && task.totalElapsed > 0
      ? task.totalElapsed
      : sumSec;

    return { stageItems: items, totalActiveSec: totalSec };
  }, [task, allTasks, columns, isRunning]);

  useLayoutEffect(() => {
    if (!isOpen || !triggerRect) return;
    const el = popoverRef.current;
    const popoverHeight = el?.offsetHeight || 160;

    let left = triggerRect.left;
    if (left + POPOVER_WIDTH > window.innerWidth - VIEWPORT_MARGIN) {
      left = Math.max(VIEWPORT_MARGIN, window.innerWidth - POPOVER_WIDTH - VIEWPORT_MARGIN);
    }

    let top = triggerRect.bottom + 6;
    if (top + popoverHeight > window.innerHeight - VIEWPORT_MARGIN) {
      const topAbove = triggerRect.top - popoverHeight - 6;
      top = topAbove >= VIEWPORT_MARGIN ? topAbove : Math.max(VIEWPORT_MARGIN, window.innerHeight - popoverHeight - VIEWPORT_MARGIN);
    }

    setPos({ left, top });
  }, [isOpen, triggerRect, stageItems]);

  if (!isOpen || !triggerRect || stageItems.length === 0) {
    return null;
  }

  const content = (
    <div
      ref={popoverRef}
      role="tooltip"
      data-testid="stage-durations-popover"
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
      style={{ top: `${pos.top}px`, left: `${pos.left}px`, width: `${POPOVER_WIDTH}px` }}
      className="fixed z-50 bg-[#17171b] border border-[#2c2c33] text-[#a1a1aa] rounded-lg shadow-xl p-2.5 text-xs font-sans pointer-events-auto select-none transition-opacity duration-150 animate-in fade-in"
    >
      <div className="text-[10px] font-semibold text-[#71717a] uppercase tracking-wider mb-2">
        {t('card.stageDurationsTitle')}
      </div>
      <div className="space-y-1">
        {stageItems.map((st) => (
          <div key={st.id} className="flex items-center justify-between gap-2 py-0.5 text-xs">
            <div className="flex items-center gap-1.5 min-w-0">
              {st.isRunning ? (
                <span className="h-1.5 w-1.5 rounded-full bg-[#22c55e] animate-pulse shrink-0" />
              ) : (
                <span className="h-1.5 w-1.5 rounded-full bg-[#3a3a44] shrink-0" />
              )}
              <span className="truncate max-w-[130px] text-[#f4f4f5] font-medium" title={st.title}>
                {st.title}
              </span>
            </div>
            <span className="font-mono tabular-nums text-[11px] text-[#7ee787]">
              {formatTimerMMSS(st.duration)}
            </span>
          </div>
        ))}
      </div>
      <div className="my-1.5 border-t border-[#2c2c33]" />
      <div className="flex items-center justify-between text-xs pt-0.5">
        <span className="text-[#71717a] font-medium">{t('card.totalActiveTime')}</span>
        <span className="font-mono font-semibold tabular-nums text-[#f4f4f5]">
          {formatTimerMMSS(totalActiveSec)}
        </span>
      </div>
    </div>
  );

  if (typeof document === 'undefined') {
    return content;
  }

  return createPortal(content, document.body);
});

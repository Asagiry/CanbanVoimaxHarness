import { useState, useRef, useEffect, useLayoutEffect, memo } from 'react';
import { createPortal } from 'react-dom';
import { CheckCircle2, ListTodo, Circle } from 'lucide-react';
import { TodoItem } from '../../types';
import { useTranslation } from '../../stores/languageStore';
import { useDrawerStore } from '../../stores/drawerStore';

export interface CardTodoProgressProps {
  todos?: TodoItem[];
  stageTitle?: string;
  stageColor?: string;
  isDragging?: boolean;
  taskId?: string;
}

export const CardTodoProgress = memo(function CardTodoProgress({
  todos,
  stageTitle,
  stageColor,
  isDragging,
  taskId,
}: CardTodoProgressProps) {
  const { t, language } = useTranslation();
  const openDrawer = useDrawerStore((s) => s.openDrawer);

  const [isHovered, setIsHovered] = useState(false);
  const [coords, setCoords] = useState<{ top: number; left: number; placement: 'bottom' | 'top' } | null>(null);

  const triggerRef = useRef<HTMLDivElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const hideTimerRef = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (hideTimerRef.current) {
        clearTimeout(hideTimerRef.current);
        hideTimerRef.current = null;
      }
    };
  }, []);

  // Close immediately when card is being dragged
  useEffect(() => {
    if (isDragging && isHovered) {
      if (hideTimerRef.current) {
        clearTimeout(hideTimerRef.current);
        hideTimerRef.current = null;
      }
      setIsHovered(false);
      setCoords(null);
    }
  }, [isDragging, isHovered]);

  // Position calculation and boundary collision handling
  const updatePosition = () => {
    if (!triggerRef.current) return;
    const rect = triggerRef.current.getBoundingClientRect();
    const popoverWidth = popoverRef.current?.offsetWidth || 340;
    const popoverHeight = popoverRef.current?.offsetHeight || 220;

    let left = rect.left;
    left = Math.max(8, Math.min(left, window.innerWidth - popoverWidth - 8));

    let top = rect.bottom + 6;
    let placement: 'bottom' | 'top' = 'bottom';
    if (top + popoverHeight > window.innerHeight - 8) {
      const topAbove = rect.top - popoverHeight - 6;
      if (topAbove >= 8) {
        top = topAbove;
        placement = 'top';
      } else {
        top = Math.max(8, window.innerHeight - popoverHeight - 8);
        placement = 'bottom';
      }
    }

    setCoords({ top, left, placement });
  };

  useLayoutEffect(() => {
    if (isHovered) {
      updatePosition();
    }
  }, [isHovered, todos]);

  // Close on window scroll, column scroll (capture phase), resize, or pointerdown outside
  useEffect(() => {
    if (!isHovered) return;

    const handleClose = () => {
      if (hideTimerRef.current) {
        clearTimeout(hideTimerRef.current);
        hideTimerRef.current = null;
      }
      setIsHovered(false);
      setCoords(null);
    };

    const handleScroll = (e: Event) => {
      if (e.target instanceof Node && popoverRef.current?.contains(e.target)) {
        return;
      }
      handleClose();
    };

    const handlePointerDown = (e: PointerEvent) => {
      if (e.target instanceof Node && popoverRef.current?.contains(e.target)) {
        return;
      }
      if (e.target instanceof Node && triggerRef.current?.contains(e.target)) {
        return;
      }
      handleClose();
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        handleClose();
      }
    };

    const handleTriggerWheel = (e: WheelEvent) => {
      if (!listRef.current) return;
      if (e.deltaY !== 0) {
        e.preventDefault();
        e.stopPropagation();
        listRef.current.scrollTop += e.deltaY;
      }
    };

    const handlePopoverWheel = (e: WheelEvent) => {
      if (!listRef.current) return;
      e.stopPropagation();
      const list = listRef.current;
      const isAtTop = list.scrollTop <= 0 && e.deltaY < 0;
      const isAtBottom = list.scrollTop + list.clientHeight >= list.scrollHeight - 1 && e.deltaY > 0;
      if (isAtTop || isAtBottom) {
        e.preventDefault();
        return;
      }
      if (!list.contains(e.target as Node)) {
        e.preventDefault();
        list.scrollTop += e.deltaY;
      }
    };

    const triggerEl = triggerRef.current;
    const popoverEl = popoverRef.current;

    if (triggerEl) {
      triggerEl.addEventListener('wheel', handleTriggerWheel, { passive: false });
    }
    if (popoverEl) {
      popoverEl.addEventListener('wheel', handlePopoverWheel, { passive: false });
    }

    window.addEventListener('scroll', handleScroll, { capture: true, passive: true });
    window.addEventListener('resize', handleClose);
    window.addEventListener('pointerdown', handlePointerDown);
    window.addEventListener('keydown', handleKeyDown);

    return () => {
      if (triggerEl) {
        triggerEl.removeEventListener('wheel', handleTriggerWheel);
      }
      if (popoverEl) {
        popoverEl.removeEventListener('wheel', handlePopoverWheel);
      }
      window.removeEventListener('scroll', handleScroll, { capture: true });
      window.removeEventListener('resize', handleClose);
      window.removeEventListener('pointerdown', handlePointerDown);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [isHovered]);

  if (!todos || todos.length === 0) {
    return null;
  }

  const totalCount = todos.length;
  const completedCount = todos.filter((t) => t.status === 'completed').length;
  const inProgressCount = todos.filter((t) => t.status === 'in_progress').length;
  const pendingCount = totalCount - completedCount - inProgressCount;
  const percent = Math.round((completedCount / totalCount) * 100);
  const isAllCompleted = completedCount === totalCount;

  const handleMouseEnter = () => {
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
      hideTimerRef.current = null;
    }
    if (!isHovered) {
      setCoords(null);
    }
    setIsHovered(true);
  };

  const handleMouseLeave = () => {
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
    }
    hideTimerRef.current = window.setTimeout(() => {
      setIsHovered(false);
      setCoords(null);
      hideTimerRef.current = null;
    }, 300);
  };

  return (
    <>
      {/* Progress Strip Container */}
      <div className="mb-2 flex items-center gap-2 select-none">
        {/* Interactive Checklist Badge Trigger (Icon + Numbers + In-Progress Dot) */}
        <div
          ref={triggerRef}
          onMouseEnter={handleMouseEnter}
          onMouseLeave={handleMouseLeave}
          className="flex items-center gap-1 font-mono text-[10px] tabular-nums font-medium text-[#666666] dark:text-[#8b949e] shrink-0 cursor-pointer rounded px-1 -mx-1 py-0.5 -my-0.5 transition-colors hover:bg-neutral-100/80 dark:hover:bg-[#21262d]/80 hover:text-[#171717] dark:hover:text-[#f0f6fc]"
          aria-label={t('card.todosProgress', { completed: completedCount, total: totalCount, percent })}
        >
          {isAllCompleted ? (
            <CheckCircle2 className="h-3 w-3 text-[#15803d] dark:text-[#4ade80] shrink-0" />
          ) : (
            <ListTodo className="h-3 w-3 text-[#8f8f8f] dark:text-[#8b949e] shrink-0" />
          )}
          <span className="inline-flex items-center gap-1 translate-y-px leading-none">
            <span className={completedCount > 0 ? 'text-[#15803d] dark:text-[#4ade80] font-semibold' : 'text-[#171717] dark:text-[#f0f6fc]'}>
              {completedCount}
            </span>
            <span className="text-[#a1a1aa] dark:text-[#6e7681]">/</span>
            <span>{totalCount}</span>
          </span>
          {inProgressCount > 0 && !isAllCompleted && (
            <span className="inline-block h-1.5 w-1.5 rounded-full bg-amber-500 animate-pulse ml-0.5" />
          )}
        </div>

        {/* Micro Progress Bar (Inert to Tooltip Hover) */}
        <div className="flex-1 h-1 rounded-full bg-[#f4f4f5] dark:bg-[#21262d] overflow-hidden border border-[#ebebeb]/60 dark:border-[#30363d]/60">
          <div
            className="h-full rounded-full bg-[#15803d] dark:bg-[#4ade80] transition-all duration-300"
            style={{ width: `${percent}%` }}
          />
        </div>

        {/* Percentage Label */}
        <span className="font-mono text-[9px] tabular-nums text-[#8f8f8f] dark:text-[#8b949e] shrink-0 leading-none">
          {percent}%
        </span>
      </div>

      {/* Floating Popover Portal */}
      {isHovered &&
        createPortal(
          <div
            ref={popoverRef}
            onMouseEnter={handleMouseEnter}
            onMouseLeave={handleMouseLeave}
            onClick={(e) => e.stopPropagation()}
            onPointerDown={(e) => e.stopPropagation()}
            style={{
              top: `${coords?.top ?? -9999}px`,
              left: `${coords?.left ?? -9999}px`,
              visibility: coords ? 'visible' : 'hidden',
            }}
            className={`fixed z-[9999] w-[320px] sm:w-[350px] max-w-[calc(100vw-24px)] bg-white dark:bg-[#161b22] border border-neutral-200 dark:border-[#30363d] shadow-2xl rounded-lg p-3 text-left select-text overscroll-contain ${
              coords?.placement === 'top'
                ? "after:content-[''] after:absolute after:-bottom-3 after:left-0 after:right-0 after:h-3"
                : "before:content-[''] before:absolute before:-top-3 before:left-0 before:right-0 before:h-3"
            }`}
          >
            {/* Header: Stage badge & progress counter */}
            <div className="flex items-center justify-between gap-2 pb-2 mb-2 border-b border-neutral-100 dark:border-[#21262d]">
              <div className="flex items-center gap-1.5 min-w-0">
                <span
                  className="w-2 h-2 rounded-full shrink-0"
                  style={{ backgroundColor: stageColor || '#6366f1' }}
                />
                <span className="font-semibold text-xs text-neutral-800 dark:text-[#f0f6fc] truncate">
                  {stageTitle || t('card.stageChecklist')}
                </span>
              </div>
              <div className="flex items-center gap-1.5 font-mono text-xs tabular-nums shrink-0">
                <span className={completedCount > 0 ? 'text-[#15803d] dark:text-[#4ade80] font-semibold' : 'text-neutral-600 dark:text-[#8b949e]'}>
                  {completedCount}/{totalCount}
                </span>
                <span className="text-neutral-300 dark:text-[#6e7681]">·</span>
                <span className="text-neutral-500 dark:text-[#8b949e]">
                  {percent}%
                </span>
              </div>
            </div>

            {/* Summary strip with counts & progress bar */}
            <div className="mb-2.5">
              <div className="flex items-center justify-between text-[11px] text-neutral-500 dark:text-[#8b949e] font-sans mb-1.5">
                <div className="flex items-center gap-1.5 flex-wrap">
                  <span className={completedCount > 0 ? 'text-[#15803d] dark:text-[#4ade80] font-medium' : ''}>
                    {t('card.todosCompleted', { count: completedCount })}
                  </span>
                  {inProgressCount > 0 && (
                    <>
                      <span className="text-neutral-300 dark:text-[#6e7681]">·</span>
                      <span className="text-amber-600 dark:text-amber-400 font-medium flex items-center gap-1">
                        <span className="w-1.5 h-1.5 rounded-full bg-amber-500 animate-pulse" />
                        {t('card.inProgressCount', { count: inProgressCount })}
                      </span>
                    </>
                  )}
                  <span className="text-neutral-300 dark:text-[#6e7681]">·</span>
                  <span>
                    {t('card.todosPending', { count: pendingCount })}
                  </span>
                </div>
              </div>
              <div className="h-1.5 w-full rounded-full bg-neutral-100 dark:bg-[#21262d] overflow-hidden border border-neutral-200/60 dark:border-[#30363d]/60">
                <div
                  className="h-full rounded-full bg-[#15803d] dark:bg-[#4ade80] transition-all duration-300"
                  style={{ width: `${percent}%` }}
                />
              </div>
            </div>

            {/* Scrollable todos list */}
            <div
              ref={listRef}
              className="max-h-60 overflow-y-auto divide-y divide-neutral-100 dark:divide-[#21262d] pr-1 overscroll-contain"
            >
              {todos.map((item, idx) => {
                const isCompleted = item.status === 'completed';
                const isInProgress = item.status === 'in_progress';
                return (
                  <div
                    key={item.id || idx}
                    className={`py-1.5 px-2 rounded flex items-start gap-2 text-xs transition-colors ${
                      isInProgress
                        ? 'bg-amber-50/60 dark:bg-amber-950/30 text-neutral-800 dark:text-[#f0f6fc]'
                        : isCompleted
                        ? 'text-neutral-400 dark:text-neutral-500'
                        : 'text-neutral-700 dark:text-[#c9d1d9]'
                    }`}
                  >
                    {isCompleted ? (
                      <CheckCircle2 className="w-3.5 h-3.5 text-[#15803d] dark:text-[#4ade80] shrink-0 mt-0.5" />
                    ) : isInProgress ? (
                      <div className="w-3.5 h-3.5 flex items-center justify-center shrink-0 mt-0.5">
                        <span className="w-2 h-2 rounded-full bg-amber-500 animate-pulse" />
                      </div>
                    ) : (
                      <Circle className="w-3.5 h-3.5 text-neutral-300 dark:text-neutral-600 shrink-0 mt-0.5" />
                    )}
                    <div className="flex-1 min-w-0 break-words leading-tight">
                      <span className="font-mono text-[10px] text-neutral-400 dark:text-neutral-500 mr-1.5 select-none">
                        #{idx + 1}
                      </span>
                      <span className={isCompleted ? 'line-through' : isInProgress ? 'font-medium' : ''}>
                        {item.content || (language === 'ru' ? `Шаг ${idx + 1}` : `Step ${idx + 1}`)}
                      </span>
                    </div>
                  </div>
                );
              })}
            </div>

            {/* Footer hint */}
            <div
              onClick={() => {
                if (taskId) {
                  openDrawer(taskId);
                  setIsHovered(false);
                }
              }}
              className="text-[10px] text-neutral-400 dark:text-neutral-500 font-mono text-center pt-2 mt-2 border-t border-neutral-100 dark:border-[#21262d] cursor-pointer hover:text-neutral-600 dark:hover:text-neutral-300 transition-colors"
            >
              {t('card.clickToOpenTask')}
            </div>
          </div>,
          document.body
        )}
    </>
  );
});

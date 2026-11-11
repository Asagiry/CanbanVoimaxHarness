import { useState, useRef, useEffect, useLayoutEffect, memo } from 'react';
import { createPortal } from 'react-dom';
import { CheckCircle2, ListTodo } from 'lucide-react';
import { TodoItem } from '../../types';
import { useTranslation } from '../../stores/languageStore';
import { useDrawerStore } from '../../stores/drawerStore';
import { CardTodoPopover } from './CardTodoPopover';

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
  const { t } = useTranslation();
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
      if (e.ctrlKey || e.metaKey) return;
      if (!listRef.current) return;
      if (e.deltaY !== 0) {
        e.preventDefault();
        e.stopPropagation();
        listRef.current.scrollTop += e.deltaY;
      }
    };

    const handlePopoverWheel = (e: WheelEvent) => {
      if (e.ctrlKey || e.metaKey) return;
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
  const completedCount = todos.filter((item) => item.status === 'completed').length;
  const inProgressCount = todos.filter((item) => item.status === 'in_progress').length;
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
      <div className="mb-2 flex items-center gap-2 select-none">
        <div
          ref={triggerRef}
          onMouseEnter={handleMouseEnter}
          onMouseLeave={handleMouseLeave}
          className="flex items-center gap-1 font-mono text-[10px] tabular-nums font-medium text-[#666666] dark:text-[#71717a] shrink-0 cursor-pointer rounded px-1 -mx-1 py-0.5 -my-0.5 transition-colors hover:bg-neutral-100/80 dark:hover:bg-[#232328]/80 hover:text-[#171717] dark:hover:text-[#f4f4f5]"
          aria-label={t('card.todosProgress', { completed: completedCount, total: totalCount, percent })}
        >
          {isAllCompleted ? (
            <CheckCircle2 className="h-3 w-3 block shrink-0 text-[#15803d] dark:text-[#4ade80]" />
          ) : (
            <ListTodo className="h-3 w-3 block shrink-0 text-[#8f8f8f] dark:text-[#71717a]" />
          )}
          <span className="inline-flex items-center gap-1 leading-none">
            <span className={completedCount > 0 ? 'text-[#15803d] dark:text-[#4ade80] font-semibold' : 'text-[#171717] dark:text-[#f4f4f5]'}>
              {completedCount}
            </span>
            <span className="text-[#a1a1aa] dark:text-[#52525b]">/</span>
            <span>{totalCount}</span>
          </span>
          {inProgressCount > 0 && !isAllCompleted && (
            <span className="inline-block h-1.5 w-1.5 rounded-full bg-amber-500 animate-pulse ml-0.5" />
          )}
        </div>

        <div className="flex-1 h-1 rounded-full bg-[#f4f4f5] dark:bg-[#212126] overflow-hidden border border-[#ebebeb]/60 dark:border-[#2c2c33]/60">
          <div
            className="h-full rounded-full bg-[#15803d] dark:bg-[#4ade80] transition-all duration-300"
            style={{ width: `${percent}%` }}
          />
        </div>

        <span className="font-mono text-[9px] tabular-nums text-[#8f8f8f] dark:text-[#71717a] shrink-0 leading-none">
          {percent}%
        </span>
      </div>

      {isHovered &&
        createPortal(
          <CardTodoPopover
            popoverRef={popoverRef}
            listRef={listRef}
            coords={coords}
            todos={todos}
            stageTitle={stageTitle}
            stageColor={stageColor}
            completedCount={completedCount}
            inProgressCount={inProgressCount}
            pendingCount={pendingCount}
            totalCount={totalCount}
            percent={percent}
            taskId={taskId}
            onMouseEnter={handleMouseEnter}
            onMouseLeave={handleMouseLeave}
            onOpenTask={(id) => {
              openDrawer(id);
              setIsHovered(false);
            }}
          />,
          document.body
        )}
    </>
  );
});

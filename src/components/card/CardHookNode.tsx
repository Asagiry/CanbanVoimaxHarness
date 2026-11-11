import { useState, useRef, useEffect, useLayoutEffect, memo } from 'react';
import { createPortal } from 'react-dom';
import { Check, X, Loader2 } from 'lucide-react';
import { HookExecutionState } from '../../types';
import { CardHookPopover } from './CardHookPopover';

export interface CardHookNodeProps {
  side: 'left' | 'right';
  hookType: 'on_enter' | 'on_leave';
  hookName: string;
  description?: string;
  state?: HookExecutionState;
  stageTitle?: string;
  sourceStageTitle?: string;
  isTransition?: boolean;
  isEnterRunningTarget?: boolean;
  taskId: string;
  isDragging?: boolean;
}

export const CardHookNode = memo(function CardHookNode({
  side,
  hookType,
  hookName,
  description,
  state,
  stageTitle,
  sourceStageTitle,
  isTransition,
  isEnterRunningTarget,
  isDragging,
}: CardHookNodeProps) {
  const [isHovered, setIsHovered] = useState(false);
  const [coords, setCoords] = useState<{ top: number; left: number } | null>(null);
  const [now, setNow] = useState(Date.now());
  const timerRef = useRef<number | null>(null);
  const triggerRef = useRef<HTMLDivElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const hideTimerRef = useRef<number | null>(null);

  const status = state?.status ?? 'idle';
  const hookDescription = description || state?.description;

  useEffect(() => {
    if (status === 'running') {
      timerRef.current = window.setInterval(() => {
        setNow(Date.now());
      }, 250);
    } else if (timerRef.current) {
      clearInterval(timerRef.current);
      timerRef.current = null;
    }
    return () => {
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [status]);

  useEffect(() => {
    return () => {
      if (hideTimerRef.current) {
        clearTimeout(hideTimerRef.current);
        hideTimerRef.current = null;
      }
    };
  }, []);

  const elapsedSec = state?.startedAt
    ? Math.max(0, ((status === 'running' ? now : (state.startedAt + (state.durationMs ?? 0))) - state.startedAt) / 1000)
    : 0;

  const stdout = state?.stdout?.trim() ?? '';
  const stderr = state?.stderr?.trim() ?? '';
  const errorText = state?.error?.trim() || stderr;
  const hasOutput = stdout.length > 0 || stderr.length > 0;

  const updatePosition = () => {
    if (!triggerRef.current) return;
    const rect = triggerRef.current.getBoundingClientRect();
    const popoverWidth = 420;
    const popoverHeight = popoverRef.current?.offsetHeight ?? 260;

    let left = side === 'left' ? rect.left : rect.right - popoverWidth;
    left = Math.max(8, Math.min(left, window.innerWidth - popoverWidth - 8));

    let top = rect.bottom + 6;
    if (top + popoverHeight > window.innerHeight - 8) {
      const topAbove = rect.top - popoverHeight - 6;
      if (topAbove >= 8) {
        top = topAbove;
      } else {
        top = Math.max(8, window.innerHeight - popoverHeight - 8);
      }
    }

    setCoords({ top, left });
  };

  useLayoutEffect(() => {
    if (isHovered) {
      updatePosition();
    }
  }, [isHovered, stdout, stderr, errorText]);

  useEffect(() => {
    if (isDragging && isHovered) {
      if (hideTimerRef.current) {
        clearTimeout(hideTimerRef.current);
        hideTimerRef.current = null;
      }
      setIsHovered(false);
    }
  }, [isDragging, isHovered]);

  useEffect(() => {
    if (!isHovered) return;

    const handleClose = () => {
      if (hideTimerRef.current) {
        clearTimeout(hideTimerRef.current);
        hideTimerRef.current = null;
      }
      setIsHovered(false);
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

    window.addEventListener('scroll', handleScroll, { capture: true, passive: true });
    window.addEventListener('resize', handleClose);
    window.addEventListener('pointerdown', handlePointerDown);
    window.addEventListener('keydown', handleKeyDown);

    return () => {
      window.removeEventListener('scroll', handleScroll, { capture: true });
      window.removeEventListener('resize', handleClose);
      window.removeEventListener('pointerdown', handlePointerDown);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [isHovered]);

  const handleMouseEnter = () => {
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
      hideTimerRef.current = null;
    }
    updatePosition();
    setIsHovered(true);
  };

  const handleMouseLeave = () => {
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
    }
    hideTimerRef.current = window.setTimeout(() => {
      setIsHovered(false);
      hideTimerRef.current = null;
    }, 180);
  };

  const handlePopoverMouseEnter = () => {
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
      hideTimerRef.current = null;
    }
  };

  const handlePopoverMouseLeave = () => {
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
    }
    hideTimerRef.current = window.setTimeout(() => {
      setIsHovered(false);
      hideTimerRef.current = null;
    }, 180);
  };

  const nodeColorClass = (() => {
    switch (status) {
      case 'running':
        return 'bg-amber-500 text-black border-2 border-white dark:border-[#111114] animate-pulse shadow-md';
      case 'success':
        return 'bg-emerald-500 text-white border-2 border-white dark:border-[#111114] shadow-md';
      case 'failed':
        return 'bg-rose-500 text-white border-2 border-white dark:border-[#111114] shadow-md';
      case 'idle':
      default:
        return 'bg-white dark:bg-[#17171b] border border-neutral-300 dark:border-neutral-600 hover:border-amber-400 dark:hover:border-amber-400 shadow-xs';
    }
  })();

  const sidePosClass = side === 'left' ? '-left-2' : '-right-2';

  return (
    <>
      <div
        className={`absolute top-1/2 -translate-y-1/2 ${sidePosClass} z-30 flex items-center justify-center`}
        onClick={(e) => e.stopPropagation()}
      >
        <div
          ref={triggerRef}
          className={`w-4 h-4 rounded-full flex items-center justify-center cursor-pointer transition-transform hover:scale-110 ${nodeColorClass}`}
          aria-label={`${hookType}: ${hookName}`}
          onMouseEnter={handleMouseEnter}
          onMouseLeave={handleMouseLeave}
        >
          {status === 'running' && <Loader2 className="w-2.5 h-2.5 block shrink-0 animate-spin" />}
          {status === 'success' && <Check className="w-2.5 h-2.5 block shrink-0 stroke-[3]" />}
          {status === 'failed' && <X className="w-2.5 h-2.5 block shrink-0 stroke-[3]" />}
          {status === 'idle' && (
            <span className="w-1.5 h-1.5 rounded-full bg-neutral-400 dark:bg-neutral-500" />
          )}
        </div>
      </div>

      {isHovered && coords && typeof document !== 'undefined' && createPortal(
        <CardHookPopover
          popoverRef={popoverRef}
          coords={coords}
          hookType={hookType}
          hookName={hookName}
          hookDescription={hookDescription}
          state={state}
          status={status}
          elapsedSec={elapsedSec}
          stageTitle={stageTitle}
          sourceStageTitle={sourceStageTitle}
          isTransition={isTransition}
          isEnterRunningTarget={isEnterRunningTarget}
          stdout={stdout}
          stderr={stderr}
          errorText={errorText}
          hasOutput={hasOutput}
          onMouseEnter={handlePopoverMouseEnter}
          onMouseLeave={handlePopoverMouseLeave}
        />,
        document.body
      )}
    </>
  );
});

export default CardHookNode;

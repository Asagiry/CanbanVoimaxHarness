import { useState, useRef, useEffect, useLayoutEffect, memo } from 'react';
import { createPortal } from 'react-dom';
import { Check, X, Loader2, Terminal } from 'lucide-react';
import {
  KanbanTask,
  PipelineColumn,
  HookExecutionState,
  getHookScript,
  getHookDescription,
} from '../../types';
import { useBoardStore } from '../../stores/boardStore';
import { useShallow } from 'zustand/react/shallow';

export interface CardHookNodeProps {
  side: 'left' | 'right';
  hookType: 'on_enter' | 'on_leave';
  hookName: string;
  description?: string;
  state?: HookExecutionState;
  stageTitle?: string;
  sourceStageTitle?: string;
  isTransition?: boolean;
  taskId: string;
  isDragging?: boolean;
}

export function useCardHooks(
  task: KanbanTask,
  taskColumn: PipelineColumn | undefined,
  columns: PipelineColumn[]
) {
  const storeHooks = useBoardStore(useShallow((s) => s.hookStates[task.id]));
  const parentHooks = useBoardStore(useShallow((s) => (task.parentId ? s.hookStates[task.parentId] : undefined)));
  const taskHooks = storeHooks || task.hookStates || parentHooks;

  const enterHookDef = taskColumn?.hooks?.['on_enter'];
  const enterHookName = getHookScript(enterHookDef);
  const enterHookDescription = getHookDescription(enterHookDef);
  const rawEnterHookState = taskHooks?.on_enter;
  const enterHookState =
    rawEnterHookState && (!rawEnterHookState.stageId || rawEnterHookState.stageId === task.columnId)
      ? rawEnterHookState
      : undefined;

  const leaveHookDef =
    taskColumn?.hooks?.['on_leave'] ||
    taskColumn?.hooks?.['on_exit'] ||
    taskColumn?.hooks?.['on_finish'];
  const leaveHookName = getHookScript(leaveHookDef);
  const leaveHookDescription = getHookDescription(leaveHookDef);
  const leaveHookState = taskHooks?.on_leave;

  // Check predecessor column in board order
  const currentColIndex = columns.findIndex((c) => c.id === task.columnId);
  const prevCol = currentColIndex > 0 ? columns[currentColIndex - 1] : null;

  const stageEntryTime = task.stageEnteredAt || task.createdAt || 0;
  const hookDuration = leaveHookState?.durationMs || 0;
  const isRecentHook =
    Boolean(leaveHookState?.startedAt) &&
    stageEntryTime > 0 &&
    (leaveHookState!.startedAt >= stageEntryTime - hookDuration - 10000 ||
      Math.abs(stageEntryTime - leaveHookState!.startedAt) <= hookDuration + 10000);

  const matchesPreviousStage = task.previousStageId
    ? task.previousStageId === leaveHookState?.stageId
    : Boolean(prevCol && prevCol.id === leaveHookState?.stageId);

  const isTransitionHookFromResult = Boolean(
    !enterHookState &&
    leaveHookState &&
    leaveHookState.stageId !== task.columnId &&
    matchesPreviousStage &&
    (isRecentHook || !stageEntryTime)
  );

  const effectiveLeftState = enterHookState || (isTransitionHookFromResult ? leaveHookState : undefined);
  const effectiveLeftHookName =
    enterHookName ||
    effectiveLeftState?.hookName ||
    (isTransitionHookFromResult ? leaveHookState?.hookName : undefined);
  const hasLeftHook = Boolean(enterHookName || effectiveLeftState);

  const sourceStageCol = effectiveLeftState?.stageId
    ? columns.find((c) => c.id === effectiveLeftState.stageId)
    : undefined;
  const sourceStageTitle = sourceStageCol?.title || effectiveLeftState?.stageId;
  const sourceLeaveDef =
    sourceStageCol?.hooks?.['on_leave'] ||
    sourceStageCol?.hooks?.['on_exit'] ||
    sourceStageCol?.hooks?.['on_finish'];
  const transitionHookDescription = getHookDescription(sourceLeaveDef);

  // Right node: exit hook from this column
  const hasRightHook = Boolean(
    leaveHookName || (leaveHookState && leaveHookState.stageId === task.columnId)
  );
  const effectiveRightState =
    leaveHookState && leaveHookState.stageId === task.columnId ? leaveHookState : undefined;
  const effectiveRightHookName = leaveHookName || effectiveRightState?.hookName;

  return {
    hasLeftHook,
    effectiveLeftHookName,
    effectiveLeftState,
    enterHookDescription,
    transitionHookDescription,
    isTransitionHookFromResult,
    sourceStageTitle,
    hasRightHook,
    effectiveRightHookName,
    effectiveRightState,
    leaveHookDescription,
  };
}

export const CardHookNode = memo(function CardHookNode({
  side,
  hookType: _hookType,
  hookName,
  description,
  state,
  stageTitle,
  sourceStageTitle,
  isTransition,
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

  // Close immediately when card is being dragged
  useEffect(() => {
    if (isDragging && isHovered) {
      if (hideTimerRef.current) {
        clearTimeout(hideTimerRef.current);
        hideTimerRef.current = null;
      }
      setIsHovered(false);
    }
  }, [isDragging, isHovered]);

  // Close on window/column scroll, resize, or pointerdown outside
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
        return 'bg-amber-500 text-black border-2 border-white dark:border-[#0d1117] animate-pulse shadow-md';
      case 'success':
        return 'bg-emerald-500 text-white border-2 border-white dark:border-[#0d1117] shadow-md';
      case 'failed':
        return 'bg-rose-500 text-white border-2 border-white dark:border-[#0d1117] shadow-md';
      case 'idle':
      default:
        return 'bg-white dark:bg-[#161b22] border border-neutral-300 dark:border-neutral-600 hover:border-amber-400 dark:hover:border-amber-400 shadow-xs';
    }
  })();

  const sidePosClass = side === 'left' ? '-left-2' : '-right-2';

  return (
    <>
      <div
        className={`absolute top-1/2 -translate-y-1/2 ${sidePosClass} z-30 flex items-center justify-center`}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Circle Node Trigger */}
        <div
          ref={triggerRef}
          className={`w-4 h-4 rounded-full flex items-center justify-center cursor-pointer transition-transform hover:scale-110 ${nodeColorClass}`}
          aria-label={`${side === 'left' ? 'on_enter' : 'on_leave'}: ${hookName}`}
          onMouseEnter={handleMouseEnter}
          onMouseLeave={handleMouseLeave}
        >
          {status === 'running' && <Loader2 className="w-2.5 h-2.5 animate-spin" />}
          {status === 'success' && <Check className="w-2.5 h-2.5 stroke-[3]" />}
          {status === 'failed' && <X className="w-2.5 h-2.5 stroke-[3]" />}
          {status === 'idle' && (
            <span className="w-1.5 h-1.5 rounded-full bg-neutral-400 dark:bg-neutral-500" />
          )}
        </div>
      </div>

      {/* Hover Popover via Portal */}
      {isHovered && coords && typeof document !== 'undefined' && createPortal(
        <div
          ref={popoverRef}
          className="fixed z-[9999] w-[420px] max-w-[calc(100vw-24px)] shadow-2xl rounded-lg border border-neutral-200 dark:border-[#30363d] bg-white dark:bg-[#161b22] p-3.5 text-left cursor-default select-text pointer-events-auto overscroll-contain"
          style={{
            top: `${coords.top}px`,
            left: `${coords.left}px`,
          }}
          onMouseEnter={handlePopoverMouseEnter}
          onMouseLeave={handlePopoverMouseLeave}
          onClick={(e) => e.stopPropagation()}
        >
          {/* Header */}
          <div className="flex items-center justify-between gap-1.5 pb-2 border-b border-neutral-100 dark:border-[#21262d]">
            <span className="text-xs font-mono font-medium text-[#71717a] dark:text-[#8b949e]">
              {isTransition
                ? 'Хук перехода (on_leave)'
                : side === 'left'
                ? 'Входной хук (on_enter)'
                : 'Выходной хук (on_leave)'}
            </span>
            <div className="flex items-center gap-1">
              {status === 'running' && (
                <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-mono tabular-nums text-amber-700 dark:text-amber-300 bg-amber-50 dark:bg-amber-950/40 border border-amber-200 dark:border-amber-800">
                  <Loader2 className="w-3 h-3 animate-spin text-amber-600 dark:text-amber-400" />
                  <span>{elapsedSec.toFixed(1)}s</span>
                </span>
              )}
              {status === 'success' && (
                <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-mono tabular-nums text-emerald-700 dark:text-emerald-300 bg-emerald-50 dark:bg-emerald-950/40 border border-emerald-200 dark:border-emerald-800">
                  <Check className="w-3 h-3 stroke-[2.5]" />
                  <span>Успех {state?.durationMs ? `${(state.durationMs / 1000).toFixed(2)}s` : ''}</span>
                </span>
              )}
              {status === 'failed' && (
                <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-mono tabular-nums text-rose-700 dark:text-rose-300 bg-rose-50 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-800">
                  <X className="w-3 h-3 stroke-[2.5]" />
                  <span>Ошибка {state?.exitCode != null ? `(код ${state.exitCode})` : ''}</span>
                </span>
              )}
              {status === 'idle' && (
                <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-mono text-neutral-600 dark:text-neutral-400 bg-neutral-100 dark:bg-[#0d1117] border border-neutral-200 dark:border-[#30363d]">
                  Готов к запуску
                </span>
              )}
            </div>
          </div>

          {/* Hook name */}
          <div className="mt-2 text-[13px] font-mono font-semibold text-neutral-900 dark:text-[#f0f6fc] truncate" aria-label={hookName} title={hookName}>
            {hookName}
          </div>

          {isTransition && sourceStageTitle ? (
            <div className="text-xs text-[#71717a] dark:text-[#8b949e] font-mono mt-0.5">
              Переход из: {sourceStageTitle}
            </div>
          ) : stageTitle ? (
            <div className="text-xs text-[#71717a] dark:text-[#8b949e] font-mono mt-0.5">
              Стадия: {stageTitle}
            </div>
          ) : null}

          {/* Hook description */}
          {hookDescription && (
            <div className="mt-2 text-xs text-neutral-600 dark:text-neutral-300 font-sans leading-relaxed">
              {hookDescription}
            </div>
          )}

          {/* Error description if failed */}
          {status === 'failed' && errorText.length > 0 && (
            <div className="mt-2 p-2.5 rounded bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900 text-xs font-mono text-rose-800 dark:text-rose-300 whitespace-pre-wrap break-words max-h-44 overflow-y-auto leading-relaxed overscroll-contain">
              <div className="font-semibold text-rose-900 dark:text-rose-200 mb-1 flex items-center gap-1">
                <span>⛔ Отклонено хуком:</span>
              </div>
              {errorText}
            </div>
          )}

          {/* Terminal log output */}
          {hasOutput ? (
            <div className="mt-2.5">
              <div className="flex items-center justify-between text-xs font-mono text-[#71717a] dark:text-[#8b949e] mb-1">
                <span className="flex items-center gap-1">
                  <Terminal className="w-3.5 h-3.5" />
                  <span>Лог терминала</span>
                </span>
                <span className="text-[10px]">
                  {stdout.length > 0 ? 'stdout' : ''} {stderr.length > 0 ? 'stderr' : ''}
                </span>
              </div>
              <div className="p-2.5 rounded bg-[#0d1117] border border-[#30363d] font-mono text-xs text-[#c9d1d9] leading-relaxed max-h-56 overflow-y-auto whitespace-pre-wrap break-words [word-break:break-word] select-text overscroll-contain">
                {stdout || stderr}
              </div>
            </div>
          ) : (
            (!hookDescription || status === 'running') && (
              <div className="mt-2 text-xs text-[#71717a] dark:text-[#8b949e] font-sans leading-relaxed">
                {status === 'running'
                  ? 'Выполняется скрипт жизненного цикла стадии...'
                  : isTransition
                  ? 'Скрипт перехода, выполненный при выходе из предыдущей стадии.'
                  : side === 'left'
                  ? 'Скрипт выполнится автоматически при входе в эту стадию.'
                  : 'Скрипт выполнится автоматически при выходе из этой стадии.'}
              </div>
            )
          )}
        </div>,
        document.body
      )}
    </>
  );
});

export default CardHookNode;

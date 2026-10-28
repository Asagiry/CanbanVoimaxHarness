import { useState, useRef, useEffect, useLayoutEffect, memo } from 'react';
import { createPortal } from 'react-dom';
import { Draggable } from '@hello-pangea/dnd';
import { KanbanTask, TaskStatus, HookExecutionState, TodoItem } from '../../types';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore, isDoneColumnDef } from '../../stores/boardStore';
import { useTranslation, Language } from '../../stores/languageStore';
import { useShallow } from 'zustand/react/shallow';
import { GitBranch, CornerUpLeft, ChevronDown, MessageSquare, GitFork, Check, X, Loader2, Terminal, Paperclip } from 'lucide-react';
import TaskContextMenu from './TaskContextMenu';
import { CardTodoProgress } from './CardTodoProgress';
import { getDeterministicAgentColor } from '../../lib/agentPalette';
import { useUiZoomStore, adjustDndTransformForZoom } from '../../hooks/useUiZoom';

function formatCardDate(ts?: number): string {
  if (!ts || !Number.isFinite(ts)) return '';
  const d = new Date(ts);
  const dd = String(d.getDate()).padStart(2, '0');
  const mm = String(d.getMonth() + 1).padStart(2, '0');
  const hh = String(d.getHours()).padStart(2, '0');
  const min = String(d.getMinutes()).padStart(2, '0');
  return `${dd}.${mm} ${hh}:${min}`;
}

function formatStageDuration(totalSeconds: number, lang: Language = 'en'): string {
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

function formatTimerMMSS(totalSeconds: number): string {
  const s = Math.max(0, Math.floor(totalSeconds));
  const hrs = Math.floor(s / 3600);
  const mins = Math.floor((s % 3600) / 60);
  const secs = s % 60;
  if (hrs > 0) {
    return `${hrs}:${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
  }
  return `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
}

interface CardHookNodeProps {
  side: 'left' | 'right';
  hookType: 'on_enter' | 'on_leave';
  hookName: string;
  state?: HookExecutionState;
  stageTitle?: string;
  taskId: string;
  isDragging?: boolean;
}

function CardHookNode({ side, hookType: _hookType, hookName, state, stageTitle, isDragging }: CardHookNodeProps) {
  const [isHovered, setIsHovered] = useState(false);
  const [coords, setCoords] = useState<{ top: number; left: number } | null>(null);
  const [now, setNow] = useState(Date.now());
  const timerRef = useRef<number | null>(null);
  const triggerRef = useRef<HTMLDivElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const hideTimerRef = useRef<number | null>(null);

  const status = state?.status ?? 'idle';

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

    const handlePointerDown = (e: PointerEvent) => {
      if (popoverRef.current?.contains(e.target as Node)) {
        return;
      }
      if (triggerRef.current?.contains(e.target as Node)) {
        return;
      }
      handleClose();
    };

    window.addEventListener('scroll', handleClose, { capture: true, passive: true });
    window.addEventListener('resize', handleClose);
    window.addEventListener('pointerdown', handlePointerDown);

    return () => {
      window.removeEventListener('scroll', handleClose, { capture: true });
      window.removeEventListener('resize', handleClose);
      window.removeEventListener('pointerdown', handlePointerDown);
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
          className="fixed z-[9999] w-[420px] max-w-[calc(100vw-24px)] shadow-2xl rounded-lg border border-neutral-200 dark:border-[#30363d] bg-white dark:bg-[#161b22] p-3.5 text-left cursor-default select-text pointer-events-auto"
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
            <span className="text-xs font-mono font-medium text-neutral-500 dark:text-neutral-400">
              {side === 'left' ? 'Входной хук (on_enter)' : 'Выходной хук (on_leave)'}
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

          {stageTitle && (
            <div className="text-xs text-neutral-500 dark:text-neutral-400 font-mono mt-0.5">
              Стадия: {stageTitle}
            </div>
          )}

          {/* Error description if failed */}
          {status === 'failed' && errorText.length > 0 && (
            <div className="mt-2 p-2.5 rounded bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900 text-xs font-mono text-rose-800 dark:text-rose-300 whitespace-pre-wrap break-words max-h-44 overflow-y-auto leading-relaxed">
              <div className="font-semibold text-rose-900 dark:text-rose-200 mb-1 flex items-center gap-1">
                <span>⛔ Отклонено хуком:</span>
              </div>
              {errorText}
            </div>
          )}

          {/* Terminal log output */}
          {hasOutput ? (
            <div className="mt-2.5">
              <div className="flex items-center justify-between text-xs font-mono text-neutral-500 dark:text-neutral-400 mb-1">
                <span className="flex items-center gap-1">
                  <Terminal className="w-3.5 h-3.5" />
                  <span>Лог терминала</span>
                </span>
                <span className="text-[10px]">
                  {stdout.length > 0 ? 'stdout' : ''} {stderr.length > 0 ? 'stderr' : ''}
                </span>
              </div>
              <div className="p-2.5 rounded bg-[#0d1117] border border-[#30363d] font-mono text-xs text-[#c9d1d9] leading-relaxed max-h-56 overflow-y-auto whitespace-pre-wrap break-words [word-break:break-word] select-text">
                {stdout || stderr}
              </div>
            </div>
          ) : (
            <div className="mt-2 text-xs text-neutral-500 dark:text-neutral-400 font-sans leading-relaxed">
              {status === 'running'
                ? 'Выполняется скрипт жизненного цикла стадии...'
                : side === 'left'
                ? 'Скрипт выполнится автоматически при входе в эту стадию.'
                : 'Скрипт выполнится автоматически при выходе из этой стадии.'}
            </div>
          )}
        </div>,
        document.body
      )}
    </>
  );
}

interface KanbanCardProps {
  task: KanbanTask;
  index: number;
}

function KanbanCard({ task, index }: KanbanCardProps) {
  const zoom = useUiZoomStore((s) => s.zoom);
  const openDrawer = useDrawerStore((s) => s.openDrawer);
  const setHoveredTaskId = useBoardStore((s) => s.setHoveredTaskId);
  const columns = useBoardStore((s) => s.columns);
  const { language, t } = useTranslation();
  const isHighlighted = useBoardStore((s) => {
    const hId = s.hoveredTaskId;
    if (!hId) return false;
    if (hId === task.id) return true;
    if (task.parentId && hId === task.parentId) return true;
    if (task.waitingForSubtaskId && hId === task.waitingForSubtaskId) return true;
    const hoveredTask = s.tasks.find((t) => t.id === hId);
    if (!hoveredTask) return false;
    // Parent of hovered task
    if (hoveredTask.parentId === task.id) return true;
    // Sibling subtask sharing the same parent
    if (task.parentId && hoveredTask.parentId && task.parentId === hoveredTask.parentId) return true;
    // Subtask that hovered task is currently waiting for
    if (hoveredTask.waitingForSubtaskId === task.id) return true;
    return false;
  });

  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);
  const [isSubtasksOpen, setIsSubtasksOpen] = useState(false);

  const taskColumn = columns.find((c) => c.id === task.columnId);
  const assignedAgentColor = taskColumn?.persona
    ? getDeterministicAgentColor(taskColumn.persona)
    : (taskColumn?.color || getDeterministicAgentColor(task.columnId));
  const isDone = Boolean(task.status === 'done' || isDoneColumnDef(taskColumn));

  // Subtasks belonging to this task (queried only if card is in done column)
  const subtasks = useBoardStore(
    useShallow((s) => (isDone ? s.tasks.filter((t) => t.parentId === task.id) : []))
  );

  // Hook nodes data
  const storeHooks = useBoardStore(useShallow((s) => s.hookStates[task.id]));
  const parentHooks = useBoardStore(useShallow((s) => (task.parentId ? s.hookStates[task.parentId] : undefined)));
  const taskHooks = storeHooks || task.hookStates || parentHooks;

  const enterHookName = taskColumn?.hooks?.['on_enter'];
  const rawEnterHookState = taskHooks?.on_enter;
  const enterHookState =
    rawEnterHookState && (!rawEnterHookState.stageId || rawEnterHookState.stageId === task.columnId)
      ? rawEnterHookState
      : undefined;

  const leaveHookName =
    taskColumn?.hooks?.['on_leave'] ||
    taskColumn?.hooks?.['on_exit'] ||
    taskColumn?.hooks?.['on_finish'];
  const leaveHookState = taskHooks?.on_leave;

  // If this task recently transitioned into this stage via an on_leave hook from previous column,
  // show that transition result on the left (entry) node if this column has no separate on_enter hook.
  const isTransitionHookFromResult =
    !enterHookState &&
    leaveHookState &&
    leaveHookState.stageId !== task.columnId;

  const effectiveLeftState = enterHookState || (isTransitionHookFromResult ? leaveHookState : undefined);
  const effectiveLeftHookName =
    enterHookName ||
    effectiveLeftState?.hookName ||
    (isTransitionHookFromResult ? leaveHookState?.hookName : undefined);
  const hasLeftHook = Boolean(enterHookName || effectiveLeftState);

  // Right node: exit hook from this column
  const hasRightHook = Boolean(
    leaveHookName || (leaveHookState && leaveHookState.stageId === task.columnId)
  );
  const effectiveRightState =
    leaveHookState && leaveHookState.stageId === task.columnId ? leaveHookState : undefined;
  const effectiveRightHookName = leaveHookName || effectiveRightState?.hookName;

  const isRunning = task.status === 'running';
  const stageStart = task.stageEnteredAt;
  const currentStageDuration = isRunning && stageStart
    ? Math.max(0, Math.floor((Date.now() - stageStart) / 1000))
    : (task.activeStageDuration ?? 0);

  const currentTotalElapsed = isRunning && stageStart
    ? (task.totalElapsed ?? 0) + Math.max(0, Math.floor((Date.now() - stageStart) / 1000))
    : (task.totalElapsed ?? 0);

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY });
  };

  const getStatusBadge = (status: TaskStatus) => {
    const waitingSub = task.waitingForSubtaskId
      ? useBoardStore.getState().tasks.find((t) => t.id === task.waitingForSubtaskId)
      : null;
    const waitingSubCol = waitingSub ? columns.find((c) => c.id === waitingSub.columnId) : null;
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
  };

  return (
    <>
      <Draggable draggableId={task.id} index={index}>
        {(provided, snapshot) => {
          const baseTransform = adjustDndTransformForZoom(provided.draggableProps.style?.transform, zoom);
          return (
            <div
              id={`task-card-${task.id}`}
              data-task-id={task.id}
              ref={provided.innerRef}
              {...provided.draggableProps}
              {...provided.dragHandleProps}
              onClick={() => openDrawer(task.id)}
              onContextMenu={handleContextMenu}
              onMouseEnter={() => setHoveredTaskId(task.id)}
              onMouseLeave={() => setHoveredTaskId(null)}
              style={{
                ...provided.draggableProps.style,
                transform: snapshot.isDragging && baseTransform
                  ? `${baseTransform} rotate(1.8deg) scale(1.025)`
                  : baseTransform,
              transition: provided.draggableProps.style?.transition
                ? `${provided.draggableProps.style.transition}, transform 0.08s ease-out, box-shadow 0.15s ease-out`
                : 'transform 0.08s ease-out, box-shadow 0.15s ease-out',
              borderColor: snapshot.isDragging
                ? undefined
                : isHighlighted
                ? undefined
                : `${assignedAgentColor}50`,
            }}
            className={`group relative mb-2 rounded-md border p-3 select-none ${
              snapshot.isDragging
                ? 'scale-[1.02] rotate-[1.5deg] shadow-2xl ring-1 ring-black/10 dark:ring-white/20 cursor-grabbing z-50 border-[#171717] dark:border-[#30363d] bg-white dark:bg-[#161b22]'
                : `transition-transform duration-100 ease-out cursor-pointer active:scale-[0.99] ${
                    isHighlighted
                      ? 'border-[#8b5cf6] shadow-md ring-2 ring-[#8b5cf6]/50 bg-purple-50/20 dark:bg-[#1c1b29]'
                      : 'border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#161b22] hover:border-[#d4d4d8] dark:hover:border-[#30363d] shadow-xs'
                  }`
            }`}
          >
            {/* Header: ID, Parent link & Concise Status */}
            <div className="flex items-center justify-between mb-2 gap-1.5 flex-wrap">
              <div className="flex items-center space-x-1.5">
                <span className="font-mono text-[10px] font-semibold text-[#171717] dark:text-[#f0f6fc] bg-[#f4f4f5] dark:bg-[#0d1117] px-1.5 py-0.5 rounded border border-[#ebebeb] dark:border-[#30363d] tabular-nums">
                  {task.id}
                </span>

                {/* Subtask link to parent */}
                {task.parentId && (
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      openDrawer(task.parentId!);
                    }}
                    className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#6366f1] dark:text-[#a5b4fc] bg-[#eef2ff] dark:bg-[#312e81]/40 border border-[#c7d2fe] dark:border-[#4338ca] hover:bg-[#e0e7ff] dark:hover:bg-[#3730a3]/50 transition cursor-pointer"
                    aria-label={t('card.parentTask', { id: task.parentId })}
                  >
                    <CornerUpLeft className="h-2.5 w-2.5 text-[#6366f1] dark:text-[#a5b4fc]" />
                    <span>{task.parentId}</span>
                  </button>
                )}
              </div>
              {getStatusBadge(task.status)}
            </div>

            {/* Title */}
            <h4 className="text-xs font-medium text-[#171717] dark:text-[#f0f6fc] leading-snug mb-2 font-sans break-words">
              {task.title}
            </h4>

            {/* Todo checklist progress strip */}
            <CardTodoProgress
              todos={task.todos}
              stageTitle={taskColumn?.title || task.columnId}
              stageColor={assignedAgentColor}
              isDragging={snapshot.isDragging}
              taskId={task.id}
            />

            {/* Footer: model, branch, created & elapsed time */}
            <div className="pt-2 border-t border-[#ebebeb] dark:border-[#21262d]">
              {task.model && task.model.trim().length > 0 && (
                <div
                  className="truncate font-mono text-[10px] text-[#666666] dark:text-[#8b949e] mb-1"
                  aria-label={t('card.model', { model: task.model })}
                >
                  {task.model}
                </div>
              )}
              {task.branch ? (
                <div className="flex items-center justify-between gap-2 font-mono text-[10px]">
                  <span className="inline-flex items-center gap-1 truncate text-[#8f8f8f] dark:text-[#8b949e] min-w-0">
                    <GitBranch className="h-2.5 w-2.5 shrink-0" />
                    <span className="truncate">{task.branch}</span>
                  </span>
                </div>
              ) : null}
              {(task.createdAt || currentTotalElapsed > 0 || (task.attachments && task.attachments.length > 0)) && (
                <div className="mt-1 flex items-center justify-between gap-1 font-mono text-[10px] text-[#8f8f8f] dark:text-[#8b949e] tabular-nums whitespace-nowrap">
                  <div className="flex items-center gap-1 min-w-0 truncate">
                    {task.createdAt ? (
                      <span>{t('card.created', { date: formatCardDate(task.createdAt) })}</span>
                    ) : null}
                    {task.createdAt && currentTotalElapsed > 0 ? <span>·</span> : null}
                    {currentTotalElapsed > 0 ? (
                      <span>
                        {t('card.elapsed', { time: formatStageDuration(currentTotalElapsed, language) })}
                      </span>
                    ) : null}
                  </div>
                  {task.attachments && task.attachments.length > 0 && (
                    <span
                      className="inline-flex items-center gap-0.5 text-[10px] font-mono text-[#666666] dark:text-[#8b949e] shrink-0"
                      aria-label={t('newTask.attachmentsLabel')}
                    >
                      <Paperclip className="h-2.5 w-2.5 shrink-0" />
                      <span>{task.attachments.length}</span>
                    </span>
                  )}
                </div>
              )}
            </div>

            {/* In Done column: Collapsible list of completed subtasks with dialogue open */}
            {isDone && subtasks.length > 0 && (
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
            )}

            {/* Left Edge Hook Node (on_enter / transition result) */}
            {hasLeftHook && effectiveLeftHookName && (
              <CardHookNode
                side="left"
                hookType="on_enter"
                hookName={effectiveLeftHookName}
                state={effectiveLeftState}
                stageTitle={taskColumn?.title || task.columnId}
                taskId={task.id}
                isDragging={snapshot.isDragging}
              />
            )}

            {/* Right Edge Hook Node (on_leave / exit) */}
            {hasRightHook && effectiveRightHookName && (
              <CardHookNode
                side="right"
                hookType="on_leave"
                hookName={effectiveRightHookName}
                state={effectiveRightState}
                stageTitle={taskColumn?.title || task.columnId}
                taskId={task.id}
                isDragging={snapshot.isDragging}
              />
            )}
          </div>
        );
      }}
      </Draggable>
      {contextMenu && (
        <TaskContextMenu
          task={task}
          x={contextMenu.x}
          y={contextMenu.y}
          onClose={() => setContextMenu(null)}
        />
      )}
    </>
  );
}

function areTodosEqual(a?: TodoItem[], b?: TodoItem[]): boolean {
  if (a === b) return true;
  if (!a || !b) return false;
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    const itemA = a[i];
    const itemB = b[i];
    if (!itemA || !itemB) return false;
    if (
      itemA.id !== itemB.id ||
      itemA.status !== itemB.status ||
      itemA.content !== itemB.content
    ) {
      return false;
    }
  }
  return true;
}

function areCardPropsEqual(prev: KanbanCardProps, next: KanbanCardProps): boolean {
  if (prev.index !== next.index) return false;
  if (prev.task === next.task) return true;

  const a = prev.task;
  const b = next.task;

  return (
    a.id === b.id &&
    a.title === b.title &&
    a.description === b.description &&
    a.columnId === b.columnId &&
    a.boardId === b.boardId &&
    a.status === b.status &&
    a.priority === b.priority &&
    a.branch === b.branch &&
    a.worktreePath === b.worktreePath &&
    a.persona === b.persona &&
    a.verdict === b.verdict &&
    a.verdictNote === b.verdictNote &&
    a.model === b.model &&
    a.parentId === b.parentId &&
    a.subtaskIndex === b.subtaskIndex &&
    a.waitingForSubtaskId === b.waitingForSubtaskId &&
    a.handoverSummary === b.handoverSummary &&
    a.activeStageDuration === b.activeStageDuration &&
    a.totalElapsed === b.totalElapsed &&
    a.stageEnteredAt === b.stageEnteredAt &&
    a.updatedAt === b.updatedAt &&
    a.createdAt === b.createdAt &&
    a.isTimerRunning === b.isTimerRunning &&
    a.returnsCount === b.returnsCount &&
    a.attachments === b.attachments &&
    a.hookStates === b.hookStates &&
    areTodosEqual(a.todos, b.todos)
  );
}

export default memo(KanbanCard, areCardPropsEqual);

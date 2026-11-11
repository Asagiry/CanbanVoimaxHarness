import React from 'react';
import { Ban, Check, X, Loader2, Terminal } from 'lucide-react';
import type { HookExecutionState } from '../../types';

interface CardHookPopoverProps {
  popoverRef: React.RefObject<HTMLDivElement | null>;
  coords: { top: number; left: number };
  hookType: 'on_enter' | 'on_leave';
  hookName: string;
  hookDescription?: string;
  state?: HookExecutionState;
  status: 'idle' | 'running' | 'success' | 'failed';
  elapsedSec: number;
  stageTitle?: string;
  sourceStageTitle?: string;
  isTransition?: boolean;
  isEnterRunningTarget?: boolean;
  stdout: string;
  stderr: string;
  errorText: string;
  hasOutput: boolean;
  onMouseEnter: () => void;
  onMouseLeave: () => void;
}

export function CardHookPopover({
  popoverRef,
  coords,
  hookType,
  hookName,
  hookDescription,
  state,
  status,
  elapsedSec,
  stageTitle,
  sourceStageTitle,
  isTransition,
  isEnterRunningTarget,
  stdout,
  stderr,
  errorText,
  hasOutput,
  onMouseEnter,
  onMouseLeave,
}: CardHookPopoverProps) {
  return (
    <div
      ref={popoverRef}
      className="fixed z-[9999] w-[420px] max-w-[calc(100vw-24px)] shadow-2xl rounded-lg border border-neutral-200 dark:border-[#2c2c33] bg-white dark:bg-[#17171b] p-3.5 text-left cursor-default select-text pointer-events-auto overscroll-contain"
      style={{
        top: `${coords.top}px`,
        left: `${coords.left}px`,
      }}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
      onClick={(e) => e.stopPropagation()}
    >
      <div className="flex items-center justify-between gap-1.5 pb-2 border-b border-neutral-100 dark:border-[#212126]">
        <span className="text-xs font-mono font-medium text-[#71717a] dark:text-[#71717a]">
          {isTransition
            ? 'Хук перехода (on_leave)'
            : hookType === 'on_enter'
            ? 'Входной хук (on_enter)'
            : 'Выходной хук (on_leave)'}
        </span>
        <div className="flex items-center gap-1">
          {status === 'running' && (
            <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-mono tabular-nums text-amber-700 dark:text-amber-300 bg-amber-50 dark:bg-amber-950/40 border border-amber-200 dark:border-amber-800">
              <Loader2 className="w-3 h-3 block shrink-0 animate-spin text-amber-600 dark:text-amber-400" />
              <span>{elapsedSec.toFixed(1)}s</span>
            </span>
          )}
          {status === 'success' && (
            <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-mono tabular-nums text-emerald-700 dark:text-emerald-300 bg-emerald-50 dark:bg-emerald-950/40 border border-emerald-200 dark:border-emerald-800">
              <Check className="w-3 h-3 block shrink-0 stroke-[2.5]" />
              <span>Успех {state?.durationMs ? `${(state.durationMs / 1000).toFixed(2)}s` : ''}</span>
            </span>
          )}
          {status === 'failed' && (
            <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-mono tabular-nums text-rose-700 dark:text-rose-300 bg-rose-50 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-800">
              <X className="w-3 h-3 block shrink-0 stroke-[2.5]" />
              <span>Ошибка {state?.exitCode != null ? `(код ${state.exitCode})` : ''}</span>
            </span>
          )}
          {status === 'idle' && (
            <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-mono text-neutral-600 dark:text-neutral-400 bg-neutral-100 dark:bg-[#111114] border border-neutral-200 dark:border-[#2c2c33]">
              Готов к запуску
            </span>
          )}
        </div>
      </div>

      <div className="mt-2 text-[13px] font-mono font-semibold text-neutral-900 dark:text-[#f4f4f5] truncate" aria-label={hookName} title={hookName}>
        {hookName}
      </div>

      {isTransition && sourceStageTitle ? (
        <div className="text-xs text-[#71717a] dark:text-[#71717a] font-mono mt-0.5">
          Переход из: {sourceStageTitle}
        </div>
      ) : stageTitle ? (
        <div className="text-xs text-[#71717a] dark:text-[#71717a] font-mono mt-0.5">
          Стадия: {stageTitle}
        </div>
      ) : null}

      {hookDescription && (
        <div className="mt-2 text-xs text-neutral-600 dark:text-neutral-300 font-sans leading-relaxed">
          {hookDescription}
        </div>
      )}

      {status === 'failed' && errorText.length > 0 && (
        <div className="mt-2 p-2.5 rounded bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900 text-xs font-mono text-rose-800 dark:text-rose-300 whitespace-pre-wrap break-words max-h-44 overflow-y-auto leading-relaxed overscroll-contain">
          <div className="font-semibold text-rose-900 dark:text-rose-200 mb-1 flex items-center gap-1.5">
            <Ban className="w-3.5 h-3.5 shrink-0" />
            <span>Отклонено хуком:</span>
          </div>
          {errorText}
        </div>
      )}

      {hasOutput ? (
        <div className="mt-2.5">
          <div className="flex items-center justify-between text-xs font-mono text-[#71717a] dark:text-[#71717a] mb-1">
            <span className="flex items-center gap-1">
              <Terminal className="w-3.5 h-3.5" />
              <span>Лог терминала</span>
            </span>
            <span className="text-[10px]">
              {stdout.length > 0 ? 'stdout' : ''} {stderr.length > 0 ? 'stderr' : ''}
            </span>
          </div>
          <div className="p-2.5 rounded bg-[#111114] border border-[#2c2c33] font-mono text-xs text-[#a1a1aa] leading-relaxed max-h-56 overflow-y-auto whitespace-pre-wrap break-words [word-break:break-word] select-text overscroll-contain">
            {stdout || stderr}
          </div>
        </div>
      ) : (
        (!hookDescription || status === 'running') && (
          <div className="mt-2 text-xs text-[#71717a] dark:text-[#71717a] font-sans leading-relaxed">
            {status === 'running'
              ? 'Выполняется скрипт жизненного цикла стадии...'
              : isTransition
              ? 'Скрипт перехода, выполненный при выходе из предыдущей стадии.'
              : hookType === 'on_enter'
              ? isEnterRunningTarget
                ? 'Скрипт выполнится автоматически при входе в следующую стадию.'
                : 'Скрипт выполнится автоматически при входе в эту стадию.'
              : 'Скрипт выполнится автоматически при выходе из этой стадии.'}
          </div>
        )
      )}
    </div>
  );
}

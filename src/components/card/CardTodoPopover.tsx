import React from 'react';
import { CheckCircle2, Circle } from 'lucide-react';
import type { TodoItem } from '../../types';
import { useTranslation } from '../../stores/languageStore';

interface CardTodoPopoverProps {
  popoverRef: React.RefObject<HTMLDivElement | null>;
  listRef: React.RefObject<HTMLDivElement | null>;
  coords: { top: number; left: number; placement: 'bottom' | 'top' } | null;
  todos: TodoItem[];
  stageTitle?: string;
  stageColor?: string;
  completedCount: number;
  inProgressCount: number;
  pendingCount: number;
  totalCount: number;
  percent: number;
  taskId?: string;
  onMouseEnter: () => void;
  onMouseLeave: () => void;
  onOpenTask: (taskId: string) => void;
}

export function CardTodoPopover({
  popoverRef,
  listRef,
  coords,
  todos,
  stageTitle,
  stageColor,
  completedCount,
  inProgressCount,
  pendingCount,
  totalCount,
  percent,
  taskId,
  onMouseEnter,
  onMouseLeave,
  onOpenTask,
}: CardTodoPopoverProps) {
  const { t, language } = useTranslation();

  return (
    <div
      ref={popoverRef}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
      onClick={(e) => e.stopPropagation()}
      onPointerDown={(e) => e.stopPropagation()}
      style={{
        top: `${coords?.top ?? -9999}px`,
        left: `${coords?.left ?? -9999}px`,
        visibility: coords ? 'visible' : 'hidden',
      }}
      className={`fixed z-[9999] w-[320px] sm:w-[350px] max-w-[calc(100vw-24px)] bg-white dark:bg-[#17171b] border border-neutral-200 dark:border-[#2c2c33] shadow-2xl rounded-lg p-3 text-left select-text overscroll-contain ${
        coords?.placement === 'top'
          ? "after:content-[''] after:absolute after:-bottom-3 after:left-0 after:right-0 after:h-3"
          : "before:content-[''] before:absolute before:-top-3 before:left-0 before:right-0 before:h-3"
      }`}
    >
      <div className="flex items-center justify-between gap-2 pb-2 mb-2 border-b border-neutral-100 dark:border-[#212126]">
        <div className="flex items-center gap-1.5 min-w-0">
          <span
            className="w-2 h-2 rounded-full shrink-0"
            style={{ backgroundColor: stageColor || '#0ea5e9' }}
          />
          <span className="font-semibold text-xs text-neutral-800 dark:text-[#f4f4f5] truncate">
            {stageTitle || t('card.stageChecklist')}
          </span>
        </div>
        <div className="flex items-center gap-1.5 font-mono text-xs tabular-nums shrink-0">
          <span className={completedCount > 0 ? 'text-[#15803d] dark:text-[#4ade80] font-semibold' : 'text-neutral-600 dark:text-[#71717a]'}>
            {completedCount}/{totalCount}
          </span>
          <span className="text-neutral-300 dark:text-[#52525b]">·</span>
          <span className="text-neutral-500 dark:text-[#71717a]">{percent}%</span>
        </div>
      </div>

      <div className="mb-2.5">
        <div className="flex items-center justify-between text-[11px] text-neutral-500 dark:text-[#71717a] font-sans mb-1.5">
          <div className="flex items-center gap-1.5 flex-wrap">
            <span className={completedCount > 0 ? 'text-[#15803d] dark:text-[#4ade80] font-medium' : ''}>
              {t('card.todosCompleted', { count: completedCount })}
            </span>
            {inProgressCount > 0 && (
              <>
                <span className="text-neutral-300 dark:text-[#52525b]">·</span>
                <span className="text-amber-600 dark:text-amber-400 font-medium flex items-center gap-1">
                  <span className="w-1.5 h-1.5 rounded-full bg-amber-500 animate-pulse" />
                  {t('card.inProgressCount', { count: inProgressCount })}
                </span>
              </>
            )}
            <span className="text-neutral-300 dark:text-[#52525b]">·</span>
            <span>{t('card.todosPending', { count: pendingCount })}</span>
          </div>
        </div>
        <div className="h-1.5 w-full rounded-full bg-neutral-100 dark:bg-[#212126] overflow-hidden border border-neutral-200/60 dark:border-[#2c2c33]/60">
          <div
            className="h-full rounded-full bg-[#15803d] dark:bg-[#4ade80] transition-all duration-300"
            style={{ width: `${percent}%` }}
          />
        </div>
      </div>

      <div
        ref={listRef}
        className="max-h-60 overflow-y-auto divide-y divide-neutral-100 dark:divide-[#212126] pr-1 overscroll-contain"
      >
        {todos.map((item, idx) => {
          const isCompleted = item.status === 'completed';
          const isInProgress = item.status === 'in_progress';
          return (
            <div
              key={item.id || `todo-${idx}-${item.content.slice(0, 16)}`}
              className={`py-1.5 px-2 rounded flex items-start gap-2 text-xs transition-colors ${
                isInProgress
                  ? 'bg-amber-50/60 dark:bg-amber-950/30 text-neutral-800 dark:text-[#f4f4f5]'
                  : isCompleted
                  ? 'text-neutral-400 dark:text-neutral-500'
                  : 'text-neutral-700 dark:text-[#a1a1aa]'
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
                <span className="font-mono text-[10px] text-neutral-400 dark:text-neutral-500 mr-1.5 select-none tabular-nums">
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

      {taskId ? (
        <button
          type="button"
          onClick={() => onOpenTask(taskId)}
          className="w-full text-[10px] text-neutral-400 dark:text-neutral-500 font-mono text-center pt-2 mt-2 border-t border-neutral-100 dark:border-[#212126] cursor-pointer hover:text-neutral-600 dark:hover:text-neutral-300 transition-colors"
        >
          {t('card.clickToOpenTask')}
        </button>
      ) : (
        <div className="text-[10px] text-neutral-400 dark:text-neutral-500 font-mono text-center pt-2 mt-2 border-t border-neutral-100 dark:border-[#212126]">
          {t('card.clickToOpenTask')}
        </div>
      )}
    </div>
  );
}

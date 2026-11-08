import React from 'react';
import { CheckCircle2, Loader2, Circle } from 'lucide-react';
import { useTranslation } from '../../../stores/languageStore';

export interface TodoChecklistItem {
  id?: string;
  content: string;
  status: 'pending' | 'in_progress' | 'completed' | string;
}

export interface TodoChecklistProps {
  todos: TodoChecklistItem[];
  emptyMessage?: React.ReactNode;
  maxHeightClass?: string;
  className?: string;
  showProgressBar?: boolean;
}

export const TodoChecklist: React.FC<TodoChecklistProps> = ({
  todos,
  emptyMessage,
  maxHeightClass,
  className = '',
  showProgressBar = true,
}) => {
  const { t } = useTranslation();
  const total = todos.length;
  const completed = todos.filter((t) => t.status === 'completed').length;
  const inProgress = todos.filter((t) => t.status === 'in_progress').length;

  if (total === 0 && emptyMessage) {
    return <div className={className}>{emptyMessage}</div>;
  }

  return (
    <div className={`space-y-2.5 ${className}`}>
      {/* Dual Progress Bar Track */}
      {showProgressBar && total > 0 && (
        <div className="w-full bg-[#e4e4e7] dark:bg-[#232328] h-1.5 rounded-full overflow-hidden flex">
          <div
            className="bg-emerald-500 h-full transition-all duration-300"
            style={{ width: `${(completed / total) * 100}%` }}
          />
          <div
            className="bg-amber-500 h-full transition-all duration-300"
            style={{ width: `${(inProgress / total) * 100}%` }}
          />
        </div>
      )}

      {/* Task Item Checklist */}
      <div className={`space-y-1.5 min-h-0 ${maxHeightClass ? `${maxHeightClass} overflow-y-auto` : ''}`}>
        {todos.map((todo, idx) => {
          const isDone = todo.status === 'completed';
          const isWorking = todo.status === 'in_progress';
          const key = todo.id || `todo-${idx}`;

          return (
            <div
              key={key}
              className={`flex items-start space-x-2 py-1 px-2 rounded-md transition-colors ${
                isWorking
                  ? 'bg-amber-500/[0.08] dark:bg-amber-500/[0.12] border border-amber-500/20'
                  : isDone
                  ? 'bg-emerald-500/[0.04] dark:bg-emerald-500/[0.06]'
                  : 'hover:bg-[#f4f4f5] dark:hover:bg-[#1e1e23]'
              }`}
            >
              <span
                className="mt-0.5 shrink-0"
                title={t('drawer.todosItemStatus', { status: todo.status })}
              >
                {isDone ? (
                  <CheckCircle2 className="h-3.5 w-3.5 text-emerald-600 dark:text-emerald-400 animate-checkmark-pop" />
                ) : isWorking ? (
                  <Loader2 className="h-3.5 w-3.5 text-amber-600 dark:text-amber-400 animate-spin" />
                ) : (
                  <Circle className="h-3.5 w-3.5 text-[#8f8f8f] dark:text-[#71717a]" />
                )}
              </span>
              <span
                className={`font-mono text-xs leading-relaxed select-text min-w-0 break-words ${
                  isDone
                    ? 'line-through text-[#8f8f8f] dark:text-[#71717a]'
                    : isWorking
                    ? 'text-amber-900 dark:text-amber-200 font-medium'
                    : 'text-[#171717] dark:text-[#d4d4d8]'
                }`}
              >
                {todo.content}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
};

export default TodoChecklist;

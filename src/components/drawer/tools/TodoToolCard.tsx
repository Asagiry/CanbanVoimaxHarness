import { useState } from 'react';
import { ChevronRight, ListTodo, CheckCircle2, Loader2, Circle } from 'lucide-react';
import {
  ToolCardProps,
  isToolFailed,
  StatusIndicator,
  CopyLogsButton,
  formatDurationCompact,
} from './shared';

interface TodoItem {
  content: string;
  status: 'pending' | 'in_progress' | 'completed';
}

export default function TodoToolCard({ tool }: ToolCardProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  const toolName = tool.toolName ?? 'todo_write';
  const params = (tool.params && typeof tool.params === 'object'
    ? (tool.params as Record<string, unknown>)
    : {}) as { todos?: unknown };

  let rawTodos: TodoItem[] = [];
  if (Array.isArray(params.todos)) {
    rawTodos = params.todos as TodoItem[];
  } else if (typeof params.todos === 'string') {
    try {
      const parsed = JSON.parse(params.todos);
      if (Array.isArray(parsed)) rawTodos = parsed;
    } catch {
      // ignore JSON parse error
    }
  }

  const isFailed = isToolFailed(tool);
  const isRunning = tool.status === 'running';
  const durationText = formatDurationCompact(tool.durationMs);

  const total = rawTodos.length;
  const completed = rawTodos.filter((t) => t.status === 'completed').length;
  const inProgress = rawTodos.filter((t) => t.status === 'in_progress').length;

  const percent = total > 0 ? Math.round((completed / total) * 100) : 0;

  return (
    <div
      className={`group rounded-md border text-xs shadow-2xs hover:shadow-xs overflow-hidden my-1 animate-silk-entrance transition-colors bg-white dark:bg-[#161b22] ${
        isRunning
          ? 'border-indigo-500/40 dark:border-indigo-500/30 bg-indigo-500/[0.02]'
          : 'border-[#ebebeb] dark:border-[#21262d]'
      }`}
    >
      {/* Header Strip */}
      <div
        onClick={() => setIsExpanded(!isExpanded)}
        className={`flex items-center justify-between px-2.5 py-1.5 bg-[#f8f8f9] dark:bg-[#161b22] hover:bg-[#f2f2f3] dark:hover:bg-[#1c2128] transition-colors cursor-pointer select-none gap-2 text-xs min-h-[30px] ${
          isExpanded ? 'border-b border-[#ebebeb] dark:border-[#21262d]' : ''
        }`}
      >
        <div className="flex items-center space-x-1.5 min-w-0 flex-1">
          <button
            type="button"
            className="p-0.5 text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] rounded shrink-0 transition cursor-pointer btn-tactile"
            onClick={(e) => {
              e.stopPropagation();
              setIsExpanded(!isExpanded);
            }}
            aria-label={isExpanded ? 'Свернуть задачи' : 'Развернуть задачи'}
          >
            <ChevronRight
              className={`h-3 w-3 transition-transform duration-150 ${
                isExpanded ? 'rotate-90 text-[#171717] dark:text-[#f0f6fc]' : ''
              }`}
            />
          </button>

          <span className="flex items-center shrink-0 select-none text-indigo-600 dark:text-indigo-400">
            <ListTodo className="h-3.5 w-3.5" />
          </span>

          <span className="font-mono font-medium text-[#171717] dark:text-[#f0f6fc] shrink-0">
            {toolName}:
          </span>

          <span className="font-mono text-[#52525b] dark:text-[#8b949e] shrink-0 tabular-nums">
            {completed}/{total} выполнено ({percent}%)
          </span>

          {inProgress > 0 && (
            <>
              <span className="text-[#a1a1aa] dark:text-[#6e7681] select-none text-[10px]">•</span>
              <span className="font-mono text-amber-600 dark:text-amber-400 shrink-0 tabular-nums text-[11px]">
                {inProgress} в работе
              </span>
            </>
          )}

          {durationText && (
            <>
              <span className="text-[#a1a1aa] dark:text-[#6e7681] select-none text-[10px]">•</span>
              <span className="font-mono text-[#71717a] dark:text-[#8b949e] shrink-0 tabular-nums text-[11px]">
                {durationText}
              </span>
            </>
          )}
        </div>

        <div className="flex items-center space-x-1.5 shrink-0">
          <CopyLogsButton tool={tool} isExpanded={isExpanded} />
          <StatusIndicator tool={tool} isFailed={isFailed} />
        </div>
      </div>

      {/* Expanded Checklist Body */}
      {isExpanded && (
        <div className="p-3 bg-white dark:bg-[#0d1117] space-y-2.5">
          {/* Progress track */}
          {total > 0 && (
            <div className="w-full bg-[#e4e4e7] dark:bg-[#21262d] h-1.5 rounded-full overflow-hidden flex">
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

          {/* List items */}
          <div className="space-y-1.5 pt-0.5">
            {rawTodos.map((todo, idx) => {
              const isDone = todo.status === 'completed';
              const isWorking = todo.status === 'in_progress';

              return (
                <div
                  key={idx}
                  className={`flex items-start space-x-2 py-1 px-2 rounded-md transition-colors ${
                    isWorking
                      ? 'bg-amber-500/[0.08] dark:bg-amber-500/[0.12] border border-amber-500/20'
                      : isDone
                      ? 'bg-emerald-500/[0.04] dark:bg-emerald-500/[0.06]'
                      : 'hover:bg-slate-50 dark:hover:bg-slate-900/40'
                  }`}
                >
                  <span className="mt-0.5 shrink-0">
                    {isDone ? (
                      <CheckCircle2 className="h-3.5 w-3.5 text-emerald-600 dark:text-emerald-400 animate-checkmark-pop" />
                    ) : isWorking ? (
                      <Loader2 className="h-3.5 w-3.5 text-amber-600 dark:text-amber-400 animate-spin" />
                    ) : (
                      <Circle className="h-3.5 w-3.5 text-slate-400 dark:text-slate-500" />
                    )}
                  </span>
                  <span
                    className={`font-mono text-xs leading-relaxed select-text ${
                      isDone
                        ? 'line-through text-slate-400 dark:text-slate-500'
                        : isWorking
                        ? 'text-amber-900 dark:text-amber-200 font-medium'
                        : 'text-slate-700 dark:text-slate-300'
                    }`}
                  >
                    {todo.content}
                  </span>
                </div>
              );
            })}
          </div>

          {tool.stdout && (
            <div className="pt-1 border-t border-[#f4f4f5] dark:border-[#21262d] text-[11px] font-mono text-[#71717a] dark:text-[#8b949e]">
              {tool.stdout}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

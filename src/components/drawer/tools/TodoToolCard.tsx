import { ChevronRight, ListTodo } from 'lucide-react';
import { useTranslation } from '../../../stores/languageStore';
import {
  ToolCardProps,
  isToolFailed,
  isToolSkipped,
  StatusIndicator,
  CopyLogsButton,
  BatchBadge,
  formatDurationCompact,
  useToolExpanded,
} from './shared';
import TodoChecklist, { TodoChecklistItem } from './TodoChecklist';

export default function TodoToolCard({ tool, isExpanded: controlledExpanded, onToggleExpanded }: ToolCardProps) {
  const { t } = useTranslation();
  const [isExpanded, setIsExpanded] = useToolExpanded(controlledExpanded, onToggleExpanded, false);

  const toolName = tool.toolName ?? 'todo_write';
  const params = (tool.params && typeof tool.params === 'object'
    ? (tool.params as Record<string, unknown>)
    : {}) as { todos?: unknown };

  let rawTodos: TodoChecklistItem[] = [];
  if (Array.isArray(params.todos)) {
    rawTodos = params.todos as TodoChecklistItem[];
  } else if (typeof params.todos === 'string') {
    try {
      const parsed = JSON.parse(params.todos);
      if (Array.isArray(parsed)) rawTodos = parsed;
    } catch (err) {
      console.debug('[TodoToolCard] todos JSON parse skipped:', err);
    }
  }

  const isFailed = isToolFailed(tool);
  const isSkipped = isToolSkipped(tool);
  const isRunning = tool.status === 'running';
  const durationText = formatDurationCompact(tool.durationMs);

  const total = rawTodos.length;
  const completed = rawTodos.filter((t) => t.status === 'completed').length;
  const inProgress = rawTodos.filter((t) => t.status === 'in_progress').length;

  const percent = total > 0 ? Math.round((completed / total) * 100) : 0;

  return (
    <div
      className={`group rounded-md border text-xs shadow-2xs hover:shadow-xs overflow-hidden my-1 animate-silk-entrance transition-colors bg-white dark:bg-[#17171b] ${
        isRunning
          ? 'border-sky-500/40 dark:border-sky-500/30 bg-sky-500/[0.02]'
          : 'border-[#ebebeb] dark:border-[#212126]'
      } ${isSkipped ? 'opacity-70 dark:opacity-65' : ''}`}
    >
      {/* Header Strip */}
      <div
        className={`flex items-center justify-between px-2.5 py-1.5 bg-[#f8f8f9] dark:bg-[#17171b] hover:bg-[#f2f2f3] dark:hover:bg-[#1e1e23] transition-colors select-none gap-2 text-xs min-h-[30px] ${
          isExpanded ? 'border-b border-[#ebebeb] dark:border-[#212126]' : ''
        }`}
      >
        <button
          type="button"
          onClick={() => setIsExpanded(!isExpanded)}
          aria-expanded={isExpanded}
          aria-label={isExpanded ? t('tools.todo_write.collapse') : t('tools.todo_write.expand')}
          className="flex items-center gap-1.5 min-w-0 flex-1 leading-none text-left cursor-pointer"
        >
          <span className="p-0.5 text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] rounded shrink-0 transition">
            <ChevronRight
              className={`h-3 w-3 block shrink-0 transition-transform duration-150 ${
                isExpanded ? 'rotate-90 text-[#171717] dark:text-[#f4f4f5]' : ''
              }`}
            />
          </span>

          <span className="flex items-center shrink-0 select-none text-sky-600 dark:text-sky-400">
            <ListTodo className="h-3.5 w-3.5 block shrink-0" />
          </span>

          <span className="font-mono font-medium text-[#171717] dark:text-[#f4f4f5] shrink-0">
            {toolName}:
          </span>

          <span className="font-mono text-[#52525b] dark:text-[#71717a] shrink-0 tabular-nums">
            {t('tools.todo_write.completed', { completed, total, percent })}
          </span>

          {inProgress > 0 && (
            <>
              <span className="text-[#a1a1aa] dark:text-[#52525b] select-none text-[10px] leading-none">•</span>
              <span className="font-mono text-amber-600 dark:text-amber-400 shrink-0 tabular-nums text-[11px]">
                {t('tools.todo_write.inProgress', { count: inProgress })}
              </span>
            </>
          )}

          {durationText && (
            <>
              <span className="text-[#a1a1aa] dark:text-[#52525b] select-none text-[10px] leading-none">•</span>
              <span className="font-mono text-[#71717a] dark:text-[#71717a] shrink-0 tabular-nums text-[11px]">
                {durationText}
              </span>
            </>
          )}
        </button>

        <div className="flex items-center space-x-1.5 shrink-0">
          <BatchBadge tool={tool} />
          <CopyLogsButton tool={tool} isExpanded={isExpanded} />
          <StatusIndicator tool={tool} isFailed={isFailed} />
        </div>
      </div>

      {/* Expanded Checklist Body */}
      {isExpanded && (
        <div className="p-3 bg-white dark:bg-[#111114] space-y-2.5">
          <TodoChecklist todos={rawTodos} />

          {tool.stdout && (
            <div className="pt-1 border-t border-[#f4f4f5] dark:border-[#212126] text-[11px] font-mono text-[#71717a] dark:text-[#71717a]">
              {tool.stdout}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

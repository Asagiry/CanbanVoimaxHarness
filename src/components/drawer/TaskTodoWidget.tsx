import React, { useState, useRef, useEffect, useMemo } from 'react';
import {
  ListTodo,
  CheckCircle2,
  Circle,
  ChevronDown,
  X,
} from 'lucide-react';
import { KanbanTask, TodoItem } from '../../types';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';

interface TaskTodoWidgetProps {
  task: KanbanTask;
  align?: 'left' | 'right';
  className?: string;
}

// Floating overlay popover todo widget:
// - Pill trigger sits compactly adjacent to steer bar / input controls with zero vertical layout shift
// - Collapsible plan floats as an overlay popover (z-30 shadow-xl) above the steer bar without displacing chat feed
// - Read-only plan strip: ONLY the agent fills it via todo_write tool
// - When activeSubagentId is active, displays the subagent's own todos from its dialogue/session!
const TaskTodoWidget: React.FC<TaskTodoWidgetProps> = ({
  task,
  align = 'left',
  className = '',
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const {
    activeSubagentId,
    activeSubagentTitle,
    activityFeed,
    toolIdToSubagentId,
    subagentIdToToolId,
  } = useDrawerStore();

  const extractTodosFromFeed = (feed: any[]): TodoItem[] | null => {
    for (let i = feed.length - 1; i >= 0; i--) {
      const item = feed[i];
      if (item && (item.item_type === 'todos' || item.type === 'todos')) {
        try {
          const parsed = typeof item.content === 'string' ? JSON.parse(item.content) : item.content;
          if (Array.isArray(parsed) && parsed.length > 0) {
            return parsed.map((t: any, idx: number) => ({
              id: t.id || `todo-${idx + 1}`,
              content: String(t.content).trim(),
              status: (t.status === 'completed' || t.status === 'done'
                ? 'completed'
                : t.status === 'in_progress' || t.status === 'running'
                ? 'in_progress'
                : 'pending') as TodoItem['status'],
            }));
          }
        } catch {}
      }
      const tool = item?.tool;
      const toolName = tool?.toolName || item?.toolName;
      if (toolName === 'todo_write' && tool?.params) {
        const rawTodos = (tool.params as Record<string, unknown>)?.todos;
        if (Array.isArray(rawTodos) && rawTodos.length > 0) {
          return rawTodos
            .filter((t: any) => t && typeof t.content === 'string' && t.content.trim().length > 0)
            .map((t: any, idx: number) => ({
              id: t.id || `todo-${idx + 1}`,
              content: String(t.content).trim(),
              status: (t.status === 'completed' || t.status === 'done'
                ? 'completed'
                : t.status === 'in_progress' || t.status === 'running'
                ? 'in_progress'
                : 'pending') as TodoItem['status'],
            }));
        }
      }
    }
    return null;
  };

  const todos: TodoItem[] = useMemo(() => {
    if (activeSubagentId) {
      // 1. If this activeSubagentId is a board subtask, inspect its todos
      const boardSubtask = useBoardStore.getState().tasks.find((t) => t.id === activeSubagentId);
      if (boardSubtask?.todos && boardSubtask.todos.length > 0) {
        return boardSubtask.todos;
      }

      // 2. Search activity feed for the subagent's latest todo_write tool execution or todos item
      const feed =
        activityFeed[activeSubagentId] ||
        (toolIdToSubagentId[activeSubagentId] ? activityFeed[toolIdToSubagentId[activeSubagentId]] : undefined) ||
        (subagentIdToToolId[activeSubagentId] ? activityFeed[subagentIdToToolId[activeSubagentId]] : undefined) ||
        [];

      const subTodos = extractTodosFromFeed(feed);
      if (subTodos && subTodos.length > 0) {
        return subTodos;
      }

      // If active subagent has not called todo_write yet, return empty list!
      // (STRICTLY FORBIDDEN from displaying the parent task's todos!)
      return [];
    }

    // Main parent task view: show parent task's todos
    if (task.todos && task.todos.length > 0) {
      return task.todos;
    }

    // Fallback: extract latest todos from parent task's activity feed
    const parentFeed = activityFeed[task.id] || [];
    const feedTodos = extractTodosFromFeed(parentFeed);
    if (feedTodos && feedTodos.length > 0) {
      return feedTodos;
    }

    return task.todos ?? [];
  }, [activeSubagentId, task.id, task.todos, activityFeed, toolIdToSubagentId, subagentIdToToolId]);

  const completedCount = todos.filter((t) => t.status === 'completed').length;
  const inProgressCount = todos.filter((t) => t.status === 'in_progress').length;
  const pendingCount = todos.filter((t) => t.status === 'pending').length;
  const hasTodos = todos.length > 0;

  // Handle click outside & Escape key to smoothly close popover
  useEffect(() => {
    if (!isOpen) return;

    const handlePointerDown = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setIsOpen(false);
      }
    };

    document.addEventListener('mousedown', handlePointerDown);
    document.addEventListener('keydown', handleKeyDown);

    return () => {
      document.removeEventListener('mousedown', handlePointerDown);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [isOpen]);

  return (
    <div ref={containerRef} className={`relative inline-flex items-center ${className}`}>
      {/* Compact Trigger */}
      <button
        type="button"
        onClick={() => setIsOpen((prev) => !prev)}
        aria-expanded={isOpen}
        aria-label="План выполнения задач агента"
        className={`group px-2.5 py-1 rounded-md text-xs font-sans transition-colors cursor-pointer flex items-center gap-1.5 select-none border ${
          isOpen
            ? 'bg-[#e4e4e7] dark:bg-[#21262d] border-[#d4d4d8] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc]'
            : hasTodos
            ? 'bg-[#f4f4f5] dark:bg-[#161b22] hover:bg-[#ececee] dark:hover:bg-[#21262d] border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc]'
            : 'bg-[#f4f4f5]/60 dark:bg-[#161b22]/60 hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] border-[#ebebeb] dark:border-[#30363d] text-[#737373] dark:text-[#8b949e]'
        }`}
      >
        <ListTodo
          className={`h-3.5 w-3.5 shrink-0 transition-colors ${
            hasTodos ? 'text-[#171717] dark:text-[#f0f6fc]' : 'text-[#8f8f8f] dark:text-[#8b949e]'
          }`}
        />
        <span className="font-medium text-xs tracking-tight">Задачи</span>

        {hasTodos ? (
          <div className="flex items-center space-x-1 pl-0.5">
            <span className="font-mono tabular-nums text-[11px] font-semibold text-[#15803d] dark:text-[#4ade80]">
              {completedCount}
            </span>
            <span className="font-mono tabular-nums text-[11px] text-[#a1a1aa] dark:text-[#8b949e]">/</span>
            <span className="font-mono tabular-nums text-[11px] font-medium text-[#52525b] dark:text-[#8b949e]">
              {todos.length}
            </span>
          </div>
        ) : (
          <span className="font-mono tabular-nums text-[11px] text-[#8f8f8f] dark:text-[#8b949e]">0</span>
        )}

        {/* Static indicator when an item is currently in progress */}
        {inProgressCount > 0 && (
          <span className="inline-flex rounded-full h-1.5 w-1.5 bg-amber-500 dark:bg-amber-400 ml-0.5" />
        )}

        <ChevronDown
          className={`h-3 w-3 text-[#8f8f8f] dark:text-[#8b949e] group-hover:text-[#171717] dark:group-hover:text-[#f0f6fc] transition-transform duration-200 shrink-0 ${
            isOpen ? 'rotate-180 text-[#171717] dark:text-[#f0f6fc]' : ''
          }`}
        />
      </button>

      {/* Floating Overlay Popover */}
      {isOpen && (
        <div
          role="dialog"
          aria-label="Список задач агента"
          className={`absolute bottom-full mb-2 ${
            align === 'right' ? 'right-0' : 'left-0'
          } w-[340px] sm:w-[420px] max-w-[calc(100vw-2rem)] z-30 bg-white dark:bg-[#0d1117] rounded-lg border border-[#ebebeb] dark:border-[#21262d] shadow-md overflow-hidden select-text font-sans text-[#171717] dark:text-[#f0f6fc]`}
        >
          {/* Popover Header */}
          <div className="px-3.5 py-2.5 bg-[#fafafa] dark:bg-[#161b22] border-b border-[#ebebeb] dark:border-[#21262d] flex items-center justify-between gap-2 select-none">
            <div className="flex items-center space-x-2 min-w-0">
              <ListTodo className="h-4 w-4 text-[#171717] dark:text-[#f0f6fc] shrink-0" />
              <span className="text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] tracking-tight shrink-0 truncate max-w-[240px]">
                {activeSubagentId
                  ? activeSubagentTitle
                    ? `План: ${activeSubagentTitle}`
                    : 'План субагента'
                  : 'План выполнения'}
              </span>
              {hasTodos && (
                <span className="font-mono tabular-nums text-[11px] text-[#15803d] dark:text-[#4ade80] bg-[#f0fdf4] dark:bg-[#052e16]/40 border border-[#dcfce7] dark:border-[#166534] px-1.5 py-0.5 rounded-md font-medium shrink-0">
                  {completedCount}/{todos.length}
                </span>
              )}
            </div>

            <button
              type="button"
              onClick={() => setIsOpen(false)}
              className="h-6 w-6 rounded-md hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] flex items-center justify-center transition cursor-pointer"
              aria-label="Закрыть"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          </div>

          {/* Subheader summary strip */}
          {hasTodos && (
            <div className="px-3.5 py-1.5 bg-white dark:bg-[#0d1117] border-b border-[#f4f4f5] dark:border-[#21262d] flex items-center space-x-3 text-[11px] text-[#737373] dark:text-[#8b949e] select-none">
              <div className="flex items-center space-x-1">
                <span className="font-mono tabular-nums text-[#15803d] dark:text-[#4ade80] font-semibold">
                  {completedCount}
                </span>
                <span className="text-[#737373] dark:text-[#8b949e]">готово</span>
              </div>
              <span className="text-[#e4e4e7] dark:text-[#30363d]">·</span>
              <div className="flex items-center space-x-1">
                <span className="font-mono tabular-nums text-amber-700 dark:text-amber-400 font-semibold">
                  {inProgressCount}
                </span>
                <span className="text-[#737373] dark:text-[#8b949e]">в работе</span>
              </div>
              <span className="text-[#e4e4e7] dark:text-[#30363d]">·</span>
              <div className="flex items-center space-x-1">
                <span className="font-mono tabular-nums text-[#737373] dark:text-[#8b949e] font-semibold">
                  {pendingCount}
                </span>
                <span className="text-[#737373] dark:text-[#8b949e]">в очереди</span>
              </div>
            </div>
          )}

          {/* Popover Body: Scrollable Task List */}
          <div className="min-h-0 min-w-0">
            {!hasTodos ? (
              <div className="p-4 text-center text-xs text-[#8f8f8f] dark:text-[#8b949e] font-sans leading-relaxed">
                {activeSubagentId ? (
                  <>
                    Субагент пока не составил свой план задач. Шаги появятся здесь, когда он вызовет инструмент{' '}
                    <code className="px-1.5 py-0.5 bg-[#f4f4f5] dark:bg-[#161b22] rounded border border-[#ebebeb] dark:border-[#30363d] text-[11px] font-mono text-[#171717] dark:text-[#f0f6fc]">
                      todo_write
                    </code>
                    .
                  </>
                ) : (
                  <>
                    План пока пуст. Шаги появятся здесь, когда агент вызовет инструмент{' '}
                    <code className="px-1.5 py-0.5 bg-[#f4f4f5] dark:bg-[#161b22] rounded border border-[#ebebeb] dark:border-[#30363d] text-[11px] font-mono text-[#171717] dark:text-[#f0f6fc]">
                      todo_write
                    </code>
                    .
                  </>
                )}
              </div>
            ) : (
              <div className="max-h-64 sm:max-h-72 overflow-y-auto divide-y divide-[#f4f4f5] dark:divide-[#21262d] min-h-0">
                {todos.map((item) => {
                  const isCompleted = item.status === 'completed';
                  const isInProgress = item.status === 'in_progress';

                  return (
                    <div
                      key={item.id}
                      className={`flex items-start gap-2.5 px-3.5 py-2.5 min-w-0 transition-colors ${
                        isInProgress
                          ? 'bg-amber-50/80 dark:bg-amber-950/60 border-y border-amber-200 dark:border-amber-700'
                          : 'hover:bg-[#fafafa] dark:hover:bg-[#161b22]'
                      }`}
                    >
                      {/* Status indicator icon */}
                      <span className="mt-0.5 shrink-0" title={`Статус: ${item.status}`}>
                        {isCompleted ? (
                          <CheckCircle2 className="h-4 w-4 text-[#15803d] dark:text-[#7ee787]" />
                        ) : isInProgress ? (
                          <span className="h-4 w-4 flex items-center justify-center">
                            <span className="h-1.5 w-1.5 rounded-full bg-amber-500 dark:bg-amber-400 animate-pulse" />
                          </span>
                        ) : (
                          <Circle className="h-4 w-4 text-[#d4d4d8] dark:text-[#6e7681]" />
                        )}
                      </span>

                      {/* Content */}
                      <div className="flex-1 min-w-0 py-0.5">
                        <p
                          className={`text-xs leading-relaxed break-words font-sans ${
                            isCompleted
                              ? 'line-through text-[#8f8f8f] dark:text-[#8b949e]'
                              : isInProgress
                              ? 'text-amber-800 dark:text-amber-200 font-medium'
                              : 'text-[#171717] dark:text-[#f0f6fc]'
                          }`}
                        >
                          {item.content}
                        </p>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>

        </div>
      )}
    </div>
  );
};

export default TaskTodoWidget;

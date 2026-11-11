import React, { useState, useRef, useEffect, useMemo } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { ListTodo, ChevronDown, X } from 'lucide-react';
import { KanbanTask, TodoItem } from '../../types';
import { useDrawerStore, type DrawerActivityItem } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import { useTranslation } from '../../stores/languageStore';
import TodoChecklist from './tools/TodoChecklist';

interface TaskTodoWidgetProps {
  task: KanbanTask;
  align?: 'left' | 'right';
  className?: string;
  defaultOpen?: boolean;
}

function normalizeTodoEntry(raw: unknown, idx: number): TodoItem | null {
  if (!raw || typeof raw !== 'object') return null;
  const rec = raw as Record<string, unknown>;
  if (typeof rec.content !== 'string' || rec.content.trim().length === 0) return null;
  const statusRaw = typeof rec.status === 'string' ? rec.status : 'pending';
  const status: TodoItem['status'] =
    statusRaw === 'completed' || statusRaw === 'done'
      ? 'completed'
      : statusRaw === 'in_progress' || statusRaw === 'running'
      ? 'in_progress'
      : 'pending';
  return {
    id: typeof rec.id === 'string' && rec.id ? rec.id : `todo-${idx + 1}`,
    content: rec.content.trim(),
    status,
  };
}

function extractTodosFromFeed(feed: DrawerActivityItem[]): TodoItem[] | null {
  for (let i = feed.length - 1; i >= 0; i--) {
    const item = feed[i];
    const itemRecord = item as unknown as Record<string, unknown>;
    if (item && (itemRecord.item_type === 'todos' || itemRecord.type === 'todos')) {
      try {
        const parsed: unknown =
          typeof item.content === 'string' ? JSON.parse(item.content) : item.content;
        if (Array.isArray(parsed) && parsed.length > 0) {
          const items = parsed
            .map((entry, idx) => normalizeTodoEntry(entry, idx))
            .filter((entry): entry is TodoItem => entry !== null);
          if (items.length > 0) return items;
        }
      } catch (err) {
        console.debug('[TaskTodoWidget] Failed to parse todos item payload:', err);
      }
    }
    const tool = item?.tool;
    const toolName = tool?.toolName;
    if (toolName === 'todo_write' && tool?.params) {
      const rawTodos = (tool.params as Record<string, unknown>).todos;
      if (Array.isArray(rawTodos) && rawTodos.length > 0) {
        const items = rawTodos
          .map((entry, idx) => normalizeTodoEntry(entry, idx))
          .filter((entry): entry is TodoItem => entry !== null);
        if (items.length > 0) return items;
      }
    }
  }
  return null;
}

const TaskTodoWidget: React.FC<TaskTodoWidgetProps> = ({
  task,
  align = 'left',
  className = '',
  defaultOpen = false,
}) => {
  const { t } = useTranslation();
  const [isOpen, setIsOpen] = useState(defaultOpen);
  const containerRef = useRef<HTMLDivElement>(null);

  const {
    activeSubagentId,
    activeSubagentTitle,
    activityFeed,
    toolIdToSubagentId,
    subagentIdToToolId,
  } = useDrawerStore(
    useShallow((s) => ({
      activeSubagentId: s.activeSubagentId,
      activeSubagentTitle: s.activeSubagentTitle,
      activityFeed: s.activityFeed,
      toolIdToSubagentId: s.toolIdToSubagentId,
      subagentIdToToolId: s.subagentIdToToolId,
    }))
  );

  const todos: TodoItem[] = useMemo(() => {
    if (activeSubagentId) {
      const boardSubtask = useBoardStore.getState().tasks.find((bt) => bt.id === activeSubagentId);
      if (boardSubtask?.todos && boardSubtask.todos.length > 0) {
        return boardSubtask.todos;
      }

      const feed =
        activityFeed[activeSubagentId] ||
        (toolIdToSubagentId[activeSubagentId] ? activityFeed[toolIdToSubagentId[activeSubagentId]] : undefined) ||
        (subagentIdToToolId[activeSubagentId] ? activityFeed[subagentIdToToolId[activeSubagentId]] : undefined) ||
        [];

      const subTodos = extractTodosFromFeed(feed);
      if (subTodos && subTodos.length > 0) {
        return subTodos;
      }

      return [];
    }

    if (task.todos && task.todos.length > 0) {
      return task.todos;
    }

    const parentFeed = activityFeed[task.id] || [];
    const feedTodos = extractTodosFromFeed(parentFeed);
    if (feedTodos && feedTodos.length > 0) {
      return feedTodos;
    }

    return task.todos ?? [];
  }, [activeSubagentId, task.id, task.todos, activityFeed, toolIdToSubagentId, subagentIdToToolId]);

  const completedCount = todos.filter((item) => item.status === 'completed').length;
  const inProgressCount = todos.filter((item) => item.status === 'in_progress').length;
  const pendingCount = todos.filter((item) => item.status === 'pending').length;
  const hasTodos = todos.length > 0;

  useEffect(() => {
    if (!isOpen) return;

    const handlePointerDown = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        event.stopImmediatePropagation();
        setIsOpen(false);
      }
    };

    document.addEventListener('mousedown', handlePointerDown);
    window.addEventListener('keydown', handleKeyDown, { capture: true });

    return () => {
      document.removeEventListener('mousedown', handlePointerDown);
      window.removeEventListener('keydown', handleKeyDown, { capture: true });
    };
  }, [isOpen]);

  return (
    <div ref={containerRef} className={`relative inline-flex items-center ${className}`}>
      <button
        type="button"
        onClick={() => setIsOpen((prev) => !prev)}
        aria-expanded={isOpen}
        aria-label={t('drawer.todosAriaTrigger')}
        className={`group px-2.5 py-1 rounded-md text-xs font-sans transition-colors cursor-pointer flex items-center gap-1.5 select-none border ${
          isOpen
            ? 'bg-[#e4e4e7] dark:bg-[#232328] border-[#d4d4d8] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5]'
            : hasTodos
            ? 'bg-[#f4f4f5] dark:bg-[#17171b] hover:bg-[#ececee] dark:hover:bg-[#232328] border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5]'
            : 'bg-[#f4f4f5]/60 dark:bg-[#17171b]/60 hover:bg-[#f4f4f5] dark:hover:bg-[#17171b] border-[#ebebeb] dark:border-[#2c2c33] text-[#737373] dark:text-[#71717a]'
        }`}
      >
        <ListTodo className="h-3.5 w-3.5 block shrink-0 text-sky-600 dark:text-sky-400 transition-colors" />
        <span className="font-medium text-xs tracking-tight">{t('drawer.todos')}</span>

        {hasTodos ? (
          <div className="flex items-center gap-1 pl-0.5 leading-none">
            <span className="font-mono tabular-nums text-[11px] font-semibold text-[#15803d] dark:text-[#4ade80]">
              {completedCount}
            </span>
            <span className="font-mono tabular-nums text-[11px] text-[#a1a1aa] dark:text-[#71717a]">/</span>
            <span className="font-mono tabular-nums text-[11px] font-medium text-[#52525b] dark:text-[#71717a]">
              {todos.length}
            </span>
          </div>
        ) : (
          <span className="font-mono tabular-nums text-[11px] text-[#8f8f8f] dark:text-[#71717a] leading-none">0</span>
        )}

        {inProgressCount > 0 && (
          <span className="inline-flex rounded-full h-1.5 w-1.5 bg-amber-500 dark:bg-amber-400 ml-0.5" />
        )}

        <ChevronDown
          className={`h-3 w-3 block shrink-0 text-[#8f8f8f] dark:text-[#71717a] group-hover:text-[#171717] dark:group-hover:text-[#f4f4f5] transition-transform duration-200 ${
            isOpen ? 'rotate-180 text-[#171717] dark:text-[#f4f4f5]' : ''
          }`}
        />
      </button>

      {isOpen && (
        <div
          role="dialog"
          aria-label={t('drawer.todosAriaDialog')}
          className={`absolute bottom-full mb-2 ${
            align === 'right' ? 'right-0' : 'left-0'
          } w-[340px] sm:w-[420px] max-w-[calc(100vw-2rem)] z-30 bg-white dark:bg-[#111114] rounded-lg border border-[#ebebeb] dark:border-[#212126] shadow-md overflow-hidden select-text font-sans text-[#171717] dark:text-[#f4f4f5]`}
        >
          <div className="px-3.5 py-2.5 bg-[#fafafa] dark:bg-[#17171b] border-b border-[#ebebeb] dark:border-[#212126] flex items-center justify-between gap-2 select-none">
            <div className="flex items-center space-x-2 min-w-0">
              <ListTodo className="h-4 w-4 text-sky-600 dark:text-sky-400 shrink-0" />
              <span className="text-xs font-semibold text-[#171717] dark:text-[#f4f4f5] tracking-tight shrink-0 truncate max-w-[240px]">
                {activeSubagentId
                  ? activeSubagentTitle
                    ? t('drawer.todosSubagentPlanWithTitle', { title: activeSubagentTitle })
                    : t('drawer.todosSubagentPlan')
                  : t('drawer.todosExecutionPlan')}
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
              className="h-6 w-6 rounded-md hover:bg-[#f4f4f5] dark:hover:bg-[#232328] text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] flex items-center justify-center transition cursor-pointer"
              aria-label={t('drawer.todosAriaClose')}
            >
              <X className="h-3.5 w-3.5" />
            </button>
          </div>

          {hasTodos && (
            <div className="px-3.5 py-1.5 bg-white dark:bg-[#111114] border-b border-[#f4f4f5] dark:border-[#212126] flex items-baseline space-x-3 text-[11px] text-[#737373] dark:text-[#71717a] select-none">
              <div className="flex items-baseline space-x-1">
                <span className="font-mono tabular-nums text-[#15803d] dark:text-[#4ade80] font-semibold">
                  {completedCount}
                </span>
                <span className="text-[#737373] dark:text-[#71717a]">{t('drawer.todosDone')}</span>
              </div>
              <span className="text-[#e4e4e7] dark:text-[#2c2c33]">·</span>
              <div className="flex items-baseline space-x-1">
                <span className="font-mono tabular-nums text-amber-700 dark:text-amber-400 font-semibold">
                  {inProgressCount}
                </span>
                <span className="text-[#737373] dark:text-[#71717a]">{t('drawer.todosInProgress')}</span>
              </div>
              <span className="text-[#e4e4e7] dark:text-[#2c2c33]">·</span>
              <div className="flex items-baseline space-x-1">
                <span className="font-mono tabular-nums text-[#737373] dark:text-[#71717a] font-semibold">
                  {pendingCount}
                </span>
                <span className="text-[#737373] dark:text-[#71717a]">{t('drawer.todosQueued')}</span>
              </div>
            </div>
          )}

          <div className="p-3 min-h-0 min-w-0">
            <TodoChecklist
              todos={todos}
              maxHeightClass="max-h-64 sm:max-h-72"
              emptyMessage={
                <div className="p-2 text-center text-xs text-[#8f8f8f] dark:text-[#71717a] font-sans leading-relaxed">
                  {activeSubagentId
                    ? t('drawer.todosSubagentEmptyPlan', { tool: 'todo_write' })
                    : t('drawer.todosEmptyPlan', { tool: 'todo_write' })}
                </div>
              }
            />
          </div>
        </div>
      )}
    </div>
  );
};

export default TaskTodoWidget;

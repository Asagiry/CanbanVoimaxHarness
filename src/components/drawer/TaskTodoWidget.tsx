import React, { useState } from 'react';
import {
  ListTodo,
  CheckCircle2,
  Circle,
  ChevronDown,
  ChevronRight,
} from 'lucide-react';
import { KanbanTask, TodoItem } from '../../types';

interface TaskTodoWidgetProps {
  task: KanbanTask;
}

// Read-only plan strip: ONLY the agent fills it via the todo_write tool.
// Manual editing is intentionally disabled — user interactions removed.
export const TaskTodoWidget: React.FC<TaskTodoWidgetProps> = ({ task }) => {
  const [isCollapsed, setIsCollapsed] = useState(false);

  const todos: TodoItem[] = task.todos ?? [];

  const completedCount = todos.filter((t) => t.status === 'completed').length;
  const inProgressCount = todos.filter((t) => t.status === 'in_progress').length;
  const pendingCount = todos.filter((t) => t.status === 'pending').length;

  return (
    <div className="rounded-lg border border-[#ebebeb] bg-white shadow-xs text-[#171717] overflow-hidden min-w-0 transition-colors">
      {/* Header with collapse/expand toggle */}
      <div
        onClick={() => setIsCollapsed((prev) => !prev)}
        className="px-3.5 py-2.5 bg-[#f9f9f9] hover:bg-[#f4f4f5] transition-colors flex items-center justify-between cursor-pointer select-none border-b border-[#ebebeb]"
        title={isCollapsed ? 'Развернуть план (To-dos)' : 'Свернуть план'}
      >
        <div className="flex items-center space-x-2.5 min-w-0">
          <ListTodo className="h-4 w-4 text-[#666666] shrink-0" />
          <span className="text-xs font-semibold text-[#171717] tracking-wide font-sans shrink-0">
            To-dos
          </span>
          <div className="text-[11px] font-sans text-[#8f8f8f] flex items-center space-x-1.5 truncate">
            <span className="font-mono tabular-nums text-[#15803d] font-medium">
              {completedCount}
            </span>
            <span>completed</span>
            <span className="text-[#d4d4d8]">·</span>
            <span className="font-mono tabular-nums text-[#b45309] font-medium">
              {inProgressCount}
            </span>
            <span>in progress</span>
            <span className="text-[#d4d4d8]">·</span>
            <span className="font-mono tabular-nums text-[#8f8f8f] font-medium">
              {pendingCount}
            </span>
            <span>pending</span>
          </div>
        </div>

        <div className="flex items-center space-x-1.5 shrink-0 ml-2">
          <span className="text-[#8f8f8f] p-0.5">
            {isCollapsed ? (
              <ChevronRight className="h-4 w-4" />
            ) : (
              <ChevronDown className="h-4 w-4" />
            )}
          </span>
        </div>
      </div>

      {/* Collapsible Body (read-only) */}
      {!isCollapsed && (
        <div className="min-h-0 min-w-0">
          {todos.length === 0 ? (
            <div className="p-3 text-center text-xs text-[#8f8f8f] font-sans">
              Агент ещё не составил план. Шаги появятся здесь, когда агент вызовет todo_write.
            </div>
          ) : (
            <div className="max-h-56 overflow-y-auto divide-y divide-[#ebebeb] min-h-0">
              {todos.map((item) => {
                const isCompleted = item.status === 'completed';
                const isInProgress = item.status === 'in_progress';

                return (
                  <div
                    key={item.id}
                    className={`flex items-start gap-2.5 px-3.5 py-2 min-w-0 ${
                      isInProgress ? 'bg-[#fffbeb]/50' : ''
                    }`}
                  >
                    {/* Static status icon (no interaction) */}
                    <span className="mt-0.5 shrink-0" title={`Статус: ${item.status}`}>
                      {isCompleted ? (
                        <CheckCircle2 className="h-4 w-4 text-[#15803d]" />
                      ) : isInProgress ? (
                        <span className="h-4 w-4 flex items-center justify-center">
                          <span className="h-1.5 w-1.5 rounded-full bg-[#b45309] animate-pulse" />
                        </span>
                      ) : (
                        <Circle className="h-4 w-4 text-[#d4d4d4]" />
                      )}
                    </span>

                    {/* Content */}
                    <div className="flex-1 min-w-0 py-0.5">
                      <p
                        className={`text-xs leading-relaxed break-words font-sans select-text ${
                          isCompleted
                            ? 'line-through text-[#8f8f8f]'
                            : isInProgress
                            ? 'text-[#171717] font-medium'
                            : 'text-[#4d4d4d]'
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
      )}
    </div>
  );
};

export default TaskTodoWidget;

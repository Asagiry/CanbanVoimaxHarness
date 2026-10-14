import React, { useState, useId } from 'react';
import {
  ListTodo,
  CheckCircle2,
  Circle,
  Loader2,
  ChevronDown,
  ChevronRight,
  Plus,
  Trash2,
  X,
} from 'lucide-react';
import { KanbanTask, TodoItem } from '../../types';
import { useKanbanStore } from '../../stores/kanbanStore';

interface TaskTodoWidgetProps {
  task: KanbanTask;
}

export const TaskTodoWidget: React.FC<TaskTodoWidgetProps> = ({ task }) => {
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [newContent, setNewContent] = useState('');
  const [isAdding, setIsAdding] = useState(false);
  const inputId = useId();

  const updateTask = useKanbanStore((state) => state.updateTask);

  const todos: TodoItem[] = task.todos ?? [];

  const completedCount = todos.filter((t) => t.status === 'completed').length;
  const inProgressCount = todos.filter((t) => t.status === 'in_progress').length;
  const pendingCount = todos.filter((t) => t.status === 'pending').length;

  const handleToggleStatus = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    const updated = todos.map((t) => {
      if (t.id !== id) return t;
      const nextStatus: TodoItem['status'] =
        t.status === 'pending'
          ? 'in_progress'
          : t.status === 'in_progress'
          ? 'completed'
          : 'pending';
      return { ...t, status: nextStatus };
    });
    updateTask(task.id, { todos: updated });
  };

  const handleSetStatus = (id: string, status: TodoItem['status'], e: React.MouseEvent) => {
    e.stopPropagation();
    const updated = todos.map((t) => (t.id === id ? { ...t, status } : t));
    updateTask(task.id, { todos: updated });
  };

  const handleDelete = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    const updated = todos.filter((t) => t.id !== id);
    updateTask(task.id, { todos: updated });
  };

  const handleAddTodo = (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = newContent.trim();
    if (!trimmed) return;

    const newTodo: TodoItem = {
      id: `todo-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      content: trimmed,
      status: 'pending',
    };

    updateTask(task.id, { todos: [...todos, newTodo] });
    setNewContent('');
    setIsAdding(false);
  };

  return (
    <div className="rounded-lg border border-[#21262d] bg-[#0d1117] shadow-sm text-zinc-200 overflow-hidden min-w-0 transition-colors">
      {/* Header with collapse/expand toggle */}
      <div
        onClick={() => setIsCollapsed((prev) => !prev)}
        className="px-3.5 py-2.5 bg-[#161b22]/70 hover:bg-[#161b22] transition-colors flex items-center justify-between cursor-pointer select-none border-b border-[#21262d]/60"
        title={isCollapsed ? 'Развернуть план (To-dos)' : 'Свернуть план'}
      >
        <div className="flex items-center space-x-2.5 min-w-0">
          <ListTodo className="h-4 w-4 text-zinc-400 shrink-0" />
          <span className="text-xs font-semibold text-zinc-200 tracking-wide font-sans shrink-0">
            To-dos
          </span>
          <div className="text-[11px] font-sans text-zinc-400 flex items-center space-x-1.5 truncate">
            <span className="font-mono tabular-nums text-emerald-400 font-medium">
              {completedCount}
            </span>
            <span>completed</span>
            <span className="text-zinc-600">·</span>
            <span className="font-mono tabular-nums text-amber-400 font-medium">
              {inProgressCount}
            </span>
            <span>in progress</span>
            <span className="text-zinc-600">·</span>
            <span className="font-mono tabular-nums text-zinc-400 font-medium">
              {pendingCount}
            </span>
            <span>pending</span>
          </div>
        </div>

        <div className="flex items-center space-x-1.5 shrink-0 ml-2">
          {!isCollapsed && !isAdding && (
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                setIsAdding(true);
              }}
              className="p-1 rounded text-zinc-400 hover:text-zinc-200 hover:bg-[#21262d] transition-colors cursor-pointer"
              title="Добавить шаг плана"
            >
              <Plus className="h-3.5 w-3.5" />
            </button>
          )}
          <span className="text-zinc-400 p-0.5">
            {isCollapsed ? (
              <ChevronRight className="h-4 w-4" />
            ) : (
              <ChevronDown className="h-4 w-4" />
            )}
          </span>
        </div>
      </div>

      {/* Collapsible Body */}
      {!isCollapsed && (
        <div className="min-h-0 min-w-0">
          {todos.length === 0 && !isAdding ? (
            <div className="p-3 text-center text-xs text-zinc-500 font-sans flex items-center justify-center space-x-2">
              <span>План выполнения пока не сформирован.</span>
              <button
                type="button"
                onClick={() => setIsAdding(true)}
                className="text-amber-400 hover:text-amber-300 underline font-medium cursor-pointer"
              >
                + Добавить пункт
              </button>
            </div>
          ) : (
            <div className="max-h-56 overflow-y-auto divide-y divide-[#21262d]/40 min-h-0">
              {todos.map((item) => {
                const isCompleted = item.status === 'completed';
                const isInProgress = item.status === 'in_progress';

                return (
                  <div
                    key={item.id}
                    className={`group flex items-start gap-2.5 px-3.5 py-2 transition-colors min-w-0 ${
                      isInProgress
                        ? 'bg-amber-500/[0.04]'
                        : isCompleted
                        ? 'bg-transparent'
                        : 'hover:bg-[#161b22]/50'
                    }`}
                  >
                    {/* Status Icon with toggle click */}
                    <button
                      type="button"
                      onClick={(e) => handleToggleStatus(item.id, e)}
                      className="mt-0.5 shrink-0 focus:outline-hidden cursor-pointer"
                      title={`Статус: ${item.status}. Нажмите для переключения`}
                    >
                      {isCompleted ? (
                        <CheckCircle2 className="h-4 w-4 text-emerald-400" />
                      ) : isInProgress ? (
                        <Loader2 className="h-4 w-4 text-amber-400 animate-spin" />
                      ) : (
                        <Circle className="h-4 w-4 text-zinc-500 hover:text-zinc-300 transition-colors" />
                      )}
                    </button>

                    {/* Content */}
                    <div
                      onClick={(e) => handleToggleStatus(item.id, e)}
                      className="flex-1 min-w-0 cursor-pointer select-text py-0.5"
                    >
                      <p
                        className={`text-xs leading-relaxed break-words font-sans ${
                          isCompleted
                            ? 'line-through text-zinc-500'
                            : isInProgress
                            ? 'text-zinc-100 font-medium'
                            : 'text-zinc-300'
                        }`}
                      >
                        {item.content}
                      </p>
                    </div>

                    {/* Status switcher pills & Delete button */}
                    <div className="flex items-center space-x-1 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
                      <button
                        type="button"
                        onClick={(e) => handleSetStatus(item.id, 'pending', e)}
                        className={`px-1.5 py-0.5 rounded text-[10px] font-mono transition-colors cursor-pointer ${
                          item.status === 'pending'
                            ? 'bg-zinc-700 text-zinc-200'
                            : 'text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800'
                        }`}
                        title="Установить pending"
                      >
                        wait
                      </button>
                      <button
                        type="button"
                        onClick={(e) => handleSetStatus(item.id, 'in_progress', e)}
                        className={`px-1.5 py-0.5 rounded text-[10px] font-mono transition-colors cursor-pointer ${
                          item.status === 'in_progress'
                            ? 'bg-amber-900/60 text-amber-300 border border-amber-600/40'
                            : 'text-amber-500/70 hover:text-amber-400 hover:bg-zinc-800'
                        }`}
                        title="Установить in_progress"
                      >
                        run
                      </button>
                      <button
                        type="button"
                        onClick={(e) => handleSetStatus(item.id, 'completed', e)}
                        className={`px-1.5 py-0.5 rounded text-[10px] font-mono transition-colors cursor-pointer ${
                          item.status === 'completed'
                            ? 'bg-emerald-900/60 text-emerald-300 border border-emerald-600/40'
                            : 'text-emerald-500/70 hover:text-emerald-400 hover:bg-zinc-800'
                        }`}
                        title="Установить completed"
                      >
                        done
                      </button>
                      <button
                        type="button"
                        onClick={(e) => handleDelete(item.id, e)}
                        className="p-1 rounded text-zinc-500 hover:text-red-400 hover:bg-[#21262d] transition-colors cursor-pointer ml-1"
                        title="Удалить пункт"
                      >
                        <Trash2 className="h-3 w-3" />
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}

          {/* Add Item Form */}
          {isAdding && (
            <form
              onSubmit={handleAddTodo}
              className="p-2.5 bg-[#161b22]/50 border-t border-[#21262d]/60 flex items-center space-x-2"
            >
              <input
                id={inputId}
                type="text"
                autoFocus
                value={newContent}
                onChange={(e) => setNewContent(e.target.value)}
                placeholder="Что необходимо сделать? [Enter для добавления]"
                className="flex-1 bg-[#0d1117] border border-[#30363d] focus:border-amber-500/70 rounded px-2.5 py-1 text-xs text-zinc-200 placeholder-zinc-500 focus:outline-hidden font-sans"
              />
              <button
                type="submit"
                disabled={!newContent.trim()}
                className="px-2.5 py-1 bg-amber-500/20 hover:bg-amber-500/30 text-amber-300 border border-amber-500/40 rounded text-xs font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
              >
                Добавить
              </button>
              <button
                type="button"
                onClick={() => {
                  setIsAdding(false);
                  setNewContent('');
                }}
                className="p-1 rounded text-zinc-400 hover:text-zinc-200 hover:bg-[#21262d] transition-colors cursor-pointer"
                title="Отмена"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            </form>
          )}
        </div>
      )}
    </div>
  );
};

export default TaskTodoWidget;

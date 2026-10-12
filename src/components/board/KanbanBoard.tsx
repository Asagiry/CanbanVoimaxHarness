import { useEffect, useState } from 'react';
import { DragDropContext, DropResult } from '@hello-pangea/dnd';
import { useBoardStore } from '../../stores/boardStore';
import KanbanColumn from './KanbanColumn';
import { KanbanTask } from '../../types';
import { Plus, X } from 'lucide-react';

export default function KanbanBoard() {
  const { columns, tasks, searchQuery, moveTask, addTask, tickTimers } = useBoardStore();
  const [isNewTaskModalOpen, setIsNewTaskModalOpen] = useState(false);
  const [targetColumnForNew, setTargetColumnForNew] = useState<string>('backlog');

  // New task form fields (no priority field as per user requirement)
  const [newTitle, setNewTitle] = useState('');
  const [newDescription, setNewDescription] = useState('');
  const [newFastTrackReview, setNewFastTrackReview] = useState(false);
  const [newFastTrackRetro, setNewFastTrackRetro] = useState(false);

  // 1-second timer ticker interval for live cards
  useEffect(() => {
    const timer = setInterval(() => {
      tickTimers();
    }, 1000);

    return () => clearInterval(timer);
  }, [tickTimers]);

  const handleDragEnd = (result: DropResult) => {
    const { source, destination, draggableId } = result;
    if (!destination) return;
    if (source.droppableId === destination.droppableId && source.index === destination.index) {
      return;
    }

    moveTask(draggableId, destination.droppableId);
  };

  const handleOpenAddTask = (columnId: string) => {
    setTargetColumnForNew(columnId);
    setNewTitle('');
    setNewDescription('');
    setNewFastTrackReview(false);
    setNewFastTrackRetro(false);
    setIsNewTaskModalOpen(true);
  };

  const handleCreateTask = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newTitle.trim()) return;

    const nextIdNum = tasks.length + 1;
    const paddedId = String(nextIdNum).padStart(3, '0');
    const taskId = `CVH-${paddedId}`;
    const slug = newTitle.toLowerCase().replace(/[^a-z0-9а-яё]+/gi, '-').slice(0, 24);

    const newTask: KanbanTask = {
      id: taskId,
      title: newTitle.trim(),
      description: newDescription.trim() || undefined,
      priority: 'medium',
      columnId: targetColumnForNew,
      persona: targetColumnForNew === 'dev' ? 'developer' : targetColumnForNew === 'review' ? 'reviewer' : 'artisan',
      branch: `feat/${slug}`,
      worktreePath: `worktrees/${slug}`,
      status: targetColumnForNew === 'dev' ? 'running' : 'idle',
      activeStageDuration: 0,
      stageDurations: {},
      totalElapsed: 0,
      isTimerRunning: targetColumnForNew === 'dev',
      returnsCount: 0,
      fastTrack: {
        noReview: newFastTrackReview,
        noRetro: newFastTrackRetro,
      },
      verdict: 'NONE',
    };

    addTask(newTask);
    setIsNewTaskModalOpen(false);
  };

  const filteredTasks = tasks.filter((task) => {
    if (!searchQuery) return true;
    const q = searchQuery.toLowerCase();
    return (
      task.id.toLowerCase().includes(q) ||
      task.title.toLowerCase().includes(q) ||
      task.branch.toLowerCase().includes(q) ||
      (task.description && task.description.toLowerCase().includes(q))
    );
  });

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden bg-obsidian-950">
      {/* Board Top Toolbar */}
      <div className="flex items-center justify-between px-3 py-1.5 border-b border-obsidian-800 bg-obsidian-900 text-xs font-mono select-none">
        <div className="flex items-center space-x-2 text-slate-400">
          <span>Задачи:</span>
          <strong className="text-slate-200 tabular-nums">{tasks.length}</strong>
        </div>

        <button
          onClick={() => handleOpenAddTask('backlog')}
          className="flex items-center space-x-1 px-2.5 py-1 rounded bg-obsidian-800 hover:bg-obsidian-750 border border-obsidian-700 text-slate-200 text-xs font-sans font-medium transition cursor-pointer"
          title="Создать задачу"
        >
          <Plus className="h-3.5 w-3.5" />
          <span>Создать задачу</span>
        </button>
      </div>

      {/* Kanban Columns with Horizontal Wheel Scroll */}
      <DragDropContext onDragEnd={handleDragEnd}>
        <div
          onWheel={(e) => {
            if (e.deltaY !== 0) {
              e.currentTarget.scrollLeft += e.deltaY;
            }
          }}
          className="flex-1 overflow-x-auto overflow-y-hidden p-3"
        >
          <div className="flex items-start space-x-3 h-full min-w-max pb-1">
            {columns.map((column) => {
              const columnTasks = filteredTasks.filter((t) => t.columnId === column.id);
              return (
                <KanbanColumn
                  key={column.id}
                  column={column}
                  tasks={columnTasks}
                  onAddTask={handleOpenAddTask}
                />
              );
            })}
          </div>
        </div>
      </DragDropContext>

      {/* Add Task Modal without priority field */}
      {isNewTaskModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-3">
          <div className="relative flex flex-col w-[480px] max-w-full rounded border border-obsidian-750 bg-obsidian-900 shadow-xl p-4 text-slate-200">
            <div className="flex items-center justify-between pb-2.5 border-b border-obsidian-800">
              <h3 className="text-xs font-semibold text-slate-100">
                Создать задачу
              </h3>
              <button
                onClick={() => setIsNewTaskModalOpen(false)}
                className="p-1 rounded text-slate-400 hover:text-white transition cursor-pointer"
                title="Закрыть"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            </div>

            <form onSubmit={handleCreateTask} className="mt-3 space-y-3 text-xs">
              <div>
                <label className="block text-[11px] font-mono text-slate-400 mb-1">
                  Название задачи
                </label>
                <input
                  type="text"
                  required
                  value={newTitle}
                  onChange={(e) => setNewTitle(e.target.value)}
                  placeholder="Реализация диспетчера событий"
                  className="w-full px-2.5 py-1.5 rounded bg-obsidian-950 border border-obsidian-800 text-slate-100 placeholder-slate-600 focus:outline-hidden focus:border-slate-500 font-sans"
                />
              </div>

              <div>
                <label className="block text-[11px] font-mono text-slate-400 mb-1">
                  Описание / Инварианты
                </label>
                <textarea
                  rows={3}
                  value={newDescription}
                  onChange={(e) => setNewDescription(e.target.value)}
                  placeholder="Архитектурные требования, проверяемые сенсорами..."
                  className="w-full px-2.5 py-1.5 rounded bg-obsidian-950 border border-obsidian-800 text-slate-100 placeholder-slate-600 focus:outline-hidden focus:border-slate-500 font-sans resize-none"
                />
              </div>

              <div>
                <label className="block text-[11px] font-mono text-slate-400 mb-1">
                  Начальный этап
                </label>
                <select
                  value={targetColumnForNew}
                  onChange={(e) => setTargetColumnForNew(e.target.value)}
                  className="w-full px-2 py-1.5 rounded bg-obsidian-950 border border-obsidian-800 text-slate-200 focus:outline-hidden focus:border-slate-500 font-sans text-xs cursor-pointer"
                >
                  {columns.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.title}
                    </option>
                  ))}
                </select>
              </div>

              {/* Fast-Track Options */}
              <div className="pt-2 border-t border-obsidian-800/80 flex items-center space-x-4 text-xs font-mono text-slate-400">
                <label className="flex items-center space-x-1.5 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={newFastTrackReview}
                    onChange={(e) => setNewFastTrackReview(e.target.checked)}
                    className="rounded border-obsidian-750 text-slate-200 focus:ring-0"
                  />
                  <span>Пропустить ревью</span>
                </label>
                <label className="flex items-center space-x-1.5 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={newFastTrackRetro}
                    onChange={(e) => setNewFastTrackRetro(e.target.checked)}
                    className="rounded border-obsidian-750 text-slate-200 focus:ring-0"
                  />
                  <span>Пропустить ретроспективу</span>
                </label>
              </div>

              <div className="flex items-center justify-end space-x-2 pt-2 border-t border-obsidian-800">
                <button
                  type="button"
                  onClick={() => setIsNewTaskModalOpen(false)}
                  className="px-3 py-1 rounded text-slate-400 hover:text-white transition cursor-pointer text-xs"
                >
                  Отмена
                </button>
                <button
                  type="submit"
                  className="px-3 py-1 rounded bg-slate-200 hover:bg-white text-obsidian-950 font-medium transition cursor-pointer text-xs"
                >
                  Создать задачу
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}

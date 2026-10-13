import { useEffect, useState, useMemo } from 'react';
import { DragDropContext, DropResult } from '@hello-pangea/dnd';
import { useBoardStore } from '../../stores/boardStore';
import KanbanColumn from './KanbanColumn';
import { KanbanTask } from '../../types';
import { Plus, X, Check, ArrowRight } from 'lucide-react';

const STAGE_LABELS: Record<string, { label: string; color: string }> = {
  dev: { label: 'Разработка', color: '#f59e0b' },
  review: { label: 'Ревью', color: '#a855f7' },
  retro: { label: 'Ретро', color: '#06b6d4' },
  ready: { label: 'Готово', color: '#10b981' },
};

export default function KanbanBoard() {
  const { columns, tasks, searchQuery, moveTask, addTask, tickTimers } = useBoardStore();
  const [isNewTaskModalOpen, setIsNewTaskModalOpen] = useState(false);
  const [targetColumnForNew, setTargetColumnForNew] = useState<string>('backlog');

  // New task form fields
  const [newTitle, setNewTitle] = useState('');
  const [newDescription, setNewDescription] = useState('');

  // Custom Pipeline Flow stages toggles
  const [enableReview, setEnableReview] = useState(true);
  const [enableRetro, setEnableRetro] = useState(true);

  // Active Flow computed chain
  const activeFlow = useMemo(() => {
    const flow = ['dev'];
    if (enableReview) flow.push('review');
    if (enableRetro) flow.push('retro');
    flow.push('ready');
    return flow;
  }, [enableReview, enableRetro]);

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
    setEnableReview(true);
    setEnableRetro(true);
    setIsNewTaskModalOpen(true);
  };

  const handleCreateTask = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newTitle.trim()) return;

    const nextIdNum = tasks.length + 1;
    const paddedId = String(nextIdNum).padStart(3, '0');
    const taskId = `CVH-${paddedId}`;
    const slug = newTitle.toLowerCase().replace(/[^a-z0-9]+/g, '-').slice(0, 24);

    const newTask: KanbanTask = {
      id: taskId,
      title: newTitle.trim(),
      description: newDescription.trim() || undefined,
      priority: 'high',
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
      customFlow: activeFlow,
      fastTrack: {
        noReview: !enableReview,
        noRetro: !enableRetro,
      },
      verdict: 'NONE',
    };

    addTask(newTask);
    setIsNewTaskModalOpen(false);
  };

  // Filter tasks based on searchQuery
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
      <div className="flex items-center justify-between px-3 py-1.5 border-b border-obsidian-800/80 bg-obsidian-900/40 text-xs font-mono select-none">
        <div className="flex items-center space-x-1.5 text-slate-400 text-[11px]">
          <span>Задач:</span>
          <span className="text-slate-200 font-semibold tabular-nums">{tasks.length}</span>
        </div>

        <button
          onClick={() => handleOpenAddTask('backlog')}
          className="flex items-center space-x-1 px-2.5 py-1 rounded bg-obsidian-850 hover:bg-obsidian-800 border border-obsidian-750 hover:border-obsidian-700 text-slate-200 hover:text-white font-sans text-xs transition cursor-pointer"
          title="Создать новую задачу"
        >
          <Plus className="h-3.5 w-3.5 text-cyan-400" />
          <span>Создать задачу</span>
        </button>
      </div>

      {/* Kanban Columns Horizontal Container with Mouse Wheel Scroll */}
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

      {/* Add Task Modal with Custom Flow Selector */}
      {isNewTaskModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-3 animate-in fade-in duration-100">
          <div className="relative flex flex-col w-[480px] max-w-full rounded-lg border border-obsidian-750 bg-obsidian-900 shadow-xl p-4 text-slate-200">
            <div className="flex items-center justify-between pb-2.5 border-b border-obsidian-800">
              <h3 className="text-sm font-semibold text-white">
                Новая задача
              </h3>
              <button
                onClick={() => setIsNewTaskModalOpen(false)}
                className="p-1 rounded text-slate-400 hover:text-white hover:bg-obsidian-800 transition cursor-pointer"
                title="Закрыть"
              >
                <X className="h-4 w-4" />
              </button>
            </div>

            <form onSubmit={handleCreateTask} className="mt-3.5 space-y-3 text-xs">
              <div>
                <label className="block font-sans text-slate-400 mb-1">
                  Название
                </label>
                <input
                  type="text"
                  required
                  value={newTitle}
                  onChange={(e) => setNewTitle(e.target.value)}
                  placeholder="Краткое описание задачи..."
                  className="w-full px-2.5 py-1.5 rounded bg-obsidian-950 border border-obsidian-750 text-slate-100 placeholder-slate-500 focus:outline-hidden focus:border-cyan-500 font-sans"
                />
              </div>

              <div>
                <label className="block font-sans text-slate-400 mb-1">
                  Описание
                </label>
                <textarea
                  rows={2}
                  value={newDescription}
                  onChange={(e) => setNewDescription(e.target.value)}
                  placeholder="Детали и требования к задаче..."
                  className="w-full px-2.5 py-1.5 rounded bg-obsidian-950 border border-obsidian-750 text-slate-100 placeholder-slate-500 focus:outline-hidden focus:border-cyan-500 font-sans resize-none"
                />
              </div>

              <div>
                <label className="block font-sans text-slate-400 mb-1">
                  Колонка размещения
                </label>
                <select
                  value={targetColumnForNew}
                  onChange={(e) => setTargetColumnForNew(e.target.value)}
                  className="w-full px-2 py-1.5 rounded bg-obsidian-950 border border-obsidian-750 text-slate-200 focus:outline-hidden focus:border-cyan-500 font-mono text-xs cursor-pointer"
                >
                  {columns.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.title}
                    </option>
                  ))}
                </select>
              </div>

              {/* Custom Pipeline Flow Selector */}
              <div className="space-y-2 pt-2 border-t border-obsidian-800">
                <div className="flex items-center justify-between">
                  <label className="block font-sans text-slate-400 font-medium">
                    Кастомная цепочка (Custom Pipeline Flow)
                  </label>
                  <span className="text-[10px] text-cyan-400 font-mono">
                    {enableReview && enableRetro
                      ? 'Полный цикл'
                      : !enableReview && !enableRetro
                      ? 'Fast-track'
                      : 'Выборочно'}
                  </span>
                </div>

                {/* Interactive Stage Chips */}
                <div className="flex items-center gap-1.5">
                  {/* Dev: always on */}
                  <div
                    className="flex items-center space-x-1 px-2.5 py-1 rounded bg-amber-950/40 border border-amber-600/40 text-amber-300 font-mono text-[11px] cursor-not-allowed select-none"
                    title="Разработка обязательна"
                  >
                    <Check className="h-3 w-3 text-amber-400" />
                    <span>Разработка</span>
                  </div>

                  {/* Review: toggleable */}
                  <button
                    type="button"
                    onClick={() => setEnableReview(!enableReview)}
                    className={`flex items-center space-x-1 px-2.5 py-1 rounded border font-mono text-[11px] transition cursor-pointer select-none ${
                      enableReview
                        ? 'bg-purple-950/40 border-purple-500/50 text-purple-300 hover:bg-purple-900/40'
                        : 'bg-obsidian-950 border-obsidian-800 text-slate-500 hover:text-slate-300 hover:border-obsidian-700'
                    }`}
                  >
                    {enableReview ? (
                      <Check className="h-3 w-3 text-purple-400" />
                    ) : (
                      <X className="h-3 w-3 text-slate-500" />
                    )}
                    <span>Ревью</span>
                  </button>

                  {/* Retro: toggleable */}
                  <button
                    type="button"
                    onClick={() => setEnableRetro(!enableRetro)}
                    className={`flex items-center space-x-1 px-2.5 py-1 rounded border font-mono text-[11px] transition cursor-pointer select-none ${
                      enableRetro
                        ? 'bg-cyan-950/40 border-cyan-500/50 text-cyan-300 hover:bg-cyan-900/40'
                        : 'bg-obsidian-950 border-obsidian-800 text-slate-500 hover:text-slate-300 hover:border-obsidian-700'
                    }`}
                  >
                    {enableRetro ? (
                      <Check className="h-3 w-3 text-cyan-400" />
                    ) : (
                      <X className="h-3 w-3 text-slate-500" />
                    )}
                    <span>Ретро</span>
                  </button>

                  {/* Ready: always destination */}
                  <div
                    className="flex items-center space-x-1 px-2.5 py-1 rounded bg-emerald-950/40 border border-emerald-600/40 text-emerald-300 font-mono text-[11px] cursor-not-allowed select-none"
                    title="Готово (финал)"
                  >
                    <Check className="h-3 w-3 text-emerald-400" />
                    <span>Готово</span>
                  </div>
                </div>

                {/* Live Visual Arrow Chain */}
                <div className="flex items-center space-x-1.5 p-2 rounded bg-obsidian-950 border border-obsidian-800 text-[11px] font-mono overflow-x-auto">
                  {activeFlow.map((stageId, idx) => (
                    <div key={stageId} className="flex items-center space-x-1.5 shrink-0">
                      <span
                        className="px-2 py-0.5 rounded font-semibold text-[10px]"
                        style={{
                          color: STAGE_LABELS[stageId].color,
                          backgroundColor: `${STAGE_LABELS[stageId].color}15`,
                          border: `1px solid ${STAGE_LABELS[stageId].color}40`,
                        }}
                      >
                        {STAGE_LABELS[stageId].label}
                      </span>
                      {idx < activeFlow.length - 1 && (
                        <ArrowRight className="h-3 w-3 text-slate-500 shrink-0" />
                      )}
                    </div>
                  ))}
                </div>
              </div>

              <div className="flex items-center justify-end space-x-2 pt-2.5 border-t border-obsidian-800">
                <button
                  type="button"
                  onClick={() => setIsNewTaskModalOpen(false)}
                  className="px-3 py-1.5 rounded text-slate-400 hover:text-white hover:bg-obsidian-800 transition cursor-pointer font-sans"
                >
                  Отмена
                </button>
                <button
                  type="submit"
                  className="px-3.5 py-1.5 rounded bg-cyan-400 hover:bg-cyan-300 text-obsidian-950 font-semibold transition cursor-pointer font-sans"
                >
                  Создать
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}

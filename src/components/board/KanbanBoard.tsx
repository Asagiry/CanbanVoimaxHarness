import { useEffect, useState } from 'react';
import { DragDropContext, DropResult } from '@hello-pangea/dnd';
import { useBoardStore } from '../../stores/boardStore';
import KanbanColumn from './KanbanColumn';
import { KanbanTask, TaskPriority } from '../../types';
import { Plus, X, Sparkles } from 'lucide-react';

export default function KanbanBoard() {
  const { columns, tasks, searchQuery, moveTask, addTask, tickTimers } = useBoardStore();
  const [isNewTaskModalOpen, setIsNewTaskModalOpen] = useState(false);
  const [targetColumnForNew, setTargetColumnForNew] = useState<string>('backlog');

  // New task form fields
  const [newTitle, setNewTitle] = useState('');
  const [newDescription, setNewDescription] = useState('');
  const [newPriority, setNewPriority] = useState<TaskPriority>('high');
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
    setNewPriority('high');
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
    const slug = newTitle.toLowerCase().replace(/[^a-z0-9]+/g, '-').slice(0, 24);

    const newTask: KanbanTask = {
      id: taskId,
      title: newTitle.trim(),
      description: newDescription.trim() || undefined,
      priority: newPriority,
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
      <div className="flex items-center justify-between px-5 py-2.5 border-b border-slate-800 bg-obsidian-900/90 text-xs font-mono">
        <div className="flex items-center space-x-3">
          <span className="text-slate-400">Total Active Tasks: <strong className="text-cyber-cyan tabular-nums">{tasks.length}</strong></span>
          <span className="text-slate-600">|</span>
          <span className="text-slate-400">In-Flight Agents: <strong className="text-cyber-emerald tabular-nums">{tasks.filter((t) => t.status === 'running').length}</strong></span>
          <span className="text-slate-600">|</span>
          <span className="text-slate-400">Review Queue: <strong className="text-cyber-purple tabular-nums">{tasks.filter((t) => t.columnId === 'review').length}</strong></span>
        </div>

        <div className="flex items-center space-x-3">
          <button
            onClick={() => handleOpenAddTask('backlog')}
            className="flex items-center space-x-1.5 px-3 py-1 rounded bg-cyber-indigo hover:bg-indigo-500 text-white font-sans text-xs font-semibold transition active:scale-95 shadow-sm shadow-indigo-900/40"
          >
            <Plus className="h-3.5 w-3.5" />
            <span>New Task Card</span>
          </button>
        </div>
      </div>

      {/* Kanban Columns Horizontal Container */}
      <DragDropContext onDragEnd={handleDragEnd}>
        <div className="flex-1 overflow-x-auto overflow-y-hidden p-4">
          <div className="flex items-start space-x-4 h-full min-w-max pb-2">
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

      {/* Quick Add Task Modal */}
      {isNewTaskModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-xs p-4 animate-in fade-in duration-150">
          <div className="relative flex flex-col w-[540px] max-w-full rounded-xl border border-slate-750 bg-obsidian-900 shadow-2xl p-5 text-slate-200">
            <div className="flex items-center justify-between pb-3 border-b border-slate-800">
              <h3 className="text-sm font-bold text-white flex items-center space-x-2">
                <Sparkles className="h-4 w-4 text-cyber-cyan" />
                <span>Create New Engineering Task</span>
              </h3>
              <button
                onClick={() => setIsNewTaskModalOpen(false)}
                className="p-1 rounded text-slate-400 hover:text-white hover:bg-obsidian-800 transition"
              >
                <X className="h-4 w-4" />
              </button>
            </div>

            <form onSubmit={handleCreateTask} className="mt-4 space-y-3.5 text-xs">
              <div>
                <label className="block text-[11px] font-semibold text-slate-300 uppercase tracking-wider mb-1">
                  Task Title
                </label>
                <input
                  type="text"
                  required
                  value={newTitle}
                  onChange={(e) => setNewTitle(e.target.value)}
                  placeholder="e.g. Implement Tokio Event Broadcaster"
                  className="w-full px-3 py-1.5 rounded bg-obsidian-950 border border-slate-750 text-slate-100 placeholder-slate-500 focus:outline-hidden focus:border-cyan-500 font-sans"
                />
              </div>

              <div>
                <label className="block text-[11px] font-semibold text-slate-300 uppercase tracking-wider mb-1">
                  Description / Specification Snippet
                </label>
                <textarea
                  rows={3}
                  value={newDescription}
                  onChange={(e) => setNewDescription(e.target.value)}
                  placeholder="Detailed architectural scope and Ground Truth Sensor requirements..."
                  className="w-full px-3 py-1.5 rounded bg-obsidian-950 border border-slate-750 text-slate-100 placeholder-slate-500 focus:outline-hidden focus:border-cyan-500 font-sans resize-none"
                />
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-[11px] font-semibold text-slate-300 uppercase tracking-wider mb-1">
                    Initial Stage
                  </label>
                  <select
                    value={targetColumnForNew}
                    onChange={(e) => setTargetColumnForNew(e.target.value)}
                    className="w-full px-2.5 py-1.5 rounded bg-obsidian-950 border border-slate-750 text-slate-200 focus:outline-hidden focus:border-cyan-500 font-mono"
                  >
                    {columns.map((c) => (
                      <option key={c.id} value={c.id}>
                        {c.title}
                      </option>
                    ))}
                  </select>
                </div>

                <div>
                  <label className="block text-[11px] font-semibold text-slate-300 uppercase tracking-wider mb-1">
                    Priority
                  </label>
                  <select
                    value={newPriority}
                    onChange={(e) => setNewPriority(e.target.value as TaskPriority)}
                    className="w-full px-2.5 py-1.5 rounded bg-obsidian-950 border border-slate-750 text-slate-200 focus:outline-hidden focus:border-cyan-500 font-mono"
                  >
                    <option value="critical">Critical (Crimson)</option>
                    <option value="high">High (Amber)</option>
                    <option value="medium">Medium (Sky)</option>
                    <option value="low">Low (Slate)</option>
                  </select>
                </div>
              </div>

              {/* Fast-Track Options */}
              <div className="p-2.5 rounded bg-obsidian-950 border border-slate-800 space-y-2">
                <span className="text-[10px] font-bold text-slate-400 uppercase tracking-wider block font-mono">
                  ⚡ Fast-Track Pipeline Overrides
                </span>
                <div className="flex items-center space-x-4">
                  <label className="flex items-center space-x-1.5 cursor-pointer text-slate-300">
                    <input
                      type="checkbox"
                      checked={newFastTrackReview}
                      onChange={(e) => setNewFastTrackReview(e.target.checked)}
                      className="rounded border-slate-750 text-cyan-500 focus:ring-0"
                    />
                    <span>Skip Review Gate</span>
                  </label>
                  <label className="flex items-center space-x-1.5 cursor-pointer text-slate-300">
                    <input
                      type="checkbox"
                      checked={newFastTrackRetro}
                      onChange={(e) => setNewFastTrackRetro(e.target.checked)}
                      className="rounded border-slate-750 text-cyan-500 focus:ring-0"
                    />
                    <span>Skip Retrospective</span>
                  </label>
                </div>
              </div>

              <div className="flex items-center justify-end space-x-2 pt-2 border-t border-slate-800">
                <button
                  type="button"
                  onClick={() => setIsNewTaskModalOpen(false)}
                  className="px-3 py-1.5 rounded text-slate-400 hover:text-white hover:bg-obsidian-800 transition"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="px-4 py-1.5 rounded bg-cyber-cyan hover:bg-cyan-400 text-obsidian-950 font-bold transition active:scale-95 shadow-md shadow-cyan-900/30"
                >
                  Create Task
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}

import { Draggable } from '@hello-pangea/dnd';
import { KanbanTask, TaskStatus } from '../../types';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore, syncTasksBackend } from '../../stores/boardStore';
import { Code2, ShieldCheck, Sparkles, Bot, Palette, Play } from 'lucide-react';

interface KanbanCardProps {
  task: KanbanTask;
  index: number;
}

export default function KanbanCard({ task, index }: KanbanCardProps) {
  const { openDrawer } = useDrawerStore();
  const { tasks, runTaskAgent } = useBoardStore();

  const handleRunAgent = async (e: React.MouseEvent) => {
    e.stopPropagation();
    await syncTasksBackend(tasks);
    await runTaskAgent(task.id);
  };

  const getPersonaIcon = (persona?: string) => {
    switch (persona?.toLowerCase()) {
      case 'developer':
        return <Code2 className="h-3 w-3 text-[#171717] shrink-0" />;
      case 'reviewer':
        return <ShieldCheck className="h-3 w-3 text-[#171717] shrink-0" />;
      case 'retro':
        return <Sparkles className="h-3 w-3 text-[#171717] shrink-0" />;
      case 'artisan':
        return <Palette className="h-3 w-3 text-[#171717] shrink-0" />;
      default:
        return <Bot className="h-3 w-3 text-[#666666] shrink-0" />;
    }
  };

  const getStatusBadge = (status: TaskStatus) => {
    switch (status) {
      case 'running':
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#15803d] bg-[#f0fdf4] border border-[#bbf7d0]">
            <span className="h-1.5 w-1.5 rounded-full bg-[#15803d] animate-pulse" />
            <span>В работе</span>
          </span>
        );
      case 'paused':
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#b45309] bg-[#fffbeb] border border-[#fde68a]">
            <span className="h-1.5 w-1.5 rounded-full bg-[#b45309]" />
            <span>Пауза</span>
          </span>
        );
      case 'waiting_review':
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#7e22ce] bg-[#faf5ff] border border-[#e9d5ff]">
            <span>Ревью</span>
          </span>
        );
      case 'done':
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#15803d] bg-[#f0fdf4] border border-[#bbf7d0]">
            <span>Готово</span>
          </span>
        );
      case 'blocked':
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#b91c1c] bg-[#fef2f2] border border-[#fecaca]">
            <span>Блок</span>
          </span>
        );
      case 'idle':
      default:
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#666666] bg-[#f4f4f5] border border-[#ebebeb]">
            <span>Ожидает</span>
          </span>
        );
    }
  };

  return (
    <Draggable draggableId={task.id} index={index}>
      {(provided, snapshot) => (
        <div
          ref={provided.innerRef}
          {...provided.draggableProps}
          {...provided.dragHandleProps}
          onClick={() => openDrawer(task.id)}
          className={`group relative mb-2 rounded-md border p-3 cursor-pointer select-none transition duration-150 active:scale-[0.99] ${
            snapshot.isDragging
              ? 'border-[#171717] shadow-md bg-white z-40'
              : 'border-[#ebebeb] bg-white hover:border-[#d4d4d8] shadow-xs'
          }`}
        >
          {/* Header: ID, Play Button & Concise Status */}
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center space-x-1.5">
              <span className="font-mono text-[10px] font-semibold text-[#171717] bg-[#f4f4f5] px-1.5 py-0.5 rounded border border-[#ebebeb] tabular-nums">
                {task.id}
              </span>
              {(task.columnId === 'backlog' || (task.columnId === 'dev' && task.status !== 'running')) && (
                <button
                  type="button"
                  onClick={handleRunAgent}
                  className="flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#171717] bg-[#f4f4f5] hover:bg-[#171717] hover:text-white border border-[#ebebeb] transition cursor-pointer"
                  title="Запустить агента (run_agent)"
                >
                  <Play className="h-2.5 w-2.5 fill-current" />
                  <span>Пуск</span>
                </button>
              )}
            </div>
            {getStatusBadge(task.status)}
          </div>

          {/* Title */}
          <h4 className="text-xs font-medium text-[#171717] leading-snug mb-2.5 font-sans break-words">
            {task.title}
          </h4>

          {/* Footer: Persona Assignee, Model Badge & Branch */}
          <div className="flex items-center justify-between pt-2 border-t border-[#ebebeb] text-[10px] font-mono text-[#666666]">
            <div className="flex items-center space-x-1.5">
              {getPersonaIcon(task.persona)}
              <span className="capitalize text-[#4d4d4d]">{task.persona || 'agent'}</span>
            </div>
            <div className="flex items-center space-x-1.5 shrink-0">
              {task.model && (
                <span
                  className="text-[9px] font-mono text-[#666666] bg-[#f4f4f5] px-1.5 py-0.5 rounded border border-[#ebebeb] truncate max-w-[120px]"
                  title={`Модель: ${task.model}`}
                >
                  {task.model}
                </span>
              )}
              {task.branch && (
                <span className="text-[#8f8f8f] truncate max-w-[100px]" title={task.branch}>
                  {task.branch}
                </span>
              )}
            </div>
          </div>
        </div>
      )}
    </Draggable>
  );
}

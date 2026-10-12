import { Draggable } from '@hello-pangea/dnd';
import { KanbanTask, TaskStatus } from '../../types';
import { useDrawerStore } from '../../stores/drawerStore';
import { Code2, ShieldCheck, Sparkles, Bot, Palette } from 'lucide-react';

interface KanbanCardProps {
  task: KanbanTask;
  index: number;
}

export default function KanbanCard({ task, index }: KanbanCardProps) {
  const { openDrawer } = useDrawerStore();

  const getPersonaIcon = (persona?: string) => {
    switch (persona?.toLowerCase()) {
      case 'developer':
        return <Code2 className="h-3 w-3 text-amber-400 shrink-0" />;
      case 'reviewer':
        return <ShieldCheck className="h-3 w-3 text-purple-400 shrink-0" />;
      case 'retro':
        return <Sparkles className="h-3 w-3 text-cyan-400 shrink-0" />;
      case 'artisan':
        return <Palette className="h-3 w-3 text-pink-400 shrink-0" />;
      default:
        return <Bot className="h-3 w-3 text-slate-400 shrink-0" />;
    }
  };

  const getStatusBadge = (status: TaskStatus) => {
    switch (status) {
      case 'running':
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.2 rounded text-[10px] font-mono text-emerald-400 bg-emerald-950/40 border border-emerald-500/30">
            <span className="h-1.5 w-1.5 rounded-full bg-emerald-400 animate-pulse" />
            <span>В работе</span>
          </span>
        );
      case 'paused':
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.2 rounded text-[10px] font-mono text-amber-400 bg-amber-950/40 border border-amber-500/30">
            <span className="h-1.5 w-1.5 rounded-full bg-amber-400" />
            <span>Пауза</span>
          </span>
        );
      case 'waiting_review':
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.2 rounded text-[10px] font-mono text-purple-400 bg-purple-950/40 border border-purple-500/30">
            <span>Ревью</span>
          </span>
        );
      case 'done':
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.2 rounded text-[10px] font-mono text-emerald-400 bg-emerald-950/40 border border-emerald-500/30">
            <span>Готово</span>
          </span>
        );
      case 'blocked':
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.2 rounded text-[10px] font-mono text-rose-400 bg-rose-950/40 border border-rose-500/30">
            <span>Блок</span>
          </span>
        );
      case 'idle':
      default:
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.2 rounded text-[10px] font-mono text-slate-400 bg-obsidian-900 border border-obsidian-750">
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
          className={`group relative mb-2 rounded-md border p-2.5 cursor-pointer select-none transition duration-150 active:scale-[0.99] ${
            snapshot.isDragging
              ? 'border-cyan-500 shadow-lg shadow-cyan-950/40 bg-obsidian-800 z-40'
              : 'border-obsidian-800 bg-obsidian-850 hover:border-slate-650 hover:bg-obsidian-800'
          }`}
        >
          {/* Header: ID & Concise Status */}
          <div className="flex items-center justify-between mb-1.5">
            <span className="font-mono text-[10px] font-bold text-cyan-400 bg-cyan-950/60 px-1.5 py-0.2 rounded border border-cyan-800/40 tabular-nums">
              {task.id}
            </span>
            {getStatusBadge(task.status)}
          </div>

          {/* Title */}
          <h4 className="text-xs font-medium text-slate-100 group-hover:text-white leading-snug mb-2 font-sans break-words">
            {task.title}
          </h4>

          {/* Footer: Persona Assignee */}
          <div className="flex items-center justify-between pt-1.5 border-t border-obsidian-800 text-[10px] font-mono text-slate-400">
            <div className="flex items-center space-x-1">
              {getPersonaIcon(task.persona)}
              <span className="capitalize text-slate-300">{task.persona || 'agent'}</span>
            </div>
            {task.branch && (
              <span className="text-slate-500 truncate max-w-[120px]" title={task.branch}>
                {task.branch}
              </span>
            )}
          </div>
        </div>
      )}
    </Draggable>
  );
}

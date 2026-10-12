import { Draggable } from '@hello-pangea/dnd';
import { KanbanTask, TaskPriority, TaskVerdict } from '../../types';
import { formatDuration, formatShortDuration } from '../../lib/utils';
import { useDrawerStore } from '../../stores/drawerStore';
import {
  Clock,
  RotateCcw,
  Zap,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  GitBranch,
  Terminal,
  Code2,
  ShieldCheck,
  Sparkles,
  Palette,
  Bot,
} from 'lucide-react';

interface KanbanCardProps {
  task: KanbanTask;
  index: number;
}

export default function KanbanCard({ task, index }: KanbanCardProps) {
  const { openDrawer } = useDrawerStore();

  const getPriorityBadge = (priority: TaskPriority) => {
    switch (priority) {
      case 'critical':
        return (
          <span className="px-1.5 py-0.2 rounded text-[10px] font-mono font-bold uppercase tracking-wider bg-rose-950/40 text-rose-300 border border-rose-500/40 tabular-nums">
            Critical
          </span>
        );
      case 'high':
        return (
          <span className="px-1.5 py-0.2 rounded text-[10px] font-mono font-bold uppercase tracking-wider bg-amber-950/40 text-amber-300 border border-amber-500/40 tabular-nums">
            High
          </span>
        );
      case 'medium':
        return (
          <span className="px-1.5 py-0.2 rounded text-[10px] font-mono font-medium uppercase tracking-wider bg-sky-950/40 text-sky-300 border border-sky-500/30 tabular-nums">
            Med
          </span>
        );
      case 'low':
      default:
        return (
          <span className="px-1.5 py-0.2 rounded text-[10px] font-mono font-medium uppercase tracking-wider bg-slate-900/60 text-slate-400 border border-obsidian-750 tabular-nums">
            Low
          </span>
        );
    }
  };

  const getPersonaIcon = (persona: string) => {
    switch (persona.toLowerCase()) {
      case 'developer':
        return <Code2 className="h-3 w-3 text-cyber-amber shrink-0" />;
      case 'reviewer':
        return <ShieldCheck className="h-3 w-3 text-cyber-purple shrink-0" />;
      case 'retro':
        return <Sparkles className="h-3 w-3 text-cyber-cyan shrink-0" />;
      case 'artisan':
        return <Palette className="h-3 w-3 text-pink-400 shrink-0" />;
      default:
        return <Bot className="h-3 w-3 text-slate-400 shrink-0" />;
    }
  };

  const getVerdictBadge = (verdict?: TaskVerdict) => {
    if (!verdict || verdict === 'NONE') return null;
    switch (verdict) {
      case 'APPROVED':
        return (
          <div className="flex items-center space-x-1 px-1.5 py-0.5 rounded bg-emerald-950/30 border border-emerald-500/40 text-cyber-emerald text-[10px] font-mono font-bold">
            <CheckCircle2 className="h-3 w-3 shrink-0" />
            <span>APPROVED</span>
          </div>
        );
      case 'APPROVED_WITH_CAVEATS':
        return (
          <div className="flex items-center space-x-1 px-1.5 py-0.5 rounded bg-amber-950/30 border border-amber-500/40 text-cyber-amber text-[10px] font-mono font-bold">
            <AlertTriangle className="h-3 w-3 shrink-0" />
            <span>APPROVED W/ CAVEATS</span>
          </div>
        );
      case 'CHANGES_REQUESTED':
        return (
          <div className="flex items-center space-x-1 px-1.5 py-0.5 rounded bg-rose-950/30 border border-rose-500/40 text-cyber-crimson text-[10px] font-mono font-bold">
            <XCircle className="h-3 w-3 shrink-0" />
            <span>CHANGES REQUESTED</span>
          </div>
        );
    }
  };

  // Previous frozen stage entries
  const frozenStages = Object.entries(task.stageDurations || {}).filter(
    ([stage]) => stage !== task.columnId
  );

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
              ? 'border-cyber-cyan shadow-lg shadow-cyan-950/40 scale-[1.01] z-40 bg-obsidian-800'
              : 'border-obsidian-800 bg-obsidian-850 hover:border-slate-650 hover:bg-obsidian-800 shadow-xs'
          }`}
        >
          {/* Card Top: ID, Priority, and Returns Badge */}
          <div className="flex items-center justify-between mb-1.5">
            <div className="flex items-center space-x-1.5">
              <span className="font-mono text-[10px] font-bold text-cyber-cyan bg-cyan-950/70 px-1.5 py-0.2 rounded border border-cyan-800/40 tabular-nums">
                {task.id}
              </span>
              {getPriorityBadge(task.priority)}
            </div>

            {/* Returns Counter Badge (Crime/Rework Crimson) */}
            {task.returnsCount > 0 && (
              <div
                className="flex items-center space-x-1 px-1.5 py-0.2 rounded bg-rose-950/50 border border-rose-500/60 text-cyber-crimson font-mono text-[10px] font-bold tabular-nums animate-pulse"
                title={`${task.returnsCount} rework iterations`}
              >
                <RotateCcw className="h-2.5 w-2.5 shrink-0" />
                <span>↩ Returns: {task.returnsCount}x</span>
              </div>
            )}
          </div>

          {/* Card Title */}
          <h4 className="text-xs font-semibold text-slate-100 group-hover:text-white leading-snug mb-1">
            {task.title}
          </h4>

          {/* Description Snippet */}
          {task.description && (
            <p className="text-[11px] text-slate-400 line-clamp-2 leading-relaxed mb-2 font-sans">
              {task.description}
            </p>
          )}

          {/* Fast-Track Tags */}
          {(task.fastTrack?.noReview || task.fastTrack?.noRetro) && (
            <div className="flex flex-wrap gap-1 mb-2">
              {task.fastTrack?.noReview && (
                <span className="flex items-center space-x-0.5 text-[9px] font-mono font-medium px-1.5 py-0.2 rounded bg-amber-950/30 border border-amber-500/30 text-amber-300 tabular-nums">
                  <Zap className="h-2.5 w-2.5 text-amber-400" />
                  <span>⚡ Fast-Track: No Review</span>
                </span>
              )}
              {task.fastTrack?.noRetro && (
                <span className="flex items-center space-x-0.5 text-[9px] font-mono font-medium px-1.5 py-0.2 rounded bg-amber-950/30 border border-amber-500/30 text-amber-300 tabular-nums">
                  <Zap className="h-2.5 w-2.5 text-amber-400" />
                  <span>⚡ Fast-Track: No Retro</span>
                </span>
              )}
            </div>
          )}

          {/* Verdict Banner */}
          {task.verdict && task.verdict !== 'NONE' && (
            <div className="mb-2">
              {getVerdictBadge(task.verdict)}
            </div>
          )}

          {/* Persona and Branch Info */}
          <div className="flex items-center justify-between pt-1.5 pb-1.5 border-t border-obsidian-800 text-[10px] font-mono">
            <div className="flex items-center space-x-1 px-1.5 py-0.2 rounded bg-obsidian-900 border border-obsidian-750 text-slate-300">
              {getPersonaIcon(task.persona)}
              <span className="capitalize">{task.persona}</span>
            </div>

            <div className="flex items-center space-x-1 text-slate-400 truncate max-w-[130px]" title={task.branch}>
              <GitBranch className="h-2.5 w-2.5 shrink-0 text-slate-500" />
              <span className="truncate">{task.branch}</span>
            </div>
          </div>

          {/* Timers Bar (Monospace Tabular-Nums) */}
          <div className="pt-1.5 border-t border-obsidian-800 flex items-center justify-between text-[10px] font-mono tabular-nums text-slate-400">
            {/* Active Stage Running Timer */}
            <div className="flex items-center space-x-1">
              <span
                className={`h-1.5 w-1.5 rounded-full ${
                  task.isTimerRunning ? 'bg-cyber-emerald animate-pulse' : 'bg-slate-600'
                }`}
              />
              <span className="text-slate-300 font-medium capitalize">
                {task.columnId}: <strong className="text-slate-100 font-mono tabular-nums">{formatDuration(task.activeStageDuration)}</strong>
              </span>
            </div>

            {/* Frozen / Total Timer */}
            <div className="flex items-center space-x-2 text-[10px] text-slate-400">
              {frozenStages.length > 0 && (
                <span className="hidden sm:inline text-slate-500 font-mono tabular-nums" title="Previous stage">
                  {frozenStages[0][0].substring(0, 4)}: {formatShortDuration(frozenStages[0][1])}
                </span>
              )}
              <div className="flex items-center space-x-0.5 text-slate-400 font-mono tabular-nums" title="Total Task Elapsed">
                <Clock className="h-2.5 w-2.5 text-slate-500" />
                <span>Tot: {formatShortDuration(task.totalElapsed)}</span>
              </div>
            </div>
          </div>

          {/* Quick Drawer Open Indicator on Hover */}
          <div className="absolute right-1.5 top-1.5 opacity-0 group-hover:opacity-100 transition-opacity">
            <span className="p-1 rounded bg-obsidian-750 text-slate-300 hover:text-white" title="Open Agent Console">
              <Terminal className="h-3 w-3" />
            </span>
          </div>
        </div>
      )}
    </Draggable>
  );
}

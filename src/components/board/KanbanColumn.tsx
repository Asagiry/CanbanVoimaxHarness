import { Droppable } from '@hello-pangea/dnd';
import { PipelineColumn, KanbanTask } from '../../types';
import KanbanCard from '../card/KanbanCard';
import { useCortexStore } from '../../stores/cortexStore';
import { formatTokens } from '../../lib/utils';
import {
  Inbox,
  ListTodo,
  Code2,
  ShieldCheck,
  Sparkles,
  CheckCircle2,
  Cpu,
  Layers,
  Plus,
} from 'lucide-react';

interface KanbanColumnProps {
  column: PipelineColumn;
  tasks: KanbanTask[];
  onAddTask?: (columnId: string) => void;
}

export default function KanbanColumn({ column, tasks, onAddTask }: KanbanColumnProps) {
  const { focusStage, focusedStageId } = useCortexStore();
  const isFocused = focusedStageId === column.id;
  const isAtWipLimit = column.wipLimit ? tasks.length >= column.wipLimit : false;

  const getColumnIcon = (iconName: string, color: string) => {
    const props = { className: 'h-3.5 w-3.5 shrink-0', style: { color } };
    switch (iconName) {
      case 'Inbox':
        return <Inbox {...props} />;
      case 'ListTodo':
        return <ListTodo {...props} />;
      case 'Code2':
        return <Code2 {...props} />;
      case 'ShieldCheck':
        return <ShieldCheck {...props} />;
      case 'Sparkles':
        return <Sparkles {...props} />;
      case 'CheckCircle2':
        return <CheckCircle2 {...props} />;
      default:
        return <Layers {...props} />;
    }
  };

  return (
    <div
      className={`flex flex-col w-76 shrink-0 h-full rounded-lg bg-obsidian-900 border transition-all duration-150 select-none overflow-hidden ${
        isFocused
          ? 'border-cyber-cyan shadow-sm shadow-cyan-950/50'
          : 'border-obsidian-800 hover:border-obsidian-750'
      }`}
    >
      {/* Top Hairline Accent Bar */}
      <div
        className="h-[2px] w-full shrink-0"
        style={{ backgroundColor: column.color }}
      />

      {/* Column Header - Tight Developer Ergonomics */}
      <div
        onClick={() => focusStage(column.id)}
        className="px-3 py-2 border-b border-obsidian-800 bg-obsidian-850 hover:bg-obsidian-800/70 transition cursor-pointer group"
        title="Click to inspect Agent Mind in Cortex Explorer"
      >
        {/* Title, Icon and WIP Counter */}
        <div className="flex items-center justify-between mb-1">
          <div className="flex items-center space-x-2 min-w-0">
            {getColumnIcon(column.icon, column.color)}
            <h3 className="text-xs font-semibold text-slate-100 group-hover:text-white truncate">
              {column.title}
            </h3>
          </div>

          <div className="flex items-center space-x-1">
            {/* WIP Counter Pill */}
            <span
              className={`px-1.5 py-0.2 rounded font-mono text-[10px] font-bold tabular-nums border ${
                isAtWipLimit
                  ? 'bg-rose-950/40 text-rose-300 border-rose-500/50 animate-pulse'
                  : 'bg-obsidian-950 text-slate-400 border-obsidian-750'
              }`}
              title={column.wipLimit ? `WIP Limit: ${tasks.length} / ${column.wipLimit}` : `${tasks.length} tasks`}
            >
              {tasks.length}
              {column.wipLimit ? ` / ${column.wipLimit}` : ''}
            </span>

            {/* Quick Add Card */}
            {onAddTask && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onAddTask(column.id);
                }}
                className="p-1 rounded text-slate-400 hover:text-white hover:bg-obsidian-750 transition cursor-pointer"
                title="Add Task to this stage"
              >
                <Plus className="h-3 w-3" />
              </button>
            )}
          </div>
        </div>

        {/* Sub-Header: Gatekeeper Sensor & Context Limit Pill */}
        <div className="flex items-center justify-between text-[10px] font-mono text-slate-400">
          {/* Gatekeeper Sensor Indicator */}
          <div className="flex items-center space-x-1 min-w-0 truncate">
            {column.gatekeeperSensor ? (
              <span
                className="flex items-center space-x-1 px-1.5 py-0.2 rounded bg-obsidian-950 border border-obsidian-800 text-slate-300 truncate"
                title={`Sensor Gatekeeper: ${column.gatekeeperSensor}`}
              >
                <ShieldCheck className="h-2.5 w-2.5 text-cyber-emerald shrink-0" />
                <span className="truncate">{column.gatekeeperSensor}</span>
              </span>
            ) : (
              <span className="text-slate-600">Unrestricted</span>
            )}
          </div>

          {/* Context Limit Pill */}
          <div
            className="flex items-center space-x-1 px-1.5 py-0.2 rounded bg-obsidian-950 border border-obsidian-800 text-slate-400 shrink-0"
            title={`Context Window: ${column.contextWindow.toLocaleString()} tokens`}
          >
            <Cpu className="h-2.5 w-2.5 text-cyber-cyan" />
            <span className="tabular-nums font-medium text-slate-300">
              {formatTokens(column.contextWindow)}
            </span>
          </div>
        </div>
      </div>

      {/* Cards Drop Area (Droppable) */}
      <Droppable droppableId={column.id}>
        {(provided, snapshot) => (
          <div
            ref={provided.innerRef}
            {...provided.droppableProps}
            className={`flex-1 p-2 overflow-y-auto min-h-[140px] transition-colors ${
              snapshot.isDraggingOver
                ? 'bg-obsidian-850/80 border-2 border-dashed border-cyan-500/40 rounded-b-lg'
                : 'bg-obsidian-950/30'
            }`}
          >
            {tasks.map((task, index) => (
              <KanbanCard key={task.id} task={task} index={index} />
            ))}
            {provided.placeholder}

            {tasks.length === 0 && !snapshot.isDraggingOver && (
              <div className="h-24 flex flex-col items-center justify-center border border-dashed border-obsidian-800 rounded-md text-slate-600 text-[11px] font-mono p-3 text-center">
                <span>No active agents</span>
                <span className="text-[10px] text-slate-600 mt-0.5">Drop task to dispatch</span>
              </div>
            )}
          </div>
        )}
      </Droppable>
    </div>
  );
}

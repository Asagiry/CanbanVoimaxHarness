import { Droppable } from '@hello-pangea/dnd';
import { PipelineColumn, KanbanTask } from '../../types';
import KanbanCard from '../card/KanbanCard';
import { useCortexStore } from '../../stores/cortexStore';
import {
  Inbox,
  ListTodo,
  Code2,
  ShieldCheck,
  Sparkles,
  CheckCircle2,
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

  const getColumnIcon = (iconName: string) => {
    const props = { className: 'h-3.5 w-3.5 shrink-0 text-[#171717]' };
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
      className={`flex flex-col w-72 shrink-0 h-full rounded-lg bg-[#f9f9f9] border transition-all duration-150 select-none overflow-hidden ${
        isFocused
          ? 'border-[#171717] shadow-sm'
          : 'border-[#ebebeb] hover:border-[#d4d4d8]'
      }`}
    >
      {/* Top Hairline Accent Bar */}
      <div
        className="h-[2px] w-full shrink-0"
        style={{ backgroundColor: column.color || '#171717' }}
      />

      {/* Column Header */}
      <div
        onClick={() => focusStage(column.id)}
        className="px-3 py-2 border-b border-[#ebebeb] bg-white hover:bg-[#fafafa] transition cursor-pointer group"
      >
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2 min-w-0">
            {getColumnIcon(column.icon)}
            <h3 className="text-xs font-semibold text-[#171717] truncate">
              {column.title}
            </h3>
          </div>

          <div className="flex items-center space-x-1.5">
            {/* Task Counter */}
            <span
              className={`px-1.5 py-0.5 rounded font-mono text-[10px] tabular-nums leading-none border ${
                isAtWipLimit
                  ? 'bg-red-50 text-red-700 border-red-200'
                  : 'bg-[#ebebeb] text-[#171717] border-[#ebebeb]'
              }`}
            >
              {tasks.length}
            </span>

            {/* Quick Add Button */}
            {onAddTask && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onAddTask(column.id);
                }}
                className="p-1 rounded text-[#666666] hover:text-[#171717] hover:bg-[#f4f4f5] transition cursor-pointer"
                title="Добавить задачу"
              >
                <Plus className="h-3 w-3" />
              </button>
            )}
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
                ? 'bg-[#f4f4f5] border border-dashed border-[#171717]/30 rounded-b-lg'
                : 'bg-transparent'
            }`}
          >
            {tasks.map((task, index) => (
              <KanbanCard key={task.id} task={task} index={index} />
            ))}
            {provided.placeholder}
          </div>
        )}
      </Droppable>
    </div>
  );
}

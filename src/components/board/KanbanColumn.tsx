import { Droppable } from '@hello-pangea/dnd';
import { PipelineColumn, KanbanTask } from '../../types';
import KanbanCard from '../card/KanbanCard';
import { useCortexStore } from '../../stores/cortexStore';
import { Plus } from 'lucide-react';

interface KanbanColumnProps {
  column: PipelineColumn;
  tasks: KanbanTask[];
  onAddTask?: (columnId: string) => void;
}

export default function KanbanColumn({ column, tasks, onAddTask }: KanbanColumnProps) {
  const { focusStage, focusedStageId } = useCortexStore();
  const isFocused = focusedStageId === column.id;

  return (
    <div
      className={`flex flex-col w-72 shrink-0 h-full rounded border bg-obsidian-900 transition-colors select-none ${
        isFocused ? 'border-slate-500' : 'border-obsidian-800'
      }`}
    >
      {/* Column Header */}
      <div
        onClick={() => focusStage(column.id)}
        className="px-3 py-2 border-b border-obsidian-800 bg-obsidian-850 flex items-center justify-between cursor-pointer"
        title="Инструкции роли в проводнике"
      >
        <div className="flex items-center space-x-2 min-w-0">
          <span
            className="w-2 h-2 rounded-full shrink-0"
            style={{ backgroundColor: column.color }}
          />
          <h3 className="text-xs font-semibold text-slate-200 truncate">
            {column.title}
          </h3>
          <span className="text-[11px] font-mono text-slate-500 tabular-nums">
            {tasks.length}
          </span>
        </div>

        {onAddTask && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              onAddTask(column.id);
            }}
            className="p-1 rounded text-slate-500 hover:text-slate-200 hover:bg-obsidian-800 transition cursor-pointer"
            title="Создать задачу"
          >
            <Plus className="h-3.5 w-3.5" />
          </button>
        )}
      </div>

      {/* Cards Area */}
      <Droppable droppableId={column.id}>
        {(provided, snapshot) => (
          <div
            ref={provided.innerRef}
            {...provided.droppableProps}
            className={`flex-1 p-2 overflow-y-auto min-h-[140px] transition-colors ${
              snapshot.isDraggingOver ? 'bg-obsidian-850/60' : 'bg-transparent'
            }`}
          >
            {tasks.map((task, index) => (
              <KanbanCard key={task.id} task={task} index={index} />
            ))}
            {provided.placeholder}

            {tasks.length === 0 && !snapshot.isDraggingOver && (
              <div className="h-20 flex items-center justify-center border border-dashed border-obsidian-800/80 rounded text-slate-600 text-[11px] font-mono">
                Нет задач
              </div>
            )}
          </div>
        )}
      </Droppable>
    </div>
  );
}

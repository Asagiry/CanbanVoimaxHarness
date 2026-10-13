import { useState } from 'react';
import { Draggable, Droppable } from '@hello-pangea/dnd';
import { PipelineColumn, KanbanTask } from '../../types';
import KanbanCard from '../card/KanbanCard';
import ColumnContextMenu from './ColumnContextMenu';
import { useCortexStore } from '../../stores/cortexStore';
import {
  Inbox,
  ListTodo,
  Code2,
  ShieldCheck,
  Sparkles,
  CheckCircle2,
  Layers,
} from 'lucide-react';

interface KanbanColumnProps {
  column: PipelineColumn;
  tasks: KanbanTask[];
  index: number;
  onDeleteColumn?: (columnId: string) => void;
}

export default function KanbanColumn({ column, tasks, index, onDeleteColumn }: KanbanColumnProps) {
  const { focusStage, focusedStageId } = useCortexStore();
  const isFocused = focusedStageId === column.id;
  const isAtWipLimit = column.wipLimit ? tasks.length >= column.wipLimit : false;
  const [menuPos, setMenuPos] = useState<{ x: number; y: number } | null>(null);

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
    <Draggable draggableId={column.id} index={index}>
      {(colProvided, colSnapshot) => (
    <div
      ref={colProvided.innerRef}
      {...colProvided.draggableProps}
      className={`flex flex-col w-72 shrink-0 h-full rounded-lg bg-[#f9f9f9] border transition-all duration-150 select-none overflow-hidden ${
        colSnapshot.isDragging
          ? 'border-[#171717] shadow-md opacity-90'
          : isFocused
            ? 'border-[#171717] shadow-sm'
            : 'border-[#ebebeb] hover:border-[#d4d4d8]'
      }`}
    >
      {/* Top Hairline Accent Bar */}
      <div
        className="h-[2px] w-full shrink-0"
        style={{ backgroundColor: column.color || '#171717' }}
      />

      {/* Column Header (drag handle для смены порядка колонок; ПКМ — действия с колонкой) */}
      <div
        {...colProvided.dragHandleProps}
        onClick={() => focusStage(column.id)}
        onContextMenu={(e) => {
          e.preventDefault();
          e.stopPropagation();
          setMenuPos({ x: e.clientX, y: e.clientY });
        }}
        title="Перетащите для смены порядка колонок. Правая кнопка — действия с колонкой"
        className="flex items-center h-9 px-3 border-b border-[#ebebeb] bg-white hover:bg-[#fafafa] transition cursor-grab active:cursor-grabbing group overflow-hidden"
      >
        <div className="flex items-center gap-2 min-w-0 w-full overflow-hidden">
          {getColumnIcon(column.icon)}
          <h3 className="text-xs font-semibold text-[#171717] truncate leading-none">
            {column.title}
          </h3>
          {/* Task Counter */}
          <span
            className={`text-[10px] font-mono tabular-nums whitespace-nowrap leading-none shrink-0 ${
              isAtWipLimit ? 'text-red-600' : 'text-[#8f8f8f]'
            }`}
          >
            Всего задач: {tasks.length}
          </span>
        </div>
      </div>

      {/* Cards Drop Area (Droppable; type TASK отделяет задачи от перетаскивания колонок) */}
      <Droppable droppableId={column.id} type="TASK">
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
      {menuPos && (
        <ColumnContextMenu
          column={column}
          x={menuPos.x}
          y={menuPos.y}
          onClose={() => setMenuPos(null)}
          onDeleteColumn={onDeleteColumn}
        />
      )}
    </div>
      )}
    </Draggable>
  );
}

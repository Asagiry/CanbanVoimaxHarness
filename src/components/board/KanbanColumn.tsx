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
  totalColumns?: number;
  isPinned?: boolean;
  onDeleteColumn?: (columnId: string) => void;
}

export default function KanbanColumn({
  column,
  tasks,
  index,
  isPinned: isPinnedProp,
  onDeleteColumn,
}: KanbanColumnProps) {
  // Клик по шапке больше не подсвечивает колонку и не трогает проводник
  // (фокус из проводника в доску оставлен, обратного нет).
  const { focusedStageId } = useCortexStore();
  const isFocused = focusedStageId === column.id;
  const isDoneColumn = Boolean(
    column.terminal ||
    column.id === 'ready' ||
    column.title.toLowerCase().includes('готов') ||
    column.title.toLowerCase().includes('done')
  );
  // В колонке "Done" отображается ТОЛЬКО корневая основная задача (CVH-012)
  const displayTasks = isDoneColumn ? tasks.filter((t) => !t.parentId) : tasks;
  const isAtWipLimit = column.wipLimit ? displayTasks.length >= column.wipLimit : false;
  const [menuPos, setMenuPos] = useState<{ x: number; y: number } | null>(null);

  // Закреплены только backlog/ready/terminal по id/флагу (без привязки к индексу).
  // index===0 — лишь fallback, когда id бэклога неизвестен. По index===last не пиним.
  const isPinned = Boolean(
    isPinnedProp ?? (
      column.id === 'backlog' ||
      column.id === 'ready' ||
      Boolean(column.terminal) ||
      index === 0
    )
  );

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
    <Draggable draggableId={column.id} index={index} isDragDisabled={isPinned}>
      {(colProvided, colSnapshot) => (
    <div
      ref={colProvided.innerRef}
      {...colProvided.draggableProps}
      style={
        isPinned
          ? { ...colProvided.draggableProps.style, transform: 'none' }
          : colProvided.draggableProps.style
      }
      className={`flex flex-col w-72 shrink-0 h-full rounded-lg bg-[#f9f9f9] border select-none overflow-hidden transition-[border-color,box-shadow,opacity] duration-150 ${
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
        {...(!isPinned ? colProvided.dragHandleProps : {})}
        onContextMenu={(e) => {
          e.preventDefault();
          e.stopPropagation();
          setMenuPos({ x: e.clientX, y: e.clientY });
        }}
        title={
          isPinned
            ? 'Колонка закреплена. Правая кнопка — действия с колонкой'
            : 'Перетащите для смены порядка колонок. Правая кнопка — действия с колонкой'
        }
        className={`flex items-center h-9 px-3 border-b border-[#ebebeb] bg-white hover:bg-[#fafafa] transition-colors ${
          isPinned ? 'cursor-default' : 'cursor-grab active:cursor-grabbing'
        } group overflow-hidden`}
      >
        <div className="flex items-center gap-2 min-w-0 w-full overflow-hidden">
          {getColumnIcon(column.icon)}
          <h3 className="text-[13px] font-semibold text-[#171717] truncate leading-none">
            {column.title}
          </h3>
          {/* Task Counter (same baseline as the title) */}
          <span
            className={`text-[10px] font-mono tabular-nums whitespace-nowrap leading-none shrink-0 translate-y-[1px] ${
              isAtWipLimit ? 'text-red-600' : 'text-[#8f8f8f]'
            }`}
          >
            Всего задач: {displayTasks.length}
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
            {displayTasks.map((task, index) => (
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

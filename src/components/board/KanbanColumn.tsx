import { useState, useRef, useEffect, memo } from 'react';
import { Draggable, Droppable } from '@hello-pangea/dnd';
import { ChevronDown } from 'lucide-react';
import { PipelineColumn, KanbanTask } from '../../types';
import KanbanCard from '../card/KanbanCard';
import ColumnContextMenu from './ColumnContextMenu';
import { useCortexStore } from '../../stores/cortexStore';
import AgentIcon from '../common/AgentIcon';

const PAGE_SIZE = 10;

interface KanbanColumnProps {
  column: PipelineColumn;
  tasks: KanbanTask[];
  index: number;
  totalColumns?: number;
  isPinned?: boolean;
  onDeleteColumn?: (columnId: string) => void;
}

function KanbanColumn({
  column,
  tasks,
  index,
  isPinned: isPinnedProp,
  onDeleteColumn,
}: KanbanColumnProps) {
  // Клик по шапке больше не подсвечивает колонку и не трогает проводник
  // (фокус из проводника в доску оставлен, обратного нет).
  const isFocused = useCortexStore((s) => s.focusedStageId === column.id);
  const assignedPersona = useCortexStore((s) =>
    column.persona ? s.agentPersonas.find((p) => p.role === column.persona) : undefined
  );
  const effectiveIcon = column.icon || assignedPersona?.icon || 'Layers';
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

  // Progressive card loading state
  const [visibleCount, setVisibleCount] = useState<number>(PAGE_SIZE);

  // Track tasks to detect newly inserted items at or beyond the visible boundary
  const prevTasksRef = useRef<KanbanTask[]>(tasks);
  useEffect(() => {
    const prevIds = new Set(prevTasksRef.current.map((t) => t.id));
    const addedTask = displayTasks.find((t) => !prevIds.has(t.id));
    if (addedTask) {
      const addedIndex = displayTasks.findIndex((t) => t.id === addedTask.id);
      if (addedIndex >= visibleCount) {
        setVisibleCount((prev) => Math.max(prev, Math.ceil((addedIndex + 1) / PAGE_SIZE) * PAGE_SIZE));
      }
    }
    prevTasksRef.current = tasks;
  }, [tasks, displayTasks, visibleCount]);

  const visibleTasks = displayTasks.slice(0, visibleCount);
  const hasMore = visibleCount < displayTasks.length;
  const remaining = displayTasks.length - visibleCount;
  const nextCount = Math.min(PAGE_SIZE, remaining);

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
      className={`flex flex-col w-72 shrink-0 h-full rounded-lg bg-[#f9f9f9] dark:bg-[#0d1117] border select-none overflow-hidden transition-[border-color,box-shadow,opacity] duration-150 ${
        colSnapshot.isDragging
          ? 'border-[#171717] dark:border-[#30363d] shadow-md opacity-90'
          : isFocused
            ? 'border-[#171717] dark:border-[#30363d] shadow-sm'
            : 'border-[#ebebeb] dark:border-[#21262d] hover:border-[#d4d4d8] dark:hover:border-[#30363d]'
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
        className={`flex items-center h-9 px-3 border-b border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#161b22] hover:bg-[#fafafa] dark:hover:bg-[#1c2128] transition-colors ${
          isPinned ? 'cursor-default' : 'cursor-grab active:cursor-grabbing'
        } group overflow-hidden`}
      >
        <div className="flex items-center gap-2 min-w-0 w-full overflow-hidden">
          <AgentIcon name={effectiveIcon} className="h-3.5 w-3.5 shrink-0 text-[#171717] dark:text-[#f0f6fc]" />
          <h3 className="text-[13px] font-semibold text-[#171717] dark:text-[#f0f6fc] truncate leading-none">
            {column.title}
          </h3>
          {/* Task Counter (same baseline as the title) */}
          <span
            className={`text-[10px] font-mono tabular-nums whitespace-nowrap leading-none shrink-0 translate-y-[1px] ${
              isAtWipLimit ? 'text-red-600 dark:text-red-400' : 'text-[#8f8f8f] dark:text-[#8b949e]'
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
            className={`flex-1 min-h-0 p-2 overflow-y-auto transition-colors ${
              snapshot.isDraggingOver
                ? 'bg-[#f4f4f5] dark:bg-[#161b22]/50 border border-dashed border-[#171717]/30 dark:border-[#30363d] rounded-b-lg'
                : 'bg-transparent'
            }`}
          >
            {visibleTasks.map((task, index) => (
              <KanbanCard key={task.id} task={task} index={index} />
            ))}
            {provided.placeholder}
            {hasMore && (
              <button
                type="button"
                onClick={() => setVisibleCount((prev) => prev + PAGE_SIZE)}
                className="w-full mt-1 mb-2 py-2 px-3 flex items-center justify-center gap-1.5 rounded-[6px] border border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#161b22] hover:bg-[#f9f9f9] dark:hover:bg-[#21262d] text-[#0d0d0d] dark:text-[#f0f6fc] font-mono text-xs font-medium transition-all duration-150 cursor-pointer shadow-xs active:bg-[#f4f4f5] dark:active:bg-[#1c2128] select-none"
              >
                <ChevronDown className="h-3.5 w-3.5 text-[#8f8f8f] dark:text-[#8b949e] transition-transform duration-150" />
                <span>Загрузить еще (еще {nextCount} из {remaining})</span>
              </button>
            )}
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

function areColumnPropsEqual(prev: KanbanColumnProps, next: KanbanColumnProps): boolean {
  if (prev.index !== next.index) return false;
  if (prev.isPinned !== next.isPinned) return false;
  if (prev.totalColumns !== next.totalColumns) return false;
  if (prev.onDeleteColumn !== next.onDeleteColumn) return false;

  // Compare column definition properties
  const cA = prev.column;
  const cB = next.column;
  if (
    cA.id !== cB.id ||
    cA.title !== cB.title ||
    cA.color !== cB.color ||
    cA.icon !== cB.icon ||
    cA.persona !== cB.persona ||
    cA.wipLimit !== cB.wipLimit ||
    cA.terminal !== cB.terminal
  ) {
    return false;
  }

  // Shallow comparison of tasks array
  if (prev.tasks === next.tasks) return true;
  if (prev.tasks.length !== next.tasks.length) return false;

  for (let i = 0; i < prev.tasks.length; i++) {
    if (prev.tasks[i] !== next.tasks[i]) {
      return false;
    }
  }

  return true;
}

export default memo(KanbanColumn, areColumnPropsEqual);

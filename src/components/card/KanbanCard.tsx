import { useState } from 'react';
import { Draggable } from '@hello-pangea/dnd';
import { KanbanTask, TaskStatus } from '../../types';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore, syncTasksBackend } from '../../stores/boardStore';
import { Play, Clock } from 'lucide-react';
import TaskContextMenu from './TaskContextMenu';

function formatStageDuration(totalSeconds: number): string {
  const s = Math.max(0, Math.floor(totalSeconds));
  if (s < 60) return `${s}с`;
  const minutesTotal = Math.floor(s / 60);
  if (minutesTotal < 60) return `${minutesTotal}м`;
  const hoursTotal = Math.floor(minutesTotal / 60);
  if (hoursTotal < 24) {
    const m = minutesTotal % 60;
    return m > 0 ? `${hoursTotal}ч ${m}м` : `${hoursTotal}ч`;
  }
  const days = Math.floor(hoursTotal / 24);
  const h = hoursTotal % 24;
  return `${days}д ${h}ч`;
}

interface KanbanCardProps {
  task: KanbanTask;
  index: number;
}

export default function KanbanCard({ task, index }: KanbanCardProps) {
  const { openDrawer } = useDrawerStore();
  const { runTaskAgent, columns } = useBoardStore();
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);
  const taskColumn = columns.find((c) => c.id === task.columnId);
  const canRunAgent = Boolean(taskColumn?.persona);

  const handleRunAgent = async (e: React.MouseEvent) => {
    e.stopPropagation();
    await syncTasksBackend(useBoardStore.getState().tasks);
    await runTaskAgent(task.id);
  };

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY });
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
    <>
      <Draggable draggableId={task.id} index={index}>
        {(provided, snapshot) => (
          <div
            ref={provided.innerRef}
            {...provided.draggableProps}
            {...provided.dragHandleProps}
            onClick={() => openDrawer(task.id)}
            onContextMenu={handleContextMenu}
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
              {canRunAgent && (
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

          {/* Footer: модель (строка 1), ветка + время в колонке (строка 2) */}
          <div className="pt-2 border-t border-[#ebebeb]">
            <div
              className="truncate font-mono text-[10px] text-[#666666]"
              title={task.model ? `Модель: ${task.model}` : 'Модель не задана'}
            >
              {task.model || 'модель не задана'}
            </div>
            <div className="mt-1 flex items-center justify-between gap-2 font-mono text-[10px]">
              <span
                className="truncate text-[#8f8f8f] min-w-0"
                title={task.branch ? `Ветка: ${task.branch}` : 'Ветка не задана'}
              >
                {task.branch || 'без ветки'}
              </span>
              <span
                title="Время в текущей колонке"
                className="inline-flex items-center gap-1 text-[#8f8f8f] tabular-nums whitespace-nowrap shrink-0"
              >
                <Clock className="h-3 w-3 shrink-0" />
                <span>
                  {(task.activeStageDuration ?? 0) <= 0
                    ? 'только что'
                    : `в колонке ${formatStageDuration(task.activeStageDuration ?? 0)}`}
                </span>
              </span>
            </div>
          </div>
        </div>
      )}
    </Draggable>
      {contextMenu && (
        <TaskContextMenu
          task={task}
          x={contextMenu.x}
          y={contextMenu.y}
          onClose={() => setContextMenu(null)}
        />
      )}
    </>
  );
}

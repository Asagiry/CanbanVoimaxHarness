import { useState } from 'react';
import { Draggable } from '@hello-pangea/dnd';
import { KanbanTask, TaskStatus } from '../../types';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore, syncTasksBackend } from '../../stores/boardStore';
import { Play, Clock, GitBranch, ChevronDown, MessageSquare, GitFork } from 'lucide-react';
import TaskContextMenu from './TaskContextMenu';

function formatCardDate(ts?: number): string {
  if (!ts || !Number.isFinite(ts)) return '';
  const d = new Date(ts);
  const dd = String(d.getDate()).padStart(2, '0');
  const mm = String(d.getMonth() + 1).padStart(2, '0');
  const hh = String(d.getHours()).padStart(2, '0');
  const min = String(d.getMinutes()).padStart(2, '0');
  return `${dd}.${mm} ${hh}:${min}`;
}

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
  const { runTaskAgent, columns, hoveredTaskId, setHoveredTaskId, tasks } = useBoardStore();
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);
  const [isSubtasksOpen, setIsSubtasksOpen] = useState(false);

  const taskColumn = columns.find((c) => c.id === task.columnId);
  const canRunAgent = Boolean(taskColumn?.persona);
  const isDone = Boolean(
    task.status === 'done' ||
    taskColumn?.terminal === true ||
    task.columnId === 'ready' ||
    taskColumn?.title?.toLowerCase().includes('готов') ||
    taskColumn?.title?.toLowerCase().includes('done')
  );

  // Subtasks belonging to this task
  const subtasks = tasks.filter((t) => t.parentId === task.id);

  // Check if this card is highlighted via hover on itself, its subtask, or its parent
  const isHighlighted =
    hoveredTaskId === task.id ||
    (task.waitingForSubtaskId && hoveredTaskId === task.waitingForSubtaskId) ||
    (task.parentId && hoveredTaskId === task.parentId);

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
    if (task.waitingForSubtaskId || status === 'waiting_subtask') {
      return (
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            if (task.waitingForSubtaskId) openDrawer(task.waitingForSubtaskId);
          }}
          className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#b45309] bg-[#fffbeb] border border-[#fde68a] hover:bg-[#fef3c7] hover:border-[#f59e0b] transition cursor-pointer"
          title={task.waitingForSubtaskId ? `Открыть подзадачу ${task.waitingForSubtaskId}` : 'Ожидает завершения подзадачи'}
        >
          <span className="h-1.5 w-1.5 rounded-full bg-[#f59e0b] animate-pulse" />
          <span>Ждем подзадачу {task.waitingForSubtaskId || ''}</span>
        </button>
      );
    }

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
            id={`task-card-${task.id}`}
            data-task-id={task.id}
            ref={provided.innerRef}
            {...provided.draggableProps}
            {...provided.dragHandleProps}
            onClick={() => openDrawer(task.id)}
            onContextMenu={handleContextMenu}
            onMouseEnter={() => setHoveredTaskId(task.id)}
            onMouseLeave={() => setHoveredTaskId(null)}
            className={`group relative mb-2 rounded-md border p-3 cursor-pointer select-none transition duration-150 active:scale-[0.99] ${
              snapshot.isDragging
                ? 'border-[#171717] shadow-md bg-white z-40'
                : isHighlighted
                ? 'border-[#8b5cf6] shadow-md ring-1 ring-[#8b5cf6]/40 bg-white'
                : 'border-[#ebebeb] bg-white hover:border-[#d4d4d8] shadow-xs'
            }`}
          >
            {/* Header: ID, Play Button, Parent link & Concise Status */}
            <div className="flex items-center justify-between mb-2 gap-1.5 flex-wrap">
              <div className="flex items-center space-x-1.5">
                <span className="font-mono text-[10px] font-semibold text-[#171717] bg-[#f4f4f5] px-1.5 py-0.5 rounded border border-[#ebebeb] tabular-nums">
                  {task.id}
                </span>

                {/* Subtask link to parent */}
                {task.parentId && (
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      openDrawer(task.parentId!);
                    }}
                    className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#6366f1] bg-[#eef2ff] border border-[#c7d2fe] hover:bg-[#e0e7ff] transition cursor-pointer"
                    title={`Родительская задача: ${task.parentId}`}
                  >
                    <GitBranch className="h-2.5 w-2.5 text-[#6366f1]" />
                    <span>{task.parentId}</span>
                  </button>
                )}

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
              {(task.createdAt || (task.totalElapsed ?? 0) > 0) && (
                <div className="mt-1 flex items-center gap-1 font-mono text-[10px] text-[#8f8f8f] tabular-nums whitespace-nowrap">
                  {task.createdAt ? (
                    <span title="Дата создания задачи">созд. {formatCardDate(task.createdAt)}</span>
                  ) : null}
                  {task.createdAt && (task.totalElapsed ?? 0) > 0 ? <span>·</span> : null}
                  {(task.totalElapsed ?? 0) > 0 ? (
                    <span title="Общее время выполнения задачи">
                      идёт {formatStageDuration(task.totalElapsed ?? 0)}
                    </span>
                  ) : null}
                </div>
              )}
            </div>

            {/* In Done column: Collapsible list of completed subtasks with dialogue open */}
            {isDone && subtasks.length > 0 && (
              <div className="mt-2.5 pt-2 border-t border-[#ebebeb]">
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    setIsSubtasksOpen(!isSubtasksOpen);
                  }}
                  className="flex items-center justify-between w-full text-[10px] font-mono font-medium text-[#4d4d4d] hover:text-[#171717] transition cursor-pointer py-1 px-1 rounded hover:bg-[#f4f4f5]"
                  title="Раскрыть список завершенных этапов и подзадач"
                >
                  <span className="flex items-center gap-1.5">
                    <GitFork className="h-3 w-3 text-[#8b5cf6]" />
                    <span>Завершенные подзадачи ({subtasks.length})</span>
                  </span>
                  <ChevronDown
                    className={`h-3 w-3 transition-transform duration-150 ${
                      isSubtasksOpen ? 'rotate-180' : ''
                    }`}
                  />
                </button>

                {isSubtasksOpen && (
                  <div className="mt-1 space-y-1">
                    {subtasks.map((sub) => {
                      const subId = sub?.id ?? '';
                      const subSuffix =
                        sub?.subtaskIndex != null
                          ? `.${sub.subtaskIndex}`
                          : subId.split('.').pop()
                          ? `.${subId.split('.').pop()}`
                          : '';
                      const subCol = columns.find((c) => c.id === sub?.columnId);
                      const stageTitle = subCol?.title || sub?.columnId || '';

                      return (
                          <div
                            key={subId || Math.random().toString(36)}
                            onClick={(e) => {
                              e.stopPropagation();
                              if (subId) openDrawer(subId);
                            }}
                          className="flex items-center justify-between p-1.5 rounded bg-[#f8f9fa] hover:bg-[#f1f3f5] border border-[#e9ecef] cursor-pointer transition text-[10px] font-mono group"
                          title={`Открыть диалог и мысли подзадачи ${sub.id}`}
                        >
                          <div className="flex items-center gap-1.5 truncate min-w-0">
                            <span className="font-semibold text-[#8b5cf6] shrink-0">
                              {subSuffix} {stageTitle}
                            </span>
                            <span className="text-[#666666] truncate">
                              {(sub?.title ?? '').replace(/^\[.*?\]\s*/, '')}
                            </span>
                          </div>
                          <div className="flex items-center gap-1.5 shrink-0 ml-2">
                            {sub?.verdict && sub.verdict !== 'NONE' ? (
                              <span
                                className={`px-1 py-0.2 rounded text-[9px] font-mono ${
                                  sub?.verdict === 'APPROVED'
                                    ? 'bg-emerald-50 text-emerald-700 border border-emerald-200'
                                    : sub?.verdict === 'APPROVED_WITH_CAVEATS'
                                    ? 'bg-amber-50 text-amber-700 border border-amber-200'
                                    : 'bg-rose-50 text-rose-700 border border-rose-200'
                                }`}
                              >
                                {sub?.verdict === 'APPROVED'
                                  ? 'OK'
                                  : sub?.verdict === 'APPROVED_WITH_CAVEATS'
                                  ? 'CAVEATS'
                                  : 'REWORK'}
                              </span>
                            ) : (
                              <span className="px-1 py-0.2 rounded text-[9px] font-mono bg-[#f4f4f5] text-[#666666] border border-[#ebebeb]">
                                Готово
                              </span>
                            )}
                            <MessageSquare className="h-3 w-3 text-[#8f8f8f] group-hover:text-[#171717] transition" />
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            )}
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

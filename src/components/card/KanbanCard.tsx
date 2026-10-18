import { useState } from 'react';
import { Draggable } from '@hello-pangea/dnd';
import { KanbanTask, TaskStatus } from '../../types';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore, syncTasksBackend } from '../../stores/boardStore';
import { Play, Clock, GitBranch, ChevronDown, MessageSquare, GitFork, Sparkles } from 'lucide-react';
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
  const sec = s % 60;
  if (minutesTotal < 60) {
    return sec > 0 ? `${minutesTotal}м ${sec}с` : `${minutesTotal}м`;
  }
  const hoursTotal = Math.floor(minutesTotal / 60);
  const min = minutesTotal % 60;
  if (hoursTotal < 24) {
    return min > 0 ? `${hoursTotal}ч ${min}м` : `${hoursTotal}ч`;
  }
  const days = Math.floor(hoursTotal / 24);
  const h = hoursTotal % 24;
  return h > 0 ? `${days}д ${h}ч` : `${days}д`;
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

  const stageStart = task.stageEnteredAt || task.updatedAt || task.createdAt;
  const currentStageDuration = isDone
    ? (task.activeStageDuration ?? 0)
    : stageStart
    ? Math.max(0, Math.floor((Date.now() - stageStart) / 1000))
    : (task.activeStageDuration ?? 0);

  const currentTotalElapsed = isDone
    ? (task.totalElapsed ?? 0)
    : task.createdAt
    ? Math.max(0, Math.floor((Date.now() - task.createdAt) / 1000))
    : (task.totalElapsed ?? 0);

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
          className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#b45309] dark:text-[#fbbf24] bg-[#fffbeb] dark:bg-[#451a03]/40 border border-[#fde68a] dark:border-[#92400e] hover:bg-[#fef3c7] dark:hover:bg-[#451a03]/60 hover:border-[#f59e0b] transition cursor-pointer"
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
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#15803d] dark:text-[#4ade80] bg-[#f0fdf4] dark:bg-[#052e16]/40 border border-[#bbf7d0] dark:border-[#166534]">
            <span className="h-1.5 w-1.5 rounded-full bg-[#15803d] dark:bg-[#4ade80] animate-pulse" />
            <span>В работе</span>
          </span>
        );
      case 'paused':
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#b45309] dark:text-[#fbbf24] bg-[#fffbeb] dark:bg-[#451a03]/40 border border-[#fde68a] dark:border-[#92400e]">
            <span className="h-1.5 w-1.5 rounded-full bg-[#b45309] dark:bg-[#fbbf24]" />
            <span>Пауза</span>
          </span>
        );
      case 'waiting_review':
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#7e22ce] dark:text-[#c084fc] bg-[#faf5ff] dark:bg-[#3b0764]/40 border border-[#e9d5ff] dark:border-[#7e22ce]">
            <span>Ревью</span>
          </span>
        );
      case 'done':
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#15803d] dark:text-[#4ade80] bg-[#f0fdf4] dark:bg-[#052e16]/40 border border-[#bbf7d0] dark:border-[#166534]">
            <span>Готово</span>
          </span>
        );
      case 'blocked':
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#b91c1c] dark:text-[#f87171] bg-[#fef2f2] dark:bg-[#450a0a]/40 border border-[#fecaca] dark:border-[#991b1b]">
            <span>Блок</span>
          </span>
        );
      case 'idle':
      default:
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#666666] dark:text-[#8b949e] bg-[#f4f4f5] dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#21262d]">
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
                ? 'border-[#171717] dark:border-[#30363d] shadow-md bg-white dark:bg-[#161b22] z-40'
                : isHighlighted
                ? 'border-[#8b5cf6] shadow-md ring-1 ring-[#8b5cf6]/40 bg-white dark:bg-[#161b22]'
                : 'border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#161b22] hover:border-[#d4d4d8] dark:hover:border-[#30363d] shadow-xs'
            }`}
          >
            {/* Header: ID, Play Button, Parent link & Concise Status */}
            <div className="flex items-center justify-between mb-2 gap-1.5 flex-wrap">
              <div className="flex items-center space-x-1.5">
                <span className="font-mono text-[10px] font-semibold text-[#171717] dark:text-[#f0f6fc] bg-[#f4f4f5] dark:bg-[#0d1117] px-1.5 py-0.5 rounded border border-[#ebebeb] dark:border-[#30363d] tabular-nums">
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
                    className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#6366f1] dark:text-[#a5b4fc] bg-[#eef2ff] dark:bg-[#312e81]/40 border border-[#c7d2fe] dark:border-[#4338ca] hover:bg-[#e0e7ff] dark:hover:bg-[#3730a3]/50 transition cursor-pointer"
                    title={`Родительская задача: ${task.parentId}`}
                  >
                    <GitBranch className="h-2.5 w-2.5 text-[#6366f1] dark:text-[#a5b4fc]" />
                    <span>{task.parentId}</span>
                  </button>
                )}

                {canRunAgent && (
                  <button
                    type="button"
                    onClick={handleRunAgent}
                    className="flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#171717] dark:text-[#f0f6fc] bg-[#f4f4f5] dark:bg-[#0d1117] hover:bg-[#171717] dark:hover:bg-[#21262d] hover:text-white border border-[#ebebeb] dark:border-[#30363d] transition cursor-pointer"
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
            <h4 className="text-xs font-medium text-[#171717] dark:text-[#f0f6fc] leading-snug mb-2 font-sans break-words">
              {task.title}
            </h4>



            {/* Relay Handoff / Handover Summary Preview (if present) */}
            {task.handoverSummary && (
              <div
                className="mb-2 flex items-start gap-1.5 p-1.5 rounded-md bg-[#f5f3ff] dark:bg-[#2e1065]/40 border border-[#ddd6fe] dark:border-[#581c87] text-[10px] text-[#5b21b6] dark:text-[#d8b4fe] font-mono select-none"
                title={`Эстафета пайплайна:\n${task.handoverSummary}`}
                onClick={(e) => e.stopPropagation()}
              >
                <Sparkles className="h-3 w-3 text-[#7c3aed] dark:text-[#a855f7] shrink-0 mt-0.5" />
                <div className="truncate min-w-0">
                  <span className="font-semibold text-[#6d28d9] dark:text-[#c084fc]">Эстафета: </span>
                  <span className="text-[#4c1d95] dark:text-[#e9d5ff]">{task.handoverSummary}</span>
                </div>
              </div>
            )}

            {/* Footer: модель (строка 1, если задана), ветка + время в колонке (строка 2) */}
            <div className="pt-2 border-t border-[#ebebeb] dark:border-[#21262d]">
              {task.model && task.model.trim().length > 0 && (
                <div
                  className="truncate font-mono text-[10px] text-[#666666] dark:text-[#8b949e] mb-1"
                  title={`Модель: ${task.model}`}
                >
                  {task.model}
                </div>
              )}
              <div className="flex items-center justify-between gap-2 font-mono text-[10px]">
                <span
                  className="truncate text-[#8f8f8f] dark:text-[#8b949e] min-w-0"
                  title={task.branch ? `Ветка: ${task.branch}` : 'Ветка не задана'}
                >
                  {task.branch || 'без ветки'}
                </span>
                <span
                  title="Время в текущей колонке"
                  className="inline-flex items-center gap-1 text-[#8f8f8f] dark:text-[#8b949e] tabular-nums whitespace-nowrap shrink-0"
                >
                  <Clock className="h-3 w-3 shrink-0" />
                  <span>
                    {currentStageDuration <= 0
                      ? 'только что'
                      : `в колонке ${formatStageDuration(currentStageDuration)}`}
                  </span>
                </span>
              </div>
              {(task.createdAt || currentTotalElapsed > 0) && (
                <div className="mt-1 flex items-center gap-1 font-mono text-[10px] text-[#8f8f8f] dark:text-[#8b949e] tabular-nums whitespace-nowrap">
                  {task.createdAt ? (
                    <span title="Дата создания задачи">созд. {formatCardDate(task.createdAt)}</span>
                  ) : null}
                  {task.createdAt && currentTotalElapsed > 0 ? <span>·</span> : null}
                  {currentTotalElapsed > 0 ? (
                    <span title="Общее время выполнения задачи">
                      идёт {formatStageDuration(currentTotalElapsed)}
                    </span>
                  ) : null}
                </div>
              )}
            </div>

            {/* In Done column: Collapsible list of completed subtasks with dialogue open */}
            {isDone && subtasks.length > 0 && (
              <div className="mt-2.5 pt-2 border-t border-[#ebebeb] dark:border-[#21262d]">
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    setIsSubtasksOpen(!isSubtasksOpen);
                  }}
                  className="flex items-center justify-between w-full text-[10px] font-mono font-medium text-[#4d4d4d] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] transition cursor-pointer py-1 px-1 rounded hover:bg-[#f4f4f5] dark:hover:bg-[#21262d]"
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
                          className="flex items-center justify-between p-1.5 rounded bg-[#f8f9fa] dark:bg-[#0d1117] hover:bg-[#f1f3f5] dark:hover:bg-[#21262d] border border-[#e9ecef] dark:border-[#30363d] cursor-pointer transition text-[10px] font-mono group"
                          title={`Открыть диалог и мысли подзадачи ${sub.id}`}
                        >
                          <div className="flex items-center gap-1.5 truncate min-w-0">
                            <span className="font-semibold text-[#8b5cf6] shrink-0">
                              {subSuffix} {stageTitle}
                            </span>
                            <span className="text-[#666666] dark:text-[#8b949e] truncate">
                              {(sub?.title ?? '').replace(/^\[.*?\]\s*/, '')}
                            </span>
                          </div>
                          <div className="flex items-center gap-1.5 shrink-0 ml-2">
                            {sub?.verdict && sub.verdict !== 'NONE' ? (
                              <span
                                className={`px-1 py-0.2 rounded text-[9px] font-mono ${
                                  sub?.verdict === 'APPROVED'
                                    ? 'bg-emerald-50 dark:bg-[#12281e] text-emerald-700 dark:text-[#7ee787] border border-emerald-200 dark:border-[#238636]'
                                    : sub?.verdict === 'APPROVED_WITH_CAVEATS'
                                    ? 'bg-amber-50 dark:bg-[#422006] text-amber-700 dark:text-[#facc15] border border-amber-200 dark:border-[#713f12]'
                                    : 'bg-rose-50 dark:bg-[#3b1219] text-rose-700 dark:text-[#ff7b72] border border-rose-200 dark:border-[#da3633]'
                                }`}
                              >
                                {sub?.verdict === 'APPROVED'
                                  ? 'OK'
                                  : sub?.verdict === 'APPROVED_WITH_CAVEATS'
                                  ? 'CAVEATS'
                                  : 'REWORK'}
                              </span>
                            ) : (
                              <span className="px-1 py-0.2 rounded text-[9px] font-mono bg-[#f4f4f5] dark:bg-[#161b22] text-[#666666] dark:text-[#8b949e] border border-[#ebebeb] dark:border-[#30363d]">
                                Готово
                              </span>
                            )}
                            <MessageSquare className="h-3 w-3 text-[#8f8f8f] dark:text-[#8b949e] group-hover:text-[#171717] dark:group-hover:text-[#f0f6fc] transition" />
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

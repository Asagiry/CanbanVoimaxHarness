import { useState, memo } from 'react';
import { Draggable } from '@hello-pangea/dnd';
import { KanbanTask, TaskStatus } from '../../types';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore, syncTasksBackend } from '../../stores/boardStore';
import { useShallow } from 'zustand/react/shallow';
import { Play, GitBranch, ChevronDown, MessageSquare, GitFork, CornerDownRight } from 'lucide-react';
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

function formatTimerMMSS(totalSeconds: number): string {
  const s = Math.max(0, Math.floor(totalSeconds));
  const hrs = Math.floor(s / 3600);
  const mins = Math.floor((s % 3600) / 60);
  const secs = s % 60;
  if (hrs > 0) {
    return `${hrs}:${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
  }
  return `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
}

interface KanbanCardProps {
  task: KanbanTask;
  index: number;
}

function KanbanCard({ task, index }: KanbanCardProps) {
  const openDrawer = useDrawerStore((s) => s.openDrawer);
  const runTaskAgent = useBoardStore((s) => s.runTaskAgent);
  const setHoveredTaskId = useBoardStore((s) => s.setHoveredTaskId);
  const columns = useBoardStore((s) => s.columns);
  const isHighlighted = useBoardStore((s) =>
    s.hoveredTaskId === task.id ||
    (Boolean(task.waitingForSubtaskId) && s.hoveredTaskId === task.waitingForSubtaskId) ||
    (Boolean(task.parentId) && s.hoveredTaskId === task.parentId)
  );

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

  // Subtasks belonging to this task (queried only if card is in done column)
  const subtasks = useBoardStore(
    useShallow((s) => (isDone ? s.tasks.filter((t) => t.parentId === task.id) : []))
  );

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
          className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#d97706] dark:text-[#f59e0b] bg-[#fefce8] dark:bg-[#161b22] border border-[#fef08a] dark:border-[#30363d] hover:border-[#f59e0b] transition cursor-pointer"
        >
          <span className="h-1.5 w-1.5 rounded-full bg-[#f59e0b]" />
          <span>Ждем подзадачу {task.waitingForSubtaskId || ''}</span>
        </button>
      );
    }

    switch (status) {
      case 'running':
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono tabular-nums text-[#15803d] dark:text-[#4ade80] bg-[#f0fdf4] dark:bg-[#161b22] border border-[#bbf7d0] dark:border-[#30363d]">
            <span className="h-1.5 w-1.5 rounded-full bg-[#15803d] dark:bg-[#4ade80]" />
            <span>В работе {formatTimerMMSS(currentStageDuration)}</span>
          </span>
        );
      case 'paused':
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#b45309] dark:text-[#fbbf24] bg-[#fefce8] dark:bg-[#161b22] border border-[#fef08a] dark:border-[#30363d]">
            <span className="h-1.5 w-1.5 rounded-full bg-[#b45309] dark:bg-[#fbbf24]" />
            <span>Пауза</span>
          </span>
        );
      case 'waiting_review':
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#7e22ce] dark:text-[#c084fc] bg-[#faf5ff] dark:bg-[#161b22] border border-[#e9d5ff] dark:border-[#30363d]">
            <span>Ревью</span>
          </span>
        );
      case 'done':
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#15803d] dark:text-[#4ade80] bg-[#f0fdf4] dark:bg-[#161b22] border border-[#bbf7d0] dark:border-[#30363d]">
            <span>Готово</span>
          </span>
        );
      case 'blocked':
        return (
          <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#b91c1c] dark:text-[#f87171] bg-[#fef2f2] dark:bg-[#161b22] border border-[#fecaca] dark:border-[#30363d]">
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
                className="mb-2 flex items-start gap-1.5 p-1.5 rounded-md bg-[#f9fafb] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[10px] text-[#4d4d4d] dark:text-[#c9d1d9] font-mono select-none"
                onClick={(e) => e.stopPropagation()}
              >
                <CornerDownRight className="h-3 w-3 text-[#666666] dark:text-[#8b949e] shrink-0 mt-0.5" />
                <div className="truncate min-w-0">
                  <span className="font-semibold text-[#171717] dark:text-[#f0f6fc]">Эстафета: </span>
                  <span className="text-[#4d4d4d] dark:text-[#8b949e]">{task.handoverSummary}</span>
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
                                    ? 'bg-[#f0fdf4] dark:bg-[#161b22] text-[#15803d] dark:text-[#7ee787] border border-[#bbf7d0] dark:border-[#238636]'
                                    : sub?.verdict === 'APPROVED_WITH_CAVEATS'
                                    ? 'bg-[#fffbeb] dark:bg-[#161b22] text-[#b45309] dark:text-[#facc15] border border-[#fde68a] dark:border-[#713f12]'
                                    : 'bg-[#fef2f2] dark:bg-[#161b22] text-[#b91c1c] dark:text-[#ff7b72] border border-[#fecaca] dark:border-[#da3633]'
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

function areCardPropsEqual(prev: KanbanCardProps, next: KanbanCardProps): boolean {
  if (prev.index !== next.index) return false;
  if (prev.task === next.task) return true;

  const a = prev.task;
  const b = next.task;

  return (
    a.id === b.id &&
    a.title === b.title &&
    a.description === b.description &&
    a.columnId === b.columnId &&
    a.boardId === b.boardId &&
    a.status === b.status &&
    a.priority === b.priority &&
    a.branch === b.branch &&
    a.worktreePath === b.worktreePath &&
    a.persona === b.persona &&
    a.verdict === b.verdict &&
    a.verdictNote === b.verdictNote &&
    a.model === b.model &&
    a.parentId === b.parentId &&
    a.subtaskIndex === b.subtaskIndex &&
    a.waitingForSubtaskId === b.waitingForSubtaskId &&
    a.handoverSummary === b.handoverSummary &&
    a.activeStageDuration === b.activeStageDuration &&
    a.totalElapsed === b.totalElapsed &&
    a.stageEnteredAt === b.stageEnteredAt &&
    a.updatedAt === b.updatedAt &&
    a.createdAt === b.createdAt &&
    a.isTimerRunning === b.isTimerRunning &&
    a.returnsCount === b.returnsCount &&
    a.attachments === b.attachments &&
    a.todos === b.todos
  );
}

export default memo(KanbanCard, areCardPropsEqual);

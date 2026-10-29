import { useState, memo } from 'react';
import { Draggable } from '@hello-pangea/dnd';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore, isDoneColumnDef } from '../../stores/boardStore';
import { useCortexStore } from '../../stores/cortexStore';
import { useTranslation } from '../../stores/languageStore';
import { useShallow } from 'zustand/react/shallow';
import { GitBranch } from 'lucide-react';
import CardContextMenu from './CardContextMenu';
import { CardTodoProgress } from './CardTodoProgress';
import { CardTimerBadge, CardTimeFooter } from './CardTimerBadge';
import { CardSubtaskBadge } from './CardSubtaskBadge';
import { CardHookNode, useCardHooks } from './CardHookNode';
import { areCardPropsEqual, KanbanCardProps } from './cardComparators';
import { getDeterministicAgentColor } from '../../lib/agentPalette';
import { useUiZoomStore, adjustDndTransformForZoom } from '../../hooks/useUiZoom';

export type { KanbanCardProps };

function KanbanCard({ task, index }: KanbanCardProps) {
  const zoom = useUiZoomStore((s) => s.zoom);
  const openDrawer = useDrawerStore((s) => s.openDrawer);
  const setHoveredTaskId = useBoardStore((s) => s.setHoveredTaskId);
  const columns = useBoardStore(useShallow((s) => s.columns));
  // Invariant § 2.22 & Defect 10: Targeted selector prevents cascading board re-renders on unrelated question updates
  const directQuestion = useBoardStore((s) => s.activeQuestions[task.id]);
  const subtaskQuestion = useBoardStore((s) => (task.waitingForSubtaskId ? s.activeQuestions[task.waitingForSubtaskId] : undefined));
  const activeQuestion = directQuestion || subtaskQuestion;
  const activeQuestionTargetId = directQuestion ? task.id : task.waitingForSubtaskId;
  const { t } = useTranslation();

  const isHighlighted = useBoardStore((s) => {
    const hId = s.hoveredTaskId;
    if (!hId) return false;
    if (hId === task.id) return true;
    if (task.parentId && hId === task.parentId) return true;
    if (task.waitingForSubtaskId && hId === task.waitingForSubtaskId) return true;
    const hoveredTask = s.tasks.find((t) => t.id === hId);
    if (!hoveredTask) return false;
    if (hoveredTask.parentId === task.id) return true;
    if (task.parentId && hoveredTask.parentId && task.parentId === hoveredTask.parentId) return true;
    if (hoveredTask.waitingForSubtaskId === task.id) return true;
    return false;
  });

  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);

  const taskColumn = columns.find((c) => c.id === task.columnId);
  const assignedPersonaColor = useCortexStore((s) =>
    taskColumn?.persona ? s.agentPersonas.find((p) => p.role === taskColumn.persona)?.color : undefined
  );
  const assignedAgentColor = taskColumn?.persona
    ? (assignedPersonaColor || getDeterministicAgentColor(taskColumn.persona))
    : (taskColumn?.color || getDeterministicAgentColor(task.columnId));
  const ringGlowColor = assignedAgentColor.startsWith('#') && assignedAgentColor.length === 7
    ? `${assignedAgentColor}50`
    : assignedAgentColor;
  const isDone = Boolean(task.status === 'done' || isDoneColumnDef(taskColumn));

  // Subtasks belonging to this task (queried only if card is in done column)
  const subtasks = useBoardStore(
    useShallow((s) => (isDone ? s.tasks.filter((t) => t.parentId === task.id) : []))
  );

  // Hook nodes data
  const hookData = useCardHooks(task, taskColumn, columns);

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY });
  };

  return (
    <>
      <Draggable draggableId={task.id} index={index}>
        {(provided, snapshot) => {
          const baseTransform = adjustDndTransformForZoom(provided.draggableProps.style?.transform, zoom);
          return (
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
              style={{
                ...provided.draggableProps.style,
                transform: snapshot.isDragging && baseTransform
                  ? `${baseTransform} rotate(1.8deg) scale(1.025)`
                  : baseTransform,
                transition: provided.draggableProps.style?.transition
                  ? `${provided.draggableProps.style.transition}, transform 0.08s ease-out, box-shadow 0.15s ease-out, border-color 0.15s ease-out`
                  : 'transform 0.08s ease-out, box-shadow 0.15s ease-out, border-color 0.15s ease-out',
                borderColor: snapshot.isDragging
                  ? undefined
                  : isHighlighted
                  ? assignedAgentColor
                  : `${assignedAgentColor}50`,
                boxShadow: snapshot.isDragging
                  ? undefined
                  : isHighlighted
                  ? `0 0 0 2px ${ringGlowColor}, 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -2px rgba(0, 0, 0, 0.1)`
                  : undefined,
              }}
              className={`group relative mb-2 rounded-md border p-3 select-none ${
                snapshot.isDragging
                  ? 'scale-[1.02] rotate-[1.5deg] shadow-2xl ring-1 ring-black/10 dark:ring-white/20 cursor-grabbing z-50 border-[#171717] dark:border-[#30363d] bg-white dark:bg-[#161b22]'
                  : `transition-transform duration-100 ease-out cursor-pointer active:scale-[0.99] bg-white dark:bg-[#161b22] ${
                      isHighlighted
                        ? 'shadow-md'
                        : 'border-[#ebebeb] dark:border-[#21262d] hover:border-[#d4d4d8] dark:hover:border-[#30363d] shadow-xs'
                    }`
              }`}
            >
              {/* Header: ID & Concise Status */}
              <div className="flex items-center justify-between mb-2 gap-1.5 flex-wrap">
                <div className="flex items-center space-x-1.5">
                  <span className="font-mono text-[10px] font-semibold text-[#171717] dark:text-[#f0f6fc] bg-[#f4f4f5] dark:bg-[#0d1117] px-1.5 py-0.5 rounded border border-[#ebebeb] dark:border-[#30363d] tabular-nums">
                    {task.id}
                  </span>
                </div>
                <CardTimerBadge task={task} isDone={isDone} />
              </div>

              {/* Active Agent Question Alert Badge */}
              {activeQuestion && (
                <div
                  role="button"
                  tabIndex={0}
                  onClick={(e) => {
                    e.stopPropagation();
                    if (activeQuestionTargetId) {
                      openDrawer(activeQuestionTargetId, 'thoughts');
                    }
                  }}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' || e.key === ' ') {
                      e.stopPropagation();
                      if (activeQuestionTargetId) {
                        openDrawer(activeQuestionTargetId, 'thoughts');
                      }
                    }
                  }}
                  className="mb-2 flex items-center gap-1.5 px-2 py-1.5 rounded bg-amber-500/10 dark:bg-amber-500/20 border border-amber-500/40 text-amber-700 dark:text-amber-300 hover:bg-amber-500/20 dark:hover:bg-amber-500/30 transition cursor-pointer text-left overflow-hidden shadow-xs"
                >
                  <span className="relative flex h-2 w-2 shrink-0">
                    <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-amber-400 opacity-75"></span>
                    <span className="relative inline-flex rounded-full h-2 w-2 bg-amber-500"></span>
                  </span>
                  <div className="flex items-center gap-1 min-w-0 flex-1">
                    <span className="text-[10px] font-mono font-bold tracking-tight shrink-0">
                      {t('card.agentQuestion')}:
                    </span>
                    <span className="text-[10px] font-sans truncate text-[#4d4d4d] dark:text-[#d4d4d8]">
                      {activeQuestion.text}
                    </span>
                  </div>
                </div>
              )}

              {/* Title */}
              <h4 className="text-xs font-medium text-[#171717] dark:text-[#f0f6fc] leading-snug mb-2 font-sans break-words">
                {task.title}
              </h4>

              {/* Todo checklist progress strip */}
              <CardTodoProgress
                todos={task.todos}
                stageTitle={taskColumn?.title || task.columnId}
                stageColor={assignedAgentColor}
                isDragging={snapshot.isDragging}
                taskId={task.id}
              />

              {/* Footer: model, branch, created & elapsed time */}
              <div className="pt-2 border-t border-[#ebebeb] dark:border-[#21262d]">
                {task.model && task.model.trim().length > 0 && (
                  <div
                    className="truncate font-mono text-[10px] text-[#666666] dark:text-[#8b949e] mb-1"
                    aria-label={t('card.model', { model: task.model })}
                  >
                    {task.model}
                  </div>
                )}
                {(() => {
                  const cardBranch = task.branch || (task.parentId ? useBoardStore.getState().tasks.find((t) => t.id === task.parentId)?.branch : undefined);
                  return cardBranch ? (
                    <div className="flex items-center justify-between gap-2 font-mono text-[10px]">
                      <span className="inline-flex items-center gap-1 truncate text-[#8f8f8f] dark:text-[#8b949e] min-w-0">
                        <GitBranch className="h-2.5 w-2.5 shrink-0" />
                        <span className="truncate">{cardBranch}</span>
                      </span>
                    </div>
                  ) : null;
                })()}
                <CardTimeFooter task={task} isDone={isDone} />
              </div>

              {/* In Done column: Collapsible list of completed subtasks with dialogue open */}
              <CardSubtaskBadge task={task} subtasks={subtasks} isDone={isDone} />

              {/* Left Edge Hook Node (on_enter / transition result) */}
              {hookData.hasLeftHook && hookData.effectiveLeftHookName && (
                <CardHookNode
                  side="left"
                  hookType="on_enter"
                  hookName={hookData.effectiveLeftHookName}
                  description={
                    hookData.isTransitionHookFromResult
                      ? hookData.transitionHookDescription
                      : hookData.enterHookDescription
                  }
                  state={hookData.effectiveLeftState}
                  stageTitle={taskColumn?.title || task.columnId}
                  sourceStageTitle={hookData.sourceStageTitle}
                  isTransition={hookData.isTransitionHookFromResult}
                  taskId={task.id}
                  isDragging={snapshot.isDragging}
                />
              )}

              {/* Right Edge Hook Node (on_leave / exit) */}
              {hookData.hasRightHook && hookData.effectiveRightHookName && (
                <CardHookNode
                  side="right"
                  hookType="on_leave"
                  hookName={hookData.effectiveRightHookName}
                  description={hookData.leaveHookDescription}
                  state={hookData.effectiveRightState}
                  stageTitle={taskColumn?.title || task.columnId}
                  taskId={task.id}
                  isDragging={snapshot.isDragging}
                />
              )}
            </div>
          );
        }}
      </Draggable>
      {contextMenu && (
        <CardContextMenu
          task={task}
          x={contextMenu.x}
          y={contextMenu.y}
          onClose={() => setContextMenu(null)}
        />
      )}
    </>
  );
}

export default memo(KanbanCard, areCardPropsEqual);

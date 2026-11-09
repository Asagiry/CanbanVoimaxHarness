import { useState, memo, useMemo } from 'react';
import { Draggable } from '@hello-pangea/dnd';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore, isDoneColumnDef } from '../../stores/boardStore';
import { useCortexStore } from '../../stores/cortexStore';
import { useSettingsStore } from '../../stores/settingsStore';
import { resolveCardModel, formatCardModel } from './cardModel';
import { useTranslation } from '../../stores/languageStore';
import { useShallow } from 'zustand/react/shallow';
import { GitBranch } from 'lucide-react';
import CardContextMenu from './CardContextMenu';
import { CardTodoProgress } from './CardTodoProgress';
import { CardTimerBadge, CardTimeFooter } from './CardTimerBadge';
import { CardSubtaskBadge } from './CardSubtaskBadge';
import { CardHookNode } from './CardHookNode';
import { useCardHooks } from './useCardHooks';
import { areCardPropsEqual, KanbanCardProps } from './cardComparators';
import { isFamilyHighlighted } from './cardFamily';
import { resolveStageColor } from '../../lib/agentPalette';
import { useUiZoomStore, adjustDndTransformForZoom } from '../../hooks/useUiZoom';

export type { KanbanCardProps };

function KanbanCard({ task, index }: KanbanCardProps) {
  const zoom = useUiZoomStore((s) => s.zoom);
  const openDrawer = useDrawerStore((s) => s.openDrawer);
  const setHoveredTaskId = useBoardStore((s) => s.setHoveredTaskId);
  const columns = useBoardStore(useShallow((s) => s.columns));
  // Invariant: Badge only on the asking card (exact key match). Never mirror via waitingForSubtaskId.
  const activeQuestion = useBoardStore((s) => s.activeQuestions[task.id]);
  const activeQuestionTargetId = task.id;
  const { t } = useTranslation();

  const hoveredTaskId = useBoardStore((s) => s.hoveredTaskId);
  const isHighlighted = isFamilyHighlighted(task, hoveredTaskId);

  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);

  const taskColumn = columns.find((c) => c.id === task.stageId);
  const assignedPersona = useCortexStore((s) =>
    taskColumn?.persona ? s.agentPersonas.find((p) => p.role === taskColumn.persona) : undefined
  );
  const globalModel = useSettingsStore((s) => s.settings.model);
  const globalProvider = useSettingsStore((s) => s.settings.provider);
  const providers = useSettingsStore(useShallow((s) => s.providers));
  const cardModel = useMemo(
    () => resolveCardModel(task, taskColumn, assignedPersona, globalModel, providers, globalProvider),
    [task.model, task.providerId, taskColumn?.modelOverride, assignedPersona?.model, assignedPersona?.providerId, globalModel, globalProvider, providers]
  );
  const cardModelText = cardModel ? formatCardModel(cardModel) : undefined;
  const assignedAgentColor = resolveStageColor(taskColumn, assignedPersona);
  const ringGlowColor = assignedAgentColor.startsWith('#') && assignedAgentColor.length === 7
    ? `${assignedAgentColor}50`
    : assignedAgentColor;
  const cardBranch = useBoardStore((s) => {
    if (task.branch) return task.branch;
    if (!task.parentId) return undefined;
    return s.tasks.find((t) => t.id === task.parentId)?.branch;
  });
  const isTerminalColumn = Boolean(taskColumn?.terminal || task.stageId === 'ready' || task.stageId === 'done' || isDoneColumnDef(taskColumn));
  const isDone = Boolean(task.status === 'done' || isTerminalColumn);

  // Subtasks belonging to this task (queried only if card is in done column)
  const subtasks = useBoardStore(
    useShallow((s) => (isTerminalColumn ? s.tasks.filter((t) => t.parentId === task.id) : []))
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
                  ? 'scale-[1.02] rotate-[1.5deg] shadow-2xl ring-1 ring-black/10 dark:ring-white/20 cursor-grabbing z-50 border-[#171717] dark:border-[#2c2c33] bg-white dark:bg-[#17171b]'
                  : `transition-transform duration-100 ease-out cursor-pointer active:scale-[0.99] bg-white dark:bg-[#17171b] ${
                      isHighlighted
                        ? 'shadow-md'
                        : 'border-[#ebebeb] dark:border-[#212126] hover:border-[#d4d4d8] dark:hover:border-[#2c2c33] shadow-xs'
                    }`
              }`}
            >
              {/* Header: ID & Concise Status */}
              <div className="flex items-center justify-between mb-2 gap-1.5 flex-wrap">
                <div className="flex items-center space-x-1.5">
                  <span className="font-mono text-[10px] font-semibold text-[#171717] dark:text-[#f4f4f5] bg-[#f4f4f5] dark:bg-[#111114] px-1.5 py-0.5 rounded border border-[#ebebeb] dark:border-[#2c2c33] tabular-nums">
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
                    <span className="text-[10px] font-sans truncate text-[#4d4d4d] dark:text-[#a1a1aa]">
                      {activeQuestion.text}
                    </span>
                  </div>
                </div>
              )}

              {/* Title */}
              <h4 className="text-xs font-medium text-[#171717] dark:text-[#f4f4f5] leading-snug mb-2 font-sans break-words">
                {task.title}
              </h4>

              {/* Todo checklist progress strip */}
              <CardTodoProgress
                todos={task.todos}
                stageTitle={taskColumn?.title || task.stageId}
                stageColor={assignedAgentColor}
                isDragging={snapshot.isDragging}
                taskId={task.id}
              />

              {/* Footer: model, branch, created & elapsed time */}
              <div className="pt-2 border-t border-[#ebebeb] dark:border-[#212126]">
                {cardModel && cardModelText && (
                  <div
                    className="truncate font-mono text-[10px] text-[#666666] dark:text-[#71717a] mb-1"
                    aria-label={cardModel.providerName
                      ? t('card.modelWithProvider', { provider: cardModel.providerName, model: cardModel.label })
                      : t('card.model', { model: cardModel.label })}
                  >
                    {cardModelText}
                  </div>
                )}
                {cardBranch && (
                  <div className="flex items-center justify-between gap-2 font-mono text-[10px]">
                    <span className="inline-flex items-center gap-1 truncate text-[#8f8f8f] dark:text-[#71717a] min-w-0 leading-none">
                      <GitBranch className="h-2.5 w-2.5 block shrink-0" />
                      <span className="truncate">{cardBranch}</span>
                    </span>
                  </div>
                )}
                <CardTimeFooter task={task} isDone={isDone} />
              </div>

              {/* In Done column: Collapsible list of completed subtasks with dialogue open */}
              <CardSubtaskBadge task={task} subtasks={subtasks} isDone={isTerminalColumn} />

              {/* Left Edge Hook Node (on_enter / transition result) */}
              {hookData.hasLeftHook && hookData.effectiveLeftHookName && (
                <CardHookNode
                  side="left"
                  hookType={hookData.effectiveLeftHookType}
                  hookName={hookData.effectiveLeftHookName}
                  description={
                    hookData.isTransitionHookFromResult
                      ? hookData.transitionHookDescription
                      : hookData.enterHookDescription
                  }
                  state={hookData.effectiveLeftState}
                  stageTitle={taskColumn?.title || task.stageId}
                  sourceStageTitle={hookData.sourceStageTitle}
                  isTransition={hookData.isTransitionHookFromResult}
                  taskId={task.id}
                  isDragging={snapshot.isDragging}
                />
              )}

              {/* Right Edge Hook Node (on_leave / exit, or running on_enter into next column) */}
              {hookData.hasRightHook && hookData.effectiveRightHookName && (
                <CardHookNode
                  side="right"
                  hookType={hookData.effectiveRightHookType}
                  hookName={hookData.effectiveRightHookName}
                  description={
                    hookData.isEnterRunningTarget
                      ? hookData.targetEnterHookDescription
                      : hookData.leaveHookDescription
                  }
                  state={hookData.effectiveRightState}
                  stageTitle={
                    hookData.isEnterRunningTarget
                      ? hookData.targetStageTitle || task.stageId
                      : taskColumn?.title || task.stageId
                  }
                  isEnterRunningTarget={hookData.isEnterRunningTarget}
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

import {
  KanbanTask,
  PipelineColumn,
  HookExecutionState,
  getHookScript,
  getHookDescription,
} from '../../types';
import { useBoardStore } from '../../stores/boardStore';
import { useShallow } from 'zustand/react/shallow';

export interface CardHooksResult {
  hasLeftHook: boolean;
  effectiveLeftHookName: string | undefined;
  effectiveLeftState: HookExecutionState | undefined;
  effectiveLeftHookType: 'on_enter' | 'on_leave';
  enterHookDescription: string | undefined;
  transitionHookDescription: string | undefined;
  isTransitionHookFromResult: boolean;
  sourceStageTitle: string | undefined;
  hasRightHook: boolean;
  effectiveRightHookName: string | undefined;
  effectiveRightState: HookExecutionState | undefined;
  effectiveRightHookType: 'on_enter' | 'on_leave';
  leaveHookDescription: string | undefined;
  targetStageTitle: string | undefined;
  targetEnterHookDescription: string | undefined;
  isEnterRunningTarget: boolean;
}

export function useCardHooks(
  task: KanbanTask,
  taskColumn: PipelineColumn | undefined,
  columns: PipelineColumn[]
): CardHooksResult {
  const storeHooks = useBoardStore(useShallow((s) => s.hookStates[task.id]));
  const parentHooks = useBoardStore(useShallow((s) => (task.parentId ? s.hookStates[task.parentId] : undefined)));
  const taskHooks = storeHooks || task.hookStates || parentHooks;

  const enterHookDef = taskColumn?.hooks?.['on_enter'];
  const enterHookName = getHookScript(enterHookDef);
  const enterHookDescription = getHookDescription(enterHookDef);
  const rawEnterHookState = taskHooks?.on_enter;
  const enterHookState =
    rawEnterHookState &&
    (!rawEnterHookState.stageId ||
      rawEnterHookState.stageId === task.stageId ||
      rawEnterHookState.stageId === task.previousStageId ||
      (rawEnterHookState.status !== 'running' &&
        task.stageEnteredAt &&
        rawEnterHookState.startedAt &&
        Math.abs(task.stageEnteredAt - rawEnterHookState.startedAt) < 30000))
      ? rawEnterHookState
      : undefined;

  // While an on_enter hook is still running, the card is usually still in
  // the source column (stageId not yet updated), so the target stageId is
  // still different from task.stageId. Per the left = source / right = target
  // contract, the incoming hook pulsing toward the next column must be on
  // the RIGHT edge, not the left.
  const isEnterRunningTarget = Boolean(
    rawEnterHookState?.status === 'running' && rawEnterHookState.stageId && rawEnterHookState.stageId !== task.stageId
  );
  const enterRunningState = isEnterRunningTarget ? rawEnterHookState : undefined;

  const leaveHookDef =
    taskColumn?.hooks?.['on_leave'] ||
    taskColumn?.hooks?.['on_exit'] ||
    taskColumn?.hooks?.['on_finish'];
  const leaveHookName = getHookScript(leaveHookDef);
  const leaveHookDescription = getHookDescription(leaveHookDef);
  const leaveHookState = taskHooks?.on_leave;

  // Check predecessor column in board order
  const currentColIndex = columns.findIndex((c) => c.id === task.stageId);
  const prevCol = currentColIndex > 0 ? columns[currentColIndex - 1] : null;

  const stageEntryTime = task.stageEnteredAt || task.createdAt || 0;
  const hookDuration = leaveHookState?.durationMs || 0;
  const isRecentHook =
    Boolean(leaveHookState?.startedAt) &&
    stageEntryTime > 0 &&
    (leaveHookState!.startedAt >= stageEntryTime - hookDuration - 10000 ||
      Math.abs(stageEntryTime - leaveHookState!.startedAt) <= hookDuration + 10000);

  const matchesPreviousStage = task.previousStageId
    ? task.previousStageId === leaveHookState?.stageId
    : Boolean(prevCol && prevCol.id === leaveHookState?.stageId);

  const isTransitionHookFromResult = Boolean(
    !enterHookState &&
    leaveHookState &&
    leaveHookState.stageId !== task.stageId &&
    matchesPreviousStage &&
    (isRecentHook || !stageEntryTime)
  );

  const effectiveLeftState =
    enterHookState || (isTransitionHookFromResult ? leaveHookState : undefined);
  const effectiveLeftHookName =
    enterHookName ||
    effectiveLeftState?.hookName ||
    (isTransitionHookFromResult ? leaveHookState?.hookName : undefined);
  const effectiveLeftHookType: 'on_enter' | 'on_leave' = isTransitionHookFromResult ? 'on_leave' : 'on_enter';
  const hasLeftHook = Boolean(enterHookName || effectiveLeftState);

  const sourceStageCol = effectiveLeftState?.stageId
    ? columns.find((c) => c.id === effectiveLeftState.stageId)
    : undefined;
  const sourceStageTitle = sourceStageCol?.title || effectiveLeftState?.stageId;
  const sourceLeaveDef =
    sourceStageCol?.hooks?.['on_leave'] ||
    sourceStageCol?.hooks?.['on_exit'] ||
    sourceStageCol?.hooks?.['on_finish'];
  const transitionHookDescription = getHookDescription(sourceLeaveDef);

  const hasLeaveStateHere = Boolean(leaveHookState && leaveHookState.stageId === task.stageId);
  const effectiveRightState =
    hasLeaveStateHere ? leaveHookState : enterRunningState;
  const effectiveRightHookName = leaveHookName || effectiveRightState?.hookName;
  const effectiveRightHookType: 'on_enter' | 'on_leave' = hasLeaveStateHere ? 'on_leave' : 'on_enter';
  const hasRightHook = Boolean(leaveHookName || effectiveRightState);

  const targetCol = enterRunningState?.stageId
    ? columns.find((c) => c.id === enterRunningState.stageId)
    : undefined;
  const targetStageTitle = targetCol?.title || enterRunningState?.stageId;
  const targetEnterDef = targetCol?.hooks?.['on_enter'];
  const targetEnterHookDescription = getHookDescription(targetEnterDef);

  return {
    hasLeftHook,
    effectiveLeftHookName,
    effectiveLeftState,
    effectiveLeftHookType,
    enterHookDescription,
    transitionHookDescription,
    isTransitionHookFromResult,
    sourceStageTitle,
    hasRightHook,
    effectiveRightHookName,
    effectiveRightState,
    effectiveRightHookType,
    leaveHookDescription,
    targetStageTitle,
    targetEnterHookDescription,
    isEnterRunningTarget,
  };
}

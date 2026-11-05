import type { KanbanTask, PipelineColumn } from '../../types';
import { calculateStageDuration } from '../card/CardTimerBadge';
import { useAgentSessionStore } from '../../stores/agentSessionStore';

export interface StageStatusResult {
  isCurrent: boolean;
  isColRunning: boolean;
  isPassed: boolean;
}

export function resolveStageTask(
  colId: string,
  workingColumns: PipelineColumn[],
  task: KanbanTask,
  rootTask: KanbanTask | null,
  subtasks: KanbanTask[]
): KanbanTask | null {
  const matchingSubtask = [...subtasks].reverse().find((s) => s.stageId === colId);
  if (matchingSubtask) {
    return matchingSubtask;
  }

  if (task && task.stageId === colId) {
    return task;
  }

  if (rootTask) {
    const hasRootDuration =
      typeof rootTask.stageDurations?.[colId] === 'number' && rootTask.stageDurations[colId] > 0;
    const isRootCurrentStage = rootTask.stageId === colId;
    const isFirstWorkingStage = workingColumns.length > 0 && workingColumns[0]?.id === colId;
    if (isRootCurrentStage || hasRootDuration || isFirstWorkingStage) {
      return rootTask;
    }
  }

  if (task && typeof task.stageDurations?.[colId] === 'number' && task.stageDurations[colId] > 0) {
    return task;
  }

  return null;
}

export function resolveStageDuration(
  colId: string,
  task: KanbanTask,
  rootTask: KanbanTask | null,
  subtasks: KanbanTask[],
  isDrawerRunning: boolean
): number {
  const matchingSubtask = [...subtasks].reverse().find((s) => s.stageId === colId);
  if (matchingSubtask) {
    const isSubtaskRunning = isDrawerRunning && matchingSubtask.status === 'running';
    if (isSubtaskRunning) {
      const live = calculateStageDuration(matchingSubtask, true);
      if (live > 0) return live;
    }
    const recorded = matchingSubtask.stageDurations?.[colId];
    if (typeof recorded === 'number' && recorded > 0) return recorded;
    if (matchingSubtask.activeStageDuration && matchingSubtask.activeStageDuration > 0) {
      return matchingSubtask.activeStageDuration;
    }
  }

  if (rootTask) {
    const isRootAtStage = rootTask.stageId === colId;
    const isRootRunning = isDrawerRunning && rootTask.status === 'running' && isRootAtStage;
    if (isRootRunning) {
      const live = calculateStageDuration(rootTask, true);
      if (live > 0) return live;
    }
    const recorded = rootTask.stageDurations?.[colId];
    if (typeof recorded === 'number' && recorded > 0) return recorded;
    if (isRootAtStage && rootTask.activeStageDuration && rootTask.activeStageDuration > 0) {
      return rootTask.activeStageDuration;
    }
  }

  if (task) {
    const isTaskAtStage = task.stageId === colId;
    const isTaskRunning = isDrawerRunning && task.status === 'running' && isTaskAtStage;
    if (isTaskRunning) {
      const live = calculateStageDuration(task, true);
      if (live > 0) return live;
    }
    const recorded = task.stageDurations?.[colId];
    if (typeof recorded === 'number' && recorded > 0) return recorded;
    if (isTaskAtStage && task.activeStageDuration && task.activeStageDuration > 0) {
      return task.activeStageDuration;
    }
  }

  return 0;
}

export function resolveStageStatus(
  colId: string,
  colIndex: number,
  currentStageIdx: number,
  targetTask: KanbanTask | null,
  task: KanbanTask,
  rootTask: KanbanTask | null,
  subtasks: KanbanTask[],
  activeTaskId: string | null,
  isDrawerRunning: boolean,
  workingColumns: PipelineColumn[] = []
): StageStatusResult {
  const isCurrent =
    targetTask != null
      ? (subtasks.length > 0 ? targetTask.id === activeTaskId : colId === task.stageId)
      : colId === task.stageId;

  let isColRunning = false;
  if (targetTask?.status !== 'done') {
    if (targetTask) {
      const isTargetSubtask = targetTask.parentId != null || (rootTask != null && targetTask.id !== rootTask.id);
      if (isTargetSubtask) {
        isColRunning =
          targetTask.status === 'running' ||
          useAgentSessionStore.getState().isSessionRunning(targetTask.id) ||
          (targetTask.id === activeTaskId && isDrawerRunning && task.status === 'running');
      } else {
        const anySubtaskRunning = subtasks.some(
          (s) => s.status === 'running' || useAgentSessionStore.getState().isSessionRunning(s.id)
        );
        const subsequentSubtaskExists = subtasks.some((s) => s.stageId !== colId);
        if (!anySubtaskRunning && !subsequentSubtaskExists) {
          isColRunning =
            targetTask.stageId === colId &&
            (targetTask.status === 'running' ||
              (targetTask.id === activeTaskId && isDrawerRunning && task.status === 'running'));
        }
      }
    } else if (colId === task.stageId && task.status === 'running' && isDrawerRunning) {
      isColRunning = true;
    }
  }

  const isTerminalRoot =
    rootTask != null &&
    (rootTask.status === 'done' || rootTask.stageId === 'ready' || rootTask.stageId === 'done');

  const taskWorkingIdx = workingColumns.findIndex((c) => c.id === task?.stageId);
  const isEarlierThanCurrent =
    (taskWorkingIdx >= 0 && colIndex < taskWorkingIdx) ||
    (currentStageIdx >= 0 && colIndex < currentStageIdx);

  const hasSubsequentSubtask = subtasks.some((s) => {
    const sIdx = workingColumns.findIndex((c) => c.id === s.stageId);
    return sIdx > colIndex;
  });

  const isPassed =
    !isColRunning &&
    Boolean(targetTask) &&
    (targetTask?.status === 'done' || isTerminalRoot || isEarlierThanCurrent || hasSubsequentSubtask);

  return { isCurrent, isColRunning, isPassed };
}

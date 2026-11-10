import { useMemo } from 'react';
import type { KanbanTask } from '../../types';
import { useBoardStore } from '../../stores/boardStore';
import { useSettingsStore } from '../../stores/settingsStore';
import { useShallow } from 'zustand/react/shallow';

export function useDrawerTaskState(activeTaskId: string | null) {
  const rawTask = useBoardStore((s) => (activeTaskId ? s.tasks.find((t) => t.id === activeTaskId) : undefined));
  const updateTask = useBoardStore((s) => s.updateTask);
  const columns = useBoardStore(useShallow((s) => s.columns));

  const settings = useSettingsStore((s) => s.settings);
  const providers = useSettingsStore((s) => s.providers);

  const fallbackBoardId = useBoardStore.getState().activeBoardId || 'development';
  const fallbackColId = columns.find((c) => c.isIntake)?.id ?? columns[0]?.id ?? 'backlog';
  const fallbackPersona = columns.find((c) => !c.isIntake && c.id !== 'backlog' && !c.terminal)?.persona || 'general';
  const parentIdFromId = activeTaskId?.includes('.') ? activeTaskId.split('.')[0] : undefined;
  const parentTaskForFallback = useBoardStore((s) => (parentIdFromId ? s.tasks.find((t) => t.id === parentIdFromId) : undefined));

  const baseTask: KanbanTask = rawTask ?? {
    id: activeTaskId || 'task',
    title: activeTaskId || 'Task',
    stageId: fallbackColId,
    columnId: fallbackColId,
    boardId: fallbackBoardId,
    persona: fallbackPersona,
    status: 'idle',
    priority: 'high',
    parentId: parentIdFromId,
    branch: parentTaskForFallback?.branch || `feat/${(parentIdFromId || activeTaskId || 'task').toLowerCase()}`,
    worktreePath: parentTaskForFallback?.worktreePath || `worktrees/${(parentIdFromId || activeTaskId || 'task').toLowerCase()}`,
    createdAt: Date.now(),
    updatedAt: Date.now(),
    activeStageDuration: 0,
    stageDurations: {},
    totalElapsed: 0,
    isTimerRunning: false,
    returnsCount: 0,
    verdict: 'NONE',
  };

  const parentOfBase = useBoardStore((s) => (baseTask.parentId ? s.tasks.find((t) => t.id === baseTask.parentId) : undefined));
  const rootTask = parentOfBase || baseTask;
  const rootTaskId = rootTask.id;

  const task: KanbanTask = {
    ...baseTask,
    branch: baseTask.branch || rootTask.branch || `feat/${(rootTask.id || baseTask.id).toLowerCase()}`,
    worktreePath: baseTask.worktreePath || rootTask.worktreePath || `worktrees/${(rootTask.id || baseTask.id).toLowerCase()}`,
  };

  const subtasks = useBoardStore(
    useShallow((s) => (rootTaskId ? s.tasks.filter((t) => t.parentId === rootTaskId) : []))
  );

  const effectiveModel = settings.model || '';
  const selectableModels = useMemo(() => {
    const seen = new Set<string>(effectiveModel ? [effectiveModel] : []);
    const catalogIds = providers.flatMap((p) => p.models.map((m) => m.id));
    const fallback = catalogIds.length > 0 ? catalogIds : (settings.model ? [settings.model] : []);
    for (const id of fallback) {
      if (id) seen.add(id);
    }
    return Array.from(seen);
  }, [effectiveModel, providers, settings.model]);

  return {
    rawTask,
    task,
    baseTask,
    rootTask,
    rootTaskId,
    subtasks,
    columns,
    updateTask,
    selectableModels,
    effectiveModel,
  };
}

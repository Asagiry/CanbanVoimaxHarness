import { useBoardStore } from '../../stores/boardStore';
import { useDrawerStore } from '../../stores/drawerStore';
import { useCortexStore } from '../../stores/cortexStore';
import { showAdvanceTaskNotification, showRejectTaskNotification } from '../notificationService';
import type { AddListenerFn } from './agentEventRouter';
import type { TextBufferManager } from './textBuffer';
import { isEventForCurrentProject } from './helpers';

export async function registerTaskEventListeners(
  addListener: AddListenerFn,
  bufferManager: TextBufferManager,
  isDisposedRef: { current: boolean }
): Promise<void> {
  // 11. task:created
  await addListener('task:created', (payload) => {
    const eventRoot = payload?.projectPath || payload?.task?.projectPath || (payload?.task as any)?.project_path;
    if (!isEventForCurrentProject(eventRoot)) return;
    if (payload?.task) {
      useBoardStore.getState().upsertBackendTask(payload.task);
    }
  });

  // 12. task:moved
  await addListener('task:moved', (payload) => {
    const eventRoot = payload?.projectPath || payload?.task?.projectPath || (payload?.task as any)?.project_path;
    if (!isEventForCurrentProject(eventRoot)) return;
    if (payload?.task) {
      const rawTask = payload.task;
      if (payload.fromStage && typeof rawTask === 'object') {
        rawTask.previousStageId = payload.fromStage;
      }
      useBoardStore.getState().upsertBackendTask(rawTask);
    }
    if (payload?.rejected) {
      const taskId = payload.taskId || payload.task?.id;
      if (taskId) {
        void showRejectTaskNotification({
          taskId,
          reason: payload.reason,
          toStage: payload.toStage || payload.stageId || payload.task?.stage_id,
        });
      }
    }
  });

  // 13. task:advanced
  await addListener('task:advanced', (payload) => {
    if (payload) {
      void showAdvanceTaskNotification(payload);
    }
  });

  // 13b. task:activity / task:dialog (real-time task activities emitted by backend transitions & tools)
  const seenItemIds = new Set<string>();
  const handleTaskActivity = (payload: { taskId?: string; item?: any }) => {
    const { taskId, item } = payload || {};
    if (taskId && item) {
      const dedupKey = item.id ? `${taskId}:${item.id}` : null;
      if (dedupKey && seenItemIds.has(dedupKey)) return;
      if (dedupKey) {
        seenItemIds.add(dedupKey);
        if (seenItemIds.size > 200) {
          const first = seenItemIds.values().next().value;
          if (first) seenItemIds.delete(first);
        }
      }
      bufferManager.flushTask(taskId);
      useDrawerStore.getState().appendActivityItem(taskId, item);
    }
  };
  await addListener('task:activity', handleTaskActivity);
  await addListener('task:dialog', handleTaskActivity);

  // 14. hook:start
  await addListener('hook:start', (payload) => {
    const { taskId, stageId, hookType, hookName, description } = payload || {};
    if (!taskId || !stageId || !hookType) return;
    const normalizedHookType = hookType === 'on_finish' || hookType === 'on_exit' ? 'on_leave' : hookType;
    if (!hookName) {
      console.warn('[hook:start] missing hookName, falling back to hookType', payload);
    }
    useBoardStore.getState().setHookStart({
      taskId,
      stageId,
      hookType: normalizedHookType,
      hookName: hookName || normalizedHookType,
      description,
    });
  });

  // 15. hook:finish
  await addListener('hook:finish', (payload) => {
    const { taskId, stageId, hookType, success, exitCode, stdout, stderr, error, hookName } = payload || {};
    if (taskId && stageId && hookType) {
      const normalizedHookType = hookType === 'on_finish' || hookType === 'on_exit' ? 'on_leave' : hookType;
      useBoardStore.getState().setHookFinish({
        taskId,
        stageId,
        hookType: normalizedHookType,
        success: Boolean(success),
        exitCode,
        stdout,
        stderr,
        error,
        hookName,
      });
    }
  });

  // 16. task:updated
  await addListener('task:updated', (payload) => {
    const eventRoot = payload?.projectPath || payload?.task?.projectPath || (payload?.task as any)?.project_path;
    if (!isEventForCurrentProject(eventRoot)) return;
    const { taskId, todos, task } = payload || {};
    if (taskId) {
      bufferManager.flushTask(taskId);
    } else {
      bufferManager.flushAll();
    }
    if (task) {
      useBoardStore.getState().upsertBackendTask(task);
    } else if (taskId && todos) {
      useBoardStore.getState().updateTask(taskId, { todos });
    } else {
      void useBoardStore.getState().loadTasks();
    }
  });

  // 17. tasks:updated
  await addListener('tasks:updated', (payload) => {
    if (!isEventForCurrentProject(payload?.projectPath)) return;
    bufferManager.flushAll();
    if (Array.isArray(payload?.tasks)) {
      useBoardStore.getState().setBackendTasks(payload.tasks);
    } else {
      void useBoardStore.getState().loadTasks();
    }
  });

  // 19. project:changed & project:switched
  const handleProjectSwitched = async () => {
    if (useBoardStore.getState().isLoadingTasks) {
      return;
    }
    await useBoardStore.getState().loadBoards();
    await useBoardStore.getState().loadTasks();
    await useCortexStore.getState().loadAgentPersonas();
    await useCortexStore.getState().loadGlobalRules();
    useCortexStore.getState().syncStagesFromColumns(useBoardStore.getState().columns);
  };

  await addListener('project:changed', handleProjectSwitched);
  await addListener('project:switched', handleProjectSwitched);

  // 20. agent:personas_updated
  await addListener('agent:personas_updated', async () => {
    await useCortexStore.getState().loadAgentPersonas();
    useCortexStore.getState().syncStagesFromColumns(useBoardStore.getState().columns);
  });

  // 21. boards:updated & pipeline:updated (consolidated and deduplicated)
  let isSyncingBoards = false;
  let hasPendingBoardsSync = false;
  const handleBoardsOrPipelineUpdated = async () => {
    if (isSyncingBoards) {
      hasPendingBoardsSync = true;
      return;
    }
    isSyncingBoards = true;
    try {
      do {
        hasPendingBoardsSync = false;
        await useBoardStore.getState().loadBoards();
        await useBoardStore.getState().loadTasks();
        useCortexStore.getState().syncStagesFromColumns(useBoardStore.getState().columns);
      } while (hasPendingBoardsSync && !isDisposedRef.current);
    } finally {
      isSyncingBoards = false;
    }
  };

  await addListener('boards:updated', handleBoardsOrPipelineUpdated);
  await addListener('pipeline:updated', handleBoardsOrPipelineUpdated);

  // 22. cortex:rules_updated
  await addListener('cortex:rules_updated', async () => {
    await useCortexStore.getState().loadGlobalRules();
  });
}

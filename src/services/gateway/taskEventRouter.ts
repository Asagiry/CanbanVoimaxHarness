import { useBoardStore } from '../../stores/boardStore';
import { useDrawerStore } from '../../stores/drawerStore';
import { useCortexStore } from '../../stores/cortexStore';
import { useAgentSessionStore } from '../../stores/agentSessionStore';
import { showAdvanceTaskNotification, showRejectTaskNotification } from '../notificationService';
import type { AddListenerFn } from './agentEventRouter';
import type { TextBufferManager } from './textBuffer';
import type { TaskActivityPayload } from './types';
import { isEventForCurrentProject } from './helpers';

function waitForTasksLoadIdle(): Promise<void> {
  if (!useBoardStore.getState().isLoadingTasks) return Promise.resolve();
  return new Promise((resolve) => {
    const unsub = useBoardStore.subscribe((state) => {
      if (!state.isLoadingTasks) {
        unsub();
        resolve();
      }
    });
  });
}

export async function registerTaskEventListeners(
  addListener: AddListenerFn,
  bufferManager: TextBufferManager,
  isDisposedRef: { current: boolean }
): Promise<void> {
  // 11. task:created
  await addListener('task:created', (payload) => {
    const eventRoot = payload?.projectPath || payload?.task?.projectPath || payload?.task?.project_path;
    if (!isEventForCurrentProject(eventRoot)) return;
    if (payload?.task) {
      useBoardStore.getState().upsertBackendTask(payload.task);
    }
  });

  // 12. task:moved
  await addListener('task:moved', (payload) => {
    const eventRoot = payload?.projectPath || payload?.task?.projectPath || payload?.task?.project_path;
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
          toStage: payload.toStage || payload.stageId || payload.task?.stageId || payload.task?.stage_id,
        });
      }
    }
  });

  // 12b. task:deleted
  await addListener('task:deleted', (payload) => {
    if (!isEventForCurrentProject(payload?.projectPath)) return;
    const ids = new Set<string>();
    if (payload?.taskId) ids.add(payload.taskId);
    if (Array.isArray(payload?.deletedIds)) {
      for (const id of payload.deletedIds) {
        if (id) ids.add(id);
      }
    }
    if (ids.size === 0) return;

    for (const id of ids) {
      bufferManager.clearTask(id);
    }
    const drawer = useDrawerStore.getState();
    if (drawer.activeTaskId && ids.has(drawer.activeTaskId)) {
      drawer.closeDrawer();
    }
    for (const id of ids) {
      drawer.clearTaskDraft(id);
    }

    const board = useBoardStore.getState();
    const remaining = board.tasks.filter((t) => !ids.has(t.id));
    if (remaining.length !== board.tasks.length) {
      useBoardStore.setState({
        tasks: remaining,
        selectedTaskId: ids.has(board.selectedTaskId ?? '') ? null : board.selectedTaskId,
      });
    }
  });

  // 13. task:advanced
  await addListener('task:advanced', (payload) => {
    if (payload) {
      void showAdvanceTaskNotification(payload);
    }
  });

  // 13b. task:activity / task:dialog
  const seenItemIds = new Set<string>();
  const handleTaskActivity = (payload: TaskActivityPayload) => {
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

  // 13c. dialog:updated
  await addListener('dialog:updated', (payload) => {
    const taskId = payload?.taskId;
    if (!taskId) return;
    const drawer = useDrawerStore.getState();
    const isLoadedOrActive = drawer.activeTaskId === taskId || taskId in drawer.activityFeed;
    if (isLoadedOrActive && !drawer.activeAgentTaskIds[taskId]) {
      void drawer.loadTaskActivity(taskId);
    }
  });

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
    const eventRoot = payload?.projectPath || payload?.task?.projectPath || payload?.task?.project_path;
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

  // 19. project:changed & project:switched (coalesced, never dropped during active load)
  let isSyncingProject = false;
  let hasPendingProjectSync = false;
  const handleProjectSwitched = async () => {
    if (isSyncingProject) {
      hasPendingProjectSync = true;
      return;
    }
    isSyncingProject = true;
    try {
      do {
        hasPendingProjectSync = false;
        if (useBoardStore.getState().isLoadingTasks) {
          await waitForTasksLoadIdle();
          if (isDisposedRef.current) return;
        }
        useDrawerStore.getState().resetProjectContext();
        useCortexStore.getState().closeEditor?.();
        useAgentSessionStore.getState().reset();
        await useBoardStore.getState().loadBoards();
        await useBoardStore.getState().loadTasks();
        await useCortexStore.getState().loadAgentPersonas();
        await useCortexStore.getState().loadGlobalRules();
        useCortexStore.getState().syncStagesFromColumns(useBoardStore.getState().columns);
        await useAgentSessionStore.getState().syncActiveSessions();
      } while (hasPendingProjectSync && !isDisposedRef.current);
    } finally {
      isSyncingProject = false;
    }
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

import { useBoardStore } from '../../stores/boardStore';
import { useDrawerStore } from '../../stores/drawerStore';
import { useProjectStore } from '../../stores/projectStore';
import { useAgentSessionStore } from '../../stores/agentSessionStore';
import type { TextBufferManager } from './textBuffer';

export function isEventForCurrentProject(eventProjectPath?: string): boolean {
  if (!eventProjectPath || typeof eventProjectPath !== 'string') return true;
  const activeRoot = useProjectStore.getState().activeProject?.rootPath;
  if (!activeRoot) return true;
  const normEvent = eventProjectPath.replace(/\\/g, '/').toLowerCase().replace(/\/+$/, '');
  const normActive = activeRoot.replace(/\\/g, '/').toLowerCase().replace(/\/+$/, '');
  return normEvent === normActive;
}

/**
 * Predicate checking whether the specified task is actively selected in the drawer
 * or is a subtask belonging to the active drawer session.
 */
export function isTaskActiveInDrawer(taskId: string): boolean {
  const drawerState = useDrawerStore.getState();
  if (!drawerState.isOpen || !drawerState.activeTaskId) return false;
  const activeId = drawerState.activeTaskId;
  if (taskId === activeId) return true;
  const subId = drawerState.activeSubagentId;
  if (subId) {
    if (
      taskId === subId ||
      drawerState.toolIdToSubagentId[taskId] === subId ||
      drawerState.subagentIdToToolId[taskId] === subId ||
      drawerState.toolIdToSubagentId[subId] === taskId ||
      drawerState.subagentIdToToolId[subId] === taskId
    ) {
      return true;
    }
  }
  if (
    taskId.startsWith(`${activeId}.sub-`) ||
    taskId.startsWith(`${activeId}:sub:`) ||
    taskId.startsWith(`${activeId}.`)
  ) {
    return true;
  }
  const task = useBoardStore.getState().tasks.find((t) => t.id === taskId);
  return Boolean(task && task.parentId === activeId);
}

/**
 * Ensures that if an event arrives for an idle/uninitialized task, its status
 * reflects 'running' on the board.
 */
export function ensureTaskRunningOnBoard(taskId: string): void {
  const boardState = useBoardStore.getState();
  let currentTask = boardState.tasks.find((t) => t.id === taskId);
  if (!currentTask && taskId.includes('.')) {
    const parentCandidateId = taskId.split('.')[0];
    currentTask = boardState.tasks.find((t) => t.id === parentCandidateId);
  } else if (!currentTask && taskId.includes(':sub:')) {
    const parentCandidateId = taskId.split(':sub:')[0];
    currentTask = boardState.tasks.find((t) => t.id === parentCandidateId);
  }
  if (
    currentTask &&
    currentTask.status !== 'running' &&
    currentTask.status !== 'done' &&
    currentTask.status !== 'waiting_subagent'
  ) {
    boardState.updateTaskStatus(currentTask.id, 'running');
  }
}

/**
 * Marks currently running tools as failed when an agent finishes/fails/aborts.
 */
export function failRunningToolsForTask(taskId: string, reason = 'Агент завершён'): void {
  const ds = useDrawerStore.getState();
  for (const tool of ds.toolExecutions[taskId] || []) {
    if (tool.status === 'running') {
      ds.updateToolExecution(taskId, tool.id, { status: 'failed', stderr: reason });
    }
  }
}

export interface FinalizeRunOptions {
  failReason?: string;
  saveActivity?: boolean;
  errorMessage?: string;
  resetThinkingTime?: boolean;
  allowDoneReset?: boolean;
}

/**
 * Shared teardown path for agent:finished, agent:error, agent:status, and agent:aborted.
 */
export function finalizeAgentTaskRun(
  taskId: string,
  bufferManager: TextBufferManager,
  options: FinalizeRunOptions = {},
): boolean {
  useBoardStore.getState().clearActiveQuestion(taskId);
  failRunningToolsForTask(taskId, options.failReason);
  const activeInDrawer = isTaskActiveInDrawer(taskId);
  const drawer = useDrawerStore.getState();
  if (activeInDrawer) {
    bufferManager.flushTask(taskId);
    if (options.errorMessage) {
      drawer.appendOutput(taskId, options.errorMessage, 'error');
    }
    drawer.setActiveAgentTask(taskId, false);
    if (options.saveActivity) {
      void drawer.saveTaskActivity(taskId);
    }
    if (options.resetThinkingTime) {
      drawer.setThinkingStartTime(taskId, null);
    }
  } else {
    bufferManager.clearTask(taskId);
    drawer.setActiveAgentTask(taskId, false);
  }

  const currentTask = useBoardStore.getState().tasks.find((t) => t.id === taskId);
  const shouldReset = options.allowDoneReset
    ? Boolean(currentTask && currentTask.status !== 'done')
    : Boolean(currentTask && currentTask.status === 'running');
  if (shouldReset && !useAgentSessionStore.getState().isSessionRunning(taskId)) {
    useBoardStore.getState().updateTaskStatus(taskId, 'idle');
  }
  return activeInDrawer;
}

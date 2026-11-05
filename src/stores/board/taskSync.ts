import { KanbanTask, TodoItem, TaskStatus } from '../../types';
import { useDrawerStore } from '../drawerStore';
import { logEvent } from '../../lib/logger';
import { useBoardStore } from '../boardStore';
import { isTaskForeign } from './taskNormalizer';

export async function syncTasksBackend(tasks: KanbanTask[]): Promise<void> {
  const store = useBoardStore.getState();
  if (store.isLoadingTasks) {
    logEvent('DEBUG', 'BOARD_STORE', 'syncTasksBackend: skipping save while tasks are loading/switching');
    return;
  }

  try {
    const { useProjectStore } = await import('../projectStore');
    const activeRoot = useProjectStore.getState().activeProject?.rootPath;
    const projectTasks = activeRoot
      ? tasks.filter((t) => !isTaskForeign(t, activeRoot))
      : tasks;

    if (tasks.length > 0 && projectTasks.length === 0) {
      logEvent('WARN', 'BOARD_STORE', 'syncTasksBackend: suppressed save of tasks belonging to foreign projects');
      return;
    }

    const { invoke } = await import('@tauri-apps/api/core');
    const { activeBoardId, boards } = useBoardStore.getState();
    const fallbackBoardId = activeBoardId || boards[0]?.id || 'development';
    logEvent(
      'DEBUG',
      'BOARD_STORE',
      `syncTasksBackend: syncing ${projectTasks.length} tasks to save_tasks`,
      JSON.stringify(projectTasks.map((t) => t.id))
    );
    // NOTE: send ONLY canonical camelCase keys. Sending both stageId+columnId
    // makes Rust serde fail with "duplicate field `stageId`" and save_tasks
    // silently never persists (this broke run_agent with "Task not found").
    const backendPayload = projectTasks.map((t) => {
      const hasBoard = boards.some((b) => b.id === t.boardId);
      const effectiveBoardId = hasBoard ? t.boardId : fallbackBoardId;
      const isTerminalTask = t.stageId === 'ready' || t.stageId === 'done';
      return {
        id: t.id,
        title: t.title,
        description: t.description || '',
        stageId: t.stageId,
        boardId: effectiveBoardId,
        worktreePath: t.worktreePath,
        branch: t.branch,
        verdict: t.verdict === 'NONE' ? null : t.verdict,
        customFlow: t.customFlow,
        status: t.status === 'waiting_subtask' ? (isTerminalTask ? 'done' : 'idle') : t.status,
        model: t.model,
        parentId: t.parentId ?? null,
        subtaskIndex: t.subtaskIndex ?? null,
        waitingForSubtaskId: isTerminalTask ? null : (t.waitingForSubtaskId ?? null),
        // NOTE: todos MUST be included — otherwise every sync wipes the plan
        // the agent saved via todo_write (backend save_tasks replaces the file).
        todos: Array.isArray(t.todos)
          ? t.todos.map((td: TodoItem) => ({
              id: String(td?.id ?? ''),
              content: String(td?.content ?? ''),
              status: td?.status === 'completed' || td?.status === 'in_progress' ? td.status : 'pending',
            }))
          : null,
        currentTokens: typeof t.currentTokens === 'number' ? t.currentTokens : null,
        attachments: t.attachments && t.attachments.length > 0 ? t.attachments : null,
        coverImage: t.coverImage ?? null,
        handoverSummary: t.handoverSummary ?? null,
        priority: t.priority ?? null,
        tags: t.tags ?? null,
        createdAt: t.createdAt || Date.now(),
        updatedAt: t.updatedAt || Date.now(),
        stageEnteredAt: t.stageEnteredAt || t.createdAt || Date.now(),
        completedAt: t.completedAt ?? null,
        skipWorktree: t.skipWorktree ?? false,
        hookStates: t.hookStates ?? null,
        projectPath: t.projectPath || activeRoot || null,
        stageDurations: t.stageDurations ?? null,
        totalElapsed: typeof t.totalElapsed === 'number' ? t.totalElapsed : null,
      };
    });
    await invoke('save_tasks', { tasks: backendPayload });
  } catch (err) {
    console.info('Backend save_tasks not available or in web mode:', err);
    logEvent('WARN', 'BOARD_STORE', 'syncTasksBackend: save_tasks failed or in web mode', String(err));
  }
}

const inFlightAgentRuns = new Set<string>();

export async function triggerRunAgent(
  taskId: string,
  prompt?: string,
  options?: { skipSync?: boolean; projectPath?: string }
): Promise<void> {
  const normalizedId = taskId.trim().toLowerCase();
  if (inFlightAgentRuns.has(normalizedId)) {
    logEvent(
      'WARN',
      'BOARD_STORE',
      `triggerRunAgent: run_agent already in-flight for "${taskId}", skipping duplicate call`
    );
    return;
  }
  inFlightAgentRuns.add(normalizedId);

  try {
    // Ensure the task is guaranteed to be saved and known to the backend before running agent (§ Requirement 4).
    // When skipSync is true (e.g. after move_task_stage or autorun event), avoid syncTasksBackend
    // to prevent race conditions and clobbering backend-assigned fields.
    if (!options?.skipSync) {
      const currentTasks = useBoardStore.getState().tasks;
      const taskToRun = currentTasks.find(
        (t) => t.id === taskId || t.id.trim().toLowerCase() === taskId.trim().toLowerCase()
      );
      if (taskToRun) {
        await syncTasksBackend(currentTasks);
      }
    }

    const { invoke } = await import('@tauri-apps/api/core');
    const { useProjectStore } = await import('../projectStore');
    const activeRoot = useProjectStore.getState().activeProject?.rootPath;
    const projectPath = options?.projectPath || activeRoot || null;
    await invoke('run_agent', { taskId, prompt: prompt || null, projectPath });
  } catch (err) {
    const errMsg = String(err);
    logEvent('ERROR', 'BOARD_STORE', `triggerRunAgent: run_agent failed for task "${taskId}"`, errMsg);
    console.error(`[triggerRunAgent] run_agent failed for task "${taskId}":`, err);

    // Safely update the task in boardStore: revert status from 'running' to 'idle' (or 'error'), set isTimerRunning: false
    const tasksAfterErr = useBoardStore.getState().tasks;
    const reverted = tasksAfterErr.map((t) =>
      t.id === taskId || t.id.trim().toLowerCase() === taskId.trim().toLowerCase()
        ? {
            ...t,
            status: 'idle' as TaskStatus,
            isTimerRunning: false,
            updatedAt: Date.now(),
          }
        : t
    );
    useBoardStore.setState({ tasks: reverted });
    useDrawerStore.getState().setActiveAgentTask(taskId, false);
    if (!options?.skipSync) {
      void syncTasksBackend(reverted);
    }

    // Display user notification / alert so the user understands why the agent did not start
    alert(`Не удалось запустить агента для задачи "${taskId}":\n${errMsg}`);
  } finally {
    inFlightAgentRuns.delete(normalizedId);
  }
}

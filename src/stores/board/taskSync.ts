import { KanbanTask, TodoItem, TaskStatus } from '../../types';
import { useDrawerStore } from '../drawerStore';
import { logEvent } from '../../lib/logger';
import { useBoardStore } from '../boardStore';

export async function syncTasksBackend(tasks: KanbanTask[]): Promise<void> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const { activeBoardId, boards } = useBoardStore.getState();
    const fallbackBoardId = activeBoardId || boards[0]?.id || 'development';
    logEvent(
      'DEBUG',
      'BOARD_STORE',
      `syncTasksBackend: syncing ${tasks.length} tasks to save_tasks`,
      JSON.stringify(tasks.map((t) => t.id))
    );
    // NOTE: send ONLY canonical camelCase keys. Sending both stageId+columnId
    // makes Rust serde fail with "duplicate field `stageId`" and save_tasks
    // silently never persists (this broke run_agent with "Task not found").
    const backendPayload = tasks.map((t) => {
      const hasBoard = boards.some((b) => b.id === t.boardId);
      const effectiveBoardId = hasBoard ? t.boardId : fallbackBoardId;
      const isTerminalTask = t.columnId === 'ready' || t.status === 'done';
      return {
        id: t.id,
        title: t.title,
        description: t.description || '',
        stageId: t.columnId,
        boardId: effectiveBoardId,
        worktreePath: t.worktreePath,
        branch: t.branch,
        verdict: t.verdict === 'NONE' ? null : t.verdict,
        customFlow: t.customFlow,
        status: isTerminalTask && t.status === 'waiting_subtask' ? 'done' : t.status,
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
      };
    });
    await invoke('save_tasks', { tasks: backendPayload });
  } catch (err) {
    console.info('Backend save_tasks not available or in web mode:', err);
    logEvent('WARN', 'BOARD_STORE', 'syncTasksBackend: save_tasks failed or in web mode', String(err));
  }
}

export async function triggerRunAgent(taskId: string, prompt?: string): Promise<void> {
  // Ensure the task is guaranteed to be saved and known to the backend before running agent (§ Requirement 4)
  const currentTasks = useBoardStore.getState().tasks;
  const taskToRun = currentTasks.find(
    (t) => t.id === taskId || t.id.trim().toLowerCase() === taskId.trim().toLowerCase()
  );
  if (taskToRun) {
    await syncTasksBackend(currentTasks);
  }

  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('run_agent', { taskId, prompt: prompt || null });
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
    void syncTasksBackend(reverted);

    // Display user notification / alert so the user understands why the agent did not start
    alert(`Не удалось запустить агента для задачи "${taskId}":\n${errMsg}`);
  }
}

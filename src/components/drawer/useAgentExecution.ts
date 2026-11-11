import { useState } from 'react';
import type { KanbanTask, ToolExecution } from '../../types';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import { useAgentSessionStore } from '../../stores/agentSessionStore';

function getErrorMessage(err: unknown, fallback: string): string {
  if (err instanceof Error && err.message) return err.message;
  if (typeof err === 'string' && err) return err;
  return fallback;
}

export interface UseAgentExecutionProps {
  task: KanbanTask;
  subtasks: KanbanTask[];
  updateTask: (taskId: string, partial: Partial<KanbanTask>) => void;
  activeTaskId: string | null;
  activeSubagentId: string | null;
  activeTaskTools?: ToolExecution[];
  activeSubagentTools?: ToolExecution[];
  isAgentActive: boolean;
}

export function useAgentExecution({
  task,
  subtasks,
  updateTask,
  activeTaskId,
  activeSubagentId,
  activeTaskTools,
  activeSubagentTools,
  isAgentActive,
}: UseAgentExecutionProps) {
  const [runError, setRunError] = useState<string | null>(null);

  const appendOutput = useDrawerStore((s) => s.appendOutput);
  const setActiveAgentTask = useDrawerStore((s) => s.setActiveAgentTask);

  const isAuthoritativeSessionRunning = useAgentSessionStore((s) => {
    if (!activeTaskId) return false;
    if (s.isSessionRunning(activeTaskId)) return true;
    if (activeSubagentId && s.isSessionRunning(activeSubagentId)) return true;
    return false;
  });

  const hasRunningTool = Boolean(
    activeTaskTools?.some((t) => t.status === 'running') ||
    (activeSubagentId && activeSubagentTools?.some((t) => t.status === 'running'))
  );

  const isRunning = task.status === 'running' || isAuthoritativeSessionRunning || isAgentActive || hasRunningTool;

  const handleRunAgent = async () => {
    setRunError(null);
    updateTask(task.id, { status: 'running', isTimerRunning: true });
    setActiveAgentTask(task.id, true);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('run_agent', { taskId: task.id, prompt: null });
    } catch (err: unknown) {
      console.info('run_agent invoke failed or web mode:', err);
      const errMsg = getErrorMessage(err, 'Не удалось запустить агента');
      setRunError(errMsg);
      updateTask(task.id, { status: 'idle', isTimerRunning: false });
      setActiveAgentTask(task.id, false);
    }
  };

  const handleAbortAgent = async () => {
    let abortError: string | null = null;
    try {
      const { invoke } = await import('@tauri-apps/api/core');

      if (activeSubagentId) {
        const resolvedSubagentId =
          useDrawerStore.getState().toolIdToSubagentId[activeSubagentId] || activeSubagentId;
        try {
          await invoke('abort_agent', { taskId: resolvedSubagentId });
        } catch (err: unknown) {
          abortError = getErrorMessage(err, 'Не удалось остановить сабагента (backend)');
        }
        setActiveAgentTask(activeSubagentId, false);
        setActiveAgentTask(resolvedSubagentId, false);

        const boardSub = subtasks.find((s) => s.id === activeSubagentId || s.id === resolvedSubagentId);
        if (boardSub) {
          updateTask(boardSub.id, { status: 'paused', isTimerRunning: false });
        }

        const stateTools = useDrawerStore.getState().toolExecutions;
        const subTools = [
          ...(stateTools[activeSubagentId] || []),
          ...(stateTools[resolvedSubagentId] || []),
        ];
        for (const t of subTools) {
          if (t.status === 'running') {
            useDrawerStore.getState().updateToolExecution(activeSubagentId, t.id, {
              status: 'failed',
              stderr: 'Прервано пользователем',
            });
            useDrawerStore.getState().updateToolExecution(resolvedSubagentId, t.id, {
              status: 'failed',
              stderr: 'Прервано пользователем',
            });
          }
        }
        if (abortError) setRunError(abortError);
        return;
      }

      try {
        await invoke('abort_agent', { taskId: task.id });
      } catch (err: unknown) {
        abortError = getErrorMessage(err, 'Не удалось остановить агента (backend)');
      }
      updateTask(task.id, { status: 'paused', isTimerRunning: false });
      setActiveAgentTask(task.id, false);
      const currTools = useDrawerStore.getState().toolExecutions[task.id] || [];
      for (const t of currTools) {
        if (t.status === 'running') {
          useDrawerStore.getState().updateToolExecution(task.id, t.id, {
            status: 'failed',
            stderr: 'Прервано пользователем',
          });
        }
      }

      const allSubagents = useDrawerStore.getState().subagents[task.id] || [];
      for (const sub of allSubagents) {
        setActiveAgentTask(sub.id, false);
        useDrawerStore.getState().updateSubagentStatus(sub.id, 'stopped', 'Прервано пользователем');
        const subTools = useDrawerStore.getState().toolExecutions[sub.id] || [];
        for (const st of subTools) {
          if (st.status === 'running') {
            useDrawerStore.getState().updateToolExecution(sub.id, st.id, {
              status: 'failed',
              stderr: 'Прервано пользователем',
            });
          }
        }
      }
      if (abortError) setRunError(abortError);
    } catch (err) {
      console.info('abort_agent invoke failed or web mode:', err);
    }
  };

  const handleSendPrompt = async (
    promptText: string,
    images: Array<{ id: string; name: string; dataUrl: string }>
  ): Promise<boolean> => {
    const trimmed = promptText.trim();
    if (!trimmed && images.length === 0) {
      if (activeSubagentId) {
        const isSubagentABoardTask = Boolean(
          useBoardStore.getState().tasks.some((t) => t.id === activeSubagentId)
        );
        if (!isSubagentABoardTask) {
          const resolvedSubagentId =
            useDrawerStore.getState().toolIdToSubagentId[activeSubagentId] || activeSubagentId;
          const subCfg =
            useDrawerStore.getState().subagentConfigs?.[activeSubagentId] ||
            useDrawerStore.getState().subagentConfigs?.[resolvedSubagentId];
          const parentTaskItem = useBoardStore.getState().tasks.find((t) => t.id === task.id);
          const continueModel = subCfg?.model ?? parentTaskItem?.model ?? task.model ?? null;
          const continueProvider = subCfg?.providerId ?? parentTaskItem?.providerId ?? task.providerId ?? null;
          setActiveAgentTask(activeSubagentId, true);
          setRunError(null);
          try {
            const { invoke } = await import('@tauri-apps/api/core');
            await invoke('retry_subagent', {
              parentTaskId: task.id,
              subagentId: resolvedSubagentId,
              message: null,
              model: continueModel,
              providerId: continueProvider,
            });
            return true;
          } catch (err: unknown) {
            const errMsg = getErrorMessage(err, 'Ошибка перезапуска субагента');
            setRunError(errMsg);
            setActiveAgentTask(activeSubagentId, false);
            setActiveAgentTask(resolvedSubagentId, false);
            return false;
          }
        }
      }

      if (isRunning) return false;
      await handleRunAgent();
      return true;
    }

    const imageUrls = images.map((img) => img.dataUrl);
    setRunError(null);

    const mainText = trimmed || (images.length > 0 ? 'Изображение прикреплено' : '');

    const isSubagentABoardTask = Boolean(activeSubagentId && useBoardStore.getState().tasks.some((t) => t.id === activeSubagentId));
    const targetId = isSubagentABoardTask && activeSubagentId ? activeSubagentId : task.id;
    const targetTask = useBoardStore.getState().tasks.find((item) => item.id === targetId) || task;
    const targetIsRunning = targetId === task.id
      ? isRunning
      : Boolean(useDrawerStore.getState().activeAgentTaskIds[targetId]) ||
        useAgentSessionStore.getState().isSessionRunning(targetId) || targetTask.status === 'running';
    setActiveAgentTask(targetId, true);
    if (activeSubagentId && !isSubagentABoardTask) {
      setActiveAgentTask(activeSubagentId, true);
      appendOutput(activeSubagentId, mainText, 'user', imageUrls.length > 0 ? imageUrls : undefined);

      const resolvedSubagentId =
        useDrawerStore.getState().toolIdToSubagentId[activeSubagentId] || activeSubagentId;
      if (resolvedSubagentId !== activeSubagentId) {
        appendOutput(resolvedSubagentId, mainText, 'user', imageUrls.length > 0 ? imageUrls : undefined);
      }

      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const subCfg =
          useDrawerStore.getState().subagentConfigs?.[activeSubagentId] ||
          useDrawerStore.getState().subagentConfigs?.[resolvedSubagentId];
        const steerModel = subCfg?.model ?? targetTask.model ?? null;
        const steerProvider = subCfg?.providerId ?? targetTask.providerId ?? null;
        await invoke('retry_subagent', {
          parentTaskId: task.id,
          subagentId: resolvedSubagentId,
          message: mainText,
          model: steerModel,
          providerId: steerProvider,
        });
        return true;
      } catch (err: unknown) {
        const errMsg = getErrorMessage(err, 'Ошибка отправки команды субагенту');
        setRunError(errMsg);
        setActiveAgentTask(activeSubagentId, false);
        setActiveAgentTask(resolvedSubagentId, false);
        return false;
      }
    }

    const isSteer = targetIsRunning;
    appendOutput(targetId, mainText, isSteer ? 'steer' : 'user', imageUrls.length > 0 ? imageUrls : undefined);
    const optimisticFeed = useDrawerStore.getState().activityFeed[targetId] ?? [];
    const messageId = optimisticFeed[optimisticFeed.length - 1]?.id;

    if (!targetTask.description) {
      updateTask(targetId, { description: mainText, status: 'running', isTimerRunning: true });
    } else {
      updateTask(targetId, { status: 'running', isTimerRunning: true });
    }

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const steerResult = await invoke('steer_agent', {
        taskId: targetId,
        message: mainText,
        messageId,
        images: imageUrls.length > 0 ? imageUrls : undefined,
      }) as string;
      if (typeof steerResult === 'string' && (steerResult.includes('saved for stopped task') || steerResult.includes('Task completed'))) {
        setRunError('Steer сохранён, агент не запущен — задача завершена');
        updateTask(targetId, { status: 'idle', isTimerRunning: false });
        setActiveAgentTask(targetId, false);
      }
      return true;
    } catch (err: unknown) {
      console.info('steer_agent invoke failed or web mode:', err);
      const errMsg = getErrorMessage(err, 'Ошибка отправки команды агенту');
      setRunError(errMsg);
      if (!targetIsRunning) {
        updateTask(targetId, { status: 'idle', isTimerRunning: false });
        setActiveAgentTask(targetId, false);
      }
      return false;
    }
  };

  return {
    isRunning,
    runError,
    setRunError,
    handleRunAgent,
    handleAbortAgent,
    handleSendPrompt,
  };
}

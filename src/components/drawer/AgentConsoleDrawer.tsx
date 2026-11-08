import { useState, useEffect, useMemo } from 'react';
import type { KanbanTask } from '../../types';
import { useDrawerStore, DrawerActivityItem } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import { useSettingsStore } from '../../stores/settingsStore';
import { useToolPolicyStore } from '../../stores/toolPolicyStore';
import { useAgentSessionStore } from '../../stores/agentSessionStore';
import ErrorBoundary from '../common/ErrorBoundary';
import DrawerHeader from './DrawerHeader';
import DrawerActivityStream from './DrawerActivityStream';
import DrawerSteerBar from './DrawerSteerBar';
import GitDiffSidebar from './GitDiffSidebar';
import SubagentsSidebar from './SubagentsSidebar';
import { useUiZoom, getZoomOverlayStyle } from '../../hooks/useUiZoom';
import { useShallow } from 'zustand/react/shallow';

export default function AgentConsoleDrawer() {
  const { zoom } = useUiZoom();
  const isOpen = useDrawerStore((s) => s.isOpen);
  const activeTaskId = useDrawerStore((s) => s.activeTaskId);
  const activeSubagentId = useDrawerStore((s) => s.activeSubagentId);
  const closeDrawer = useDrawerStore((s) => s.closeDrawer);
  const goBackDrawer = useDrawerStore((s) => s.goBackDrawer);
  const openDrawer = useDrawerStore((s) => s.openDrawer);
  const appendOutput = useDrawerStore((s) => s.appendOutput);
  const setActiveAgentTask = useDrawerStore((s) => s.setActiveAgentTask);

  const activeFeed = useDrawerStore(useShallow((s) => {
    if (activeSubagentId) {
      let subFeed = s.activityFeed[activeSubagentId] || [];
      if (subFeed.length === 0) {
        const mappedId = s.toolIdToSubagentId[activeSubagentId];
        if (mappedId && s.activityFeed[mappedId]?.length) {
          subFeed = s.activityFeed[mappedId];
        } else {
          for (const [toolId, subId] of Object.entries(s.toolIdToSubagentId)) {
            if (subId === activeSubagentId && s.activityFeed[toolId]?.length) {
              subFeed = s.activityFeed[toolId];
              break;
            }
          }
        }
      }
      return subFeed.filter((item) => item.type !== 'prompt');
    }
    return activeTaskId ? (s.activityFeed[activeTaskId] || []) : [];
  }));

  const activeStream = useDrawerStore(useShallow((s) =>
    activeTaskId ? s.streamEntries[activeTaskId] : undefined
  ));

  const activeTaskTools = useDrawerStore(useShallow((s) =>
    activeTaskId ? s.toolExecutions[activeTaskId] : undefined
  ));

  const activeSubagentTools = useDrawerStore(useShallow((s) =>
    activeSubagentId ? s.toolExecutions[activeSubagentId] : undefined
  ));

  const isAgentActive = useDrawerStore((s) => Boolean(
    (activeTaskId && s.activeAgentTaskIds[activeTaskId]) ||
    (activeSubagentId && s.activeAgentTaskIds[activeSubagentId])
  ));

  const rawTask = useBoardStore((s) => (activeTaskId ? s.tasks.find((t) => t.id === activeTaskId) : undefined));
  const updateTask = useBoardStore((s) => s.updateTask);
  const columns = useBoardStore(useShallow((s) => s.columns));

  const settings = useSettingsStore((s) => s.settings);
  const providers = useSettingsStore((s) => s.providers);
  const openSettings = useSettingsStore((s) => s.openSettings);
  const isToolAllowed = useToolPolicyStore((s) => s.isToolAllowed);
  const openToolPolicyModal = useToolPolicyStore((s) => s.openModal);

  const isAuthoritativeSessionRunning = useAgentSessionStore((s) => {
    if (!activeTaskId) return false;
    if (s.isSessionRunning(activeTaskId)) return true;
    if (activeSubagentId && s.isSessionRunning(activeSubagentId)) return true;
    return false;
  });

  const [runError, setRunError] = useState<string | null>(null);
  const [isDiffCollapsed, setIsDiffCollapsed] = useState(false);
  const [isSubagentsCollapsed, setIsSubagentsCollapsed] = useState(false);

  const fallbackBoardId = useBoardStore.getState().activeBoardId || 'development';
  const fallbackColId = columns.find((c) => c.isIntake)?.id ?? columns[0]?.id ?? 'backlog';
  const fallbackPersona = columns.find((c) => !c.isIntake && c.id !== 'backlog' && !c.terminal)?.persona || 'general';
  const parentIdFromId = activeTaskId?.includes('.') ? activeTaskId.split('.')[0] : undefined;
  const parentTaskForFallback = useBoardStore((s) => (parentIdFromId ? s.tasks.find((t) => t.id === parentIdFromId) : undefined));

  // Resilient fallback: ensure task is never undefined if drawer is active
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

  // Reset error when switching tasks & load activity history from disk
  useEffect(() => {
    setRunError(null);
    if (activeTaskId) {
      useDrawerStore.getState().loadTaskActivity(activeTaskId);
    }
  }, [activeTaskId]);

  // Esc key listener to navigate back or close drawer
  useEffect(() => {
    if (!isOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        goBackDrawer();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, goBackDrawer]);

  // Unified Chronological Activity List (Parent stream or Nested Subagent stream)
  const unifiedActivities = useMemo(() => {
    if (activeSubagentId) {
      return activeFeed;
    }

    if (!task) return [];

    if (activeFeed && activeFeed.length > 0) {
      return activeFeed;
    }

    // Fallback: merge old streamEntries and toolExecutions if feed is not populated yet
    const rawStream = activeStream || { thoughts: [], outputs: [] };
    const rawTools = activeTaskTools || [];

    const merged: DrawerActivityItem[] = [
      ...(rawStream.thoughts ?? []).map((th) => ({
        id: th?.id ?? `th-${Date.now()}`,
        type: 'thought' as const,
        timestamp: th?.timestamp ?? '',
        content: th?.content ?? '',
        durationMs: th?.durationMs,
      })),
      ...(rawStream.outputs ?? []).map((out) => ({
        id: out?.id ?? `out-${Date.now()}`,
        type: (out?.type === 'steer' || out?.type === 'system' || out?.type === 'error' || out?.type === 'user'
          ? out.type
          : 'output') as any,
        timestamp: out?.timestamp ?? '',
        content: out?.content ?? '',
      })),
      ...(rawTools ?? []).map((t) => ({
        id: t?.id ?? `tool-${Date.now()}`,
        type: 'tool' as const,
        timestamp: t?.timestamp ?? '',
        tool: t,
      })),
    ];

    return merged;
  }, [task, activeSubagentId, activeFeed, activeStream, activeTaskTools]);

  const effectiveModel = settings.model || 'crouton-alpha-free';
  const selectableModels = useMemo(() => {
    const seen = new Set<string>([effectiveModel]);
    const catalogIds = providers.flatMap((p) => p.models.map((m) => m.id));
    const fallback = catalogIds.length > 0 ? catalogIds : [settings.model || 'crouton-alpha-free'];
    for (const id of fallback) {
      if (id) seen.add(id);
    }
    return Array.from(seen);
  }, [effectiveModel, providers, settings.model]);

  const turns = useMemo(() => {
    if (!task) return 0;
    const feed = activeFeed || [];
    const feedCount = (feed ?? []).filter((a) => a?.type === 'tool' || a?.type === 'thought' || a?.type === 'output').length;
    const toolsCount = activeTaskTools?.length || 0;
    const outputsCount = activeStream?.outputs?.filter((o) => o?.type === 'output').length || 0;
    return Math.max(feedCount, toolsCount + outputsCount, 0);
  }, [task, activeFeed, activeTaskTools, activeStream]);

  if (!isOpen || !task) return null;

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
    } catch (err: any) {
      console.info('run_agent invoke failed or web mode:', err);
      const errMsg = err?.message || (typeof err === 'string' ? err : 'Не удалось запустить агента');
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
        // Granular abort: stop ONLY active subagent without killing parent task
        const resolvedSubagentId =
          useDrawerStore.getState().toolIdToSubagentId[activeSubagentId] || activeSubagentId;
        try {
          await invoke('abort_agent', { taskId: resolvedSubagentId });
        } catch (err: any) {
          abortError = err?.message || (typeof err === 'string' ? err : 'Не удалось остановить сабагента (backend)');
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

      // Root task abort
      try {
        await invoke('abort_agent', { taskId: task.id });
      } catch (err: any) {
        abortError = err?.message || (typeof err === 'string' ? err : 'Не удалось остановить агента (backend)');
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

      // Cascade abort all subagents of this task
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
          } catch (err: any) {
            const errMsg = err?.message || (typeof err === 'string' ? err : 'Ошибка перезапуска субагента');
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
      // Append user steer message into subagent feed
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
      } catch (err: any) {
        const errMsg = err?.message || (typeof err === 'string' ? err : 'Ошибка отправки команды субагенту');
        setRunError(errMsg);
        setActiveAgentTask(activeSubagentId, false);
        setActiveAgentTask(resolvedSubagentId, false);
        return false;
      }
    }

    // Append user steer message into feed immediately with attached images
    const isSteer = targetIsRunning;
    appendOutput(targetId, mainText, isSteer ? 'steer' : 'user', imageUrls.length > 0 ? imageUrls : undefined);
    const optimisticFeed = useDrawerStore.getState().activityFeed[targetId] ?? [];
    const messageId = optimisticFeed[optimisticFeed.length - 1]?.id;

    // Do NOT pollute task description with user steer messages.
    // If task has no description yet, set initial prompt. Otherwise, keep original description untouched.
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
    } catch (err: any) {
      console.info('steer_agent invoke failed or web mode:', err);
      const errMsg = err?.message || (typeof err === 'string' ? err : 'Ошибка отправки команды агенту');
      setRunError(errMsg);
      if (!targetIsRunning) {
        updateTask(targetId, { status: 'idle', isTimerRunning: false });
        setActiveAgentTask(targetId, false);
      }
      return false;
    }
  };

  return (
    <div
      style={getZoomOverlayStyle(zoom)}
      className="fixed inset-0 z-50 flex flex-col overflow-hidden bg-white dark:bg-[#111114] text-[#171717] dark:text-[#f4f4f5] select-none animate-in fade-in duration-150 transition-colors"
    >
      {/* Outer guard: header/banner are outside inner boundaries */}
      <ErrorBoundary scopeLabel="окно чата" onClose={closeDrawer}>
        {/* Count subagents for header badge */}
        {(() => {
          const storeSubagents = useDrawerStore.getState().subagents[task.id] || [];
          const subagentToolsCount = (activeTaskTools || []).filter(
            (t) => t.toolName === 'subagent' || t.toolName === 'subagent_bg'
          ).length;
          const boardSubtasksCount = subtasks.length;
          const subagentsCount = Math.max(storeSubagents.length, subagentToolsCount, boardSubtasksCount);

          return (
            <DrawerHeader
              task={task}
              columns={columns}
              subtasks={subtasks}
              rootTask={rootTask}
              activeTaskId={activeTaskId}
              isRunning={isRunning}
              isDiffCollapsed={isDiffCollapsed}
              isSubagentsCollapsed={isSubagentsCollapsed}
              subagentsCount={subagentsCount}
              onClose={goBackDrawer}
              onAbort={handleAbortAgent}
              onToggleDiff={() => setIsDiffCollapsed((prev) => !prev)}
              onToggleSubagents={() => setIsSubagentsCollapsed((prev) => !prev)}
              onOpenDrawer={openDrawer}
              onOpenToolPolicy={openToolPolicyModal}
              onOpenSettings={openSettings}
            />
          );
        })()}

        {/* Three-Column Layout: Left (SubagentsSidebar) | Center (Feed + Steer Bar) | Right (GitDiffSidebar) */}
        {/* Obeying § 2.13 Viewport Overlay & Flex Boundary Invariants (min-h-0, min-w-0, overflow-hidden) */}
        <div className="flex-1 flex flex-row min-h-0 min-w-0 overflow-hidden bg-[#f4f4f5] dark:bg-[#111114]">
          {/* Left Column: Subagents Interactive Sidebar */}
          <ErrorBoundary scopeLabel="панель субагентов" onClose={() => setIsSubagentsCollapsed(true)}>
            <SubagentsSidebar
              taskId={task.id}
              taskTitle={task.title}
              isCollapsed={isSubagentsCollapsed}
              onToggleCollapse={() => setIsSubagentsCollapsed((prev) => !prev)}
            />
          </ErrorBoundary>

          {/* Center Column: Activity Feed + Steer Bar - DSH-style inset cockpit card */}
          <div className="flex-1 flex flex-col min-h-0 min-w-0 bg-[#fafafa] dark:bg-[#0b0b0e] overflow-hidden transition-colors rounded-xl border border-[#ebebeb] dark:border-[#212126] shadow-2xs my-1.5 mx-1.5">
            <ErrorBoundary scopeLabel="чат задачи" onClose={closeDrawer}>
              {/* Modular Activity Stream: Objective card, Relay handoff, Live thinking, Tools, Subagent cards */}
              <DrawerActivityStream
                task={task}
                unifiedActivities={unifiedActivities}
                isRunning={isRunning}
                runError={runError}
                onRunAgent={handleRunAgent}
                onClearRunError={() => setRunError(null)}
                subtasks={subtasks}
                onOpenDrawer={openDrawer}
              />

              {/* Modular Steer Bar: Floating light card, Textarea, Action button, Model selector, ToDos pill, ContextGauge */}
              <DrawerSteerBar
                task={task}
                isRunning={isRunning}
                turns={turns}
                onSendPrompt={handleSendPrompt}
                onAbortAgent={handleAbortAgent}
                isToolAllowed={isToolAllowed}
                selectableModels={selectableModels}
                onSelectEffort={(effort) => {
                  if (activeSubagentId) {
                    const isSubBoardTask = subtasks.some((s) => s.id === activeSubagentId);
                    const resolvedSubId =
                      useDrawerStore.getState().toolIdToSubagentId[activeSubagentId] || activeSubagentId;
                    if (isSubBoardTask) {
                      updateTask(activeSubagentId, { reasoningEffort: effort });
                    } else {
                      useDrawerStore.getState().updateSubagentConfig(activeSubagentId, { reasoningEffort: effort });
                      void (async () => {
                        try {
                          const { invoke } = await import('@tauri-apps/api/core');
                          await invoke('update_task', {
                            taskId: resolvedSubId,
                            model: null,
                            providerId: null,
                            reasoningEffort: effort,
                            title: null,
                            description: null,
                            status: null,
                            verdict: null,
                          });
                        } catch {
                          // ignore in web mode
                        }
                      })();
                    }
                    useDrawerStore.getState().appendOutput(activeSubagentId, `Effort изменён на ${effort}`, 'system');
                    if (resolvedSubId !== activeSubagentId) {
                      useDrawerStore.getState().appendOutput(resolvedSubId, `Effort изменён на ${effort}`, 'system');
                    }
                    return;
                  }
                  updateTask(task.id, { reasoningEffort: effort });
                  useDrawerStore.getState().appendOutput(task.id, `Effort изменён на ${effort}`, 'system');
                }}
                onSelectPair={(providerId, m) => {
                  if (activeSubagentId) {
                    const isSubBoardTask = subtasks.some((s) => s.id === activeSubagentId);
                    const resolvedSubId =
                      useDrawerStore.getState().toolIdToSubagentId[activeSubagentId] || activeSubagentId;
                    const subBoardTask = subtasks.find((s) => s.id === activeSubagentId);
                    const subCfg =
                      useDrawerStore.getState().subagentConfigs?.[activeSubagentId] ||
                      useDrawerStore.getState().subagentConfigs?.[resolvedSubId];
                    const prevSubModel = subBoardTask?.model || subCfg?.model || task.model || effectiveModel;
                    if (isSubBoardTask) {
                      updateTask(activeSubagentId, { model: m, providerId });
                    } else {
                      useDrawerStore.getState().updateSubagentConfig(activeSubagentId, { model: m, providerId });
                      void (async () => {
                        try {
                          const { invoke } = await import('@tauri-apps/api/core');
                          await invoke('update_task', {
                            taskId: resolvedSubId,
                            model: m,
                            providerId,
                            reasoningEffort: null,
                            title: null,
                            description: null,
                            status: null,
                            verdict: null,
                          });
                        } catch {
                          // ignore in web mode
                        }
                      })();
                    }
                    if (prevSubModel !== m) {
                      const prov = useSettingsStore.getState().providers.find((p) => p.id === providerId);
                      const display = prov?.models.find((model) => model.id === m)?.displayName || m;
                      const label = prov ? `${prov.name} / ${display}` : display;
                      useDrawerStore.getState().appendOutput(activeSubagentId, `Модель изменена на ${label}`, 'system');
                      if (resolvedSubId !== activeSubagentId) {
                        useDrawerStore.getState().appendOutput(resolvedSubId, `Модель изменена на ${label}`, 'system');
                      }
                    }
                    return;
                  }
                  const prevModel = task.model || effectiveModel;
                  updateTask(task.id, { model: m, providerId });
                  void (async () => {
                    const prov = useSettingsStore.getState().providers.find((p) => p.id === providerId);
                    if (prevModel !== m) {
                      const display = prov?.models.find((model) => model.id === m)?.displayName || m;
                      const label = prov ? `${prov.name} / ${display}` : display;
                      useDrawerStore.getState().appendOutput(task.id, `Модель изменена на ${label}`, 'system');
                    }
                  })();
                }}
              />
            </ErrorBoundary>
          </div>

          {/* Right Column: Git Diff Interactive Sidebar */}
          <ErrorBoundary scopeLabel="Git Diff" onClose={() => setIsDiffCollapsed(true)}>
            <GitDiffSidebar
              taskId={task.id}
              isCollapsed={isDiffCollapsed}
              onToggleCollapse={() => setIsDiffCollapsed((prev) => !prev)}
            />
          </ErrorBoundary>
        </div>

      </ErrorBoundary>
    </div>
  );
}

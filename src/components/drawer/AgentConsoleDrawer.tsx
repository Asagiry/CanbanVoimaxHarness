import { useState, useEffect, useMemo } from 'react';
import type { KanbanTask } from '../../types';
import { useDrawerStore, DrawerActivityItem } from '../../stores/drawerStore';
import { useBoardStore, syncTasksBackend } from '../../stores/boardStore';
import { useSettingsStore } from '../../stores/settingsStore';
import { useToolPolicyStore } from '../../stores/toolPolicyStore';
import ErrorBoundary from '../common/ErrorBoundary';
import DrawerHeader from './DrawerHeader';
import DrawerActivityStream from './DrawerActivityStream';
import DrawerSteerBar from './DrawerSteerBar';
import GitDiffSidebar from './GitDiffSidebar';
import SubagentsSidebar from './SubagentsSidebar';

export default function AgentConsoleDrawer() {
  const {
    isOpen,
    activeTaskId,
    activityFeed,
    streamEntries,
    toolExecutions,
    closeDrawer,
    openDrawer,
    appendOutput,
    activeSubagentId,
    activeAgentTaskIds,
    setActiveAgentTask,
  } = useDrawerStore();

  const { tasks, updateTask, columns } = useBoardStore();
  const { settings, availableModels, openSettings } = useSettingsStore();
  const isToolAllowed = useToolPolicyStore((s) => s.isToolAllowed);
  const openToolPolicyModal = useToolPolicyStore((s) => s.openModal);

  const [runError, setRunError] = useState<string | null>(null);
  const [isDiffCollapsed, setIsDiffCollapsed] = useState(false);
  const [isSubagentsCollapsed, setIsSubagentsCollapsed] = useState(false);

  const rawTask = tasks.find((t) => t.id === activeTaskId);
  const fallbackBoardId = useBoardStore.getState().activeBoardId || 'development';
  const fallbackColId = columns[0]?.id ?? 'backlog';
  // Resilient fallback: ensure task is never undefined if drawer is active
  const task: KanbanTask = rawTask ?? {
    id: activeTaskId || 'task',
    title: activeTaskId || 'Task',
    columnId: fallbackColId,
    boardId: fallbackBoardId,
    persona: 'developer',
    status: 'idle',
    priority: 'high',
    branch: `feat/${(activeTaskId || 'task').toLowerCase()}`,
    worktreePath: `worktrees/${(activeTaskId || 'task').toLowerCase()}`,
    createdAt: Date.now(),
    updatedAt: Date.now(),
    activeStageDuration: 0,
    stageDurations: {},
    totalElapsed: 0,
    isTimerRunning: false,
    returnsCount: 0,
    verdict: 'NONE',
  };

  const rootTask = task.parentId
    ? tasks.find((t) => t.id === task.parentId) || task
    : task;
  const subtasks = tasks.filter((t) => t.parentId === rootTask.id);

  // Reset error when switching tasks & load dialog history from disk
  useEffect(() => {
    setRunError(null);
    if (activeTaskId) {
      useDrawerStore.getState().loadTaskDialog(activeTaskId);
    }
  }, [activeTaskId]);

  // Listen to agent error & status events from Tauri backend
  useEffect(() => {
    if (!task?.id) return;
    let unlistenStatus: (() => void) | undefined;
    let unlistenFinished: (() => void) | undefined;
    let unlistenError: (() => void) | undefined;

    const setupErrorListeners = async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');

        // Listen for agent:status terminal states
        unlistenStatus = await listen<{ taskId?: string; status?: string }>('agent:status', (event) => {
          if (event.payload?.taskId === task.id) {
            const st = event.payload.status;
            // Immediate reset of running status on terminal states
            if (st === 'finished' || st === 'aborted' || st === 'error') {
              useBoardStore.getState().updateTaskStatus(task.id, 'idle');
              useDrawerStore.getState().setActiveAgentTask(task.id, false);
              try {
                const ds = useDrawerStore.getState();
                ds.setGenerationEndTime(task.id, Date.now());
                ds.setTaskEndTime(task.id, Date.now());
                if (ds.finalTps[task.id] == null) {
                  const tele = ds.taskTelemetry[task.id];
                  const outTokens = tele?.outputTokens || ds.streamedTurnTokens[task.id] || 0;
                  const start = ds.generationStartTime[task.id] ?? ds.thinkingStartTime[task.id];
                  const end = ds.generationEndTime[task.id] ?? Date.now();
                  if (start != null && outTokens > 0) {
                    const elapsedSec = (end - start) / 1000;
                    if (elapsedSec > 0.05) {
                      ds.recordTurnMetrics(task.id, outTokens, elapsedSec);
                    }
                  }
                }
              } catch {
                // ignore
              }
            }
            if (st === 'error') {
              setRunError((prev) => prev || 'Агент сообщил об ошибке выполнения');
            } else if (st === 'running') {
              setRunError(null);
            }
          }
        });

        // Listen for agent:finished
        unlistenFinished = await listen<{ taskId?: string; response?: string }>('agent:finished', (event) => {
          if (event.payload?.taskId === task.id) {
            useBoardStore.getState().updateTaskStatus(task.id, 'idle');
            useDrawerStore.getState().setActiveAgentTask(task.id, false);
            try {
              const ds = useDrawerStore.getState();
              ds.setThinkingStartTime(task.id, null);
            } catch {
              // ignore
            }
          }
        });

        // Listen for agent:error
        unlistenError = await listen<{ taskId?: string; error?: string }>('agent:error', (event) => {
          if (event.payload && (!event.payload.taskId || event.payload.taskId === task.id)) {
            setRunError(event.payload.error || 'Произошла непредвиденная ошибка агента');
            useBoardStore.getState().updateTaskStatus(task.id, 'idle');
            if (event.payload.taskId) {
              useDrawerStore.getState().setActiveAgentTask(event.payload.taskId, false);
            } else if (task?.id) {
              useDrawerStore.getState().setActiveAgentTask(task.id, false);
            }
            try {
              const ds = useDrawerStore.getState();
              ds.setThinkingStartTime(task.id, null);
            } catch {
              // ignore
            }
          }
        });
      } catch {
        // web mode fallback
      }
    };

    setupErrorListeners();
    return () => {
      if (unlistenStatus) unlistenStatus();
      if (unlistenFinished) unlistenFinished();
      if (unlistenError) unlistenError();
    };
  }, [task?.id, updateTask]);

  // Esc key listener to close drawer
  useEffect(() => {
    if (!isOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        closeDrawer();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, closeDrawer]);

  // Unified Chronological Activity List (Parent stream or Nested Subagent stream)
  const unifiedActivities = useMemo(() => {
    if (activeSubagentId) {
      let subFeed = activityFeed[activeSubagentId] || [];
      if (subFeed.length === 0) {
        const state = useDrawerStore.getState();
        const mappedId = state.toolIdToSubagentId[activeSubagentId];
        if (mappedId && activityFeed[mappedId]?.length) {
          subFeed = activityFeed[mappedId];
        } else {
          for (const [toolId, subId] of Object.entries(state.toolIdToSubagentId)) {
            if (subId === activeSubagentId && activityFeed[toolId]?.length) {
              subFeed = activityFeed[toolId];
              break;
            }
          }
        }
      }
      return subFeed.filter((item) => item.type !== 'prompt');
    }

    if (!task) return [];

    const feed = activityFeed[task.id];
    if (feed && feed.length > 0) {
      return feed;
    }

    // Fallback: merge old streamEntries and toolExecutions if feed is not populated yet
    const rawStream = streamEntries[task.id] || { thoughts: [], outputs: [] };
    const rawTools = toolExecutions[task.id] || [];

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
  }, [task, activeSubagentId, activityFeed, streamEntries, toolExecutions]);

  const effectiveModel = task?.model || settings.model || 'gemini-3.8-flash-high';
  const selectableModels = useMemo(() => {
    return Array.from(
      new Set([
        effectiveModel,
        ...(availableModels || []),
        'gemini-3.8-flash-high',
        'gemini-2.5-pro',
        'claude-3-7-sonnet-20250219',
        'gpt-4o',
      ])
    );
  }, [effectiveModel, availableModels]);

  const turns = useMemo(() => {
    if (!task) return 0;
    const feed = activityFeed[task.id] || [];
    const feedCount = (feed ?? []).filter((a) => a?.type === 'tool' || a?.type === 'thought' || a?.type === 'output').length;
    const toolsCount = toolExecutions[task.id]?.length || 0;
    const outputsCount = streamEntries[task.id]?.outputs?.filter((o) => o?.type === 'output').length || 0;
    return Math.max(feedCount, toolsCount + outputsCount, 0);
  }, [task, activityFeed, toolExecutions, streamEntries]);

  if (!isOpen || !task) return null;

  const hasRunningTool = Boolean(
    toolExecutions[task.id]?.some((t) => t.status === 'running') ||
    (activeSubagentId && toolExecutions[activeSubagentId]?.some((t) => t.status === 'running'))
  );

  const isAgentActive = Boolean(
    activeAgentTaskIds?.[task.id] ||
    (activeSubagentId && activeAgentTaskIds?.[activeSubagentId])
  );

  const isRunning = task.status === 'running' || hasRunningTool || isAgentActive;

  const handleRunAgent = async () => {
    setRunError(null);
    updateTask(task.id, { status: 'running', isTimerRunning: true });
    setActiveAgentTask(task.id, true);
    try {
      await syncTasksBackend(useBoardStore.getState().tasks);
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
    try {
      const { invoke } = await import('@tauri-apps/api/core');

      if (activeSubagentId) {
        // Granular abort: stop ONLY active subagent without killing parent task
        const resolvedSubagentId =
          useDrawerStore.getState().toolIdToSubagentId[activeSubagentId] || activeSubagentId;
        await invoke('abort_agent', { taskId: resolvedSubagentId });
        setActiveAgentTask(activeSubagentId, false);
        setActiveAgentTask(resolvedSubagentId, false);

        const boardSub = subtasks.find((s) => s.id === activeSubagentId || s.id === resolvedSubagentId);
        if (boardSub) {
          updateTask(boardSub.id, { status: 'paused', isTimerRunning: false });
        }

        const subTools = [
          ...(toolExecutions[activeSubagentId] || []),
          ...(toolExecutions[resolvedSubagentId] || []),
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
        return;
      }

      // Root task abort
      await invoke('abort_agent', { taskId: task.id });
      updateTask(task.id, { status: 'paused', isTimerRunning: false });
      setActiveAgentTask(task.id, false);
      const currTools = toolExecutions[task.id] || [];
      for (const t of currTools) {
        if (t.status === 'running') {
          useDrawerStore.getState().updateToolExecution(task.id, t.id, {
            status: 'failed',
            stderr: 'Прервано пользователем',
          });
        }
      }
    } catch (err) {
      console.info('abort_agent invoke failed or web mode:', err);
    }
  };

  const handleSendPrompt = async (
    promptText: string,
    images: Array<{ id: string; name: string; dataUrl: string }>
  ) => {
    const trimmed = promptText.trim();
    if (!trimmed && images.length === 0) return;

    const imageUrls = images.map((img) => img.dataUrl);
    setRunError(null);

    const mainText = trimmed || (images.length > 0 ? 'Изображение прикреплено' : '');
    const imageNotice = images.length > 0
      ? `\n[Прикреплено ${images.length} изображений: ${images.map((i) => i.name).join(', ')}]`
      : '';
    const fullMessage = `${mainText}${imageNotice}`;

    setActiveAgentTask(task.id, true);
    if (activeSubagentId) {
      setActiveAgentTask(activeSubagentId, true);
      // Append user steer message into subagent feed
      appendOutput(activeSubagentId, mainText, 'user', imageUrls.length > 0 ? imageUrls : undefined);
      useDrawerStore.getState().flushSaveDialog(activeSubagentId);

      const resolvedSubagentId =
        useDrawerStore.getState().toolIdToSubagentId[activeSubagentId] || activeSubagentId;

      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('retry_subagent', {
          parentTaskId: task.id,
          subagentId: resolvedSubagentId,
          message: fullMessage,
        });
      } catch (err) {
        console.info('retry_subagent invoke error or web mode:', err);
      }
      return;
    }

    // Append user steer message into feed immediately with attached images
    const isSteer = isRunning;
    appendOutput(task.id, mainText, isSteer ? 'steer' : 'user', imageUrls.length > 0 ? imageUrls : undefined);
    useDrawerStore.getState().flushSaveDialog(task.id);

    // Do NOT pollute task description with user steer messages.
    // If task has no description yet, set initial prompt. Otherwise, keep original description untouched.
    if (!task.description) {
      updateTask(task.id, { description: fullMessage, status: 'running', isTimerRunning: true });
    } else {
      updateTask(task.id, { status: 'running', isTimerRunning: true });
    }

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await syncTasksBackend(useBoardStore.getState().tasks);
      if (isRunning) {
        await invoke('steer_agent', { taskId: task.id, message: fullMessage });
      } else {
        await invoke('run_agent', { taskId: task.id, prompt: fullMessage });
      }
    } catch (err: any) {
      console.info('steer_agent / run_agent invoke failed or web mode:', err);
      const errMsg = err?.message || (typeof err === 'string' ? err : 'Ошибка отправки команды агенту');
      setRunError(errMsg);
      if (!isRunning) {
        updateTask(task.id, { status: 'idle', isTimerRunning: false });
      }
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex flex-col h-screen max-h-screen overflow-hidden bg-white dark:bg-[#0d1117] text-[#171717] dark:text-[#f0f6fc] select-none animate-in fade-in duration-150 transition-colors">
      {/* Outer guard: header/banner are outside inner boundaries */}
      <ErrorBoundary scopeLabel="окно чата" onClose={closeDrawer}>
        {/* Count subagents for header badge */}
        {(() => {
          const storeSubagents = useDrawerStore.getState().subagents[task.id] || [];
          const subagentToolsCount = (toolExecutions[task.id] || []).filter(
            (t) => t.toolName === 'subagent' || t.toolName === 'subagent_bg'
          ).length;
          const boardSubtasksCount = tasks.filter((t) => t.parentId === task.id).length;
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
              onClose={closeDrawer}
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
        <div className="flex-1 flex flex-row min-h-0 min-w-0 overflow-hidden">
          {/* Left Column: Subagents Interactive Sidebar */}
          <ErrorBoundary scopeLabel="панель субагентов" onClose={() => setIsSubagentsCollapsed(true)}>
            <SubagentsSidebar
              taskId={task.id}
              taskTitle={task.title}
              isCollapsed={isSubagentsCollapsed}
              onToggleCollapse={() => setIsSubagentsCollapsed((prev) => !prev)}
            />
          </ErrorBoundary>

          {/* Center Column: Activity Feed + Steer Bar */}
          <div className="flex-1 flex flex-col min-h-0 min-w-0 bg-[#fafafa] dark:bg-[#090a0f] overflow-hidden transition-colors">
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
                effectiveModel={effectiveModel}
                selectableModels={selectableModels}
                onSelectModel={(m) => updateTask(task.id, { model: m })}
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

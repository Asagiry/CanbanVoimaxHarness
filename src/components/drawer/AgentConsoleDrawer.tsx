import { useState, useEffect } from 'react';
import { useDrawerStore } from '../../stores/drawerStore';
import { useSettingsStore } from '../../stores/settingsStore';
import { useToolPolicyStore } from '../../stores/toolPolicyStore';
import { useCortexStore } from '../../stores/cortexStore';
import ErrorBoundary from '../common/ErrorBoundary';
import DrawerHeader from './DrawerHeader';
import DrawerActivityStream from './DrawerActivityStream';
import DrawerSteerBar from './DrawerSteerBar';
import GitDiffSidebar from './GitDiffSidebar';
import SubagentsSidebar from './SubagentsSidebar';
import { useUiZoom, getZoomOverlayStyle } from '../../hooks/useUiZoom';
import { useDrawerTaskState } from './useDrawerTaskState';
import { useUnifiedActivities } from './useUnifiedActivities';
import { useAgentExecution } from './useAgentExecution';

export default function AgentConsoleDrawer() {
  const { zoom } = useUiZoom();
  const isOpen = useDrawerStore((s) => s.isOpen);
  const activeTaskId = useDrawerStore((s) => s.activeTaskId);
  const activeSubagentId = useDrawerStore((s) => s.activeSubagentId);
  const closeDrawer = useDrawerStore((s) => s.closeDrawer);
  const goBackDrawer = useDrawerStore((s) => s.goBackDrawer);
  const openDrawer = useDrawerStore((s) => s.openDrawer);

  const openSettings = useSettingsStore((s) => s.openSettings);
  const isToolAllowed = useToolPolicyStore((s) => s.isToolAllowed);
  const openToolPolicyModal = useToolPolicyStore((s) => s.openModal);

  const [isDiffCollapsed, setIsDiffCollapsed] = useState(false);
  const [isSubagentsCollapsed, setIsSubagentsCollapsed] = useState(false);

  const {
    task,
    rootTask,
    subtasks,
    columns,
    updateTask,
    selectableModels,
    effectiveModel,
  } = useDrawerTaskState(activeTaskId);

  const {
    activeTaskTools,
    activeSubagentTools,
    isAgentActive,
    unifiedActivities,
    turns,
  } = useUnifiedActivities(activeTaskId, activeSubagentId, task);

  const {
    isRunning,
    runError,
    setRunError,
    handleRunAgent,
    handleAbortAgent,
    handleSendPrompt,
  } = useAgentExecution({
    task,
    subtasks,
    updateTask,
    activeTaskId,
    activeSubagentId,
    activeTaskTools,
    activeSubagentTools,
    isAgentActive,
  });

  // Reset error when switching tasks & load activity history from disk
  useEffect(() => {
    setRunError(null);
    if (activeTaskId) {
      useDrawerStore.getState().loadTaskActivity(activeTaskId);
    }
  }, [activeTaskId, setRunError]);

  // Esc key listener to navigate back or close drawer (yields to stacked modals/popovers)
  useEffect(() => {
    if (!isOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key !== 'Escape' || e.defaultPrevented) return;
      if (
        useSettingsStore.getState().isOpen ||
        useToolPolicyStore.getState().isModalOpen ||
        useCortexStore.getState().isEditorOpen
      ) {
        return;
      }
      goBackDrawer();
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, goBackDrawer]);

  if (!isOpen || !task) return null;

  return (
    <div
      style={getZoomOverlayStyle(zoom)}
      className="fixed inset-0 z-50 flex flex-col overflow-hidden bg-white dark:bg-[#111114] text-[#171717] dark:text-[#f4f4f5] select-none animate-in fade-in duration-150 transition-colors"
    >
      <ErrorBoundary scopeLabel="окно чата" onClose={closeDrawer}>
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

        <div className="flex-1 flex flex-row min-h-0 min-w-0 overflow-hidden bg-[#f4f4f5] dark:bg-[#111114]">
          <ErrorBoundary scopeLabel="панель субагентов" onClose={() => setIsSubagentsCollapsed(true)}>
            <SubagentsSidebar
              taskId={task.id}
              taskTitle={task.title}
              isCollapsed={isSubagentsCollapsed}
              onToggleCollapse={() => setIsSubagentsCollapsed((prev) => !prev)}
            />
          </ErrorBoundary>

          <div className="flex-1 flex flex-col min-h-0 min-w-0 bg-[#fafafa] dark:bg-[#0b0b0e] overflow-hidden transition-colors rounded-xl border border-[#ebebeb] dark:border-[#212126] shadow-2xs my-1.5 mx-1.5">
            <ErrorBoundary scopeLabel="чат задачи" onClose={closeDrawer}>
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
                        } catch (err) {
                          console.debug('[AgentConsoleDrawer] update_task effort skipped in web mode:', err);
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
                        } catch (err) {
                          console.debug('[AgentConsoleDrawer] update_task model skipped in web mode:', err);
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

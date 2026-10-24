import React from 'react';
import type { KanbanTask, PipelineColumn } from '../../types';
import Logo from '../common/Logo';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import { useCortexStore } from '../../stores/cortexStore';
import AgentIcon from '../common/AgentIcon';
import { getRoleBadgeStyle } from '../../lib/agentPalette';
import { formatSubagentDisplayTitle, formatSubagentRoleDisplay, getSubagentOrderMap } from './SubagentsSidebar';
import WindowControls, { toggleMaximizeWindow } from '../layout/WindowControls';
import {
  ArrowLeft,
  ChevronRight,
  Check,
  Wrench,
  Settings,
  FolderGit2,
  GitBranch,
  Sun,
  Moon,
  Layers,
  Zap,
} from 'lucide-react';
import { useThemeStore } from '../../stores/themeStore';
import { useTranslation } from '../../stores/languageStore';

export interface DrawerHeaderProps {
  task: KanbanTask;
  columns: PipelineColumn[];
  subtasks: KanbanTask[];
  rootTask: KanbanTask | null;
  activeTaskId: string | null;
  isRunning?: boolean;
  isDiffCollapsed?: boolean;
  isSubagentsCollapsed?: boolean;
  subagentsCount?: number;
  onClose: () => void;
  onAbort?: () => void;
  onAbortAgent?: () => void;
  onToggleDiff?: () => void;
  onToggleSubagents?: () => void;
  onOpenDrawer: (taskId: string) => void;
  onOpenToolPolicy: () => void;
  onOpenSettings: () => void;
}

export default function DrawerHeader({
  task,
  columns,
  subtasks,
  rootTask,
  activeTaskId,
  isRunning = false,
  isDiffCollapsed: _isDiffCollapsed = true,
  isSubagentsCollapsed: _isSubagentsCollapsed = false,
  subagentsCount: _subagentsCount = 0,
  onClose,
  onAbort: _onAbort,
  onAbortAgent: _onAbortAgent,
  onToggleDiff: _onToggleDiff,
  onToggleSubagents: _onToggleSubagents,
  onOpenDrawer,
  onOpenToolPolicy,
  onOpenSettings,
}: DrawerHeaderProps) {
  const handleHeaderDoubleClick = (e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    if (target.closest('button, input, select, textarea, [data-no-drag]')) {
      return;
    }
    void toggleMaximizeWindow();
  };

  const activeSubagentId = useDrawerStore((s) => s.activeSubagentId);
  const activeSubagentTitle = useDrawerStore((s) => s.activeSubagentTitle);
  const activeSubagentRole = useDrawerStore((s) => s.activeSubagentRole);
  const closeSubagent = useDrawerStore((s) => s.closeSubagent);
  const storeSubagents = useDrawerStore((s) => s.subagents);
  const toolExecutions = useDrawerStore((s) => s.toolExecutions);
  const toolIdToSubagentId = useDrawerStore((s) => s.toolIdToSubagentId);
  const boardTasks = useBoardStore((s) => s.tasks);
  const agentPersonas = useCortexStore((s) => s.agentPersonas);
  const matchedPersona = agentPersonas.find(
    (p) => p.role.toLowerCase() === (activeSubagentRole || '').toLowerCase()
  );
  const { theme, toggleThemeWithAnimation } = useThemeStore();
  const { language, toggleLanguage, t } = useTranslation();

  const subagentNumber = React.useMemo(() => {
    if (!activeSubagentId || !task?.id) return null;
    const map = getSubagentOrderMap(task.id, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks);
    return map.get(activeSubagentId) ?? (toolIdToSubagentId[activeSubagentId] ? map.get(toolIdToSubagentId[activeSubagentId]) : null) ?? 1;
  }, [activeSubagentId, task?.id, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks]);

  const isBoardSubtask = React.useMemo(() => {
    if (!activeSubagentId) return false;
    return boardTasks.some((t) => t.id === activeSubagentId);
  }, [activeSubagentId, boardTasks]);

  const activeSubagentIsBg = React.useMemo(() => {
    if (!activeSubagentId || !task?.id) return false;
    const subs = storeSubagents[task.id] || [];
    const directSub = subs.find((s) => s.id === activeSubagentId || s.toolCallId === activeSubagentId);
    if (directSub?.isBg !== undefined) return directSub.isBg;
    const tools = toolExecutions[task.id] || [];
    const t = tools.find((x) => x.id === activeSubagentId || (x.params as any)?.subagent_id === activeSubagentId);
    if (t) {
      const p = t.params as any;
      return t.toolName === 'subagent_bg' || p?.is_bg === true || p?.isBg === true;
    }
    return false;
  }, [activeSubagentId, task?.id, storeSubagents, toolExecutions]);

  const workingColumns = columns.filter(
    (c) => c.id !== 'backlog' && c.id !== 'ready' && !c.terminal
  );
  const currentStageIdx = columns.findIndex((c) => c?.id === task?.columnId);

  return (
    <header
      data-tauri-drag-region
      onDoubleClick={handleHeaderDoubleClick}
      className="h-12 border-b border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] pl-4 pr-0 shrink-0 z-10 shadow-xs flex items-center select-none transition-colors"
    >
      <div className="flex items-center justify-between w-full h-full" data-tauri-drag-region>
        {/* Left: Logo + Back + Breadcrumbs/Navigation + Worktree Badges */}
        <div className="flex items-center space-x-2 shrink-0 select-none pointer-events-auto" data-no-drag>
          <Logo size={18} className="shrink-0 select-none" />
          <button
            data-no-drag
            type="button"
            onMouseDown={(e) => e.stopPropagation()}
            onClick={activeSubagentId ? closeSubagent : onClose}
            className="flex items-center space-x-1.5 px-3 py-1 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#161b22] dark:hover:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc] border border-[#ebebeb] dark:border-[#30363d] transition font-sans text-xs font-medium cursor-pointer shadow-xs select-none pointer-events-auto"
            aria-label={activeSubagentId ? t('drawer.backToTask') : t('drawer.backToBoard')}
          >
            <ArrowLeft className="h-4 w-4 text-[#171717] dark:text-[#f0f6fc]" />
            <span>{activeSubagentId ? t('drawer.backToTask') : t('drawer.backToBoard')}</span>
          </button>

          {/* Breadcrumbs / Task badge (only when NOT in subagent mode) */}
          {!activeSubagentId && task?.id && (
            rootTask && rootTask.id !== task.id ? (
              <div className="flex items-center space-x-1 shrink-0 select-none pointer-events-auto" data-no-drag>
                <button
                  type="button"
                  onClick={() => onOpenDrawer(rootTask.id)}
                  className="font-mono text-xs text-[#71717a] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] px-1.5 py-0.5 rounded hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] transition cursor-pointer"
                  aria-label={t('drawer.backToParentTitle', { id: rootTask.id })}
                >
                  {rootTask.id}
                </button>
                <ChevronRight className="h-3 w-3 text-[#a1a1aa] dark:text-[#6e7681] shrink-0" />
                <span className="font-mono text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] px-2 py-0.5 rounded bg-[#f4f4f5] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] shrink-0">
                  {task.id}
                </span>
              </div>
            ) : (
              <span className="font-mono text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] px-2 py-0.5 rounded bg-[#f4f4f5] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] shrink-0 select-none pointer-events-auto">
                {task.id}
              </span>
            )
          )}

          {/* Directly adjacent to the title/breadcrumbs, render the compact worktree/branch badges */}
          {!activeSubagentId && !task?.skipWorktree && task?.worktreePath && (
            <div className="flex items-center space-x-1.5 px-2 py-0.5 rounded-md bg-[#f4f4f5] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[11px] font-mono shrink-0">
              <FolderGit2 className="h-3 w-3 text-[#8f8f8f] dark:text-[#8b949e]" />
              <span className="text-[#4d4d4d] dark:text-[#c9d1d9] max-w-[140px] truncate">{task.worktreePath}</span>
            </div>
          )}
          {!activeSubagentId && !task?.skipWorktree && task?.branch && (
            <div className="flex items-center space-x-1.5 px-2 py-0.5 rounded-md bg-[#f4f4f5] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[11px] font-mono shrink-0">
              <GitBranch className="h-3 w-3 text-[#8f8f8f] dark:text-[#8b949e]" />
              <span className="text-[#4d4d4d] dark:text-[#c9d1d9] max-w-[140px] truncate">{task.branch}</span>
            </div>
          )}
        </div>

        {/* Center: Stage Stepper OR Centered Subagent Breadcrumb/Badge */}
        <div
          className="flex-1 flex items-center justify-center space-x-2 px-2 overflow-hidden select-none"
          data-tauri-drag-region
        >
          {activeSubagentId ? (
            <div
              data-no-drag
              className="flex items-center space-x-2 px-3 py-1 rounded-md bg-[#f4f4f5] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-xs font-mono max-w-xl mx-auto shadow-2xs select-none pointer-events-auto min-w-0"
            >
              <span className="flex items-center space-x-1.5 font-semibold text-[#171717] dark:text-[#f0f6fc] truncate min-w-0">
                {activeSubagentIsBg ? (
                  <div className="relative shrink-0 flex items-center justify-center">
                    <Layers className="h-3.5 w-3.5 text-purple-600 dark:text-purple-400" />
                    <span className="absolute -bottom-0.5 -right-0.5 h-1.5 w-1.5 rounded-full bg-purple-500 ring-1 ring-white dark:ring-[#161b22]" />
                  </div>
                ) : isBoardSubtask ? (
                  <AgentIcon name={matchedPersona?.icon} className="h-3.5 w-3.5 text-[#171717] dark:text-[#f0f6fc] shrink-0" />
                ) : (
                  <div className="relative shrink-0 flex items-center justify-center">
                    <Zap className="h-3.5 w-3.5 text-sky-600 dark:text-sky-400 fill-sky-500/20" />
                    <span className="absolute -bottom-0.5 -right-0.5 h-1.5 w-1.5 rounded-full bg-sky-500 ring-1 ring-white dark:ring-[#161b22]" />
                  </div>
                )}
                <span className="truncate" title={activeSubagentTitle || undefined}>
                  {formatSubagentDisplayTitle(activeSubagentTitle, language)}
                </span>
              </span>
              <span
                style={getRoleBadgeStyle(activeSubagentRole, matchedPersona?.color)}
                className="px-2 py-0.5 rounded text-[11px] font-mono font-bold uppercase tracking-wider border shrink-0 select-none"
              >
                {formatSubagentRoleDisplay(activeSubagentRole, subagentNumber || 1, isBoardSubtask, language)}
              </span>
            </div>
          ) : task && workingColumns.length > 0 ? (
            <div
              data-no-drag
              className="flex items-center gap-0.5 px-1.5 py-0.5 rounded-lg bg-[#f9fafb] dark:bg-[#161b22] shrink-0 select-none pointer-events-auto shadow-2xs overflow-x-auto no-scrollbar max-w-2xl"
            >
              {workingColumns.map((col, idx) => {
                const matchingSubtask = subtasks.find(
                  (s) => s.columnId === col.id || (s as any).stageId === col.id
                );
                const isRootStage = rootTask ? rootTask.columnId === col.id : false;
                const targetTaskId = matchingSubtask
                  ? matchingSubtask.id
                  : isRootStage && rootTask
                  ? rootTask.id
                  : null;

                const isCurrent =
                  col.id === task.columnId ||
                  (targetTaskId != null && targetTaskId === activeTaskId);
                const isRunningStage = isCurrent && isRunning;
                const isPassed =
                  Boolean(targetTaskId) &&
                  (matchingSubtask?.status === 'done' ||
                    (currentStageIdx >= 0 &&
                      columns.findIndex((c) => c.id === col.id) < currentStageIdx));

                return (
                  <div key={col.id} className="flex items-center shrink-0">
                    {idx > 0 && (
                      <ChevronRight className="h-3 w-3 text-[#a1a1aa] dark:text-[#6e7681] mx-0.5 shrink-0" />
                    )}
                    <button
                      data-no-drag
                      type="button"
                      disabled={isCurrent || !targetTaskId}
                      onMouseDown={(e) => e.stopPropagation()}
                      onClick={(e) => {
                        e.stopPropagation();
                        if (targetTaskId && targetTaskId !== activeTaskId) {
                          onOpenDrawer(targetTaskId);
                        }
                      }}
                      aria-label={
                        isCurrent
                          ? t('drawer.stageCurrent', { title: col.title })
                          : targetTaskId
                          ? t('drawer.stageSwitch', { title: col.title })
                          : t('drawer.stageNotStarted')
                      }
                      className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-xs font-mono transition-all duration-150 tabular-nums ${
                        isCurrent
                          ? 'bg-[#f4f4f5] dark:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] font-semibold shadow-2xs cursor-default'
                          : targetTaskId
                          ? isPassed
                            ? 'bg-[#f0fdf4] dark:bg-[#12281e] text-[#15803d] dark:text-[#7ee787] border border-[#bbf7d0] dark:border-[#238636] hover:bg-[#dcfce7] dark:hover:bg-[#173a2b] cursor-pointer active:scale-95'
                            : 'bg-white dark:bg-[#161b22] text-[#171717] dark:text-[#c9d1d9] border border-[#ebebeb] dark:border-[#30363d] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:border-[#d4d4d8] dark:hover:border-[#484f58] cursor-pointer active:scale-95'
                          : 'bg-[#fafafa] dark:bg-[#161b22] text-[#71717a] dark:text-[#8b949e] border border-[#ebebeb] dark:border-[#30363d] cursor-default select-none'
                      }`}
                    >
                      {isPassed && <Check className="h-3 w-3 text-[#15803d] dark:text-[#7ee787] shrink-0" />}
                      {isRunningStage && (
                        <span className="h-2 w-2 rounded-full bg-[#22c55e] shrink-0" />
                      )}
                      <span className="truncate max-w-[180px] sm:max-w-[240px]">{col.title}</span>
                    </button>
                  </div>
                );
              })}
            </div>
          ) : null}
        </div>

        {/* Right: tool policy, settings, theme toggle, window controls */}
        <div className="flex items-center h-full space-x-2 shrink-0 select-none pointer-events-auto pr-0" data-tauri-drag-region>
          <div className="flex items-center space-x-2 select-none pointer-events-auto" data-no-drag>
            <button
              data-no-drag
              type="button"
              onMouseDown={(e) => e.stopPropagation()}
              onClick={onOpenToolPolicy}
              className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#161b22] dark:hover:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#666666] hover:text-[#171717] dark:text-[#8b949e] dark:hover:text-[#f0f6fc] transition cursor-pointer select-none pointer-events-auto"
              aria-label={t('topbar.toolPolicyTitle')}
            >
              <Wrench className="h-3.5 w-3.5" />
            </button>
            <button
              data-no-drag
              type="button"
              onMouseDown={(e) => e.stopPropagation()}
              onClick={onOpenSettings}
              className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#161b22] dark:hover:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#666666] hover:text-[#171717] dark:text-[#8b949e] dark:hover:text-[#f0f6fc] transition cursor-pointer select-none pointer-events-auto"
              aria-label={t('topbar.llmSettings')}
            >
              <Settings className="h-3.5 w-3.5" />
            </button>

            {/* Language Switcher Button (EN / RU) */}
            <button
              data-no-drag
              type="button"
              onMouseDown={(e) => e.stopPropagation()}
              onClick={toggleLanguage}
              className="px-2 py-1 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#161b22] dark:hover:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#666666] hover:text-[#171717] dark:text-[#8b949e] dark:hover:text-[#f0f6fc] transition cursor-pointer font-mono font-semibold text-[11px] select-none flex items-center space-x-1"
              aria-label={t('topbar.switchLanguage')}
            >
              <span className={language === 'en' ? 'text-[#171717] dark:text-[#f0f6fc] font-bold' : 'text-[#8f8f8f] dark:text-[#6e7681]'}>
                EN
              </span>
              <span className="text-[#d4d4d8] dark:text-[#30363d]">/</span>
              <span className={language === 'ru' ? 'text-[#171717] dark:text-[#f0f6fc] font-bold' : 'text-[#8f8f8f] dark:text-[#6e7681]'}>
                RU
              </span>
            </button>

            {/* Theme Toggle Button with Circular Wave Animation */}
            <button
              data-no-drag
              type="button"
              onMouseDown={(e) => e.stopPropagation()}
              onClick={(e) => toggleThemeWithAnimation(e)}
              className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#161b22] dark:hover:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#666666] hover:text-[#171717] dark:text-[#8b949e] dark:hover:text-[#f0f6fc] transition cursor-pointer relative overflow-hidden group select-none pointer-events-auto"
              aria-label={t('topbar.switchTheme')}
            >
              <div className="relative h-3.5 w-3.5 flex items-center justify-center">
                <Sun
                  className={`h-3.5 w-3.5 transition-all duration-300 transform absolute inset-0 ${
                    theme === 'dark'
                      ? 'rotate-90 scale-0 opacity-0'
                      : 'rotate-0 scale-100 opacity-100 text-amber-500'
                  }`}
                />
                <Moon
                  className={`h-3.5 w-3.5 transition-all duration-300 transform absolute inset-0 ${
                    theme === 'dark'
                      ? 'rotate-0 scale-100 opacity-100 text-indigo-400'
                      : '-rotate-90 scale-0 opacity-0'
                  }`}
                />
              </div>
            </button>
          </div>
          <div className="h-4 w-px bg-[#ebebeb] dark:bg-[#21262d] mx-1" />
          <div className="select-none pointer-events-auto" data-no-drag>
            <WindowControls />
          </div>
        </div>
      </div>
    </header>
  );
}

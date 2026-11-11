import React from 'react';
import { useShallow } from 'zustand/react/shallow';
import type { KanbanTask, PipelineStage, PipelineColumn } from '../../types';
import Logo from '../common/Logo';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import { useCortexStore } from '../../stores/cortexStore';
import AgentIcon from '../common/AgentIcon';
import { getRoleBadgeStyle } from '../../lib/agentPalette';
import { formatSubagentDisplayTitle, formatSubagentRoleDisplay, getSubagentOrderMap } from './SubagentsSidebar';
import { toggleMaximizeWindow } from '../layout/WindowControls';
import { formatTimerMMSS } from '../card/CardTimerBadge';
import { resolveStageTask, resolveStageDuration, resolveStageStatus } from './stageStepperHelpers';
import { ArrowLeft, ChevronRight, Check, FolderGit2, GitBranch, Layers, Zap } from 'lucide-react';
import { useThemeStore } from '../../stores/themeStore';
import DrawerHeaderControls from './DrawerHeaderControls';
import { useTranslation, type TranslationKey } from '../../stores/languageStore';

export function getStageButtonClass(
  isCurrent: boolean,
  isPassed: boolean,
  targetTaskId: string | null
): string {
  if (isCurrent) {
    return 'bg-[#f4f4f5] dark:bg-[#232328] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] font-semibold shadow-2xs cursor-default';
  }
  if (targetTaskId && isPassed) {
    return 'bg-[#f0fdf4] dark:bg-[#12281e] text-[#15803d] dark:text-[#7ee787] border border-[#bbf7d0] dark:border-[#238636] hover:bg-[#dcfce7] dark:hover:bg-[#173a2b] cursor-pointer active:scale-95';
  }
  if (targetTaskId) {
    return 'bg-white dark:bg-[#17171b] text-[#171717] dark:text-[#a1a1aa] border border-[#ebebeb] dark:border-[#2c2c33] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:border-[#d4d4d8] dark:hover:border-[#3a3a44] cursor-pointer active:scale-95';
  }
  return 'bg-[#fafafa] dark:bg-[#17171b] text-[#71717a] dark:text-[#71717a] border border-[#ebebeb] dark:border-[#2c2c33] cursor-default select-none';
}

export function getStageAriaLabel(
  isCurrent: boolean,
  targetTaskId: string | null,
  title: string,
  t: (key: TranslationKey, params?: Record<string, string | number>) => string
): string {
  if (isCurrent) {
    return t('drawer.stageCurrent', { title });
  }
  if (targetTaskId) {
    return t('drawer.stageSwitch', { title });
  }
  return t('drawer.stageNotStarted');
}

export interface DrawerHeaderProps {
  task: KanbanTask;
  stages?: PipelineStage[];
  /** @deprecated Use stages instead */
  columns?: PipelineColumn[];
  subtasks: KanbanTask[];
  rootTask: KanbanTask | null;
  activeTaskId: string | null;
  isRunning?: boolean;
  isDiffCollapsed?: boolean;
  isSubagentsCollapsed?: boolean;
  subagentsCount?: number;
  onClose?: () => void;
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
  stages,
  columns: columnsProp,
  subtasks,
  rootTask,
  activeTaskId,
  isRunning = false,
  onClose,
  onOpenDrawer,
  onOpenToolPolicy,
  onOpenSettings,
}: DrawerHeaderProps) {
  const columns = stages ?? columnsProp ?? [];
  const handleHeaderDoubleClick = (e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    if (target.closest('button, input, select, textarea, [data-no-drag]')) {
      return;
    }
    void toggleMaximizeWindow();
  };

  const {
    activeSubagentId,
    activeSubagentTitle,
    activeSubagentRole,
    goBackDrawer,
    storeSubagents,
    toolExecutions,
    toolIdToSubagentId,
  } = useDrawerStore(
    useShallow((s) => ({
      activeSubagentId: s.activeSubagentId,
      activeSubagentTitle: s.activeSubagentTitle,
      activeSubagentRole: s.activeSubagentRole,
      goBackDrawer: s.goBackDrawer,
      storeSubagents: s.subagents,
      toolExecutions: s.toolExecutions,
      toolIdToSubagentId: s.toolIdToSubagentId,
    }))
  );
  const boardTasks = useBoardStore((s) => s.tasks);
  const agentPersonas = useCortexStore((s) => s.agentPersonas);
  const matchedPersona = agentPersonas.find(
    (p) => p.role.toLowerCase() === (activeSubagentRole || '').toLowerCase()
  );
  const { theme, toggleThemeWithAnimation } = useThemeStore(
    useShallow((s) => ({ theme: s.theme, toggleThemeWithAnimation: s.toggleThemeWithAnimation }))
  );
  const { language, toggleLanguage, t } = useTranslation();

  const subagentNumber = React.useMemo(() => {
    if (!activeSubagentId || !task?.id) return null;
    const map = getSubagentOrderMap(task.id, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks);
    return map.get(activeSubagentId) ?? (toolIdToSubagentId[activeSubagentId] ? map.get(toolIdToSubagentId[activeSubagentId]) : null) ?? 1;
  }, [activeSubagentId, task?.id, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks]);

  const isBoardSubtask = React.useMemo(() => {
    if (!activeSubagentId) return false;
    return boardTasks.some((bt) => bt.id === activeSubagentId);
  }, [activeSubagentId, boardTasks]);

  const activeSubagentIsBg = React.useMemo(() => {
    if (!activeSubagentId || !task?.id) return false;
    const subs = storeSubagents[task.id] || [];
    const directSub = subs.find((s) => s.id === activeSubagentId || s.toolCallId === activeSubagentId);
    if (directSub?.isBg !== undefined) return directSub.isBg;
    const tools = toolExecutions[task.id] || [];
    const matchedTool = tools.find((x) => {
      const p = x.params && typeof x.params === 'object' ? (x.params as Record<string, unknown>) : null;
      return x.id === activeSubagentId || p?.subagent_id === activeSubagentId;
    });
    if (matchedTool) {
      const p = matchedTool.params && typeof matchedTool.params === 'object'
        ? (matchedTool.params as Record<string, unknown>)
        : null;
      return matchedTool.toolName === 'subagent_bg' || p?.is_bg === true || p?.isBg === true;
    }
    return false;
  }, [activeSubagentId, task?.id, storeSubagents, toolExecutions]);

  const workingColumns = columns.filter(
    (c) => !c.isIntake && c.id !== 'backlog' && c.id !== 'ready' && !c.terminal
  );
  const currentStageIdx = columns.findIndex((c) => c?.id === task?.stageId);

  const [, setTick] = React.useState(0);
  React.useEffect(() => {
    if (!isRunning) return;
    const interval = setInterval(() => setTick((prev) => prev + 1), 1000);
    return () => clearInterval(interval);
  }, [isRunning]);

  return (
    <header
      data-tauri-drag-region
      onDoubleClick={handleHeaderDoubleClick}
      className="h-12 border-b border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114] pl-4 pr-0 shrink-0 z-10 shadow-xs flex items-center select-none transition-colors"
    >
      <div className="flex items-center justify-between w-full h-full" data-tauri-drag-region>
        {/* Left: Logo + Back + Breadcrumbs/Navigation + Worktree Badges */}
        <div className="flex items-center space-x-2 shrink-0 select-none pointer-events-auto" data-no-drag>
          <Logo size={18} className="shrink-0 select-none" />
          <button
            data-no-drag
            type="button"
            onMouseDown={(e) => e.stopPropagation()}
            onClick={onClose || goBackDrawer}
            className="flex items-center gap-1.5 px-3 py-1 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#17171b] dark:hover:bg-[#232328] text-[#171717] dark:text-[#f4f4f5] border border-[#ebebeb] dark:border-[#2c2c33] transition font-sans text-xs font-medium cursor-pointer shadow-xs select-none pointer-events-auto btn-tactile leading-none"
            aria-label={activeSubagentId ? t('drawer.backToTask') : t('drawer.backToBoard')}
          >
            <ArrowLeft className="h-3.5 w-3.5 block shrink-0 text-[#171717] dark:text-[#f4f4f5]" />
            <span className="leading-none">{activeSubagentId ? t('drawer.backToTask') : t('drawer.backToBoard')}</span>
          </button>

          {/* Breadcrumbs / Task badge (only when NOT in subagent mode) */}
          {!activeSubagentId && task?.id && (rootTask && rootTask.id !== task.id ? (
            <div className="flex items-center space-x-1 shrink-0 select-none pointer-events-auto" data-no-drag>
              <button
                type="button"
                onClick={() => onOpenDrawer(rootTask.id)}
                className="font-mono text-xs text-[#71717a] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] px-1.5 py-0.5 rounded hover:bg-[#f4f4f5] dark:hover:bg-[#17171b] transition cursor-pointer"
                aria-label={t('drawer.backToParentTitle', { id: rootTask.id })}
              >
                {rootTask.id}
              </button>
              <ChevronRight className="h-3 w-3 text-[#a1a1aa] dark:text-[#52525b] shrink-0" />
              <span className="font-mono text-xs font-semibold text-[#171717] dark:text-[#f4f4f5] px-2 py-0.5 rounded bg-[#f4f4f5] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] shrink-0">
                {task.id}
              </span>
            </div>
          ) : (
            <span className="font-mono text-xs font-semibold text-[#171717] dark:text-[#f4f4f5] px-2 py-0.5 rounded bg-[#f4f4f5] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] shrink-0 select-none pointer-events-auto">
              {task.id}
            </span>
          ))}

          {/* Directly adjacent to the title/breadcrumbs, render the compact worktree/branch badges */}
          {!activeSubagentId && !task?.skipWorktree && (task?.worktreePath || rootTask?.worktreePath) && (
            <div className="flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-[#f4f4f5] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] text-[11px] font-mono shrink-0 leading-none">
              <FolderGit2 className="h-3 w-3 block shrink-0 text-[#8f8f8f] dark:text-[#71717a]" />
              <span className="text-[#4d4d4d] dark:text-[#a1a1aa] max-w-[140px] truncate leading-none">{task?.worktreePath || rootTask?.worktreePath}</span>
            </div>
          )}
          {!activeSubagentId && !task?.skipWorktree && (task?.branch || rootTask?.branch) && (
            <div className="flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-[#f4f4f5] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] text-[11px] font-mono shrink-0 leading-none">
              <GitBranch className="h-3 w-3 block shrink-0 text-[#8f8f8f] dark:text-[#71717a]" />
              <span className="text-[#4d4d4d] dark:text-[#a1a1aa] max-w-[140px] truncate leading-none">{task?.branch || rootTask?.branch}</span>
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
              className="flex items-center space-x-2 px-3 py-1 rounded-md bg-[#f4f4f5] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] text-xs font-mono max-w-xl mx-auto shadow-2xs select-none pointer-events-auto min-w-0"
            >
              <span className="flex items-center space-x-1.5 font-semibold text-[#171717] dark:text-[#f4f4f5] truncate min-w-0">
                {activeSubagentIsBg ? (
                  <div className="relative shrink-0 flex items-center justify-center">
                    <Layers className="h-3.5 w-3.5 text-cyan-600 dark:text-cyan-400" />
                    <span className="absolute -bottom-0.5 -right-0.5 h-1.5 w-1.5 rounded-full bg-cyan-500 ring-1 ring-white dark:ring-[#17171b]" />
                  </div>
                ) : isBoardSubtask ? (
                  <AgentIcon name={matchedPersona?.icon} className="h-3.5 w-3.5 text-[#171717] dark:text-[#f4f4f5] shrink-0" />
                ) : (
                  <div className="relative shrink-0 flex items-center justify-center">
                    <Zap className="h-3.5 w-3.5 text-sky-600 dark:text-sky-400 fill-sky-500/20" />
                    <span className="absolute -bottom-0.5 -right-0.5 h-1.5 w-1.5 rounded-full bg-sky-500 ring-1 ring-white dark:ring-[#17171b]" />
                  </div>
                )}
                <span className="truncate" title={activeSubagentTitle || undefined}>
                  {formatSubagentDisplayTitle(activeSubagentTitle, language)}
                </span>
              </span>
              <span
                style={getRoleBadgeStyle(activeSubagentRole, matchedPersona?.color)}
                className="px-2 py-0.5 rounded text-[11px] font-mono font-medium border shrink-0 select-none"
              >
                {formatSubagentRoleDisplay(activeSubagentRole, subagentNumber || 1, isBoardSubtask, language)}
              </span>
            </div>
          ) : task && workingColumns.length > 0 ? (
            <div
              data-no-drag
              className="flex items-center gap-0.5 px-1.5 py-0.5 rounded-lg bg-[#f9fafb] dark:bg-[#17171b] shrink-0 select-none pointer-events-auto shadow-2xs overflow-x-auto no-scrollbar max-w-2xl"
            >
              {workingColumns.map((col, idx) => {
                const targetTask = resolveStageTask(col.id, workingColumns, task, rootTask, subtasks);
                const targetTaskId = targetTask?.id ?? null;
                const { isCurrent, isColRunning, isPassed } = resolveStageStatus(
                  col.id,
                  idx,
                  currentStageIdx,
                  targetTask,
                  task,
                  rootTask,
                  subtasks,
                  activeTaskId,
                  isRunning,
                  workingColumns
                );
                const stageSec = resolveStageDuration(col.id, task, rootTask, subtasks, isRunning);

                return (
                  <div key={col.id} className="flex items-center shrink-0">
                    {idx > 0 && (
                      <ChevronRight className="h-3 w-3 text-[#a1a1aa] dark:text-[#52525b] mx-0.5 shrink-0" />
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
                      aria-label={getStageAriaLabel(isCurrent, targetTaskId, col.title, t)}
                      className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-xs font-mono transition-all duration-150 tabular-nums ${getStageButtonClass(
                        isCurrent,
                        isPassed,
                        targetTaskId
                      )}`}
                    >
                      {isPassed && !isColRunning && (
                        <Check className="h-3 w-3 text-[#15803d] dark:text-[#7ee787] shrink-0" />
                      )}
                      {isColRunning && (
                        <span className="h-2 w-2 rounded-full bg-[#22c55e] animate-pulse shrink-0" />
                      )}
                      <span className="truncate max-w-[180px] sm:max-w-[240px]">
                        {stageSec > 0 ? `${col.title} ${formatTimerMMSS(stageSec)}` : col.title}
                      </span>
                    </button>
                  </div>
                );
              })}
            </div>
          ) : null}
        </div>

        {/* Right: tool policy, settings, theme toggle, window controls */}
        <DrawerHeaderControls
          onOpenToolPolicy={onOpenToolPolicy}
          onOpenSettings={onOpenSettings}
          toggleLanguage={toggleLanguage}
          language={language}
          theme={theme}
          toggleThemeWithAnimation={toggleThemeWithAnimation}
          t={t}
        />
      </div>
    </header>
  );
}

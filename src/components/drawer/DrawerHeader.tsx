import React from 'react';
import type { KanbanTask, PipelineColumn } from '../../types';
import Logo from '../common/Logo';
import { useDrawerStore } from '../../stores/drawerStore';
import { getRoleBadgeClass } from './ToolExecutionCard';
import { getRoleIcon, formatSubagentDisplayTitle } from './SubagentsSidebar';
import WindowControls, { toggleMaximizeWindow } from '../layout/WindowControls';
import {
  ArrowLeft,
  ChevronRight,
  Check,
  Wrench,
  Settings,
  FolderGit2,
  GitBranch,
  Square,
  GitCompare,
  Sun,
  Moon,
} from 'lucide-react';
import { useThemeStore } from '../../stores/themeStore';

export interface DrawerHeaderProps {
  task: KanbanTask;
  columns: PipelineColumn[];
  subtasks: KanbanTask[];
  rootTask: KanbanTask | null;
  activeTaskId: string | null;
  isRunning: boolean;
  isDiffCollapsed: boolean;
  isSubagentsCollapsed?: boolean;
  subagentsCount?: number;
  onClose: () => void;
  onAbort: () => void;
  onToggleDiff: () => void;
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
  isRunning,
  isDiffCollapsed,
  isSubagentsCollapsed: _isSubagentsCollapsed = false,
  subagentsCount: _subagentsCount = 0,
  onClose,
  onAbort,
  onToggleDiff,
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

  const {
    activeSubagentId,
    activeSubagentTitle,
    activeSubagentRole,
    closeSubagent,
  } = useDrawerStore();
  const { theme, toggleThemeWithAnimation } = useThemeStore();

  const workingColumns = columns.filter(
    (c) => c.id !== 'backlog' && c.id !== 'ready' && !c.terminal
  );
  const currentStageIdx = columns.findIndex((c) => c?.id === task?.columnId);

  return (
    <>
      {/* Header mirrors the main topbar: h-10, drag region, window controls */}
      <header
        data-tauri-drag-region
        onDoubleClick={handleHeaderDoubleClick}
        className="h-10 border-b border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] pl-4 pr-0 shrink-0 z-10 shadow-xs flex items-center select-none transition-colors"
      >
        <div className="flex items-center justify-between w-full h-full" data-tauri-drag-region>
          {/* Left: Logo + Back */}
          <div className="flex items-center space-x-2 shrink-0 select-none pointer-events-auto" data-no-drag>
            <Logo size={18} className="shrink-0 select-none" />
            <button
              data-no-drag
              type="button"
              onMouseDown={(e) => e.stopPropagation()}
              onClick={activeSubagentId ? closeSubagent : onClose}
              className="flex items-center space-x-1.5 px-3 py-1 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#161b22] dark:hover:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc] border border-[#ebebeb] dark:border-[#30363d] transition font-sans text-xs font-medium cursor-pointer shadow-xs select-none pointer-events-auto"
              aria-label={activeSubagentId ? "Назад к родительской задаче" : "Назад к доске"}
            >
              <ArrowLeft className="h-4 w-4 text-[#171717] dark:text-[#f0f6fc]" />
              <span>{activeSubagentId ? "К задаче" : "Назад"}</span>
            </button>
          </div>

          {/* Center: Stage Stepper & Task badges / Subagent breadcrumbs */}
          <div
            className="flex-1 flex items-center justify-center space-x-2 px-4 overflow-hidden select-none"
            data-tauri-drag-region
          >
            {activeSubagentId ? (
              /* Subagent Breadcrumb: <Parent Task Title> / <Subagent Task Title> */
              <div
                data-no-drag
                className="flex items-center space-x-2 px-3 py-1 rounded-md bg-[#f4f4f5] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-xs font-mono min-w-0 max-w-[650px] overflow-hidden select-none pointer-events-auto shadow-2xs"
              >
                <button
                  type="button"
                  onClick={closeSubagent}
                  className="flex items-center space-x-1.5 text-[#52525b] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:underline cursor-pointer truncate max-w-[240px] font-medium transition"
                  title={`Вернуться к задаче: ${task?.title || task?.id || 'Родительская задача'}`}
                >
                  <ArrowLeft className="h-3.5 w-3.5 shrink-0 text-[#171717] dark:text-[#f0f6fc]" />
                  <span className="truncate">{task?.title || task?.id || 'Задача'}</span>
                </button>
                <span className="text-[#a1a1aa] dark:text-[#6e7681] shrink-0 font-sans">/</span>
                {(() => {
                  const SubagentRoleIcon = getRoleIcon(activeSubagentRole);
                  return (
                    <span className="flex items-center space-x-1.5 font-semibold text-[#171717] dark:text-[#f0f6fc] truncate max-w-[320px]">
                      <SubagentRoleIcon className="h-3.5 w-3.5 text-[#171717] dark:text-[#f0f6fc] shrink-0" />
                      <span className="truncate">{formatSubagentDisplayTitle(activeSubagentTitle)}</span>
                    </span>
                  );
                })()}
                <span className={`px-1.5 py-0.5 rounded text-[9px] font-mono font-bold uppercase tracking-wider border shrink-0 ${getRoleBadgeClass(activeSubagentRole)}`}>
                  {(activeSubagentRole || 'general').toUpperCase()}
                </span>
              </div>
            ) : (
              <>
                {/* Task badge */}
                {task?.id && (
                  rootTask && rootTask.id !== task.id ? (
                    <div className="flex items-center space-x-1 shrink-0 select-none pointer-events-auto" data-no-drag>
                      <button
                        type="button"
                        onClick={() => onOpenDrawer(rootTask.id)}
                        className="font-mono text-xs text-[#71717a] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] px-1.5 py-0.5 rounded hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] transition cursor-pointer"
                        title={`К родительской задаче ${rootTask.id}`}
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

                {/* Stage Stepper for Console Header */}
                {task && workingColumns.length > 0 && (
                  <div
                    data-no-drag
                    className="flex items-center gap-0.5 px-1.5 py-0.5 rounded-lg bg-[#f9fafb] dark:bg-[#161b22] shrink-0 select-none pointer-events-auto shadow-2xs overflow-x-auto no-scrollbar max-w-[500px]"
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
                        title={
                          isCurrent
                            ? `Текущий этап: ${col.title}`
                            : targetTaskId
                            ? `Перейти к этапу: ${col.title}`
                            : 'Этап еще не начат'
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
                        <span className="truncate max-w-[85px]">{col.title}</span>
                      </button>
                    </div>
                  );
                })}
              </div>
            )}
            </>
          )}
          </div>

          {/* Right: triggers + window controls */}
          <div className="flex items-center h-full space-x-1 shrink-0 select-none pointer-events-auto" data-tauri-drag-region>
            <div className="flex items-center space-x-1.5 pr-2 select-none pointer-events-auto" data-no-drag>
              <button
                data-no-drag
                type="button"
                onMouseDown={(e) => e.stopPropagation()}
                onClick={onOpenToolPolicy}
                className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#161b22] dark:hover:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#666666] hover:text-[#171717] dark:text-[#8b949e] dark:hover:text-[#f0f6fc] transition cursor-pointer select-none pointer-events-auto"
                title="Матрица инструментов — доступ тулов по агентам"
              >
                <Wrench className="h-3.5 w-3.5" />
              </button>
              <button
                data-no-drag
                type="button"
                onMouseDown={(e) => e.stopPropagation()}
                onClick={onOpenSettings}
                className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#161b22] dark:hover:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#666666] hover:text-[#171717] dark:text-[#8b949e] dark:hover:text-[#f0f6fc] transition cursor-pointer select-none pointer-events-auto"
                title="Настройки LLM"
              >
                <Settings className="h-3.5 w-3.5" />
              </button>

              {/* Theme Toggle Button with Circular Wave Animation */}
              <button
                data-no-drag
                type="button"
                onMouseDown={(e) => e.stopPropagation()}
                onClick={(e) => toggleThemeWithAnimation(e)}
                className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#161b22] dark:hover:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#666666] hover:text-[#171717] dark:text-[#8b949e] dark:hover:text-[#f0f6fc] transition cursor-pointer relative overflow-hidden group select-none pointer-events-auto"
                title={theme === 'dark' ? 'Включить светлую тему' : 'Включить тёмную тему'}
                aria-label="Сменить тему оформления"
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

      {/* Worktree strip below the header */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] text-xs font-mono text-[#666666] dark:text-[#8b949e] shrink-0 transition-colors">
        <div className="flex items-center space-x-2.5">
          {task?.worktreePath && (
            <div className="flex items-center space-x-1.5 px-2 py-0.5 rounded-md bg-[#f9f9f9] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d]">
              <FolderGit2 className="h-3.5 w-3.5 text-[#8f8f8f] dark:text-[#8b949e]" />
              <span className="text-[#4d4d4d] dark:text-[#c9d1d9]">{task.worktreePath}</span>
            </div>
          )}
          {task?.branch && (
            <div className="flex items-center space-x-1.5 px-2 py-0.5 rounded-md bg-[#f9f9f9] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d]">
              <GitBranch className="h-3.5 w-3.5 text-[#8f8f8f] dark:text-[#8b949e]" />
              <span className="text-[#4d4d4d] dark:text-[#c9d1d9]">{task.branch}</span>
            </div>
          )}
        </div>

        {/* Right Controls: Status & Git Diff */}
        <div className="flex items-center space-x-3">
          {isRunning ? (
            <div className="flex items-center space-x-2">
              <span className="flex items-center space-x-1.5 px-2.5 py-1 rounded-md text-xs font-mono text-[#15803d] dark:text-[#7ee787] bg-[#f0fdf4] dark:bg-[#12281e] border border-[#bbf7d0] dark:border-[#238636]">
                <span className="h-2 w-2 rounded-full bg-[#15803d] dark:bg-[#22c55e] shrink-0" />
                <span>Агент выполняет задачу...</span>
              </span>
              <button
                type="button"
                onClick={onAbort}
                className="flex items-center space-x-1 px-2.5 py-1 rounded-md text-xs font-mono text-[#b91c1c] dark:text-[#ff7b72] bg-[#fef2f2] dark:bg-[#3b1219] hover:bg-[#fee2e2] dark:hover:bg-[#4d1922] border border-[#fecaca] dark:border-[#da3633] transition cursor-pointer"
              >
                <Square className="h-3 w-3 fill-current" />
                <span>Остановить</span>
              </button>
            </div>
          ) : (
            <div className="flex items-center space-x-2">
              <span className="flex items-center space-x-1.5 px-2.5 py-1 rounded-md text-xs font-mono text-[#666666] dark:text-[#8b949e] bg-[#f4f4f5] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d]">
                <span>Ожидание указаний</span>
              </span>
            </div>
          )}

          {/* Git Diff Toggle Button */}
          <button
            type="button"
            onClick={onToggleDiff}
            className={`flex items-center space-x-1.5 px-2.5 py-1 rounded-md text-xs font-mono border transition cursor-pointer ${
              !isDiffCollapsed
                ? 'bg-[#171717] dark:bg-[#21262d] text-white border-[#171717] dark:border-[#30363d] shadow-xs'
                : 'bg-[#f4f4f5] dark:bg-[#161b22] text-[#171717] dark:text-[#f0f6fc] border-[#ebebeb] dark:border-[#30363d] hover:bg-[#e4e4e7] dark:hover:bg-[#21262d]'
            }`}
            title={isDiffCollapsed ? 'Развернуть Git Diff панель' : 'Скрыть Git Diff панель'}
          >
            <GitCompare className="h-3.5 w-3.5" />
            <span>Git Diff</span>
          </button>
        </div>
      </div>
    </>
  );
}

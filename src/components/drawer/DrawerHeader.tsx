import React from 'react';
import type { KanbanTask, PipelineColumn } from '../../types';
import Logo from '../common/Logo';
import { useDrawerStore } from '../../stores/drawerStore';
import { getRoleBadgeClass } from './ToolExecutionCard';
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
} from 'lucide-react';

export interface DrawerHeaderProps {
  task: KanbanTask;
  columns: PipelineColumn[];
  subtasks: KanbanTask[];
  rootTask: KanbanTask | null;
  activeTaskId: string | null;
  isRunning: boolean;
  isDiffCollapsed: boolean;
  onClose: () => void;
  onAbort: () => void;
  onToggleDiff: () => void;
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
  onClose,
  onAbort,
  onToggleDiff,
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
        className="h-10 border-b border-[#ebebeb] bg-white pl-4 pr-0 shrink-0 z-10 shadow-xs flex items-center select-none"
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
              className="flex items-center space-x-1.5 px-3 py-1 rounded-md bg-white hover:bg-[#f9f9f9] text-[#171717] border border-[#ebebeb] transition font-sans text-xs font-medium cursor-pointer shadow-xs active:scale-[0.98] select-none pointer-events-auto"
              title={activeSubagentId ? "Назад к родительской задаче" : "Назад к доске (Esc)"}
            >
              <ArrowLeft className="h-4 w-4 text-[#171717]" />
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
                className="flex items-center space-x-2 px-3 py-1 rounded-md bg-[#f4f4f5] border border-[#ebebeb] text-xs font-mono min-w-0 max-w-[650px] overflow-hidden select-none pointer-events-auto shadow-2xs"
              >
                <button
                  type="button"
                  onClick={closeSubagent}
                  className="flex items-center space-x-1.5 text-[#52525b] hover:text-[#171717] hover:underline cursor-pointer truncate max-w-[240px] font-medium transition"
                  title={`Вернуться к задаче: ${task?.title || task?.id || 'Родительская задача'}`}
                >
                  <ArrowLeft className="h-3.5 w-3.5 shrink-0 text-[#171717]" />
                  <span className="truncate">{task?.title || task?.id || 'Задача'}</span>
                </button>
                <span className="text-[#a1a1aa] shrink-0 font-sans">/</span>
                <span className="flex items-center space-x-1.5 font-semibold text-[#171717] truncate max-w-[320px]">
                  <span className="text-[#171717] shrink-0 text-xs">■</span>
                  <span className="truncate">{activeSubagentTitle || 'Субагент'}</span>
                </span>
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
                        className="font-mono text-xs text-[#71717a] hover:text-[#171717] px-1.5 py-0.5 rounded hover:bg-[#f4f4f5] transition cursor-pointer"
                        title={`К родительской задаче ${rootTask.id}`}
                      >
                        {rootTask.id}
                      </button>
                      <ChevronRight className="h-3 w-3 text-[#a1a1aa] shrink-0" />
                      <span className="font-mono text-xs font-semibold text-[#171717] px-2 py-0.5 rounded bg-[#f4f4f5] border border-[#ebebeb] shrink-0">
                        {task.id}
                      </span>
                    </div>
                  ) : (
                    <span className="font-mono text-xs font-semibold text-[#171717] px-2 py-0.5 rounded bg-[#f4f4f5] border border-[#ebebeb] shrink-0 select-none pointer-events-auto">
                      {task.id}
                    </span>
                  )
                )}

                {/* Stage Stepper for Console Header */}
                {task && workingColumns.length > 0 && (
                  <div
                    data-no-drag
                    className="flex items-center gap-0.5 px-1.5 py-0.5 rounded-lg bg-[#f9fafb] border border-[#ebebeb] shrink-0 select-none pointer-events-auto shadow-2xs overflow-x-auto no-scrollbar max-w-[500px]"
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
                        <ChevronRight className="h-3 w-3 text-[#d4d4d8] mx-0.5 shrink-0" />
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
                            ? 'bg-[#171717] text-white font-semibold shadow-xs ring-1 ring-[#171717] cursor-default'
                            : targetTaskId
                            ? isPassed
                              ? 'bg-[#f0fdf4] text-[#15803d] border border-[#bbf7d0] hover:bg-[#dcfce7] cursor-pointer active:scale-95'
                              : 'bg-white text-[#71717a] border border-[#ebebeb] hover:bg-[#f4f4f5] hover:text-[#171717] hover:border-[#d4d4d8] cursor-pointer active:scale-95'
                            : 'bg-[#fafafa] text-[#a1a1aa] border border-[#ebebeb]/60 cursor-default select-none opacity-60'
                        }`}
                      >
                        {isPassed && <Check className="h-3 w-3 text-[#15803d] shrink-0" />}
                        {isRunningStage && (
                          <span className="h-2 w-2 rounded-full bg-[#22c55e] animate-pulse shrink-0" />
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
                className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] border border-[#ebebeb] text-[#666666] hover:text-[#171717] transition cursor-pointer select-none pointer-events-auto"
                title="Матрица инструментов — доступ тулов по агентам"
              >
                <Wrench className="h-3.5 w-3.5" />
              </button>
              <button
                data-no-drag
                type="button"
                onMouseDown={(e) => e.stopPropagation()}
                onClick={onOpenSettings}
                className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] border border-[#ebebeb] text-[#666666] hover:text-[#171717] transition cursor-pointer select-none pointer-events-auto"
                title="Настройки LLM"
              >
                <Settings className="h-3.5 w-3.5" />
              </button>
            </div>
            <div className="h-4 w-px bg-[#ebebeb] mx-1" />
            <div className="select-none pointer-events-auto" data-no-drag>
              <WindowControls />
            </div>
          </div>
        </div>
      </header>

      {/* Worktree strip below the header */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-[#ebebeb] bg-white text-xs font-mono text-[#666666] shrink-0">
        <div className="flex items-center space-x-2.5">
          {task?.worktreePath && (
            <div className="flex items-center space-x-1.5 px-2 py-0.5 rounded-md bg-[#f9f9f9] border border-[#ebebeb]">
              <FolderGit2 className="h-3.5 w-3.5 text-[#8f8f8f]" />
              <span className="text-[#4d4d4d]">{task.worktreePath}</span>
            </div>
          )}
          {task?.branch && (
            <div className="flex items-center space-x-1.5 px-2 py-0.5 rounded-md bg-[#f9f9f9] border border-[#ebebeb]">
              <GitBranch className="h-3.5 w-3.5 text-[#8f8f8f]" />
              <span className="text-[#4d4d4d]">{task.branch}</span>
            </div>
          )}
        </div>

        {/* Right Controls: Status & Git Diff */}
        <div className="flex items-center space-x-3">
          {isRunning ? (
            <div className="flex items-center space-x-2">
              <span className="flex items-center space-x-1.5 px-2.5 py-1 rounded-md text-xs font-mono text-[#15803d] bg-[#f0fdf4] border border-[#bbf7d0]">
                <span className="h-2 w-2 rounded-full bg-[#15803d] animate-pulse" />
                <span>Агент выполняет задачу...</span>
              </span>
              <button
                type="button"
                onClick={onAbort}
                className="flex items-center space-x-1 px-2.5 py-1 rounded-md text-xs font-mono text-[#b91c1c] bg-[#fef2f2] hover:bg-[#fee2e2] border border-[#fecaca] transition cursor-pointer"
                title="Остановить выполнение (abort_agent)"
              >
                <Square className="h-3 w-3 fill-current" />
                <span>Остановить</span>
              </button>
            </div>
          ) : (
            <div className="flex items-center space-x-2">
              <span className="flex items-center space-x-1.5 px-2.5 py-1 rounded-md text-xs font-mono text-[#666666] bg-[#f4f4f5] border border-[#ebebeb]">
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
                ? 'bg-[#171717] text-white border-[#171717] shadow-xs'
                : 'bg-[#f4f4f5] text-[#171717] border-[#ebebeb] hover:bg-[#e4e4e7]'
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

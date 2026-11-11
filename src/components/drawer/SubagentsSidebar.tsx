import { useMemo, useState, useEffect } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import { useCortexStore } from '../../stores/cortexStore';
import {
  Bot,
  PanelLeftClose,
  PanelLeftOpen,
} from 'lucide-react';
import { useTranslation } from '../../stores/languageStore';
import { useSidebarResize } from './subagents/useSidebarResize';
import { BoardSubtaskCard } from './subagents/BoardSubtaskCard';
import { InProcessSubagentCard } from './subagents/InProcessSubagentCard';

import {
  getSubagentsForTask,
  getSubagentOrderMap,
} from './subagentHelpers';

export {
  formatSubagentDisplayTitle,
  getSubagentRoleLabel,
  formatSubagentRoleDisplay,
  formatDuration,
  formatSteps,
  getSubagentsForTask,
  getSubagentOrderMap,
} from './subagentHelpers';

interface SubagentsSidebarProps {
  taskId: string;
  taskTitle?: string;
  isCollapsed: boolean;
  onToggleCollapse: () => void;
  className?: string;
}

export default function SubagentsSidebar({
  taskId,
  isCollapsed,
  onToggleCollapse,
  className = '',
}: SubagentsSidebarProps) {
  const {
    subagents: storeSubagents,
    activeSubagentId,
    toolExecutions,
    activityFeed,
    toolIdToSubagentId,
  } = useDrawerStore(
    useShallow((s) => ({
      subagents: s.subagents,
      activeSubagentId: s.activeSubagentId,
      toolExecutions: s.toolExecutions,
      activityFeed: s.activityFeed,
      toolIdToSubagentId: s.toolIdToSubagentId,
    })),
  );

  const boardTasks = useBoardStore((s) => s.tasks);
  const columns = useBoardStore((s) => s.columns);
  const agentPersonas = useCortexStore((s) => s.agentPersonas);

  const { sidebarWidth, isResizing, handleMouseDown } = useSidebarResize();

  const orderMap = useMemo(() => {
    return getSubagentOrderMap(taskId, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks, columns);
  }, [taskId, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks, columns]);

  const allSubagents = useMemo(() => {
    const subs = getSubagentsForTask(taskId, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks, columns);
    return subs.map((sub) => {
      const feed = activityFeed[sub.id] || (sub.toolCallId ? activityFeed[sub.toolCallId] : []) || [];
      return {
        ...sub,
        stepsCount: Math.max(sub.stepsCount || 0, feed.length),
      };
    });
  }, [taskId, storeSubagents, toolExecutions, toolIdToSubagentId, activityFeed, boardTasks, columns]);

  const boardSubtasks = useMemo(() => {
    return allSubagents.filter((sub) => Boolean(sub.isBoardSubtask || boardTasks.some((t) => t.id === sub.id)));
  }, [allSubagents, boardTasks]);

  const inProcessSubagents = useMemo(() => {
    return allSubagents.filter((sub) => !sub.isBoardSubtask && !boardTasks.some((t) => t.id === sub.id));
  }, [allSubagents, boardTasks]);

  const hasRunningSubagents = allSubagents.some((s) => s.status === 'running');
  const [now, setNow] = useState(() => Date.now());
  const { language, t } = useTranslation();

  // Dynamic 1-second live ticker only when subagents are actively running
  useEffect(() => {
    if (!hasRunningSubagents) return;
    const interval = setInterval(() => {
      setNow(Date.now());
    }, 1000);
    return () => clearInterval(interval);
  }, [hasRunningSubagents]);

  const runningCount = allSubagents.filter((s) => s.status === 'running').length;

  return (
    <aside
      style={{ width: `${isCollapsed ? 44 : sidebarWidth}px` }}
      className={`relative shrink-0 my-1.5 ml-1.5 rounded-xl border border-[#ebebeb] dark:border-[#212126] shadow-2xs bg-white dark:bg-[#111114] flex flex-col min-h-0 h-[calc(100%-0.75rem)] overflow-hidden select-none transition-colors ${
        isResizing ? 'select-none' : 'transition-[width] duration-200 ease-out'
      } ${className || ''}`}
      aria-label={t('subagents.panelAria')}
    >
      {/* Collapsed Rail View */}
      <div
        className={`absolute inset-0 w-11 bg-white dark:bg-[#111114] flex flex-col items-center py-3 select-none transition-opacity duration-200 ${
          isCollapsed ? 'opacity-100 pointer-events-auto z-10' : 'opacity-0 pointer-events-none -z-10'
        }`}
      >
        <button
          type="button"
          onClick={onToggleCollapse}
          className="p-1.5 rounded-md text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#17171b] transition cursor-pointer"
          aria-label={t('subagents.expandPanel')}
        >
          <PanelLeftOpen className="h-4 w-4 block shrink-0" />
        </button>
        <div
          onClick={onToggleCollapse}
          className="mt-6 flex-1 flex items-center justify-center cursor-pointer group"
          aria-label={t('subagents.expandPanel')}
        >
          <span className="text-[11px] font-mono text-[#8f8f8f] dark:text-[#71717a] group-hover:text-[#171717] dark:group-hover:text-[#f4f4f5] transition tracking-wider [writing-mode:vertical-rl] rotate-180">
            {t('subagents.title')}
          </span>
        </div>
      </div>

      {/* Expanded Content View */}
      <div
        style={{ width: `${sidebarWidth}px` }}
        className={`h-full flex flex-col min-h-0 overflow-hidden transition-opacity duration-200 ${
          isCollapsed ? 'opacity-0 pointer-events-none' : 'opacity-100 pointer-events-auto'
        }`}
      >
        {!isCollapsed && (
          <div
            onMouseDown={handleMouseDown}
            className="group/handle absolute top-0 bottom-0 right-0 w-2 cursor-col-resize z-30 flex items-center justify-center select-none"
            aria-label={t('subagents.panelAria')}
          >
            <div
              className={`w-[2px] h-full transition-colors ${
                isResizing
                  ? 'bg-[#15803d] dark:bg-[#7ee787]'
                  : 'group-hover/handle:bg-[#15803d]/60 dark:group-hover/handle:bg-[#7ee787]/60'
              }`}
            />
          </div>
        )}

        <div className="h-10 px-3.5 border-b border-[#ebebeb] dark:border-[#212126] flex items-center justify-between shrink-0 bg-[#fafafa] dark:bg-[#17171b]">
          <div className="flex items-center space-x-2 min-w-0">
            <Bot className="h-4 w-4 block shrink-0 text-[#171717] dark:text-[#f4f4f5]" />
            <span className="font-sans font-semibold text-xs text-[#171717] dark:text-[#f4f4f5] tracking-tight">
              {t('subagents.title')}
            </span>
            <span className="inline-flex items-center justify-center font-mono text-[11px] tabular-nums px-1.5 py-0.5 rounded-full leading-none bg-[#e4e4e7] dark:bg-[#232328] text-[#52525b] dark:text-[#71717a] font-medium">
              {allSubagents.length}
            </span>
            {runningCount > 0 && (
              <span className="flex items-center space-x-1 px-1.5 py-0.5 rounded-md bg-[#f0fdf4] dark:bg-[#12281e] text-[#15803d] dark:text-[#7ee787] text-[10px] font-mono border border-[#bbf7d0] dark:border-[#238636]">
                <span className="h-1.5 w-1.5 rounded-full bg-emerald-500" />
                <span>{t('subagents.runningCount', { count: runningCount })}</span>
              </span>
            )}
          </div>

          <button
            type="button"
            onClick={onToggleCollapse}
            className="p-1 rounded-md text-[#71717a] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#e4e4e7] dark:hover:bg-[#232328] transition cursor-pointer"
            aria-label={t('subagents.collapsePanel')}
          >
            <PanelLeftClose className="h-3.5 w-3.5 block shrink-0" />
          </button>
        </div>

        <div className="flex-1 min-h-0 overflow-y-auto p-3 space-y-3">
          {allSubagents.length === 0 ? (
            <div className="flex-1" />
          ) : (
            <>
              {boardSubtasks.length > 0 && (
                <div className="space-y-2">
                  <div className="flex items-center justify-between px-1 pt-0.5 pb-0.5">
                    <span className="text-[11px] font-mono uppercase tracking-wider text-[#71717a] dark:text-[#71717a] font-semibold">
                      {t('subagents.boardSubtasks')}
                    </span>
                    <span className="inline-flex items-center justify-center text-[10px] font-mono tabular-nums px-1.5 py-0.5 rounded-full leading-none bg-[#e4e4e7] dark:bg-[#232328] text-[#52525b] dark:text-[#71717a] font-medium">
                      {boardSubtasks.length}
                    </span>
                  </div>
                  {boardSubtasks.map((sub) => (
                    <BoardSubtaskCard
                      key={sub.id}
                      sub={sub}
                      agentPersonas={agentPersonas}
                      language={language}
                    />
                  ))}
                </div>
              )}

              {inProcessSubagents.length > 0 && (
                <div className="space-y-2">
                  <div className="flex items-center justify-between px-1 pt-2 pb-0.5">
                    <span className="text-[11px] font-mono uppercase tracking-wider text-[#71717a] dark:text-[#71717a] font-semibold">
                      {t('subagents.inProcess')}
                    </span>
                    <span className="inline-flex items-center justify-center text-[10px] font-mono tabular-nums px-1.5 py-0.5 rounded-full leading-none bg-[#e4e4e7] dark:bg-[#232328] text-[#52525b] dark:text-[#71717a] font-medium">
                      {inProcessSubagents.length}
                    </span>
                  </div>
                  {inProcessSubagents.map((sub, idx) => (
                    <InProcessSubagentCard
                      key={sub.id}
                      sub={sub}
                      idx={idx}
                      activeSubagentId={activeSubagentId}
                      toolIdToSubagentId={toolIdToSubagentId}
                      agentPersonas={agentPersonas}
                      orderMap={orderMap}
                      now={now}
                      language={language}
                      activityFeed={activityFeed}
                    />
                  ))}
                </div>
              )}
            </>
          )}
        </div>
      </div>
    </aside>
  );
}

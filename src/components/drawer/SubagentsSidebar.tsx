import { useMemo, useState, useEffect, useCallback, useRef } from 'react';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import { useCortexStore } from '../../stores/cortexStore';
import type { SubagentInfo, ToolExecution, KanbanTask } from '../../types';
import {
  getRoleBadgeStyle,
  getRoleActiveCardStyle,
} from '../../lib/agentPalette';
import {
  Bot,
  Clock,
  GitBranch,
  Layers,
  PanelLeftClose,
  PanelLeftOpen,
  Square,
  Zap,
} from 'lucide-react';
import { useTranslation, Language } from '../../stores/languageStore';

interface SubagentsSidebarProps {
  taskId: string;
  taskTitle: string;
  isCollapsed: boolean;
  onToggleCollapse: () => void;
  className?: string;
}


export function formatSubagentDisplayTitle(title?: string | null, lang?: Language): string {
  const fallback = lang === 'ru' ? 'Субагент' : 'Subagent';
  if (!title) return fallback;
  if (title.includes('sub-call_')) {
    const subWord = lang === 'ru' ? 'субагент' : 'subagent';
    const cleaned = title.replace(/(?:[A-Za-z0-9_-]+\.)?sub-call_\d+/g, subWord).trim();
    return cleaned || fallback;
  }
  return title;
}

function getSubagentRoleLabel(role: string, isRu: boolean): string {
  const r = role.trim().toLowerCase();
  if (r === 'architect') return isRu ? 'Архитектор' : 'Architect';
  if (r === 'developer' || r === 'dev') return isRu ? 'Разработчик' : 'Developer';
  if (r === 'reviewer' || r === 'review') return isRu ? 'Ревьюер' : 'Reviewer';
  if (r === 'scout') return isRu ? 'Скаут' : 'Scout';
  if (r === 'explorer') return isRu ? 'Исследователь' : 'Explorer';
  if (r === 'subtask') return isRu ? 'Подзадача' : 'Subtask';
  if (r === 'subagent' || r === 'general' || !r) return isRu ? 'Субагент' : 'Subagent';
  if (r === 'cvh-dev') return 'CVH-DEV';
  return role.trim().charAt(0).toUpperCase() + role.trim().slice(1);
}

export function formatSubagentRoleDisplay(
  role?: string | null,
  subIndex?: number,
  isBoardSubtask?: boolean,
  lang?: Language
): string {
  const isRu = lang === 'ru';
  if (isBoardSubtask) {
    const r = (role || '').trim().toLowerCase();
    if (role && r && r !== 'general') {
      return getSubagentRoleLabel(role, isRu);
    }
    return isRu ? 'Подзадача' : 'Subtask';
  }

  const r = (role || '').trim().toLowerCase();
  const hasRole = Boolean(role && r && r !== 'general');
  const baseName = hasRole ? getSubagentRoleLabel(role!, isRu) : (isRu ? 'Субагент' : 'Subagent');

  if (subIndex != null) {
    return `${baseName} #${subIndex}`;
  }
  return baseName;
}

export function formatDuration(ms?: number, isRunning?: boolean): string {
  if (ms === undefined || ms === null || !Number.isFinite(ms) || ms < 0) {
    return isRunning ? '0s' : '—';
  }
  if (!isRunning && ms < 1000) return `${Math.round(ms)}ms`;
  const totalSecs = Math.floor(ms / 1000);
  if (totalSecs < 60) return `${totalSecs}s`;
  const mins = Math.floor(totalSecs / 60);
  const secs = totalSecs % 60;
  return `${mins}m ${secs}s`;
}

export function formatSteps(count: number, lang: Language = 'en'): string {
  if (lang === 'en') {
    return count === 1 ? '1 step' : `${count} steps`;
  }
  const mod10 = count % 10;
  const mod100 = count % 100;
  if (mod100 >= 11 && mod100 <= 19) return `${count} шагов`;
  if (mod10 === 1) return `${count} шаг`;
  if (mod10 >= 2 && mod10 <= 4) return `${count} шага`;
  return `${count} шагов`;
}

function getSubagentsForTask(
  taskId: string,
  storeSubagents: Record<string, SubagentInfo[]>,
  toolExecutions: Record<string, ToolExecution[]>,
  toolIdToSubagentId: Record<string, string>,
  boardTasks: KanbanTask[],
  columns?: { id: string; persona?: string | null }[]
): SubagentInfo[] {
  const cols = columns || useBoardStore.getState().columns || [];
  const map = new Map<string, SubagentInfo>();

  // 1. Explicit store subagents (from backend list_subagents + live events)
  const list = storeSubagents[taskId] || [];
  for (const sub of list) {
    map.set(sub.id, {
      ...sub,
      createdAt: sub.createdAt || (sub.status === 'running' ? Date.now() : undefined),
    });
  }

  // 2. Derive from parent task's tool executions if not yet tracked
  const tools = toolExecutions[taskId] || [];
  for (const t of tools) {
    if (t.toolName === 'subagent' || t.toolName === 'subagent_bg') {
      const params = (t.params as Record<string, unknown>) || {};
      const subId =
        (params?.subagent_id as string) ||
        toolIdToSubagentId[t.id] ||
        t.id;
      const existing = map.get(subId);
      const isBg =
        t.toolName === 'subagent_bg' ||
        (params?.is_bg === true) ||
        (params?.isBg === true) ||
        existing?.isBg === true;
      const parsedTime = t.timestamp ? new Date(t.timestamp).getTime() : undefined;
      const toolCreatedAt =
        existing?.createdAt ||
        (parsedTime && !isNaN(parsedTime) ? parsedTime : undefined) ||
        (t.status === 'running' ? Date.now() : undefined);

      if (!existing) {
        map.set(subId, {
          id: subId,
          parentTaskId: taskId,
          toolCallId: t.id,
          title: (params?.title as string) || (params?.prompt as string)?.slice(0, 50) || 'Делегированная задача',
          role: (params?.role as string) || 'general',
          prompt: (params?.prompt as string) || '',
          status: t.status === 'running' ? 'running' : t.status === 'failed' ? 'error' : 'finished',
          createdAt: toolCreatedAt,
          // CRITICAL: NEVER take durationMs from subagent_bg tool execution (which is 181ms spawn time)
          durationMs: isBg ? undefined : t.durationMs,
          stepsCount: 0,
          isBg,
        });
      } else {
        if (isBg) {
          existing.isBg = true;
        }
        if (!existing.createdAt && toolCreatedAt) {
          existing.createdAt = toolCreatedAt;
        }
        // Do NOT take durationMs from subagent_bg tool call!
        if (!isBg && !existing.durationMs && t.durationMs) {
          existing.durationMs = t.durationMs;
        }
      }
    }
  }

  // 3. Subtasks on the Kanban board
  const childTasks = (boardTasks || []).filter((t) => t.parentId === taskId || (t.id.startsWith(taskId + '.') && !t.parentId));
  for (const child of childTasks) {
    const childCreated = (child as any).createdAt ? Number((child as any).createdAt) : undefined;
    const isReviewSubtask = child.title.toLowerCase().includes('ревью') || child.title.toLowerCase().includes('review');
    const childStageId = (child as any).stageId || child.columnId;
    const col = (cols || []).find((c) => c.id === childStageId);
    const resolvedRole = isReviewSubtask ? 'reviewer' : (col?.persona || 'developer');
    map.set(child.id, {
      id: child.id,
      parentTaskId: taskId,
      title: child.title,
      role: resolvedRole,
      prompt: child.description || '',
      status: child.status === 'running' ? 'running' : child.status === 'done' ? 'finished' : 'finished',
      createdAt: childCreated,
      durationMs: child.totalElapsed ? child.totalElapsed * 1000 : undefined,
      stepsCount: (child.todos || []).length,
      isBoardSubtask: true,
    });
  }

  const rawList = Array.from(map.values());

  // Filter out duplicate ghost/phantom subagents:
  // If multiple subagent entries share the exact same prompt (or title),
  // and one has stepsCount === 0 && (!durationMs || durationMs === 0) while another has
  // valid steps/duration or is finished/running, discard the 0-step phantom entry!
  const result = rawList.filter((sub, idx, all) => {
    // A running subagent is actively executing and must NEVER be treated as a phantom!
    const isPhantomCandidate =
      sub.status !== 'running' &&
      (!sub.stepsCount || sub.stepsCount === 0) &&
      (!sub.durationMs || sub.durationMs === 0);

    if (!isPhantomCandidate) return true;

    const subPrompt = sub.prompt?.trim();
    const subTitle = sub.title?.trim();

    const hasBetterDuplicate = all.some((other, otherIdx) => {
      if (idx === otherIdx) return false;

      const otherPrompt = other.prompt?.trim();
      const otherTitle = other.title?.trim();

      const samePrompt = Boolean(subPrompt && otherPrompt && subPrompt === otherPrompt);
      const sameTitle = Boolean(
        subTitle &&
        otherTitle &&
        subTitle === otherTitle &&
        subTitle !== 'Делегированная задача' &&
        subTitle !== 'Субагент' &&
        (!subPrompt || !otherPrompt || subPrompt === otherPrompt)
      );

      if (!samePrompt && !sameTitle) return false;

      // If the duplicate entry is actively running, it is always the authentic subagent
      if (other.status === 'running') return true;

      const otherHasStepsOrDuration =
        (typeof other.stepsCount === 'number' && other.stepsCount > 0) ||
        (typeof other.durationMs === 'number' && other.durationMs > 0);

      if (otherHasStepsOrDuration) return true;

      const otherIsFinished = other.status === 'finished';
      if (otherIsFinished && (other.stepsCount || 0) > (sub.stepsCount || 0)) return true;
      if (otherIsFinished && (other.durationMs || 0) > (sub.durationMs || 0)) return true;

      // Both are non-running 0-step phantoms with identical prompt/title: keep first
      if (otherIdx < idx && samePrompt && sameTitle) return true;

      return false;
    });

    return !hasBetterDuplicate;
  });

  // Sort stably by creation order: createdAt ascending
  result.sort((a, b) => {
    const aTime = a.createdAt || 0;
    const bTime = b.createdAt || 0;
    if (aTime && bTime && aTime !== bTime) {
      return aTime - bTime;
    }
    return 0;
  });

  return result;
}

export function getSubagentOrderMap(
  taskId: string,
  storeSubagents: Record<string, SubagentInfo[]>,
  toolExecutions: Record<string, ToolExecution[]>,
  toolIdToSubagentId: Record<string, string>,
  boardTasks: KanbanTask[],
  columns?: { id: string; persona?: string | null }[]
): Map<string, number> {
  const map = new Map<string, number>();
  if (!taskId) return map;
  const list = getSubagentsForTask(taskId, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks, columns);
  list.forEach((sub, idx) => {
    const num = idx + 1;
    map.set(sub.id, num);
    if (sub.toolCallId) {
      map.set(sub.toolCallId, num);
    }
    const mapped = toolIdToSubagentId[sub.id];
    if (mapped) {
      map.set(mapped, num);
    }
  });
  return map;
}

const DEFAULT_SIDEBAR_WIDTH = 320;
const MIN_SIDEBAR_WIDTH = 240;
const MAX_SIDEBAR_WIDTH = 600;

export default function SubagentsSidebar({
  taskId,
  taskTitle: _taskTitle = '',
  isCollapsed,
  onToggleCollapse,
  className = '',
}: SubagentsSidebarProps) {
  const {
    subagents: storeSubagents,
    activeSubagentId,
    openSubagent,
    closeSubagent,
    openDrawer,
    toolExecutions,
    activityFeed,
    toolIdToSubagentId,
  } = useDrawerStore();

  const boardTasks = useBoardStore((s) => s.tasks);
  const columns = useBoardStore((s) => s.columns);
  const agentPersonas = useCortexStore((s) => s.agentPersonas);

  // Width & Resizability (default 320px, stored in localStorage)
  const [sidebarWidth, setSidebarWidth] = useState<number>(() => {
    try {
      const saved = localStorage.getItem('cvh_subagents_sidebar_width');
      if (saved) {
        const parsed = parseInt(saved, 10);
        if (!isNaN(parsed) && parsed >= MIN_SIDEBAR_WIDTH && parsed <= MAX_SIDEBAR_WIDTH) {
          return parsed;
        }
      }
    } catch {}
    return DEFAULT_SIDEBAR_WIDTH;
  });
  const [isResizing, setIsResizing] = useState(false);
  const isDraggingRef = useRef(false);
  const startXRef = useRef(0);
  const startWidthRef = useRef(sidebarWidth);
  const activeMouseMoveRef = useRef<((e: MouseEvent) => void) | null>(null);
  const activeMouseUpRef = useRef<(() => void) | null>(null);

  // Drag handle resize logic
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    isDraggingRef.current = true;
    startXRef.current = e.clientX;
    startWidthRef.current = sidebarWidth;
    setIsResizing(true);
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (!isDraggingRef.current) return;
      const delta = moveEvent.clientX - startXRef.current;
      const maxAllowed = Math.min(MAX_SIDEBAR_WIDTH, window.innerWidth * 0.5);
      const newWidth = Math.min(Math.max(MIN_SIDEBAR_WIDTH, startWidthRef.current + delta), maxAllowed);
      setSidebarWidth(newWidth);
    };

    const handleMouseUp = () => {
      if (isDraggingRef.current) {
        isDraggingRef.current = false;
        setIsResizing(false);
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
        setSidebarWidth((finalWidth) => {
          try {
            localStorage.setItem('cvh_subagents_sidebar_width', String(Math.round(finalWidth)));
          } catch {}
          return finalWidth;
        });
      }
      activeMouseMoveRef.current = null;
      activeMouseUpRef.current = null;
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };

    activeMouseMoveRef.current = handleMouseMove;
    activeMouseUpRef.current = handleMouseUp;
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  }, [sidebarWidth]);

  useEffect(() => {
    return () => {
      if (activeMouseMoveRef.current) {
        window.removeEventListener('mousemove', activeMouseMoveRef.current);
      }
      if (activeMouseUpRef.current) {
        window.removeEventListener('mouseup', activeMouseUpRef.current);
      }
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
  }, []);

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
      className={`relative shrink-0 mt-1 ml-1.5 rounded-tl-xl rounded-tr-xl border-t border-x border-[#ebebeb] dark:border-[#21262d] shadow-2xs bg-white dark:bg-[#0d1117] flex flex-col min-h-0 h-[calc(100%-0.25rem)] overflow-hidden select-none transition-colors ${
        isResizing ? 'select-none' : 'transition-[width] duration-200 ease-out'
      } ${className || ''}`}
      aria-label={t('subagents.panelAria')}
    >
      {/* Collapsed Rail View (44px wide, smoothly fades in when collapsed) */}
      <div
        className={`absolute inset-0 w-11 bg-white dark:bg-[#0d1117] flex flex-col items-center py-3 select-none transition-opacity duration-200 ${
          isCollapsed ? 'opacity-100 pointer-events-auto z-10' : 'opacity-0 pointer-events-none -z-10'
        }`}
      >
        <button
          type="button"
          onClick={onToggleCollapse}
          className="p-1.5 rounded-md text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] transition cursor-pointer"
          aria-label={t('subagents.expandPanel')}
        >
          <PanelLeftOpen className="h-4 w-4" />
        </button>
        <div
          onClick={onToggleCollapse}
          className="mt-6 flex-1 flex items-center justify-center cursor-pointer group"
          aria-label={t('subagents.expandPanel')}
        >
          <span className="text-[11px] font-mono text-[#8f8f8f] dark:text-[#8b949e] group-hover:text-[#171717] dark:group-hover:text-[#f0f6fc] transition tracking-wider [writing-mode:vertical-rl] rotate-180">
            {t('subagents.title')}
          </span>
        </div>
      </div>

      {/* Expanded Content View (Fixed layout width = sidebarWidth, stays mounted, fades out when collapsed) */}
      <div
        style={{ width: `${sidebarWidth}px` }}
        className={`h-full flex flex-col min-h-0 overflow-hidden transition-opacity duration-200 ${
          isCollapsed ? 'opacity-0 pointer-events-none' : 'opacity-100 pointer-events-auto'
        }`}
      >
        {/* Right Resize Drag Handle (Only active when expanded) */}
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

        {/* Sidebar Header */}
        <div className="h-10 px-3.5 border-b border-[#ebebeb] dark:border-[#21262d] flex items-center justify-between shrink-0 bg-[#fafafa] dark:bg-[#161b22]">
          <div className="flex items-center space-x-2 min-w-0">
            <Bot className="h-4 w-4 text-[#171717] dark:text-[#f0f6fc] shrink-0" />
            <span className="font-sans font-semibold text-xs text-[#171717] dark:text-[#f0f6fc] tracking-tight">
              {t('subagents.title')}
            </span>
            <span className="font-mono text-[11px] tabular-nums px-1.5 py-0.2 rounded-full bg-[#e4e4e7] dark:bg-[#21262d] text-[#52525b] dark:text-[#8b949e] font-medium">
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
            className="p-1 rounded-md text-[#71717a] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#e4e4e7] dark:hover:bg-[#21262d] transition cursor-pointer"
            aria-label={t('subagents.collapsePanel')}
          >
            <PanelLeftClose className="h-3.5 w-3.5" />
          </button>
        </div>

        {/* Subagents List */}
        <div className="flex-1 min-h-0 overflow-y-auto p-3 space-y-3">
          {allSubagents.length === 0 ? (
            <div className="flex-1" />
          ) : (
            <>
              {/* 1. Board Subtasks Section */}
              {boardSubtasks.length > 0 && (
                <div className="space-y-2">
                  <div className="flex items-center justify-between px-1 pt-0.5 pb-0.5">
                    <span className="text-[11px] font-mono uppercase tracking-wider text-[#71717a] dark:text-[#8b949e] font-semibold">
                      {t('subagents.boardSubtasks')}
                    </span>
                    <span className="text-[10px] font-mono px-1.5 py-0.2 rounded-full bg-[#e4e4e7] dark:bg-[#21262d] text-[#52525b] dark:text-[#8b949e] font-medium">
                      {boardSubtasks.length}
                    </span>
                  </div>
                  {boardSubtasks.map((sub) => {
                    const matchedPersona = agentPersonas.find(
                      (p) => p.role.toLowerCase() === (sub.role || '').toLowerCase()
                    );
                    const roleBadgeStyle = getRoleBadgeStyle(sub.role, matchedPersona?.color);
                    const stageBadge = getSubagentRoleLabel(sub.role || 'developer', language === 'ru');

                    return (
                      <div
                        key={sub.id}
                        role="button"
                        tabIndex={0}
                        onClick={() => {
                          closeSubagent();
                          openDrawer(sub.id);
                        }}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter' || e.key === ' ') {
                            e.preventDefault();
                            closeSubagent();
                            openDrawer(sub.id);
                          }
                        }}
                        className="group relative rounded-lg border p-2.5 transition-all duration-150 cursor-pointer select-none text-left bg-white dark:bg-[#0d1117] border-[#ebebeb] dark:border-[#21262d] hover:bg-[#fafafa] dark:hover:bg-[#161b22] hover:border-[#d4d4d8] dark:hover:border-[#30363d]"
                      >
                        {/* Top row: GitBranch + Stage Badge + Task ID + Status */}
                        <div className="flex items-center justify-between gap-1.5 mb-1.5">
                          <div className="flex items-center space-x-1.5 min-w-0 flex-wrap gap-y-1">
                            <GitBranch className="h-3.5 w-3.5 text-indigo-500 dark:text-indigo-400 shrink-0" />
                            <span
                              style={roleBadgeStyle}
                              className="px-2 py-0.5 rounded text-[11px] font-mono font-medium border shrink-0 select-none"
                            >
                              {stageBadge}
                            </span>
                            <span className="font-mono text-[11px] font-semibold text-[#52525b] dark:text-[#8b949e]">
                              {sub.id}
                            </span>
                          </div>

                          <div className="shrink-0 flex items-center space-x-1">
                            {sub.status === 'running' ? (
                              <>
                                <span className="flex items-center justify-center h-5 w-5">
                                  <span className="h-2 w-2 rounded-full bg-emerald-500 animate-pulse shrink-0 ring-2 ring-emerald-500/20" />
                                </span>
                                <button
                                  type="button"
                                  onClick={async (e) => {
                                    e.stopPropagation();
                                    try {
                                      const { invoke } = await import('@tauri-apps/api/core');
                                      await invoke('abort_agent', { taskId: sub.id });
                                      useDrawerStore.getState().setActiveAgentTask(sub.id, false);
                                      useDrawerStore.getState().updateSubagentStatus(sub.id, 'stopped', 'Прервано пользователем');
                                      const boardTask = useBoardStore.getState().tasks.find((t) => t.id === sub.id);
                                      if (boardTask) {
                                        useBoardStore.getState().updateTask(sub.id, { status: 'paused', isTimerRunning: false });
                                      }
                                    } catch (err) {
                                      console.warn('Failed to abort board subtask', sub.id, err);
                                    }
                                  }}
                                  className="flex items-center justify-center h-5 w-5 rounded text-[#dc2626] dark:text-[#ff7b72] hover:bg-[#fef2f2] dark:hover:bg-[#3b1219] border border-[#fecaca] dark:border-[#da3633] transition cursor-pointer shrink-0"
                                  aria-label={t('subagents.stop')}
                                >
                                  <Square className="h-2.5 w-2.5 fill-current shrink-0" />
                                </button>
                              </>
                            ) : sub.status === 'error' || sub.status === 'stopped' ? (
                              <span className="flex items-center justify-center h-5 w-5">
                                <span className="h-2 w-2 rounded-full bg-red-500 shrink-0" />
                              </span>
                            ) : (
                              <span className="flex items-center justify-center h-5 w-5">
                                <span className="h-2 w-2 rounded-full bg-[#a1a1aa] dark:bg-[#52525b] shrink-0" />
                              </span>
                            )}
                          </div>
                        </div>

                        {/* Title */}
                        <div className="font-sans font-medium text-xs text-[#171717] dark:text-[#f0f6fc] group-hover:text-black dark:group-hover:text-white transition-colors line-clamp-2 leading-snug mb-2">
                          {sub.title}
                        </div>

                        {/* Footer: Duration + Steps */}
                        <div className="flex items-center justify-between text-[11px] font-mono text-[#71717a] dark:text-[#8b949e] pt-1.5 border-t border-[#f4f4f5] dark:border-[#21262d]/60">
                          <div className="flex items-center space-x-2">
                            <div className="flex items-center space-x-1">
                              <Clock className="h-3 w-3 shrink-0 text-[#a1a1aa] dark:text-[#6e7681]" />
                              <span className="tabular-nums">
                                {formatDuration(sub.durationMs, sub.status === 'running')}
                              </span>
                            </div>
                            <div className="flex items-center space-x-1">
                              <Layers className="h-3 w-3 shrink-0 text-[#a1a1aa] dark:text-[#6e7681]" />
                              <span className="tabular-nums">{formatSteps(sub.stepsCount || 0, language)}</span>
                            </div>
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}

              {/* 2. In-Process Subagents Section */}
              {inProcessSubagents.length > 0 && (
                <div className="space-y-2">
                  <div className="flex items-center justify-between px-1 pt-2 pb-0.5">
                    <span className="text-[11px] font-mono uppercase tracking-wider text-[#71717a] dark:text-[#8b949e] font-semibold">
                      {t('subagents.inProcess')}
                    </span>
                    <span className="text-[10px] font-mono px-1.5 py-0.2 rounded-full bg-[#e4e4e7] dark:bg-[#21262d] text-[#52525b] dark:text-[#8b949e] font-medium">
                      {inProcessSubagents.length}
                    </span>
                  </div>
                  {inProcessSubagents.map((sub, idx) => {
                    const isActive =
                      activeSubagentId === sub.id ||
                      (sub.toolCallId && activeSubagentId === sub.toolCallId) ||
                      (toolIdToSubagentId[activeSubagentId || ''] === sub.id);
                    const matchedPersona = agentPersonas.find(
                      (p) => p.role.toLowerCase() === (sub.role || '').toLowerCase()
                    );
                    const roleBadgeStyle = getRoleBadgeStyle(sub.role, matchedPersona?.color);
                    const activeCardStyle = getRoleActiveCardStyle(sub.role, matchedPersona?.color);
                    const subIndex = orderMap.get(sub.id) ?? (sub.toolCallId ? orderMap.get(sub.toolCallId) : null) ?? (idx + 1);
                    const roleDisplay = formatSubagentRoleDisplay(sub.role, subIndex, false, language);

                    return (
                      <div
                        key={sub.id}
                        role="button"
                        tabIndex={0}
                        onClick={() => {
                          if (isActive) {
                            closeSubagent();
                          } else {
                            openSubagent(sub.id, sub.title, sub.role, sub.prompt);
                          }
                        }}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter' || e.key === ' ') {
                            e.preventDefault();
                            if (isActive) closeSubagent();
                            else openSubagent(sub.id, sub.title, sub.role, sub.prompt);
                          }
                        }}
                        className={`group relative rounded-lg border p-2.5 transition-all duration-150 cursor-pointer select-none text-left ${
                          isActive
                            ? 'shadow-xs'
                            : 'bg-white dark:bg-[#0d1117] border-[#ebebeb] dark:border-[#21262d] hover:bg-[#fafafa] dark:hover:bg-[#161b22] hover:border-[#d4d4d8] dark:hover:border-[#30363d]'
                        }`}
                        style={isActive ? activeCardStyle : undefined}
                      >
                        {/* Card Top Row: Role Icon + Role Badge + Status */}
                        <div className="flex items-center justify-between gap-1.5 mb-1.5">
                          <div className="flex items-center space-x-1.5 min-w-0 flex-wrap gap-y-1">
                            {sub.isBg ? (
                              <div className="relative shrink-0 flex items-center justify-center">
                                <Layers className="h-3.5 w-3.5 text-purple-600 dark:text-purple-400" />
                                <span className="absolute -bottom-0.5 -right-0.5 h-1.5 w-1.5 rounded-full bg-purple-500 ring-1 ring-white dark:ring-[#0d1117]" />
                              </div>
                            ) : (
                              <div className="relative shrink-0 flex items-center justify-center">
                                <Zap className="h-3.5 w-3.5 text-sky-600 dark:text-sky-400 fill-sky-500/20" />
                                <span className="absolute -bottom-0.5 -right-0.5 h-1.5 w-1.5 rounded-full bg-sky-500 ring-1 ring-white dark:ring-[#0d1117]" />
                              </div>
                            )}
                            <span
                              style={roleBadgeStyle}
                              className="px-2 py-0.5 rounded text-[11px] font-mono font-medium border shrink-0 select-none"
                            >
                              {roleDisplay}
                            </span>
                          </div>

                          {/* Status Indicator & Granular Stop Button */}
                          <div className="shrink-0 flex items-center space-x-1">
                            {sub.status === 'running' ? (
                              <>
                                <span className="flex items-center justify-center h-5 w-5">
                                  <span className="h-2 w-2 rounded-full bg-emerald-500 animate-pulse shrink-0 ring-2 ring-emerald-500/20" />
                                </span>
                                <button
                                  type="button"
                                  onClick={async (e) => {
                                    e.stopPropagation();
                                    try {
                                      const { invoke } = await import('@tauri-apps/api/core');
                                      await invoke('abort_agent', { taskId: sub.id });
                                      useDrawerStore.getState().setActiveAgentTask(sub.id, false);
                                      useDrawerStore.getState().updateSubagentStatus(sub.id, 'stopped', 'Прервано пользователем');
                                      const tools = useDrawerStore.getState().toolExecutions[sub.id] || [];
                                      for (const t of tools) {
                                        if (t.status === 'running') {
                                          useDrawerStore.getState().updateToolExecution(sub.id, t.id, {
                                            status: 'failed',
                                            stderr: 'Прервано пользователем',
                                          });
                                        }
                                      }
                                    } catch (err) {
                                      console.warn('Failed to abort subagent', sub.id, err);
                                    }
                                  }}
                                  className="flex items-center justify-center h-5 w-5 rounded text-[#dc2626] dark:text-[#ff7b72] hover:bg-[#fef2f2] dark:hover:bg-[#3b1219] border border-[#fecaca] dark:border-[#da3633] transition cursor-pointer shrink-0"
                                  aria-label={t('subagents.stop')}
                                >
                                  <Square className="h-2.5 w-2.5 fill-current shrink-0" />
                                </button>
                              </>
                            ) : sub.status === 'error' || sub.status === 'stopped' ? (
                              <span className="flex items-center justify-center h-5 w-5">
                                <span className="h-2 w-2 rounded-full bg-red-500 shrink-0" />
                              </span>
                            ) : (
                              <span className="flex items-center justify-center h-5 w-5">
                                <span className="h-2 w-2 rounded-full bg-[#a1a1aa] dark:bg-[#52525b] shrink-0" />
                              </span>
                            )}
                          </div>
                        </div>

                        {/* Subagent Title */}
                        <div className="font-sans font-medium text-xs text-[#171717] dark:text-[#f0f6fc] group-hover:text-black dark:group-hover:text-white transition-colors line-clamp-2 leading-snug mb-2">
                          {formatSubagentDisplayTitle(sub.title, language)}
                        </div>

                        {/* Card Footer: Metrics (Duration + Steps + Execution Mode) */}
                        <div className="flex items-center justify-between text-[11px] font-mono text-[#71717a] dark:text-[#8b949e] pt-1.5 border-t border-[#f4f4f5] dark:border-[#21262d]/60">
                          <div className="flex items-center space-x-2">
                            <div
                              className="flex items-center space-x-1"
                            >
                              <Clock className="h-3 w-3 shrink-0 text-[#a1a1aa] dark:text-[#6e7681]" />
                              <span className="tabular-nums">
                                {(() => {
                                  const isSubRunning = sub.status === 'running';
                                  if (isSubRunning) {
                                    // Real elapsed from sub.createdAt to now for running subagents
                                    const liveMs = sub.createdAt && sub.createdAt > 0
                                      ? Math.max(0, now - sub.createdAt)
                                      : 0;
                                    return formatDuration(liveMs, true);
                                  }
                                  // For completed subagents: real durationMs or finish - start difference
                                  let finalMs = sub.durationMs;
                                  // Guard against 181ms spawn duration on bg subagents
                                  if (sub.isBg && finalMs !== undefined && finalMs < 1000 && sub.createdAt) {
                                    if (sub.finishedAt) {
                                      finalMs = Math.max(0, sub.finishedAt - sub.createdAt);
                                    } else {
                                      finalMs = undefined;
                                    }
                                  }
                                  if (!finalMs && sub.createdAt && sub.finishedAt) {
                                    finalMs = Math.max(0, sub.finishedAt - sub.createdAt);
                                  }
                                  if (!finalMs && sub.createdAt) {
                                    const feed = activityFeed[sub.id] || (sub.toolCallId ? activityFeed[sub.toolCallId] : []) || [];
                                    let feedDuration = 0;
                                    for (const it of feed) {
                                      if (it.durationMs && it.durationMs > 0) feedDuration += it.durationMs;
                                    }
                                    if (feedDuration > 0) {
                                      finalMs = feedDuration;
                                    }
                                  }
                                  return formatDuration(finalMs, false);
                                })()}
                              </span>
                            </div>
                            <div
                              className="flex items-center space-x-1"
                            >
                              <Layers className="h-3 w-3 shrink-0 text-[#a1a1aa] dark:text-[#6e7681]" />
                              <span className="tabular-nums">{formatSteps(sub.stepsCount || 0, language)}</span>
                            </div>
                          </div>

                          {/* Mode Label */}
                          <div
                            className="flex items-center space-x-1 text-[10px] shrink-0"
                          >
                            {sub.isBg ? (
                              <span className="inline-flex items-center gap-1 font-sans font-medium text-purple-600 dark:text-purple-400">
                                <span className="h-1.5 w-1.5 rounded-full bg-purple-500 shrink-0" />
                                <span>{language === 'ru' ? 'Фоновый субагент' : 'Background agent'}</span>
                              </span>
                            ) : (
                              <span className="inline-flex items-center gap-1 font-sans font-medium text-sky-600 dark:text-sky-400">
                                <span className="h-1.5 w-1.5 rounded-full bg-sky-500 shrink-0" />
                                <span>{language === 'ru' ? 'Синхронный вызов' : 'Synchronous call'}</span>
                              </span>
                            )}
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </>
          )}
        </div>
      </div>
    </aside>
  );
}

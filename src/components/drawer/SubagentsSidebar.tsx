import { useMemo } from 'react';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import { SubagentInfo } from '../../types';
import { getRoleBadgeClass } from './ToolExecutionCard';
import {
  Bot,
  Compass,
  Code2,
  ShieldCheck,
  CheckCircle2,
  AlertCircle,
  Clock,
  Layers,
  ArrowLeft,
  PanelLeftClose,
} from 'lucide-react';

interface SubagentsSidebarProps {
  taskId: string;
  taskTitle: string;
  isCollapsed: boolean;
  onToggleCollapse: () => void;
  className?: string;
}

export function getRoleIcon(role?: string | null) {
  const r = (role || 'general').toLowerCase();
  if (r.includes('exp') || r.includes('arch') || r.includes('res') || r.includes('cortex')) {
    return Compass;
  }
  if (r.includes('dev') || r.includes('code') || r.includes('eng')) {
    return Code2;
  }
  if (r.includes('rev') || r.includes('audit') || r.includes('test') || r.includes('retro') || r.includes('verif')) {
    return ShieldCheck;
  }
  return Bot;
}

function formatDuration(ms?: number): string {
  if (ms === undefined || ms === null || !Number.isFinite(ms) || ms <= 0) return '—';
  if (ms < 1000) return `${Math.round(ms)}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  const mins = Math.floor(ms / 60000);
  const secs = Math.round((ms % 60000) / 1000);
  return `${mins}m ${secs}s`;
}

function formatSteps(count: number): string {
  const mod10 = count % 10;
  const mod100 = count % 100;
  if (mod100 >= 11 && mod100 <= 19) return `${count} шагов`;
  if (mod10 === 1) return `${count} шаг`;
  if (mod10 >= 2 && mod10 <= 4) return `${count} шага`;
  return `${count} шагов`;
}

export default function SubagentsSidebar({
  taskId,
  taskTitle,
  isCollapsed,
  onToggleCollapse,
  className = '',
}: SubagentsSidebarProps) {
  const {
    subagents: storeSubagents,
    activeSubagentId,
    openSubagent,
    closeSubagent,
    toolExecutions,
    activityFeed,
    toolIdToSubagentId,
  } = useDrawerStore();

  const boardTasks = useBoardStore((s) => s.tasks);

  // Combine and deduplicate subagents from all available sources:
  // 1. drawerStore.subagents[taskId]
  // 2. toolExecutions[taskId] (where toolName === 'subagent' || 'subagent_bg')
  // 3. board tasks with parentId === taskId
  const allSubagents = useMemo(() => {
    const map = new Map<string, SubagentInfo>();

    // 1. Explicit store subagents (from backend list_subagents + live events)
    const list = storeSubagents[taskId] || [];
    for (const sub of list) {
      map.set(sub.id, { ...sub });
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
        const feed = activityFeed[subId] || activityFeed[t.id] || [];
        const existing = map.get(subId);

        if (!existing) {
          map.set(subId, {
            id: subId,
            parentTaskId: taskId,
            toolCallId: t.id,
            title: (params?.title as string) || (params?.prompt as string)?.slice(0, 50) || 'Делегированная задача',
            role: (params?.role as string) || 'general',
            prompt: (params?.prompt as string) || '',
            status: t.status === 'running' ? 'running' : t.status === 'failed' ? 'error' : 'finished',
            durationMs: t.durationMs,
            stepsCount: feed.length,
          });
        } else {
          // Augment with execution duration and steps count if missing
          if (!existing.durationMs && t.durationMs) {
            existing.durationMs = t.durationMs;
          }
          if (existing.stepsCount <= 1 && feed.length > 1) {
            existing.stepsCount = feed.length;
          }
        }
      }
    }

    // 3. Subtasks on the Kanban board
    const childTasks = boardTasks.filter((t) => t.parentId === taskId);
    for (const child of childTasks) {
      if (!map.has(child.id)) {
        map.set(child.id, {
          id: child.id,
          parentTaskId: taskId,
          title: child.title,
          role: child.persona || 'developer',
          prompt: child.description || '',
          status: child.status === 'running' ? 'running' : child.status === 'done' ? 'finished' : 'finished',
          durationMs: child.totalElapsed ? child.totalElapsed * 1000 : undefined,
          stepsCount: (child.todos || []).length,
        });
      }
    }

    return Array.from(map.values());
  }, [taskId, storeSubagents, toolExecutions, toolIdToSubagentId, activityFeed, boardTasks]);

  if (isCollapsed) {
    return null;
  }

  const runningCount = allSubagents.filter((s) => s.status === 'running').length;

  return (
    <aside
      className={`w-72 sm:w-80 shrink-0 border-r border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] flex flex-col min-h-0 overflow-hidden select-none transition-colors ${className}`}
      aria-label="Панель субагентов"
    >
      {/* Sidebar Header */}
      <div className="h-10 px-3.5 border-b border-[#ebebeb] dark:border-[#21262d] flex items-center justify-between shrink-0 bg-[#fafafa] dark:bg-[#161b22]">
        <div className="flex items-center space-x-2 min-w-0">
          <Bot className="h-4 w-4 text-[#171717] dark:text-[#f0f6fc] shrink-0" />
          <span className="font-sans font-semibold text-xs text-[#171717] dark:text-[#f0f6fc] tracking-tight">
            Субагенты
          </span>
          <span className="font-mono text-[11px] tabular-nums px-1.5 py-0.2 rounded-full bg-[#e4e4e7] dark:bg-[#21262d] text-[#52525b] dark:text-[#8b949e] font-medium">
            {allSubagents.length}
          </span>
          {runningCount > 0 && (
            <span className="flex items-center space-x-1 px-1.5 py-0.5 rounded-full bg-[#f0fdf4] dark:bg-[#12281e] text-[#15803d] dark:text-[#7ee787] text-[10px] font-mono border border-[#bbf7d0] dark:border-[#238636]">
              <span className="h-1.5 w-1.5 rounded-full bg-emerald-500 animate-pulse" />
              <span>{runningCount} в работе</span>
            </span>
          )}
        </div>

        <button
          type="button"
          onClick={onToggleCollapse}
          className="p-1 rounded-md text-[#71717a] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#e4e4e7] dark:hover:bg-[#21262d] transition cursor-pointer"
          title="Скрыть панель субагентов"
        >
          <PanelLeftClose className="h-3.5 w-3.5" />
        </button>
      </div>

      {/* Subagents List */}
      <div className="flex-1 min-h-0 overflow-y-auto p-3 space-y-2">
        {/* Prominent 'Back to Main Task' Button when a subagent is currently active */}
        {activeSubagentId && (
          <button
            type="button"
            onClick={closeSubagent}
            className="w-full flex items-center justify-between px-3 py-2 rounded-lg bg-[#f0fdf4] dark:bg-[#12281e] text-[#15803d] dark:text-[#7ee787] border border-[#bbf7d0] dark:border-[#238636] hover:bg-[#dcfce7] dark:hover:bg-[#173a2b] transition font-sans text-xs font-semibold cursor-pointer shadow-xs active:scale-[0.98] mb-3 group"
            title="Вернуться к родительской задаче"
          >
            <div className="flex items-center space-x-2 truncate min-w-0">
              <ArrowLeft className="h-3.5 w-3.5 shrink-0 transition-transform group-hover:-translate-x-0.5" />
              <span className="truncate">К задаче: {taskTitle || taskId}</span>
            </div>
            <span className="text-[9px] uppercase font-mono px-1.5 py-0.5 rounded bg-emerald-100 dark:bg-emerald-900/60 font-bold shrink-0 ml-1">
              Главная
            </span>
          </button>
        )}

        {allSubagents.length === 0 ? (
          /* Empty State */
          <div className="py-10 px-4 text-center flex flex-col items-center justify-center text-[#71717a] dark:text-[#8b949e]">
            <div className="h-10 w-10 rounded-full bg-[#f4f4f5] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] flex items-center justify-center mb-3">
              <Bot className="h-5 w-5 text-[#a1a1aa] dark:text-[#6e7681]" />
            </div>
            <div className="text-xs font-medium text-[#171717] dark:text-[#f0f6fc] mb-1">
              Субагенты не вызывались
            </div>
            <p className="text-[11px] leading-relaxed text-[#71717a] dark:text-[#8b949e] max-w-[220px]">
              Список субагентов появится здесь автоматически при вызове инструментов subagent или subagent_bg.
            </p>
          </div>
        ) : (
          allSubagents.map((sub) => {
            const isActive =
              activeSubagentId === sub.id ||
              (sub.toolCallId && activeSubagentId === sub.toolCallId) ||
              (toolIdToSubagentId[activeSubagentId || ''] === sub.id);
            const RoleIcon = getRoleIcon(sub.role);
            const roleBadgeClass = getRoleBadgeClass(sub.role);

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
                    ? 'bg-[#f4f4f5] dark:bg-[#161b22] border-[#171717] dark:border-[#58a6ff] ring-1 ring-[#171717]/20 dark:ring-[#58a6ff]/40 shadow-xs'
                    : 'bg-white dark:bg-[#0d1117] border-[#ebebeb] dark:border-[#21262d] hover:bg-[#fafafa] dark:hover:bg-[#161b22] hover:border-[#d4d4d8] dark:hover:border-[#30363d]'
                }`}
              >
                {/* Active left indicator stripe */}
                {isActive && (
                  <div className="absolute left-0 top-2 bottom-2 w-1 rounded-r bg-[#171717] dark:bg-[#58a6ff]" />
                )}

                {/* Card Top Row: Role Icon + Role Badge + Status */}
                <div className="flex items-center justify-between gap-1.5 mb-1.5">
                  <div className="flex items-center space-x-1.5 min-w-0">
                    <RoleIcon className="h-3.5 w-3.5 text-[#171717] dark:text-[#f0f6fc] shrink-0" />
                    <span
                      className={`px-1.5 py-0.2 rounded text-[9px] font-mono font-bold tracking-wider uppercase border shrink-0 ${roleBadgeClass}`}
                    >
                      {sub.role}
                    </span>
                  </div>

                  {/* Status Indicator */}
                  <div className="shrink-0">
                    {sub.status === 'running' ? (
                      <span className="flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#15803d] dark:text-[#7ee787] bg-[#f0fdf4] dark:bg-[#12281e] border border-[#bbf7d0] dark:border-[#238636]">
                        <span className="h-1.5 w-1.5 rounded-full bg-emerald-500 animate-pulse shrink-0" />
                        <span>Работает</span>
                      </span>
                    ) : sub.status === 'error' ? (
                      <span className="flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#b91c1c] dark:text-[#ff7b72] bg-[#fef2f2] dark:bg-[#3b1219] border border-[#fecaca] dark:border-[#da3633]">
                        <AlertCircle className="h-3 w-3 shrink-0" />
                        <span>Ошибка</span>
                      </span>
                    ) : (
                      <span className="flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono text-[#52525b] dark:text-[#8b949e] bg-[#f4f4f5] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d]">
                        <CheckCircle2 className="h-3 w-3 text-emerald-600 dark:text-emerald-400 shrink-0" />
                        <span>Готово</span>
                      </span>
                    )}
                  </div>
                </div>

                {/* Subagent Title */}
                <div className="font-sans font-medium text-xs text-[#171717] dark:text-[#f0f6fc] group-hover:text-black dark:group-hover:text-white transition-colors line-clamp-2 leading-snug mb-2">
                  {sub.title}
                </div>

                {/* Card Footer: Metrics (Duration + Steps) */}
                <div className="flex items-center justify-between text-[11px] font-mono text-[#71717a] dark:text-[#8b949e] pt-1.5 border-t border-[#f4f4f5] dark:border-[#21262d]/60">
                  <div className="flex items-center space-x-1">
                    <Clock className="h-3 w-3 shrink-0 text-[#a1a1aa] dark:text-[#6e7681]" />
                    <span className="tabular-nums">{formatDuration(sub.durationMs)}</span>
                  </div>

                  <div className="flex items-center space-x-1">
                    <Layers className="h-3 w-3 shrink-0 text-[#a1a1aa] dark:text-[#6e7681]" />
                    <span className="tabular-nums">{formatSteps(sub.stepsCount || 0)}</span>
                  </div>
                </div>
              </div>
            );
          })
        )}
      </div>
    </aside>
  );
}

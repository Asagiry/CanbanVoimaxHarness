import { useState, useMemo, useEffect } from 'react';
import { ToolExecution } from '../../../types';
import { useDrawerStore } from '../../../stores/drawerStore';
import { useBoardStore } from '../../../stores/boardStore';
import { useCortexStore } from '../../../stores/cortexStore';
import { useTranslation } from '../../../stores/languageStore';
import { getRoleBadgeStyle, getDeterministicAgentIcon } from '../../../lib/agentPalette';
import AgentIcon from '../../common/AgentIcon';
import {
  formatSubagentDisplayTitle,
  formatSubagentRoleDisplay,
  getSubagentOrderMap,
  formatDuration,
  formatSteps,
} from '../SubagentsSidebar';
import { formatDurationCompact } from '../ThinkingCard';
import {
  ChevronRight,
  ChevronDown,
  Clock,
  Check,
  X,
  Layers,
  Zap,
  Users,
  FolderDown,
} from 'lucide-react';
import { ToolCardProps } from './shared';

export function CollectSubagentExecutionCard({ tool }: { tool: ToolExecution }) {
  const [isExpanded, setIsExpanded] = useState(true);
  const [now, setNow] = useState(() => Date.now());
  const { t, language } = useTranslation();

  const activeTaskId = useDrawerStore((s) => s.activeTaskId);
  const storeSubagents = useDrawerStore((s) => s.subagents);
  const activityFeed = useDrawerStore((s) => s.activityFeed);
  const toolExecutions = useDrawerStore((s) => s.toolExecutions);
  const toolIdToSubagentId = useDrawerStore((s) => s.toolIdToSubagentId);
  const openSubagent = useDrawerStore((s) => s.openSubagent);
  const boardTasks = useBoardStore((s) => s.tasks);
  const columns = useBoardStore((s) => s.columns);
  const agentPersonas = useCortexStore((s) => s.agentPersonas);

  const params: Record<string, unknown> =
    tool.params && typeof tool.params === 'object'
      ? (tool.params as Record<string, unknown>)
      : {};

  const rawIds: string[] = [];
  const extractIds = (val: unknown) => {
    if (!val) return;
    if (Array.isArray(val)) {
      for (const item of val) {
        if (typeof item === 'string' && item.trim()) rawIds.push(item.trim());
      }
    } else if (typeof val === 'string') {
      const trimmed = val.trim();
      if (!trimmed) return;
      if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
        try {
          const parsed = JSON.parse(trimmed);
          if (Array.isArray(parsed)) {
            for (const item of parsed) {
              if (typeof item === 'string' && item.trim()) rawIds.push(item.trim());
            }
            return;
          }
        } catch {}
      }
      trimmed.split(',').forEach((s) => {
        const part = s.trim();
        if (part) rawIds.push(part);
      });
    }
  };

  extractIds(params?.subagent_ids);
  extractIds(params?.subagent_id);
  extractIds(params?.ids);

  const uniqueIds = Array.from(new Set(rawIds));

  const orderMap = useMemo(() => {
    return activeTaskId
      ? getSubagentOrderMap(activeTaskId, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks, columns)
      : new Map<string, number>();
  }, [activeTaskId, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks, columns]);

  const awaitedSubagents = useMemo(() => {
    const list = (activeTaskId ? storeSubagents[activeTaskId] : null) || [];
    const allTools = (activeTaskId ? toolExecutions[activeTaskId] : null) || [];

    // If no uniqueIds specified, show all known subagents for the task
    const targetIds = uniqueIds.length > 0 ? uniqueIds : list.map((s) => s.id);

    return targetIds.map((id, idx) => {
      // 1. Check in storeSubagents
      const storeSub = list.find((s) => s.id === id || s.toolCallId === id || toolIdToSubagentId[id] === s.id);

      // 2. Check in board subtasks
      const boardSub = boardTasks.find((b) => b.id === id || (b.parentId === activeTaskId && b.id.endsWith(id)));

      // 3. Check in tool executions (tool that spawned this subagent)
      const spawnTool = allTools.find((t) => {
        const p = (t.params || {}) as Record<string, unknown>;
        return (
          t.id === id ||
          p.subagent_id === id ||
          toolIdToSubagentId[t.id] === id
        );
      });

      const resolvedId = storeSub?.id || boardSub?.id || (toolIdToSubagentId[id] || id);

      // Resolve number
      let num = orderMap.get(resolvedId) ?? orderMap.get(id);
      if (num === undefined && storeSub?.toolCallId) {
        num = orderMap.get(storeSub.toolCallId);
      }
      if (num === undefined) {
        const match = id.match(/(?:call_|subagent_|sub-|_|#)(\d+)$/i);
        if (match) num = parseInt(match[1], 10);
      }
      if (num === undefined || isNaN(num)) num = idx + 1;

      // Role
      const role = storeSub?.role || boardSub?.persona || (spawnTool?.params as any)?.role || 'general';

      // Title
      const rawTitle = storeSub?.title || boardSub?.title || (spawnTool?.params as any)?.title || (spawnTool?.params as any)?.prompt;
      const displayTitle = formatSubagentDisplayTitle(rawTitle, language);

      // Status
      let status: 'running' | 'finished' | 'stopped' | 'error' = 'running';
      if (storeSub?.status) {
        status = storeSub.status as any;
      } else if (boardSub?.status) {
        status = boardSub.status === 'done' ? 'finished' : boardSub.status === 'running' ? 'running' : 'finished';
      } else if (tool.status === 'success') {
        status = 'finished';
      } else if (spawnTool) {
        status = spawnTool.status === 'running' ? 'running' : spawnTool.status === 'success' ? 'finished' : 'running';
      }

      // Duration & Start Time
      const durationMs = storeSub?.durationMs ?? spawnTool?.durationMs;
      let startTime: number | undefined = storeSub?.createdAt;
      if (!startTime && spawnTool?.timestamp) {
        const parsed = Date.parse(spawnTool.timestamp);
        if (!isNaN(parsed)) startTime = parsed;
      }

      // Steps count
      const feed = (activityFeed[resolvedId] || activityFeed[id] || (storeSub?.toolCallId ? activityFeed[storeSub.toolCallId] : [])) || [];
      const stepsCount = Math.max(storeSub?.stepsCount || 0, feed.length);

      const isBg = Boolean(storeSub?.isBg || (spawnTool && spawnTool.toolName === 'subagent_bg'));
      const isBoardSubtask = Boolean(boardSub);

      return {
        id: resolvedId,
        rawId: id,
        number: num,
        role,
        title: displayTitle,
        status,
        durationMs,
        startTime,
        stepsCount,
        isBg,
        isBoardSubtask,
      };
    });
  }, [uniqueIds, activeTaskId, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks, orderMap, language, tool.status, activityFeed]);

  const hasRunning = awaitedSubagents.some((s) => s.status === 'running');
  useEffect(() => {
    if (!hasRunning) return;
    const interval = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(interval);
  }, [hasRunning]);

  const isRunning = tool.status === 'running';
  const isSuccess = tool.status === 'success';

  return (
    <div className="rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#161b22] p-3 shadow-xs my-2 select-none transition-all duration-150">
      {/* Header: Clickable to toggle expand/collapse */}
      <div
        onClick={() => setIsExpanded((prev) => !prev)}
        className="flex items-center justify-between gap-2.5 cursor-pointer"
      >
        <div className="flex items-center space-x-2 min-w-0 flex-1 overflow-hidden">
          <FolderDown className="h-4 w-4 text-sky-600 dark:text-sky-400 shrink-0 select-none" />

          <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-medium border border-sky-200 dark:border-sky-800/60 bg-sky-50 dark:bg-sky-950/40 text-sky-700 dark:text-sky-300 shrink-0">
            {t('tools.collect.badge')}
          </span>

          <span className="font-mono text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] shrink-0">
            collect_subagent
          </span>

          {/* Target subagents rendered as separate purple badges */}
          <div className="flex items-center gap-1.5 flex-wrap">
            {uniqueIds.length > 0 ? (
              uniqueIds.map((id, idx) => {
                let num = orderMap.get(id) ?? orderMap.get(toolIdToSubagentId[id]);
                if (num === undefined && activeTaskId) {
                  const found = storeSubagents[activeTaskId]?.find((s) => s.id === id || s.toolCallId === id);
                  if (found) num = orderMap.get(found.id) ?? (found.toolCallId ? orderMap.get(found.toolCallId) : undefined);
                }
                if (num === undefined) {
                  const match = id.match(/(?:call_|subagent_|sub-|_|#)(\d+)$/i);
                  if (match) num = parseInt(match[1], 10);
                }
                if (num === undefined || isNaN(num)) num = idx + 1;

                return (
                  <span
                    key={id}
                    className="px-1.5 py-0.5 rounded text-[10px] font-mono font-medium border border-purple-200 dark:border-purple-800/60 bg-purple-50 dark:bg-purple-950/40 text-purple-700 dark:text-purple-300 shrink-0"
                  >
                    {t('subagents.subagentWithNum', { index: num })}
                  </span>
                );
              })
            ) : (
              <span className="font-mono text-[11px] text-[#71717a] dark:text-[#8b949e] px-1.5 py-0.5 rounded bg-[#f4f4f5] dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] truncate">
                {t('tools.collect.all')}
              </span>
            )}
          </div>
        </div>

        {/* Right side status and Chevron toggle */}
        <div className="flex items-center space-x-2 shrink-0">
          {isRunning ? (
            <div className="flex items-center space-x-1.5">
              <span className="relative flex h-2 w-2 shrink-0">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
                <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500" />
              </span>
              <span className="text-[11px] font-mono text-[#15803d] dark:text-[#7ee787]">
                {t('tools.collect.waiting')}
              </span>
            </div>
          ) : isSuccess ? (
            <div className="flex items-center space-x-1.5">
              <Check className="h-3.5 w-3.5 text-[#15803d] dark:text-[#7ee787] stroke-[2.5]" />
              <span className="text-[11px] font-mono font-medium text-[#15803d] dark:text-[#7ee787]">
                {t('tools.collect.received')}
              </span>
            </div>
          ) : (
            <div className="flex items-center space-x-1.5">
              <X className="h-3.5 w-3.5 text-[#dc2626] dark:text-[#ff7b72] stroke-[2.5]" />
              <span className="text-[11px] font-mono font-medium text-[#dc2626] dark:text-[#ff7b72]">
                {t('tools.collect.error')}
              </span>
            </div>
          )}

          <div className="p-0.5 text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] transition">
            <ChevronDown
              className={`h-4 w-4 transition-transform duration-200 ${
                isExpanded ? 'rotate-180' : ''
              }`}
            />
          </div>
        </div>
      </div>

      {/* Expanded Subagent Cards List & stdout */}
      {isExpanded && (
        <div className="mt-2.5 pt-2 border-t border-[#f4f4f5] dark:border-[#21262d] space-y-1.5">
          {awaitedSubagents.length > 0 ? (
            awaitedSubagents.map((sub) => {
              const persona = agentPersonas.find(
                (p) => p.role.toLowerCase() === (sub.role || '').toLowerCase()
              );
              const roleBadgeStyle = getRoleBadgeStyle(sub.role, persona?.color);
              const personaIcon = persona?.icon || getDeterministicAgentIcon(sub.role);
              const durationStr =
                sub.status === 'running'
                  ? sub.startTime
                    ? formatDuration(Math.max(0, now - sub.startTime), true)
                    : formatDuration(sub.durationMs, true)
                  : formatDuration(sub.durationMs, false);

              const subStatus = sub.status as string;
              const subIsRunning = subStatus === 'running';
              const subIsSuccess = subStatus === 'finished' || subStatus === 'done' || subStatus === 'success';
              const subIsAborted = subStatus === 'stopped' || subStatus === 'aborted' || subStatus === 'paused';

              return (
                <div
                  key={sub.id}
                  onClick={(e) => {
                    e.stopPropagation();
                    openSubagent(sub.id, sub.title, sub.role, '');
                  }}
                  className="group flex items-center justify-between gap-2.5 px-3 py-2 rounded-md bg-[#fafafa] dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#21262d] hover:border-[#171717]/40 dark:hover:border-[#30363d] cursor-pointer transition-colors text-xs"
                >
                  {/* Left: Role icon & badge + Title */}
                  <div className="flex items-center space-x-2 min-w-0 flex-1 overflow-hidden">
                    {sub.isBg ? (
                      <div className="relative shrink-0 flex items-center justify-center">
                        <Layers className="h-3.5 w-3.5 text-purple-600 dark:text-purple-400" />
                        <span className="absolute -bottom-0.5 -right-0.5 h-1.5 w-1.5 rounded-full bg-purple-500 ring-1 ring-white dark:ring-[#0d1117]" />
                      </div>
                    ) : sub.isBoardSubtask ? (
                      <AgentIcon name={personaIcon} className="h-3.5 w-3.5 text-[#171717] dark:text-[#f0f6fc] shrink-0" />
                    ) : (
                      <div className="relative shrink-0 flex items-center justify-center">
                        <Zap className="h-3.5 w-3.5 text-sky-600 dark:text-sky-400 fill-sky-500/20" />
                        <span className="absolute -bottom-0.5 -right-0.5 h-1.5 w-1.5 rounded-full bg-sky-500 ring-1 ring-white dark:ring-[#0d1117]" />
                      </div>
                    )}

                    <span
                      style={roleBadgeStyle}
                      className="px-1.5 py-0.5 rounded text-[10px] font-mono font-medium border shrink-0 select-none"
                    >
                      {formatSubagentRoleDisplay(sub.role, sub.number, sub.isBoardSubtask, language)}
                    </span>

                    <span className="font-sans font-medium text-[11px] text-[#171717] dark:text-[#f0f6fc] group-hover:text-[#000000] dark:group-hover:text-white transition-colors truncate">
                      {sub.title}
                    </span>
                  </div>

                  {/* Right: Duration + Steps + Live status + Chevron */}
                  <div className="flex items-center space-x-2 shrink-0">
                    {durationStr && durationStr !== '—' && (
                      <div className="flex items-center space-x-1 font-mono text-[10px] text-[#71717a] dark:text-[#8b949e]">
                        <Clock className="h-3 w-3 shrink-0" />
                        <span className="tabular-nums">{durationStr}</span>
                      </div>
                    )}

                    <span className="font-mono text-[10px] text-[#71717a] dark:text-[#8b949e] px-1.5 py-0.5 rounded bg-[#f4f4f5] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] shrink-0 tabular-nums">
                      {formatSteps(sub.stepsCount, language)}
                    </span>

                    {subIsRunning ? (
                      <div className="flex items-center space-x-1">
                        <span className="relative flex h-1.5 w-1.5 shrink-0">
                          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
                          <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-emerald-500" />
                        </span>
                        <span className="text-[10px] font-mono text-[#15803d] dark:text-[#7ee787]">
                          {t('tools.status.running')}
                        </span>
                      </div>
                    ) : subIsSuccess ? (
                      <div className="flex items-center space-x-1">
                        <Check className="h-3 w-3 text-[#15803d] dark:text-[#7ee787] stroke-[2.5]" />
                        <span className="text-[10px] font-mono font-medium text-[#15803d] dark:text-[#7ee787]">
                          {t('tools.status.done')}
                        </span>
                      </div>
                    ) : subIsAborted ? (
                      <div className="flex items-center space-x-1">
                        <X className="h-3 w-3 text-[#d97706] dark:text-[#f59e0b] stroke-[2.5]" />
                        <span className="text-[10px] font-mono font-medium text-[#d97706] dark:text-[#f59e0b]">
                          {t('tools.status.aborted')}
                        </span>
                      </div>
                    ) : (
                      <div className="flex items-center space-x-1">
                        <X className="h-3 w-3 text-[#dc2626] dark:text-[#ff7b72] stroke-[2.5]" />
                        <span className="text-[10px] font-mono font-medium text-[#dc2626] dark:text-[#ff7b72]">
                          {t('tools.status.error')}
                        </span>
                      </div>
                    )}

                    <ChevronRight className="h-3.5 w-3.5 text-[#8f8f8f] group-hover:text-[#171717] dark:group-hover:text-white group-hover:translate-x-0.5 transition-all shrink-0" />
                  </div>
                </div>
              );
            })
          ) : (
            <div className="py-2 text-center text-xs text-[#8f8f8f] dark:text-[#8b949e] font-mono">
              {t('tools.subagent_list.notFound')}
            </div>
          )}

          {/* Report excerpt if available */}
          {tool.stdout && (
            <div className="mt-2 pt-2 border-t border-[#f4f4f5] dark:border-[#21262d] text-[11px] font-mono text-[#52525b] dark:text-[#8b949e] line-clamp-3 leading-relaxed">
              {tool.stdout}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default function SubagentToolCard({ tool }: ToolCardProps) {
  const { t, language } = useTranslation();
  const openSubagent = useDrawerStore((s) => s.openSubagent);
  const agentPersonas = useCortexStore((s) => s.agentPersonas);
  const activeTaskId = useDrawerStore((s) => s.activeTaskId);
  const storeSubagents = useDrawerStore((s) => s.subagents);
  const toolExecutions = useDrawerStore((s) => s.toolExecutions);
  const toolIdToSubagentId = useDrawerStore((s) => s.toolIdToSubagentId);
  const boardTasks = useBoardStore((s) => s.tasks);

  const toolName = tool.toolName ?? 'subagent';
  const params: Record<string, unknown> =
    tool.params && typeof tool.params === 'object'
      ? (tool.params as Record<string, unknown>)
      : {};

  if (toolName === 'collect_subagent') {
    return <CollectSubagentExecutionCard tool={tool} />;
  }

  if (toolName === 'subagent_list' || toolName === 'list_subagents') {
    const isRunning = tool.status === 'running';
    const isSuccess = tool.status === 'success';

    const orderMap = activeTaskId
      ? getSubagentOrderMap(activeTaskId, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks)
      : new Map<string, number>();

    const discoveredList: Array<{
      id: string;
      number: number;
      title: string;
      role: string;
      status: string;
      durationMs?: number;
      isBg?: boolean;
    }> = [];

    if (tool.stdout) {
      try {
        const parsed = JSON.parse(tool.stdout);
        const items = Array.isArray(parsed)
          ? parsed
          : Array.isArray(parsed?.subagents)
          ? parsed.subagents
          : Array.isArray(parsed?.items)
          ? parsed.items
          : null;

        if (items) {
          items.forEach((item: any, idx: number) => {
            const sid = (item.id || item.subagent_id || `sub-${idx + 1}`) as string;
            const mappedNum = orderMap.get(sid) ?? orderMap.get(toolIdToSubagentId[sid]) ?? (idx + 1);
            discoveredList.push({
              id: sid,
              number: mappedNum,
              title: (item.title || item.name || item.prompt?.slice(0, 50) || t('subagents.subagentWithNum', { index: mappedNum })) as string,
              role: (item.role || 'general') as string,
              status: (item.status || 'finished') as string,
              durationMs: typeof item.duration_ms === 'number' ? item.duration_ms : typeof item.durationMs === 'number' ? item.durationMs : undefined,
              isBg: Boolean(item.is_bg || item.isBg),
            });
          });
        }
      } catch {}
    }

    if (discoveredList.length === 0 && activeTaskId) {
      const known = storeSubagents[activeTaskId] || [];
      known.forEach((sub, idx) => {
        const mappedNum = orderMap.get(sub.id) ?? orderMap.get(sub.toolCallId || '') ?? (idx + 1);
        discoveredList.push({
          id: sub.id,
          number: mappedNum,
          title: sub.title || t('subagents.subagentWithNum', { index: mappedNum }),
          role: sub.role || 'general',
          status: sub.status,
          durationMs: sub.durationMs,
          isBg: sub.isBg,
        });
      });
    }

    const durationText = formatDurationCompact(tool.durationMs);

    return (
      <div className="rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#161b22] p-3 shadow-xs my-2 select-none">
        <div className="flex items-center justify-between gap-2.5">
          <div className="flex items-center space-x-2 min-w-0 flex-1 overflow-hidden">
            <div className="relative shrink-0 flex items-center justify-center">
              <Users className="h-4 w-4 text-purple-600 dark:text-purple-400" />
            </div>

            <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-medium border border-purple-200 dark:border-purple-800/60 bg-purple-50 dark:bg-purple-950/40 text-purple-700 dark:text-purple-300 shrink-0">
              {t('tools.subagent_list.badge')}
            </span>

            <span className="font-mono text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] shrink-0">
              subagent_list
            </span>

            <span className="font-mono text-[11px] text-[#71717a] dark:text-[#8b949e] px-1.5 py-0.5 rounded bg-[#f4f4f5] dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] truncate">
              {t('tools.subagent_list.total', { count: discoveredList.length })}
            </span>
          </div>

          <div className="flex items-center space-x-2 shrink-0">
            {isRunning ? (
              <div className="flex items-center space-x-1.5">
                <span className="relative flex h-2 w-2 shrink-0">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
                  <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500" />
                </span>
                <span className="text-[11px] font-mono text-[#15803d] dark:text-[#7ee787]">{t('tools.subagent_list.search')}</span>
              </div>
            ) : isSuccess ? (
              <div className="flex items-center space-x-1.5">
                <Check className="h-3.5 w-3.5 text-[#15803d] dark:text-[#7ee787] stroke-[2.5]" />
                <span className="text-[11px] font-mono font-medium text-[#15803d] dark:text-[#7ee787]">
                  {discoveredList.length > 0
                    ? t('tools.subagent_list.found', { count: discoveredList.length })
                    : t('tools.subagent_list.received')}
                </span>
              </div>
            ) : (
              <div className="flex items-center space-x-1.5">
                <X className="h-3.5 w-3.5 text-[#dc2626] dark:text-[#ff7b72] stroke-[2.5]" />
                <span className="text-[11px] font-mono font-medium text-[#dc2626] dark:text-[#ff7b72]">{t('tools.status.error')}</span>
              </div>
            )}

            {durationText && (
              <span className="font-mono text-[#71717a] dark:text-[#8b949e] shrink-0 tabular-nums text-[11px]">
                {durationText}
              </span>
            )}
          </div>
        </div>

        {discoveredList.length > 0 ? (
          <div className="mt-2.5 pt-2 border-t border-[#f4f4f5] dark:border-[#21262d] space-y-1.5">
            {discoveredList.map((sub, idx) => {
              const persona = agentPersonas.find(
                (p) => p.role.toLowerCase() === (sub.role || '').toLowerCase()
              );
              const roleBadgeStyle = getRoleBadgeStyle(sub.role, persona?.color);
              const displaySubTitle = formatSubagentDisplayTitle(sub.title || t('subagents.subagentWithNum', { index: sub.number }), language);
              const subIsRunning = sub.status === 'running';
              const subIsSuccess = sub.status === 'finished' || sub.status === 'done' || (sub.status as string) === 'success';
              const subIsAborted = sub.status === 'stopped' || sub.status === 'aborted';

              return (
                <div
                  key={sub.id || idx}
                  onClick={() => {
                    if (sub.id) openSubagent(sub.id, sub.title, sub.role, '');
                  }}
                  className="group flex items-center justify-between gap-2 px-2.5 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#21262d] hover:border-[#171717]/40 dark:hover:border-[#30363d] cursor-pointer transition-colors text-xs"
                >
                  <div className="flex items-center space-x-2 min-w-0 flex-1 overflow-hidden">
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
                      className="px-1.5 py-0.5 rounded text-[10px] font-mono font-medium border shrink-0 select-none"
                    >
                      {formatSubagentRoleDisplay(sub.role, sub.number, false, language)}
                    </span>

                    <span className="font-sans font-medium text-[11px] text-[#171717] dark:text-[#f0f6fc] group-hover:text-[#000000] dark:group-hover:text-white transition-colors truncate">
                      {displaySubTitle}
                    </span>
                  </div>

                  <div className="flex items-center space-x-2 shrink-0">
                    {subIsRunning ? (
                      <div className="flex items-center space-x-1">
                        <span className="relative flex h-1.5 w-1.5 shrink-0">
                          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
                          <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-emerald-500" />
                        </span>
                        <span className="text-[10px] font-mono text-[#15803d] dark:text-[#7ee787]">
                          {t('tools.status.running')}
                        </span>
                      </div>
                    ) : subIsSuccess ? (
                      <div className="flex items-center space-x-1">
                        <Check className="h-3 w-3 text-[#15803d] dark:text-[#7ee787] stroke-[2.5]" />
                        <span className="text-[10px] font-mono font-medium text-[#15803d] dark:text-[#7ee787]">
                          {t('tools.status.done')}
                        </span>
                      </div>
                    ) : subIsAborted ? (
                      <div className="flex items-center space-x-1">
                        <X className="h-3 w-3 text-[#d97706] dark:text-[#f59e0b] stroke-[2.5]" />
                        <span className="text-[10px] font-mono font-medium text-[#d97706] dark:text-[#f59e0b]">
                          {t('tools.status.aborted')}
                        </span>
                      </div>
                    ) : (
                      <div className="flex items-center space-x-1">
                        <X className="h-3 w-3 text-[#dc2626] dark:text-[#ff7b72] stroke-[2.5]" />
                        <span className="text-[10px] font-mono font-medium text-[#dc2626] dark:text-[#ff7b72]">
                          {t('tools.status.error')}
                        </span>
                      </div>
                    )}

                    {sub.durationMs ? (
                      <span className="font-mono text-[10px] text-[#8f8f8f] dark:text-[#8b949e] tabular-nums">
                        {formatDurationCompact(sub.durationMs)}
                      </span>
                    ) : null}

                    <ChevronRight className="h-3.5 w-3.5 text-[#8f8f8f] group-hover:text-[#171717] dark:group-hover:text-white group-hover:translate-x-0.5 transition-all shrink-0" />
                  </div>
                </div>
              );
            })}
          </div>
        ) : !isRunning ? (
          <div className="mt-2 pt-2 border-t border-[#f4f4f5] dark:border-[#21262d] text-[11px] font-mono text-[#8f8f8f] dark:text-[#8b949e]">
            {t('tools.subagent_list.notFound')}
          </div>
        ) : null}
      </div>
    );
  }

  // subagent / subagent_bg
  const rawRole = (params?.role as string) || 'general';
  const title = (params?.title as string) || (params?.prompt as string)?.slice(0, 60) || t('subagents.delegatedTask');
  const prompt = (params?.prompt as string) || '';
  const isBg = toolName === 'subagent_bg' || (params?.is_bg === true) || (params?.isBg === true);

  let subagentId = typeof params?.subagent_id === 'string' && params.subagent_id ? params.subagent_id : '';
  if (!subagentId) {
    const mapped = useDrawerStore.getState().toolIdToSubagentId[tool.id];
    if (mapped) subagentId = mapped;
  }
  if (!subagentId && tool.stdout) {
    try {
      const parsed = JSON.parse(tool.stdout);
      if (parsed.subagent_id) subagentId = parsed.subagent_id;
    } catch {}
    if (!subagentId) {
      const match = tool.stdout.match(/\[Subagent ID:\s*([^\s\]]+)\]/);
      if (match && match[1]) subagentId = match[1];
    }
  }
  if (!subagentId) {
    subagentId = tool.id;
  }

  const persona = agentPersonas.find(
    (p) => p.role.toLowerCase() === rawRole.toLowerCase()
  );
  const roleBadgeStyle = getRoleBadgeStyle(rawRole, persona?.color);
  const displayTitle = formatSubagentDisplayTitle(title);

  const subagentNumber = (() => {
    if (!activeTaskId) return 1;
    const map = getSubagentOrderMap(activeTaskId, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks);
    return map.get(subagentId) ?? map.get(tool.id) ?? (toolIdToSubagentId[subagentId] ? map.get(toolIdToSubagentId[subagentId]) : null) ?? 1;
  })();

  const list = storeSubagents[activeTaskId || ''] || [];
  const sub = list.find(
    (s) =>
      s.id === subagentId ||
      s.toolCallId === tool.id ||
      (toolIdToSubagentId[tool.id] && s.id === toolIdToSubagentId[tool.id]) ||
      (s.toolCallId && s.toolCallId === subagentId)
  );

  let effectiveStatus: 'running' | 'success' | 'aborted' | 'error';
  if (isBg) {
    if (sub) {
      if (sub.status === 'running') {
        effectiveStatus = 'running';
      } else if (sub.status === 'finished' || (sub.status as string) === 'done') {
        effectiveStatus = 'success';
      } else if (sub.status === 'stopped' || (sub.status as string) === 'aborted') {
        effectiveStatus = 'aborted';
      } else if (sub.status === 'error' || (sub.status as string) === 'failed') {
        effectiveStatus = 'error';
      } else {
        effectiveStatus = 'running';
      }
    } else {
      if (tool.status === 'failed' || (tool.status as string) === 'error') {
        effectiveStatus = 'error';
      } else {
        effectiveStatus = 'running';
      }
    }
  } else {
    if (tool.status === 'running') {
      effectiveStatus = 'running';
    } else if (tool.status === 'failed' || (tool.status as string) === 'error') {
      effectiveStatus = 'error';
    } else if (sub && (sub.status === 'stopped' || (sub.status as string) === 'aborted')) {
      effectiveStatus = 'aborted';
    } else if (sub && (sub.status === 'error' || (sub.status as string) === 'failed')) {
      effectiveStatus = 'error';
    } else if (sub && sub.status === 'running') {
      effectiveStatus = 'running';
    } else if (tool.status === 'success') {
      effectiveStatus = 'success';
    } else {
      effectiveStatus = 'error';
    }
  }

  const isRunning = effectiveStatus === 'running';
  const isSuccess = effectiveStatus === 'success';
  const isAborted = effectiveStatus === 'aborted';

  const subDurationText = (() => {
    if (isRunning) return null;
    if (isBg) {
      if (sub?.durationMs && (!sub.isBg || sub.durationMs >= 1000)) {
        return formatDurationCompact(sub.durationMs);
      }
      if (sub?.createdAt && sub?.finishedAt) {
        return formatDurationCompact(sub.finishedAt - sub.createdAt);
      }
      return null;
    }
    return formatDurationCompact(tool.durationMs);
  })();

  return (
    <div
      onClick={() => openSubagent(subagentId, title, rawRole, prompt)}
      className="group rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#161b22] hover:bg-[#f9f9f9] dark:hover:bg-[#1c2128] hover:border-[#171717]/40 dark:hover:border-[#30363d] p-3 shadow-xs transition-all duration-150 cursor-pointer select-none my-2"
    >
      <div className="flex items-center justify-between gap-2.5">
        <div className="flex items-center space-x-2 min-w-0 flex-1 overflow-hidden">
          {isBg ? (
            <div className="relative shrink-0 flex items-center justify-center">
              <Layers className="h-4 w-4 text-purple-600 dark:text-purple-400" />
              <span className="absolute -bottom-0.5 -right-0.5 h-1.5 w-1.5 rounded-full bg-purple-500 ring-1 ring-white dark:ring-[#161b22]" />
            </div>
          ) : (
            <div className="relative shrink-0 flex items-center justify-center">
              <Zap className="h-4 w-4 text-sky-600 dark:text-sky-400 fill-sky-500/20" />
              <span className="absolute -bottom-0.5 -right-0.5 h-1.5 w-1.5 rounded-full bg-sky-500 ring-1 ring-white dark:ring-[#161b22]" />
            </div>
          )}

          <span
            style={roleBadgeStyle}
            className="px-2 py-0.5 rounded text-[11px] font-mono font-medium border shrink-0 select-none"
          >
            {formatSubagentRoleDisplay(rawRole, subagentNumber || 1, false, language)}
          </span>

          <span className="font-sans font-medium text-xs text-[#0d0d0d] dark:text-[#f0f6fc] group-hover:text-[#171717] dark:group-hover:text-white transition-colors truncate">
            {displayTitle}
          </span>
        </div>

        <div className="flex items-center space-x-2 shrink-0">
          {isRunning ? (
            <div className="flex items-center space-x-1.5">
              <span className="relative flex h-2 w-2 shrink-0">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
                <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500" />
              </span>
              <span className="text-[11px] font-mono text-[#15803d] dark:text-[#7ee787]">{t('tools.status.running')}</span>
            </div>
          ) : isSuccess ? (
            <div className="flex items-center space-x-1.5">
              <Check className="h-3.5 w-3.5 text-[#15803d] dark:text-[#7ee787] stroke-[2.5]" />
              <span className="text-[11px] font-mono font-medium text-[#15803d] dark:text-[#7ee787]">{t('tools.status.done')}</span>
            </div>
          ) : isAborted ? (
            <div className="flex items-center space-x-1.5">
              <X className="h-3.5 w-3.5 text-[#d97706] dark:text-[#f59e0b] stroke-[2.5]" />
              <span className="text-[11px] font-mono font-medium text-[#d97706] dark:text-[#f59e0b]">{t('tools.status.aborted')}</span>
            </div>
          ) : (
            <div className="flex items-center space-x-1.5">
              <X className="h-3.5 w-3.5 text-[#dc2626] dark:text-[#ff7b72] stroke-[2.5]" />
              <span className="text-[11px] font-mono font-medium text-[#dc2626] dark:text-[#ff7b72]">{t('tools.status.error')}</span>
            </div>
          )}

          {subDurationText && (
            <span className="font-mono text-[#71717a] dark:text-[#8b949e] shrink-0 tabular-nums text-[11px]">
              {subDurationText}
            </span>
          )}

          <ChevronRight className="h-4 w-4 text-[#8f8f8f] group-hover:text-[#171717] dark:group-hover:text-white group-hover:translate-x-0.5 transition-all shrink-0" />
        </div>
      </div>

      {prompt && (
        <div className="mt-2 pt-2 border-t border-[#f4f4f5] dark:border-[#21262d] text-[11px] font-mono text-[#52525b] dark:text-[#8b949e] line-clamp-2 leading-relaxed">
          {prompt}
        </div>
      )}
    </div>
  );
}

import { useState, useMemo, useEffect } from 'react';
import { ChevronDown, Check, X, FolderDown } from 'lucide-react';
import { useDrawerStore } from '../../../../stores/drawerStore';
import { useBoardStore } from '../../../../stores/boardStore';
import { useTranslation } from '../../../../stores/languageStore';
import {
  formatSubagentDisplayTitle,
  getSubagentOrderMap,
} from '../../subagentHelpers';
import { ToolCardProps, BatchBadge, useToolExpanded } from '../shared';
import {
  extractSubagentIds,
  AwaitedSubagentItem,
  AwaitedSubagentRow,
} from './AwaitedSubagentRow';

export { extractSubagentIds, type AwaitedSubagentItem, AwaitedSubagentRow };

export function CollectSubagentExecutionCard({ tool, isExpanded: controlledExpanded, onToggleExpanded }: ToolCardProps) {
  const [isExpanded, setIsExpanded] = useToolExpanded(controlledExpanded, onToggleExpanded, false);
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

  const params: Record<string, unknown> =
    tool.params && typeof tool.params === 'object'
      ? (tool.params as Record<string, unknown>)
      : {};

  const uniqueIds = useMemo(() => extractSubagentIds(params), [params]);

  const orderMap = useMemo(() => {
    return activeTaskId
      ? getSubagentOrderMap(activeTaskId, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks, columns)
      : new Map<string, number>();
  }, [activeTaskId, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks, columns]);

  const awaitedSubagents: AwaitedSubagentItem[] = useMemo(() => {
    const list = (activeTaskId ? storeSubagents[activeTaskId] : null) || [];
    const allTools = (activeTaskId ? toolExecutions[activeTaskId] : null) || [];

    const targetIds = uniqueIds.length > 0 ? uniqueIds : list.map((s) => s.id);

    return targetIds.map((id, idx) => {
      const storeSub = list.find((s) => s.id === id || s.toolCallId === id || toolIdToSubagentId[id] === s.id);
      const boardSub = boardTasks.find((b) => b.id === id || (b.parentId === activeTaskId && b.id.endsWith(id)));

      const spawnTool = allTools.find((t) => {
        const p = (t.params || {}) as Record<string, unknown>;
        return (
          t.id === id ||
          p.subagent_id === id ||
          toolIdToSubagentId[t.id] === id ||
          id.endsWith(t.id) ||
          (id.includes('.sub-') && id.split('.sub-')[1] === t.id) ||
          (Boolean(t.stdout) && t.stdout!.includes(id))
        );
      });

      const resolvedId = storeSub?.id || boardSub?.id || (toolIdToSubagentId[id] || id);

      let num = orderMap.get(resolvedId) ?? orderMap.get(id);
      if (num === undefined && storeSub?.toolCallId) {
        num = orderMap.get(storeSub.toolCallId);
      }
      if (num === undefined) {
        const match = id.match(/(?:call_|subagent_|sub-|_|#)(\d+)$/i);
        if (match) num = parseInt(match[1], 10);
      }
      if (num === undefined || isNaN(num)) num = idx + 1;

      const role = storeSub?.role || boardSub?.persona || (spawnTool?.params as any)?.role || 'general';
      const rawTitle = storeSub?.title || boardSub?.title || (spawnTool?.params as any)?.title || (spawnTool?.params as any)?.prompt;
      const displayTitle = formatSubagentDisplayTitle(rawTitle, language);

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

      const feed = (activityFeed[resolvedId] || activityFeed[id] || (storeSub?.toolCallId ? activityFeed[storeSub.toolCallId] : [])) || [];
      const stepsCount = Math.max(storeSub?.stepsCount || 0, feed.length);

      const candidateTimes: number[] = [];
      if (storeSub?.createdAt && storeSub.createdAt > 0) {
        candidateTimes.push(storeSub.createdAt);
      }
      if (spawnTool?.timestamp) {
        const parsed = Date.parse(spawnTool.timestamp);
        if (!isNaN(parsed) && parsed > 0) {
          candidateTimes.push(parsed);
        }
      }
      if (candidateTimes.length === 0 && feed[0]?.timestamp) {
        const feedParsed = Date.parse(feed[0].timestamp);
        if (!isNaN(feedParsed) && feedParsed > 0) {
          candidateTimes.push(feedParsed);
        }
      }
      const startTime: number | undefined = candidateTimes.length > 0 ? Math.min(...candidateTimes) : undefined;

      let durationMs = storeSub?.durationMs;
      if (!durationMs && storeSub?.finishedAt && startTime) {
        durationMs = Math.max(0, storeSub.finishedAt - startTime);
      }
      if (!durationMs && spawnTool && spawnTool.toolName !== 'subagent_bg') {
        durationMs = spawnTool.durationMs;
      }

      const isBg = Boolean(storeSub?.isBg || (spawnTool && spawnTool.toolName === 'subagent_bg'));
      const isBoardSubtask = Boolean(boardSub);
      const spawnPrompt = (spawnTool?.params as any)?.prompt;
      const prompt =
        storeSub?.prompt ||
        (typeof spawnPrompt === 'string' ? spawnPrompt : '') ||
        (boardSub as any)?.description ||
        '';

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
        prompt,
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
    <div className="rounded-lg border border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#17171b] p-3 shadow-xs my-2 select-none transition-all duration-150">
      <div
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center justify-between gap-2.5 cursor-pointer"
      >
        <div className="flex items-center gap-1.5 min-w-0 flex-1 leading-none">
          <FolderDown className="h-4 w-4 text-sky-600 dark:text-sky-400 shrink-0 select-none" />

          <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-mono font-medium border border-sky-200 dark:border-sky-800/60 bg-sky-50 dark:bg-sky-950/40 text-sky-700 dark:text-sky-300 shrink-0">
            {t('tools.collect.badge')}
          </span>

          <span className="font-mono text-xs font-semibold text-[#171717] dark:text-[#f4f4f5] shrink-0">
            collect_subagent
          </span>

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
                    className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-mono font-medium border border-purple-200 dark:border-purple-800/60 bg-purple-50 dark:bg-purple-950/40 text-purple-700 dark:text-purple-300 shrink-0"
                  >
                    {t('subagents.subagentWithNum', { index: num })}
                  </span>
                );
              })
            ) : (
              <span className="inline-flex items-center font-mono text-[11px] text-[#71717a] dark:text-[#71717a] px-1.5 py-0.5 rounded bg-[#f4f4f5] dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] truncate">
                {t('tools.collect.all')}
              </span>
            )}
          </div>
        </div>

        <div className="flex items-center space-x-2 shrink-0">
          <BatchBadge tool={tool} />
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

          <div className="p-0.5 text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] transition">
            <ChevronDown
              className={`h-4 w-4 transition-transform duration-200 ${
                isExpanded ? 'rotate-180' : ''
              }`}
            />
          </div>
        </div>
      </div>

      {isExpanded && (
        <div className="mt-2.5 pt-2 border-t border-[#f4f4f5] dark:border-[#212126] space-y-1.5">
          {awaitedSubagents.length > 0 ? (
            awaitedSubagents.map((sub) => (
              <AwaitedSubagentRow
                key={sub.id}
                sub={sub}
                now={now}
                language={language}
                onOpen={() => openSubagent(sub.id, sub.title, sub.role, sub.prompt)}
              />
            ))
          ) : (
            <div className="py-2 text-center text-xs text-[#8f8f8f] dark:text-[#71717a] font-mono">
              {t('tools.subagent_list.notFound')}
            </div>
          )}

          {tool.stdout && (
            <div className="mt-2 pt-2 border-t border-[#f4f4f5] dark:border-[#212126] text-[11px] font-mono text-[#52525b] dark:text-[#71717a] line-clamp-3 leading-relaxed">
              {tool.stdout}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

import { useMemo } from 'react';
import { ChevronRight, Users, Check, X } from 'lucide-react';
import type { SubagentInfo } from '../../../../types';
import { useDrawerStore } from '../../../../stores/drawerStore';
import { useBoardStore } from '../../../../stores/boardStore';
import { useCortexStore } from '../../../../stores/cortexStore';
import { useTranslation } from '../../../../stores/languageStore';
import { getRoleBadgeStyle } from '../../../../lib/agentPalette';
import {
  formatSubagentDisplayTitle,
  formatSubagentRoleDisplay,
  getSubagentOrderMap,
} from '../../subagentHelpers';
import { formatDurationCompact } from '../../ThinkingCard';
import { ToolCardProps, BatchBadge, useToolExpanded } from '../shared';
import { SubagentStatusBadge, SubagentTypeIcon } from './SubagentStatusBadge';

export interface DiscoveredSubagent {
  id: string;
  number: number;
  title: string;
  role: string;
  status: string;
  durationMs?: number;
  isBg?: boolean;
  prompt?: string;
}

export function parseDiscoveredSubagents(
  stdout: string | undefined | null,
  activeTaskId: string | null,
  storeSubagents: Record<string, SubagentInfo[]>,
  orderMap: Map<string, number>,
  toolIdToSubagentId: Record<string, string>,
  t: (key: any, options?: any) => string
): DiscoveredSubagent[] {
  const discoveredList: DiscoveredSubagent[] = [];

  if (stdout) {
    try {
      const parsed = JSON.parse(stdout);
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
            prompt: typeof item.prompt === 'string' ? item.prompt : undefined,
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
        prompt: sub.prompt || undefined,
      });
    });
  }

  return discoveredList;
}

export function DiscoveredSubagentRow({
  sub,
  language,
  onOpen,
}: {
  sub: DiscoveredSubagent;
  language: any;
  onOpen: () => void;
}) {
  const { t } = useTranslation();
  const agentPersonas = useCortexStore((s) => s.agentPersonas);
  const persona = agentPersonas.find(
    (p) => p.role.toLowerCase() === (sub.role || '').toLowerCase()
  );
  const roleBadgeStyle = getRoleBadgeStyle(sub.role, persona?.color);
  const displaySubTitle = formatSubagentDisplayTitle(sub.title || t('subagents.subagentWithNum', { index: sub.number }), language);

  return (
    <div
      onClick={onOpen}
      className="group flex items-center justify-between gap-2 px-2.5 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#111114] border border-[#ebebeb] dark:border-[#212126] hover:border-[#171717]/40 dark:hover:border-[#2c2c33] cursor-pointer transition-colors text-xs"
    >
      <div className="flex items-center gap-1.5 min-w-0 flex-1 leading-none">
        <SubagentTypeIcon isBg={sub.isBg} />

        <span
          style={roleBadgeStyle}
          className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-mono font-medium border shrink-0 select-none"
        >
          {formatSubagentRoleDisplay(sub.role, sub.number, false, language)}
        </span>

        <span className="font-sans font-medium text-[11px] text-[#171717] dark:text-[#f4f4f5] group-hover:text-[#000000] dark:group-hover:text-white transition-colors truncate">
          {displaySubTitle}
        </span>
      </div>

      <div className="flex items-center space-x-2 shrink-0">
        <SubagentStatusBadge status={sub.status} t={t} />

        {sub.durationMs ? (
          <span className="font-mono text-[10px] text-[#8f8f8f] dark:text-[#71717a] tabular-nums">
            {formatDurationCompact(sub.durationMs)}
          </span>
        ) : null}

        <ChevronRight className="h-3.5 w-3.5 text-[#8f8f8f] group-hover:text-[#171717] dark:group-hover:text-white group-hover:translate-x-0.5 transition-all shrink-0" />
      </div>
    </div>
  );
}

export function SubagentListExecutionCard({ tool, isExpanded: controlledExpanded, onToggleExpanded }: ToolCardProps) {
  const { t, language } = useTranslation();
  const openSubagent = useDrawerStore((s) => s.openSubagent);
  const activeTaskId = useDrawerStore((s) => s.activeTaskId);
  const storeSubagents = useDrawerStore((s) => s.subagents);
  const toolExecutions = useDrawerStore((s) => s.toolExecutions);
  const toolIdToSubagentId = useDrawerStore((s) => s.toolIdToSubagentId);
  const boardTasks = useBoardStore((s) => s.tasks);

  const isRunning = tool.status === 'running';
  const isSuccess = tool.status === 'success';

  const orderMap = useMemo(() => {
    return activeTaskId
      ? getSubagentOrderMap(activeTaskId, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks)
      : new Map<string, number>();
  }, [activeTaskId, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks]);

  const discoveredList = useMemo(() => {
    return parseDiscoveredSubagents(tool.stdout, activeTaskId, storeSubagents, orderMap, toolIdToSubagentId, t);
  }, [tool.stdout, activeTaskId, storeSubagents, orderMap, toolIdToSubagentId, t]);

  const durationText = formatDurationCompact(tool.durationMs);
  const [isExpanded, setIsExpanded] = useToolExpanded(controlledExpanded, onToggleExpanded, false);

  return (
    <div className="rounded-lg border border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#17171b] p-3 shadow-xs my-2 select-none">
      <div
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center justify-between gap-2.5 cursor-pointer"
      >
        <div className="flex items-center gap-1.5 min-w-0 flex-1 leading-none">
          <ChevronRight
            className={`h-3.5 w-3.5 block text-[#8f8f8f] transition-transform duration-150 shrink-0 ${
              isExpanded ? 'rotate-90' : ''
            }`}
          />
          <div className="relative shrink-0 flex items-center justify-center">
            <Users className="h-4 w-4 text-purple-600 dark:text-purple-400" />
          </div>

          <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-mono font-medium border border-purple-200 dark:border-purple-800/60 bg-purple-50 dark:bg-purple-950/40 text-purple-700 dark:text-purple-300 shrink-0">
            {t('tools.subagent_list.badge')}
          </span>

          <span className="font-mono text-xs font-semibold text-[#171717] dark:text-[#f4f4f5] shrink-0">
            subagent_list
          </span>

          <span className="inline-flex items-center font-mono text-[11px] text-[#71717a] dark:text-[#71717a] px-1.5 py-0.5 rounded bg-[#f4f4f5] dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] truncate">
            {t('tools.subagent_list.total', { count: discoveredList.length })}
          </span>
        </div>

        <div className="flex items-center space-x-2 shrink-0">
          <BatchBadge tool={tool} />
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
            <span className="font-mono text-[#71717a] dark:text-[#71717a] shrink-0 tabular-nums text-[11px]">
              {durationText}
            </span>
          )}
        </div>
      </div>

      {isExpanded && (discoveredList.length > 0 ? (
        <div className="mt-2.5 pt-2 border-t border-[#f4f4f5] dark:border-[#212126] space-y-1.5">
          {discoveredList.map((sub, idx) => (
            <DiscoveredSubagentRow
              key={sub.id || idx}
              sub={sub}
              language={language}
              onOpen={() => {
                if (sub.id) openSubagent(sub.id, sub.title, sub.role, sub.prompt || '');
              }}
            />
          ))}
        </div>
      ) : !isRunning ? (
        <div className="mt-2 pt-2 border-t border-[#f4f4f5] dark:border-[#212126] text-[11px] font-mono text-[#8f8f8f] dark:text-[#71717a]">
          {t('tools.subagent_list.notFound')}
        </div>
      ) : null)}
    </div>
  );
}

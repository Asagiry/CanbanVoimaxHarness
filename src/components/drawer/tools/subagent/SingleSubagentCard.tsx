import { ChevronRight } from 'lucide-react';
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
import { ToolCardProps, useToolExpanded } from '../shared';
import { SubagentStatusBadge, SubagentTypeIcon } from './SubagentStatusBadge';

export function SingleSubagentExecutionCard({ tool, isExpanded: controlledExpanded, onToggleExpanded }: ToolCardProps) {
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
  const displayTitle = formatSubagentDisplayTitle(title, language);

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

  const [isExpanded, setIsExpanded] = useToolExpanded(controlledExpanded, onToggleExpanded, false);

  return (
    <div
      onClick={() => openSubagent(subagentId, title, rawRole, prompt)}
      className="group rounded-lg border border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#17171b] hover:bg-[#f9f9f9] dark:hover:bg-[#1e1e23] hover:border-[#171717]/40 dark:hover:border-[#2c2c33] p-3 shadow-xs transition-all duration-150 cursor-pointer select-none my-2"
    >
      <div className="flex items-center justify-between gap-2.5">
        <div className="flex items-center gap-1.5 min-w-0 flex-1 leading-none">
          <button
            type="button"
            className="p-0.5 text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] rounded shrink-0 transition cursor-pointer"
            onClick={(e) => {
              e.stopPropagation();
              setIsExpanded(!isExpanded);
            }}
            aria-label={isExpanded ? 'Свернуть подробности' : 'Развернуть подробности'}
          >
            <ChevronRight
              className={`h-3.5 w-3.5 block shrink-0 transition-transform duration-150 ${
                isExpanded ? 'rotate-90' : ''
              }`}
            />
          </button>
          <SubagentTypeIcon isBg={isBg} />

          <span
            style={roleBadgeStyle}
            className="inline-flex items-center px-2 py-0.5 rounded text-[11px] font-mono font-medium border shrink-0 select-none"
          >
            {formatSubagentRoleDisplay(rawRole, subagentNumber || 1, false, language)}
          </span>

          <span className="font-sans font-medium text-xs text-[#0d0d0d] dark:text-[#f4f4f5] group-hover:text-[#171717] dark:group-hover:text-white transition-colors truncate">
            {displayTitle}
          </span>
        </div>

        <div className="flex items-center space-x-2 shrink-0">
          <SubagentStatusBadge status={effectiveStatus} t={t} />

          {subDurationText && (
            <span className="font-mono text-[#71717a] dark:text-[#71717a] shrink-0 tabular-nums text-[11px]">
              {subDurationText}
            </span>
          )}

          <ChevronRight className="h-4 w-4 text-[#8f8f8f] group-hover:text-[#171717] dark:group-hover:text-white group-hover:translate-x-0.5 transition-all shrink-0" />
        </div>
      </div>

      {isExpanded && prompt && (
        <div className="mt-2 pt-2 border-t border-[#f4f4f5] dark:border-[#212126] text-[11px] font-mono text-[#52525b] dark:text-[#71717a] line-clamp-2 leading-relaxed">
          {prompt}
        </div>
      )}
    </div>
  );
}

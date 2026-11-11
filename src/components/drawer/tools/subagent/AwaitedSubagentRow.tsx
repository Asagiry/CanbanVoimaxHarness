import { Clock, ChevronRight } from 'lucide-react';
import { useCortexStore } from '../../../../stores/cortexStore';
import { useTranslation, type Language } from '../../../../stores/languageStore';
import { getRoleBadgeStyle, getDeterministicAgentIcon } from '../../../../lib/agentPalette';
import {
  formatSubagentRoleDisplay,
  formatDuration,
  formatSteps,
} from '../../subagentHelpers';
import { SubagentStatusBadge, SubagentTypeIcon } from './SubagentStatusBadge';

export function extractSubagentIds(params?: Record<string, unknown> | null): string[] {
  if (!params) return [];
  const rawIds: string[] = [];
  const extract = (val: unknown) => {
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
          const parsed: unknown = JSON.parse(trimmed);
          if (Array.isArray(parsed)) {
            for (const item of parsed) {
              if (typeof item === 'string' && item.trim()) rawIds.push(item.trim());
            }
            return;
          }
        } catch (err) {
          console.debug('[AwaitedSubagentRow] JSON array parse skipped:', err);
        }
      }
      trimmed.split(',').forEach((s) => {
        const part = s.trim();
        if (part) rawIds.push(part);
      });
    }
  };
  extract(params.subagent_ids);
  extract(params.subagent_id);
  extract(params.ids);
  return Array.from(new Set(rawIds));
}

export interface AwaitedSubagentItem {
  id: string;
  rawId: string;
  number: number;
  role: string;
  title: string;
  status: 'running' | 'finished' | 'stopped' | 'error';
  durationMs?: number;
  startTime?: number;
  stepsCount: number;
  isBg: boolean;
  isBoardSubtask: boolean;
  prompt: string;
}

export function AwaitedSubagentRow({
  sub,
  now,
  language,
  onOpen,
}: {
  sub: AwaitedSubagentItem;
  now: number;
  language: Language;
  onOpen: () => void;
}) {
  const { t } = useTranslation();
  const agentPersonas = useCortexStore((s) => s.agentPersonas);
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

  return (
    <button
      type="button"
      onClick={(e) => {
        e.stopPropagation();
        onOpen();
      }}
      className="w-full text-left group flex items-center justify-between gap-2.5 px-3 py-2 rounded-md bg-[#fafafa] dark:bg-[#111114] border border-[#ebebeb] dark:border-[#212126] hover:border-[#171717]/40 dark:hover:border-[#2c2c33] cursor-pointer transition-colors text-xs"
    >
      <div className="flex items-center gap-1.5 min-w-0 flex-1 leading-none">
        <SubagentTypeIcon isBg={sub.isBg} isBoardSubtask={sub.isBoardSubtask} personaIcon={personaIcon} />

        <span
          style={roleBadgeStyle}
          className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-mono font-medium border shrink-0 select-none"
        >
          {formatSubagentRoleDisplay(sub.role, sub.number, sub.isBoardSubtask, language)}
        </span>

        <span className="font-sans font-medium text-[11px] text-[#171717] dark:text-[#f4f4f5] group-hover:text-[#000000] dark:group-hover:text-white transition-colors truncate">
          {sub.title}
        </span>
      </div>

      <div className="flex items-center space-x-2 shrink-0">
        {durationStr && durationStr !== '—' && (
          <div className="flex items-center space-x-1 font-mono text-[10px] text-[#71717a] dark:text-[#71717a]">
            <Clock className="h-3 w-3 shrink-0" />
            <span className="tabular-nums">{durationStr}</span>
          </div>
        )}

        <span className="inline-flex items-center font-mono text-[10px] text-[#71717a] dark:text-[#71717a] px-1.5 py-0.5 rounded bg-[#f4f4f5] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] shrink-0 tabular-nums">
          {formatSteps(sub.stepsCount, language)}
        </span>

        <SubagentStatusBadge status={sub.status} t={t} />

        <ChevronRight className="h-3.5 w-3.5 text-[#8f8f8f] group-hover:text-[#171717] dark:group-hover:text-white group-hover:translate-x-0.5 transition-all shrink-0" />
      </div>
    </button>
  );
}

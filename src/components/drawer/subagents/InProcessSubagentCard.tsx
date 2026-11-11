import { Layers, Zap, Square, Clock } from 'lucide-react';
import type { SubagentInfo } from '../../../types';
import type { AgentPersonaInfo } from '../../../stores/cortex/types';
import type { Language } from '../../../stores/languageStore';
import { useDrawerStore, DrawerActivityItem } from '../../../stores/drawerStore';
import { useTranslation } from '../../../stores/languageStore';
import { getRoleBadgeStyle, getRoleActiveCardStyle } from '../../../lib/agentPalette';
import {
  formatSubagentRoleDisplay,
  formatSubagentDisplayTitle,
  formatDuration,
  formatSteps,
} from '../subagentHelpers';

export interface InProcessSubagentCardProps {
  sub: SubagentInfo;
  idx: number;
  activeSubagentId: string | null;
  toolIdToSubagentId: Record<string, string>;
  agentPersonas: AgentPersonaInfo[];
  orderMap: Map<string, number>;
  now: number;
  language: Language;
  activityFeed: Record<string, DrawerActivityItem[]>;
}

export function InProcessSubagentCard({
  sub,
  idx,
  activeSubagentId,
  toolIdToSubagentId,
  agentPersonas,
  orderMap,
  now,
  language,
  activityFeed,
}: InProcessSubagentCardProps) {
  const { t } = useTranslation();
  const openSubagent = useDrawerStore((s) => s.openSubagent);
  const closeSubagent = useDrawerStore((s) => s.closeSubagent);

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

  const handleToggleSubagent = () => {
    if (isActive) {
      closeSubagent();
    } else {
      openSubagent(sub.id, sub.title, sub.role, sub.prompt);
    }
  };

  return (
    <div
      className={`group relative rounded-lg border p-2.5 transition-all duration-150 select-none text-left ${
        isActive
          ? 'shadow-xs'
          : 'bg-white dark:bg-[#111114] border-[#ebebeb] dark:border-[#212126] hover:bg-[#fafafa] dark:hover:bg-[#17171b] hover:border-[#d4d4d8] dark:hover:border-[#2c2c33]'
      }`}
      style={isActive ? activeCardStyle : undefined}
    >
      {/* Card Top Row: Role Icon + Role Badge + Status */}
      <div className="flex items-center justify-between gap-1.5 mb-1.5">
        <button
          type="button"
          onClick={handleToggleSubagent}
          className="flex items-center space-x-1.5 min-w-0 flex-1 flex-wrap gap-y-1 text-left cursor-pointer"
        >
          {sub.isBg ? (
            <div className="relative shrink-0 flex items-center justify-center">
              <Layers className="h-3.5 w-3.5 block shrink-0 text-cyan-600 dark:text-cyan-400" />
              <span className="absolute -bottom-0.5 -right-0.5 h-1.5 w-1.5 rounded-full bg-cyan-500 ring-1 ring-white dark:ring-[#111114]" />
            </div>
          ) : (
            <div className="relative shrink-0 flex items-center justify-center">
              <Zap className="h-3.5 w-3.5 block shrink-0 text-sky-600 dark:text-sky-400 fill-sky-500/20" />
              <span className="absolute -bottom-0.5 -right-0.5 h-1.5 w-1.5 rounded-full bg-sky-500 ring-1 ring-white dark:ring-[#111114]" />
            </div>
          )}
          <span
            style={roleBadgeStyle}
            className="px-2 py-0.5 rounded text-[11px] font-mono font-medium border shrink-0 select-none"
          >
            {roleDisplay}
          </span>
        </button>

        {/* Status Indicator & Granular Stop Button */}
        <div className="shrink-0 flex items-center space-x-1">
          {sub.status === 'running' ? (
            <>
              <span className="flex items-center justify-center h-5 w-5">
                <span className="h-2 w-2 rounded-full bg-emerald-500 animate-pulse shrink-0 ring-2 ring-emerald-500/20" />
              </span>
              <button
                type="button"
                onClick={async () => {
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
                <Square className="h-2.5 w-2.5 block shrink-0 fill-current" />
              </button>
            </>
          ) : sub.status === 'paused' ? (
            <span className="flex items-center justify-center h-5 w-5">
              <span className="h-2 w-2 rounded-full bg-amber-500 shrink-0" title="Пауза: временная ошибка провайдера, ждёт Continue/Stop" />
            </span>
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

      {/* Subagent Title & Footer */}
      <button
        type="button"
        onClick={handleToggleSubagent}
        className="w-full text-left cursor-pointer block"
      >
        <div className="font-sans font-medium text-xs text-[#171717] dark:text-[#f4f4f5] group-hover:text-black dark:group-hover:text-white transition-colors line-clamp-2 leading-snug mb-2">
          {formatSubagentDisplayTitle(sub.title, language)}
        </div>

        {/* Card Footer: Metrics (Duration + Steps + Execution Mode) */}
        <div className="flex items-center justify-between text-[11px] font-mono text-[#71717a] dark:text-[#71717a] pt-1.5 border-t border-[#f4f4f5] dark:border-[#212126]/60">
          <div className="flex items-center space-x-2">
            <div className="flex items-center space-x-1">
              <Clock className="h-3 w-3 block shrink-0 text-[#a1a1aa] dark:text-[#52525b]" />
              <span className="tabular-nums">
                {(() => {
                  const isSubRunning = sub.status === 'running';
                  if (isSubRunning) {
                    const liveMs = sub.createdAt && sub.createdAt > 0
                      ? Math.max(0, now - sub.createdAt)
                      : 0;
                    return formatDuration(liveMs, true);
                  }
                  let finalMs = sub.durationMs;
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
            <div className="flex items-center space-x-1">
              <Layers className="h-3 w-3 block shrink-0 text-[#a1a1aa] dark:text-[#52525b]" />
              <span className="tabular-nums">{formatSteps(sub.stepsCount || 0, language)}</span>
            </div>
          </div>

          {/* Mode Label */}
          <div className="flex items-center space-x-1 text-[10px] shrink-0">
            {sub.isBg ? (
              <span className="inline-flex items-center gap-1 font-sans font-medium text-cyan-600 dark:text-cyan-400">
                <span className="h-1.5 w-1.5 rounded-full bg-cyan-500 shrink-0" />
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
      </button>
    </div>
  );
}

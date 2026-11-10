import { GitBranch, Square, Clock, Layers } from 'lucide-react';
import type { SubagentInfo } from '../../../types';
import type { AgentPersonaInfo } from '../../../stores/cortex/types';
import type { Language } from '../../../stores/languageStore';
import { useDrawerStore } from '../../../stores/drawerStore';
import { useBoardStore } from '../../../stores/boardStore';
import { useTranslation } from '../../../stores/languageStore';
import { getRoleBadgeStyle } from '../../../lib/agentPalette';
import { getSubagentRoleLabel, formatDuration, formatSteps } from '../subagentHelpers';

export interface BoardSubtaskCardProps {
  sub: SubagentInfo;
  agentPersonas: AgentPersonaInfo[];
  language: Language;
}

export function BoardSubtaskCard({ sub, agentPersonas, language }: BoardSubtaskCardProps) {
  const { t } = useTranslation();
  const closeSubagent = useDrawerStore((s) => s.closeSubagent);
  const openDrawer = useDrawerStore((s) => s.openDrawer);

  const matchedPersona = agentPersonas.find(
    (p) => p.role.toLowerCase() === (sub.role || '').toLowerCase()
  );
  const roleBadgeStyle = getRoleBadgeStyle(sub.role, matchedPersona?.color);
  const stageBadge = getSubagentRoleLabel(sub.role || 'general', language === 'ru');

  return (
    <div
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
      className="group relative rounded-lg border p-2.5 transition-all duration-150 cursor-pointer select-none text-left bg-white dark:bg-[#111114] border-[#ebebeb] dark:border-[#212126] hover:bg-[#fafafa] dark:hover:bg-[#17171b] hover:border-[#d4d4d8] dark:hover:border-[#2c2c33]"
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
          <span className="font-mono text-[11px] font-semibold text-[#52525b] dark:text-[#71717a]">
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

      {/* Title */}
      <div className="font-sans font-medium text-xs text-[#171717] dark:text-[#f4f4f5] group-hover:text-black dark:group-hover:text-white transition-colors line-clamp-2 leading-snug mb-2">
        {sub.title}
      </div>

      {/* Footer: Duration + Steps */}
      <div className="flex items-center justify-between text-[11px] font-mono text-[#71717a] dark:text-[#71717a] pt-1.5 border-t border-[#f4f4f5] dark:border-[#212126]/60">
        <div className="flex items-center space-x-2">
          <div className="flex items-center space-x-1">
            <Clock className="h-3 w-3 block shrink-0 text-[#a1a1aa] dark:text-[#52525b]" />
            <span className="tabular-nums">
              {formatDuration(sub.durationMs, sub.status === 'running')}
            </span>
          </div>
          <div className="flex items-center space-x-1">
            <Layers className="h-3 w-3 block shrink-0 text-[#a1a1aa] dark:text-[#52525b]" />
            <span className="tabular-nums">{formatSteps(sub.stepsCount || 0, language)}</span>
          </div>
        </div>
      </div>
    </div>
  );
}

import { ChevronDown, ChevronRight, Cpu, Plus } from 'lucide-react';
import AgentIcon from '../../common/AgentIcon';
import { getDefaultColumnIcon } from '../../../lib/columnIcons';
import { useTranslation } from '../../../stores/languageStore';
import {
  BACKLOG_COLOR,
  READY_COLOR,
  getDeterministicAgentColor,
} from '../../../lib/agentPalette';
import type { CortexAgentStage } from '../../../types';

export interface StageHeaderButtonProps {
  stage: CortexAgentStage;
  hint: string;
  overrideIcon?: string;
  overrideColor?: string;
  onToggle: () => void;
  onPlus?: () => void;
}

export function StageHeaderButton({
  stage,
  hint,
  overrideIcon,
  overrideColor,
  onToggle,
  onPlus,
}: StageHeaderButtonProps) {
  const { t } = useTranslation();
  const isBuiltin = stage.personaName === 'cvh-dev' || Boolean(stage.isBuiltin);
  const isBacklog = stage.stageId === 'backlog';
  const isReady = stage.stageId === 'ready';
  const iconName = overrideIcon || stage.icon || getDefaultColumnIcon(stage.stageId);
  const stageColor = isBacklog
    ? BACKLOG_COLOR
    : isReady
    ? READY_COLOR
    : (overrideColor || stage.color || (stage.personaName ? getDeterministicAgentColor(stage.personaName) : '#171717'));

  return (
    <div className="group flex items-center gap-0.5 px-1">
      <button
        onClick={onToggle}
        className="flex-1 flex items-center justify-between px-1.5 py-1.5 rounded-md text-left transition cursor-pointer min-w-0"
      >
        <div className="flex items-center space-x-2 min-w-0">
          {stage.isExpanded ? (
            <ChevronDown className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
          )}

          {iconName ? (
            <AgentIcon
              name={iconName}
              className={`h-3.5 w-3.5 shrink-0 ${isBuiltin ? 'text-indigo-600' : ''}`}
              style={!isBuiltin ? { color: stageColor } : undefined}
            />
          ) : isBuiltin ? (
            <Cpu className="h-3.5 w-3.5 text-indigo-600 shrink-0" />
          ) : (
            <span
              className="w-2 h-2 rounded-full shrink-0"
              style={{ backgroundColor: stageColor }}
            />
          )}

          <span className="font-sans text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] truncate">
            {stage.stageTitle}
          </span>

          {isBuiltin && (
            <span className="px-1.5 py-0.5 rounded text-[10px] font-sans font-medium bg-indigo-50 dark:bg-indigo-950/50 text-indigo-700 dark:text-indigo-400 border border-indigo-200 dark:border-indigo-800 shrink-0 self-center translate-y-[0.5px] leading-tight">
              {t('cortex.builtinBadge')}
            </span>
          )}
        </div>

        {hint !== '' && !isBuiltin && (
          <span className="text-[10px] font-mono text-[#666666] dark:text-[#8b949e] truncate ml-1">
            {hint}
          </span>
        )}
      </button>

      {onPlus && (
        <button
          onClick={(e) => {
            e.stopPropagation();
            onPlus();
          }}
          aria-label={t('cortex.newRule')}
          className="p-1 rounded-md text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#ebebeb]/60 dark:hover:bg-[#21262d] transition cursor-pointer opacity-0 group-hover:opacity-100 focus:opacity-100 focus-visible:opacity-100 shrink-0"
        >
          <Plus className="h-3.5 w-3.5" />
        </button>
      )}
    </div>
  );
}

export default StageHeaderButton;

import { Check, X, Layers, Zap } from 'lucide-react';
import AgentIcon from '../../../common/AgentIcon';

export function SubagentStatusBadge({ status, t }: { status: string; t: (key: any) => string }) {
  if (status === 'running') {
    return (
      <div className="flex items-center space-x-1">
        <span className="relative flex h-1.5 w-1.5 shrink-0">
          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
          <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-emerald-500" />
        </span>
        <span className="text-[10px] font-mono text-[#15803d] dark:text-[#7ee787]">
          {t('tools.status.running')}
        </span>
      </div>
    );
  }
  if (status === 'finished' || status === 'done' || status === 'success') {
    return (
      <div className="flex items-center space-x-1">
        <Check className="h-3 w-3 text-[#15803d] dark:text-[#7ee787] stroke-[2.5]" />
        <span className="text-[10px] font-mono font-medium text-[#15803d] dark:text-[#7ee787]">
          {t('tools.status.done')}
        </span>
      </div>
    );
  }
  if (status === 'stopped' || status === 'aborted' || status === 'paused') {
    return (
      <div className="flex items-center space-x-1">
        <X className="h-3 w-3 text-[#d97706] dark:text-[#f59e0b] stroke-[2.5]" />
        <span className="text-[10px] font-mono font-medium text-[#d97706] dark:text-[#f59e0b]">
          {t('tools.status.aborted')}
        </span>
      </div>
    );
  }
  return (
    <div className="flex items-center space-x-1">
      <X className="h-3 w-3 text-[#dc2626] dark:text-[#ff7b72] stroke-[2.5]" />
      <span className="text-[10px] font-mono font-medium text-[#dc2626] dark:text-[#ff7b72]">
        {t('tools.status.error')}
      </span>
    </div>
  );
}

export function SubagentTypeIcon({
  isBg,
  isBoardSubtask,
  personaIcon,
}: {
  isBg?: boolean;
  isBoardSubtask?: boolean;
  personaIcon?: string;
}) {
  if (isBg) {
    return (
      <div className="relative shrink-0 flex items-center justify-center">
        <Layers className="h-3.5 w-3.5 text-purple-600 dark:text-purple-400" />
        <span className="absolute -bottom-0.5 -right-0.5 h-1.5 w-1.5 rounded-full bg-purple-500 ring-1 ring-white dark:ring-[#111114]" />
      </div>
    );
  }
  if (isBoardSubtask) {
    return <AgentIcon name={personaIcon} className="h-3.5 w-3.5 text-[#171717] dark:text-[#f4f4f5] shrink-0" />;
  }
  return (
    <div className="relative shrink-0 flex items-center justify-center">
      <Zap className="h-3.5 w-3.5 text-sky-600 dark:text-sky-400 fill-sky-500/20" />
      <span className="absolute -bottom-0.5 -right-0.5 h-1.5 w-1.5 rounded-full bg-sky-500 ring-1 ring-white dark:ring-[#111114]" />
    </div>
  );
}

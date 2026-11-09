import { ArrowRightToLine, ArrowRightFromLine, Zap } from 'lucide-react';
import type { CortexAgentFile, CortexHookMoment } from '../../types';
import { hookMomentOf } from '../cortex/types';

export interface HookIconProps {
  moment?: CortexHookMoment | string | null;
  file?: CortexAgentFile | null;
  className?: string;
}

export function HookIcon({ moment, file, className }: HookIconProps) {
  const effectiveMoment = moment ?? (file ? hookMomentOf(file) : null);

  if (effectiveMoment === 'on_enter') {
    return (
      <ArrowRightToLine
        className={`text-emerald-600 dark:text-emerald-400 block shrink-0 ${className ?? 'h-3.5 w-3.5'}`}
        aria-label="on_enter hook"
      />
    );
  }

  if (effectiveMoment === 'on_leave' || effectiveMoment === 'on_finish' || effectiveMoment === 'on_exit') {
    return (
      <ArrowRightFromLine
        className={`text-amber-600 dark:text-amber-400 block shrink-0 ${className ?? 'h-3.5 w-3.5'}`}
        aria-label={`${effectiveMoment} hook`}
      />
    );
  }

  return (
    <Zap
      className={`text-[#666666] dark:text-[#8b949e] block shrink-0 ${className ?? 'h-3.5 w-3.5'}`}
      aria-label="hook"
    />
  );
}

export default HookIcon;

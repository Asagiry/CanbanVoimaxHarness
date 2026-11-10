import type { CortexAgentFile, CortexAgentStage, CortexHookMoment } from '../../types';

export interface ExpandAllSignal {
  expanded: boolean;
  nonce: number;
}

export function hookMomentOf(fileOrName: CortexAgentFile | string | null | undefined): CortexHookMoment | null {
  if (!fileOrName) return null;

  if (typeof fileOrName === 'object') {
    if (fileOrName.hookMoment) return fileOrName.hookMoment;

    const id = fileOrName.id || '';
    if (id.includes('-hook-on_enter')) return 'on_enter';
    if (id.includes('-hook-on_leave')) return 'on_leave';
    if (id.includes('-hook-on_finish')) return 'on_finish';
    if (id.includes('-hook-on_exit')) return 'on_leave';

    const resolvedFromPath = hookMomentOf(fileOrName.path || fileOrName.name);
    if (resolvedFromPath) return resolvedFromPath;

    return null;
  }

  const raw = fileOrName.trim();
  if (!raw) return null;

  const baseName = (raw.split(/[/\\]/).pop() || raw).toLowerCase();

  // Prefixes
  if (
    baseName.startsWith('on_enter') ||
    baseName.startsWith('on-enter') ||
    baseName.startsWith('enter_') ||
    baseName.startsWith('enter-')
  ) {
    return 'on_enter';
  }
  if (
    baseName.startsWith('on_leave') ||
    baseName.startsWith('on-leave') ||
    baseName.startsWith('leave_') ||
    baseName.startsWith('leave-')
  ) {
    return 'on_leave';
  }
  if (
    baseName.startsWith('on_finish') ||
    baseName.startsWith('on-finish') ||
    baseName.startsWith('finish_') ||
    baseName.startsWith('finish-')
  ) {
    return 'on_finish';
  }
  if (
    baseName.startsWith('on_exit') ||
    baseName.startsWith('on-exit') ||
    baseName.startsWith('exit_') ||
    baseName.startsWith('exit-')
  ) {
    return 'on_leave';
  }

  // Infix substrings
  if (
    baseName.includes('on_enter') ||
    baseName.includes('_enter') ||
    baseName.includes('-enter')
  ) {
    return 'on_enter';
  }
  if (
    baseName.includes('on_leave') ||
    baseName.includes('_leave') ||
    baseName.includes('-leave')
  ) {
    return 'on_leave';
  }
  if (
    baseName.includes('on_finish') ||
    baseName.includes('_finish') ||
    baseName.includes('-finish')
  ) {
    return 'on_finish';
  }
  if (
    baseName.includes('on_exit') ||
    baseName.includes('_exit') ||
    baseName.includes('-exit')
  ) {
    return 'on_leave';
  }

  return null;
}

export function isBacklogSystemStage(stage: CortexAgentStage): boolean {
  if (stage.stageId !== 'backlog') return false;
  const persona = (stage.personaName ?? '').trim();
  return persona === 'система' || persona.toLowerCase() === 'system';
}

export { savePipelineConfig } from '../../stores/cortex/cortexHelpers';

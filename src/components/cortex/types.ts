import type { CortexHookMoment } from '../../stores/cortexStore';
import type { CortexAgentStage } from '../../types';

export function hookMomentOf(fileName: string): CortexHookMoment | null {
  if (fileName.startsWith('on_enter')) return 'on_enter';
  if (fileName.startsWith('on_leave')) return 'on_leave';
  if (fileName.startsWith('on_finish')) return 'on_finish';
  return null;
}

export function isBacklogSystemStage(stage: CortexAgentStage): boolean {
  if (stage.stageId !== 'backlog') return false;
  const persona = (stage.personaName ?? '').trim();
  return persona === 'система' || persona.toLowerCase() === 'system';
}

export { savePipelineConfig } from '../../stores/cortex/cortexHelpers';

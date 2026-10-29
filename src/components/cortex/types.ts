import type { CortexHookMoment } from '../../stores/cortexStore';
import type { PipelineColumn, PipelineConfig, CortexAgentStage } from '../../types';

export interface SharedFolderState {
  openRulesFolders: Record<string, boolean>;
  openHooksFolders: Record<string, boolean>;
  creatingStageId: string | null;
  creatingKind: 'rule' | 'hook';
}

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

export async function savePipelineConfig(next: PipelineColumn[]): Promise<boolean> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    let cfg: PipelineConfig | null = null;
    try {
      cfg = await invoke<PipelineConfig>('get_pipeline');
    } catch {
      cfg = null;
    }
    const pipeline: PipelineConfig = {
      id: cfg?.id ?? 'default-engineering',
      title: cfg?.title ?? 'Инженерный пайплайн',
      columns: next,
    };
    await invoke('save_pipeline', { pipeline });
    return true;
  } catch (err) {
    console.info('save_pipeline недоступен, изменения колонок откачены:', err);
    return false;
  }
}

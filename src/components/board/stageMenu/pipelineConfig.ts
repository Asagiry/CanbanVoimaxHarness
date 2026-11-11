import type { PipelineStage, PipelineConfig } from '../../../types';
import { useBoardStore } from '../../../stores/boardStore';

export const PROTECTED_STAGE_IDS = new Set(['backlog', 'ready']);
export const MENU_WIDTH = 264;
export const VIEWPORT_MARGIN = 8;

export function isStageProtected(stage: PipelineStage): boolean {
  return PROTECTED_STAGE_IDS.has(stage.id) || Boolean(stage.isIntake) || Boolean(stage.terminal);
}

export function canStageChangeAgent(stage: PipelineStage): boolean {
  return stage.id !== 'backlog' && stage.id !== 'ready' && !stage.isIntake && !stage.terminal;
}

export async function savePipelineConfig(next: PipelineStage[]): Promise<boolean> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    let cfg: PipelineConfig | null = null;
    try {
      cfg = await invoke<PipelineConfig>('get_pipeline');
    } catch (err) {
      console.debug('[pipelineConfig] get_pipeline unavailable, using fallback:', err);
      cfg = null;
    }
    const pipeline = {
      id: cfg?.id ?? 'default-engineering',
      title: cfg?.title ?? 'Инженерный пайплайн',
      stages: next,
    };
    await useBoardStore.getState().saveCurrentBoard(next);
    await invoke('save_pipeline', { pipeline });
    return true;
  } catch (err) {
    console.info('save_pipeline недоступен, изменения стадий откачены:', err);
    return false;
  }
}

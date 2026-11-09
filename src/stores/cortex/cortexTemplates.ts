import { CortexHookExt, CortexHookMoment, CortexState } from './types';
import { CortexAgentFile, CortexAgentStage } from '../../types';
import {
  buildGlobalRuleTemplate,
  buildHookFileName,
  buildHookTemplate,
  buildPersonaTemplate,
  buildRuleTemplate,
  buildScriptTemplate,
  buildSettingTemplate,
  compareGlobalRules,
  cortexAgentDir,
  normalizeCortexFileName,
  savePipelineConfig,
} from './cortexHelpers';
import { useBoardStore } from '../boardStore';

export type CortexSet = (
  partial: Partial<CortexState> | ((state: CortexState) => Partial<CortexState>),
) => void;
export type CortexGet = () => CortexState;

export function replaceFileInStageList(
  stages: CortexAgentStage[],
  stageId: string,
  fileId: string,
  loaded: CortexAgentFile,
): CortexAgentStage[] {
  return stages.map((st) =>
    st.stageId === stageId
      ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
      : st,
  );
}

export function clearFileMissing(
  fileMissing: Record<string, boolean>,
  fileId: string,
): Record<string, boolean> {
  if (!fileMissing[fileId]) return fileMissing;
  const next = { ...fileMissing };
  delete next[fileId];
  return next;
}

async function applyStageTemplateAction(
  set: CortexSet,
  get: CortexGet,
  fileId: string,
  buildTemplate: (stage: CortexAgentStage) => string,
  reloadPersonas = false,
): Promise<void> {
  const { stages, agentStages } = get();
  for (const stage of [...stages, ...agentStages]) {
    const file = stage.files.find((f) => f.id === fileId);
    if (!file) continue;

    const template = buildTemplate(stage);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_cortex_file', { path: file.path, content: template });
    } catch (err) {
      console.info('save_cortex_file not available or in web mode:', err);
    }
    const loaded: CortexAgentFile = { ...file, content: template, isDirty: false };
    set((state) => ({
      stages: replaceFileInStageList(state.stages, stage.stageId, fileId, loaded),
      agentStages: replaceFileInStageList(state.agentStages, stage.stageId, fileId, loaded),
      activeFile: loaded,
      isEditorOpen: true,
      saveFeedback: null,
      fileMissing: clearFileMissing(state.fileMissing, fileId),
    }));
    if (reloadPersonas) await get().loadAgentPersonas();
    return;
  }
}

export function createPersonaTemplateAction(
  set: CortexSet,
  get: CortexGet,
  fileId: string,
): Promise<void> {
  return applyStageTemplateAction(
    set,
    get,
    fileId,
    (stage) => buildPersonaTemplate(stage.personaName, stage.stageTitle),
    true,
  );
}

export function createSettingTemplateAction(
  set: CortexSet,
  get: CortexGet,
  fileId: string,
): Promise<void> {
  return applyStageTemplateAction(set, get, fileId, (stage) => {
    const role = (stage.personaName ?? '').trim() || stage.stageId;
    return buildSettingTemplate(role, stage.stageTitle);
  });
}

export async function createCortexFileAction(
  set: CortexSet,
  get: CortexGet,
  stageId: string,
  rawName: string,
  kind: 'rule' | 'hook',
  opts?: { hookMoment?: CortexHookMoment; hookExt?: CortexHookExt },
): Promise<void> {
  const { stages, agentStages } = get();
  const stage =
    stages.find((st) => st.stageId === stageId) ??
    agentStages.find((st) => st.stageId === stageId);
  if (!stage) throw new Error('Стадия не найдена');

  const isColumnStage =
    !stageId.startsWith('agent:') && stages.some((st) => st.stageId === stageId);
  const role = (stage.personaName ?? '').trim() || stageId;
  const dir = cortexAgentDir(stage.personaName, stageId);

  let targetPath: string;
  let content: string;
  let fileName: string;
  let fileType: CortexAgentFile['type'];

  if (kind === 'rule') {
    const { base, fileName: ruleName } = normalizeCortexFileName(rawName);
    if (!base) throw new Error('Введите имя файла');
    if (!/^[A-Za-z0-9-_]+$/.test(base)) {
      throw new Error('Только латиница, цифры, - и _');
    }
    fileName = ruleName;
    fileType = 'rule';
    targetPath = `${dir}/rules/${fileName}`;
    content = buildRuleTemplate(base, stage.stageTitle);
  } else {
    const moment: CortexHookMoment = opts?.hookMoment ?? 'on_enter';
    const ext: CortexHookExt = opts?.hookExt ?? 'bat';
    if (moment !== 'on_enter' && moment !== 'on_leave' && moment !== 'on_finish') {
      throw new Error('Момент хука: on_enter или on_leave');
    }
    if (ext !== 'bat' && ext !== 'ps1' && ext !== 'py' && ext !== 'sh') {
      throw new Error('Расширение хука: .bat, .ps1, .py или .sh');
    }
    const typed = rawName.trim().replace(/\.[A-Za-z0-9]+$/, '').trim();
    if (typed && !/^[A-Za-z0-9-_]+$/.test(typed)) {
      throw new Error('Только латиница, цифры, - и _');
    }
    fileName = buildHookFileName(rawName, moment, ext, isColumnStage ? stageId : undefined);
    fileType = 'hook';
    if (isColumnStage) {
      targetPath = `.cvh/hooks/${fileName}`;
      content = buildScriptTemplate(targetPath, moment, stageId);
    } else {
      targetPath = `${dir}/hooks/${fileName}`;
      content = buildHookTemplate(moment, ext, typed || moment, role, stage.stageTitle, false);
    }
  }

  const duplicate = stage.files.some(
    (f) =>
      f.name.toLowerCase() === fileName.toLowerCase() ||
      f.path.toLowerCase() === targetPath.toLowerCase(),
  );
  if (duplicate) throw new Error(`Файл ${fileName} уже есть в стадии`);

  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('save_cortex_file', { path: targetPath, content });
  } catch (err) {
    const detail = err instanceof Error ? err.message : String(err);
    throw new Error(`Не удалось сохранить файл: ${detail}`);
  }

  const idBase = `${stageId}-${kind}-${fileName}`;
  let fileId = idBase;
  let suffix = 2;
  const existingIds = new Set(stage.files.map((f) => f.id));
  while (existingIds.has(fileId)) {
    fileId = `${idBase}-${suffix}`;
    suffix += 1;
  }

  const createdId =
    isColumnStage && fileType === 'hook'
      ? `${stageId}-hook-${opts?.hookMoment ?? 'on_enter'}`
      : fileId;

  const created: CortexAgentFile = {
    id: createdId,
    name: fileName,
    stageId,
    type: fileType,
    hookMoment: fileType === 'hook' ? (opts?.hookMoment ?? 'on_enter') : undefined,
    path: targetPath,
    content,
    isDirty: false,
  };

  set((state) => ({
    stages: state.stages.map((st) =>
      st.stageId === stageId ? { ...st, files: [...st.files, created] } : st,
    ),
    agentStages: state.agentStages.map((st) =>
      st.stageId === stageId ? { ...st, files: [...st.files, created] } : st,
    ),
    activeFile: created,
    isEditorOpen: true,
    saveFeedback: null,
  }));

  if (isColumnStage && fileType === 'hook') {
    const moment: CortexHookMoment = opts?.hookMoment ?? 'on_enter';
    const currentColumns = useBoardStore.getState().columns;
    const nextColumns = currentColumns.map((col) => {
      if (col.id !== stageId) return col;
      return {
        ...col,
        hooks: {
          ...col.hooks,
          [moment]: {
            script: targetPath,
            description: `Хук ${moment} для стадии ${stage.stageTitle}`,
          },
        },
      };
    });

    useBoardStore.getState().setColumns(nextColumns);
    await savePipelineConfig(nextColumns);
    get().syncStagesFromColumns(nextColumns);
  }
}

export async function createGlobalRuleAction(
  set: CortexSet,
  get: CortexGet,
  rawName: string,
  folder?: string,
): Promise<void> {
  const { globalRules } = get();
  const { base, fileName } = normalizeCortexFileName(rawName);
  if (!base) throw new Error('Введите имя файла');
  if (!/^[A-Za-z0-9-_]+$/.test(base)) {
    throw new Error('Только латиница, цифры, - и _');
  }
  const cleanFolder = folder?.trim() || undefined;
  if (cleanFolder && !/^[A-Za-z0-9-_]+$/.test(cleanFolder)) {
    throw new Error('Только латиница, цифры, - и _');
  }

  const targetPath = cleanFolder ? `.cvh/rules/${cleanFolder}/${fileName}` : `.cvh/rules/${fileName}`;
  const content = buildGlobalRuleTemplate(base);

  const duplicate = globalRules.some(
    (f) =>
      f.name.toLowerCase() === fileName.toLowerCase() ||
      f.path.toLowerCase() === targetPath.toLowerCase(),
  );
  if (duplicate) throw new Error(`Файл ${fileName} уже есть в глобальных правилах`);

  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('save_cortex_file', { path: targetPath, content });
  } catch (err) {
    const detail = err instanceof Error ? err.message : String(err);
    throw new Error(`Не удалось сохранить файл: ${detail}`);
  }

  const created: CortexAgentFile = {
    id: cleanFolder ? `global-rule-${cleanFolder}/${fileName}` : `global-rule-${fileName}`,
    name: fileName,
    stageId: 'global',
    type: 'rule',
    path: targetPath,
    content,
    isDirty: false,
    folder: cleanFolder,
  };

  set((state) => ({
    globalRules: [...state.globalRules, created].sort(compareGlobalRules),
    activeFile: created,
    isEditorOpen: true,
    saveFeedback: null,
  }));
}

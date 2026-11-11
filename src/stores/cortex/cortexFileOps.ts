import { CortexAgentFile } from '../../types';
import {
  buildBuiltinHookDescription,
  buildCommandHookDescription,
  buildScriptTemplate,
  compareGlobalRules,
  extractScriptPath,
} from './cortexHelpers';
import {
  CortexGet,
  CortexSet,
  clearFileMissing,
  replaceFileInStageList,
} from './cortexTemplates';

export async function loadGlobalRulesAction(set: CortexSet, get: CortexGet): Promise<void> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const files = await invoke<Array<{ name: string; path: string; fileType: string; folder?: string }>>(
      'list_global_rules',
    );
    const prevGlobalRules = get().globalRules;
    const prevMap = new Map(prevGlobalRules.map((f) => [f.path, f]));
    const mapped: CortexAgentFile[] = (files ?? []).map((file) => {
      const prev = prevMap.get(file.path);
      const folder = file.folder || undefined;
      const isFolder = file.fileType === 'folder';
      return {
        id: isFolder
          ? `global-folder-${folder}`
          : folder
            ? `global-rule-${folder}/${file.name}`
            : `global-rule-${file.name}`,
        name: file.name,
        stageId: 'global',
        type: (isFolder ? 'folder' : 'rule') as CortexAgentFile['type'],
        path: file.path,
        content: prev?.content ?? '',
        isDirty: prev?.isDirty ?? false,
        isDeleted: prev?.isDeleted ?? false,
        folder,
      };
    });
    const tombstoned = prevGlobalRules.filter((f) => f.isDeleted && !mapped.some((m) => m.id === f.id));
    mapped.push(...tombstoned);
    set({ globalRules: mapped.sort(compareGlobalRules) });
  } catch (err) {
    console.info('loadGlobalRules not available or in web mode:', err);
  }
}

export async function openFileAction(
  set: CortexSet,
  get: CortexGet,
  fileId: string,
): Promise<void> {
  const { globalRules } = get();
  const globalFile = globalRules.find((f) => f.id === fileId);
  if (globalFile) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const content = await invoke<string>('read_cortex_file', { path: globalFile.path });
      const loaded: CortexAgentFile = { ...globalFile, content, isDirty: false };
      set((state) => ({
        globalRules: state.globalRules.map((f) => (f.id === fileId ? loaded : f)),
        activeFile: loaded,
        isEditorOpen: true,
        saveFeedback: null,
      }));
    } catch (err) {
      console.info('read_cortex_file failed for global rule:', err);
      if (globalFile.content) {
        set({ activeFile: { ...globalFile }, isEditorOpen: true, saveFeedback: null });
      }
    }
    return;
  }

  const { stages, agentStages } = get();
  for (const stage of [...stages, ...agentStages]) {
    const file = stage.files.find((f) => f.id === fileId);
    if (!file) continue;

    const applyLoadedFile = (loaded: CortexAgentFile, clearMissing = false) => {
      set((state) => ({
        stages: replaceFileInStageList(state.stages, stage.stageId, fileId, loaded),
        agentStages: replaceFileInStageList(state.agentStages, stage.stageId, fileId, loaded),
        activeFile: loaded,
        isEditorOpen: true,
        saveFeedback: null,
        fileMissing: clearMissing
          ? clearFileMissing(state.fileMissing, fileId)
          : state.fileMissing,
      }));
    };

    if (file.path.startsWith('builtin:')) {
      const content =
        file.content && file.content.trim().length > 0
          ? file.content
          : buildBuiltinHookDescription(file.path, file.name, stage.stageTitle);
      applyLoadedFile({ ...file, content, isDirty: false });
      return;
    }

    if (file.type === 'hook') {
      const scriptPath = extractScriptPath(file.path);
      if (scriptPath) {
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          const content = await invoke<string>('read_cortex_file', { path: scriptPath });
          applyLoadedFile({ ...file, content, isDirty: false }, true);
          return;
        } catch (err) {
          console.info('read_cortex_file for hook script failed, using template/memory:', err);
          const content =
            file.content && file.content.trim().length > 0
              ? file.content
              : buildScriptTemplate(scriptPath, file.name, stage.stageId);
          applyLoadedFile({ ...file, content, isDirty: false });
          return;
        }
      }

      const content =
        file.content && file.content.trim().length > 0
          ? file.content
          : buildCommandHookDescription(file.path, file.name, stage.stageTitle);
      applyLoadedFile({ ...file, content, isDirty: false });
      return;
    }

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const content = await invoke<string>('read_cortex_file', { path: file.path });
      applyLoadedFile({ ...file, content, isDirty: false }, true);
    } catch (err) {
      console.info('read_cortex_file not available or file is missing:', err);
      if (file.content) {
        set({ activeFile: { ...file }, isEditorOpen: true, saveFeedback: null });
        return;
      }
      set((state) => ({
        fileMissing: { ...state.fileMissing, [fileId]: true },
      }));
    }
    return;
  }
}

export async function saveRulesAction(set: CortexSet, get: CortexGet): Promise<void> {
  const { activeFile, stages, agentStages, globalRules } = get();
  if (!activeFile) return;

  set({ isSaving: true });

  const isBuiltin = activeFile.path.startsWith('builtin:');
  const scriptPath = extractScriptPath(activeFile.path);
  const isDirectCommand = activeFile.type === 'hook' && !isBuiltin && !scriptPath;

  // File Save is disk-only: it must NEVER git-commit/push. Only saveCvhToGitAction
  // (cvh_git_save) may commit/push. Builtin/direct-command hooks have no disk
  // target, so they are in-memory only — feedback below makes that explicit
  // instead of silently no-op'ing.
  if (!isBuiltin && !isDirectCommand) {
    const targetPath = scriptPath ?? activeFile.path;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_cortex_file', { path: targetPath, content: activeFile.content });
    } catch (err) {
      const detail = err instanceof Error ? err.message : String(err);
      console.error('save_cortex_file failed:', err);
      set({
        isSaving: false,
        saveFeedback: null,
        cvhGitError: `Не удалось сохранить ${activeFile.name}: ${detail}`,
      });
      return;
    }
  }

  const savedFile: CortexAgentFile = { ...activeFile, isDirty: false };
  const updatedStages = replaceFileInStageList(stages, activeFile.stageId, activeFile.id, savedFile);
  const updatedAgentStages = replaceFileInStageList(
    agentStages,
    activeFile.stageId,
    activeFile.id,
    savedFile,
  );
  const updatedGlobalRules = globalRules.map((file) =>
    file.id === activeFile.id ? savedFile : file,
  );

  const feedbackMsg = isBuiltin
    ? `Встроенный хук ${activeFile.name} обновлен (только в памяти, на диск не сохранен)`
    : isDirectCommand
      ? `Команда ${activeFile.name} обновлена (только в памяти, на диск не сохранена)`
      : `Файл ${activeFile.name} сохранен на диск (не в git)`;

  set({
    stages: updatedStages,
    agentStages: updatedAgentStages,
    globalRules: updatedGlobalRules,
    activeFile: savedFile,
    isSaving: false,
    saveFeedback: feedbackMsg,
    cvhGitError: null,
  });

  setTimeout(() => {
    set({ saveFeedback: null });
  }, 2800);

  void get().refreshCvhGitStatus();
}

export async function deleteGlobalRuleAction(
  set: CortexSet,
  get: CortexGet,
  fileName: string,
): Promise<void> {
  const { globalRules, activeFile } = get();
  const target = globalRules.find((f) => f.name === fileName || f.id === fileName);
  if (!target) return;

  const isActiveDeleted = activeFile?.id === target.id;
  const key = `file:${target.id}`;
  set((state) => ({
    globalRules: state.globalRules.map((f) => (f.id === target.id ? { ...f, isDeleted: true } : f)),
    pendingDeletes: { ...state.pendingDeletes, [key]: { path: target.path, kind: 'globalRule' as const, stageId: 'global', fileId: target.id } },
    activeFile: isActiveDeleted ? null : state.activeFile,
    isEditorOpen: isActiveDeleted ? false : state.isEditorOpen,
  }));
}

export function reorderFilesInStage(
  files: CortexAgentFile[],
  ruleNames: string[],
): CortexAgentFile[] {
  const rules = files.filter((f) => f.type === 'rule');
  const nonRules = files.filter((f) => f.type !== 'rule');

  rules.sort((a, b) => {
    const idxA = ruleNames.indexOf(a.name);
    const idxB = ruleNames.indexOf(b.name);
    if (idxA !== -1 && idxB !== -1) return idxA - idxB;
    if (idxA !== -1) return -1;
    if (idxB !== -1) return 1;
    return a.name.localeCompare(b.name);
  });

  const personaFiles = nonRules.filter((f) => f.type === 'persona');
  const hookFiles = nonRules.filter((f) => f.type === 'hook');
  const settingFiles = nonRules.filter((f) => f.type === 'settings');
  const otherFiles = nonRules.filter(
    (f) => f.type !== 'persona' && f.type !== 'hook' && f.type !== 'settings',
  );

  return [...personaFiles, ...hookFiles, ...rules, ...otherFiles, ...settingFiles];
}

export async function createGlobalFolderAction(
  _set: CortexSet,
  get: CortexGet,
  folderName: string,
): Promise<void> {
  const trimmed = folderName.trim();
  if (!trimmed) throw new Error('Введите имя папки');
  if (!/^[A-Za-z0-9-_]+$/.test(trimmed)) {
    throw new Error('Только латиница, цифры, - и _');
  }
  const { globalRules } = get();
  if (globalRules.some((r) => r.folder?.toLowerCase() === trimmed.toLowerCase())) {
    throw new Error(`Папка ${trimmed} уже существует`);
  }

  const targetPath = `.cvh/rules/${trimmed}/.gitkeep`;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('save_cortex_file', { path: targetPath, content: '' });
  await get().loadGlobalRules();
}

export async function deleteGlobalFolderAction(
  set: CortexSet,
  _get: CortexGet,
  folderName: string,
): Promise<void> {
  const trimmed = folderName.trim();
  const folderPath = `.cvh/rules/${trimmed}`;
  const key = `folder:${trimmed}`;
  set((state) => ({
    globalRules: state.globalRules.filter((f) => f.folder !== trimmed),
    pendingDeletes: {
      ...state.pendingDeletes,
      [key]: { path: folderPath, kind: 'globalRule' as const, stageId: 'global', fileId: key },
    },
  }));
}

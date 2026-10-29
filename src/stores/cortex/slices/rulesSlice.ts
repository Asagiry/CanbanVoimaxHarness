import { StateCreator } from 'zustand';
import {
  CortexHookExt,
  CortexHookMoment,
  CortexState,
  RulesSlice,
} from '../types';
import { CortexAgentFile } from '../../../types';
import {
  buildGlobalRuleTemplate,
  buildHookTemplate,
  buildRuleTemplate,
  cortexAgentDir,
  extractScriptPath,
  normalizeCortexFileName,
} from '../cortexHelpers';

export const createRulesSlice: StateCreator<
  CortexState,
  [],
  [],
  RulesSlice
> = (set, get) => ({
  createCortexFile: async (stageId, rawName, kind, opts) => {
    const { stages, agentStages } = get();
    // Создание работает и из секции колонок, и из секции агентов.
    const stage =
      stages.find((st) => st.stageId === stageId) ??
      agentStages.find((st) => st.stageId === stageId);
    if (!stage) throw new Error('Стадия не найдена');

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
      // Хук: имя файла кодирует момент срабатывания.
      const moment: CortexHookMoment = opts?.hookMoment ?? 'on_enter';
      const ext: CortexHookExt = opts?.hookExt ?? 'bat';
      if (moment !== 'on_enter' && moment !== 'on_leave' && moment !== 'on_finish') {
        throw new Error('Момент хука: on_enter или on_leave');
      }
      if (ext !== 'bat' && ext !== 'ps1' && ext !== 'py' && ext !== 'sh') {
        throw new Error('Расширение хука: .bat, .ps1, .py или .sh');
      }
      // Пользователь мог вписать расширение вручную — срезать, чтобы не дублировалось.
      const typed = rawName.trim().replace(/\.[A-Za-z0-9]+$/, '').trim();
      if (typed && !/^[A-Za-z0-9-_]+$/.test(typed)) {
        throw new Error('Только латиница, цифры, - и _');
      }
      fileName = typed ? `${moment}-${typed}.${ext}` : `${moment}.${ext}`;
      fileType = 'hook';
      targetPath = `${dir}/hooks/${fileName}`;
      content = buildHookTemplate(moment, ext, typed || moment, role, stage.stageTitle);
    }

    const duplicate = stage.files.some(
      (f) => f.name.toLowerCase() === fileName.toLowerCase() || f.path.toLowerCase() === targetPath.toLowerCase()
    );
    if (duplicate) throw new Error(`Файл ${fileName} уже есть в стадии`);

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_cortex_file', { path: targetPath, content });
    } catch (err) {
      const detail = err instanceof Error ? err.message : String(err);
      throw new Error(`Не удалось сохранить файл: ${detail}`);
    }

    // Уникальный id узла в пределах стадии
    const idBase = `${stageId}-${kind}-${fileName}`;
    let fileId = idBase;
    let suffix = 2;
    const existingIds = new Set(stage.files.map((f) => f.id));
    while (existingIds.has(fileId)) {
      fileId = `${idBase}-${suffix}`;
      suffix += 1;
    }

    const created: CortexAgentFile = {
      id: fileId,
      name: fileName,
      stageId,
      type: fileType,
      path: targetPath,
      content,
      isDirty: false,
    };

    // Узел появляется в дереве стадии и открывается в редакторе (существующий флоу)
    set((state) => ({
      stages: state.stages.map((st) =>
        st.stageId === stageId ? { ...st, files: [...st.files, created] } : st
      ),
      agentStages: state.agentStages.map((st) =>
        st.stageId === stageId ? { ...st, files: [...st.files, created] } : st
      ),
      activeFile: created,
      isEditorOpen: true,
      saveFeedback: null,
    }));
  },

  createGlobalRule: async (rawName: string) => {
    const { globalRules } = get();
    const { base, fileName } = normalizeCortexFileName(rawName);
    if (!base) throw new Error('Введите имя файла');
    if (!/^[A-Za-z0-9-_]+$/.test(base)) {
      throw new Error('Только латиница, цифры, - и _');
    }

    const targetPath = `.cvh/rules/${fileName}`;
    const content = buildGlobalRuleTemplate(base);

    const duplicate = globalRules.some(
      (f) =>
        f.name.toLowerCase() === fileName.toLowerCase() ||
        f.path.toLowerCase() === targetPath.toLowerCase()
    );
    if (duplicate) throw new Error(`Файл ${fileName} уже есть в глобальных правилах`);

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_cortex_file', { path: targetPath, content });
    } catch (err) {
      const detail = err instanceof Error ? err.message : String(err);
      throw new Error(`Не удалось сохранить файл: ${detail}`);
    }

    const fileId = `global-rule-${fileName}`;
    const created: CortexAgentFile = {
      id: fileId,
      name: fileName,
      stageId: 'global',
      type: 'rule',
      path: targetPath,
      content,
      isDirty: false,
    };

    set((state) => ({
      globalRules: [...state.globalRules, created].sort((a, b) => a.name.localeCompare(b.name)),
      activeFile: created,
      isEditorOpen: true,
      saveFeedback: null,
    }));
  },

  updateActiveFileContent: (content) => {
    set((state) => {
      if (!state.activeFile) return state;
      return {
        activeFile: {
          ...state.activeFile,
          content,
          isDirty: true,
        },
      };
    });
  },

  saveRules: async () => {
    const { activeFile, stages, agentStages, globalRules } = get();
    if (!activeFile) return;

    set({ isSaving: true });

    const isBuiltin = activeFile.path.startsWith('builtin:');
    const scriptPath = extractScriptPath(activeFile.path);
    const isDirectCommand = activeFile.type === 'hook' && !isBuiltin && !scriptPath;

    if (!isBuiltin && !isDirectCommand) {
      const targetPath = scriptPath ?? activeFile.path;
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('save_cortex_file', { path: targetPath, content: activeFile.content });
      } catch (err) {
        console.info('save_cortex_file not available or in web mode:', err);
      }
    }

    // Update in memory stages (обе секции: файл мог быть открыт из любой)
    const updatedStages = stages.map((stage) => {
      if (stage.stageId !== activeFile.stageId) return stage;
      return {
        ...stage,
        files: stage.files.map((file) =>
          file.id === activeFile.id
            ? { ...file, content: activeFile.content, isDirty: false }
            : file
        ),
      };
    });
    const updatedAgentStages = agentStages.map((stage) => {
      if (stage.stageId !== activeFile.stageId) return stage;
      return {
        ...stage,
        files: stage.files.map((file) =>
          file.id === activeFile.id
            ? { ...file, content: activeFile.content, isDirty: false }
            : file
        ),
      };
    });
    const updatedGlobalRules = globalRules.map((file) =>
      file.id === activeFile.id
        ? { ...file, content: activeFile.content, isDirty: false }
        : file
    );

    const feedbackMsg = isBuiltin
      ? `Встроенный хук ${activeFile.name} обновлен`
      : isDirectCommand
      ? `Команда ${activeFile.name} обновлена`
      : `Файл ${activeFile.name} сохранен`;

    set({
      stages: updatedStages,
      agentStages: updatedAgentStages,
      globalRules: updatedGlobalRules,
      activeFile: { ...activeFile, isDirty: false },
      isSaving: false,
      saveFeedback: feedbackMsg,
    });

    setTimeout(() => {
      set({ saveFeedback: null });
    }, 2800);
  },

  deleteRule: async (stageId: string, ruleName: string) => {
    if (stageId === 'global') {
      return get().deleteGlobalRule(ruleName);
    }

    set((state) => {
      const filterFiles = (files: CortexAgentFile[]) =>
        files.filter((f) => !(f.type === 'rule' && (f.name === ruleName || f.id === ruleName)));

      const nextStages = state.stages.map((st) =>
        st.stageId === stageId ? { ...st, files: filterFiles(st.files) } : st
      );
      const nextAgentStages = state.agentStages.map((st) =>
        st.stageId === stageId ? { ...st, files: filterFiles(st.files) } : st
      );

      const isActiveDeleted =
        state.activeFile?.stageId === stageId &&
        state.activeFile?.type === 'rule' &&
        (state.activeFile.name === ruleName || state.activeFile.id === ruleName);

      return {
        stages: nextStages,
        agentStages: nextAgentStages,
        activeFile: isActiveDeleted ? null : state.activeFile,
        isEditorOpen: isActiveDeleted ? false : state.isEditorOpen,
      };
    });
  },

  deleteGlobalRule: async (fileName: string) => {
    const { globalRules, activeFile } = get();
    const target = globalRules.find((f) => f.name === fileName || f.id === fileName);
    if (!target) return;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('delete_cortex_file', { path: target.path });
    } catch (err) {
      const detail = err instanceof Error ? err.message : String(err);
      throw new Error(`Не удалось удалить файл: ${detail}`);
    }

    const isActiveDeleted = activeFile?.id === target.id;
    set((state) => ({
      globalRules: state.globalRules.filter((f) => f.id !== target.id),
      activeFile: isActiveDeleted ? null : state.activeFile,
      isEditorOpen: isActiveDeleted ? false : state.isEditorOpen,
    }));
  },

  reorderRules: (stageId: string, ruleNames: string[]) => {
    set((state) => {
      const reorderList = (files: CortexAgentFile[]) => {
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

        // Maintain canonical layout: persona at top, hooks, rules, settings at bottom
        const personaFiles = nonRules.filter((f) => f.type === 'persona');
        const hookFiles = nonRules.filter((f) => f.type === 'hook');
        const settingFiles = nonRules.filter((f) => f.type === 'settings');
        const otherFiles = nonRules.filter(
          (f) => f.type !== 'persona' && f.type !== 'hook' && f.type !== 'settings'
        );

        return [...personaFiles, ...hookFiles, ...rules, ...otherFiles, ...settingFiles];
      };

      return {
        stages: state.stages.map((st) =>
          st.stageId === stageId ? { ...st, files: reorderList(st.files) } : st
        ),
        agentStages: state.agentStages.map((st) =>
          st.stageId === stageId ? { ...st, files: reorderList(st.files) } : st
        ),
      };
    });
  },
});

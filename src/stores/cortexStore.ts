import { create } from 'zustand';
import { CortexAgentFile, CortexAgentStage, PipelineColumn } from '../types';
import { INITIAL_CORTEX_STAGES } from './mockData';

interface CortexState {
  stages: CortexAgentStage[];
  activeFile: CortexAgentFile | null;
  focusedStageId: string | null;
  isEditorOpen: boolean;
  isSaving: boolean;
  saveFeedback: string | null;
  // Файлы персон, отсутствующие на диске (read_cortex_file завершился ошибкой), по id файла
  personaMissing: Record<string, boolean>;

  // Actions
  toggleStageExpand: (stageId: string) => void;
  focusStage: (stageId: string) => void;
  // Перестроить стадии из колонок доски: колонка с persona = агент
  syncStagesFromColumns: (columns: PipelineColumn[]) => void;
  openFile: (fileId: string) => Promise<void>;
  createPersonaTemplate: (fileId: string) => Promise<void>;
  // Создать новый .md-файл в стадии: kind определяет папку и шаблон.
  // Правила -> `.cvh/rules/<stageId|common>/`, персоны -> `.cvh/agents/`.
  // При ошибке IPC бросает Error с русским текстом (форма показывает его красным).
  createCortexFile: (stageId: string, rawName: string, kind: 'rule' | 'persona') => Promise<void>;
  updateActiveFileContent: (content: string) => void;
  saveRules: () => Promise<void>;
  closeEditor: () => void;
}

// Минимальный русский шаблон правила для `.cvh/rules/<stage>/`
function buildRuleTemplate(baseName: string, stageTitle: string): string {
  return (
    `# Правило: ${baseName} (${stageTitle})\n` +
    `\n` +
    `## Назначение\n` +
    `Опишите назначение правила.\n` +
    `\n` +
    `## Проверки\n` +
    `- Проверять результат реальными сенсорами перед сдачей.\n`
  );
}

// Нормализация имени: trim + автосуффикс .md. Возвращает { base, fileName }.
function normalizeCortexFileName(rawName: string): { base: string; fileName: string } {
  const trimmed = rawName.trim();
  const withoutExt = /\.md$/i.test(trimmed) ? trimmed.slice(0, -3) : trimmed;
  const base = withoutExt.trim();
  return { base, fileName: `${base}.md` };
}

// Шаблон персоны на русском для отсутствующего .cvh/agents/<persona>.md
function buildPersonaTemplate(personaName: string, stageTitle: string): string {
  return (
    `# Роль: ${personaName} (${stageTitle})\n` +
    `\n` +
    `## Назначение\n` +
    `Опишите здесь обязанности и границы ответственности агента.\n` +
    `\n` +
    `## Инварианты\n` +
    `- Работать только в рамках своей колонки пайплайна.\n` +
    `- Не использовать заглушки и синтетические моки без необходимости.\n` +
    `- Проверять результат реальными сенсорами перед сдачей.\n` +
    `\n` +
    `## Сенсоры\n` +
    `- npm run build\n`
  );
}

export const useCortexStore = create<CortexState>((set, get) => ({
  stages: INITIAL_CORTEX_STAGES,
  activeFile: null,
  focusedStageId: null,
  isEditorOpen: false,
  isSaving: false,
  saveFeedback: null,
  personaMissing: {},

  toggleStageExpand: (stageId) => {
    set((state) => ({
      stages: state.stages.map((st) =>
        st.stageId === stageId ? { ...st, isExpanded: !st.isExpanded } : st
      ),
    }));
  },

  focusStage: (stageId) => {
    set((state) => ({
      focusedStageId: stageId,
      stages: state.stages.map((st) =>
        st.stageId === stageId ? { ...st, isExpanded: true } : st
      ),
    }));

    // Reset focused highlight after 2.5s
    setTimeout(() => {
      if (get().focusedStageId === stageId) {
        set({ focusedStageId: null });
      }
    }, 2500);
  },

  syncStagesFromColumns: (columns) => {
    const seenPersonas = new Set<string>();
    const prevStages = get().stages;
    const prevStageById = new Map(prevStages.map((st) => [st.stageId, st]));
    const prevFileById = new Map(prevStages.flatMap((st) => st.files.map((f) => [f.id, f])));
    // Ранее созданные файлы (правила/персоны) по stageId — пережить пересборку стадий.
    const prevExtrasByStage = new Map<string, CortexAgentFile[]>();
    for (const st of prevStages) {
      for (const f of st.files) {
        const arr = prevExtrasByStage.get(f.stageId) ?? [];
        arr.push(f);
        prevExtrasByStage.set(f.stageId, arr);
      }
    }

    // Уникальные persona в порядке колонок; колонки без персоны пропускаются
    const boardStages: CortexAgentStage[] = [];
    for (const col of columns) {
      if (col.persona == null || seenPersonas.has(col.persona)) continue;
      seenPersonas.add(col.persona);
      const fileId = `${col.id}-persona`;
      const prevFile = prevFileById.get(fileId);
      // Подхватить ранее созданные файлы стадии (рядом с моками вида .cvh/rules/dev/*.md),
      // чтобы пересборка из колонок их не стирала.
      const extras = (prevExtrasByStage.get(col.id) ?? []).filter((f) => f.id !== fileId);
      boardStages.push({
        stageId: col.id,
        stageTitle: col.title,
        color: col.color,
        icon: col.icon,
        personaName: col.persona,
        isExpanded: prevStageById.get(col.id)?.isExpanded ?? true,
        files: [
          {
            id: fileId,
            name: `${col.persona}.md`,
            stageId: col.id,
            type: 'persona',
            path: `.cvh/agents/${col.persona}.md`,
            content: prevFile?.content ?? '',
            isDirty: prevFile?.isDirty ?? false,
          },
          ...extras,
        ],
      });
    }

    if (boardStages.length === 0) {
      // Fallback для web-preview без бэкенда: мок-стадии
      set({ stages: INITIAL_CORTEX_STAGES });
      return;
    }

    // Сохранить пометки об отсутствующих персонах для переживших пересборку файлов
    const prevMissing = get().personaMissing;
    const nextMissing: Record<string, boolean> = {};
    for (const st of boardStages) {
      for (const f of st.files) {
        if (prevMissing[f.id]) nextMissing[f.id] = true;
      }
    }
    set({ stages: boardStages, personaMissing: nextMissing });
  },

  openFile: async (fileId) => {
    const { stages } = get();
    for (const stage of stages) {
      const file = stage.files.find((f) => f.id === fileId);
      if (!file) continue;

      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const content = await invoke<string>('read_cortex_file', { path: file.path });
        const loaded: CortexAgentFile = { ...file, content, isDirty: false };
        set((state) => ({
          stages: state.stages.map((st) =>
            st.stageId === stage.stageId
              ? {
                  ...st,
                  files: st.files.map((f) =>
                    f.id === fileId ? { ...f, content, isDirty: false } : f
                  ),
                }
              : st
          ),
          activeFile: loaded,
          isEditorOpen: true,
          saveFeedback: null,
        }));
        // Файл на месте: снять пометку об отсутствии, если была
        set((state) => {
          if (!state.personaMissing[fileId]) return state;
          const next = { ...state.personaMissing };
          delete next[fileId];
          return { personaMissing: next };
        });
      } catch (err) {
        console.info('read_cortex_file not available or file is missing:', err);
        if (file.content) {
          // Fallback для web-preview без бэкенда: открыть содержимое из памяти
          set({ activeFile: { ...file }, isEditorOpen: true, saveFeedback: null });
          return;
        }
        // Файла нет: показать в дереве плейсхолдер «Персона не настроена»
        set((state) => ({
          personaMissing: { ...state.personaMissing, [fileId]: true },
        }));
      }
      return;
    }
  },

  createPersonaTemplate: async (fileId) => {
    const { stages } = get();
    for (const stage of stages) {
      const file = stage.files.find((f) => f.id === fileId);
      if (!file) continue;

      const template = buildPersonaTemplate(stage.personaName, stage.stageTitle);
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('save_cortex_file', { path: file.path, content: template });
      } catch (err) {
        console.info('save_cortex_file not available or in web mode:', err);
      }
      // Перезагрузить узел: обновить содержимое, снять пометку, открыть в редакторе
      const loaded: CortexAgentFile = { ...file, content: template, isDirty: false };
      set((state) => {
        const nextMissing = { ...state.personaMissing };
        delete nextMissing[fileId];
        return {
          stages: state.stages.map((st) =>
            st.stageId === stage.stageId
              ? {
                  ...st,
                  files: st.files.map((f) =>
                    f.id === fileId ? { ...f, content: template, isDirty: false } : f
                  ),
                }
              : st
          ),
          activeFile: loaded,
          isEditorOpen: true,
          saveFeedback: null,
          personaMissing: nextMissing,
        };
      });
      return;
    }
  },

  createCortexFile: async (stageId, rawName, kind) => {
    const { stages } = get();
    const stage = stages.find((st) => st.stageId === stageId);
    if (!stage) throw new Error('Стадия не найдена');

    const hasPersona = (stage.personaName ?? '').trim().length > 0;
    const effectiveKind: 'rule' | 'persona' = !hasPersona ? 'rule' : kind;

    const { base, fileName } = normalizeCortexFileName(rawName);
    if (!base) throw new Error('Введите имя файла');
    if (!/^[A-Za-z0-9-_]+$/.test(base)) {
      throw new Error('Только латиница, цифры, - и _');
    }

    // Папки рядом с существующими путями: правила — `.cvh/rules/<stageId|common>/`,
    // персоны — `.cvh/agents/` (как `.cvh/rules/dev/safety.md`, `.cvh/agents/<role>.md`).
    const targetPath =
      effectiveKind === 'persona'
        ? `.cvh/agents/${fileName}`
        : `.cvh/rules/${hasPersona ? stage.stageId : 'common'}/${fileName}`;

    const duplicate = stage.files.some(
      (f) => f.name.toLowerCase() === fileName.toLowerCase() || f.path.toLowerCase() === targetPath.toLowerCase()
    );
    if (duplicate) throw new Error(`Файл ${fileName} уже есть в стадии`);

    const content =
      effectiveKind === 'persona'
        ? buildPersonaTemplate(base, stage.stageTitle)
        : buildRuleTemplate(base, stage.stageTitle);

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_cortex_file', { path: targetPath, content });
    } catch (err) {
      const detail = err instanceof Error ? err.message : String(err);
      throw new Error(`Не удалось сохранить файл: ${detail}`);
    }

    // Уникальный id узла в пределах стадии
    let fileId = `${stageId}-${effectiveKind}-${base}`;
    let suffix = 2;
    const existingIds = new Set(stage.files.map((f) => f.id));
    while (existingIds.has(fileId)) {
      fileId = `${stageId}-${effectiveKind}-${base}-${suffix}`;
      suffix += 1;
    }

    const created: CortexAgentFile = {
      id: fileId,
      name: fileName,
      stageId,
      type: effectiveKind,
      path: targetPath,
      content,
      isDirty: false,
    };

    // Узел появляется в дереве стадии и открывается в редакторе (существующий флоу)
    set((state) => ({
      stages: state.stages.map((st) =>
        st.stageId === stageId ? { ...st, files: [...st.files, created] } : st
      ),
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
    const { activeFile, stages } = get();
    if (!activeFile) return;

    set({ isSaving: true });

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_cortex_file', { path: activeFile.path, content: activeFile.content });
    } catch (err) {
      console.info('save_cortex_file not available or in web mode:', err);
    }

    // Update in memory stages
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

    set({
      stages: updatedStages,
      activeFile: { ...activeFile, isDirty: false },
      isSaving: false,
      saveFeedback: `Файл ${activeFile.name} сохранен`,
    });

    setTimeout(() => {
      set({ saveFeedback: null });
    }, 2800);
  },

  closeEditor: () => {
    set({
      isEditorOpen: false,
      activeFile: null,
      saveFeedback: null,
    });
  },
}));

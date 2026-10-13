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
  updateActiveFileContent: (content: string) => void;
  saveRules: () => Promise<void>;
  closeEditor: () => void;
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

    // Уникальные persona в порядке колонок; колонки без персоны пропускаются
    const boardStages: CortexAgentStage[] = [];
    for (const col of columns) {
      if (col.persona == null || seenPersonas.has(col.persona)) continue;
      seenPersonas.add(col.persona);
      const fileId = `${col.id}-persona`;
      const prevFile = prevFileById.get(fileId);
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

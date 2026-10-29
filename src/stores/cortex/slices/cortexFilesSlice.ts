import { StateCreator } from 'zustand';
import { CortexFilesSlice, CortexState } from '../types';
import { CortexAgentFile, CortexAgentStage } from '../../../types';
import { DEFAULT_CORTEX_STAGES } from '../cortexConstants';
import {
  buildBuiltinHookDescription,
  buildCommandHookDescription,
  buildPersonaTemplate,
  buildScriptTemplate,
  buildSettingTemplate,
  cortexAgentDir,
  extractScriptPath,
} from '../cortexHelpers';
import {
  BACKLOG_COLOR,
  READY_COLOR,
  getDeterministicAgentColor,
  isAllowedAgentColor,
} from '../../../lib/agentPalette';
import { getDefaultColumnIcon } from '../../../lib/columnIcons';

export const createCortexFilesSlice: StateCreator<
  CortexState,
  [],
  [],
  CortexFilesSlice
> = (set, get) => ({
  stages: DEFAULT_CORTEX_STAGES,
  agentStages: [],
  globalRules: [],
  activeFile: null,
  focusedStageId: null,
  isEditorOpen: false,
  isSaving: false,
  saveFeedback: null,
  fileMissing: {},

  toggleStageExpand: (stageId) => {
    // Один флоу на обе секции: id колонок и агентов (`agent:<role>`) не пересекаются.
    set((state) => ({
      stages: state.stages.map((st) =>
        st.stageId === stageId ? { ...st, isExpanded: !st.isExpanded } : st
      ),
      agentStages: state.agentStages.map((st) =>
        st.stageId === stageId ? { ...st, isExpanded: !st.isExpanded } : st
      ),
    }));
  },

  expandStage: (stageId) => {
    set((state) => ({
      stages: state.stages.map((st) =>
        st.stageId === stageId ? { ...st, isExpanded: true } : st
      ),
      agentStages: state.agentStages.map((st) =>
        st.stageId === stageId ? { ...st, isExpanded: true } : st
      ),
    }));
  },

  focusStage: (stageId) => {
    set((state) => ({
      focusedStageId: stageId,
      stages: state.stages.map((st) =>
        st.stageId === stageId ? { ...st, isExpanded: true } : st
      ),
      agentStages: state.agentStages.map((st) =>
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

    // Каждая колонка — стадия в проводнике разума. Все колонки из columns
    // остаются в boardStages проводника разума (дедупа по persona здесь нет:
    // дедуп живёт только в секции агентов).
    // Системные узлы (backlog + ready/terminal без персоны) показывают свои hooks
    // из колонки и personaName: 'система'.
    // Колонки без выбранного агента отображаются со статусом personaName: 'без агента',
    // сохраняя файлы хуков/правил стадии.
    const boardStages: CortexAgentStage[] = [];
    for (const col of columns) {
      const persona = col.persona?.trim() ?? '';
      const isSystemNode =
        persona === '' &&
        (col.id === 'backlog' || col.id === 'ready' || col.terminal === true);
      const dir = cortexAgentDir(col.persona ?? '', col.id);
      if (isSystemNode) {
        // Хуки колонки (on_enter/on_leave) как узлы типа hook.
        const hookMoments = Object.entries(col.hooks ?? {})
          .filter(([moment]) => moment === 'on_enter' || moment === 'on_leave')
          .sort(([a], [b]) => a.localeCompare(b));
        const hookFiles = hookMoments.map(([moment, target]) => {
          const hookId = `${col.id}-hook-${moment}`;
          const prev = prevFileById.get(hookId);
          const targetPath = typeof target === 'string' ? target : target?.script || (target as any)?.path || '';
          return {
            id: hookId,
            name: moment,
            stageId: col.id,
            type: 'hook' as const,
            path: targetPath,
            content: prev?.content ?? '',
            isDirty: false,
          };
        });
        const extras = (prevExtrasByStage.get(col.id) ?? [])
          .filter(
            (f) =>
              !hookFiles.some((h) => h.id === f.id) &&
              (f.type === 'rule' || f.type === 'hook') &&
              f.path.startsWith(`${dir}/`),
          )
          .sort((a, b) => {
            const order = (t: string) => (t === 'rule' ? 0 : 1);
            return order(a.type) - order(b.type) || a.name.localeCompare(b.name);
          });
        boardStages.push({
          stageId: col.id,
          stageTitle: col.title,
          color: col.color,
          icon: col.icon,
          personaName: 'система',
          isExpanded: prevStageById.get(col.id)?.isExpanded ?? true,
          files: [...hookFiles, ...extras],
        });
        continue;
      }
      const personaId = `${col.id}-persona`;
      const settingsId = `${col.id}-settings`;
      const prevPersona = prevFileById.get(personaId);
      const prevSettings = prevFileById.get(settingsId);
      // Хуки колонки (on_enter/on_leave) как узлы типа hook
      const hookMoments = Object.entries(col.hooks ?? {})
        .filter(([moment]) => moment === 'on_enter' || moment === 'on_leave' || moment === 'on_finish' || moment === 'on_exit')
        .sort(([a], [b]) => a.localeCompare(b));
      const hookFiles = hookMoments.map(([moment, target]) => {
        const hookId = `${col.id}-hook-${moment}`;
        const prev = prevFileById.get(hookId);
        const targetPath = typeof target === 'string' ? target : target?.script || (target as any)?.path || '';
        return {
          id: hookId,
          name: moment,
          stageId: col.id,
          type: 'hook' as const,
          path: targetPath,
          content: prev?.content ?? '',
          isDirty: false,
        };
      });
      // Ранее созданные файлы стадии (правила/хуки новой схемы) — пережить пересборку.
      // Фильтр по префиксу каталога агента: файлы чужой роли и старой схемы
      // (`.cvh/rules/...`, `.cvh/agents/<role>.md`) в дерево НЕ возвращаются.
      // Если персона снята, сохраняем хуки и правила этой стадии.
      const extras = (prevExtrasByStage.get(col.id) ?? [])
        .filter(
          (f) =>
            f.id !== personaId &&
            f.id !== settingsId &&
            !hookFiles.some((h) => h.id === f.id) &&
            (f.type === 'rule' || f.type === 'hook') &&
            (f.path.startsWith(`${dir}/`) || f.type === 'hook' || !persona),
        )
        .sort((a, b) => {
          const order = (t: string) => (t === 'rule' ? 0 : 1);
          return order(a.type) - order(b.type) || a.name.localeCompare(b.name);
        });
      const personaInfo = get().agentPersonas.find((ap) => ap.role === col.persona?.trim());
      const personaFileName = personaInfo?.personaFile ?? 'persona.md';

      const diskRules = (personaInfo?.rules ?? []).map((ruleName) => {
        const ruleId = `${col.id}-rule-${ruleName}`;
        const prevRule = prevFileById.get(ruleId);
        return {
          id: ruleId,
          name: ruleName,
          stageId: col.id,
          type: 'rule' as const,
          path: `${dir}/rules/${ruleName}`,
          content: prevRule?.content ?? '',
          isDirty: prevRule?.isDirty ?? false,
        };
      });

      const inMemoryExtras = extras.filter(
        (f) => f.type !== 'rule' || !diskRules.some((dr) => dr.name === f.name),
      );

      const allExtras = [...diskRules, ...inMemoryExtras].sort((a, b) => {
        const order = (t: string) => (t === 'rule' ? 0 : 1);
        return order(a.type) - order(b.type) || a.name.localeCompare(b.name);
      });

      // Четкая стилизация Backlog (#64748b) и Ready (#10b981) + реактивная иконка и цвет персоны
      let stageColor = col.color;
      let stageIcon = col.icon;
      if (col.id === 'backlog') {
        stageColor = BACKLOG_COLOR;
        stageIcon = col.icon || getDefaultColumnIcon('backlog');
      } else if (col.id === 'ready') {
        stageColor = READY_COLOR;
        stageIcon = col.icon || getDefaultColumnIcon('ready');
      } else {
        if (col.persona) {
          if (personaInfo?.icon) {
            stageIcon = personaInfo.icon;
          } else if (!stageIcon) {
            stageIcon = getDefaultColumnIcon(col.id);
          }
          if (personaInfo?.color && isAllowedAgentColor(personaInfo.color)) {
            stageColor = personaInfo.color;
          } else if (col.color && isAllowedAgentColor(col.color)) {
            stageColor = col.color;
          } else {
            stageColor = getDeterministicAgentColor(col.persona);
          }
        } else {
          stageIcon = col.icon || getDefaultColumnIcon(col.id);
          stageColor = isAllowedAgentColor(col.color) ? col.color : '#171717';
        }
      }

      boardStages.push({
        stageId: col.id,
        stageTitle: col.title,
        color: stageColor,
        icon: stageIcon,
        personaName: col.persona?.trim() || 'без агента',
        isExpanded: prevStageById.get(col.id)?.isExpanded ?? true,
        files: [
          {
            id: personaId,
            name: personaFileName,
            stageId: col.id,
            type: 'persona',
            path: `${dir}/${personaFileName}`,
            content: prevPersona?.content ?? '',
            isDirty: prevPersona?.isDirty ?? false,
          },
          ...hookFiles,
          ...allExtras,
          {
            id: settingsId,
            name: 'setting',
            stageId: col.id,
            type: 'settings',
            path: `${dir}/setting`,
            content: prevSettings?.content ?? '',
            isDirty: prevSettings?.isDirty ?? false,
          },
        ],
      });
    }

    if (boardStages.length === 0) {
      if (columns.length === 0) {
        // Fallback для web-preview без бэкенда: мок-стадии.
        // Секцию агентов не трогаем: agentStages живут отдельно.
        set({ stages: DEFAULT_CORTEX_STAGES });
        return;
      }
      // Свежий проект без агентов (только backlog+ready без persona):
      // пустое состояние секции колонок. Пометы agentStages не сбрасываем.
      set((state) => {
        const aliveAgentIds = new Set(
          state.agentStages.flatMap((s) => s.files.map((f) => f.id)),
        );
        const nextMissing: Record<string, boolean> = {};
        for (const [id, value] of Object.entries(state.fileMissing)) {
          if (aliveAgentIds.has(id)) nextMissing[id] = value;
        }
        return { stages: [], fileMissing: nextMissing };
      });
      return;
    }

    // Сохранить пометки об отсутствующих файлах для переживших пересборку узлов.
    // Встроенный маркер `builtin:create_worktree` — не файл: пометку не переносим.
    // Пометы секции агентов переживают пересборку колонок без изменений.
    const prevMissing = get().fileMissing;
    const prevAgentStages = get().agentStages;
    const nextMissing: Record<string, boolean> = {};
    for (const st of boardStages) {
      for (const f of st.files) {
        if (f.path.startsWith('builtin:')) continue;
        if (prevMissing[f.id]) nextMissing[f.id] = true;
      }
    }
    for (const st of prevAgentStages) {
      for (const f of st.files) {
        if (prevMissing[f.id]) nextMissing[f.id] = true;
      }
    }
    set({ stages: boardStages, fileMissing: nextMissing });
  },

  loadGlobalRules: async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const files = await invoke<Array<{ name: string; path: string; fileType: string }>>(
        'list_global_rules'
      );
      const prevGlobalRules = get().globalRules;
      const prevMap = new Map(prevGlobalRules.map((f) => [f.name, f]));
      const mapped: CortexAgentFile[] = (files ?? []).map((file) => {
        const prev = prevMap.get(file.name);
        return {
          id: `global-rule-${file.name}`,
          name: file.name,
          stageId: 'global',
          type: 'rule',
          path: file.path,
          content: prev?.content ?? '',
          isDirty: prev?.isDirty ?? false,
        };
      });
      set({ globalRules: mapped });
    } catch (err) {
      console.info('loadGlobalRules not available or in web mode:', err);
    }
  },

  openFile: async (fileId) => {
    // 0. Сначала проверяем глобальные правила проекта (.cvh/rules/)
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

    // Один флоу открытия на обе секции: ищем узел и в колонках, и в агентах.
    const { stages, agentStages } = get();
    const allStages = [...stages, ...agentStages];
    for (const stage of allStages) {
      const file = stage.files.find((f) => f.id === fileId);
      if (!file) continue;

      // 1. Встроенные хуки (builtin:create_worktree и др.)
      if (file.path.startsWith('builtin:')) {
        const content =
          file.content && file.content.trim().length > 0
            ? file.content
            : buildBuiltinHookDescription(file.path, file.name, stage.stageTitle);
        const loaded: CortexAgentFile = { ...file, content, isDirty: false };
        set((state) => ({
          stages: state.stages.map((st) =>
            st.stageId === stage.stageId
              ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
              : st
          ),
          agentStages: state.agentStages.map((st) =>
            st.stageId === stage.stageId
              ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
              : st
          ),
          activeFile: loaded,
          isEditorOpen: true,
          saveFeedback: null,
        }));
        return;
      }

      // 2. Хуки стадий: скрипты или системные команды
      if (file.type === 'hook') {
        const scriptPath = extractScriptPath(file.path);
        if (scriptPath) {
          // Хук является скриптом: пробуем прочитать его реальный исходный код с диска
          try {
            const { invoke } = await import('@tauri-apps/api/core');
            const content = await invoke<string>('read_cortex_file', { path: scriptPath });
            const loaded: CortexAgentFile = { ...file, content, isDirty: false };
            set((state) => ({
              stages: state.stages.map((st) =>
                st.stageId === stage.stageId
                  ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
                  : st
              ),
              agentStages: state.agentStages.map((st) =>
                st.stageId === stage.stageId
                  ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
                  : st
              ),
              activeFile: loaded,
              isEditorOpen: true,
              saveFeedback: null,
            }));
            // Файл найден: снимаем пометку отсутствия
            set((state) => {
              if (!state.fileMissing[fileId]) return state;
              const next = { ...state.fileMissing };
              delete next[fileId];
              return { fileMissing: next };
            });
            return;
          } catch (err) {
            console.info('read_cortex_file for hook script failed, using template/memory:', err);
            const content =
              file.content && file.content.trim().length > 0
                ? file.content
                : buildScriptTemplate(scriptPath, file.name, stage.stageId);
            const loaded: CortexAgentFile = { ...file, content, isDirty: false };
            set((state) => ({
              stages: state.stages.map((st) =>
                st.stageId === stage.stageId
                  ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
                  : st
              ),
              agentStages: state.agentStages.map((st) =>
                st.stageId === stage.stageId
                  ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
                  : st
              ),
              activeFile: loaded,
              isEditorOpen: true,
              saveFeedback: null,
            }));
            return;
          }
        } else {
          // Прямая команда (напр. "cargo check", "npm run build") — формируем структурированное описание
          const content =
            file.content && file.content.trim().length > 0
              ? file.content
              : buildCommandHookDescription(file.path, file.name, stage.stageTitle);
          const loaded: CortexAgentFile = { ...file, content, isDirty: false };
          set((state) => ({
            stages: state.stages.map((st) =>
              st.stageId === stage.stageId
                ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
                : st
            ),
            agentStages: state.agentStages.map((st) =>
              st.stageId === stage.stageId
                ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
                : st
            ),
            activeFile: loaded,
            isEditorOpen: true,
            saveFeedback: null,
          }));
          return;
        }
      }

      // 3. Обычные текстовые файлы (persona, rule, settings)
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
          agentStages: state.agentStages.map((st) =>
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
          if (!state.fileMissing[fileId]) return state;
          const next = { ...state.fileMissing };
          delete next[fileId];
          return { fileMissing: next };
        });
      } catch (err) {
        console.info('read_cortex_file not available or file is missing:', err);
        if (file.content) {
          // Fallback для web-preview без бэкенда: открыть содержимое из памяти
          set({ activeFile: { ...file }, isEditorOpen: true, saveFeedback: null });
          return;
        }
        // Файла нет на диске: показать в дереве плейсхолдер «нет файла»
        // с кнопкой создания по существующему флоу шаблонов
        set((state) => ({
          fileMissing: { ...state.fileMissing, [fileId]: true },
        }));
      }
      return;
    }
  },

  createPersonaTemplate: async (fileId) => {
    const { stages, agentStages } = get();
    const allStages = [...stages, ...agentStages];
    for (const stage of allStages) {
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
        const nextMissing = { ...state.fileMissing };
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
          agentStages: state.agentStages.map((st) =>
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
          fileMissing: nextMissing,
        };
      });
      await get().loadAgentPersonas();
      return;
    }
  },

  createSettingTemplate: async (fileId) => {
    const { stages, agentStages } = get();
    const allStages = [...stages, ...agentStages];
    for (const stage of allStages) {
      const file = stage.files.find((f) => f.id === fileId);
      if (!file) continue;

      const role = (stage.personaName ?? '').trim() || stage.stageId;
      const template = buildSettingTemplate(role, stage.stageTitle);
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('save_cortex_file', { path: file.path, content: template });
      } catch (err) {
        console.info('save_cortex_file not available or in web mode:', err);
      }
      // Перезагрузить узел: обновить содержимое, снять пометку, открыть в редакторе
      const loaded: CortexAgentFile = { ...file, content: template, isDirty: false };
      set((state) => {
        const nextMissing = { ...state.fileMissing };
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
          agentStages: state.agentStages.map((st) =>
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
          fileMissing: nextMissing,
        };
      });
      return;
    }
  },

  closeEditor: () => {
    set({
      isEditorOpen: false,
      activeFile: null,
      saveFeedback: null,
    });
  },
});

import { create } from 'zustand';
import { CortexAgentFile, CortexAgentStage, PipelineColumn } from '../types';
import { INITIAL_CORTEX_STAGES } from './mockData';

interface CortexState {
  stages: CortexAgentStage[];
  // Секция «Агенты»: стадии всех доступных персон (`list_agent_personas`),
  // независимые от колонок доски. Файлы правил/хуков здесь — только
  // созданные через createCortexFile в текущей сессии (листинга директорий нет).
  agentStages: CortexAgentStage[];
  agentPersonas: AgentPersonaInfo[];
  activeFile: CortexAgentFile | null;
  focusedStageId: string | null;
  isEditorOpen: boolean;
  isSaving: boolean;
  saveFeedback: string | null;
  // Файлы, отсутствующие на диске (read_cortex_file завершился ошибкой), по id файла.
  // Покрывает одиночные узлы новой схемы: `agent.md` и `setting`.
  // Списки rules/ и hooks/ без листинга директорий неизвестны и сюда не попадают:
  // стор показывает только файлы, созданные через createCortexFile в текущей сессии.
  fileMissing: Record<string, boolean>;

  // Actions
  toggleStageExpand: (stageId: string) => void;
  focusStage: (stageId: string) => void;
  // Перестроить стадии из колонок доски: колонка с persona = агент
  syncStagesFromColumns: (columns: PipelineColumn[]) => void;
  // Загрузить доступных агентов (`list_agent_personas`) и перестроить секцию «Агенты».
  // Ошибка IPC → пустой список, секция показывает подсказку. Колонки не трогает.
  loadAgentPersonas: () => Promise<void>;
  openFile: (fileId: string) => Promise<void>;
  createPersonaTemplate: (fileId: string) => Promise<void>;
  createSettingTemplate: (fileId: string) => Promise<void>;
  // Создать новый файл в стадии по НОВОЙ схеме `.cvh/agents/<role>/...`:
  // правило -> `rules/<имя>.md`, хук -> `hooks/<момент>[-<имя>].<расширение>`.
  // <role> = persona колонки (fallback — stageId). Родительские каталоги
  // создаёт бэкенд (save_cortex_file делает create_dir_all).
  // При ошибке IPC бросает Error с русским текстом (форма показывает его красным).
  createCortexFile: (
    stageId: string,
    rawName: string,
    kind: 'rule' | 'hook',
    opts?: { hookMoment?: CortexHookMoment; hookExt?: CortexHookExt },
  ) => Promise<void>;
  updateActiveFileContent: (content: string) => void;
  saveRules: () => Promise<void>;
  closeEditor: () => void;
}

export type CortexHookMoment = 'on_enter' | 'on_finish';
export type CortexHookExt = 'bat' | 'ps1' | 'py' | 'sh';

// Персона из бэкенда (`list_agent_personas` → `[{role, hasPrompt}]`,
// сериализация Rust — camelCase). Сырой ответ принимаем терпимо:
// учитываем и `has_prompt`, и отсутствие флага.
export interface AgentPersonaInfo {
  role: string;
  hasPrompt: boolean;
}

interface RawAgentPersona {
  role: string;
  hasPrompt?: boolean;
  has_prompt?: boolean;
}

// Каталог агента по новой схеме. Роль = persona колонки; фолбэк — stageId,
// чтобы стадии без персоны тоже получали детерминированный путь.
// Системный backlog-узел (personaName «система»/«system») — не роль:
// тоже фолбэк на stageId, иначе файлы ушли бы в `.cvh/agents/система`.
export function cortexAgentDir(personaName: string, stageId: string): string {
  const trimmed = (personaName ?? '').trim();
  const role =
    !trimmed || trimmed === 'система' || trimmed.toLowerCase() === 'system'
      ? stageId
      : trimmed;
  return `.cvh/agents/${role}`;
}

// Минимальный русский шаблон правила для `.cvh/agents/<role>/rules/`
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

// Минимальный шаблон хука: момент в имени файла, расширение задаёт интерпретатор.
function buildHookTemplate(
  moment: CortexHookMoment,
  ext: CortexHookExt,
  label: string,
  role: string,
  stageTitle: string,
): string {
  const head =
    moment === 'on_enter'
      ? `Хук входа задачи агента ${role} (${stageTitle})`
      : `Хук завершения задачи агента ${role} (${stageTitle})`;
  switch (ext) {
    case 'bat':
      return `@echo off\nREM ${head}: ${label}\nREM Добавьте команды проверки и завершения.\n`;
    case 'ps1':
      return `# ${head}: ${label}\n# Добавьте команды проверки и завершения.\n`;
    case 'py':
      return `#!/usr/bin/env python3\n# ${head}: ${label}\n# Добавьте проверки и завершите с кодом 0 при успехе.\n`;
    case 'sh':
      return `#!/bin/sh\n# ${head}: ${label}\n# Добавьте проверки и завершите с кодом 0 при успехе.\n`;
  }
}

// Минимальный русский шаблон настроек для `.cvh/agents/<role>/setting` (JSON без расширения)
function buildSettingTemplate(role: string, stageTitle: string): string {
  return (
    `{\n` +
    `  "_comment": "Настройки агента ${role} (${stageTitle})",\n` +
    `  "model": null,\n` +
    `  "temperature": 0.2,\n` +
    `  "contextWindow": 128000\n` +
    `}\n`
  );
}

// Имя файла хука по моменту и расширению. Базу из ввода очищаем от
// вручную вписанного расширения, чтобы не получить `setup.bat.bat`.
export function buildHookFileName(
  rawName: string,
  moment: CortexHookMoment,
  ext: CortexHookExt,
): string {
  const typed = rawName.trim().replace(/\.[A-Za-z0-9]+$/, '').trim();
  return typed ? `${moment}-${typed}.${ext}` : `${moment}.${ext}`;
}

// Превью имени создаваемого файла для формы (без записи на диск).
export function previewCortexFileName(
  rawName: string,
  kind: 'rule' | 'hook',
  opts?: { hookMoment?: CortexHookMoment; hookExt?: CortexHookExt },
): string {
  if (kind === 'rule') return normalizeCortexFileName(rawName).fileName;
  return buildHookFileName(rawName, opts?.hookMoment ?? 'on_enter', opts?.hookExt ?? 'bat');
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
  agentStages: [],
  agentPersonas: [],
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

    // Уникальные persona в порядке колонок; колонки без персоны пропускаются,
    // КРОМЕ системных узлов (backlog + ready/terminal без персоны):
    // показывают свои hooks из колонки (чтобы вешать on_finish вроде коммит+пуш).
    // rules/setting у системных узлов НЕТ.
    const boardStages: CortexAgentStage[] = [];
    for (const col of columns) {
      const isSystemNode =
        (col.persona == null || col.persona.trim() === '') &&
        (col.id === 'backlog' || col.id === 'ready' || col.terminal === true);
      if (!isSystemNode) {
        if (col.persona == null || seenPersonas.has(col.persona)) continue;
        seenPersonas.add(col.persona);
      }
      const dir = cortexAgentDir(col.persona ?? '', col.id);
      if (isSystemNode) {
        // Хуки колонки (on_enter/on_finish) как узлы типа hook.
        // Маркер `builtin:create_worktree` хранится в path — проводник
        // покажет бейдж «встроенный» и не будет открывать файл.
        const hookMoments = Object.entries(col.hooks ?? {})
          .filter(([moment]) => moment === 'on_enter' || moment === 'on_finish')
          .sort(([a], [b]) => a.localeCompare(b));
        const hookFiles = hookMoments.map(([moment, target]) => {
          const hookId = `${col.id}-hook-${moment}`;
          const prev = prevFileById.get(hookId);
          return {
            id: hookId,
            name: moment,
            stageId: col.id,
            type: 'hook' as const,
            path: target,
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
      // Ранее созданные файлы стадии (правила/хуки новой схемы) — пережить пересборку.
      // Фильтр по префиксу каталога агента: файлы чужой роли и старой схемы
      // (`.cvh/rules/...`, `.cvh/agents/<role>.md`) в дерево НЕ возвращаются.
      const extras = (prevExtrasByStage.get(col.id) ?? [])
        .filter(
          (f) =>
            f.id !== personaId &&
            f.id !== settingsId &&
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
        personaName: col.persona ?? '',
        isExpanded: prevStageById.get(col.id)?.isExpanded ?? true,
        files: [
          {
            id: personaId,
            name: 'agent.md',
            stageId: col.id,
            type: 'persona',
            path: `${dir}/agent.md`,
            content: prevPersona?.content ?? '',
            isDirty: prevPersona?.isDirty ?? false,
          },
          ...extras,
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
        set({ stages: INITIAL_CORTEX_STAGES });
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
        if (f.path === 'builtin:create_worktree') continue;
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

  loadAgentPersonas: async () => {
    let raw: RawAgentPersona[] = [];
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const list = await invoke<RawAgentPersona[]>('list_agent_personas');
      if (Array.isArray(list)) raw = list;
    } catch {
      // IPC недоступен — секция агентов пуста, колонки не страдают.
      raw = [];
    }
    const seen = new Set<string>();
    const personas: AgentPersonaInfo[] = [];
    for (const item of raw) {
      if (!item || typeof item.role !== 'string') continue;
      const role = item.role.trim();
      if (!role || seen.has(role)) continue;
      seen.add(role);
      personas.push({
        role,
        hasPrompt: item.hasPrompt ?? item.has_prompt ?? true,
      });
    }
    personas.sort((a, b) => a.role.localeCompare(b.role));

    const prev = get().agentStages;
    const prevByRole = new Map(prev.map((st) => [st.personaName, st]));
    const nextStages: CortexAgentStage[] = personas.map((p) => {
      const dir = cortexAgentDir(p.role, p.role);
      const stageId = `agent:${p.role}`;
      const prevStage = prevByRole.get(p.role);
      const prevFiles = prevStage?.files ?? [];
      const prevById = new Map(prevFiles.map((f) => [f.id, f]));
      const personaId = `${stageId}-persona`;
      const settingsId = `${stageId}-settings`;
      const prevPersona = prevById.get(personaId);
      const prevSettings = prevById.get(settingsId);
      // Ранее созданные файлы агента (правила/хуки) — пережить перезагрузку списка.
      const extras = prevFiles
        .filter(
          (f) =>
            f.id !== personaId &&
            f.id !== settingsId &&
            (f.type === 'rule' || f.type === 'hook'),
        )
        .sort((a, b) => {
          const order = (t: string) => (t === 'rule' ? 0 : 1);
          return order(a.type) - order(b.type) || a.name.localeCompare(b.name);
        });
      return {
        stageId,
        stageTitle: p.role,
        color: '#171717',
        icon: 'User',
        personaName: p.role,
        isExpanded: prevStage?.isExpanded ?? false,
        files: [
          {
            id: personaId,
            name: 'agent.md',
            stageId,
            type: 'persona',
            path: `${dir}/agent.md`,
            content: prevPersona?.content ?? '',
            isDirty: prevPersona?.isDirty ?? false,
          },
          ...extras,
          {
            id: settingsId,
            name: 'setting',
            stageId,
            type: 'settings',
            path: `${dir}/setting`,
            content: prevSettings?.content ?? '',
            isDirty: prevSettings?.isDirty ?? false,
          },
        ],
      };
    });

    // Снять пометы об отсутствующих файлах для агентов, которых больше нет.
    const alive = new Set(nextStages.flatMap((s) => s.files.map((f) => f.id)));
    for (const st of get().stages) {
      for (const f of st.files) alive.add(f.id);
    }
    const prunedMissing: Record<string, boolean> = {};
    for (const [id, value] of Object.entries(get().fileMissing)) {
      if (alive.has(id)) prunedMissing[id] = value;
    }
    set({ agentPersonas: personas, agentStages: nextStages, fileMissing: prunedMissing });
  },

  openFile: async (fileId) => {
    // Один флоу открытия на обе секции: ищем узел и в колонках, и в агентах.
    const { stages, agentStages } = get();
    const allStages = [...stages, ...agentStages];
    for (const stage of allStages) {
      const file = stage.files.find((f) => f.id === fileId);
      if (!file) continue;
      // Встроенный маркер — не файл на диске: открывать нечего.
      if (file.path === 'builtin:create_worktree') return;

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
      if (moment !== 'on_enter' && moment !== 'on_finish') {
        throw new Error('Момент хука: on_enter или on_finish');
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
    const { activeFile, stages, agentStages } = get();
    if (!activeFile) return;

    set({ isSaving: true });

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_cortex_file', { path: activeFile.path, content: activeFile.content });
    } catch (err) {
      console.info('save_cortex_file not available or in web mode:', err);
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

    set({
      stages: updatedStages,
      agentStages: updatedAgentStages,
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

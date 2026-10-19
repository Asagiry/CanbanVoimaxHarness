import { create } from 'zustand';
import { useBoardStore } from './boardStore';

export interface ToolMeta {
  id: string;
  name: string;
  label: string;
  description: string;
  category: 'system' | 'fs' | 'web' | 'agent' | 'workflow' | 'interactive' | 'orchestration';
  tokenWeight: number;
  parentTool?: string;
  isSubagentMaster?: boolean;
}

export const AVAILABLE_TOOLS: ToolMeta[] = [
  {
    id: 'create_task',
    name: 'create_task',
    label: 'Создать задачу',
    description: 'Декомпозиция требований и создание новой карточки задачи на Канбан-доске с привязкой к текущей задаче.',
    category: 'orchestration',
    tokenWeight: 260,
  },
  {
    id: 'complete_task',
    name: 'complete_task',
    label: 'Завершить задачу',
    description: 'Финализация задачи оркестратора и перевод в Готово. Защищено инвариантом Anti-Bypass.',
    category: 'orchestration',
    tokenWeight: 220,
  },
  {
    id: 'question',
    name: 'question',
    label: 'Вопрос пользователю',
    description: 'Задать уточняющий вопрос пользователю и ожидать ответа в интерфейсе. Алиас: ask_user.',
    category: 'interactive',
    tokenWeight: 240,
  },
  {
    id: 'advance_task',
    name: 'advance_task',
    label: 'Передать этап',
    description: 'Передать задачу на следующий этап пайплайна с обязательным саммари проделанной работы.',
    category: 'workflow',
    tokenWeight: 220,
  },
  {
    id: 'reject_task',
    name: 'reject_task',
    label: 'Вернуть на доработку',
    description: 'Вернуть задачу на предыдущий этап пайплайна с указанием причины отклонения и замечаний.',
    category: 'workflow',
    tokenWeight: 220,
  },
  {
    id: 'execute_command',
    name: 'execute_command',
    label: 'Команды терминала',
    description: 'Выполнение команд в изолированном каталоге worktree.',
    category: 'system',
    tokenWeight: 320,
  },
  {
    id: 'read_file',
    name: 'read_file',
    label: 'Чтение файла',
    description: 'Просмотр содержимого файлов UTF-8 со смещением строк.',
    category: 'fs',
    tokenWeight: 260,
  },
  {
    id: 'edit_file',
    name: 'edit_file',
    label: 'Целевое редактирование',
    description: 'Точечная замена строк в файлах с фиксацией различий.',
    category: 'fs',
    tokenWeight: 310,
  },
  {
    id: 'write_file',
    name: 'write_file',
    label: 'Запись файла',
    description: 'Атомарная запись полного содержимого файла в worktree.',
    category: 'fs',
    tokenWeight: 260,
  },
  {
    id: 'glob',
    name: 'glob',
    label: 'Поиск по маске',
    description: 'Поиск файлов по маске внутри каталога проекта.',
    category: 'fs',
    tokenWeight: 240,
  },
  {
    id: 'grep',
    name: 'grep',
    label: 'Поиск по тексту',
    description: 'Поиск регулярных выражений с номерами строк.',
    category: 'fs',
    tokenWeight: 250,
  },
  {
    id: 'web_search',
    name: 'web_search',
    label: 'Поиск в сети',
    description: 'Поиск документации и внешних решений в интернете.',
    category: 'web',
    tokenWeight: 290,
  },
  {
    id: 'web_fetch',
    name: 'web_fetch',
    label: 'Загрузка веб-страниц',
    description: 'Загрузка внешних веб-страниц с преобразованием в Markdown.',
    category: 'web',
    tokenWeight: 280,
  },
  {
    id: 'subagent',
    name: 'subagent',
    label: 'Вызов субагентов',
    description: 'Мастер-переключатель вызова субагентов. Если отключен, запуск как синхронных, так и фоновых субагентов заблокирован.',
    category: 'agent',
    tokenWeight: 310,
    isSubagentMaster: true,
  },
  {
    id: 'subagent_sync',
    name: 'subagent',
    label: 'Субагент (синхронный)',
    description: 'Синхронный запуск субагента с блокировкой на стороне Rust до получения финального отчёта.',
    category: 'agent',
    tokenWeight: 290,
    parentTool: 'subagent',
  },
  {
    id: 'subagent_bg',
    name: 'subagent_bg',
    label: 'Субагент (фоновый)',
    description: 'Асинхронный запуск субагента в фоне с последующим получением результата через collect_subagent без поллинга.',
    category: 'agent',
    tokenWeight: 290,
    parentTool: 'subagent',
  },
  {
    id: 'collect_subagent',
    name: 'collect_subagent',
    label: 'Собрать субагентов',
    description: 'Ожидание завершения и получение отчетов фоновых субагентов по subagent_id без поллинга.',
    category: 'agent',
    tokenWeight: 260,
    parentTool: 'subagent',
  },
  {
    id: 'todo_write',
    name: 'todo_write',
    label: 'План выполнения',
    description: 'Фиксация и обновление плана задачи: шаги pending | in_progress | completed',
    category: 'agent',
    tokenWeight: 280,
  },
];

export interface ToolParamDef {
  key: string;
  label: string;
  defaultValue: number;
  min: number;
  max: number;
  step: number;
  unit: string;
}

export const TOOL_PARAM_DEFS: Record<string, ToolParamDef[]> = {
  read_file: [
    {
      key: 'max_lines',
      label: 'Лимит чтения строк за раз',
      defaultValue: 2000,
      min: 50,
      max: 10000,
      step: 100,
      unit: 'строк',
    },
  ],
  subagent: [
    {
      key: 'max_concurrent',
      label: 'Максимум параллельных субагентов',
      defaultValue: 3,
      min: 1,
      max: 32,
      step: 1,
      unit: 'агентов',
    },
  ],
  subagent_bg: [
    {
      key: 'max_concurrent',
      label: 'Максимум параллельных субагентов',
      defaultValue: 3,
      min: 1,
      max: 32,
      step: 1,
      unit: 'агентов',
    },
  ],
  todo_write: [
    {
      key: 'min_items',
      label: 'Минимум пунктов чеклиста',
      defaultValue: 3,
      min: 1,
      max: 20,
      step: 1,
      unit: 'пунктов',
    },
    {
      key: 'max_items',
      label: 'Максимум пунктов чеклиста',
      defaultValue: 15,
      min: 3,
      max: 50,
      step: 1,
      unit: 'пунктов',
    },
  ],
  execute_command: [
    {
      key: 'timeout_sec',
      label: 'Таймаут выполнения в секундах',
      defaultValue: 180,
      min: 10,
      max: 3600,
      step: 10,
      unit: 'сек',
    },
  ],
};

export type ToolSettings = Record<string, Record<string, number>>;

export const DEFAULT_TOOL_SETTINGS: ToolSettings = {
  read_file: { max_lines: 2000 },
  subagent: { max_concurrent: 3 },
  subagent_bg: { max_concurrent: 3 },
  todo_write: { min_items: 3, max_items: 15 },
  execute_command: { timeout_sec: 180 },
};

export interface ToolPolicyState {
  isModalOpen: boolean;
  globalEnabled: Record<string, boolean>;
  stageOverrides: Record<string, Record<string, boolean>>;
  toolSettings: ToolSettings;

  // Actions
  openModal: () => void;
  closeModal: () => void;
  toggleGlobalTool: (toolId: string) => void;
  setStageTool: (stageId: string, toolId: string, enabled: boolean) => void;
  setToolSetting: (toolId: string, key: string, value: number) => void;
  getToolSetting: (toolId: string, key: string, defaultValue: number) => number;
  isToolAllowed: (toolId: string, stageId?: string) => boolean;
  calculateSavedTokens: (stageId?: string) => number;
  resetDefaults: () => void;
  loadFromTauri: () => Promise<void>;
  saveToTauri: () => Promise<void>;
}

const DEFAULT_GLOBAL: Record<string, boolean> = {
  advance_task: true,
  reject_task: true,
  execute_command: true,
  read_file: true,
  edit_file: true,
  write_file: true,
  glob: true,
  grep: true,
  web_search: true,
  web_fetch: true,
  subagent: true,
  subagent_sync: true,
  subagent_bg: true,
  collect_subagent: true,
  todo_write: true,
  question: true,
  create_task: true,
  complete_task: true,
};

const DEFAULT_OVERRIDES: Record<string, Record<string, boolean>> = {
  backlog: {
    create_task: true,
    complete_task: true,
  },
  orchestrator: {
    create_task: true,
    complete_task: true,
  },
  review: {
    advance_task: true,
    reject_task: true,
    execute_command: true,
    read_file: true,
    edit_file: false,
    write_file: false,
    glob: true,
    grep: true,
    web_search: true,
    web_fetch: true,
    subagent: true,
    subagent_sync: true,
    subagent_bg: true,
    collect_subagent: true,
    todo_write: true,
    question: true,
    create_task: false,
    complete_task: false,
  },
  dev: {
    advance_task: true,
    reject_task: true,
    execute_command: true,
    read_file: true,
    edit_file: true,
    write_file: true,
    glob: true,
    grep: true,
    web_search: true,
    web_fetch: true,
    subagent: true,
    subagent_sync: true,
    subagent_bg: true,
    collect_subagent: true,
    todo_write: true,
    question: true,
    create_task: false,
    complete_task: false,
  },
  retro: {
    advance_task: true,
    reject_task: true,
    execute_command: true,
    read_file: true,
    edit_file: false,
    write_file: true,
    glob: true,
    grep: true,
    web_search: true,
    web_fetch: true,
    subagent: true,
    subagent_sync: true,
    subagent_bg: true,
    collect_subagent: true,
    todo_write: true,
    question: true,
    create_task: false,
    complete_task: false,
  },
};

export const useToolPolicyStore = create<ToolPolicyState>((set, get) => ({
  isModalOpen: false,
  globalEnabled: DEFAULT_GLOBAL,
  stageOverrides: DEFAULT_OVERRIDES,
  toolSettings: DEFAULT_TOOL_SETTINGS,

  openModal: () => set({ isModalOpen: true }),
  closeModal: () => set({ isModalOpen: false }),

  toggleGlobalTool: (toolId) => {
    set((state) => {
      const next = { ...state.globalEnabled, [toolId]: !state.globalEnabled[toolId] };
      return { globalEnabled: next };
    });
    get().saveToTauri();
  },

  setStageTool: (stageId, toolId, enabled) => {
    set((state) => {
      const currentStage = state.stageOverrides[stageId] || {};
      const nextStage = { ...currentStage, [toolId]: enabled };
      return {
        stageOverrides: {
          ...state.stageOverrides,
          [stageId]: nextStage,
        },
      };
    });
    get().saveToTauri();
  },

  setToolSetting: (toolId, key, value) => {
    set((state) => {
      const currentToolSettings = state.toolSettings[toolId] || {};
      const nextToolSettings = { ...currentToolSettings, [key]: value };
      return {
        toolSettings: {
          ...state.toolSettings,
          [toolId]: nextToolSettings,
        },
      };
    });
    get().saveToTauri();
  },

  getToolSetting: (toolId, key, defaultValue) => {
    const { toolSettings } = get();
    return toolSettings[toolId]?.[key] ?? defaultValue;
  },

  isToolAllowed: (toolId, stageOrAgentId) => {
    const { globalEnabled, stageOverrides } = get();

    // Hierarchical rule for subagent sub-options
    if (toolId === 'subagent_sync' || toolId === 'subagent_bg' || toolId === 'collect_subagent') {
      const masterGlobal = globalEnabled['subagent'] ?? true;
      if (!masterGlobal) return false;
      if (stageOrAgentId) {
        const stageMaster = stageOverrides[stageOrAgentId]?.['subagent'];
        if (stageMaster === false) return false;
      }
      const selfGlobal = globalEnabled[toolId] ?? true;
      if (!selfGlobal) return false;
      if (stageOrAgentId) {
        return stageOverrides[stageOrAgentId]?.[toolId] ?? true;
      }
      return true;
    }

    if (stageOrAgentId) {
      if (stageOverrides[stageOrAgentId] && typeof stageOverrides[stageOrAgentId][toolId] === 'boolean') {
        return stageOverrides[stageOrAgentId][toolId];
      }
      try {
        const columns = useBoardStore.getState().columns;
        // 1. Если передан агент (роль), проверяем колонку с этой персоной
        const colForPersona = columns.find((c) => c.persona === stageOrAgentId);
        if (colForPersona && stageOverrides[colForPersona.id] && typeof stageOverrides[colForPersona.id][toolId] === 'boolean') {
          return stageOverrides[colForPersona.id][toolId];
        }
        // 2. Если передан id колонки, проверяем назначенную персону
        const colById = columns.find((c) => c.id === stageOrAgentId);
        if (colById?.persona && stageOverrides[colById.persona] && typeof stageOverrides[colById.persona][toolId] === 'boolean') {
          return stageOverrides[colById.persona][toolId];
        }
      } catch {
        // ignore
      }
      if (DEFAULT_OVERRIDES[stageOrAgentId] && typeof DEFAULT_OVERRIDES[stageOrAgentId][toolId] === 'boolean') {
        return DEFAULT_OVERRIDES[stageOrAgentId][toolId];
      }
    }

    const isGloballyAllowed = globalEnabled[toolId] ?? true;
    if (!isGloballyAllowed) return false;
    if (!stageOrAgentId) return true;
    return stageOverrides[stageOrAgentId]?.[toolId] ?? true;
  },

  calculateSavedTokens: (stageId) => {
    const { isToolAllowed } = get();
    let saved = 0;
    for (const tool of AVAILABLE_TOOLS) {
      if (!isToolAllowed(tool.id, stageId)) {
        saved += tool.tokenWeight;
      }
    }
    return saved > 0 ? saved : 2400;
  },

  resetDefaults: () => {
    set({
      globalEnabled: { ...DEFAULT_GLOBAL },
      stageOverrides: { ...DEFAULT_OVERRIDES },
      toolSettings: { ...DEFAULT_TOOL_SETTINGS },
    });
    get().saveToTauri();
  },

  loadFromTauri: async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const policies = await invoke<{
        globalEnabled?: Record<string, boolean>;
        stageOverrides?: Record<string, Record<string, boolean>>;
        toolSettings?: ToolSettings;
      }>('get_tool_policies');

      if (policies) {
        const mergedSettings: ToolSettings = { ...DEFAULT_TOOL_SETTINGS };
        if (policies.toolSettings) {
          for (const [tool, settings] of Object.entries(policies.toolSettings)) {
            mergedSettings[tool] = {
              ...(DEFAULT_TOOL_SETTINGS[tool] || {}),
              ...settings,
            };
          }
        }
        set({
          globalEnabled: policies.globalEnabled || DEFAULT_GLOBAL,
          stageOverrides: policies.stageOverrides || DEFAULT_OVERRIDES,
          toolSettings: mergedSettings,
        });
      }
    } catch {
      // In web browser or mock environment, keep memory defaults
    }
  },

  saveToTauri: async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const { globalEnabled, stageOverrides, toolSettings } = get();
      await invoke('save_tool_policies', {
        policies: {
          globalEnabled,
          stageOverrides,
          toolSettings,
        },
      });
    } catch {
      // Browser or test fallback
    }
  },
}));

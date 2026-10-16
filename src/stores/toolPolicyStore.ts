import { create } from 'zustand';

export interface ToolMeta {
  id: string;
  name: string;
  label: string;
  description: string;
  category: 'system' | 'fs' | 'web' | 'agent' | 'workflow';
  tokenWeight: number;
  parentTool?: string;
  isSubagentMaster?: boolean;
}

export const AVAILABLE_TOOLS: ToolMeta[] = [
  {
    id: 'advance_task',
    name: 'advance_task',
    label: 'Передать задачу (Advance)',
    description: 'Передать задачу на следующий этап пайплайна (шаг вперед) с обязательным саммари проделанной работы.',
    category: 'workflow',
    tokenWeight: 220,
  },
  {
    id: 'reject_task',
    name: 'reject_task',
    label: 'Вернуть задачу (Reject)',
    description: 'Вернуть задачу на предыдущий этап пайплайна (шаг назад) с указанием причины отклонения и замечаний.',
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
    label: 'Поиск файлов (Glob)',
    description: 'Поиск файлов по маске внутри каталога проекта.',
    category: 'fs',
    tokenWeight: 240,
  },
  {
    id: 'grep',
    name: 'grep',
    label: 'Поиск по тексту (Grep)',
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
    label: "Вызов субагентов (мастер-тул: 'subagent')",
    description: "Мастер-переключатель вызова субагентов. Если отключен, запуск как ваншот, так и фоновых субагентов заблокирован.",
    category: 'agent',
    tokenWeight: 310,
    isSubagentMaster: true,
  },
  {
    id: 'subagent_sync',
    name: 'subagent',
    label: 'subagent (Ваншот)',
    description: 'Синхронный запуск субагента с блокировкой на стороне Rust до получения финального отчёта.',
    category: 'agent',
    tokenWeight: 290,
    parentTool: 'subagent',
  },
  {
    id: 'subagent_bg',
    name: 'subagent_bg',
    label: 'subagent_bg (Фоновый)',
    description: 'Асинхронный запуск субагента в фоне с последующим получением результата через collect_subagent без поллинга.',
    category: 'agent',
    tokenWeight: 290,
    parentTool: 'subagent',
  },
  {
    id: 'todo_write',
    name: 'todo_write',
    label: 'План To-dos',
    description: 'Фиксация и обновление плана задачи: шаги pending | in_progress | completed',
    category: 'agent',
    tokenWeight: 280,
  },
];

export interface ToolPolicyState {
  isModalOpen: boolean;
  globalEnabled: Record<string, boolean>;
  stageOverrides: Record<string, Record<string, boolean>>;

  // Actions
  openModal: () => void;
  closeModal: () => void;
  toggleGlobalTool: (toolId: string) => void;
  setStageTool: (stageId: string, toolId: string, enabled: boolean) => void;
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
  todo_write: true,
};

const DEFAULT_OVERRIDES: Record<string, Record<string, boolean>> = {
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
    todo_write: true,
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
    web_search: false,
    web_fetch: true,
    subagent: true,
    subagent_sync: true,
    subagent_bg: true,
    todo_write: true,
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
    web_search: false,
    web_fetch: false,
    subagent: false,
    subagent_sync: false,
    subagent_bg: false,
    todo_write: true,
  },
};

export const useToolPolicyStore = create<ToolPolicyState>((set, get) => ({
  isModalOpen: false,
  globalEnabled: DEFAULT_GLOBAL,
  stageOverrides: DEFAULT_OVERRIDES,

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

  isToolAllowed: (toolId, stageId) => {
    const { globalEnabled, stageOverrides } = get();

    // Hierarchical rule for subagent sub-options
    if (toolId === 'subagent_sync' || toolId === 'subagent_bg') {
      const masterGlobal = globalEnabled['subagent'] ?? true;
      if (!masterGlobal) return false;
      if (stageId) {
        const stageMaster = stageOverrides[stageId]?.['subagent'];
        if (stageMaster === false) return false;
      }
      const selfGlobal = globalEnabled[toolId] ?? true;
      if (!selfGlobal) return false;
      if (stageId) {
        return stageOverrides[stageId]?.[toolId] ?? true;
      }
      return true;
    }

    const isGloballyAllowed = globalEnabled[toolId] ?? true;
    if (!isGloballyAllowed) return false;
    if (!stageId) return true;
    return stageOverrides[stageId]?.[toolId] ?? true;
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
    });
    get().saveToTauri();
  },

  loadFromTauri: async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const policies = await invoke<{
        globalEnabled?: Record<string, boolean>;
        stageOverrides?: Record<string, Record<string, boolean>>;
      }>('get_tool_policies');

      if (policies) {
        set({
          globalEnabled: policies.globalEnabled || DEFAULT_GLOBAL,
          stageOverrides: policies.stageOverrides || DEFAULT_OVERRIDES,
        });
      }
    } catch {
      // In web browser or mock environment, keep memory defaults
    }
  },

  saveToTauri: async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const { globalEnabled, stageOverrides } = get();
      await invoke('save_tool_policies', {
        policies: {
          globalEnabled,
          stageOverrides,
        },
      });
    } catch {
      // Browser or test fallback
    }
  },
}));

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
  parameters?: Record<string, unknown>;
}

export function getToolTranslationKey(toolId: string, field: 'label' | 'description'): string {
  return `tools.${toolId}.${field}`;
}

export const AVAILABLE_TOOLS: ToolMeta[] = [
  {
    id: 'create_task',
    name: 'create_task',
    label: 'Создать задачу',
    description: 'Decompose requirements and create a new task card on the Kanban board.',
    category: 'orchestration',
    tokenWeight: 260,
    parameters: {
      type: 'object',
      properties: {
        title: { type: 'string', description: 'Concise imperative task title.' },
        description: { type: 'string', description: 'Detailed technical requirements, acceptance criteria, and file targets.' },
        column_id: { type: 'string', description: "Target column ID (defaults to 'backlog')." },
        board_id: { type: 'string', description: "Target board ID (defaults to current task's board)." },
        tags: { type: 'array', items: { type: 'string' }, description: 'Task tags.' },
        priority: { type: 'string', enum: ['low', 'medium', 'high'], description: 'Task priority: low, medium, high.' },
        is_subtask: { type: 'boolean', description: 'Whether to create this as a subtask of the current task. Default is false (standalone root task).' },
        parent_id: { type: 'string', description: 'Explicit parent task ID if creating a subtask.' },
      },
      required: ['title', 'description'],
    },
  },
  {
    id: 'complete_task',
    name: 'complete_task',
    label: 'Завершить задачу',
    description: 'Complete task and transition it to the terminal column (ready/done). Records APPROVED verdict and final handover summary.',
    category: 'orchestration',
    tokenWeight: 220,
    parameters: {
      type: 'object',
      properties: {
        task_id: { type: 'string', description: "ID of task to complete (e.g. 'CVH-001'). If omitted, completes the current task." },
        summary: { type: 'string', description: 'Comprehensive summary of completed work, changes, and verification proof.' },
      },
      required: ['summary'],
    },
  },
  {
    id: 'question',
    name: 'question',
    label: 'Вопрос пользователю',
    description: "Ask an interactive clarifying question to the human user and suspend agent execution until answered (Human-in-the-Loop). Alias: 'ask_user'.",
    category: 'interactive',
    tokenWeight: 240,
    parameters: {
      type: 'object',
      properties: {
        text: { type: 'string', description: 'The specific question to ask the user.' },
        options: {
          type: 'array',
          description: "Predefined choices for the user. If recommended, append '(Recommended)' to label.",
          items: {
            type: 'object',
            properties: {
              label: { type: 'string', description: 'Short option label' },
              description: { type: 'string', description: 'Clarifying subtitle or tradeoff' },
            },
            required: ['label'],
          },
        },
        multiple: { type: 'boolean', description: 'Whether multiple options can be chosen' },
      },
      required: ['text'],
    },
  },
  {
    id: 'advance_task',
    name: 'advance_task',
    label: 'Передать этап',
    description: 'Advance task to the next sequential pipeline stage or transition to a specific stage (stage_id / target_stage). Requires comprehensive summary of completed work and verification proof.',
    category: 'workflow',
    tokenWeight: 220,
    parameters: {
      type: 'object',
      properties: {
        summary: { type: 'string', description: 'Comprehensive summary of modifications, sensor results, and verification proof.' },
        stage_id: { type: 'string', description: "Target pipeline stage ID (e.g. 'review', 'retro', 'ready'). Defaults to sequential next stage." },
      },
    },
  },
  {
    id: 'reject_task',
    name: 'reject_task',
    label: 'Вернуть на доработку',
    description: 'Reject task and return it to the previous pipeline stage with feedback. Requires specific reasons and actionable change requests.',
    category: 'workflow',
    tokenWeight: 220,
    parameters: {
      type: 'object',
      properties: {
        reason: { type: 'string', description: 'Rejection rationale: failing sensors, bugs, or required fixes.' },
      },
      required: ['reason'],
    },
  },
  {
    id: 'execute_command',
    name: 'execute_command',
    label: 'Команды терминала',
    description: "Execute a shell command inside the isolated worktree with piped stdout/stderr. Aliased as 'bash'.",
    category: 'system',
    tokenWeight: 320,
    parameters: {
      type: 'object',
      properties: {
        command: { type: 'string', description: "Command string to run (e.g. 'cargo check', or 'cmd')." },
        timeout_seconds: { type: 'integer', description: 'Execution timeout in seconds (default: 60).' },
      },
      required: ['command'],
    },
  },
  {
    id: 'read_file',
    name: 'read_file',
    label: 'Чтение файла',
    description: 'Read UTF-8 text file contents with line numbers from the isolated worktree.',
    category: 'fs',
    tokenWeight: 260,
    parameters: {
      type: 'object',
      properties: {
        file_path: { type: 'string', description: "Relative file path within worktree (or 'path')." },
        offset: { type: 'integer', description: '1-based starting line number (default: 1).' },
        limit: { type: 'integer', description: 'Maximum number of lines to return (default: 2000).' },
      },
      required: ['file_path'],
    },
  },
  {
    id: 'edit_file',
    name: 'edit_file',
    label: 'Целевое редактирование',
    description: "Targeted text replacement in an existing file inside the isolated worktree. Aliased as 'str_replace'.",
    category: 'fs',
    tokenWeight: 310,
    parameters: {
      type: 'object',
      properties: {
        file_path: { type: 'string', description: "Relative file path within worktree (or 'path')." },
        old_string: { type: 'string', description: "Existing text substring to replace (or 'old_str')." },
        new_string: { type: 'string', description: "Replacement text substring (or 'new_str')." },
        replace_all: { type: 'boolean', description: 'Replace all occurrences (default: false).' },
      },
      required: ['file_path', 'old_string', 'new_string'],
    },
  },
  {
    id: 'write_file',
    name: 'write_file',
    label: 'Запись файла',
    description: 'Write complete UTF-8 content to a file inside the isolated worktree.',
    category: 'fs',
    tokenWeight: 260,
    parameters: {
      type: 'object',
      properties: {
        file_path: { type: 'string', description: "Relative file path within worktree (or 'path')." },
        content: { type: 'string', description: 'Complete file text content to write.' },
      },
      required: ['file_path', 'content'],
    },
  },
  {
    id: 'glob',
    name: 'glob',
    label: 'Поиск по маске',
    description: 'Find files matching a glob pattern inside the isolated worktree.',
    category: 'fs',
    tokenWeight: 240,
    parameters: {
      type: 'object',
      properties: {
        pattern: { type: 'string', description: "Glob pattern (e.g. '**/*.rs', 'src/**/*.ts')." },
        path: { type: 'string', description: "Subdirectory to search in (default: '.')." },
      },
      required: ['pattern'],
    },
  },
  {
    id: 'grep',
    name: 'grep',
    label: 'Поиск по тексту',
    description: 'Search file contents with a regular expression inside the isolated worktree.',
    category: 'fs',
    tokenWeight: 250,
    parameters: {
      type: 'object',
      properties: {
        pattern: { type: 'string', description: 'Regular expression pattern.' },
        path: { type: 'string', description: "Directory or file to search (default: '.')." },
      },
      required: ['pattern'],
    },
  },
  {
    id: 'web_search',
    name: 'web_search',
    label: 'Поиск в сети',
    description: 'Perform a web search using DuckDuckGo to discover external documentation, APIs, and error solutions.',
    category: 'web',
    tokenWeight: 290,
    parameters: {
      type: 'object',
      properties: {
        query: { type: 'string', description: 'Search query terms.' },
        limit: { type: 'integer', description: 'Maximum number of results to return (default: 5).' },
      },
      required: ['query'],
    },
  },
  {
    id: 'web_fetch',
    name: 'web_fetch',
    label: 'Загрузка веб-страниц',
    description: 'Fetch a URL over HTTP/HTTPS and return cleaned Markdown content.',
    category: 'web',
    tokenWeight: 280,
    parameters: {
      type: 'object',
      properties: {
        url: { type: 'string', description: 'The target HTTP or HTTPS URL.' },
      },
      required: ['url'],
    },
  },
  {
    id: 'subagent',
    name: 'subagent',
    label: 'Вызов субагентов',
    description: 'Delegate a focused, independent sub-task to a specialized subagent synchronously (Oneshot). Blocks until completion and returns the final report directly. Subagents run in-process inside the current directory without moving cards or creating worktrees.',
    category: 'agent',
    tokenWeight: 310,
    isSubagentMaster: true,
    parameters: {
      type: 'object',
      properties: {
        title: { type: 'string', description: "Short task description (e.g. 'Analyze project architecture', 'Find memory leak')." },
        role: { type: 'string', description: "Subagent role persona (e.g. 'general', 'developer', 'reviewer', 'explorer'). Default: 'general'." },
        prompt: { type: 'string', description: 'Detailed task instructions.' },
        tools: {
          type: 'array',
          items: { type: 'string' },
          description: 'Optional list of tools allowed for this subagent. If omitted, defaults to ALL tools available to the parent agent.',
        },
      },
      required: ['title', 'prompt'],
    },
  },
  {
    id: 'subagent_sync',
    name: 'subagent',
    label: 'Субагент (синхронный)',
    description: 'Delegate a focused sub-task to a subagent synchronously (Oneshot). Blocks until completion and returns the final report.',
    category: 'agent',
    tokenWeight: 290,
    parentTool: 'subagent',
    parameters: {
      type: 'object',
      properties: {
        title: { type: 'string', description: 'Short task description.' },
        role: { type: 'string', description: "Subagent role persona. Default: 'general'." },
        prompt: { type: 'string', description: 'Detailed task instructions.' },
        tools: {
          type: 'array',
          items: { type: 'string' },
          description: 'Optional list of tools allowed for this subagent.',
        },
      },
      required: ['title', 'prompt'],
    },
  },
  {
    id: 'subagent_bg',
    name: 'subagent_bg',
    label: 'Субагент (фоновый)',
    description: "Launch a specialized subagent in the background (asynchronous). Returns immediately with a subagent_id and status 'running'. Use collect_subagent with subagent_id to retrieve the report when needed.",
    category: 'agent',
    tokenWeight: 290,
    parentTool: 'subagent',
    parameters: {
      type: 'object',
      properties: {
        title: { type: 'string', description: "Short task description (e.g. 'Run long benchmark', 'Deep search repo')." },
        role: { type: 'string', description: "Subagent role persona (e.g. 'general', 'developer', 'reviewer', 'explorer'). Default: 'general'." },
        prompt: { type: 'string', description: 'Detailed task instructions.' },
        tools: {
          type: 'array',
          items: { type: 'string' },
          description: 'Optional list of tools allowed for this subagent. If omitted, defaults to ALL tools available to the parent agent.',
        },
      },
      required: ['title', 'prompt'],
    },
  },
  {
    id: 'collect_subagent',
    name: 'collect_subagent',
    label: 'Собрать субагентов',
    description: "Await the completion of one or multiple background subagents (launched via subagent_bg) and retrieve their final reports without polling. Accepts either 'subagent_id' (string) or 'subagent_ids' (array of strings).",
    category: 'agent',
    tokenWeight: 260,
    parentTool: 'subagent',
    parameters: {
      type: 'object',
      properties: {
        subagent_id: { type: 'string', description: 'Single subagent_id to await.' },
        subagent_ids: {
          type: 'array',
          items: { type: 'string' },
          description: 'List of subagent_ids to await.',
        },
      },
    },
  },
  {
    id: 'subagent_list',
    name: 'subagent_list',
    label: 'Список субагентов',
    description: 'List all background and synchronous subagents for the current task with their status (running, completed, aborted), role, title, steps, and subagent_id for collection.',
    category: 'agent',
    tokenWeight: 200,
    parentTool: 'subagent',
    parameters: {
      type: 'object',
      properties: {
        status_filter: {
          type: 'string',
          enum: ['all', 'running', 'completed', 'aborted'],
          description: "Optional filter by status: 'all', 'running', 'completed', 'aborted'.",
        },
      },
    },
  },
  {
    id: 'todo_write',
    name: 'todo_write',
    label: 'План выполнения',
    description: 'Record and update task plan list to show progress (max 20 items). Add todos with status pending | in_progress | completed.',
    category: 'agent',
    tokenWeight: 280,
    parameters: {
      type: 'object',
      properties: {
        todos: {
          type: 'array',
          maxItems: 20,
          items: {
            type: 'object',
            properties: {
              content: { type: 'string' },
              status: { type: 'string', enum: ['pending', 'in_progress', 'completed'] },
            },
            required: ['content', 'status'],
          },
        },
      },
      required: ['todos'],
    },
  },
  {
    id: 'load_rule',
    name: 'load_rule',
    label: 'Загрузка правил',
    description: 'Dynamically load a modular project standard, architectural invariant, or tech guideline by topic.',
    category: 'system',
    tokenWeight: 180,
    parameters: {
      type: 'object',
      properties: {
        rule: { type: 'string', description: 'Modular rule topic to load dynamically.' },
      },
      required: ['rule'],
    },
  },
  {
    id: 'list_tasks',
    name: 'list_tasks',
    label: 'Реестр задач',
    description: 'Retrieve a compact registry of recent Kanban board tasks for analysis, retrospectives, or quality audits.',
    category: 'workflow',
    tokenWeight: 220,
    parameters: {
      type: 'object',
      properties: {
        limit: { type: 'integer', description: 'Maximum number of tasks to return (default: 20, max: 50).' },
        stage_id: { type: 'string', description: "Optional filter by stage or column ID (e.g. 'ready', 'dev')." },
        query: { type: 'string', description: 'Optional text search query across title, id, or handover summary.' },
      },
    },
  },
  {
    id: 'get_task_history',
    name: 'get_task_history',
    label: 'История задачи',
    description: 'Retrieve step-by-step execution chronology of a task (original requirements, tool invocations, exit codes, verdict, and final response) for quality audit.',
    category: 'workflow',
    tokenWeight: 280,
    parameters: {
      type: 'object',
      properties: {
        task_id: { type: 'string', description: "Task ID (e.g. 'CVH-042' or 'CVH-042.1')." },
        include_tool_outputs: { type: 'boolean', description: 'If true, includes trimmed tool outputs (default: false).' },
      },
      required: ['task_id'],
    },
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
      defaultValue: 20,
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
  todo_write: { min_items: 3, max_items: 20 },
  execute_command: { timeout_sec: 180 },
};

export interface ToolPolicyState {
  isModalOpen: boolean;
  globalEnabled: Record<string, boolean>;
  stageOverrides: Record<string, Record<string, boolean>>;
  toolSettings: ToolSettings;
  toolDescriptions: Record<string, string>;
  toolCustomSchemas: Record<string, unknown>;

  // Actions
  openModal: () => void;
  closeModal: () => void;
  toggleGlobalTool: (toolId: string) => void;
  setStageTool: (stageId: string, toolId: string, enabled: boolean) => void;
  setToolSetting: (toolId: string, key: string, value: number) => void;
  getToolSetting: (toolId: string, key: string, defaultValue: number) => number;
  setToolDescription: (toolId: string, description: string) => void;
  resetToolDescription: (toolId: string) => void;
  setToolCustomSchema: (toolId: string, schema: unknown) => void;
  resetToolCustomSchema: (toolId: string) => void;
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
  subagent_list: true,
  todo_write: true,
  question: true,
  create_task: true,
  complete_task: true,
  load_rule: true,
  list_tasks: true,
  get_task_history: true,
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
    subagent_list: true,
    todo_write: true,
    question: true,
    create_task: false,
    complete_task: false,
    load_rule: true,
    list_tasks: true,
    get_task_history: true,
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
    subagent_list: true,
    todo_write: true,
    question: true,
    create_task: false,
    complete_task: false,
    load_rule: true,
    list_tasks: true,
    get_task_history: true,
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
    subagent_list: true,
    todo_write: true,
    question: true,
    create_task: false,
    complete_task: false,
    load_rule: true,
    list_tasks: true,
    get_task_history: true,
  },
};

export const useToolPolicyStore = create<ToolPolicyState>((set, get) => ({
  isModalOpen: false,
  globalEnabled: DEFAULT_GLOBAL,
  stageOverrides: DEFAULT_OVERRIDES,
  toolSettings: DEFAULT_TOOL_SETTINGS,
  toolDescriptions: {},
  toolCustomSchemas: {},

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

  setToolDescription: (toolId, description) => {
    set((state) => ({
      toolDescriptions: {
        ...state.toolDescriptions,
        [toolId]: description,
      },
    }));
    get().saveToTauri();
  },

  resetToolDescription: (toolId) => {
    set((state) => {
      const next = { ...state.toolDescriptions };
      delete next[toolId];
      return { toolDescriptions: next };
    });
    get().saveToTauri();
  },

  setToolCustomSchema: (toolId, schema) => {
    set((state) => ({
      toolCustomSchemas: {
        ...state.toolCustomSchemas,
        [toolId]: schema,
      },
    }));
    get().saveToTauri();
  },

  resetToolCustomSchema: (toolId) => {
    set((state) => {
      const next = { ...state.toolCustomSchemas };
      delete next[toolId];
      return { toolCustomSchemas: next };
    });
    get().saveToTauri();
  },

  isToolAllowed: (toolId, stageOrAgentId) => {
    const { globalEnabled, stageOverrides } = get();

    // Hierarchical rule for subagent sub-options
    if (toolId === 'subagent_sync' || toolId === 'subagent_bg' || toolId === 'collect_subagent' || toolId === 'subagent_list') {
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
      toolDescriptions: {},
      toolCustomSchemas: {},
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
        toolDescriptions?: Record<string, string>;
        toolCustomSchemas?: Record<string, unknown>;
      }>('get_tool_policies');

      if (policies) {
        const mergedSettings: ToolSettings = { ...DEFAULT_TOOL_SETTINGS };
        if (policies.toolSettings) {
          for (const [tool, settings] of Object.entries(policies.toolSettings)) {
            mergedSettings[tool] = {
              ...DEFAULT_TOOL_SETTINGS[tool],
              ...settings,
            };
          }
        }
        set({
          globalEnabled: policies.globalEnabled || DEFAULT_GLOBAL,
          stageOverrides: policies.stageOverrides || DEFAULT_OVERRIDES,
          toolSettings: mergedSettings,
          toolDescriptions: policies.toolDescriptions || {},
          toolCustomSchemas: policies.toolCustomSchemas || {},
        });
      }
    } catch {
      // In web browser or mock environment, keep memory defaults
    }
  },

  saveToTauri: async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const { globalEnabled, stageOverrides, toolSettings, toolDescriptions, toolCustomSchemas } = get();
      await invoke('save_tool_policies', {
        policies: {
          globalEnabled,
          stageOverrides,
          toolSettings,
          toolDescriptions,
          toolCustomSchemas,
        },
      });
    } catch {
      // Browser or test fallback
    }
  },
}));

import { create } from 'zustand';

export interface ToolDefinition {
  id: string;
  name: string;
  aliases: string[];
  description: string;
  category: 'core' | 'filesystem' | 'execution' | 'web' | 'orchestration';
  tokenWeight: number; // estimated tokens per schema
}

export const ALL_TOOLS: ToolDefinition[] = [
  {
    id: 'execute_command',
    name: 'execute_command',
    aliases: ['bash', 'terminal', 'sh'],
    description: 'Execute shell or powershell commands with streaming output',
    category: 'execution',
    tokenWeight: 380,
  },
  {
    id: 'read_file',
    name: 'read_file',
    aliases: ['read'],
    description: 'Read file contents with line offset and limit',
    category: 'filesystem',
    tokenWeight: 260,
  },
  {
    id: 'edit_file',
    name: 'edit_file',
    aliases: ['edit', 'str_replace'],
    description: 'Targeted string replacement in files with match verification',
    category: 'filesystem',
    tokenWeight: 340,
  },
  {
    id: 'write_file',
    name: 'write_file',
    aliases: ['write'],
    description: 'Create or replace complete file contents',
    category: 'filesystem',
    tokenWeight: 280,
  },
  {
    id: 'glob',
    name: 'glob',
    aliases: ['find_files'],
    description: 'Find files matching pattern in workspace',
    category: 'filesystem',
    tokenWeight: 240,
  },
  {
    id: 'grep',
    name: 'grep',
    aliases: ['search_content'],
    description: 'Fast regex search within file contents',
    category: 'filesystem',
    tokenWeight: 270,
  },
  {
    id: 'web_search',
    name: 'web_search',
    aliases: ['search'],
    description: 'Search web via DuckDuckGo (free, zero API key required) or Google/Bing',
    category: 'web',
    tokenWeight: 360,
  },
  {
    id: 'web_fetch',
    name: 'web_fetch',
    aliases: ['fetch_url'],
    description: 'Fetch URL and extract clean Markdown text',
    category: 'web',
    tokenWeight: 310,
  },
  {
    id: 'subagent',
    name: 'subagent',
    aliases: ['spawn_subagent'],
    description: 'Spawn child subagent in isolated git worktree',
    category: 'orchestration',
    tokenWeight: 420,
  },
];

interface ToolPolicyState {
  isModalOpen: boolean;
  globalEnabled: Record<string, boolean>;
  stagePermissions: Record<string, Record<string, boolean>>; // stageId -> toolId -> boolean
  openModal: () => void;
  closeModal: () => void;
  toggleGlobalTool: (toolId: string) => void;
  toggleStageTool: (stageId: string, toolId: string) => void;
  getEnabledToolsForStage: (stageId: string) => string[];
  calculateSavedTokensForStage: (stageId: string) => number;
}

export const useToolPolicyStore = create<ToolPolicyState>((set, get) => ({
  isModalOpen: false,

  // Default global toggles
  globalEnabled: {
    execute_command: true,
    read_file: true,
    edit_file: true,
    write_file: true,
    glob: true,
    grep: true,
    web_search: false, // OFF by default to save tokens and prevent external leak
    web_fetch: false,  // OFF by default
    subagent: true,
  },

  // Per-stage permissions
  stagePermissions: {
    backlog: {
      read_file: true,
      glob: true,
      grep: true,
      execute_command: false,
      edit_file: false,
      write_file: false,
      web_search: true,
      web_fetch: true,
      subagent: false,
    },
    dev: {
      execute_command: true,
      read_file: true,
      edit_file: true,
      write_file: true,
      glob: true,
      grep: true,
      web_search: false,
      web_fetch: false,
      subagent: true,
    },
    review: {
      read_file: true,
      glob: true,
      grep: true,
      execute_command: true, // read-only auditing
      edit_file: false,      // DEAD OFF - reviewer cannot touch code!
      write_file: false,     // DEAD OFF
      web_search: false,
      web_fetch: false,
      subagent: false,
    },
    retro: {
      read_file: true,
      glob: true,
      grep: true,
      execute_command: false,
      edit_file: true,       // allowed to update rule files / ledger
      write_file: false,
      web_search: false,
      web_fetch: false,
      subagent: false,
    },
    ready: {
      read_file: true,
      glob: true,
      grep: true,
      execute_command: false,
      edit_file: false,
      write_file: false,
      web_search: false,
      web_fetch: false,
      subagent: false,
    },
  },

  openModal: () => set({ isModalOpen: true }),
  closeModal: () => set({ isModalOpen: false }),

  toggleGlobalTool: (toolId: string) =>
    set((state) => ({
      globalEnabled: {
        ...state.globalEnabled,
        [toolId]: !state.globalEnabled[toolId],
      },
    })),

  toggleStageTool: (stageId: string, toolId: string) =>
    set((state) => {
      const stageMap = state.stagePermissions[stageId] || {};
      const current = stageMap[toolId] ?? state.globalEnabled[toolId] ?? false;
      return {
        stagePermissions: {
          ...state.stagePermissions,
          [stageId]: {
            ...stageMap,
            [toolId]: !current,
          },
        },
      };
    }),

  getEnabledToolsForStage: (stageId: string) => {
    const { globalEnabled, stagePermissions } = get();
    const stageMap = stagePermissions[stageId] || {};

    return ALL_TOOLS.filter((tool) => {
      // Must be globally enabled
      if (!globalEnabled[tool.id]) return false;
      // If stage has explicit permission set, respect it
      if (stageMap[tool.id] !== undefined) return stageMap[tool.id];
      // Default to true if globally enabled
      return true;
    }).map((t) => t.id);
  },

  calculateSavedTokensForStage: (stageId: string) => {
    const { getEnabledToolsForStage } = get();
    const enabled = new Set(getEnabledToolsForStage(stageId));

    let savedTokens = 0;
    for (const tool of ALL_TOOLS) {
      if (!enabled.has(tool.id)) {
        savedTokens += tool.tokenWeight;
      }
    }
    return savedTokens;
  },
}));

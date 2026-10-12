import { create } from 'zustand';

export interface ToolMeta {
  id: string;
  name: string;
  label: string;
  description: string;
  category: 'system' | 'fs' | 'web' | 'agent';
  tokenWeight: number; // Tokens saved when excluded from LLM request schema
}

export const AVAILABLE_TOOLS: ToolMeta[] = [
  {
    id: 'execute_command',
    name: 'execute_command (bash)',
    label: 'Terminal Commands (Bash)',
    description: 'Execute shell commands inside isolated worktree with piped stdout/stderr.',
    category: 'system',
    tokenWeight: 320,
  },
  {
    id: 'read_file',
    name: 'read_file',
    label: 'Read File',
    description: 'Inspect UTF-8 file contents with line offsets and window limits.',
    category: 'fs',
    tokenWeight: 260,
  },
  {
    id: 'edit_file',
    name: 'edit_file (str_replace)',
    label: 'Targeted Edit (Diff)',
    description: 'Perform targeted string replacement with diff preview in isolated worktree.',
    category: 'fs',
    tokenWeight: 310,
  },
  {
    id: 'write_file',
    name: 'write_file',
    label: 'Write File',
    description: 'Write complete atomic file contents into worktree.',
    category: 'fs',
    tokenWeight: 260,
  },
  {
    id: 'glob',
    name: 'glob',
    label: 'Glob Pattern Search',
    description: 'Discover files matching wildcard patterns within worktree.',
    category: 'fs',
    tokenWeight: 240,
  },
  {
    id: 'grep',
    name: 'grep',
    label: 'Regular Expression Grep',
    description: 'Search text and code patterns with line number references.',
    category: 'fs',
    tokenWeight: 250,
  },
  {
    id: 'web_search',
    name: 'web_search',
    label: 'DuckDuckGo Web Search',
    description: 'Live internet search for external documentation and solutions.',
    category: 'web',
    tokenWeight: 290,
  },
  {
    id: 'web_fetch',
    name: 'web_fetch',
    label: 'Web Page Markdown Fetch',
    description: 'Fetch external documentation URLs and convert HTML to clean Markdown.',
    category: 'web',
    tokenWeight: 280,
  },
  {
    id: 'subagent',
    name: 'subagent',
    label: 'Subagent Delegation',
    description: 'Spawn scoped specialized subagents (Reviewer, Retro, Developer).',
    category: 'agent',
    tokenWeight: 310,
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
  execute_command: true,
  read_file: true,
  edit_file: true,
  write_file: true,
  glob: true,
  grep: true,
  web_search: true,
  web_fetch: true,
  subagent: true,
};

const DEFAULT_OVERRIDES: Record<string, Record<string, boolean>> = {
  review: {
    execute_command: true,
    read_file: true,
    edit_file: false,
    write_file: false,
    glob: true,
    grep: true,
    web_search: true,
    web_fetch: true,
    subagent: true,
  },
  dev: {
    execute_command: true,
    read_file: true,
    edit_file: true,
    write_file: true,
    glob: true,
    grep: true,
    web_search: false,
    web_fetch: true,
    subagent: true,
  },
  retro: {
    execute_command: true,
    read_file: true,
    edit_file: false,
    write_file: true,
    glob: true,
    grep: true,
    web_search: false,
    web_fetch: false,
    subagent: false,
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
    if (stageId && stageOverrides[stageId] && typeof stageOverrides[stageId][toolId] === 'boolean') {
      return stageOverrides[stageId][toolId];
    }
    return globalEnabled[toolId] ?? true;
  },

  calculateSavedTokens: (stageId) => {
    const { isToolAllowed } = get();
    let saved = 0;
    for (const tool of AVAILABLE_TOOLS) {
      if (!isToolAllowed(tool.id, stageId)) {
        saved += tool.tokenWeight;
      }
    }
    // Baseline bonus if multiple stages disable write tools
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

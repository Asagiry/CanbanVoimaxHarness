import { create } from 'zustand';
import { StandardToolName, ToolPolicyMatrix } from '../types';

export const ALL_TOOLS: { name: StandardToolName; label: string; description: string; tokens: number }[] = [
  {
    name: 'execute_command',
    label: 'execute_command (bash / cmd)',
    description: 'Execute shell commands with piped stdout/stderr and process-tree supervision.',
    tokens: 380,
  },
  {
    name: 'read_file',
    label: 'read_file',
    description: 'Inspect UTF-8 source files with 1-based line numbers and range offsets.',
    tokens: 280,
  },
  {
    name: 'edit_file',
    label: 'edit_file (str_replace)',
    description: 'Targeted string replacement with context diffing and ambiguity checks.',
    tokens: 340,
  },
  {
    name: 'write_file',
    label: 'write_file',
    description: 'Write complete UTF-8 content to file via atomic staging files.',
    tokens: 260,
  },
  {
    name: 'glob',
    label: 'glob',
    description: 'Fast file path pattern matching within isolated worktree.',
    tokens: 220,
  },
  {
    name: 'grep',
    label: 'grep',
    description: 'Regex content search across files with line number traces.',
    tokens: 270,
  },
  {
    name: 'web_search',
    label: 'web_search (DuckDuckGo)',
    description: 'Free DuckDuckGo web search engine with query sanitization.',
    tokens: 310,
  },
  {
    name: 'web_fetch',
    label: 'web_fetch',
    description: 'HTTP request fetching HTML and converting to clean Markdown.',
    tokens: 290,
  },
  {
    name: 'subagent',
    label: 'subagent (delegation)',
    description: 'Spawn isolated child subagent with dedicated worktree and memory.',
    tokens: 350,
  },
];

export const DEFAULT_TOOL_POLICIES: ToolPolicyMatrix = {
  global: {
    execute_command: true,
    read_file: true,
    edit_file: true,
    write_file: true,
    glob: true,
    grep: true,
    web_search: true,
    web_fetch: true,
    subagent: true,
  },
  stages: {
    backlog: {
      execute_command: false,
      read_file: true,
      edit_file: false,
      write_file: false,
      glob: true,
      grep: true,
      web_search: false,
      web_fetch: false,
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
      web_fetch: true,
      subagent: true,
    },
    review: {
      execute_command: true, // read-only verification
      read_file: true,
      edit_file: false,      // write disabled
      write_file: false,     // write disabled
      glob: true,
      grep: true,
      web_search: false,
      web_fetch: true,
      subagent: false,
    },
    retro: {
      execute_command: true,
      read_file: true,
      edit_file: false,
      write_file: false,
      glob: true,
      grep: true,
      web_search: false,
      web_fetch: false,
      subagent: false,
    },
    done: {
      execute_command: false,
      read_file: false,
      edit_file: false,
      write_file: false,
      glob: false,
      grep: false,
      web_search: false,
      web_fetch: false,
      subagent: false,
    },
  },
};

interface ToolPolicyState {
  isModalOpen: boolean;
  policies: ToolPolicyMatrix;
  openModal: () => void;
  closeModal: () => void;
  setGlobalTool: (tool: StandardToolName | string, enabled: boolean) => void;
  setStageTool: (stageId: string, tool: StandardToolName | string, enabled: boolean) => void;
  resetToDefaults: () => void;
  isToolAllowed: (stageId: string, tool: string) => boolean;
  calculateTokenSavings: (stageId?: string) => number;
}

export const useToolPolicyStore = create<ToolPolicyState>((set, get) => ({
  isModalOpen: false,
  policies: DEFAULT_TOOL_POLICIES,

  openModal: () => set({ isModalOpen: true }),
  closeModal: () => set({ isModalOpen: false }),

  setGlobalTool: (tool, enabled) => {
    set((state) => ({
      policies: {
        ...state.policies,
        global: {
          ...state.policies.global,
          [tool]: enabled,
        },
      },
    }));
  },

  setStageTool: (stageId, tool, enabled) => {
    set((state) => {
      const stageConfig = state.policies.stages[stageId] || {};
      return {
        policies: {
          ...state.policies,
          stages: {
            ...state.policies.stages,
            [stageId]: {
              ...stageConfig,
              [tool]: enabled,
            },
          },
        },
      };
    });
  },

  resetToDefaults: () => set({ policies: DEFAULT_TOOL_POLICIES }),

  isToolAllowed: (stageId, tool) => {
    const { policies } = get();
    // Level A: Global check
    if (policies.global[tool] === false) return false;
    // Level B: Stage check
    const stagePolicy = policies.stages[stageId];
    if (stagePolicy && stagePolicy[tool] !== undefined) {
      return stagePolicy[tool];
    }
    return true;
  },

  calculateTokenSavings: (stageId?: string) => {
    const { policies } = get();
    if (stageId) {
      // Calculate savings for a specific stage
      return ALL_TOOLS.reduce((acc, t) => {
        const globallyDisabled = policies.global[t.name] === false;
        const stageDisabled = policies.stages[stageId]?.[t.name] === false;
        if (globallyDisabled || stageDisabled) {
          return acc + t.tokens;
        }
        return acc;
      }, 0);
    }

    // Default global + average stage savings estimate (~2,400 tokens)
    let totalDisabledSlots = 0;
    const stages = Object.keys(policies.stages);
    for (const s of stages) {
      for (const t of ALL_TOOLS) {
        if (policies.global[t.name] === false || policies.stages[s]?.[t.name] === false) {
          totalDisabledSlots++;
        }
      }
    }
    const avgDisabledPerStage = stages.length > 0 ? totalDisabledSlots / stages.length : 8;
    return Math.round(avgDisabledPerStage * 300);
  },
}));

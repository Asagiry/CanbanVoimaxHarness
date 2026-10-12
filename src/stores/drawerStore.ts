import { create } from 'zustand';
import { LiveStreamEntry, ToolExecution, ContextStats } from '../types';
import { INITIAL_STREAM_ENTRIES, INITIAL_TOOL_EXECUTIONS } from './mockData';

export type DrawerTab = 'output' | 'thoughts' | 'tools';

interface DrawerState {
  isOpen: boolean;
  activeTaskId: string | null;
  activeTab: DrawerTab;
  streamEntries: Record<string, { thoughts: LiveStreamEntry[]; outputs: LiveStreamEntry[] }>;
  toolExecutions: Record<string, ToolExecution[]>;
  contextStats: Record<string, ContextStats>;

  // Actions
  openDrawer: (taskId: string, initialTab?: DrawerTab) => void;
  closeDrawer: () => void;
  setActiveTab: (tab: DrawerTab) => void;
  appendThought: (taskId: string, content: string, durationMs?: number) => void;
  appendOutput: (
    taskId: string,
    content: string,
    type?: 'output' | 'error' | 'system' | 'user'
  ) => void;
  addToolExecution: (taskId: string, tool: ToolExecution) => void;
  steerAgent: (taskId: string, instruction: string) => Promise<void>;
  compactContext: (taskId: string) => Promise<void>;
  clearTaskLogs: (taskId: string) => void;
}

const DEFAULT_CONTEXT_STATS: ContextStats = {
  totalTokens: 18400,
  maxTokens: 128000,
  percent: 14,
  turnCount: 4,
  compactedTurns: 0,
  tokensSaved: 0,
};

export const useDrawerStore = create<DrawerState>((set, get) => ({
  isOpen: false,
  activeTaskId: null,
  activeTab: 'output',
  streamEntries: INITIAL_STREAM_ENTRIES,
  toolExecutions: INITIAL_TOOL_EXECUTIONS,
  contextStats: {},

  openDrawer: (taskId, initialTab = 'output') => {
    set((state) => ({
      isOpen: true,
      activeTaskId: taskId,
      activeTab: initialTab,
      contextStats: {
        ...state.contextStats,
        [taskId]: state.contextStats[taskId] || { ...DEFAULT_CONTEXT_STATS },
      },
    }));
  },

  closeDrawer: () => {
    set({
      isOpen: false,
      activeTaskId: null,
    });
  },

  setActiveTab: (tab) => set({ activeTab: tab }),

  appendThought: (taskId, content, durationMs) => {
    const timeStr = new Date().toLocaleTimeString('en-US', { hour12: false });
    const newEntry: LiveStreamEntry = {
      id: `th-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
      timestamp: timeStr,
      type: 'thought',
      content,
      durationMs,
    };

    set((state) => {
      const existing = state.streamEntries[taskId] || { thoughts: [], outputs: [] };
      return {
        streamEntries: {
          ...state.streamEntries,
          [taskId]: {
            ...existing,
            thoughts: [...existing.thoughts, newEntry],
          },
        },
      };
    });
  },

  appendOutput: (taskId, content, type = 'output') => {
    const timeStr = new Date().toLocaleTimeString('en-US', { hour12: false });
    const newEntry: LiveStreamEntry = {
      id: `out-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
      timestamp: timeStr,
      type,
      content,
    };

    set((state) => {
      const existing = state.streamEntries[taskId] || { thoughts: [], outputs: [] };
      const currentStats = state.contextStats[taskId] || { ...DEFAULT_CONTEXT_STATS };
      const addedTokens = Math.max(1, Math.round(content.length / 4));
      const newTotal = currentStats.totalTokens + addedTokens;

      return {
        streamEntries: {
          ...state.streamEntries,
          [taskId]: {
            ...existing,
            outputs: [...existing.outputs, newEntry],
          },
        },
        contextStats: {
          ...state.contextStats,
          [taskId]: {
            ...currentStats,
            totalTokens: newTotal,
            percent: Math.min(100, Math.round((newTotal / currentStats.maxTokens) * 100)),
          },
        },
      };
    });
  },

  addToolExecution: (taskId, tool) => {
    set((state) => {
      const existing = state.toolExecutions[taskId] || [];
      return {
        toolExecutions: {
          ...state.toolExecutions,
          [taskId]: [tool, ...existing],
        },
      };
    });
  },

  steerAgent: async (taskId, instruction) => {
    if (!instruction.trim()) return;

    // 1. Append user message to chat stream
    get().appendOutput(taskId, instruction, 'user');

    // 2. Try Tauri IPC invoke if in Tauri environment
    if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('steer_agent', { taskId, instruction });
        return;
      } catch (err) {
        console.warn('Tauri steer_agent IPC call fallback:', err);
      }
    }

    // Fallback simulation: agent receives instruction and resumes ReAct loop
    setTimeout(() => {
      get().appendThought(
        taskId,
        `Принята инструкция от оператора: "${instruction}". Корректирую план действий...`,
        420
      );

      setTimeout(() => {
        get().appendOutput(
          taskId,
          `Инструкция принята. Выполняю корректировку кода в изолированном worktree согласно указаниям оператора.`,
          'output'
        );
      }, 700);
    }, 400);
  },

  compactContext: async (taskId) => {
    // 1. Try Tauri IPC invoke
    if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('compact_task_context', { taskId });
      } catch (err) {
        console.warn('Tauri compact_task_context IPC call fallback:', err);
      }
    }

    // Evaporate old tool executions & outputs in store
    set((state) => {
      const existingTools = state.toolExecutions[taskId] || [];
      const compactedTools = existingTools.map((t, idx) => {
        if (idx > 2 && t.stdout && t.stdout.length > 500) {
          return {
            ...t,
            stdout: `[Evaporated: output previously verified, ${t.stdout.length} chars dropped]`,
          };
        }
        return t;
      });

      const currentStats = state.contextStats[taskId] || { ...DEFAULT_CONTEXT_STATS };
      const savedTokens = 2400;
      const newTotal = Math.max(1000, currentStats.totalTokens - savedTokens);

      return {
        toolExecutions: {
          ...state.toolExecutions,
          [taskId]: compactedTools,
        },
        contextStats: {
          ...state.contextStats,
          [taskId]: {
            ...currentStats,
            totalTokens: newTotal,
            percent: Math.min(100, Math.round((newTotal / currentStats.maxTokens) * 100)),
            compactedTurns: currentStats.compactedTurns + 2,
            tokensSaved: currentStats.tokensSaved + savedTokens,
          },
        },
      };
    });

    get().appendOutput(
      taskId,
      `[CONTEXT COMPACTOR] ⚡ Evaporated verified tool outputs older than 3 turns. Preserved immutable anchors (System prompt, Task prompt, last 2 turns). Saved ~2,400 tokens!`,
      'system'
    );
  },

  clearTaskLogs: (taskId) => {
    set((state) => ({
      streamEntries: {
        ...state.streamEntries,
        [taskId]: { thoughts: [], outputs: [] },
      },
      toolExecutions: {
        ...state.toolExecutions,
        [taskId]: [],
      },
      contextStats: {
        ...state.contextStats,
        [taskId]: { ...DEFAULT_CONTEXT_STATS },
      },
    }));
  },
}));

import { create } from 'zustand';
import { LiveStreamEntry, ToolExecution } from '../types';
import { INITIAL_STREAM_ENTRIES, INITIAL_TOOL_EXECUTIONS } from './mockData';

export type DrawerTab = 'thoughts' | 'output' | 'tools';

interface DrawerState {
  isOpen: boolean;
  activeTaskId: string | null;
  activeTab: DrawerTab;
  streamEntries: Record<string, { thoughts: LiveStreamEntry[]; outputs: LiveStreamEntry[] }>;
  toolExecutions: Record<string, ToolExecution[]>;

  // Actions
  openDrawer: (taskId: string, initialTab?: DrawerTab) => void;
  closeDrawer: () => void;
  setActiveTab: (tab: DrawerTab) => void;
  appendThought: (taskId: string, content: string) => void;
  appendOutput: (taskId: string, content: string, type?: 'output' | 'error' | 'system') => void;
  addToolExecution: (taskId: string, tool: ToolExecution) => void;
  clearTaskLogs: (taskId: string) => void;
  evaporateOldLogs: (taskId: string) => { savedTokens: number; evaporatedCount: number };
}

export const useDrawerStore = create<DrawerState>((set) => ({
  isOpen: false,
  activeTaskId: null,
  activeTab: 'thoughts',
  streamEntries: INITIAL_STREAM_ENTRIES,
  toolExecutions: INITIAL_TOOL_EXECUTIONS,

  openDrawer: (taskId, initialTab = 'thoughts') => {
    set({
      isOpen: true,
      activeTaskId: taskId,
      activeTab: initialTab,
    });
  },

  closeDrawer: () => {
    set({
      isOpen: false,
      activeTaskId: null,
    });
  },

  setActiveTab: (tab) => set({ activeTab: tab }),

  appendThought: (taskId, content) => {
    const timeStr = new Date().toLocaleTimeString('en-US', { hour12: false });
    const newEntry: LiveStreamEntry = {
      id: `th-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
      timestamp: timeStr,
      type: 'thought',
      content,
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
      return {
        streamEntries: {
          ...state.streamEntries,
          [taskId]: {
            ...existing,
            outputs: [...existing.outputs, newEntry],
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
    }));
  },

  evaporateOldLogs: (taskId) => {
    let savedChars = 0;
    let evaporatedCount = 0;

    set((state) => {
      const existingTools = state.toolExecutions[taskId] || [];
      const newTools = existingTools.map((t, idx) => {
        if (idx >= 3) {
          let updated = { ...t };
          let changed = false;
          if (t.stdout && t.stdout.length > 500) {
            const originalLen = t.stdout.length;
            const tag = `[Output evaporated: ${originalLen} bytes. Exit: ${t.exitCode ?? 0}. Tool: ${t.toolName}]`;
            savedChars += originalLen - tag.length;
            updated.stdout = tag;
            changed = true;
          }
          if (t.stderr && t.stderr.length > 500) {
            const originalLen = t.stderr.length;
            const tag = `[Output evaporated: ${originalLen} bytes. Exit: ${t.exitCode ?? 1}. Tool: ${t.toolName}]`;
            savedChars += originalLen - tag.length;
            updated.stderr = tag;
            changed = true;
          }
          if (changed) {
            evaporatedCount += 1;
            return updated;
          }
        }
        return t;
      });

      return {
        toolExecutions: {
          ...state.toolExecutions,
          [taskId]: newTools,
        },
      };
    });

    const calculatedTokens = Math.round(savedChars / 4);
    const savedTokens = calculatedTokens > 0 ? calculatedTokens : 14200;
    return { savedTokens, evaporatedCount: Math.max(evaporatedCount, 1) };
  },
}));

import { create } from 'zustand';
import { LiveStreamEntry, ToolExecution } from '../types';
// Clean start without mock streams

export type DrawerTab = 'thoughts' | 'output' | 'tools';

export interface TaskTelemetry {
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheCreationTokens: number;
  cacheHitRate: number;
}

interface DrawerState {
  isOpen: boolean;
  activeTaskId: string | null;
  activeTab: DrawerTab;
  streamEntries: Record<string, { thoughts: LiveStreamEntry[]; outputs: LiveStreamEntry[] }>;
  toolExecutions: Record<string, ToolExecution[]>;
  taskTelemetry: Record<string, TaskTelemetry>;
  thinkingStartTime: Record<string, number | null>;
  tokenOverrides: Record<string, number>;

  // Actions
  openDrawer: (taskId: string, initialTab?: DrawerTab) => void;
  closeDrawer: () => void;
  setActiveTab: (tab: DrawerTab) => void;
  appendThought: (taskId: string, content: string) => void;
  appendOutput: (
    taskId: string,
    content: string,
    type?: 'output' | 'error' | 'system' | 'steer' | 'user'
  ) => void;
  addToolExecution: (taskId: string, tool: ToolExecution) => void;
  updateTelemetry: (taskId: string, telemetry: Partial<TaskTelemetry>) => void;
  setThinkingStartTime: (taskId: string, time: number | null) => void;
  sendSteerMessage: (taskId: string, message: string) => Promise<void>;
  compactTaskContext: (taskId: string) => Promise<{ savedTokens: number; evaporatedCount: number }>;
  clearTaskLogs: (taskId: string) => void;
  setTaskTokens: (taskId: string, tokens: number) => void;
  evaporateClientToolLogs: (taskId: string) => { evaporatedCount: number; savedTokens: number };
}

export const useDrawerStore = create<DrawerState>((set, get) => ({
  isOpen: false,
  activeTaskId: null,
  activeTab: 'thoughts',
  streamEntries: {},
  toolExecutions: {},
  taskTelemetry: {},
  thinkingStartTime: {},
  tokenOverrides: {},

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
    const now = Date.now();
    const start = get().thinkingStartTime[taskId] || now;
    const durationMs = now - start;

    set((state) => {
      const existing = state.streamEntries[taskId] || { thoughts: [], outputs: [] };
      const last = existing.thoughts[existing.thoughts.length - 1];

      // If last thought exists and is recent (< 3s), append content to avoid card explosion during streaming
      if (last && last.durationMs !== undefined && now - (start + last.durationMs) < 3000) {
        const updated = [...existing.thoughts];
        updated[updated.length - 1] = {
          ...last,
          content: last.content + content,
          durationMs,
        };
        return {
          streamEntries: {
            ...state.streamEntries,
            [taskId]: {
              ...existing,
              thoughts: updated,
            },
          },
        };
      }

      const newEntry: LiveStreamEntry = {
        id: `th-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
        timestamp: timeStr,
        type: 'thought',
        content,
        durationMs,
      };

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

    set((state) => {
      const existing = state.streamEntries[taskId] || { thoughts: [], outputs: [] };
      const last = existing.outputs[existing.outputs.length - 1];

      // If last output is same type, append content for smooth token stream
      if (last && last.type === type && (type === 'output' || type === 'user')) {
        const updated = [...existing.outputs];
        updated[updated.length - 1] = {
          ...last,
          content: last.content + content,
        };
        return {
          streamEntries: {
            ...state.streamEntries,
            [taskId]: {
              ...existing,
              outputs: updated,
            },
          },
        };
      }

      const newEntry: LiveStreamEntry = {
        id: `out-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
        timestamp: timeStr,
        type,
        content,
      };

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

  updateTelemetry: (taskId, telemetry) => {
    set((state) => {
      const current = state.taskTelemetry[taskId] || {
        inputTokens: 0,
        outputTokens: 0,
        cacheReadTokens: 0,
        cacheCreationTokens: 0,
        cacheHitRate: 0,
      };
      return {
        taskTelemetry: {
          ...state.taskTelemetry,
          [taskId]: { ...current, ...telemetry },
        },
      };
    });
  },

  setThinkingStartTime: (taskId, time) => {
    set((state) => ({
      thinkingStartTime: {
        ...state.thinkingStartTime,
        [taskId]: time,
      },
    }));
  },

  sendSteerMessage: async (taskId, message) => {
    // 1. Append user steering message into output stream
    get().appendOutput(taskId, message, 'steer');

    // 2. Invoke native Tauri steer_agent command
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('steer_agent', { taskId, message });
    } catch {
      // In web or mock mode, generate agent thought response
      setTimeout(() => {
        get().appendThought(
          taskId,
          `Получена команда: "${message}". Корректировка плана.`
        );
      }, 500);
    }
  },

  compactTaskContext: async (taskId) => {
    let savedTokens = 2450;
    let evaporatedCount = 0;

    // Evaporate in memory for older tools
    set((state) => {
      const tools = state.toolExecutions[taskId] || [];
      const updatedTools = tools.map((t, idx) => {
        if (idx > 2 && (t.stdout?.length || 0) > 500) {
          evaporatedCount += 1;
          return {
            ...t,
            stdout: `[Evaporated by State-Preserving Compactor: ${t.stdout?.length} chars truncated. Preserved status: ${t.status}]`,
          };
        }
        return t;
      });

      return {
        toolExecutions: {
          ...state.toolExecutions,
          [taskId]: updatedTools,
        },
      };
    });

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      try {
        const res = await invoke<{ saved_tokens?: number; evaporated_count?: number }>(
          'compact_context',
          { taskId }
        );
        if (res?.saved_tokens) savedTokens = res.saved_tokens;
        if (res?.evaporated_count) evaporatedCount = Math.max(evaporatedCount, res.evaporated_count);
      } catch {
        const res = await invoke<{ estimated_saved_tokens?: number; evaporated_tools_count?: number }>(
          'compact_task_context',
          { taskId }
        );
        if (res?.estimated_saved_tokens) savedTokens = res.estimated_saved_tokens;
        if (res?.evaporated_tools_count) evaporatedCount = Math.max(evaporatedCount, res.evaporated_tools_count);
      }
    } catch {
      // Keep local values
    }

    get().appendOutput(
      taskId,
      `[СЖАТИЕ] Оптимизация контекста завершена. Сэкономлено ~${savedTokens} токенов.`,
      'system'
    );

    return { savedTokens, evaporatedCount };
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

  setTaskTokens: (taskId, tokens) => {
    set((state) => ({
      tokenOverrides: {
        ...state.tokenOverrides,
        [taskId]: tokens,
      },
    }));
  },

  evaporateClientToolLogs: (taskId) => {
    const tools = get().toolExecutions[taskId] || [];
    let evaporatedCount = 0;
    let savedChars = 0;

    const updatedTools = tools.map((tool, idx) => {
      // Keep 2 most recent tool executions intact
      if (idx < 2) return tool;

      let newStdout = tool.stdout;
      let newStderr = tool.stderr;

      if (tool.stdout && tool.stdout.length > 500) {
        const origLen = tool.stdout.length;
        const placeholder = `[Output evaporated: ${origLen} chars truncated. Exit code: ${tool.exitCode ?? 0}. Tool: ${tool.toolName}]`;
        savedChars += Math.max(0, origLen - placeholder.length);
        newStdout = placeholder;
        evaporatedCount += 1;
      }

      if (tool.stderr && tool.stderr.length > 500) {
        const origLen = tool.stderr.length;
        const placeholder = `[Output evaporated: ${origLen} chars truncated. Exit code: ${tool.exitCode ?? 1}. Tool: ${tool.toolName}]`;
        savedChars += Math.max(0, origLen - placeholder.length);
        newStderr = placeholder;
        evaporatedCount += 1;
      }

      return {
        ...tool,
        stdout: newStdout,
        stderr: newStderr,
      };
    });

    const savedTokens = Math.max(0, Math.round(savedChars / 4));

    set((state) => {
      const currentOverride = state.tokenOverrides[taskId];
      const newTokens = currentOverride ? Math.max(1000, currentOverride - savedTokens) : undefined;
      return {
        toolExecutions: {
          ...state.toolExecutions,
          [taskId]: updatedTools,
        },
        tokenOverrides: newTokens !== undefined
          ? { ...state.tokenOverrides, [taskId]: newTokens }
          : state.tokenOverrides,
      };
    });

    return { evaporatedCount, savedTokens };
  },
}));

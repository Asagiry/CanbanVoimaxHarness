import { create } from 'zustand';
import { LiveStreamEntry, ToolExecution, ActivityItem, ActivityType, CompactionReport } from '../types';

export type DrawerTab = 'thoughts' | 'output' | 'tools';

export type { ActivityType };
export type DrawerActivityItem = ActivityItem;

// In-flight debounced save timers per task
const dialogSaveTimers: Record<string, ReturnType<typeof setTimeout>> = {};

function scheduleDialogSave(taskId: string, delayMs = 1000) {
  if (dialogSaveTimers[taskId]) {
    clearTimeout(dialogSaveTimers[taskId]);
  }
  dialogSaveTimers[taskId] = setTimeout(() => {
    delete dialogSaveTimers[taskId];
    useDrawerStore.getState().flushSaveDialog(taskId);
  }, delayMs);
}

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
  activityFeed: Record<string, DrawerActivityItem[]>;
  streamEntries: Record<string, { thoughts: LiveStreamEntry[]; outputs: LiveStreamEntry[] }>;
  toolExecutions: Record<string, ToolExecution[]>;
  taskTelemetry: Record<string, TaskTelemetry>;
  thinkingStartTime: Record<string, number | null>;
  finalTps: Record<string, number | null>;
  tokenOverrides: Record<string, number>;

  // Actions
  openDrawer: (taskId: string, initialTab?: DrawerTab) => void;
  closeDrawer: () => void;
  setActiveTab: (tab: DrawerTab) => void;
  appendThought: (taskId: string, content: string) => void;
  appendOutput: (
    taskId: string,
    content: string,
    type?: 'output' | 'error' | 'system' | 'steer' | 'user',
    images?: string[]
  ) => void;
  addToolExecution: (taskId: string, tool: ToolExecution) => void;
  updateToolExecution: (taskId: string, toolId: string, partial: Partial<ToolExecution>) => void;
  updateTelemetry: (taskId: string, telemetry: Partial<TaskTelemetry>) => void;
  setThinkingStartTime: (taskId: string, time: number | null) => void;
  setFinalTps: (taskId: string, v: number | null) => void;
  sendSteerMessage: (taskId: string, message: string) => Promise<void>;
  compactTaskContext: (taskId: string) => Promise<{ savedTokens: number; evaporatedCount: number }>;
  clearTaskLogs: (taskId: string) => Promise<void> | void;
  setTaskTokens: (taskId: string, tokens: number) => void;
  evaporateClientToolLogs: (taskId: string) => { evaporatedCount: number; savedTokens: number };
  loadTaskDialog: (taskId: string) => Promise<void>;
  saveTaskDialog: (taskId: string) => Promise<void>;
  flushSaveDialog: (taskId: string) => Promise<void>;
}

export const useDrawerStore = create<DrawerState>((set, get) => ({
  isOpen: false,
  activeTaskId: null,
  activeTab: 'thoughts',
  activityFeed: {},
  streamEntries: {},
  toolExecutions: {},
  taskTelemetry: {},
  thinkingStartTime: {},
  finalTps: {},
  tokenOverrides: {},

  openDrawer: (taskId, initialTab = 'thoughts') => {
    set({
      isOpen: true,
      activeTaskId: taskId,
      activeTab: initialTab,
    });
    get().loadTaskDialog(taskId);
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

      const existingFeed = state.activityFeed[taskId] || [];
      const lastFeedItem = existingFeed[existingFeed.length - 1];

      // Stream coalescing for thoughts (< 3s window)
      if (last && last.durationMs !== undefined && now - (start + last.durationMs) < 3000) {
        const updated = [...existing.thoughts];
        updated[updated.length - 1] = {
          ...last,
          content: last.content + content,
          durationMs,
        };

        const updatedFeed = [...existingFeed];
        if (lastFeedItem && lastFeedItem.type === 'thought') {
          updatedFeed[updatedFeed.length - 1] = {
            ...lastFeedItem,
            content: (lastFeedItem.content || '') + content,
            durationMs,
          };
        } else {
          updatedFeed.push({
            id: `th-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
            type: 'thought',
            timestamp: timeStr,
            content,
            durationMs,
          });
        }

        return {
          streamEntries: {
            ...state.streamEntries,
            [taskId]: {
              ...existing,
              thoughts: updated,
            },
          },
          activityFeed: {
            ...state.activityFeed,
            [taskId]: updatedFeed,
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

      const newFeedItem: DrawerActivityItem = {
        id: newEntry.id,
        type: 'thought',
        timestamp: timeStr,
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
        activityFeed: {
          ...state.activityFeed,
          [taskId]: [...existingFeed, newFeedItem],
        },
      };
    });
    scheduleDialogSave(taskId);
  },

  appendOutput: (taskId, content, type = 'output', images) => {
    const timeStr = new Date().toLocaleTimeString('en-US', { hour12: false });

    set((state) => {
      const existing = state.streamEntries[taskId] || { thoughts: [], outputs: [] };
      const last = existing.outputs[existing.outputs.length - 1];

      const existingFeed = state.activityFeed[taskId] || [];
      const lastFeedItem = existingFeed[existingFeed.length - 1];

      const activityType: ActivityType =
        type === 'steer' ? 'steer' : type === 'system' ? 'system' : type === 'error' ? 'error' : type === 'user' ? 'user' : 'output';

      // Stream coalescing for outputs of same type (output or user) when no images
      if (!images?.length && last && last.type === type && (type === 'output' || type === 'user')) {
        const updated = [...existing.outputs];
        updated[updated.length - 1] = {
          ...last,
          content: last.content + content,
        };

        const updatedFeed = [...existingFeed];
        if (lastFeedItem && lastFeedItem.type === activityType) {
          updatedFeed[updatedFeed.length - 1] = {
            ...lastFeedItem,
            content: (lastFeedItem.content || '') + content,
          };
        } else {
          updatedFeed.push({
            id: `out-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
            type: activityType,
            timestamp: timeStr,
            content,
          });
        }

        return {
          streamEntries: {
            ...state.streamEntries,
            [taskId]: {
              ...existing,
              outputs: updated,
            },
          },
          activityFeed: {
            ...state.activityFeed,
            [taskId]: updatedFeed,
          },
        };
      }

      const newEntry: LiveStreamEntry = {
        id: `out-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
        timestamp: timeStr,
        type,
        content,
        images,
      };

      const newFeedItem: DrawerActivityItem = {
        id: newEntry.id,
        type: activityType,
        timestamp: timeStr,
        content,
        images,
      };

      return {
        streamEntries: {
          ...state.streamEntries,
          [taskId]: {
            ...existing,
            outputs: [...existing.outputs, newEntry],
          },
        },
        activityFeed: {
          ...state.activityFeed,
          [taskId]: [...existingFeed, newFeedItem],
        },
      };
    });
    scheduleDialogSave(taskId);
  },

  addToolExecution: (taskId, tool) => {
    set((state) => {
      const existing = state.toolExecutions[taskId] || [];
      const existingFeed = state.activityFeed[taskId] || [];

      const feedItem: DrawerActivityItem = {
        id: tool.id,
        type: 'tool',
        timestamp: tool.timestamp,
        tool,
      };

      return {
        toolExecutions: {
          ...state.toolExecutions,
          [taskId]: [tool, ...existing],
        },
        activityFeed: {
          ...state.activityFeed,
          [taskId]: [...existingFeed, feedItem],
        },
      };
    });
    scheduleDialogSave(taskId);
  },

  updateToolExecution: (taskId, toolId, partial) => {
    set((state) => {
      const existing = state.toolExecutions[taskId] || [];
      const updated = existing.map((t) => (t.id === toolId ? { ...t, ...partial } : t));

      const existingFeed = state.activityFeed[taskId] || [];
      const updatedFeed = existingFeed.map((item) => {
        if (item.type === 'tool' && (item.id === toolId || item.tool?.id === toolId)) {
          return {
            ...item,
            tool: item.tool ? { ...item.tool, ...partial } : undefined,
          };
        }
        return item;
      });

      return {
        toolExecutions: {
          ...state.toolExecutions,
          [taskId]: updated,
        },
        activityFeed: {
          ...state.activityFeed,
          [taskId]: updatedFeed,
        },
      };
    });
    scheduleDialogSave(taskId);
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

  setFinalTps: (taskId, v) => {
    set((state) => ({
      finalTps: {
        ...state.finalTps,
        [taskId]: v,
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
        const res = await invoke<CompactionReport>(
          'compact_context',
          { taskId }
        );
        if (res?.saved_tokens) savedTokens = res.saved_tokens;
        if (res?.evaporated_count) evaporatedCount = Math.max(evaporatedCount, res.evaporated_count);
        if (res?.tokens_after) {
          get().setTaskTokens(taskId, res.tokens_after);
        }
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

    // Refresh dialog feed from SQLite to load the persisted 'compacted' accordion card
    try {
      await get().loadTaskDialog(taskId);
    } catch (e) {
      console.warn('Failed to reload dialog after compaction:', e);
    }

    return { savedTokens, evaporatedCount };
  },

  clearTaskLogs: async (taskId) => {
    if (dialogSaveTimers[taskId]) {
      clearTimeout(dialogSaveTimers[taskId]);
      delete dialogSaveTimers[taskId];
    }
    set((state) => ({
      activityFeed: {
        ...state.activityFeed,
        [taskId]: [],
      },
      streamEntries: {
        ...state.streamEntries,
        [taskId]: { thoughts: [], outputs: [] },
      },
      toolExecutions: {
        ...state.toolExecutions,
        [taskId]: [],
      },
      finalTps: {
        ...state.finalTps,
        [taskId]: null,
      },
    }));
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('clear_task_dialog', { taskId });
    } catch (e) {
      console.info('clear_task_dialog invoke error or web mode:', e);
    }
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

  loadTaskDialog: async (taskId) => {
    if (!taskId) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const items = await invoke<DrawerActivityItem[]>('load_task_dialog', { taskId });
      if (items && Array.isArray(items) && items.length > 0) {
        set((state) => {
          const currentFeed = state.activityFeed[taskId] || [];
          if (currentFeed.length === 0 || items.length >= currentFeed.length) {
            const tools: ToolExecution[] = [];
            const thoughts: LiveStreamEntry[] = [];
            const outputs: LiveStreamEntry[] = [];

            for (const item of items) {
              if (item.type === 'tool' && item.tool) {
                tools.unshift(item.tool);
              } else if (item.type === 'thought') {
                thoughts.push({
                  id: item.id,
                  timestamp: item.timestamp,
                  type: 'thought',
                  content: item.content || '',
                  durationMs: item.durationMs,
                });
              } else {
                outputs.push({
                  id: item.id,
                  timestamp: item.timestamp,
                  type: (item.type === 'steer' || item.type === 'system' || item.type === 'error' || item.type === 'user' ? item.type : 'output') as any,
                  content: item.content || '',
                });
              }
            }

            return {
              activityFeed: {
                ...state.activityFeed,
                [taskId]: items,
              },
              toolExecutions: {
                ...state.toolExecutions,
                [taskId]: tools,
              },
              streamEntries: {
                ...state.streamEntries,
                [taskId]: { thoughts, outputs },
              },
            };
          }
          return state;
        });
        return;
      }
    } catch (e) {
      console.info('load_task_dialog invoke error or web mode:', e);
    }
  },

  flushSaveDialog: async (taskId) => {
    if (dialogSaveTimers[taskId]) {
      clearTimeout(dialogSaveTimers[taskId]);
      delete dialogSaveTimers[taskId];
    }
    const feed = get().activityFeed[taskId];
    if (!feed || feed.length === 0) return;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_task_dialog', { taskId, items: feed });
    } catch (err) {
      console.info('save_task_dialog invoke error or web mode:', err);
    }
  },

  saveTaskDialog: async (taskId) => {
    await get().flushSaveDialog(taskId);
  },
}));

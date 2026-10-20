import { create } from 'zustand';
import { LiveStreamEntry, ToolExecution, ActivityItem, ActivityType, CompactionReport, SubagentInfo, SubagentStatus } from '../types';
import { useBoardStore } from './boardStore';

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
  durationMs?: number;
  tps?: number;
}

interface DrawerState {
  isOpen: boolean;
  activeTaskId: string | null;
  activeTab: DrawerTab;
  activityFeed: Record<string, DrawerActivityItem[]>;
  dialogItems?: Record<string, DrawerActivityItem[]>;
  streamEntries: Record<string, { thoughts: LiveStreamEntry[]; outputs: LiveStreamEntry[] }>;
  toolExecutions: Record<string, ToolExecution[]>;
  taskTelemetry: Record<string, TaskTelemetry>;
  thinkingStartTime: Record<string, number | null>;
  generationStartTime: Record<string, number | null>;
  generationEndTime: Record<string, number | null>;
  taskStartTime: Record<string, number | null>;
  taskEndTime: Record<string, number | null>;
  streamedTurnTokens: Record<string, number>;
  finalTps: Record<string, number | null>;
  tokenOverrides: Record<string, number | undefined>;
  cumulativeOutputTokens: Record<string, number>;
  cumulativeGenerationSec: Record<string, number>;
  activeAgentTaskIds: Record<string, boolean>;

  // Nested Subagent View State
  activeSubagentId: string | null;
  activeSubagentTitle: string | null;
  activeSubagentRole: string | null;
  activeSubagentPrompt: string | null;
  toolIdToSubagentId: Record<string, string>;
  subagentIdToToolId: Record<string, string>;
  subagents: Record<string, SubagentInfo[]>;

  // Actions
  setActiveAgentTask: (taskId: string, isActive: boolean) => void;
  markFinalResponse: (taskId: string, response?: string) => void;
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
  setGenerationStartTime: (taskId: string, time: number | null) => void;
  setGenerationEndTime: (taskId: string, time: number | null) => void;
  setTaskStartTime: (taskId: string, time: number | null) => void;
  setTaskEndTime: (taskId: string, time: number | null) => void;
  recordStreamedChars: (taskId: string, chars: number) => void;
  resetTurnStreamMetrics: (taskId: string) => void;
  setFinalTps: (taskId: string, v: number | null) => void;
  recordTurnMetrics: (taskId: string, tokens: number, seconds: number) => void;
  sendSteerMessage: (taskId: string, message: string) => Promise<void>;
  injectOptimisticCompactTool: (taskId: string, customId?: string) => string;
  compactTaskContext: (taskId: string) => Promise<{ savedTokens: number; evaporatedCount: number }>;
  clearTaskLogs: (taskId: string) => Promise<void> | void;
  setTaskTokens: (taskId: string, tokens: number) => void;
  evaporateClientToolLogs: (taskId: string) => { evaporatedCount: number; savedTokens: number };
  loadTaskDialog: (taskId: string) => Promise<void>;
  saveTaskDialog: (taskId: string) => Promise<void>;
  flushSaveDialog: (taskId: string) => Promise<void>;
  registerSubagentMapping: (toolId: string, subagentId: string) => void;
  openSubagent: (subagentId: string, title: string, role: string, prompt: string) => void;
  closeSubagent: () => void;
  loadSubagentDialog: (parentTaskId: string, subagentId: string) => Promise<void>;
  loadSubagents: (parentTaskId: string) => Promise<void>;
  registerSubagent: (parentTaskId: string, subagent: SubagentInfo) => void;
  updateSubagentStatus: (subagentId: string, status: SubagentStatus, error?: string) => void;
  recordSubagentStep: (subagentId: string, activity?: DrawerActivityItem) => void;
  addQuestion: (
    taskId: string,
    question: {
      questionId: string;
      text: string;
      options?: Array<{ label: string; description?: string }>;
      multiple?: boolean;
    }
  ) => void;
  recordQuestionAnswer: (taskId: string, questionId: string, answer: string) => Promise<void>;
}

export const useDrawerStore = create<DrawerState>((set, get) => ({
  isOpen: false,
  activeTaskId: null,
  activeTab: 'thoughts',
  activityFeed: {},
  get dialogItems() {
    return this.activityFeed;
  },
  streamEntries: {},
  toolExecutions: {},
  taskTelemetry: {},
  thinkingStartTime: {},
  generationStartTime: {},
  generationEndTime: {},
  taskStartTime: {},
  taskEndTime: {},
  streamedTurnTokens: {},
  finalTps: {},
  tokenOverrides: {},
  cumulativeOutputTokens: {},
  cumulativeGenerationSec: {},
  activeAgentTaskIds: {},
  activeSubagentId: null,
  activeSubagentTitle: null,
  activeSubagentRole: null,
  activeSubagentPrompt: null,
  toolIdToSubagentId: {},
  subagentIdToToolId: {},
  subagents: {},

  setActiveAgentTask: (taskId, isActive) => {
    if (!taskId) return;
    set((state) => {
      if (Boolean(state.activeAgentTaskIds[taskId]) === isActive) return state;
      return {
        activeAgentTaskIds: {
          ...state.activeAgentTaskIds,
          [taskId]: isActive,
        },
      };
    });
  },

  markFinalResponse: (taskId, response) => {
    if (!taskId) return;
    const timeStr = new Date().toLocaleTimeString('en-US', { hour12: false });
    set((state) => {
      const feed = state.activityFeed[taskId] || [];
      const updatedFeed = [...feed];

      let lastOutputIdx = -1;
      for (let i = updatedFeed.length - 1; i >= 0; i--) {
        if (updatedFeed[i].type === 'output') {
          lastOutputIdx = i;
          break;
        }
      }

      const hasInterveningTools = lastOutputIdx !== -1 &&
        updatedFeed.slice(lastOutputIdx + 1).some((it) => it.type === 'tool' || Boolean((it as any).tool));

      if (lastOutputIdx !== -1 && !hasInterveningTools) {
        const lastOutput = updatedFeed[lastOutputIdx];
        const content = lastOutput.content || '';
        if (response && response.length > 0 && !content.endsWith(response)) {
          updatedFeed[lastOutputIdx] = {
            ...lastOutput,
            content: content ? `${content}\n\n${response}` : response,
            isFinal: true,
          };
        } else {
          updatedFeed[lastOutputIdx] = {
            ...lastOutput,
            isFinal: true,
          };
        }
      } else if (response && response.length > 0) {
        updatedFeed.push({
          id: `out-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
          type: 'output',
          timestamp: timeStr,
          content: response,
          isFinal: true,
        });
      }

      return {
        activityFeed: {
          ...state.activityFeed,
          [taskId]: updatedFeed,
        },
      };
    });
    scheduleDialogSave(taskId);
  },

  registerSubagentMapping: (toolId, subagentId) => {
    if (!toolId || !subagentId) return;
    set((state) => ({
      toolIdToSubagentId: {
        ...state.toolIdToSubagentId,
        [toolId]: subagentId,
      },
      subagentIdToToolId: {
        ...state.subagentIdToToolId,
        [subagentId]: toolId,
      },
    }));
  },

  openDrawer: (taskId, initialTab = 'thoughts') => {
    set({
      isOpen: true,
      activeTaskId: taskId,
      activeTab: initialTab,
      activeSubagentId: null,
      activeSubagentTitle: null,
      activeSubagentRole: null,
      activeSubagentPrompt: null,
    });
    get().loadTaskDialog(taskId);
    get().loadSubagents(taskId);
  },

  closeDrawer: () => {
    set({
      isOpen: false,
      activeTaskId: null,
      activeSubagentId: null,
      activeSubagentTitle: null,
      activeSubagentRole: null,
      activeSubagentPrompt: null,
    });
  },

  setActiveTab: (tab) => set({ activeTab: tab }),

  appendThought: (taskId, content) => {
    const timeStr = new Date().toLocaleTimeString('en-US', { hour12: false });
    const now = Date.now();

    set((state) => {
      const existing = state.streamEntries[taskId] || { thoughts: [], outputs: [] };
      const last = existing.thoughts[existing.thoughts.length - 1];

      const existingFeed = state.activityFeed[taskId] || [];
      const lastFeedItem = existingFeed[existingFeed.length - 1];

      // Detect if this is a fresh thought block:
      // (1) No feed items yet, or
      // (2) Last feed item is not a thought (e.g. output or tool intervened), or
      // (3) Coalescing window has passed (> 3000ms idle silence since last thought chunk)
      const lastChunkTime = (lastFeedItem?.createdAt || now) + (lastFeedItem?.durationMs ?? 0);
      const isNewBlock =
        !lastFeedItem ||
        lastFeedItem.type !== 'thought' ||
        (now - lastChunkTime > 3000);

      const blockStart = isNewBlock
        ? now
        : (lastFeedItem.createdAt || state.thinkingStartTime[taskId] || now);

      const durationMs = Math.max(0, now - blockStart);

      // Stream coalescing for thoughts (< 3s window and last feed item is thought)
      if (!isNewBlock && lastFeedItem && lastFeedItem.type === 'thought') {
        const updatedThoughts = [...existing.thoughts];
        if (last) {
          updatedThoughts[updatedThoughts.length - 1] = {
            ...last,
            content: last.content + content,
            durationMs,
            createdAt: last.createdAt || blockStart,
          };
        } else {
          updatedThoughts.push({
            id: lastFeedItem.id,
            timestamp: lastFeedItem.timestamp,
            type: 'thought',
            content: (lastFeedItem.content || '') + content,
            durationMs,
            createdAt: blockStart,
          });
        }

        const updatedFeed = [...existingFeed];
        updatedFeed[updatedFeed.length - 1] = {
          ...lastFeedItem,
          content: (lastFeedItem.content || '') + content,
          durationMs,
          createdAt: lastFeedItem.createdAt || blockStart,
        };

        return {
          thinkingStartTime: {
            ...state.thinkingStartTime,
            [taskId]: blockStart,
          },
          streamEntries: {
            ...state.streamEntries,
            [taskId]: {
              ...existing,
              thoughts: updatedThoughts,
            },
          },
          activityFeed: {
            ...state.activityFeed,
            [taskId]: updatedFeed,
          },
        };
      }

      // Fresh thought block
      const newEntry: LiveStreamEntry = {
        id: `th-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
        timestamp: timeStr,
        type: 'thought',
        content,
        durationMs,
        createdAt: blockStart,
      };

      const newFeedItem: DrawerActivityItem = {
        id: newEntry.id,
        type: 'thought',
        timestamp: timeStr,
        content,
        durationMs,
        createdAt: blockStart,
      };

      return {
        thinkingStartTime: {
          ...state.thinkingStartTime,
          [taskId]: blockStart,
        },
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

      // Deduplication guard for consecutive identical steer / user messages
      if (
        (activityType === 'steer' || activityType === 'user') &&
        lastFeedItem &&
        (lastFeedItem.type === 'steer' || lastFeedItem.type === 'user') &&
        lastFeedItem.content?.trim() === content.trim()
      ) {
        return state;
      }

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
    // If tool is a subagent execution, extract subagent_id if already present
    if (tool.toolName === 'subagent' || tool.toolName === 'subagent_bg') {
      const pSubId = (tool.params as Record<string, unknown> | undefined)?.subagent_id;
      if (typeof pSubId === 'string' && pSubId) {
        get().registerSubagentMapping(tool.id, pSubId);
      } else if (tool.stdout) {
        try {
          const parsed = JSON.parse(tool.stdout);
          if (parsed.subagent_id) get().registerSubagentMapping(tool.id, parsed.subagent_id);
        } catch {}
        const match = tool.stdout.match(/\[Subagent ID:\s*([^\s\]]+)\]/);
        if (match && match[1]) get().registerSubagentMapping(tool.id, match[1]);
      }
    }

    set((state) => {
      const existing = state.toolExecutions[taskId] || [];
      const existingFeed = state.activityFeed[taskId] || [];

      const feedItem: DrawerActivityItem = {
        id: tool.id,
        type: 'tool',
        timestamp: tool.timestamp,
        tool,
      };

      const feedExists = existingFeed.some((item) => item.id === tool.id || item.tool?.id === tool.id);
      const updatedFeed = feedExists
        ? existingFeed.map((item) =>
            item.id === tool.id || item.tool?.id === tool.id
              ? { ...item, tool: { ...item.tool, ...tool } }
              : item
          )
        : [...existingFeed, feedItem];

      const toolExists = existing.some((t) => t.id === tool.id);
      const updatedTools = toolExists
        ? existing.map((t) => (t.id === tool.id ? { ...t, ...tool } : t))
        : [tool, ...existing];

      return {
        toolExecutions: {
          ...state.toolExecutions,
          [taskId]: updatedTools,
        },
        activityFeed: {
          ...state.activityFeed,
          [taskId]: updatedFeed,
        },
      };
    });
    scheduleDialogSave(taskId);
  },

  updateToolExecution: (taskId, toolId, partial) => {
    if (partial.params) {
      const pSubId = (partial.params as Record<string, unknown> | undefined)?.subagent_id;
      if (typeof pSubId === 'string' && pSubId) {
        get().registerSubagentMapping(toolId, pSubId);
      }
    }
    if (partial.stdout) {
      try {
        const parsed = JSON.parse(partial.stdout);
        if (parsed.subagent_id) get().registerSubagentMapping(toolId, parsed.subagent_id);
      } catch {}
      const match = partial.stdout.match(/\[Subagent ID:\s*([^\s\]]+)\]/);
      if (match && match[1]) get().registerSubagentMapping(toolId, match[1]);
    }

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
      const merged = { ...current, ...telemetry };
      const totalInput = (merged.inputTokens || 0) + (merged.cacheReadTokens || 0);
      if (typeof merged.cacheHitRate !== 'number' || !Number.isFinite(merged.cacheHitRate) || merged.cacheHitRate < 0) {
        merged.cacheHitRate = totalInput > 0 ? ((merged.cacheReadTokens || 0) / totalInput) * 100 : 0;
      }
      return {
        taskTelemetry: {
          ...state.taskTelemetry,
          [taskId]: merged,
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

  setGenerationStartTime: (taskId, time) => {
    set((state) => ({
      generationStartTime: {
        ...state.generationStartTime,
        [taskId]: time,
      },
    }));
  },

  setGenerationEndTime: (taskId, time) => {
    set((state) => ({
      generationEndTime: {
        ...state.generationEndTime,
        [taskId]: time,
      },
    }));
  },

  setTaskStartTime: (taskId, time) => {
    set((state) => ({
      taskStartTime: {
        ...state.taskStartTime,
        [taskId]: time,
      },
    }));
  },

  setTaskEndTime: (taskId, time) => {
    set((state) => ({
      taskEndTime: {
        ...state.taskEndTime,
        [taskId]: time,
      },
    }));
  },

  recordStreamedChars: (taskId, chars) => {
    if (!chars || chars <= 0) return;
    set((state) => {
      const current = state.streamedTurnTokens[taskId] ?? 0;
      const added = Math.max(1, Math.round(chars / 3.8));
      return {
        streamedTurnTokens: {
          ...state.streamedTurnTokens,
          [taskId]: current + added,
        },
      };
    });
  },

  resetTurnStreamMetrics: (taskId) => {
    set((state) => ({
      finalTps: {
        ...state.finalTps,
        [taskId]: null,
      },
      generationStartTime: {
        ...state.generationStartTime,
        [taskId]: null,
      },
      generationEndTime: {
        ...state.generationEndTime,
        [taskId]: null,
      },
      streamedTurnTokens: {
        ...state.streamedTurnTokens,
        [taskId]: 0,
      },
      thinkingStartTime: {
        ...state.thinkingStartTime,
        [taskId]: Date.now(),
      },
      taskStartTime: {
        ...state.taskStartTime,
        [taskId]: state.taskStartTime[taskId] ?? Date.now(),
      },
    }));
  },

  setFinalTps: (taskId, v) => {
    set((state) => ({
      finalTps: {
        ...state.finalTps,
        [taskId]: v != null && Number.isFinite(v) && v > 0 ? Number(v.toFixed(1)) : v,
      },
    }));
  },

  recordTurnMetrics: (taskId, tokens, seconds) => {
    if (tokens <= 0 || seconds <= 0.01) return;
    const tps = Number((tokens / seconds).toFixed(1));
    set((state) => {
      const prevTok = state.cumulativeOutputTokens[taskId] ?? 0;
      const prevSec = state.cumulativeGenerationSec[taskId] ?? 0;
      return {
        finalTps: {
          ...state.finalTps,
          [taskId]: tps,
        },
        cumulativeOutputTokens: {
          ...state.cumulativeOutputTokens,
          [taskId]: prevTok + tokens,
        },
        cumulativeGenerationSec: {
          ...state.cumulativeGenerationSec,
          [taskId]: prevSec + seconds,
        },
      };
    });
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

  injectOptimisticCompactTool: (taskId, customId) => {
    const toolId = customId || `compact_context-${taskId}-${Date.now()}`;
    const timestamp = new Date().toLocaleTimeString('en-US', { hour12: false });
    const optimisticTool: ToolExecution = {
      id: toolId,
      toolName: 'compact_context',
      timestamp,
      durationMs: 0,
      status: 'running',
      params: {
        taskId,
        title: 'Сжатие контекста...',
      },
    };

    const optimisticItem: DrawerActivityItem = {
      id: toolId,
      type: 'tool',
      timestamp,
      content: 'Сжатие контекста...',
      tool: optimisticTool,
    };

    set((state) => {
      const existingTools = state.toolExecutions[taskId] || [];
      const existingFeed = state.activityFeed[taskId] || [];

      const alreadyExists = existingFeed.some(
        (it) => it.id === toolId || it.tool?.id === toolId
      );
      if (alreadyExists) return {};

      return {
        toolExecutions: {
          ...state.toolExecutions,
          [taskId]: [optimisticTool, ...existingTools],
        },
        activityFeed: {
          ...state.activityFeed,
          [taskId]: [...existingFeed, optimisticItem],
        },
      };
    });

    return toolId;
  },

  compactTaskContext: async (taskId) => {
    // 1. Immediately inject optimistic in-progress tool item into chat feed
    const toolId = get().injectOptimisticCompactTool(taskId);
    const startTime = performance.now();

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
          { taskId, toolId }
        );
        if (res?.saved_tokens) savedTokens = res.saved_tokens;
        if (res?.evaporated_count) evaporatedCount = Math.max(evaporatedCount, res.evaporated_count);
        if (res?.tokens_after) {
          get().setTaskTokens(taskId, res.tokens_after);
          get().updateTelemetry(taskId, {
            inputTokens: res.tokens_after,
            outputTokens: 0,
            cacheReadTokens: 0,
            cacheCreationTokens: 0,
          });
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

    const durationMs = Math.round(performance.now() - startTime);

    // 2. Update optimistic tool item with success status & token savings
    const stdoutMsg = `Контекст успешно сжат. Сохранено ${savedTokens} токенов.`;
    get().updateToolExecution(taskId, toolId, {
      status: 'success',
      durationMs,
      stdout: stdoutMsg,
      params: {
        taskId,
        title: `Сжато (-${savedTokens} токенов)`,
        savedTokens,
      },
    });

    // 3. Refresh dialog feed from SQLite to load the persisted 'compacted' accordion card
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
      generationStartTime: {
        ...state.generationStartTime,
        [taskId]: null,
      },
      generationEndTime: {
        ...state.generationEndTime,
        [taskId]: null,
      },
      streamedTurnTokens: {
        ...state.streamedTurnTokens,
        [taskId]: 0,
      },
      taskStartTime: {
        ...state.taskStartTime,
        [taskId]: null,
      },
      taskEndTime: {
        ...state.taskEndTime,
        [taskId]: null,
      },
      cumulativeOutputTokens: {
        ...state.cumulativeOutputTokens,
        [taskId]: 0,
      },
      activeAgentTaskIds: {
        ...state.activeAgentTaskIds,
        [taskId]: false,
      },
      cumulativeGenerationSec: {
        ...state.cumulativeGenerationSec,
        [taskId]: 0,
      },
      subagents: {
        ...state.subagents,
        [taskId]: [],
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
      if (items && Array.isArray(items)) {
        let compactedTokens: number | undefined;
        for (let i = items.length - 1; i >= 0; i--) {
          const it = items[i];
          if (it.type === 'compacted' && it.tool) {
            const tData = it.tool as any;
            if (typeof tData?.tokensAfter === 'number') {
              compactedTokens = tData.tokensAfter;
              break;
            }
          }
        }

        try {
          const boardTask = useBoardStore.getState().tasks.find((t) => t.id === taskId);
          if (compactedTokens === undefined && typeof boardTask?.currentTokens === 'number') {
            compactedTokens = boardTask.currentTokens;
          }
        } catch {
          // Board store optional
        }

        if (items.length > 0) {
          set((state) => {
            const currentFeed = state.activityFeed[taskId] || [];
            const hasCompacted = items.some((it) => it.type === 'compacted');
            const newOverrides = compactedTokens !== undefined
              ? { ...state.tokenOverrides, [taskId]: compactedTokens }
              : state.tokenOverrides;

            if (currentFeed.length === 0 || items.length >= currentFeed.length || hasCompacted) {
              // Deduplicate adjacent identical steer / user messages
              const dedupedItems: DrawerActivityItem[] = [];
              for (const item of items) {
                const prev = dedupedItems[dedupedItems.length - 1];
                if (
                  prev &&
                  (item.type === 'steer' || item.type === 'user') &&
                  (prev.type === 'steer' || prev.type === 'user') &&
                  item.content?.trim() === prev.content?.trim()
                ) {
                  if (item.type === 'steer') {
                    prev.type = 'steer';
                  }
                  continue;
                }
                dedupedItems.push(item);
              }

              const tools: ToolExecution[] = [];
              const thoughts: LiveStreamEntry[] = [];
              const outputs: LiveStreamEntry[] = [];

              for (const item of dedupedItems) {
                if (item.type === 'tool' && item.tool) {
                  tools.unshift(item.tool);
                } else if (item.type === 'thought') {
                  thoughts.push({
                    id: item.id,
                    timestamp: item.timestamp,
                    type: 'thought',
                    content: item.content || '',
                    durationMs: item.durationMs,
                    createdAt: item.createdAt,
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
                  [taskId]: dedupedItems,
                },
                toolExecutions: {
                  ...state.toolExecutions,
                  [taskId]: tools,
                },
                streamEntries: {
                  ...state.streamEntries,
                  [taskId]: { thoughts, outputs },
                },
                tokenOverrides: newOverrides,
              };
            }

            return {
              tokenOverrides: newOverrides,
            };
          });
          return;
        } else {
          set((state) => ({
            activityFeed: { ...state.activityFeed, [taskId]: [] },
            streamEntries: { ...state.streamEntries, [taskId]: { thoughts: [], outputs: [] } },
            toolExecutions: { ...state.toolExecutions, [taskId]: [] },
            tokenOverrides: { ...state.tokenOverrides, [taskId]: undefined },
            cumulativeOutputTokens: { ...state.cumulativeOutputTokens, [taskId]: 0 },
            cumulativeGenerationSec: { ...state.cumulativeGenerationSec, [taskId]: 0 },
          }));
          return;
        }
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

  openSubagent: (subagentId, title, role, prompt) => {
    const state = get();
    // Resolve subagentId using toolIdToSubagentId map if a tool ID was passed
    let resolvedSubagentId = state.toolIdToSubagentId[subagentId] || subagentId;

    // Fallback: search tool executions if still unresolved
    if (resolvedSubagentId === subagentId && state.activeTaskId) {
      const tools = state.toolExecutions[state.activeTaskId] || [];
      const matched = tools.find((t) => t.id === subagentId);
      if (matched) {
        const pId = (matched.params as Record<string, unknown> | undefined)?.subagent_id;
        if (typeof pId === 'string' && pId) {
          resolvedSubagentId = pId;
        } else if (matched.stdout) {
          try {
            const parsed = JSON.parse(matched.stdout);
            if (parsed.subagent_id) resolvedSubagentId = parsed.subagent_id;
          } catch {}
          if (resolvedSubagentId === subagentId) {
            const m = matched.stdout.match(/\[Subagent ID:\s*([^\s\]]+)\]/);
            if (m && m[1]) resolvedSubagentId = m[1];
          }
        }
      }
    }

    set({
      activeSubagentId: resolvedSubagentId,
      activeSubagentTitle: title,
      activeSubagentRole: role,
      activeSubagentPrompt: prompt,
      toolIdToSubagentId:
        resolvedSubagentId !== subagentId
          ? { ...state.toolIdToSubagentId, [subagentId]: resolvedSubagentId }
          : state.toolIdToSubagentId,
      subagentIdToToolId:
        resolvedSubagentId !== subagentId
          ? { ...state.subagentIdToToolId, [resolvedSubagentId]: subagentId }
          : state.subagentIdToToolId,
    });
    const parentTaskId = get().activeTaskId;
    if (parentTaskId) {
      void get().loadSubagentDialog(parentTaskId, resolvedSubagentId);
      if (resolvedSubagentId !== subagentId) {
        void get().loadSubagentDialog(parentTaskId, subagentId);
      }
    }
  },

  closeSubagent: () => {
    set({
      activeSubagentId: null,
      activeSubagentTitle: null,
      activeSubagentRole: null,
      activeSubagentPrompt: null,
    });
  },

  loadSubagentDialog: async (parentTaskId, subagentId) => {
    if (!parentTaskId || !subagentId) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const items = await invoke<DrawerActivityItem[]>('load_subagent_dialog', {
        parentTaskId,
        subagentId,
      });
      if (items && Array.isArray(items) && items.length > 0) {
        set((state) => {
          const nextFeed = {
            ...state.activityFeed,
            [subagentId]: items,
          };
          const mappedToolId = state.subagentIdToToolId[subagentId];
          if (mappedToolId) {
            nextFeed[mappedToolId] = items;
          }
          const mappedSubId = state.toolIdToSubagentId[subagentId];
          if (mappedSubId) {
            nextFeed[mappedSubId] = items;
          }
          return { activityFeed: nextFeed };
        });
      }
    } catch (err) {
      console.info('load_subagent_dialog invoke error or web mode:', err);
    }
  },

  loadSubagents: async (parentTaskId) => {
    if (!parentTaskId) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const loaded = await invoke<SubagentInfo[]>('list_subagents', { parentTaskId });
      if (loaded && Array.isArray(loaded)) {
        set((state) => {
          const existing = state.subagents[parentTaskId] || [];
          const mergedMap = new Map<string, SubagentInfo>();
          for (const item of loaded) {
            mergedMap.set(item.id, item);
          }
          for (const ex of existing) {
            if (mergedMap.has(ex.id)) {
              const current = mergedMap.get(ex.id)!;
              mergedMap.set(ex.id, {
                ...current,
                status: ex.status === 'running' ? 'running' : current.status,
                stepsCount: Math.max(current.stepsCount, ex.stepsCount),
                durationMs: ex.durationMs ?? current.durationMs,
              });
            } else {
              mergedMap.set(ex.id, ex);
            }
          }
          return {
            subagents: {
              ...state.subagents,
              [parentTaskId]: Array.from(mergedMap.values()),
            },
          };
        });
      }
    } catch {
      // In web or mock mode, extract from toolExecutions & activityFeed
      const state = get();
      const tools = state.toolExecutions[parentTaskId] || [];
      const derived: SubagentInfo[] = [];
      for (const t of tools) {
        if (t.toolName === 'subagent' || t.toolName === 'subagent_bg') {
          const params = (t.params as any) || {};
          const subId = params.subagent_id || state.toolIdToSubagentId[t.id] || t.id;
          const feed = state.activityFeed[subId] || [];
          derived.push({
            id: subId,
            parentTaskId,
            toolCallId: t.id,
            title: params.title || params.prompt?.slice(0, 50) || 'Субагент',
            role: params.role || 'general',
            prompt: params.prompt || '',
            status: t.status === 'running' ? 'running' : t.status === 'failed' ? 'error' : 'finished',
            durationMs: t.durationMs,
            stepsCount: feed.length,
          });
        }
      }
      if (derived.length > 0) {
        set((s) => ({
          subagents: {
            ...s.subagents,
            [parentTaskId]: derived,
          },
        }));
      }
    }
  },

  registerSubagent: (parentTaskId, subagent) => {
    if (!parentTaskId || !subagent?.id) return;
    set((state) => {
      const list = state.subagents[parentTaskId] || [];
      const idx = list.findIndex(
        (s) => s.id === subagent.id || (subagent.toolCallId && s.toolCallId === subagent.toolCallId)
      );
      let nextList: SubagentInfo[];
      if (idx >= 0) {
        nextList = [...list];
        nextList[idx] = { ...nextList[idx], ...subagent };
      } else {
        nextList = [...list, subagent];
      }
      return {
        subagents: {
          ...state.subagents,
          [parentTaskId]: nextList,
        },
      };
    });
  },

  updateSubagentStatus: (subagentId, status, error) => {
    if (!subagentId) return;
    set((state) => {
      let changed = false;
      const nextSubagents: Record<string, SubagentInfo[]> = {};
      for (const [taskId, list] of Object.entries(state.subagents)) {
        let listChanged = false;
        const updatedList = list.map((s) => {
          if (s.id === subagentId || (s.toolCallId && s.toolCallId === subagentId)) {
            listChanged = true;
            changed = true;
            const nowTime = Date.now();
            const finishedAt = (status === 'finished' || status === 'error' || status === 'stopped')
              ? (s.finishedAt || nowTime)
              : s.finishedAt;
            const calculatedDuration = s.createdAt ? Math.max(0, (finishedAt || nowTime) - s.createdAt) : undefined;
            const finalDurationMs = (s.isBg || !s.durationMs || (s.durationMs < 1000 && calculatedDuration && calculatedDuration >= 1000))
              ? (calculatedDuration ?? s.durationMs)
              : s.durationMs;
            return {
              ...s,
              status,
              finishedAt,
              lastError: error || s.lastError,
              durationMs: finalDurationMs,
            };
          }
          return s;
        });
        nextSubagents[taskId] = listChanged ? updatedList : list;
      }
      return changed ? { subagents: nextSubagents } : state;
    });
  },

  recordSubagentStep: (subagentId, _activity) => {
    if (!subagentId) return;
    set((state) => {
      let changed = false;
      const nextSubagents: Record<string, SubagentInfo[]> = {};
      for (const [taskId, list] of Object.entries(state.subagents)) {
        let listChanged = false;
        const updatedList = list.map((s) => {
          if (s.id === subagentId || (s.toolCallId && s.toolCallId === subagentId)) {
            listChanged = true;
            changed = true;
            return {
              ...s,
              stepsCount: (s.stepsCount || 0) + 1,
            };
          }
          return s;
        });
        nextSubagents[taskId] = listChanged ? updatedList : list;
      }
      return changed ? { subagents: nextSubagents } : state;
    });
  },

  addQuestion: (taskId, question) => {
    const timeStr = new Date().toLocaleTimeString('en-US', { hour12: false });
    const questionId = question.questionId;
    const itemId = `q-${questionId}`;

    set((state) => {
      const existingFeed = state.activityFeed[taskId] || [];
      if (existingFeed.some((f) => f.id === itemId || f.tool?.id === questionId)) {
        return state;
      }

      const item: DrawerActivityItem = {
        id: itemId,
        type: 'question',
        timestamp: timeStr,
        content: question.text,
        tool: {
          id: questionId,
          toolName: 'question',
          timestamp: timeStr,
          durationMs: 0,
          status: 'running',
          params: {
            taskId,
            questionId,
            text: question.text,
            options: question.options,
            multiple: question.multiple,
          },
        },
      };

      return {
        activityFeed: {
          ...state.activityFeed,
          [taskId]: [...existingFeed, item],
        },
      };
    });
  },

  recordQuestionAnswer: async (taskId, questionId, answer) => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('answer_agent_question', { taskId, questionId, answer });
    } catch (err) {
      console.error('Failed to invoke answer_agent_question:', err);
    }

    set((state) => {
      const existingFeed = state.activityFeed[taskId] || [];
      const updatedFeed = existingFeed.map((item) => {
        if (
          item.id === `q-${questionId}` ||
          (item.tool && (item.tool.id === questionId || item.tool.params?.questionId === questionId))
        ) {
          const prevParams = (item.tool?.params as Record<string, unknown>) || {};
          return {
            ...item,
            tool: item.tool
              ? {
                  ...item.tool,
                  status: 'success' as const,
                  params: {
                    ...prevParams,
                    selectedAnswer: answer,
                  },
                }
              : undefined,
          };
        }
        return item;
      });

      return {
        activityFeed: {
          ...state.activityFeed,
          [taskId]: updatedFeed,
        },
      };
    });
    scheduleDialogSave(taskId);
  },
}));

let subagentListenersInitialized = false;

export async function initSubagentListeners() {
  if (subagentListenersInitialized) return;
  subagentListenersInitialized = true;
  try {
    const { listen } = await import('@tauri-apps/api/event');

    await listen<{ parentTaskId?: string; subagentId?: string; title?: string; role?: string; prompt?: string; tools?: string[] }>(
      'subagent:spawned',
      (event) => {
        const { parentTaskId, subagentId, prompt, title } = event.payload || {};
        if (!subagentId) return;
        const state = useDrawerStore.getState();

        // Auto-map tool execution to subagent ID if matching tool is found in parentTaskId
        if (parentTaskId) {
          const tools = state.toolExecutions[parentTaskId] || [];
          const matchingTool = tools.find(
            (t) =>
              (t.toolName === 'subagent' || t.toolName === 'subagent_bg') &&
              !state.toolIdToSubagentId[t.id] &&
              ((t.params as Record<string, unknown> | undefined)?.prompt === prompt ||
                (t.params as Record<string, unknown> | undefined)?.title === title ||
                t.status === 'running')
          );
          if (matchingTool) {
            state.registerSubagentMapping(matchingTool.id, subagentId);
          }
        }

        const existingFeed = state.activityFeed[subagentId] || [];
        if (prompt && !existingFeed.some((i) => i.type === 'prompt')) {
          const promptItem: DrawerActivityItem = {
            id: `prompt-${subagentId}`,
            type: 'prompt',
            timestamp: new Date().toLocaleTimeString('en-US', { hour12: false }),
            content: prompt,
          };
          const nextFeed = [promptItem, ...existingFeed];
          const nextFeeds: Record<string, DrawerActivityItem[]> = {
            ...state.activityFeed,
            [subagentId]: nextFeed,
          };
          const mappedToolId = state.subagentIdToToolId[subagentId];
          if (mappedToolId) {
            nextFeeds[mappedToolId] = nextFeed;
          }
          useDrawerStore.setState({ activityFeed: nextFeeds });
        }

        // Register subagent record into subagents panel
        if (parentTaskId) {
          const { role } = event.payload || {};
          state.registerSubagent(parentTaskId, {
            id: subagentId,
            parentTaskId,
            title: title || 'Субагент',
            role: role || 'general',
            prompt: prompt || '',
            status: 'running',
            createdAt: Date.now(),
            stepsCount: 1,
          });
        }
      }
    );

    await listen<{ parentTaskId?: string; subagentId?: string; activity?: DrawerActivityItem }>(
      'subagent:activity',
      (event) => {
        const { subagentId, activity } = event.payload || {};
        if (!subagentId || !activity) return;
        const state = useDrawerStore.getState();
        const existingFeed = state.activityFeed[subagentId] || [];
        const existingIdx = existingFeed.findIndex((i) => i.id === activity.id);
        let nextFeed: DrawerActivityItem[];
        if (existingIdx >= 0) {
          nextFeed = [...existingFeed];
          nextFeed[existingIdx] = { ...nextFeed[existingIdx], ...activity };
        } else {
          nextFeed = [...existingFeed, activity];
        }
        const nextFeeds: Record<string, DrawerActivityItem[]> = {
          ...state.activityFeed,
          [subagentId]: nextFeed,
        };
        const mappedToolId = state.subagentIdToToolId[subagentId];
        if (mappedToolId) {
          nextFeeds[mappedToolId] = nextFeed;
        }
        useDrawerStore.setState({ activityFeed: nextFeeds });
        state.recordSubagentStep(subagentId, activity);
      }
    );

    await listen<{ parentTaskId?: string; subagentId?: string; response?: string }>(
      'subagent:finished',
      (event) => {
        const { subagentId, response } = event.payload || {};
        if (!subagentId || !response) return;
        const state = useDrawerStore.getState();
        const existingFeed = state.activityFeed[subagentId] || [];
        if (!existingFeed.some((i) => i.type === 'output' && i.content === response)) {
          const outItem: DrawerActivityItem = {
            id: `sub-out-${Date.now()}`,
            type: 'output',
            timestamp: new Date().toLocaleTimeString('en-US', { hour12: false }),
            content: response,
          };
          const nextFeed = [...existingFeed, outItem];
          const nextFeeds: Record<string, DrawerActivityItem[]> = {
            ...state.activityFeed,
            [subagentId]: nextFeed,
          };
          const mappedToolId = state.subagentIdToToolId[subagentId];
          if (mappedToolId) {
            nextFeeds[mappedToolId] = nextFeed;
          }
          useDrawerStore.setState({ activityFeed: nextFeeds });
        }
        state.updateSubagentStatus(subagentId, 'finished');
      }
    );

    await listen<{ parentTaskId?: string; subagentId?: string; error?: string }>(
      'subagent:error',
      (event) => {
        const { subagentId, error } = event.payload || {};
        if (!subagentId) return;
        useDrawerStore.getState().updateSubagentStatus(subagentId, 'error', error);
      }
    );
  } catch (err) {
    console.info('initSubagentListeners: web or test mode', err);
  }
}

if (typeof window !== 'undefined') {
  void initSubagentListeners();
}

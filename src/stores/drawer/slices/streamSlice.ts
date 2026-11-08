import { StateCreator } from 'zustand';
import {
  StreamSlice,
  DrawerState,
  DrawerActivityItem,
  ActivityType,
} from '../types';
import { LiveStreamEntry } from '../../../types';

function appendStreamChunk(
  state: DrawerState,
  taskId: string,
  streamId: string,
  type: 'thought' | 'output',
  content: string,
): Partial<DrawerState> {
  // Ignore chunks queued before an attempt was discarded or superseded.
  if (taskId in state.activeStreamIds && state.activeStreamIds[taskId] !== streamId) {
    console.warn('[appendStreamChunk] dropping chunk streamId mismatch', { taskId, streamId, activeStreamId: state.activeStreamIds[taskId] });
    return state;
  }
  const id = `${streamId}:${type}`;
  const existing = state.streamEntries[taskId] || { thoughts: [], outputs: [] };
  const key = type === 'thought' ? 'thoughts' : 'outputs';
  const entries = existing[key];
  const feed = state.activityFeed[taskId] || [];
  const entryIndex = entries.findIndex((entry) => entry.id === id);
  const feedIndex = feed.findIndex((item) => item.id === id);
  const previous = feed[feedIndex] || entries[entryIndex];
  const now = Date.now();
  const createdAt = previous?.createdAt ?? now;
  const entry: LiveStreamEntry = {
    id,
    type,
    timestamp: previous?.timestamp || new Date(now).toLocaleTimeString('en-US', { hour12: false }),
    createdAt,
    content: (previous?.content || '') + content,
    ...(type === 'thought' ? { durationMs: Math.max(0, now - createdAt) } : {}),
  };
  return {
    connectionWait: { ...state.connectionWait, [taskId]: null },
    streamEntries: {
      ...state.streamEntries,
      [taskId]: {
        ...existing,
        [key]: entryIndex < 0 ? [...entries, entry] : entries.map((item, i) => i === entryIndex ? entry : item),
      },
    },
    activityFeed: {
      ...state.activityFeed,
      [taskId]: feedIndex < 0 ? [...feed, entry] : feed.map((item, i) => i === feedIndex ? { ...item, ...entry } : item),
    },
  };
}

export const createStreamSlice: StateCreator<
  DrawerState,
  [],
  [],
  StreamSlice
> = (set, get) => ({
  activeStreamIds: {},
  streamRollbackVersions: {},
  connectionWait: {},
  streamEntries: {},
  taskTelemetry: {},
  taskTelemetrySummary: {},
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

  startStream: (taskId, streamId) => {
    get().resetTurnStreamMetrics(taskId);
    set((state) => ({
      activeStreamIds: { ...state.activeStreamIds, [taskId]: streamId },
      connectionWait: { ...state.connectionWait, [taskId]: null },
      activeAgentTaskIds: { ...state.activeAgentTaskIds, [taskId]: true },
    }));
  },

  retryStream: (taskId, streamId, delaySecs) => {
    set((state) => {
      if (taskId in state.activeStreamIds && state.activeStreamIds[taskId] !== streamId) return state;
      const ids = new Set([`${streamId}:thought`, `${streamId}:output`]);
      const keys = [taskId, state.subagentIdToToolId[taskId], state.toolIdToSubagentId[taskId]].filter(Boolean);
      const activityFeed = { ...state.activityFeed };
      const streamEntries = { ...state.streamEntries };
      const streamRollbackVersions = { ...state.streamRollbackVersions };
      for (const key of keys) {
        streamRollbackVersions[key] = (streamRollbackVersions[key] || 0) + 1;
        if (activityFeed[key]) activityFeed[key] = activityFeed[key].filter((item) => !ids.has(item.id));
        const entries = streamEntries[key];
        if (entries) streamEntries[key] = {
          thoughts: entries.thoughts.filter((item) => !ids.has(item.id)),
          outputs: entries.outputs.filter((item) => !ids.has(item.id)),
        };
      }
      return {
        activityFeed,
        streamEntries,
        streamRollbackVersions,
        activeStreamIds: { ...state.activeStreamIds, [taskId]: null },
        connectionWait: { ...state.connectionWait, [taskId]: { streamId, delaySecs } },
        thinkingStartTime: { ...state.thinkingStartTime, [taskId]: null },
        generationEndTime: { ...state.generationEndTime, [taskId]: Date.now() },
        streamedTurnTokens: { ...state.streamedTurnTokens, [taskId]: 0 },
      };
    });
  },

  appendThought: (taskId, content, streamId) => {
    if (streamId) {
      set((state) => appendStreamChunk(state, taskId, streamId, 'thought', content));
      return;
    }
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
  },

  appendOutput: (taskId, content, type = 'output', images, streamId) => {
    if (streamId && type === 'output') {
      set((state) => appendStreamChunk(state, taskId, streamId, 'output', content));
      return;
    }
    const timeStr = new Date().toLocaleTimeString('en-US', { hour12: false });

    set((state) => {
      const existing = state.streamEntries[taskId] || { thoughts: [], outputs: [] };
      const last = existing.outputs[existing.outputs.length - 1];

      const existingFeed = state.activityFeed[taskId] || [];
      const lastFeedItem = existingFeed[existingFeed.length - 1];

      const activityType: ActivityType =
        type === 'steer' ? 'steer' : type === 'system' ? 'system' : type === 'error' ? 'error' : type === 'user' ? 'user' : 'output';

      // User submissions have their own identity, even when their text repeats.
      if (!images?.length && last && last.type === type && type === 'output') {
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
      const totalInput = merged.inputTokens || 0;
      merged.cacheHitRate = totalInput > 0 ? Math.min(100, ((merged.cacheReadTokens || 0) / totalInput) * 100) : 0;
      return {
        taskTelemetry: {
          ...state.taskTelemetry,
          [taskId]: merged,
        },
      };
    });
  },

  setTaskTelemetrySummary: (taskId, summary) => {
    set((state) => ({
      taskTelemetrySummary: {
        ...state.taskTelemetrySummary,
        [taskId]: summary,
      },
    }));
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
      connectionWait: { ...state.connectionWait, [taskId]: null },
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

  setTaskTokens: (taskId, tokens) => {
    set((state) => ({
      tokenOverrides: {
        ...state.tokenOverrides,
        [taskId]: tokens,
      },
    }));
  },
});

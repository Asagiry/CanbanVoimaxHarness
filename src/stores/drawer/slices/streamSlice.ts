import { StateCreator } from 'zustand';
import { StreamSlice, DrawerState } from '../types';
import {
  setBufferedChunkApplier,
  appendStreamChunk,
  enqueueStreamChunk,
  dropBufferedStreamChunks,
  flushStreamChunks,
  clearStreamChunkBuffer,
} from './stream/chunkBuffer';
import { createStreamAppendActions } from './stream/appendActions';
import { createStreamTelemetryActions } from './stream/telemetryActions';

export {
  enqueueStreamChunk,
  dropBufferedStreamChunks,
  flushStreamChunks,
  clearStreamChunkBuffer,
};

export const createStreamSlice: StateCreator<
  DrawerState,
  [],
  [],
  StreamSlice
> = (set, get) => {
  setBufferedChunkApplier((chunks) => {
    set((state) => {
      let next: DrawerState = state;
      for (const chunk of chunks) {
        const partial = appendStreamChunk(next, chunk.taskId, chunk.streamId, chunk.type, chunk.content);
        if (partial !== next) next = { ...next, ...partial };
      }
      return next;
    });
  });

  return {
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
      flushStreamChunks(taskId);
      dropBufferedStreamChunks(taskId, streamId);
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

    ...createStreamAppendActions(set),
    ...createStreamTelemetryActions(set),
  };
};

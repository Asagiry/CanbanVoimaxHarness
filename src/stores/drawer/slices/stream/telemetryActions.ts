import { StateCreator } from 'zustand';
import { StreamSlice, DrawerState } from '../../types';

type DrawerSet = Parameters<StateCreator<DrawerState, [], [], StreamSlice>>[0];

export function createStreamTelemetryActions(
  set: DrawerSet,
): Pick<
  StreamSlice,
  | 'updateTelemetry'
  | 'setTaskTelemetrySummary'
  | 'setThinkingStartTime'
  | 'setGenerationStartTime'
  | 'setGenerationEndTime'
  | 'setTaskEndTime'
  | 'recordStreamedChars'
  | 'resetTurnStreamMetrics'
  | 'setFinalTps'
  | 'recordTurnMetrics'
  | 'setTaskTokens'
> {
  return {
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
  };
}

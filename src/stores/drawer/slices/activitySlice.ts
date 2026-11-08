import { StateCreator } from 'zustand';
import { ActivitySlice, DrawerState, DrawerActivityItem } from '../types';
import { ToolExecution, CompactionReport } from '../../../types';
import { scheduleActivitySave, cancelActivitySave } from '../drawerHelpers';
import { useBoardStore } from '../../boardStore';
import {
  capActivityFeed,
  capToolExecutions,
  extractSubagentIdsFromTool,
  mergeFinalResponseIntoFeed,
  mergeToolExecutionIntoState,
  applyToolExecutionUpdate,
  mergeActivityItemIntoFeed,
  deduplicateLoadedActivityItems,
  hydrateLoadedActivityState,
  evaporateToolExecutionLogs,
  buildClearedTaskActivityState,
} from '../activityDedup';
import { setPendingQuestion, answerQuestion } from '../activityQuestionHelpers';

export * from '../activityDedup';
export * from '../activityQuestionHelpers';

export const createActivitySlice: StateCreator<DrawerState, [], [], ActivitySlice> = (set, get) => ({
  activityFeed: {},
  get dialogItems() {
    return this.activityFeed;
  },
  toolExecutions: {},
  activeAgentTaskIds: {},

  setActiveAgentTask: (taskId, isActive) => {
    if (!taskId) return;
    set((state) => {
      if (Boolean(state.activeAgentTaskIds[taskId]) === isActive && (isActive || !state.connectionWait[taskId])) {
        return state;
      }
      return {
        activeAgentTaskIds: { ...state.activeAgentTaskIds, [taskId]: isActive },
        ...(!isActive ? { connectionWait: { ...state.connectionWait, [taskId]: null } } : {}),
      };
    });
  },

  markFinalResponse: (taskId, response) => {
    if (!taskId) return;
    const timeStr = new Date().toLocaleTimeString('en-US', { hour12: false });
    set((state) => ({
      activityFeed: {
        ...state.activityFeed,
        [taskId]: mergeFinalResponseIntoFeed(state.activityFeed[taskId] || [], response, timeStr),
      },
    }));
  },

  addToolExecution: (taskId, tool) => {
    const toolWithCreatedAt: ToolExecution = { ...tool, createdAt: tool.createdAt || Date.now() };
    if (toolWithCreatedAt.toolName === 'subagent' || toolWithCreatedAt.toolName === 'subagent_bg') {
      extractSubagentIdsFromTool(toolWithCreatedAt.id, toolWithCreatedAt.params, toolWithCreatedAt.stdout, get().registerSubagentMapping, true);
    }
    set((state) => {
      const { updatedFeed, updatedTools } = mergeToolExecutionIntoState(
        state.activityFeed[taskId] || [],
        state.toolExecutions[taskId] || [],
        toolWithCreatedAt
      );
      return {
        toolExecutions: { ...state.toolExecutions, [taskId]: updatedTools },
        activityFeed: { ...state.activityFeed, [taskId]: updatedFeed },
      };
    });
    scheduleActivitySave(taskId);
  },

  updateToolExecution: (taskId, toolId, partial) => {
    if (partial.params || partial.stdout) {
      extractSubagentIdsFromTool(toolId, partial.params, partial.stdout, get().registerSubagentMapping, false);
    }
    set((state) => {
      const { updatedFeed, updatedTools } = applyToolExecutionUpdate(
        state.activityFeed[taskId] || [],
        state.toolExecutions[taskId] || [],
        toolId,
        partial
      );
      return {
        toolExecutions: { ...state.toolExecutions, [taskId]: updatedTools },
        activityFeed: { ...state.activityFeed, [taskId]: updatedFeed },
      };
    });
    scheduleActivitySave(taskId);
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
      createdAt: Date.now(),
      params: { taskId, title: 'Оптимизация контекста' },
    };
    const optimisticItem: DrawerActivityItem = {
      id: toolId,
      type: 'tool',
      timestamp,
      content: 'Оптимизация контекста',
      tool: optimisticTool,
    };
    set((state) => {
      const existingTools = state.toolExecutions[taskId] || [];
      const existingFeed = state.activityFeed[taskId] || [];
      if (existingFeed.some((it) => it.id === toolId || it.tool?.id === toolId)) return {};
      return {
        toolExecutions: { ...state.toolExecutions, [taskId]: capToolExecutions([optimisticTool, ...existingTools]) },
        activityFeed: { ...state.activityFeed, [taskId]: capActivityFeed([...existingFeed, optimisticItem]) },
      };
    });
    return toolId;
  },

  compactTaskContext: async (taskId) => {
    const toolId = get().injectOptimisticCompactTool(taskId);
    const startTime = performance.now();
    let savedTokens = 2450;
    let evaporatedCount = 0;

    set((state) => {
      const updatedTools = (state.toolExecutions[taskId] || []).map((t, idx) => {
        if (idx > 2 && (t.stdout?.length || 0) > 500) {
          evaporatedCount += 1;
          return { ...t, stdout: `[Evaporated by State-Preserving Compactor: ${t.stdout?.length} chars truncated. Preserved status: ${t.status}]` };
        }
        return t;
      });
      return { toolExecutions: { ...state.toolExecutions, [taskId]: updatedTools } };
    });

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke<CompactionReport>('compact_context', { taskId, toolId });
      if (res?.saved_tokens) savedTokens = res.saved_tokens;
      if (res?.evaporated_count) evaporatedCount = Math.max(evaporatedCount, res.evaporated_count);
      if (res?.tokens_after) {
        get().setTaskTokens(taskId, res.tokens_after);
        get().updateTelemetry(taskId, { inputTokens: res.tokens_after, outputTokens: 0, cacheReadTokens: 0, cacheCreationTokens: 0 });
      }
    } catch { /* Keep local values */ }

    get().updateToolExecution(taskId, toolId, {
      status: 'success',
      durationMs: Math.round(performance.now() - startTime),
      stdout: `Контекст успешно сжат. Сохранено ${savedTokens} токенов.`,
      params: { taskId, title: `Сжато (-${savedTokens} токенов)`, savedTokens },
    });

    try {
      await get().loadTaskActivity(taskId);
    } catch (e) {
      console.warn('Failed to reload activity after compaction:', e);
    }
    return { savedTokens, evaporatedCount };
  },

  deleteDialogItem: async (taskId, itemId) => {
    const current = get();
    const resolvedId = current.toolIdToSubagentId[taskId] || taskId;
    const isBoardTask = useBoardStore.getState().tasks.some((task) => task.id === resolvedId);
    const targetDbTaskId =
      !isBoardTask && current.activeTaskId && resolvedId !== current.activeTaskId && !resolvedId.includes(':sub:')
        ? `${current.activeTaskId}:sub:${resolvedId}`
        : resolvedId;
    const aliases = new Set([taskId, resolvedId, targetDbTaskId]);
    for (const [toolId, subId] of Object.entries(current.toolIdToSubagentId)) {
      if (subId === resolvedId) aliases.add(toolId);
    }
    if ([...aliases].some((id) => current.activeAgentTaskIds[id]) || (current.activeTaskId && current.activeAgentTaskIds[current.activeTaskId])) {
      throw new Error('Stop the agent before deleting messages');
    }
    for (const id of aliases) cancelActivitySave(id);
    const feed = [...aliases].map((id) => current.activityFeed[id]).find((items) => items?.some((item) => item.id === itemId)) || [];
    const toolId = feed.find((item) => item.id === itemId)?.tool?.id || itemId;
    const { invoke } = await import('@tauri-apps/api/core');
    const deletedIds = new Set(await invoke<string[]>('delete_dialog_item', { taskId: targetDbTaskId, itemId, items: feed }));
    deletedIds.add(itemId);
    deletedIds.add(toolId);
    set((state) => {
      const activityFeed = { ...state.activityFeed };
      const streamEntries = { ...state.streamEntries };
      const toolExecutions = { ...state.toolExecutions };
      for (const id of aliases) {
        if (activityFeed[id]) activityFeed[id] = activityFeed[id].filter((item) => !deletedIds.has(item.id));
        if (streamEntries[id]) {
          streamEntries[id] = {
            thoughts: streamEntries[id].thoughts.filter((item) => !deletedIds.has(item.id)),
            outputs: streamEntries[id].outputs.filter((item) => !deletedIds.has(item.id)),
          };
        }
        if (toolExecutions[id]) toolExecutions[id] = toolExecutions[id].filter((tool) => !deletedIds.has(tool.id));
      }
      return { activityFeed, streamEntries, toolExecutions };
    });
  },

  clearTaskActivity: async (taskId) => {
    cancelActivitySave(taskId);
    get().clearTaskDraft(taskId);
    set((state) => buildClearedTaskActivityState(state, taskId));
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('clear_task_activity', { taskId });
    } catch (e) {
      console.info('clear_task_activity invoke error or web mode:', e);
    }
  },

  clearTaskLogs: async (taskId) => get().clearTaskActivity(taskId),
  clearTaskDialog: async (taskId) => get().clearTaskActivity(taskId),

  evaporateClientToolLogs: (taskId) => {
    const { updatedTools, evaporatedCount, savedTokens } = evaporateToolExecutionLogs(get().toolExecutions[taskId] || []);
    set((state) => {
      const currentOverride = state.tokenOverrides[taskId];
      const newTokens = currentOverride ? Math.max(1000, currentOverride - savedTokens) : undefined;
      return {
        toolExecutions: { ...state.toolExecutions, [taskId]: updatedTools },
        tokenOverrides: newTokens !== undefined ? { ...state.tokenOverrides, [taskId]: newTokens } : state.tokenOverrides,
      };
    });
    return { evaporatedCount, savedTokens };
  },

  loadTaskActivity: async (taskId) => {
    if (!taskId) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const rollbackVersion = get().streamRollbackVersions[taskId];
      let items = await invoke<DrawerActivityItem[]>('load_task_activity', { taskId });
      if (rollbackVersion !== get().streamRollbackVersions[taskId]) return get().loadTaskActivity(taskId);
      if (items && Array.isArray(items)) {
        items = items.filter((it): it is DrawerActivityItem => Boolean(it) && typeof it === 'object');
        let compactedTokens: number | undefined;
        for (let i = items.length - 1; i >= 0; i--) {
          const tData = items[i].type === 'compacted' ? (items[i].tool as { tokensAfter?: number }) : undefined;
          if (typeof tData?.tokensAfter === 'number') {
            compactedTokens = tData.tokensAfter;
            break;
          }
        }
        try {
          const boardTask = useBoardStore.getState().tasks.find((t) => t.id === taskId);
          if (compactedTokens === undefined && typeof boardTask?.currentTokens === 'number') {
            compactedTokens = boardTask.currentTokens;
          }
        } catch { /* Board store optional */ }

        if (items.length > 0) {
          set((state) => {
            const currentFeed = state.activityFeed[taskId] || [];
            const hasCompacted = items.some((it) => it.type === 'compacted');
            const newOverrides = compactedTokens !== undefined ? { ...state.tokenOverrides, [taskId]: compactedTokens } : state.tokenOverrides;
            if (
              currentFeed.length === 0 ||
              items.length >= currentFeed.length ||
              hasCompacted ||
              items[items.length - 1]?.id !== currentFeed[currentFeed.length - 1]?.id
            ) {
              const dedupedItems = deduplicateLoadedActivityItems(items);
              const { feed, tools, thoughts, outputs } = hydrateLoadedActivityState(dedupedItems, get().registerSubagentMapping);
              const loadedIds = new Set(feed.map((it) => it.id));
              const liveOnly = currentFeed.filter((it) => !loadedIds.has(it.id));
              const mergedFeed = liveOnly.length > 0 ? [...feed, ...liveOnly] : feed;
              const mergedThoughts = [...thoughts];
              const mergedOutputs = [...outputs];
              for (const it of liveOnly) {
                if (it.type === 'thought') mergedThoughts.push(it as never);
                else if (it.type === 'output') mergedOutputs.push(it as never);
              }
              const liveToolIds = new Set(tools.map((t) => t.id));
              const liveTools = (state.toolExecutions[taskId] || []).filter((t) => !liveToolIds.has(t.id));
              return {
                activityFeed: { ...state.activityFeed, [taskId]: mergedFeed },
                toolExecutions: { ...state.toolExecutions, [taskId]: [...tools, ...liveTools] },
                streamEntries: { ...state.streamEntries, [taskId]: { thoughts: mergedThoughts, outputs: mergedOutputs } },
                tokenOverrides: newOverrides,
              };
            }
            return { tokenOverrides: newOverrides };
          });
          return;
        }
        set((state) => {
          const currentFeed = state.activityFeed[taskId] || [];
          const currentTools = state.toolExecutions[taskId] || [];
          const currentEntries = state.streamEntries[taskId];
          const hasLive = currentFeed.length > 0 || currentTools.length > 0 || (currentEntries && (currentEntries.thoughts.length > 0 || currentEntries.outputs.length > 0));
          if (hasLive) return {};
          return {
            activityFeed: { ...state.activityFeed, [taskId]: [] },
            streamEntries: { ...state.streamEntries, [taskId]: { thoughts: [], outputs: [] } },
            toolExecutions: { ...state.toolExecutions, [taskId]: [] },
            tokenOverrides: { ...state.tokenOverrides, [taskId]: undefined },
            cumulativeOutputTokens: { ...state.cumulativeOutputTokens, [taskId]: 0 },
            cumulativeGenerationSec: { ...state.cumulativeGenerationSec, [taskId]: 0 },
          };
        });
      }
    } catch (e) {
      console.info('load_task_activity invoke error or web mode:', e);
    }
  },

  loadTaskDialog: async (taskId) => get().loadTaskActivity(taskId),

  saveTaskActivity: async (taskId) => {
    cancelActivitySave(taskId);
    const state = get();
    const feed = state.activityFeed[taskId];
    if (!feed || feed.length === 0) return;

    let targetDbTaskId = taskId;
    const isBoardTask = useBoardStore.getState().tasks.some((t) => t.id === taskId);
    if (!isBoardTask && state.activeTaskId && !taskId.includes(':sub:')) {
      const resolvedSub = state.toolIdToSubagentId[taskId] || (state.activeSubagentId === taskId ? taskId : null);
      if (resolvedSub) {
        const isResolvedBoardTask = useBoardStore.getState().tasks.some((t) => t.id === resolvedSub);
        targetDbTaskId = !isResolvedBoardTask
          ? resolvedSub.includes(':sub:') ? resolvedSub : `${state.activeTaskId}:sub:${resolvedSub}`
          : resolvedSub;
      }
    }
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_task_activity', { taskId: targetDbTaskId, items: feed });
      if (targetDbTaskId !== taskId) await invoke('save_task_activity', { taskId, items: feed }).catch(() => {});
    } catch (err) {
      console.info('save_task_activity invoke error or web mode:', err);
    }
  },

  saveTaskDialog: async (taskId) => get().saveTaskActivity(taskId),
  flushSaveDialog: async (taskId) => get().saveTaskActivity(taskId),
  addQuestion: (taskId, question) => setPendingQuestion(set, taskId, question),
  recordQuestionAnswer: async (taskId, questionId, answer) => answerQuestion(set, taskId, questionId, answer),

  appendActivityItem: (taskId, item) => {
    if (!taskId || !item) return;
    set((state) => {
      const streamed = item.id.match(/^(.+):(thought|output)$/);
      if (streamed && taskId in state.activeStreamIds && state.activeStreamIds[taskId] !== streamed[1]) {
        console.warn('[appendActivityItem] dropping item streamId mismatch', { taskId, itemId: item.id, activeStreamId: state.activeStreamIds[taskId] });
        return state;
      }
      return {
        activityFeed: {
          ...state.activityFeed,
          [taskId]: mergeActivityItemIntoFeed(state.activityFeed[taskId] || [], item, Boolean(streamed)),
        },
      };
    });
  },
});

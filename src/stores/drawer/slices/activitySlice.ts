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
  evaporateToolExecutionLogs,
  buildClearedTaskActivityState,
} from '../activityDedup';
import { buildLoadedTaskActivityPatch, MAX_LOAD_ROLLBACK_RETRIES } from '../activityLoadHelpers';
import { setPendingQuestion, answerQuestion } from '../activityQuestionHelpers';
import { flushStreamChunks } from './streamSlice';

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
    if (!isActive) flushStreamChunks(taskId);
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
      extractSubagentIdsFromTool(
        toolWithCreatedAt.id,
        toolWithCreatedAt.params,
        toolWithCreatedAt.stdout,
        get().registerSubagentMapping,
        true,
      );
    }
    set((state) => {
      const { updatedFeed, updatedTools } = mergeToolExecutionIntoState(
        state.activityFeed[taskId] || [],
        state.toolExecutions[taskId] || [],
        toolWithCreatedAt,
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
        partial,
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

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke<CompactionReport>('compact_context', { taskId, toolId });
      const savedTokens = typeof res?.saved_tokens === 'number' && Number.isFinite(res.saved_tokens)
        ? res.saved_tokens
        : 0;
      const evaporatedCount = typeof res?.evaporated_count === 'number' && Number.isFinite(res.evaporated_count)
        ? res.evaporated_count
        : 0;

      if (typeof res?.tokens_after === 'number' && res.tokens_after > 0) {
        get().setTaskTokens(taskId, res.tokens_after);
        get().updateTelemetry(taskId, {
          inputTokens: res.tokens_after,
          outputTokens: 0,
          cacheReadTokens: 0,
          cacheCreationTokens: 0,
        });
      }

      get().updateToolExecution(taskId, toolId, {
        status: 'success',
        durationMs: Math.round(performance.now() - startTime),
        stdout: `Контекст успешно сжат. Сохранено ${savedTokens} токенов.`,
        params: { taskId, title: `Сжато (-${savedTokens} токенов)`, savedTokens },
      });

      try {
        await get().loadTaskActivity(taskId);
      } catch (reloadErr) {
        console.warn('Failed to reload activity after compaction:', reloadErr);
      }
      return { savedTokens, evaporatedCount };
    } catch (err) {
      const errMsg = err instanceof Error ? err.message : String(err);
      get().updateToolExecution(taskId, toolId, {
        status: 'failed',
        durationMs: Math.round(performance.now() - startTime),
        stderr: errMsg,
      });
      throw err instanceof Error ? err : new Error(errMsg);
    }
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
      let items: DrawerActivityItem[] | null = null;
      for (let attempt = 0; attempt < MAX_LOAD_ROLLBACK_RETRIES; attempt++) {
        const rollbackVersion = get().streamRollbackVersions[taskId];
        const loaded = await invoke<DrawerActivityItem[]>('load_task_activity', { taskId });
        if (rollbackVersion === get().streamRollbackVersions[taskId]) {
          items = loaded;
          break;
        }
      }
      if (!items || !Array.isArray(items)) return;
      const validItems = items.filter((it): it is DrawerActivityItem => Boolean(it) && typeof it === 'object');
      set((state) => buildLoadedTaskActivityPatch(state, taskId, validItems, get().registerSubagentMapping));
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
      if (targetDbTaskId !== taskId) {
        try {
          await invoke('save_task_activity', { taskId, items: feed });
        } catch (aliasErr) {
          console.warn('save_task_activity alias sync failed:', aliasErr);
        }
      }
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
        console.warn('[appendActivityItem] dropping item streamId mismatch', {
          taskId,
          itemId: item.id,
          activeStreamId: state.activeStreamIds[taskId],
        });
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

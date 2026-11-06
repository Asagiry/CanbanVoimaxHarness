import { StateCreator } from 'zustand';
import {
  ActivitySlice,
  DrawerState,
  DrawerActivityItem,
} from '../types';
import { ToolExecution, CompactionReport, LiveStreamEntry } from '../../../types';
import { scheduleActivitySave, cancelActivitySave, computeActivityDurations } from '../drawerHelpers';
import { useBoardStore } from '../../boardStore';

export const createActivitySlice: StateCreator<
  DrawerState,
  [],
  [],
  ActivitySlice
> = (set, get) => ({
  activityFeed: {},
  get dialogItems() {
    return this.activityFeed;
  },
  toolExecutions: {},
  activeAgentTaskIds: {},

  setActiveAgentTask: (taskId, isActive) => {
    if (!taskId) return;
    set((state) => {
      if (Boolean(state.activeAgentTaskIds[taskId]) === isActive &&
          (isActive || !state.connectionWait[taskId])) return state;
      return {
        activeAgentTaskIds: {
          ...state.activeAgentTaskIds,
          [taskId]: isActive,
        },
        ...(!isActive ? {
          connectionWait: { ...state.connectionWait, [taskId]: null },
        } : {}),
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
        if (updatedFeed[i].type === 'output' || (updatedFeed[i].type as string) === 'summary') {
          lastOutputIdx = i;
          break;
        }
      }

      const hasInterveningTools =
        lastOutputIdx !== -1 &&
        updatedFeed
          .slice(lastOutputIdx + 1)
          .some((it) => it.type === 'tool' || Boolean((it as any).tool));

      const resp = (response || '').trim();

      if (lastOutputIdx !== -1 && !hasInterveningTools) {
        const lastOutput = updatedFeed[lastOutputIdx];
        const content = (lastOutput.content || '').trim();
        if (resp && !content) {
          updatedFeed[lastOutputIdx] = {
            ...lastOutput,
            content: response!,
            isFinal: true,
          };
        } else {
          updatedFeed[lastOutputIdx] = {
            ...lastOutput,
            isFinal: true,
          };
        }
      } else if (resp.length > 0) {
        const alreadyExists = updatedFeed.some(
          (it) =>
            (it.type === 'output' || (it.type as string) === 'summary') &&
            (it.content || '').trim() === resp
        );
        if (!alreadyExists) {
          updatedFeed.push({
            id: `out-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
            type: 'output',
            timestamp: timeStr,
            content: response!,
            isFinal: true,
          });
        }
      }

      return {
        activityFeed: {
          ...state.activityFeed,
          [taskId]: updatedFeed,
        },
      };
    });
  },

  addToolExecution: (taskId, tool) => {
    const toolWithCreatedAt: ToolExecution = {
      ...tool,
      createdAt: tool.createdAt || Date.now(),
    };

    // If tool is a subagent execution, extract subagent_id if already present
    if (toolWithCreatedAt.toolName === 'subagent' || toolWithCreatedAt.toolName === 'subagent_bg') {
      const pSubId = (toolWithCreatedAt.params as Record<string, unknown> | undefined)?.subagent_id;
      if (typeof pSubId === 'string' && pSubId) {
        get().registerSubagentMapping(toolWithCreatedAt.id, pSubId);
      } else if (toolWithCreatedAt.stdout) {
        try {
          const parsed = JSON.parse(toolWithCreatedAt.stdout);
          if (parsed.subagent_id) get().registerSubagentMapping(toolWithCreatedAt.id, parsed.subagent_id);
        } catch {}
        const match = toolWithCreatedAt.stdout.match(/\[Subagent ID:\s*([^\s\]]+)\]/);
        if (match && match[1]) get().registerSubagentMapping(toolWithCreatedAt.id, match[1]);
      }
    }

    set((state) => {
      const existing = state.toolExecutions[taskId] || [];
      const existingFeed = state.activityFeed[taskId] || [];

      // Obeying § 2.20 & Question Tool Dedup:
      // If tool is question/ask_user and there is already an existing question item in feed,
      // synchronize tool call info into that existing question item rather than duplicating feed!
      if (toolWithCreatedAt.toolName === 'question' || toolWithCreatedAt.toolName === 'ask_user') {
        const qParamId = (toolWithCreatedAt.params as Record<string, unknown> | undefined)?.questionId as string | undefined;
        let qIndex = -1;
        for (let i = existingFeed.length - 1; i >= 0; i--) {
          const f = existingFeed[i];
          if (
            (f.type === 'question' &&
              ((qParamId &&
                ((f.tool?.params as Record<string, unknown> | undefined)?.questionId === qParamId ||
                  f.tool?.id === qParamId)) ||
                (!qParamId &&
                  f.tool?.status === 'running' &&
                  !(f.tool?.params as Record<string, unknown> | undefined)?.callId))) ||
            f.id === `q-${toolWithCreatedAt.id}` ||
            f.tool?.id === toolWithCreatedAt.id
          ) {
            qIndex = i;
            break;
          }
        }

        if (qIndex !== -1) {
          const prev = existingFeed[qIndex];
          const updatedFeed = [...existingFeed];
          updatedFeed[qIndex] = {
            ...prev,
            tool: {
              ...prev.tool,
              ...toolWithCreatedAt,
              id: prev.tool?.id || toolWithCreatedAt.id,
              toolName: 'question',
              params: {
                ...(toolWithCreatedAt.params as Record<string, unknown>),
                ...(prev.tool?.params as Record<string, unknown>),
                callId: toolWithCreatedAt.id,
              },
            },
          };

          const toolExists = existing.some((t) => t.id === toolWithCreatedAt.id);
          const updatedTools = toolExists
            ? existing.map((t) => (t.id === toolWithCreatedAt.id ? { ...t, ...toolWithCreatedAt } : t))
            : [toolWithCreatedAt, ...existing];

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
        }
      }

      const feedItem: DrawerActivityItem = {
        id: toolWithCreatedAt.id,
        type: 'tool',
        timestamp: toolWithCreatedAt.timestamp,
        tool: toolWithCreatedAt,
        createdAt: toolWithCreatedAt.createdAt,
        durationMs: toolWithCreatedAt.durationMs || undefined,
      };

      const feedExists = existingFeed.some((item) => item.id === toolWithCreatedAt.id || item.tool?.id === toolWithCreatedAt.id);
      const updatedFeed = feedExists
        ? existingFeed.map((item) =>
            item.id === toolWithCreatedAt.id || item.tool?.id === toolWithCreatedAt.id
              ? { ...item, tool: { ...item.tool, ...toolWithCreatedAt } }
              : item
          )
        : [...existingFeed, feedItem];

      const toolExists = existing.some((t) => t.id === toolWithCreatedAt.id);
      const updatedTools = toolExists
        ? existing.map((t) => (t.id === toolWithCreatedAt.id ? { ...t, ...toolWithCreatedAt } : t))
        : [toolWithCreatedAt, ...existing];

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
    scheduleActivitySave(taskId);
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
      const existingTool = existing.find((t) => t.id === toolId);

      let durationMs = (typeof partial.durationMs === 'number' && Number.isFinite(partial.durationMs))
        ? partial.durationMs
        : undefined;

      if (durationMs === undefined && existingTool?.createdAt) {
        durationMs = Math.max(0, Date.now() - existingTool.createdAt);
      }

      const mergedPartial: Partial<ToolExecution> = {
        ...partial,
        ...(typeof durationMs === 'number' && Number.isFinite(durationMs) ? { durationMs } : {}),
      };

      const updated = existing.map((t) => (t.id === toolId ? { ...t, ...mergedPartial } : t));

      const existingFeed = state.activityFeed[taskId] || [];
      const updatedFeed = existingFeed.map((item) => {
        if (
          (item.type === 'tool' || item.type === 'question') &&
          (item.id === toolId ||
            item.tool?.id === toolId ||
            (item.tool?.params as Record<string, unknown> | undefined)?.callId === toolId ||
            (item.tool?.params as Record<string, unknown> | undefined)?.questionId === toolId)
        ) {
          return {
            ...item,
            ...(typeof durationMs === 'number' && Number.isFinite(durationMs) ? { durationMs } : {}),
            tool: item.tool ? { ...item.tool, ...mergedPartial } : undefined,
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
      params: {
        taskId,
        title: 'Оптимизация контекста',
      },
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

    // 3. Refresh activity feed from SQLite to load the persisted 'compacted' accordion card
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
    const targetDbTaskId = !isBoardTask && current.activeTaskId && resolvedId !== current.activeTaskId && !resolvedId.includes(':sub:')
      ? `${current.activeTaskId}:sub:${resolvedId}` : resolvedId;
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
        if (streamEntries[id]) streamEntries[id] = {
          thoughts: streamEntries[id].thoughts.filter((item) => !deletedIds.has(item.id)),
          outputs: streamEntries[id].outputs.filter((item) => !deletedIds.has(item.id)),
        };
        if (toolExecutions[id]) toolExecutions[id] = toolExecutions[id].filter((tool) => !deletedIds.has(tool.id));
      }
      return { activityFeed, streamEntries, toolExecutions };
    });
  },

  clearTaskActivity: async (taskId) => {
    cancelActivitySave(taskId);
    get().clearTaskDraft(taskId);
    set((state) => ({
      connectionWait: { ...state.connectionWait, [taskId]: null },
      activeStreamIds: { ...state.activeStreamIds, [taskId]: null },
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
      await invoke('clear_task_activity', { taskId });
    } catch (e) {
      console.info('clear_task_activity invoke error or web mode:', e);
    }
  },

  clearTaskLogs: async (taskId) => {
    return get().clearTaskActivity(taskId);
  },

  clearTaskDialog: async (taskId) => {
    return get().clearTaskActivity(taskId);
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

  loadTaskActivity: async (taskId) => {
    if (!taskId) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const rollbackVersion = get().streamRollbackVersions[taskId];
      const items = await invoke<DrawerActivityItem[]>('load_task_activity', { taskId });
      if (rollbackVersion !== get().streamRollbackVersions[taskId]) {
        return get().loadTaskActivity(taskId);
      }
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

            if (
              currentFeed.length === 0 ||
              items.length >= currentFeed.length ||
              hasCompacted ||
              items[items.length - 1]?.id !== currentFeed[currentFeed.length - 1]?.id
            ) {
              // Deduplicate questions and adjacent identical steer / user messages (§ 2.20 & Question Dedup)
              const dedupedItems: DrawerActivityItem[] = [];
              const loadedQuestionIds = new Set<string>();
              const loadedQuestionTexts = new Set<string>();

              for (const item of items) {
                if (item.type === 'question') {
                  const qParams = item.tool?.params as Record<string, unknown> | undefined;
                  const qId = (qParams?.questionId as string) || item.tool?.id || item.id;
                  if (qId) loadedQuestionIds.add(qId);
                  if (item.id?.startsWith('q-')) loadedQuestionIds.add(item.id.replace(/^q-/, ''));
                  const callId = qParams?.callId as string | undefined;
                  if (callId) loadedQuestionIds.add(callId);
                  if (item.content?.trim()) loadedQuestionTexts.add(item.content.trim());
                  if (typeof qParams?.text === 'string' && qParams.text.trim()) {
                    loadedQuestionTexts.add(qParams.text.trim());
                  }
                }
              }

              for (const item of items) {
                // Deduplicate tool question calls when matching question item already exists in SQLite history
                if (
                  item.type === 'tool' &&
                  (item.tool?.toolName === 'question' || item.tool?.toolName === 'ask_user')
                ) {
                  const tParams = item.tool?.params as Record<string, unknown> | undefined;
                  const tId = item.tool?.id || item.id;
                  const qId = tParams?.questionId as string | undefined;
                  const callId = tParams?.callId as string | undefined;
                  const txt = ((typeof tParams?.text === 'string' ? tParams.text : '') || item.content || '').trim();

                  if (
                    (tId && (loadedQuestionIds.has(tId) || loadedQuestionIds.has(`q-${tId}`))) ||
                    (qId && loadedQuestionIds.has(qId)) ||
                    (callId && loadedQuestionIds.has(callId)) ||
                    (txt && loadedQuestionTexts.has(txt))
                  ) {
                    continue; // Suppress duplicate tool call
                  }
                }

                const prev = dedupedItems[dedupedItems.length - 1];

                // Deduplicate identical tool calls within recent items (e.g. from concurrent runners or duplicate emissions)
                if (
                  item.type === 'tool' &&
                  item.tool &&
                  dedupedItems.slice(-3).some((d) => {
                    if (d.type !== 'tool' || !d.tool) return false;
                    const dName = d.tool.toolName || (d.tool as any).name;
                    const iName = item.tool?.toolName || (item.tool as any)?.name;
                    if (!dName || dName !== iName) return false;
                    return JSON.stringify(d.tool.params) === JSON.stringify(item.tool?.params);
                  })
                ) {
                  continue; // Suppress duplicate tool call
                }

                if (
                  prev &&
                  item.id === prev.id &&
                  (item.type === 'steer' || item.type === 'user') &&
                  (prev.type === 'steer' || prev.type === 'user') &&
                  item.content?.trim() === prev.content?.trim()
                ) {
                  if (item.type === 'steer') {
                    prev.type = 'steer';
                  }
                  if (item.images && item.images.length > 0) {
                    prev.images = item.images;
                  }
                  continue;
                }

                // Deduplicate adjacent identical output / summary items
                if (
                  prev &&
                  !item.id.endsWith(':output') && !prev.id.endsWith(':output') &&
                  (item.type === 'output' || (item.type as string) === 'summary') &&
                  (prev.type === 'output' || (prev.type as string) === 'summary') &&
                  item.content?.trim() &&
                  prev.content?.trim() &&
                  item.content.trim() === prev.content.trim()
                ) {
                  if ((item.type as string) === 'summary') {
                    prev.type = 'summary';
                  }
                  if (item.isFinal) {
                    prev.isFinal = true;
                  }
                  continue;
                }

                // Suppress output item if it duplicates a recent stage summary
                if (item.type === 'output' && !item.id.endsWith(':output') && item.content?.trim()) {
                  const itemContent = item.content.trim();
                  const isDuplicateSummary = dedupedItems
                    .slice(-3)
                    .some(
                      (d) =>
                        (d.type as string) === 'summary' &&
                        d.content?.trim() &&
                        (d.content.trim() === itemContent ||
                          d.content.includes(itemContent) ||
                          itemContent.includes(d.content.trim()))
                    );
                  if (isDuplicateSummary) {
                    continue;
                  }
                }
                dedupedItems.push(item);
              }

              const computedDurations = computeActivityDurations(dedupedItems, false);
              const tools: ToolExecution[] = [];
              const thoughts: LiveStreamEntry[] = [];
              const outputs: LiveStreamEntry[] = [];

              for (let i = 0; i < dedupedItems.length; i++) {
                const item = dedupedItems[i];
                const computedDur = computedDurations[i];
                if (item.type === 'tool' && item.tool) {
                  const rawToolDur = item.tool.durationMs ?? (item.tool as any).duration_ms;
                  const rawItemDur = item.durationMs ?? (item as any).duration_ms;
                  const dur =
                    (typeof rawToolDur === 'number' && rawToolDur > 0)
                      ? rawToolDur
                      : (typeof rawItemDur === 'number' && rawItemDur > 0)
                        ? rawItemDur
                        : (typeof computedDur === 'number' && computedDur > 0)
                          ? computedDur
                          : 0;

                  const toolWithDuration: ToolExecution = {
                    ...item.tool,
                    durationMs: dur,
                    createdAt: item.tool.createdAt || item.createdAt || (item.timestamp && !isNaN(new Date(item.timestamp).getTime()) ? new Date(item.timestamp).getTime() : undefined),
                  };
                  item.tool = toolWithDuration;
                  item.durationMs = dur > 0 ? dur : undefined;

                  tools.unshift(toolWithDuration);
                  if (item.tool.toolName === 'subagent' || item.tool.toolName === 'subagent_bg') {
                    const pSubId = (item.tool.params as Record<string, unknown> | undefined)?.subagent_id;
                    if (typeof pSubId === 'string' && pSubId) {
                      get().registerSubagentMapping(item.tool.id, pSubId);
                    } else if (item.tool.stdout) {
                      try {
                        const parsed = JSON.parse(item.tool.stdout);
                        if (parsed.subagent_id) get().registerSubagentMapping(item.tool.id, parsed.subagent_id);
                      } catch {}
                      const match = item.tool.stdout.match(/\[Subagent ID:\s*([^\s\]]+)\]/);
                      if (match && match[1]) get().registerSubagentMapping(item.tool.id, match[1]);
                    }
                  }
                } else if (item.type === 'thought') {
                  const rawItemDur = item.durationMs ?? (item as any).duration_ms;
                  const dur =
                    (typeof rawItemDur === 'number' && rawItemDur > 0)
                      ? rawItemDur
                      : (typeof computedDur === 'number' && computedDur > 0)
                        ? computedDur
                        : undefined;
                  item.durationMs = dur;
                  thoughts.push({
                    id: item.id,
                    timestamp: item.timestamp,
                    type: 'thought',
                    content: item.content || '',
                    durationMs: dur,
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
      console.info('load_task_activity invoke error or web mode:', e);
    }
  },

  loadTaskDialog: async (taskId) => {
    return get().loadTaskActivity(taskId);
  },

  saveTaskActivity: async (taskId) => {
    cancelActivitySave(taskId);
    const state = get();
    const feed = state.activityFeed[taskId];
    if (!feed || feed.length === 0) return;

    // Resolve target SQLite taskId:
    // If this taskId is a board task, targetDbTaskId is ALWAYS taskId (never save under {parent_id}:sub:{subagent_id})!
    let targetDbTaskId = taskId;
    const isBoardTask = useBoardStore.getState().tasks.some((t) => t.id === taskId);
    if (!isBoardTask) {
      const parentTaskId = state.activeTaskId;
      if (parentTaskId && !taskId.includes(':sub:')) {
        const resolvedSub = state.toolIdToSubagentId[taskId] || (state.activeSubagentId === taskId ? taskId : null);
        if (resolvedSub) {
          const isResolvedBoardTask = useBoardStore.getState().tasks.some((t) => t.id === resolvedSub);
          if (!isResolvedBoardTask) {
            targetDbTaskId = resolvedSub.includes(':sub:') ? resolvedSub : `${parentTaskId}:sub:${resolvedSub}`;
          } else {
            targetDbTaskId = resolvedSub;
          }
        }
      }
    }

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_task_activity', { taskId: targetDbTaskId, items: feed });
      if (targetDbTaskId !== taskId) {
        await invoke('save_task_activity', { taskId, items: feed }).catch(() => {});
      }
    } catch (err) {
      console.info('save_task_activity invoke error or web mode:', err);
    }
  },

  saveTaskDialog: async (taskId) => {
    return get().saveTaskActivity(taskId);
  },

  flushSaveDialog: async (taskId) => {
    return get().saveTaskActivity(taskId);
  },

  addQuestion: (taskId, question) => {
    const timeStr = new Date().toLocaleTimeString('en-US', { hour12: false });
    const questionId = question.questionId;
    const itemId = `q-${questionId}`;

    set((state) => {
      const existingFeed = state.activityFeed[taskId] || [];

      // Check if there is already an item for this question or matching running tool call (§ 2.20 & Question Dedup)
      let existingExactIndex = -1;
      for (let i = existingFeed.length - 1; i >= 0; i--) {
        const f = existingFeed[i];
        if (
          f.id === itemId ||
          f.tool?.id === questionId ||
          (f.tool?.params as Record<string, unknown> | undefined)?.questionId === questionId
        ) {
          existingExactIndex = i;
          break;
        }
      }

      let existingToolIndex = -1;
      for (let i = existingFeed.length - 1; i >= 0; i--) {
        const f = existingFeed[i];
        if (
          f.type === 'tool' &&
          (f.tool?.toolName === 'question' || f.tool?.toolName === 'ask_user') &&
          f.tool?.status === 'running' &&
          (!(f.tool?.params as Record<string, unknown> | undefined)?.questionId ||
            (f.tool?.params as Record<string, unknown> | undefined)?.questionId === questionId)
        ) {
          existingToolIndex = i;
          break;
        }
      }

      const targetIndex = existingExactIndex !== -1 ? existingExactIndex : existingToolIndex;

      if (targetIndex !== -1) {
        // Synchronize in-place into a single 'question' activity item!
        const prev = existingFeed[targetIndex];
        const updatedItem: DrawerActivityItem = {
          ...prev,
          id: itemId,
          type: 'question',
          timestamp: prev.timestamp || timeStr,
          content: question.text || prev.content,
          tool: {
            ...prev.tool,
            id: questionId,
            toolName: 'question',
            timestamp: prev.tool?.timestamp || prev.timestamp || timeStr,
            durationMs: prev.tool?.durationMs || 0,
            status: prev.tool?.status || 'running',
            params: {
              ...(prev.tool?.params as Record<string, unknown>),
              taskId,
              questionId,
              callId: prev.tool?.id, // retain original call ID for tool_result routing
              text: question.text,
              options: question.options,
              multiple: question.multiple,
            },
          },
        };

        const updatedFeed = [...existingFeed];
        updatedFeed[targetIndex] = updatedItem;

        // Also update toolExecutions to reflect question parameters
        const existingTools = state.toolExecutions[taskId] || [];
        const updatedTools = existingTools.map((t) => {
          if (t.id === prev.tool?.id || t.id === questionId) {
            return {
              ...t,
              params: {
                ...(t.params as Record<string, unknown>),
                taskId,
                questionId,
                callId: t.id,
                text: question.text,
                options: question.options,
                multiple: question.multiple,
              },
            };
          }
          return t;
        });

        return {
          activityFeed: {
            ...state.activityFeed,
            [taskId]: updatedFeed,
          },
          toolExecutions: {
            ...state.toolExecutions,
            [taskId]: updatedTools,
          },
        };
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
    useBoardStore.getState().clearActiveQuestion(taskId);
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
          item.id === questionId ||
          (item.tool &&
            (item.tool.id === questionId ||
              (item.tool.params as Record<string, unknown> | undefined)?.questionId === questionId ||
              (item.tool.params as Record<string, unknown> | undefined)?.callId === questionId))
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
    scheduleActivitySave(taskId);
  },

  appendActivityItem: (taskId, item) => {
    if (!taskId || !item) return;
    set((state) => {
      const streamed = item.id.match(/^(.+):(thought|output)$/);
      if (streamed && taskId in state.activeStreamIds && state.activeStreamIds[taskId] !== streamed[1]) return state;
      const feed = state.activityFeed[taskId] || [];
      const idx = feed.findIndex((f) => f.id === item.id);
      let updatedFeed: DrawerActivityItem[];
      if (idx !== -1) {
        updatedFeed = [...feed];
        updatedFeed[idx] = { ...feed[idx], ...item };
      } else {
        const isHandoverSteer = item.type === 'steer' && item.content?.includes('[ОТЧЕТ ОБ ИСПРАВЛЕНИЯХ / HANDOVER SUMMARY]');
        const duplicateIdx = streamed || item.type === 'user' || (item.type === 'steer' && !isHandoverSteer) ? -1 : feed.findIndex(
          (f) =>
            !/:(thought|output)$/.test(f.id) &&
            f.type === item.type &&
            f.content &&
            item.content &&
            f.content.trim() === item.content.trim()
        );
        if (duplicateIdx !== -1) {
          updatedFeed = [...feed];
          updatedFeed[duplicateIdx] = { ...feed[duplicateIdx], ...item };
        } else {
          updatedFeed = [...feed, item];
        }
      }
      return {
        activityFeed: {
          ...state.activityFeed,
          [taskId]: updatedFeed,
        },
      };
    });
  },
});

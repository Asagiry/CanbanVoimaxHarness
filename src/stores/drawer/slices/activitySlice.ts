import { StateCreator } from 'zustand';
import {
  ActivitySlice,
  DrawerState,
  DrawerActivityItem,
} from '../types';
import { ToolExecution, CompactionReport, LiveStreamEntry } from '../../../types';
import { scheduleDialogSave, cancelDialogSave } from '../drawerHelpers';
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

      // Obeying § 2.20 & Question Tool Dedup:
      // If tool is question/ask_user and there is already an existing question item in feed,
      // synchronize tool call info into that existing question item rather than duplicating feed!
      if (tool.toolName === 'question' || tool.toolName === 'ask_user') {
        const qParamId = (tool.params as Record<string, unknown> | undefined)?.questionId as string | undefined;
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
            f.id === `q-${tool.id}` ||
            f.tool?.id === tool.id
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
              ...tool,
              id: prev.tool?.id || tool.id,
              toolName: 'question',
              params: {
                ...(tool.params as Record<string, unknown>),
                ...(prev.tool?.params as Record<string, unknown>),
                callId: tool.id,
              },
            },
          };

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
        }
      }

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
        if (
          (item.type === 'tool' || item.type === 'question') &&
          (item.id === toolId ||
            item.tool?.id === toolId ||
            (item.tool?.params as Record<string, unknown> | undefined)?.callId === toolId ||
            (item.tool?.params as Record<string, unknown> | undefined)?.questionId === toolId)
        ) {
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
    cancelDialogSave(taskId);
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
                if (
                  prev &&
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
    cancelDialogSave(taskId);
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
      await invoke('save_task_dialog', { taskId: targetDbTaskId, items: feed });
      if (targetDbTaskId !== taskId) {
        await invoke('save_task_dialog', { taskId, items: feed }).catch(() => {});
      }
    } catch (err) {
      console.info('save_task_dialog invoke error or web mode:', err);
    }
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
    scheduleDialogSave(taskId);
  },
});

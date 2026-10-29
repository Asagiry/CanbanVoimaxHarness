import { StateCreator } from 'zustand';
import {
  SubagentsSlice,
  DrawerState,
  DrawerActivityItem,
} from '../types';
import { SubagentInfo } from '../../../types';

export const createSubagentsSlice: StateCreator<
  DrawerState,
  [],
  [],
  SubagentsSlice
> = (set, get) => ({
  activeSubagentId: null,
  activeSubagentTitle: null,
  activeSubagentRole: null,
  activeSubagentPrompt: null,
  toolIdToSubagentId: {},
  subagentIdToToolId: {},
  subagents: {},

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
      for (const [taskId, rawList] of Object.entries(state.subagents)) {
        const list = (Array.isArray(rawList) ? rawList : []) as SubagentInfo[];
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
      for (const [taskId, rawList] of Object.entries(state.subagents)) {
        const list = (Array.isArray(rawList) ? rawList : []) as SubagentInfo[];
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
});

import type { DrawerActivityItem } from './types';
import { useDrawerStore } from '../drawerStore';
import { listenToEvent } from '../../services/tauriEventGateway';

let subagentListenersInitialized = false;

export async function initSubagentListeners(store: typeof useDrawerStore = useDrawerStore): Promise<() => void> {
  if (subagentListenersInitialized) return () => {};
  subagentListenersInitialized = true;
  const unlisteners: (() => void)[] = [];
  try {
    unlisteners.push(await listenToEvent('subagent:spawned', (payload) => {
      const { parentTaskId, subagentId, prompt, title, createdAt, role } = payload || {};
      if (!subagentId) return;
      const state = store.getState();

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
        store.setState({ activityFeed: nextFeeds });
      }

      // Register subagent record into subagents panel
      if (parentTaskId) {
        state.registerSubagent(parentTaskId, {
          id: subagentId,
          parentTaskId,
          title: title || 'Субагент',
          role: role || 'general',
          prompt: prompt || '',
          status: 'running',
          createdAt: createdAt || Date.now(),
          stepsCount: 1,
        });
      }
    }));

    unlisteners.push(await listenToEvent('subagent:activity', (payload) => {
      const { subagentId, activity } = payload || {};
      if (!subagentId || !activity) return;
      const state = store.getState();
      const streamed = typeof activity.id === 'string' ? activity.id.match(/^(.+):(thought|output)$/) : null;
      if (streamed && subagentId in state.activeStreamIds && state.activeStreamIds[subagentId] !== streamed[1]) return;
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
      store.setState({ activityFeed: nextFeeds });
      // Streaming snapshots and retries are not additional agent steps.
      if (existingIdx < 0 && !streamed) state.recordSubagentStep(subagentId, activity);
    }));

    unlisteners.push(await listenToEvent('subagent:finished', (payload) => {
      const { subagentId, response } = payload || {};
      if (!subagentId) return;
      const state = store.getState();
      state.setActiveAgentTask(subagentId, false);
      // Clear parent alias flag (open via tool id) and fail stale running tools
      const toolIdAlias = Object.entries(state.toolIdToSubagentId).find(([, v]) => v === subagentId)?.[0];
      if (toolIdAlias) state.setActiveAgentTask(toolIdAlias, false);
      const tools = state.toolExecutions[subagentId] || [];
      for (const t of tools) {
        if (t.status === 'running') {
          state.updateToolExecution(subagentId, t.id, {
            status: 'failed',
            stderr: 'Прервано пользователем',
          });
        }
      }
      const existingFeed = state.activityFeed[subagentId] || [];
      if (response && !existingFeed.some((i) => i.type === 'output' && i.content === response)) {
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
        store.setState({ activityFeed: nextFeeds });
      }
      state.updateSubagentStatus(subagentId, 'finished');
    }));

    unlisteners.push(await listenToEvent('subagent:error', (payload) => {
      const { subagentId, error } = payload || {};
      if (!subagentId) return;
      const state = store.getState();
      state.setActiveAgentTask(subagentId, false);
      // Clear parent alias flag (open via tool id) and fail stale running tools
      const toolIdAlias = Object.entries(state.toolIdToSubagentId).find(([, v]) => v === subagentId)?.[0];
      if (toolIdAlias) state.setActiveAgentTask(toolIdAlias, false);
      const tools = state.toolExecutions[subagentId] || [];
      for (const t of tools) {
        if (t.status === 'running') {
          state.updateToolExecution(subagentId, t.id, {
            status: 'failed',
            stderr: error || 'Ошибка сабагента',
          });
        }
      }
      store.getState().updateSubagentStatus(subagentId, 'error', error);
    }));

    unlisteners.push(await listenToEvent('subagent:status', ({ subagentId, status, error }) => {
      if (!subagentId) return;
      const state = store.getState();
      if (status === 'running') {
        state.updateSubagentStatus(subagentId, 'running');
        state.setActiveAgentTask(subagentId, true);
        return;
      }
      // Paused (transient provider error, e.g. 429): still "in work", awaiting
      // user Continue/Stop — never auto-finished by the backend.
      if (status === 'paused') {
        state.updateSubagentStatus(subagentId, 'paused', error);
        state.setActiveAgentTask(subagentId, false);
        const toolIdAlias = Object.entries(state.toolIdToSubagentId).find(([, v]) => v === subagentId)?.[0];
        if (toolIdAlias) state.setActiveAgentTask(toolIdAlias, false);
        const tools = state.toolExecutions[subagentId] || [];
        for (const t of tools) {
          if (t.status === 'running') {
            state.updateToolExecution(subagentId, t.id, {
              status: 'failed',
              stderr: error || 'Пауза: временная ошибка провайдера',
            });
          }
        }
        if (error) state.appendOutput(subagentId, `Пауза (провайдер): ${error}`, 'error');
        return;
      }
      if (status === 'finished' || status === 'aborted' || status === 'stopped' || status === 'error') {
        const nextStatus = status === 'finished' ? 'finished' : (status === 'aborted' || status === 'stopped') ? 'stopped' : 'error';
        state.updateSubagentStatus(subagentId, nextStatus as any);
        state.setActiveAgentTask(subagentId, false);
        const tools = state.toolExecutions[subagentId] || [];
        for (const t of tools) {
          if (t.status === 'running') {
            state.updateToolExecution(subagentId, t.id, {
              status: 'failed',
              stderr: 'Прервано пользователем',
            });
          }
        }
      }
    }));

    unlisteners.push(await listenToEvent('subagent:aborted', ({ subagentId, parentTaskId }) => {
      const state = store.getState();
      if (subagentId) {
        state.updateSubagentStatus(subagentId, 'stopped', 'Прервано пользователем');
        state.setActiveAgentTask(subagentId, false);
        const tools = state.toolExecutions[subagentId] || [];
        for (const t of tools) {
          if (t.status === 'running') {
            state.updateToolExecution(subagentId, t.id, {
              status: 'failed',
              stderr: 'Прервано пользователем',
            });
          }
        }
      } else if (parentTaskId) {
        for (const subagent of state.subagents[parentTaskId] || []) {
          state.updateSubagentStatus(subagent.id, 'stopped', 'Прервано пользователем');
          state.setActiveAgentTask(subagent.id, false);
          const tools = state.toolExecutions[subagent.id] || [];
          for (const t of tools) {
            if (t.status === 'running') {
              state.updateToolExecution(subagent.id, t.id, {
                status: 'failed',
                stderr: 'Прервано пользователем',
              });
            }
          }
        }
      }
    }));
  } catch (err) {
    console.info('initSubagentListeners: web or test mode', err);
  }

  return () => {
    subagentListenersInitialized = false;
    for (const u of unlisteners) {
      u();
    }
  };
}

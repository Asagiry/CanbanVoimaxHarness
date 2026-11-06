import type { DrawerActivityItem } from './types';
import type { useDrawerStore } from '../drawerStore';
import { listenToEvent } from '../../services/tauriEventGateway';

let subagentListenersInitialized = false;

export async function initSubagentListeners(store: typeof useDrawerStore) {
  if (subagentListenersInitialized) return;
  subagentListenersInitialized = true;
  try {
    await listenToEvent('subagent:spawned', (payload) => {
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
    });

    await listenToEvent('subagent:activity', (payload) => {
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
    });

    await listenToEvent('subagent:finished', (payload) => {
      const { subagentId, response } = payload || {};
      if (!subagentId) return;
      const state = store.getState();
      state.setActiveAgentTask(subagentId, false);
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
    });

    await listenToEvent('subagent:error', (payload) => {
      const { subagentId, error } = payload || {};
      if (!subagentId) return;
      store.getState().setActiveAgentTask(subagentId, false);
      store.getState().updateSubagentStatus(subagentId, 'error', error);
    });

    await listenToEvent('subagent:status', ({ subagentId, status }) => {
      if (subagentId && (status === 'finished' || status === 'aborted' || status === 'stopped' || status === 'error')) {
        store.getState().setActiveAgentTask(subagentId, false);
      }
    });

    await listenToEvent('subagent:aborted', ({ subagentId, parentTaskId }) => {
      if (subagentId) store.getState().setActiveAgentTask(subagentId, false);
      else if (parentTaskId) {
        for (const subagent of store.getState().subagents[parentTaskId] || []) {
          store.getState().setActiveAgentTask(subagent.id, false);
        }
      }
    });
  } catch (err) {
    console.info('initSubagentListeners: web or test mode', err);
  }
}

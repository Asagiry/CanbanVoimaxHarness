import type { DrawerActivityItem } from './types';
import type { useDrawerStore } from '../drawerStore';

let subagentListenersInitialized = false;

export async function initSubagentListeners(store: typeof useDrawerStore) {
  if (subagentListenersInitialized) return;
  subagentListenersInitialized = true;
  try {
    const { listen } = await import('@tauri-apps/api/event');

    await listen<{ parentTaskId?: string; subagentId?: string; title?: string; role?: string; prompt?: string; tools?: string[] }>(
      'subagent:spawned',
      (event) => {
        const { parentTaskId, subagentId, prompt, title } = event.payload || {};
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
          const { role } = event.payload || {};
          state.registerSubagent(parentTaskId, {
            id: subagentId,
            parentTaskId,
            title: title || 'Субагент',
            role: role || 'general',
            prompt: prompt || '',
            status: 'running',
            createdAt: Date.now(),
            stepsCount: 1,
          });
        }
      }
    );

    await listen<{ parentTaskId?: string; subagentId?: string; activity?: DrawerActivityItem }>(
      'subagent:activity',
      (event) => {
        const { subagentId, activity } = event.payload || {};
        if (!subagentId || !activity) return;
        const state = store.getState();
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
        state.recordSubagentStep(subagentId, activity);
      }
    );

    await listen<{ parentTaskId?: string; subagentId?: string; response?: string }>(
      'subagent:finished',
      (event) => {
        const { subagentId, response } = event.payload || {};
        if (!subagentId || !response) return;
        const state = store.getState();
        const existingFeed = state.activityFeed[subagentId] || [];
        if (!existingFeed.some((i) => i.type === 'output' && i.content === response)) {
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
      }
    );

    await listen<{ parentTaskId?: string; subagentId?: string; error?: string }>(
      'subagent:error',
      (event) => {
        const { subagentId, error } = event.payload || {};
        if (!subagentId) return;
        store.getState().updateSubagentStatus(subagentId, 'error', error);
      }
    );
  } catch (err) {
    console.info('initSubagentListeners: web or test mode', err);
  }
}

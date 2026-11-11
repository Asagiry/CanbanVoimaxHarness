import type { DrawerActivityItem } from './types';
import type { SubagentStatus } from '../../types';
import { useDrawerStore } from '../drawerStore';
import { listenToEvent } from '../../services/tauriEventGateway';
import type { AgentEventMap } from '../../services/gateway/types';
import { currentLanguage, t } from '../../i18n';

let activeGeneration = 0;
let activeCleanup: (() => void) | null = null;

function failRunningSubagentTools(
  state: ReturnType<typeof useDrawerStore.getState>,
  subagentId: string,
  reason: string,
): void {
  const tools = state.toolExecutions[subagentId] || [];
  for (const tool of tools) {
    if (tool.status === 'running') {
      state.updateToolExecution(subagentId, tool.id, {
        status: 'failed',
        stderr: reason,
      });
    }
  }
}

function getFinishedToolFallbackReason(): string {
  return currentLanguage() === 'ru'
    ? 'Субагент завершил работу до окончания инструмента'
    : 'Subagent finished before tool completed';
}

function getAbortedToolReason(): string {
  return currentLanguage() === 'ru'
    ? `Прервано (${t('tools.status.aborted')})`
    : t('tools.status.aborted');
}

export async function initSubagentListeners(
  store: typeof useDrawerStore = useDrawerStore,
  signal?: AbortSignal,
): Promise<() => void> {
  if (signal?.aborted) {
    return () => {};
  }

  const generation = ++activeGeneration;
  if (activeCleanup) {
    activeCleanup();
    activeCleanup = null;
  }

  const unlisteners: (() => void)[] = [];
  let cleaned = false;

  const cleanup = () => {
    if (cleaned) return;
    cleaned = true;
    signal?.removeEventListener('abort', cleanup);
    if (activeCleanup === cleanup) {
      activeCleanup = null;
    }
    while (unlisteners.length > 0) {
      const unlisten = unlisteners.pop();
      unlisten?.();
    }
  };

  signal?.addEventListener('abort', cleanup, { once: true });

  const isDisposed = () => cleaned || Boolean(signal?.aborted) || generation !== activeGeneration;

  const subscribe = async <K extends keyof AgentEventMap>(
    event: K,
    handler: (payload: AgentEventMap[K]) => void,
  ): Promise<boolean> => {
    if (isDisposed()) return false;
    const unlisten = await listenToEvent(event, (payload) => {
      if (!isDisposed()) {
        handler(payload);
      }
    });
    if (isDisposed()) {
      unlisten();
      return false;
    }
    unlisteners.push(unlisten);
    return true;
  };

  try {
    const ok1 = await subscribe('subagent:spawned', (payload) => {
      const { parentTaskId, subagentId, prompt, title, createdAt, role } = payload || {};
      if (!subagentId) return;
      const state = store.getState();

      if (parentTaskId) {
        const tools = state.toolExecutions[parentTaskId] || [];
        const matchingTool = tools.find(
          (item) =>
            (item.toolName === 'subagent' || item.toolName === 'subagent_bg') &&
            !state.toolIdToSubagentId[item.id] &&
            ((item.params as Record<string, unknown> | undefined)?.prompt === prompt ||
              (item.params as Record<string, unknown> | undefined)?.title === title ||
              item.status === 'running'),
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

      if (parentTaskId) {
        state.registerSubagent(parentTaskId, {
          id: subagentId,
          parentTaskId,
          title: title || t('subagents.subagent'),
          role: role || 'general',
          prompt: prompt || '',
          status: 'running',
          createdAt: createdAt || Date.now(),
          stepsCount: 1,
        });
      }
    });
    if (!ok1) { cleanup(); return cleanup; }

    const ok2 = await subscribe('subagent:activity', (payload) => {
      const { subagentId, activity } = payload || {};
      if (!subagentId || !activity) return;
      const state = store.getState();
      const streamed = typeof activity.id === 'string' ? activity.id.match(/^(.+):(thought|output)$/) : null;
      if (streamed && subagentId in state.activeStreamIds && state.activeStreamIds[subagentId] !== streamed[1]) return;
      const existingFeed = state.activityFeed[subagentId] || [];
      let existingIdx = existingFeed.findIndex((i) => i.id === activity.id);
      if (
        existingIdx < 0 &&
        (activity.type === 'user' || activity.type === 'steer') &&
        activity.content
      ) {
        const trimmedContent = String(activity.content).trim();
        const searchStart = Math.max(0, existingFeed.length - 3);
        for (let idx = existingFeed.length - 1; idx >= searchStart; idx--) {
          const item = existingFeed[idx];
          if (
            (item.type === 'user' || item.type === 'steer') &&
            (item.content || '').trim() === trimmedContent
          ) {
            existingIdx = idx;
            break;
          }
        }
      }
      const nextFeed = existingIdx >= 0
        ? existingFeed.map((item, i) => (i === existingIdx ? { ...item, ...activity } : item))
        : [...existingFeed, activity];
      const nextFeeds: Record<string, DrawerActivityItem[]> = {
        ...state.activityFeed,
        [subagentId]: nextFeed,
      };
      const mappedToolId = state.subagentIdToToolId[subagentId];
      if (mappedToolId) {
        nextFeeds[mappedToolId] = nextFeed;
      }
      store.setState({ activityFeed: nextFeeds });
      if (existingIdx < 0 && !streamed) state.recordSubagentStep(subagentId, activity);
    });
    if (!ok2) { cleanup(); return cleanup; }

    const ok3 = await subscribe('subagent:finished', (payload) => {
      const { subagentId, response } = payload || {};
      if (!subagentId) return;
      const state = store.getState();
      state.setActiveAgentTask(subagentId, false);
      const toolIdAlias = Object.entries(state.toolIdToSubagentId).find(([, v]) => v === subagentId)?.[0];
      if (toolIdAlias) state.setActiveAgentTask(toolIdAlias, false);
      failRunningSubagentTools(state, subagentId, getFinishedToolFallbackReason());
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
    if (!ok3) { cleanup(); return cleanup; }

    const ok4 = await subscribe('subagent:error', (payload) => {
      const { subagentId, error } = payload || {};
      if (!subagentId) return;
      const state = store.getState();
      state.setActiveAgentTask(subagentId, false);
      const toolIdAlias = Object.entries(state.toolIdToSubagentId).find(([, v]) => v === subagentId)?.[0];
      if (toolIdAlias) state.setActiveAgentTask(toolIdAlias, false);
      failRunningSubagentTools(state, subagentId, error || t('tools.status.error'));
      store.getState().updateSubagentStatus(subagentId, 'error', error);
    });
    if (!ok4) { cleanup(); return cleanup; }

    const ok5 = await subscribe('subagent:status', ({ subagentId, status, error }) => {
      if (!subagentId) return;
      const state = store.getState();
      if (status === 'running') {
        state.updateSubagentStatus(subagentId, 'running');
        state.setActiveAgentTask(subagentId, true);
        return;
      }
      if (status === 'paused') {
        state.updateSubagentStatus(subagentId, 'paused', error);
        state.setActiveAgentTask(subagentId, false);
        const toolIdAlias = Object.entries(state.toolIdToSubagentId).find(([, v]) => v === subagentId)?.[0];
        if (toolIdAlias) state.setActiveAgentTask(toolIdAlias, false);
        failRunningSubagentTools(state, subagentId, error || t('drawer.waitingForConnection'));
        if (error) state.appendOutput(subagentId, `${t('drawer.executionError')} ${error}`, 'error');
        return;
      }
      if (status === 'finished' || status === 'aborted' || status === 'stopped' || status === 'error') {
        const nextStatus: SubagentStatus =
          status === 'finished' ? 'finished' : status === 'aborted' || status === 'stopped' ? 'stopped' : 'error';
        state.updateSubagentStatus(subagentId, nextStatus);
        state.setActiveAgentTask(subagentId, false);
        const reason =
          nextStatus === 'finished'
            ? getFinishedToolFallbackReason()
            : nextStatus === 'stopped'
              ? getAbortedToolReason()
              : t('tools.status.error');
        failRunningSubagentTools(state, subagentId, reason);
      }
    });
    if (!ok5) { cleanup(); return cleanup; }

    const ok6 = await subscribe('subagent:aborted', ({ subagentId, parentTaskId }) => {
      const state = store.getState();
      const abortReason = getAbortedToolReason();
      if (subagentId) {
        state.updateSubagentStatus(subagentId, 'stopped', abortReason);
        state.setActiveAgentTask(subagentId, false);
        failRunningSubagentTools(state, subagentId, abortReason);
      } else if (parentTaskId) {
        for (const subagent of state.subagents[parentTaskId] || []) {
          state.updateSubagentStatus(subagent.id, 'stopped', abortReason);
          state.setActiveAgentTask(subagent.id, false);
          failRunningSubagentTools(state, subagent.id, abortReason);
        }
      }
    });
    if (!ok6) { cleanup(); return cleanup; }

    activeCleanup = cleanup;
  } catch (err) {
    console.info('initSubagentListeners: web or test mode', err);
    cleanup();
  }

  return cleanup;
}

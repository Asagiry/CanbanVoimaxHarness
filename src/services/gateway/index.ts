import { useDrawerStore } from '../../stores/drawerStore';
import { initSubagentListeners } from '../../stores/drawer/subagentListeners';
import type { AgentEventMap } from './types';
import { TextBufferManager } from './textBuffer';
import { registerAgentEventListeners, type AddListenerFn } from './agentEventRouter';
import { registerTaskEventListeners } from './taskEventRouter';

export * from './types';
export * from './textBuffer';
export * from './helpers';
export * from './agentEventRouter';
export * from './taskEventRouter';

/**
 * Initializes all typed Tauri IPC event listeners and manages streaming micro-batching.
 * Returns a synchronous cleanup function safe under React.StrictMode double-mount/unmount.
 */
export function setupTauriEventGateway(): () => void {
  const controller = new AbortController();
  const disposedRef = { current: false };
  const unlisteners: (() => void)[] = [];
  const bufferManager = new TextBufferManager();

  const registerListeners = async () => {
    try {
      const { listen } = await import('@tauri-apps/api/event');
      if (disposedRef.current || controller.signal.aborted) return;

      const addListener: AddListenerFn = async <K extends keyof AgentEventMap>(
        event: K,
        handler: (payload: AgentEventMap[K]) => void | Promise<void>,
      ) => {
        if (disposedRef.current || controller.signal.aborted) return;
        const unlisten = await listen<AgentEventMap[K]>(event, (e) => {
          if (!disposedRef.current && !controller.signal.aborted) {
            void handler(e.payload);
          }
        });
        if (disposedRef.current || controller.signal.aborted) {
          unlisten();
        } else {
          unlisteners.push(unlisten);
        }
      };

      await registerAgentEventListeners(addListener, bufferManager);
      if (disposedRef.current || controller.signal.aborted) return;

      await registerTaskEventListeners(addListener, bufferManager, disposedRef);
      if (disposedRef.current || controller.signal.aborted) return;

      const cleanupSubagents = await initSubagentListeners(useDrawerStore, controller.signal);
      if (disposedRef.current || controller.signal.aborted) {
        cleanupSubagents();
      } else {
        unlisteners.push(cleanupSubagents);
      }
    } catch (e) {
      console.info('Tauri event listeners not active in web mode:', e);
    }
  };

  void registerListeners();

  return () => {
    disposedRef.current = true;
    controller.abort();
    bufferManager.dispose();
    while (unlisteners.length > 0) {
      const unlisten = unlisteners.pop();
      unlisten?.();
    }
  };
}

/**
 * Authoritative typed event subscription helper via tauriEventGateway.
 * Subscribes to backend Tauri IPC events using the nominal types defined in AgentEventMap.
 * Guarantees zero raw `listen<any>` in UI components.
 */
export async function listenToEvent<K extends keyof AgentEventMap>(
  event: K,
  handler: (payload: AgentEventMap[K]) => void | Promise<void>,
): Promise<() => void> {
  const { listen } = await import('@tauri-apps/api/event');
  return listen<AgentEventMap[K]>(event, (e) => {
    void handler(e.payload);
  });
}

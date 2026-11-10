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
 * Returns a synchronous cleanup function to unsubscribe all registered listeners and flush pending buffers.
 */
export function setupTauriEventGateway(): () => void {
  const disposedRef = { current: false };
  const unlisteners: (() => void)[] = [];
  const bufferManager = new TextBufferManager();

  const registerListeners = async () => {
    try {
      const { listen } = await import('@tauri-apps/api/event');
      if (disposedRef.current) return;

      const addListener: AddListenerFn = async (event, handler) => {
        const unlisten = await listen(event, (e) => {
          if (!disposedRef.current) {
            void handler(e.payload as any);
          }
        });
        if (disposedRef.current) {
          unlisten();
        } else {
          unlisteners.push(unlisten);
        }
      };

      await registerAgentEventListeners(addListener, bufferManager);
      await registerTaskEventListeners(addListener, bufferManager, disposedRef);

      // 23. Unified subagent listeners
      const cleanupSubagents = await initSubagentListeners(useDrawerStore);
      unlisteners.push(cleanupSubagents);
    } catch (e) {
      console.info('Tauri event listeners not active in web mode:', e);
    }
  };

  void registerListeners();

  return () => {
    disposedRef.current = true;
    bufferManager.dispose();
    unlisteners.forEach((u) => u());
  };
}

/**
 * Authoritative typed event subscription helper via tauriEventGateway.
 * Subscribes to backend Tauri IPC events using the nominal types defined in AgentEventMap.
 * Guarantees zero raw `listen<any>` in UI components.
 */
export async function listenToEvent<K extends keyof AgentEventMap>(
  event: K,
  handler: (payload: AgentEventMap[K]) => void | Promise<void>
): Promise<() => void> {
  const { listen } = await import('@tauri-apps/api/event');
  return listen<AgentEventMap[K]>(event, (e) => {
    void handler(e.payload);
  });
}

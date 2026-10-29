import { useBoardStore } from '../boardStore';
import { CortexState } from './types';

let cortexStoreRef: { getState: () => CortexState } | null = null;
let cortexListenersInitialized = false;

export function registerCortexStore(store: { getState: () => CortexState }): void {
  cortexStoreRef = store;
}

export async function initCortexEventListeners(
  customStore?: { getState: () => CortexState }
): Promise<() => void> {
  const store = customStore ?? cortexStoreRef;
  if (cortexListenersInitialized) return () => {};
  try {
    const { listen } = await import('@tauri-apps/api/event');
    const uPersonas = await listen<unknown>('agent:personas_updated', async () => {
      if (!store) return;
      await store.getState().loadAgentPersonas();
      store.getState().syncStagesFromColumns(useBoardStore.getState().columns);
    });
    const uBoards = await listen<unknown>('boards:updated', async () => {
      if (!store) return;
      store.getState().syncStagesFromColumns(useBoardStore.getState().columns);
    });
    const uPipeline = await listen<unknown>('pipeline:updated', async () => {
      if (!store) return;
      store.getState().syncStagesFromColumns(useBoardStore.getState().columns);
    });
    const uRules = await listen<unknown>('cortex:rules_updated', async () => {
      if (!store) return;
      await store.getState().loadGlobalRules();
    });
    cortexListenersInitialized = true;
    return () => {
      uPersonas();
      uBoards();
      uPipeline();
      uRules();
      cortexListenersInitialized = false;
    };
  } catch (e) {
    console.info('initCortexEventListeners not active in web mode:', e);
    return () => {};
  }
}

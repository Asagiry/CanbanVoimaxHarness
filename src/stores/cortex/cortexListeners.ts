import { CortexState } from './types';

export function registerCortexStore(_store: { getState: () => CortexState }): void {
  // Retained for backward compatibility
}

/**
 * @deprecated Consolidated into setupTauriEventGateway in src/services/tauriEventGateway.ts.
 * Retained as a no-op for backward compatibility.
 */
export async function initCortexEventListeners(
  _customStore?: { getState: () => CortexState }
): Promise<() => void> {
  return () => {};
}

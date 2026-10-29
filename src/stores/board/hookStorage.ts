import { TaskHookStates } from '../../types';

export const HOOK_STATES_KEY = 'cvh_task_hook_states';

export function loadCachedHookStates(): Record<string, TaskHookStates> {
  if (typeof window === 'undefined' || !window.localStorage) return {};
  try {
    const raw = window.localStorage.getItem(HOOK_STATES_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === 'object' ? parsed : {};
  } catch (err) {
    console.warn('Failed to load hook states from localStorage:', err);
    return {};
  }
}

export function saveCachedHookStates(states: Record<string, TaskHookStates>): void {
  if (typeof window === 'undefined' || !window.localStorage) return;
  try {
    window.localStorage.setItem(HOOK_STATES_KEY, JSON.stringify(states));
  } catch (err) {
    console.warn('Failed to save hook states to localStorage:', err);
  }
}

export const persistCachedHookStates = saveCachedHookStates;

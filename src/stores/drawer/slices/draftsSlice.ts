import { StateCreator } from 'zustand';
import { DraftsSlice, DrawerState, TaskDraft } from '../types';

export const DRAFTS_STORAGE_KEY = 'cvh_task_steer_drafts';

export function loadInitialDrafts(): Record<string, TaskDraft> {
  if (typeof window === 'undefined') return {};
  try {
    const raw = localStorage.getItem(DRAFTS_STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    if (parsed && typeof parsed === 'object') {
      return parsed as Record<string, TaskDraft>;
    }
  } catch (e) {
    console.warn('[draftsSlice] Failed to load drafts from localStorage:', e);
  }
  return {};
}

export function persistDrafts(drafts: Record<string, TaskDraft>): void {
  if (typeof window === 'undefined') return;
  try {
    if (Object.keys(drafts).length === 0) {
      localStorage.removeItem(DRAFTS_STORAGE_KEY);
      return;
    }
    localStorage.setItem(DRAFTS_STORAGE_KEY, JSON.stringify(drafts));
  } catch (err) {
    console.warn(
      '[draftsSlice] Quota exceeded or failed to save drafts to localStorage, falling back to text-only:',
      err
    );
    try {
      const textOnlyDrafts: Record<string, TaskDraft> = {};
      for (const [k, v] of Object.entries(drafts)) {
        if (v.text && v.text.trim()) {
          textOnlyDrafts[k] = { text: v.text, images: [] };
        }
      }
      if (Object.keys(textOnlyDrafts).length === 0) {
        localStorage.removeItem(DRAFTS_STORAGE_KEY);
      } else {
        localStorage.setItem(DRAFTS_STORAGE_KEY, JSON.stringify(textOnlyDrafts));
      }
    } catch (e) {
      console.error('[draftsSlice] Fatal: Failed to persist even text-only drafts:', e);
    }
  }
}

export const createDraftsSlice: StateCreator<
  DrawerState,
  [],
  [],
  DraftsSlice
> = (set, get) => ({
  taskDrafts: loadInitialDrafts(),

  setTaskDraft: (draftKey: string, draft: TaskDraft) => {
    if (!draftKey) return;
    const hasText = Boolean(draft.text && draft.text.trim().length > 0);
    const hasImages = Boolean(draft.images && draft.images.length > 0);

    if (!hasText && !hasImages) {
      get().clearTaskDraft(draftKey);
      return;
    }

    set((state) => {
      const nextDrafts = {
        ...state.taskDrafts,
        [draftKey]: {
          text: draft.text,
          images: draft.images || [],
        },
      };
      persistDrafts(nextDrafts);
      return { taskDrafts: nextDrafts };
    });
  },

  clearTaskDraft: (draftKey: string) => {
    if (!draftKey) return;
    set((state) => {
      const isParentKey = !draftKey.includes(':sub:');
      const subPrefix = `${draftKey}:sub:`;
      const nextDrafts: Record<string, TaskDraft> = {};
      let changed = false;

      for (const [k, v] of Object.entries(state.taskDrafts)) {
        if (k === draftKey || (isParentKey && k.startsWith(subPrefix))) {
          changed = true;
          continue;
        }
        nextDrafts[k] = v;
      }

      if (!changed) return state;

      persistDrafts(nextDrafts);
      return { taskDrafts: nextDrafts };
    });
  },

  getTaskDraft: (draftKey: string) => {
    if (!draftKey) return undefined;
    return get().taskDrafts[draftKey];
  },
});

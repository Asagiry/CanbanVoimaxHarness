import { StateCreator } from 'zustand';
import { BoardState, ActiveQuestionInfo } from './types';

export interface TaskQuestionsSlice {
  activeQuestions: Record<string, ActiveQuestionInfo>;
  setActiveQuestion: (payload: ActiveQuestionInfo) => void;
  clearActiveQuestion: (taskId: string) => void;
}

export const createTaskQuestionsSlice: StateCreator<
  BoardState,
  [],
  [],
  TaskQuestionsSlice
> = (set) => ({
  activeQuestions: {},

  setActiveQuestion: (payload) => {
    set((state) => {
      // Dedupe to single active per root family (CVH-076.* / CVH-076)
      const root = payload.taskId.split('.')[0];
      const next: Record<string, typeof payload> = {};
      for (const [k, v] of Object.entries(state.activeQuestions)) {
        if (k.split('.')[0] !== root) next[k] = v;
      }
      next[payload.taskId] = payload;
      return { activeQuestions: next };
    });
  },

  clearActiveQuestion: (taskId) => {
    set((state) => {
      const prefix = `${taskId}.`;
      const keysToDelete = Object.keys(state.activeQuestions).filter(
        (k) => k === taskId || k.startsWith(prefix)
      );
      // Also purge ancestors' keys when clearing a descendant (recordQuestionAnswer path)
      const root = taskId.split('.')[0];
      if (root !== taskId) keysToDelete.push(root, ...Object.keys(state.activeQuestions).filter((k) => k.startsWith(`${root}.`)));
      if (keysToDelete.length === 0) return state;
      const next = { ...state.activeQuestions };
      for (const k of keysToDelete) delete next[k];
      return { activeQuestions: next };
    });
  },
});

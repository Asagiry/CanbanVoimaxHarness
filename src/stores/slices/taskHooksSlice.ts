import { StateCreator } from 'zustand';
import { BoardState } from './types';
import { TaskHookStates } from '../../types';
import { loadCachedHookStates, saveCachedHookStates } from '../board/hookStorage';

export interface TaskHooksSlice {
  hookStates: Record<string, TaskHookStates>;
  setHookStart: (payload: {
    taskId: string;
    stageId: string;
    hookType: 'on_enter' | 'on_leave';
    hookName: string;
    description?: string;
  }) => void;
  setHookFinish: (payload: {
    taskId: string;
    stageId: string;
    hookType: 'on_enter' | 'on_leave';
    success: boolean;
    exitCode?: number | null;
    stdout?: string;
    stderr?: string;
    error?: string;
    hookName?: string;
  }) => void;
  clearHookState: (taskId: string, hookType?: 'on_enter' | 'on_leave') => void;
}

export const createTaskHooksSlice: StateCreator<
  BoardState,
  [],
  [],
  TaskHooksSlice
> = (set) => ({
  hookStates: loadCachedHookStates(),

  setHookStart: ({ taskId, stageId, hookType, hookName, description }) => {
    set((state) => {
      const normalizedHookType = (hookType as string) === 'on_finish' || (hookType as string) === 'on_exit' ? 'on_leave' : hookType;
      const prev = state.hookStates[taskId] || {};
      const nextTaskHookState: TaskHookStates = {
        ...prev,
        [normalizedHookType]: {
          hookName: hookName || (normalizedHookType as string),
          stageId,
          hookType: normalizedHookType,
          status: 'running',
          startedAt: Date.now(),
          stdout: '',
          stderr: '',
          description,
        },
      };
      const nextHookStates = {
        ...state.hookStates,
        [taskId]: nextTaskHookState,
      };
      saveCachedHookStates(nextHookStates);
      const nextTasks = state.tasks.map((t) => {
        if (t.id === taskId) {
          return { ...t, hookStates: nextTaskHookState };
        }
        return t;
      });
      return {
        hookStates: nextHookStates,
        tasks: nextTasks,
      };
    });
  },

  setHookFinish: ({ taskId, stageId, hookType, success, exitCode, stdout, stderr, error, hookName }) => {
    set((state) => {
      const normalizedHookType = (hookType as string) === 'on_finish' || (hookType as string) === 'on_exit' ? 'on_leave' : hookType;
      const prev = state.hookStates[taskId] || {};
      const existing = prev[normalizedHookType];
      const startedAt = existing?.startedAt ?? Date.now();
      const durationMs = Date.now() - startedAt;
      const nextTaskHookState: TaskHookStates = {
        ...prev,
        [normalizedHookType]: {
          hookName: hookName || existing?.hookName || (normalizedHookType as string),
          stageId,
          hookType: normalizedHookType,
          status: success ? 'success' : 'failed',
          startedAt,
          durationMs,
          exitCode: exitCode ?? null,
          stdout: stdout || '',
          stderr: stderr || '',
          error,
          description: existing?.description,
        },
      };
      const nextHookStates = {
        ...state.hookStates,
        [taskId]: nextTaskHookState,
      };
      saveCachedHookStates(nextHookStates);
      const nextTasks = state.tasks.map((t) => {
        if (t.id === taskId) {
          return { ...t, hookStates: nextTaskHookState };
        }
        return t;
      });
      return {
        hookStates: nextHookStates,
        tasks: nextTasks,
      };
    });
  },

  clearHookState: (taskId, hookType) => {
    set((state) => {
      if (!state.hookStates[taskId]) return state;
      if (!hookType) {
        const next = { ...state.hookStates };
        delete next[taskId];
        saveCachedHookStates(next);
        const nextTasks = state.tasks.map((t) => {
          if (t.id === taskId) {
            const copy = { ...t };
            delete copy.hookStates;
            return copy;
          }
          return t;
        });
        return { hookStates: next, tasks: nextTasks };
      }
      const prev = state.hookStates[taskId];
      const nextTaskHooks = { ...prev };
      delete nextTaskHooks[hookType];
      const nextHookStates = {
        ...state.hookStates,
        [taskId]: nextTaskHooks,
      };
      saveCachedHookStates(nextHookStates);
      const nextTasks = state.tasks.map((t) => {
        if (t.id === taskId) {
          return { ...t, hookStates: nextTaskHooks };
        }
        return t;
      });
      return {
        hookStates: nextHookStates,
        tasks: nextTasks,
      };
    });
  },
});

import { StateCreator } from 'zustand';
import {
  DrawerMetaSlice,
  DrawerState,
} from '../types';

export const createDrawerMetaSlice: StateCreator<
  DrawerState,
  [],
  [],
  DrawerMetaSlice
> = (set, get) => ({
  isOpen: false,
  activeTaskId: null,
  activeTab: 'thoughts',
  taskHistory: [],

  openDrawer: (taskId, initialTab = 'thoughts') => {
    const state = get();
    const currentTaskId = state.activeTaskId;
    const currentHistory = state.taskHistory || [];
    const newHistory =
      currentTaskId && currentTaskId !== taskId
        ? [...currentHistory, currentTaskId]
        : currentHistory;

    set({
      isOpen: true,
      activeTaskId: taskId,
      taskHistory: newHistory,
      activeTab: initialTab,
      activeSubagentId: null,
      activeSubagentTitle: null,
      activeSubagentRole: null,
      activeSubagentPrompt: null,
    });
    get().loadTaskDialog(taskId);
    get().loadSubagents(taskId);
  },

  goBackDrawer: () => {
    const state = get();
    if (state.activeSubagentId) {
      state.closeSubagent();
      return;
    }

    if (state.taskHistory && state.taskHistory.length > 0) {
      const prevTaskId = state.taskHistory[state.taskHistory.length - 1];
      const nextHistory = state.taskHistory.slice(0, -1);
      set({
        activeTaskId: prevTaskId,
        taskHistory: nextHistory,
        activeSubagentId: null,
        activeSubagentTitle: null,
        activeSubagentRole: null,
        activeSubagentPrompt: null,
      });
      get().loadTaskDialog(prevTaskId);
      get().loadSubagents(prevTaskId);
      return;
    }

    if (state.activeTaskId && state.activeTaskId.includes('.')) {
      const parentId = state.activeTaskId.split('.')[0];
      if (parentId && parentId !== state.activeTaskId) {
        set({
          activeTaskId: parentId,
          taskHistory: [],
          activeSubagentId: null,
          activeSubagentTitle: null,
          activeSubagentRole: null,
          activeSubagentPrompt: null,
        });
        get().loadTaskDialog(parentId);
        get().loadSubagents(parentId);
        return;
      }
    }

    state.closeDrawer();
  },

  closeDrawer: () => {
    set({
      isOpen: false,
      activeTaskId: null,
      taskHistory: [],
      activeSubagentId: null,
      activeSubagentTitle: null,
      activeSubagentRole: null,
      activeSubagentPrompt: null,
    });
  },

  setActiveTab: (tab) => set({ activeTab: tab }),
});

import { StateCreator } from 'zustand';
import {
  DrawerMetaSlice,
  DrawerState,
} from '../types';
import { flushStreamChunks } from './streamSlice';

export const createDrawerMetaSlice: StateCreator<
  DrawerState,
  [],
  [],
  DrawerMetaSlice
> = (set, get) => ({
  isOpen: false,
  activeTaskId: null,
  activeTab: 'thoughts',

  openDrawer: (taskId, initialTab = 'thoughts') => {
    flushStreamChunks();
    set({
      isOpen: true,
      activeTaskId: taskId,
      activeTab: initialTab,
      activeSubagentId: null,
      activeSubagentTitle: null,
      activeSubagentRole: null,
      activeSubagentPrompt: null,
    });
    (get().loadTaskActivity || get().loadTaskDialog)(taskId);
    get().loadSubagents(taskId);
  },

  goBackDrawer: () => {
    const state = get();
    if (state.activeSubagentId) {
      state.closeSubagent();
      return;
    }
    state.closeDrawer();
  },

  closeDrawer: () => {
    flushStreamChunks();
    set({
      isOpen: false,
      activeTaskId: null,
      activeSubagentId: null,
      activeSubagentTitle: null,
      activeSubagentRole: null,
      activeSubagentPrompt: null,
    });
  },

  setActiveTab: (tab) => set({ activeTab: tab }),
});

import { create } from 'zustand';
import {
  DrawerState,
  DrawerActivityItem,
  DrawerTab,
  TaskTelemetry,
  ActivityType,
  createDrawerMetaSlice,
  createStreamSlice,
  createActivitySlice,
  createSubagentsSlice,
  registerDialogFlushCallback,
  initSubagentListeners,
} from './drawer';

export type { ActivityType };
export type { DrawerActivityItem, DrawerTab, TaskTelemetry, DrawerState };

export const useDrawerStore = create<DrawerState>()((...a) => {
  const [, get] = a;
  registerDialogFlushCallback((taskId) => {
    void get().flushSaveDialog(taskId);
  });

  return {
    ...createDrawerMetaSlice(...a),
    ...createStreamSlice(...a),
    ...createActivitySlice(...a),
    ...createSubagentsSlice(...a),
  };
});

if (typeof window !== 'undefined') {
  void initSubagentListeners(useDrawerStore);
}

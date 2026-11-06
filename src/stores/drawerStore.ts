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
  createDraftsSlice,
  registerActivityFlushCallback,
} from './drawer';

export type { ActivityType };
export type { DrawerActivityItem, DrawerTab, TaskTelemetry, DrawerState };

export const useDrawerStore = create<DrawerState>()((...a) => {
  const [, get] = a;
  registerActivityFlushCallback((taskId) => {
    void get().saveTaskActivity(taskId);
  });

  return {
    ...createDrawerMetaSlice(...a),
    ...createStreamSlice(...a),
    ...createActivitySlice(...a),
    ...createSubagentsSlice(...a),
    ...createDraftsSlice(...a),
  };
});

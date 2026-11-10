import { StateCreator } from 'zustand';
import { BoardState, TasksCrudCoreSlice } from './types';
import {
  createTasksCoreSlice,
  createTasksCreateSlice,
  createTasksMoveSlice,
  createTasksMutateSlice,
} from './tasks';

export const createTasksCrudSlice: StateCreator<
  BoardState,
  [],
  [],
  TasksCrudCoreSlice
> = (...a) => ({
  ...createTasksCoreSlice(...a),
  ...createTasksCreateSlice(...a),
  ...createTasksMoveSlice(...a),
  ...createTasksMutateSlice(...a),
});

export * from './tasks';

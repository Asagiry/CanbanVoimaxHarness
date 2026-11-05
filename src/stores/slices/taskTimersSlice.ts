import { StateCreator } from 'zustand';
import { BoardState, TaskTimersSlice } from './types';

export const createTaskTimersSlice: StateCreator<
  BoardState,
  [],
  [],
  TaskTimersSlice
> = (set) => ({
  toggleTimer: (taskId) => {
    set((state) => ({
      tasks: state.tasks.map((t) =>
        t.id === taskId ? { ...t, isTimerRunning: !t.isTimerRunning } : t
      ),
    }));
  },

  tickTimers: () => {
    set((state) => {
      const hasRunningTask = state.tasks.some(
        (task) =>
          (task.status === 'running' || task.isTimerRunning === true) &&
          task.status !== 'done' &&
          task.status !== 'paused' &&
          task.status !== 'blocked' &&
          task.status !== 'idle' &&
          task.status !== 'waiting_review' &&
          task.status !== 'waiting_subtask' &&
          task.status !== 'waiting_subagent'
      );

      if (!hasRunningTask) {
        return state;
      }

      return {
        tasks: state.tasks.map((task) => {
          const isActivelyRunning =
            (task.status === 'running' || task.isTimerRunning === true) &&
            task.status !== 'done' &&
            task.status !== 'paused' &&
            task.status !== 'blocked' &&
            task.status !== 'idle' &&
            task.status !== 'waiting_review' &&
            task.status !== 'waiting_subtask' &&
            task.status !== 'waiting_subagent';

          if (!isActivelyRunning) {
            return task;
          }

          const currentStage = task.stageId;
          const prevStageDurations =
            task.stageDurations && typeof task.stageDurations === 'object'
              ? task.stageDurations
              : {};
          const currentStageDuration = (prevStageDurations[currentStage] || 0) + 1;
          const nextStageDurations = {
            ...prevStageDurations,
            [currentStage]: currentStageDuration,
          };

          const activeStageDuration =
            typeof task.activeStageDuration === 'number' && Number.isFinite(task.activeStageDuration)
              ? task.activeStageDuration + 1
              : 1;

          const totalElapsed =
            typeof task.totalElapsed === 'number' && Number.isFinite(task.totalElapsed)
              ? task.totalElapsed + 1
              : 1;

          return {
            ...task,
            activeStageDuration,
            stageDurations: nextStageDurations,
            totalElapsed,
          };
        }),
      };
    });
  },

  startTaskTimer: (taskId) => {
    set((state) => ({
      tasks: state.tasks.map((t) =>
        t.id === taskId ? { ...t, isTimerRunning: true } : t
      ),
    }));
  },

  stopTaskTimer: (taskId) => {
    set((state) => ({
      tasks: state.tasks.map((t) =>
        t.id === taskId ? { ...t, isTimerRunning: false } : t
      ),
    }));
  },

  resetTaskTimer: (taskId) => {
    set((state) => ({
      tasks: state.tasks.map((t) =>
        t.id === taskId ? { ...t, activeStageDuration: 0, isTimerRunning: false } : t
      ),
    }));
  },
});

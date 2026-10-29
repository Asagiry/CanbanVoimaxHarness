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
    const now = Date.now();
    set((state) => ({
      tasks: state.tasks.map((task) => {
        if (task.status === 'done' || task.status === 'paused') {
          return task;
        }
        const stageStart = task.stageEnteredAt || task.createdAt;
        const activeStageDuration = stageStart
          ? Math.max(0, Math.floor((now - stageStart) / 1000))
          : (typeof task.activeStageDuration === 'number' && Number.isFinite(task.activeStageDuration) ? task.activeStageDuration + 1 : 1);

        const totalElapsed = task.createdAt
          ? Math.max(0, Math.floor((now - task.createdAt) / 1000))
          : (typeof task.totalElapsed === 'number' && Number.isFinite(task.totalElapsed) ? task.totalElapsed + 1 : 1);

        return {
          ...task,
          activeStageDuration,
          totalElapsed,
        };
      }),
    }));
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

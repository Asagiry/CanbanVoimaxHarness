// In-flight debounced save timers per task
export const activitySaveTimers: Record<string, ReturnType<typeof setTimeout>> = {};
export const dialogSaveTimers = activitySaveTimers; // backward-compatible alias

let flushCallback: ((taskId: string) => void) | null = null;

export function registerActivityFlushCallback(cb: (taskId: string) => void) {
  flushCallback = cb;
}
export const registerDialogFlushCallback = registerActivityFlushCallback; // backward-compatible alias

export function cancelActivitySave(taskId: string) {
  if (activitySaveTimers[taskId]) {
    clearTimeout(activitySaveTimers[taskId]);
    delete activitySaveTimers[taskId];
  }
}
export const cancelDialogSave = cancelActivitySave; // backward-compatible alias

export function scheduleActivitySave(taskId: string, delayMs = 1000) {
  if (activitySaveTimers[taskId]) {
    clearTimeout(activitySaveTimers[taskId]);
  }
  activitySaveTimers[taskId] = setTimeout(() => {
    delete activitySaveTimers[taskId];
    if (flushCallback) {
      flushCallback(taskId);
    }
  }, delayMs);
}
export const scheduleDialogSave = scheduleActivitySave; // backward-compatible alias

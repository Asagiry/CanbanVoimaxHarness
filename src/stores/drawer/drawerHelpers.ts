// In-flight debounced save timers per task
export const dialogSaveTimers: Record<string, ReturnType<typeof setTimeout>> = {};

let flushCallback: ((taskId: string) => void) | null = null;

export function registerDialogFlushCallback(cb: (taskId: string) => void) {
  flushCallback = cb;
}

export function cancelDialogSave(taskId: string) {
  if (dialogSaveTimers[taskId]) {
    clearTimeout(dialogSaveTimers[taskId]);
    delete dialogSaveTimers[taskId];
  }
}

export function scheduleDialogSave(taskId: string, delayMs = 1000) {
  if (dialogSaveTimers[taskId]) {
    clearTimeout(dialogSaveTimers[taskId]);
  }
  dialogSaveTimers[taskId] = setTimeout(() => {
    delete dialogSaveTimers[taskId];
    if (flushCallback) {
      flushCallback(taskId);
    }
  }, delayMs);
}

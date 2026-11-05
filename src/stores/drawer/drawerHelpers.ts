import type { DrawerActivityItem } from './types';

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

function parseActivityTimestampMs(act?: DrawerActivityItem): number {
  if (!act) return NaN;
  if (typeof act.createdAt === 'number' && Number.isFinite(act.createdAt) && act.createdAt > 0) {
    return act.createdAt;
  }
  if (typeof act.tool?.createdAt === 'number' && Number.isFinite(act.tool.createdAt) && act.tool.createdAt > 0) {
    return act.tool.createdAt;
  }
  if (act.timestamp) {
    const parsed = new Date(act.timestamp).getTime();
    if (!isNaN(parsed) && parsed > 0) return parsed;
    const isoTime = new Date(`1970-01-01T${act.timestamp}Z`).getTime();
    if (!isNaN(isoTime) && isoTime > 0) return isoTime;
  }
  return NaN;
}

/**
 * Pre-computes elapsed durations for activity items based on timestamps, createdAt, or explicit durationMs.
 */
export function computeActivityDurations(
  activities: DrawerActivityItem[],
  isRunning: boolean
): (number | undefined)[] {
  const list = activities || [];
  const res: (number | undefined)[] = Array.from({ length: list.length });
  for (let i = 0; i < list.length; i++) {
    const act = list[i];
    if (act?.durationMs && act.durationMs > 0) {
      res[i] = act.durationMs;
    } else if (act?.tool?.durationMs && act.tool.durationMs > 0) {
      res[i] = act.tool.durationMs;
    } else {
      const curTime = parseActivityTimestampMs(act);
      if (!isNaN(curTime) && curTime > 0) {
        let nextTime = NaN;
        for (let j = i + 1; j < list.length; j++) {
          const t = parseActivityTimestampMs(list[j]);
          if (!isNaN(t) && t > curTime) {
            nextTime = t;
            break;
          }
        }
        if (!isNaN(nextTime) && nextTime >= curTime) {
          const diff = nextTime - curTime;
          if (diff > 50 && diff < 3600000) {
            res[i] = diff;
          }
        } else if (i === list.length - 1 && isRunning) {
          const diff = Date.now() - curTime;
          if (diff > 50 && diff < 3600000) {
            res[i] = diff;
          }
        }
      }
    }
  }
  return res;
}


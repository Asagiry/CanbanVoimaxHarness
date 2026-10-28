import { useSettingsStore } from '../stores/settingsStore';
import { useDrawerStore } from '../stores/drawerStore';

export interface TaskAdvancedPayload {
  taskId: string;
  sourceTaskId: string;
  targetTaskId: string;
  sourceStageId: string;
  sourceStageTitle: string;
  targetStageId: string;
  targetStageTitle: string;
  isTerminal: boolean;
  summary: string;
}

export interface QuestionNotificationPayload {
  taskId: string;
  questionId: string;
  text: string;
}

export interface NotificationData {
  type: 'advance_task' | 'question';
  taskId: string;
  targetTaskId?: string;
  questionId?: string;
}

let isInitialized = false;
let notificationCounter = 1;
const notificationDataMap = new Map<number, NotificationData>();

async function handleNotificationClick(data?: NotificationData) {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('focus_main_window');
  } catch (err) {
    console.warn('focus_main_window error:', err);
  }

  const targetTaskId = data?.targetTaskId || data?.taskId;
  if (targetTaskId) {
    useDrawerStore.getState().openDrawer(targetTaskId, 'thoughts');
  }
}

export async function initNotifications(): Promise<void> {
  if (isInitialized) return;
  isInitialized = true;

  try {
    const { isPermissionGranted, requestPermission, onAction } = await import('@tauri-apps/plugin-notification');
    let granted = await isPermissionGranted();
    if (!granted) {
      const permission = await requestPermission();
      granted = permission === 'granted';
    }

    await onAction((action) => {
      const notifId = action.notification?.id;
      const extraData = action.notification?.extra as NotificationData | undefined;
      const data = (notifId !== undefined ? notificationDataMap.get(notifId) : undefined) || extraData;
      void handleNotificationClick(data);
    });
  } catch (err) {
    console.info('Notification plugin initialization notice:', err);
  }
}

export async function showAdvanceTaskNotification(payload: TaskAdvancedPayload): Promise<void> {
  const { notificationSettings } = useSettingsStore.getState();
  if (!notificationSettings.enabled) return;

  if (payload.isTerminal) {
    if (!notificationSettings.onTaskCompleted) return;
  } else {
    if (!notificationSettings.onAdvanceTask) return;
  }

  const title = payload.isTerminal
    ? `Задача ${payload.taskId} завершена.`
    : `Задача ${payload.taskId} перешла в колонку ${payload.targetStageTitle || payload.targetStageId}`;

  const cleanSummary = payload.summary
    ? payload.summary.replace(/###\s*[^\n]*\n?/g, '').trim()
    : '';
  const body = cleanSummary.length > 150 ? cleanSummary.slice(0, 147) + '...' : cleanSummary;

  const id = ++notificationCounter;
  const data: NotificationData = {
    type: 'advance_task',
    taskId: payload.taskId,
    targetTaskId: payload.targetTaskId || payload.taskId,
  };

  notificationDataMap.set(id, data);
  if (notificationDataMap.size > 50) {
    const oldestKey = notificationDataMap.keys().next().value;
    if (oldestKey !== undefined) notificationDataMap.delete(oldestKey);
  }

  try {
    const { sendNotification, isPermissionGranted, requestPermission } = await import('@tauri-apps/plugin-notification');
    let granted = await isPermissionGranted();
    if (!granted) {
      const permission = await requestPermission();
      granted = permission === 'granted';
    }
    if (granted) {
      sendNotification({
        id,
        title,
        body: body || undefined,
        extra: data as unknown as Record<string, unknown>,
      });
    }
  } catch (err) {
    console.warn('Failed to send advance_task notification:', err);
  }
}

export async function showQuestionNotification(payload: QuestionNotificationPayload): Promise<void> {
  const { notificationSettings } = useSettingsStore.getState();
  if (!notificationSettings.enabled || !notificationSettings.onQuestion) return;

  const title = `В задаче ${payload.taskId} вопрос от агента.`;
  const cleanText = payload.text ? payload.text.trim() : '';
  const body = cleanText.length > 150 ? cleanText.slice(0, 147) + '...' : cleanText;

  const id = ++notificationCounter;
  const data: NotificationData = {
    type: 'question',
    taskId: payload.taskId,
    targetTaskId: payload.taskId,
    questionId: payload.questionId,
  };

  notificationDataMap.set(id, data);
  if (notificationDataMap.size > 50) {
    const oldestKey = notificationDataMap.keys().next().value;
    if (oldestKey !== undefined) notificationDataMap.delete(oldestKey);
  }

  try {
    const { sendNotification, isPermissionGranted, requestPermission } = await import('@tauri-apps/plugin-notification');
    let granted = await isPermissionGranted();
    if (!granted) {
      const permission = await requestPermission();
      granted = permission === 'granted';
    }
    if (granted) {
      sendNotification({
        id,
        title,
        body: body || undefined,
        extra: data as unknown as Record<string, unknown>,
      });
    }
  } catch (err) {
    console.warn('Failed to send question notification:', err);
  }
}

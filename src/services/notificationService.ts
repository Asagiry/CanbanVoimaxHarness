import { useSettingsStore } from '../stores/settingsStore';
import { useDrawerStore } from '../stores/drawerStore';
import {
  formatMarkdownForToast,
  formatNotificationTitle,
  getStageIconName,
} from './notificationFormatter';

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

export interface TaskRejectedPayload {
  taskId: string;
  reason?: string;
  toStage?: string;
}

export interface NotificationData {
  type: 'advance_task' | 'question' | 'reject_task' | 'test';
  taskId?: string;
  targetTaskId?: string;
  questionId?: string;
}

let isInitialized = false;
let notificationCounter = 1;
const notificationDataMap = new Map<number, NotificationData>();

export async function handleNotificationClick(data?: NotificationData): Promise<void> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('focus_main_window');
  } catch (err) {
    console.warn('[NotificationService] focus_main_window error:', err);
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
    const { onAction, isPermissionGranted, requestPermission } = await import('@tauri-apps/plugin-notification');
    let granted = await isPermissionGranted();
    if (!granted) {
      const res = await requestPermission();
      granted = res === 'granted';
    }

    await onAction((action) => {
      const notifId = action.notification?.id;
      const extraData = action.notification?.extra as NotificationData | undefined;
      const data = (notifId !== undefined ? notificationDataMap.get(notifId) : undefined) || extraData;
      void handleNotificationClick(data);
    });
  } catch (err) {
    console.info('[NotificationService] Notification action listener registration notice:', err);
  }
}

async function sendSystemNotification(
  title: string,
  body?: string,
  taskId?: string,
  icon?: string
): Promise<boolean> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('show_system_notification', {
      title,
      body: body || null,
      taskId: taskId || null,
      icon: icon || null,
    });
    return true;
  } catch (err) {
    console.warn('[NotificationService] Failed to show system notification via IPC:', err);
    return false;
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

  const title = formatNotificationTitle('advance_task', payload.taskId, {
    stageTitle: payload.targetStageTitle || payload.targetStageId,
    isTerminal: payload.isTerminal,
  });

  const body = formatMarkdownForToast(payload.summary, 320);
  const icon = getStageIconName(payload.targetStageId, payload.isTerminal);

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

  await sendSystemNotification(title, body || undefined, payload.taskId, icon);
}

export async function showQuestionNotification(payload: QuestionNotificationPayload): Promise<void> {
  const { notificationSettings } = useSettingsStore.getState();
  if (!notificationSettings.enabled || !notificationSettings.onQuestion) return;

  const title = formatNotificationTitle('question', payload.taskId);
  const body = formatMarkdownForToast(payload.text, 320);

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

  await sendSystemNotification(title, body || undefined, payload.taskId, 'question');
}

export async function showRejectTaskNotification(payload: TaskRejectedPayload): Promise<void> {
  const { notificationSettings } = useSettingsStore.getState();
  if (!notificationSettings.enabled) return;

  const title = formatNotificationTitle('reject_task', payload.taskId);
  const body = formatMarkdownForToast(payload.reason, 320);

  const id = ++notificationCounter;
  const data: NotificationData = {
    type: 'reject_task',
    taskId: payload.taskId,
    targetTaskId: payload.taskId,
  };

  notificationDataMap.set(id, data);
  if (notificationDataMap.size > 50) {
    const oldestKey = notificationDataMap.keys().next().value;
    if (oldestKey !== undefined) notificationDataMap.delete(oldestKey);
  }

  await sendSystemNotification(title, body || undefined, payload.taskId, 'reject');
}

export async function showTestNotification(): Promise<boolean> {
  const title = formatNotificationTitle('test');
  const body = 'Тестовое системное уведомление доставлено успешно.';

  const id = ++notificationCounter;
  const data: NotificationData = {
    type: 'test',
  };

  notificationDataMap.set(id, data);
  if (notificationDataMap.size > 50) {
    const oldestKey = notificationDataMap.keys().next().value;
    if (oldestKey !== undefined) notificationDataMap.delete(oldestKey);
  }

  return await sendSystemNotification(title, body, undefined, 'app');
}

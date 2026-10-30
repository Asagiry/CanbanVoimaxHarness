import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useSettingsStore } from '../../stores/settingsStore';
import { useDrawerStore } from '../../stores/drawerStore';
import {
  initNotifications,
  handleNotificationClick,
  showAdvanceTaskNotification,
  showQuestionNotification,
  showRejectTaskNotification,
  showTestNotification,
  type NotificationData,
} from '../notificationService';

const mockInvoke = vi.fn();
const mockOnAction = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock('@tauri-apps/plugin-notification', () => ({
  onAction: (...args: unknown[]) => mockOnAction(...args),
}));

describe('notificationService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue(undefined);
    useSettingsStore.setState({
      notificationSettings: {
        enabled: true,
        onAdvanceTask: true,
        onQuestion: true,
        onTaskCompleted: true,
      },
    });
  });

  describe('initNotifications & handleNotificationClick', () => {
    it('initializes notification plugin action listener', async () => {
      await initNotifications();
      expect(mockOnAction).toHaveBeenCalledTimes(1);

      // Calling again should be idempotent
      await initNotifications();
      expect(mockOnAction).toHaveBeenCalledTimes(1);
    });

    it('focuses main window and opens drawer on notification click with taskId', async () => {
      const openDrawerSpy = vi.spyOn(useDrawerStore.getState(), 'openDrawer');
      const data: NotificationData = {
        type: 'advance_task',
        taskId: 'CVH-100',
        targetTaskId: 'CVH-100',
      };

      await handleNotificationClick(data);

      expect(mockInvoke).toHaveBeenCalledWith('focus_main_window');
      expect(openDrawerSpy).toHaveBeenCalledWith('CVH-100', 'thoughts');
    });

    it('focuses main window without opening drawer when taskId is missing', async () => {
      const openDrawerSpy = vi.spyOn(useDrawerStore.getState(), 'openDrawer');
      const data: NotificationData = {
        type: 'test',
      };

      await handleNotificationClick(data);

      expect(mockInvoke).toHaveBeenCalledWith('focus_main_window');
      expect(openDrawerSpy).not.toHaveBeenCalled();
    });
  });

  describe('showAdvanceTaskNotification', () => {
    it('does not send notification when notifications are disabled globally', async () => {
      useSettingsStore.setState({
        notificationSettings: {
          enabled: false,
          onAdvanceTask: true,
          onQuestion: true,
          onTaskCompleted: true,
        },
      });

      await showAdvanceTaskNotification({
        taskId: 'CVH-001',
        sourceTaskId: 'CVH-001',
        targetTaskId: 'CVH-001',
        sourceStageId: 'dev',
        sourceStageTitle: 'Development',
        targetStageId: 'review',
        targetStageTitle: 'Review',
        isTerminal: false,
        summary: 'Completed feature implementation',
      });

      expect(mockInvoke).not.toHaveBeenCalledWith('show_system_notification', expect.anything());
    });

    it('does not send advance notification when onAdvanceTask is false', async () => {
      useSettingsStore.setState({
        notificationSettings: {
          enabled: true,
          onAdvanceTask: false,
          onQuestion: true,
          onTaskCompleted: true,
        },
      });

      await showAdvanceTaskNotification({
        taskId: 'CVH-001',
        sourceTaskId: 'CVH-001',
        targetTaskId: 'CVH-001',
        sourceStageId: 'dev',
        sourceStageTitle: 'Development',
        targetStageId: 'review',
        targetStageTitle: 'Review',
        isTerminal: false,
        summary: 'Completed feature implementation',
      });

      expect(mockInvoke).not.toHaveBeenCalledWith('show_system_notification', expect.anything());
    });

    it('sends notification for non-terminal stage transition', async () => {
      await showAdvanceTaskNotification({
        taskId: 'CVH-001',
        sourceTaskId: 'CVH-001',
        targetTaskId: 'CVH-001',
        sourceStageId: 'dev',
        sourceStageTitle: 'Development',
        targetStageId: 'review',
        targetStageTitle: 'Review',
        isTerminal: false,
        summary: 'Feature implementation complete',
      });

      expect(mockInvoke).toHaveBeenCalledWith('show_system_notification', {
        title: 'Задача CVH-001 перешла в колонку Review',
        body: 'Feature implementation complete',
        taskId: 'CVH-001',
      });
    });

    it('sends notification for terminal stage transition when onTaskCompleted is enabled', async () => {
      await showAdvanceTaskNotification({
        taskId: 'CVH-001',
        sourceTaskId: 'CVH-001',
        targetTaskId: 'CVH-001',
        sourceStageId: 'review',
        sourceStageTitle: 'Review',
        targetStageId: 'done',
        targetStageTitle: 'Done',
        isTerminal: true,
        summary: 'All checks passed',
      });

      expect(mockInvoke).toHaveBeenCalledWith('show_system_notification', {
        title: 'Задача CVH-001 завершена.',
        body: 'All checks passed',
        taskId: 'CVH-001',
      });
    });

    it('does not send completion notification when onTaskCompleted is disabled', async () => {
      useSettingsStore.setState({
        notificationSettings: {
          enabled: true,
          onAdvanceTask: true,
          onQuestion: true,
          onTaskCompleted: false,
        },
      });

      await showAdvanceTaskNotification({
        taskId: 'CVH-001',
        sourceTaskId: 'CVH-001',
        targetTaskId: 'CVH-001',
        sourceStageId: 'review',
        sourceStageTitle: 'Review',
        targetStageId: 'done',
        targetStageTitle: 'Done',
        isTerminal: true,
        summary: 'All checks passed',
      });

      expect(mockInvoke).not.toHaveBeenCalledWith('show_system_notification', expect.anything());
    });
  });

  describe('showQuestionNotification', () => {
    it('sends question notification when enabled', async () => {
      await showQuestionNotification({
        taskId: 'CVH-042',
        questionId: 'q-1',
        text: 'Should we proceed with SQLite WAL mode?',
      });

      expect(mockInvoke).toHaveBeenCalledWith('show_system_notification', {
        title: 'В задаче CVH-042 вопрос от агента.',
        body: 'Should we proceed with SQLite WAL mode?',
        taskId: 'CVH-042',
      });
    });

    it('does not send question notification when onQuestion is false', async () => {
      useSettingsStore.setState({
        notificationSettings: {
          enabled: true,
          onAdvanceTask: true,
          onQuestion: false,
          onTaskCompleted: true,
        },
      });

      await showQuestionNotification({
        taskId: 'CVH-042',
        questionId: 'q-1',
        text: 'Question text',
      });

      expect(mockInvoke).not.toHaveBeenCalledWith('show_system_notification', expect.anything());
    });
  });

  describe('showRejectTaskNotification', () => {
    it('sends reject task notification with reason', async () => {
      await showRejectTaskNotification({
        taskId: 'CVH-042',
        reason: 'Missing sensor validation and unit tests',
        toStage: 'dev',
      });

      expect(mockInvoke).toHaveBeenCalledWith('show_system_notification', {
        title: 'Задача CVH-042 возвращена на доработку.',
        body: 'Missing sensor validation and unit tests',
        taskId: 'CVH-042',
      });
    });

    it('does not send reject task notification when globally disabled', async () => {
      useSettingsStore.setState({
        notificationSettings: {
          enabled: false,
          onAdvanceTask: true,
          onQuestion: true,
          onTaskCompleted: true,
        },
      });

      await showRejectTaskNotification({
        taskId: 'CVH-042',
        reason: 'Some reason',
      });

      expect(mockInvoke).not.toHaveBeenCalledWith('show_system_notification', expect.anything());
    });
  });

  describe('showTestNotification', () => {
    it('sends test notification via IPC', async () => {
      await showTestNotification();

      expect(mockInvoke).toHaveBeenCalledWith('show_system_notification', {
        title: 'CanbanVoimaxHarness',
        body: 'Тестовое системное уведомление доставлено успешно.',
        taskId: null,
      });
    });
  });
});

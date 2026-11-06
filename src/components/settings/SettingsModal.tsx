import { useEffect, useRef, useState } from 'react';
import { Bell, Check, Cpu, X } from 'lucide-react';
import { useSettingsStore } from '../../stores/settingsStore';
import { useTranslation } from '../../stores/languageStore';
import { useUiZoom, getZoomOverlayStyle } from '../../hooks/useUiZoom';
import { showTestNotification } from '../../services/notificationService';
import ProviderList from './providers/ProviderList';
import ProviderEditor from './providers/ProviderEditor';
import { toProviderIdValue, type Provider } from '../../stores/settings/providers';

export function NotificationToggle({
  checked,
  onChange,
  disabled = false,
  ariaLabel,
}: {
  checked: boolean;
  onChange: () => void;
  disabled?: boolean;
  ariaLabel?: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-label={ariaLabel}
      aria-checked={checked}
      disabled={disabled}
      onClick={onChange}
      className={`relative inline-flex h-5 w-9 shrink-0 items-center rounded-full p-0.5 transition-colors duration-200 ease-in-out focus:outline-hidden ${
        disabled ? 'opacity-40 cursor-not-allowed' : 'cursor-pointer'
      } ${checked ? 'bg-[#10b981]' : 'bg-[#e5e7eb] dark:bg-[#2c2c33]'}`}
    >
      <span
        aria-hidden="true"
        className={`pointer-events-none block h-4 w-4 transform rounded-full bg-white shadow-xs transition-transform duration-200 ease-in-out ${
          checked ? 'translate-x-4' : 'translate-x-0'
        }`}
      />
    </button>
  );
}

export default function SettingsModal() {
  const { zoom } = useUiZoom();
  const isOpen = useSettingsStore((s) => s.isOpen);
  const notificationSettings = useSettingsStore((s) => s.notificationSettings);
  const isSaving = useSettingsStore((s) => s.isSaving);
  const saveStatus = useSettingsStore((s) => s.saveStatus);
  const closeSettings = useSettingsStore((s) => s.closeSettings);
  const setNotificationSettings = useSettingsStore((s) => s.setNotificationSettings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const providers = useSettingsStore((s) => s.providers);

  const [activeTab, setActiveTab] = useState<'llm' | 'notifications'>('llm');
  const [testNotifSent, setTestNotifSent] = useState(false);
  const [editingProvider, setEditingProvider] = useState<Provider | null>(null);
  const { t } = useTranslation();
  const closeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const testNotifTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (closeTimerRef.current) clearTimeout(closeTimerRef.current);
      if (testNotifTimerRef.current) clearTimeout(testNotifTimerRef.current);
    };
  }, []);

  if (!isOpen) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    await saveSettings();
    if (closeTimerRef.current) clearTimeout(closeTimerRef.current);
    closeTimerRef.current = setTimeout(() => {
      closeSettings();
      closeTimerRef.current = null;
    }, 400);
  };

  const handleAddProvider = () => {
    const id = toProviderIdValue(`custom-${providers.length + 1}`);
    setEditingProvider({
      id,
      name: 'custom',
      isCustom: true,
      apiKey: '',
      baseUrl: '',
      protocol: 'openai',
      models: [],
    });
  };

  const handleTestNotification = async () => {
    await showTestNotification();
    setTestNotifSent(true);
    if (testNotifTimerRef.current) clearTimeout(testNotifTimerRef.current);
    testNotifTimerRef.current = setTimeout(() => {
      setTestNotifSent(false);
      testNotifTimerRef.current = null;
    }, 2000);
  };

  return (
    <div style={getZoomOverlayStyle(zoom)} className="fixed inset-0 z-[80] flex items-center justify-center bg-black/60 dark:bg-black/80 p-4 animate-in fade-in duration-100">
      <div className="relative flex flex-col w-[500px] max-w-full max-h-[90%] overflow-y-auto rounded-lg border border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114] shadow-md p-5 text-[#171717] dark:text-[#f4f4f5] transition-colors">
        <div className="flex items-center justify-between pb-3 border-b border-[#ebebeb] dark:border-[#212126]">
          <div className="flex items-center space-x-2">
            <h2 className="text-sm font-semibold text-[#171717] dark:text-[#f4f4f5] tracking-tight">{t('settings.title')}</h2>
          </div>
          <button
            onClick={closeSettings}
            className="p-1 rounded-md text-[#666666] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#1e1e23] transition cursor-pointer"
            aria-label={t('common.close')}
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="flex border-b border-[#ebebeb] dark:border-[#212126] mt-2">
          <button
            type="button"
            onClick={() => setActiveTab('llm')}
            className={`px-3 py-1.5 text-xs font-sans font-medium transition cursor-pointer border-b-2 -mb-px flex items-center gap-1.5 ${
              activeTab === 'llm'
                ? 'border-[#171717] dark:border-[#f4f4f5] text-[#171717] dark:text-[#f4f4f5]'
                : 'border-transparent text-[#666666] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5]'
            }`}
          >
            <Cpu className="h-3.5 w-3.5" />
            <span>{t('settings.tabLlm')}</span>
          </button>
          <button
            type="button"
            onClick={() => setActiveTab('notifications')}
            className={`px-3 py-1.5 text-xs font-sans font-medium transition cursor-pointer border-b-2 -mb-px flex items-center gap-1.5 ${
              activeTab === 'notifications'
                ? 'border-[#171717] dark:border-[#f4f4f5] text-[#171717] dark:text-[#f4f4f5]'
                : 'border-transparent text-[#666666] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5]'
            }`}
          >
            <Bell className="h-3.5 w-3.5" />
            <span>{t('settings.tabNotifications')}</span>
          </button>
        </div>

        <form onSubmit={handleSubmit} className="mt-4 space-y-3.5 text-xs font-mono">
          {activeTab === 'llm' ? (
            editingProvider ? (
              <ProviderEditor
                provider={editingProvider}
                onCancel={() => setEditingProvider(null)}
                onSaved={() => setEditingProvider(null)}
              />
            ) : (
              <ProviderList onEdit={(provider) => setEditingProvider(provider)} onAdd={handleAddProvider} />
            )
          ) : (
            <div className="space-y-4 py-1">
              <div className="flex items-center justify-between p-3 rounded-lg border border-[#ebebeb] dark:border-[#212126] bg-[#fafafa] dark:bg-[#17171b]">
                <div className="space-y-0.5 pr-2">
                  <span className="text-xs font-medium text-[#171717] dark:text-[#f4f4f5] font-sans block">
                    {t('settings.notifyWindowsMaster')}
                  </span>
                  <span className="text-[10px] text-[#666666] dark:text-[#71717a] font-sans block">
                    {t('settings.notificationsDesc')}
                  </span>
                </div>
                <NotificationToggle
                  checked={notificationSettings.enabled}
                  onChange={() => setNotificationSettings({ enabled: !notificationSettings.enabled })}
                  ariaLabel={t('settings.notifyWindowsMaster')}
                />
              </div>

              <div className={`space-y-2 px-3 transition-opacity ${notificationSettings.enabled ? 'opacity-100' : 'opacity-40 pointer-events-none'}`}>
                <div className="flex items-center justify-between py-1.5 border-b border-[#f4f4f5] dark:border-[#212126]">
                  <span className="text-xs text-[#171717] dark:text-[#f4f4f5] font-sans">{t('settings.notifyOnQuestion')}</span>
                  <NotificationToggle
                    checked={notificationSettings.onQuestion}
                    disabled={!notificationSettings.enabled}
                    onChange={() => setNotificationSettings({ onQuestion: !notificationSettings.onQuestion })}
                    ariaLabel={t('settings.notifyOnQuestion')}
                  />
                </div>
                <div className="flex items-center justify-between py-1.5 border-b border-[#f4f4f5] dark:border-[#212126]">
                  <span className="text-xs text-[#171717] dark:text-[#f4f4f5] font-sans">{t('settings.notifyOnAdvanceTask')}</span>
                  <NotificationToggle
                    checked={notificationSettings.onAdvanceTask}
                    disabled={!notificationSettings.enabled}
                    onChange={() => setNotificationSettings({ onAdvanceTask: !notificationSettings.onAdvanceTask })}
                    ariaLabel={t('settings.notifyOnAdvanceTask')}
                  />
                </div>
                <div className="flex items-center justify-between py-1.5">
                  <span className="text-xs text-[#171717] dark:text-[#f4f4f5] font-sans">{t('settings.notifyOnTaskCompleted')}</span>
                  <NotificationToggle
                    checked={notificationSettings.onTaskCompleted}
                    disabled={!notificationSettings.enabled}
                    onChange={() => setNotificationSettings({ onTaskCompleted: !notificationSettings.onTaskCompleted })}
                    ariaLabel={t('settings.notifyOnTaskCompleted')}
                  />
                </div>
              </div>

              <div className="pt-2 border-t border-[#ebebeb] dark:border-[#212126]">
                <button
                  type="button"
                  onClick={() => void handleTestNotification()}
                  className="flex items-center space-x-2 px-3 py-1.5 rounded-md border border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] text-[#171717] dark:text-[#f4f4f5] text-xs font-sans transition cursor-pointer"
                >
                  {testNotifSent ? (
                    <>
                      <Check className="w-3.5 h-3.5 text-emerald-500" />
                      <span className="text-emerald-600 dark:text-emerald-400 font-medium">{t('settings.testNotificationSent')}</span>
                    </>
                  ) : (
                    <>
                      <Bell className="w-3.5 h-3.5 text-[#666666] dark:text-[#71717a]" />
                      <span>{t('settings.sendTestNotification')}</span>
                    </>
                  )}
                </button>
              </div>
            </div>
          )}

          <div className="flex items-center justify-end space-x-2 pt-3 border-t border-[#ebebeb] dark:border-[#212126]">
            <button
              type="button"
              onClick={closeSettings}
              className="px-3 py-1.5 rounded-md bg-white dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] text-[#171717] dark:text-[#f4f4f5] font-sans text-xs transition cursor-pointer"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={isSaving}
              className="flex items-center space-x-1.5 px-3.5 py-1.5 rounded-md bg-[#171717] dark:bg-[#232328] hover:bg-[#262626] dark:hover:bg-[#2c2c33] text-white font-sans font-medium text-xs transition cursor-pointer disabled:opacity-50 shadow-xs border border-transparent dark:border-[#2c2c33]"
            >
              {saveStatus === 'saved' ? (
                <>
                  <Check className="h-3.5 w-3.5" />
                  <span>{t('common.saved')}</span>
                </>
              ) : (
                <span>{t('common.save')}</span>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

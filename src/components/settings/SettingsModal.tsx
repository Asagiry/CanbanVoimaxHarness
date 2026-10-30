import { useState, useId, useMemo, useRef, useEffect } from 'react';
import { useSettingsStore } from '../../stores/settingsStore';
import { useTranslation } from '../../stores/languageStore';
import { useUiZoom, getZoomOverlayStyle } from '../../hooks/useUiZoom';
import {
  X,
  Check,
  Eye,
  EyeOff,
  KeyRound,
  Globe,
  Cpu,
  RefreshCw,
  Search,
  CheckSquare,
  Square,
  Bell,
} from 'lucide-react';

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
      } ${
        checked ? 'bg-[#10b981]' : 'bg-[#e5e7eb] dark:bg-[#30363d]'
      }`}
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
  const {
    isOpen,
    settings,
    notificationSettings,
    availableModels,
    isSaving,
    saveStatus,
    closeSettings,
    setSettings,
    setNotificationSettings,
    saveSettings,
    addAvailableModels,
    fetchAvailableModels,
  } = useSettingsStore();

  const [activeTab, setActiveTab] = useState<'llm' | 'notifications'>('llm');
  const [showApiKey, setShowApiKey] = useState(false);
  const [isChooseModelsModalOpen, setIsChooseModelsModalOpen] = useState(false);
  const [isFetchingModels, setIsFetchingModels] = useState(false);
  const [fetchedCandidates, setFetchedCandidates] = useState<string[]>([]);
  const [selectedCandidateModels, setSelectedCandidateModels] = useState<Set<string>>(new Set());
  const [searchFilter, setSearchFilter] = useState('');
  const [isCustomModelInput, setIsCustomModelInput] = useState(false);
  const { t } = useTranslation();
  const closeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (closeTimerRef.current) {
        clearTimeout(closeTimerRef.current);
      }
    };
  }, []);

  const providerId = useId();
  const gatewayId = useId();
  const apiKeyId = useId();
  const modelId = useId();
  const compactorModelId = useId();

  // Filtered candidate models in "Choose models to add" modal
  const filteredCandidates = useMemo(() => {
    if (!searchFilter.trim()) return fetchedCandidates;
    const q = searchFilter.toLowerCase();
    return fetchedCandidates.filter((m) => m.toLowerCase().includes(q));
  }, [fetchedCandidates, searchFilter]);

  if (!isOpen) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    await saveSettings();
    if (closeTimerRef.current) {
      clearTimeout(closeTimerRef.current);
    }
    closeTimerRef.current = setTimeout(() => {
      closeSettings();
      closeTimerRef.current = null;
    }, 400);
  };

  const handleFetchModels = async () => {
    setIsFetchingModels(true);
    try {
      const models = await fetchAvailableModels(
        settings.baseUrl || settings.gatewayUrl,
        settings.apiKey
      );
      setFetchedCandidates(models);
      // Select all candidates by default
      setSelectedCandidateModels(new Set(models));
      setSearchFilter('');
      setIsChooseModelsModalOpen(true);
    } catch (err) {
      console.error('Failed to fetch available models:', err);
    } finally {
      setIsFetchingModels(false);
    }
  };

  const toggleSelectCandidate = (modelName: string) => {
    setSelectedCandidateModels((prev) => {
      const next = new Set(prev);
      if (next.has(modelName)) {
        next.delete(modelName);
      } else {
        next.add(modelName);
      }
      return next;
    });
  };

  const handleToggleSelectAll = () => {
    if (selectedCandidateModels.size === filteredCandidates.length) {
      setSelectedCandidateModels(new Set());
    } else {
      setSelectedCandidateModels(new Set(filteredCandidates));
    }
  };

  const handleAddSelectedModels = () => {
    const toAdd = Array.from(selectedCandidateModels);
    if (toAdd.length > 0) {
      addAvailableModels(toAdd);
      // If current model is empty or not in available list, set to first added
      if (!settings.model || settings.model.trim() === '') {
        setSettings({ model: toAdd[0] });
      }
    }
    setIsChooseModelsModalOpen(false);
  };

  return (
    <>
      <div style={getZoomOverlayStyle(zoom)} className="fixed inset-0 z-[80] flex items-center justify-center bg-black/60 dark:bg-black/80 p-4 animate-in fade-in duration-100">
        <div className="relative flex flex-col w-[500px] max-w-full rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] shadow-md p-5 text-[#171717] dark:text-[#f0f6fc] transition-colors">
          {/* Header */}
          <div className="flex items-center justify-between pb-3 border-b border-[#ebebeb] dark:border-[#21262d]">
            <div className="flex items-center space-x-2">
              <h2 className="text-sm font-semibold text-[#171717] dark:text-[#f0f6fc] tracking-tight">{t('settings.title')}</h2>
            </div>
            <button
              onClick={closeSettings}
              className="p-1 rounded-md text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] transition cursor-pointer"
              aria-label={t('common.close')}
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          {/* Tabs */}
          <div className="flex border-b border-[#ebebeb] dark:border-[#21262d] mt-2">
            <button
              type="button"
              onClick={() => setActiveTab('llm')}
              className={`px-3 py-1.5 text-xs font-sans font-medium transition cursor-pointer border-b-2 -mb-px flex items-center gap-1.5 ${
                activeTab === 'llm'
                  ? 'border-[#171717] dark:border-[#f0f6fc] text-[#171717] dark:text-[#f0f6fc]'
                  : 'border-transparent text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc]'
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
                  ? 'border-[#171717] dark:border-[#f0f6fc] text-[#171717] dark:text-[#f0f6fc]'
                  : 'border-transparent text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc]'
              }`}
            >
              <Bell className="h-3.5 w-3.5" />
              <span>{t('settings.tabNotifications')}</span>
            </button>
          </div>

          {/* Form Body */}
          <form onSubmit={handleSubmit} className="mt-4 space-y-3.5 text-xs font-mono">
            {activeTab === 'llm' ? (
              <>
            {/* Provider */}
            <div className="space-y-1">
              <label htmlFor={providerId} className="block text-[#4d4d4d] dark:text-[#8b949e] font-sans text-xs font-medium">
                {t('settings.provider')}
              </label>
              <div className="relative">
                <select
                  id={providerId}
                  value={settings.provider}
                  onChange={(e) => setSettings({ provider: e.target.value })}
                  className="w-full rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] px-2.5 py-1.5 text-[#171717] dark:text-[#f0f6fc] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#0d1117] font-mono transition"
                >
                  <option value="Antigravity Proxy">Antigravity Proxy</option>
                  <option value="OpenAI">OpenAI</option>
                  <option value="Anthropic">Anthropic</option>
                  <option value="Ollama">Ollama</option>
                  {!['Antigravity Proxy', 'OpenAI', 'Anthropic', 'Ollama', 'Custom'].includes(settings.provider) && (
                    <option value={settings.provider}>{settings.provider}</option>
                  )}
                  <option value="Custom">Custom</option>
                </select>
              </div>
            </div>

            {/* API Gateway URL */}
            <div className="space-y-1">
              <label htmlFor={gatewayId} className="block text-[#4d4d4d] dark:text-[#8b949e] font-sans text-xs font-medium">
                {t('settings.gatewayUrl')}
              </label>
              <div className="relative flex items-center">
                <Globe className="absolute left-2.5 h-3.5 w-3.5 text-[#8f8f8f] pointer-events-none" />
                <input
                  id={gatewayId}
                  type="text"
                  value={settings.gatewayUrl}
                  onChange={(e) => setSettings({ gatewayUrl: e.target.value, baseUrl: e.target.value })}
                  placeholder="http://127.0.0.1:port/v1"
                  className="w-full rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] pl-8 pr-2.5 py-1.5 text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#6e7681] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#0d1117] font-mono transition"
                  required
                />
              </div>
            </div>

            {/* API Key (Masked) */}
            <div className="space-y-1">
              <label htmlFor={apiKeyId} className="block text-[#4d4d4d] dark:text-[#8b949e] font-sans text-xs font-medium">
                {t('settings.apiKey')}
              </label>
              <div className="relative flex items-center">
                <KeyRound className="absolute left-2.5 h-3.5 w-3.5 text-[#8f8f8f] pointer-events-none" />
                <input
                  id={apiKeyId}
                  type={showApiKey ? 'text' : 'password'}
                  value={settings.apiKey}
                  onChange={(e) => setSettings({ apiKey: e.target.value })}
                  placeholder={
                    settings.provider === 'Antigravity Proxy' || settings.provider === 'Ollama'
                      ? t('settings.apiKeyPlaceholderProxy')
                      : 'sk-...'
                  }
                  className="w-full rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] pl-8 pr-8 py-1.5 text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#6e7681] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#0d1117] font-mono transition"
                />
                <button
                  type="button"
                  onClick={() => setShowApiKey(!showApiKey)}
                  className="absolute right-2 p-1 text-[#666666] hover:text-[#171717] cursor-pointer"
                  aria-label={showApiKey ? t('settings.hideKey') : t('settings.showKey')}
                >
                  {showApiKey ? <EyeOff className="h-3.5 w-3.5" /> : <Eye className="h-3.5 w-3.5" />}
                </button>
              </div>
            </div>

            {/* Model Section */}
            <div className="space-y-1 pt-1">
              <div className="flex items-center justify-between">
                <label htmlFor={modelId} className="block text-[#4d4d4d] dark:text-[#8b949e] font-sans text-xs font-medium">
                  {t('settings.model')}
                </label>
                <button
                  type="button"
                  onClick={handleFetchModels}
                  disabled={isFetchingModels}
                  className="flex items-center space-x-1.5 text-[11px] font-sans font-medium text-[#171717] dark:text-[#f0f6fc] hover:text-[#4d4d4d] dark:hover:text-[#8b949e] disabled:opacity-50 transition cursor-pointer px-1.5 py-0.5 rounded-md hover:bg-[#f4f4f5] dark:hover:bg-[#161b22]"
                  aria-label={t('settings.fetchModels')}
                >
                  <RefreshCw className={`h-3 w-3 ${isFetchingModels ? 'animate-spin' : ''}`} />
                  <span>{isFetchingModels ? t('settings.fetching') : t('settings.fetchModels')}</span>
                </button>
              </div>

              {isCustomModelInput ? (
                <div className="relative flex items-center">
                  <Cpu className="absolute left-2.5 h-3.5 w-3.5 text-[#8f8f8f] pointer-events-none" />
                  <input
                    id={modelId}
                    type="text"
                    value={settings.model}
                    onChange={(e) => setSettings({ model: e.target.value })}
                    placeholder={t('settings.customModelPlaceholder')}
                    className="w-full rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] pl-8 pr-2.5 py-1.5 text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#6e7681] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#0d1117] font-mono transition"
                    required
                  />
                </div>
              ) : (
                <div className="relative flex items-center">
                  <Cpu className="absolute left-2.5 h-3.5 w-3.5 text-[#8f8f8f] pointer-events-none" />
                  <select
                    id={modelId}
                    value={settings.model}
                    onChange={(e) => setSettings({ model: e.target.value })}
                    className="w-full rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] pl-8 pr-2.5 py-1.5 text-[#171717] dark:text-[#f0f6fc] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#0d1117] font-mono text-xs transition cursor-pointer"
                  >
                    {!availableModels.includes(settings.model) && settings.model && (
                      <option value={settings.model}>{settings.model}</option>
                    )}
                    {availableModels.map((m) => (
                      <option key={m} value={m}>
                        {m}
                      </option>
                    ))}
                  </select>
                </div>
              )}

              <div className="flex items-center justify-between text-[10px] text-[#666666] pt-0.5 font-sans">
                <span>
                  {t('settings.modelsCount', { count: availableModels.length })}
                </span>
                <button
                  type="button"
                  onClick={() => setIsCustomModelInput(!isCustomModelInput)}
                  className="text-[#666666] hover:text-[#171717] underline cursor-pointer"
                >
                  {isCustomModelInput ? t('settings.selectFromList') : t('settings.enterManually')}
                </button>
              </div>
            </div>

            {/* Compactor Model Section */}
            <div className="space-y-1 pt-1 border-t border-[#f4f4f5] dark:border-[#21262d]">
              <label htmlFor={compactorModelId} className="block text-[#4d4d4d] dark:text-[#8b949e] font-sans text-xs font-medium">
                {t('settings.compactorModel')}
              </label>
              <div className="relative flex items-center">
                <Cpu className="absolute left-2.5 h-3.5 w-3.5 text-[#8f8f8f] pointer-events-none" />
                <select
                  id={compactorModelId}
                  value={settings.compactorModel || 'inherit'}
                  onChange={(e) => setSettings({ compactorModel: e.target.value })}
                  className="w-full rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] pl-8 pr-2.5 py-1.5 text-[#171717] dark:text-[#f0f6fc] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#0d1117] font-mono text-xs transition cursor-pointer"
                >
                  <option value="inherit">{t('settings.inheritModel')}</option>
                  {availableModels.map((m) => (
                    <option key={m} value={m}>
                      {m}
                    </option>
                  ))}
                  {settings.compactorModel && settings.compactorModel !== 'inherit' && !availableModels.includes(settings.compactorModel) && (
                    <option value={settings.compactorModel}>{settings.compactorModel}</option>
                  )}
                </select>
              </div>
              <p className="text-[10px] text-[#666666] font-sans">
                {t('settings.compactorDesc')}
              </p>
            </div>
            </>
            ) : (
            <div className="space-y-4 py-1">
              {/* Master toggle */}
              <div className="flex items-center justify-between p-3 rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-[#fafafa] dark:bg-[#161b22]">
                <div className="space-y-0.5 pr-2">
                  <span className="text-xs font-medium text-[#171717] dark:text-[#f0f6fc] font-sans block">
                    {t('settings.notifyWindowsMaster')}
                  </span>
                  <span className="text-[10px] text-[#666666] dark:text-[#8b949e] font-sans block">
                    {t('settings.notificationsDesc')}
                  </span>
                </div>
                <NotificationToggle
                  checked={notificationSettings.enabled}
                  onChange={() => setNotificationSettings({ enabled: !notificationSettings.enabled })}
                  ariaLabel={t('settings.notifyWindowsMaster')}
                />
              </div>

              {/* Sub-toggles */}
              <div className={`space-y-2 px-3 transition-opacity ${notificationSettings.enabled ? 'opacity-100' : 'opacity-40 pointer-events-none'}`}>
                {/* On Question */}
                <div className="flex items-center justify-between py-1.5 border-b border-[#f4f4f5] dark:border-[#21262d]">
                  <span className="text-xs text-[#171717] dark:text-[#f0f6fc] font-sans">
                    {t('settings.notifyOnQuestion')}
                  </span>
                  <NotificationToggle
                    checked={notificationSettings.onQuestion}
                    disabled={!notificationSettings.enabled}
                    onChange={() => setNotificationSettings({ onQuestion: !notificationSettings.onQuestion })}
                    ariaLabel={t('settings.notifyOnQuestion')}
                  />
                </div>

                {/* On Advance Task */}
                <div className="flex items-center justify-between py-1.5 border-b border-[#f4f4f5] dark:border-[#21262d]">
                  <span className="text-xs text-[#171717] dark:text-[#f0f6fc] font-sans">
                    {t('settings.notifyOnAdvanceTask')}
                  </span>
                  <NotificationToggle
                    checked={notificationSettings.onAdvanceTask}
                    disabled={!notificationSettings.enabled}
                    onChange={() => setNotificationSettings({ onAdvanceTask: !notificationSettings.onAdvanceTask })}
                    ariaLabel={t('settings.notifyOnAdvanceTask')}
                  />
                </div>

                {/* On Task Completed */}
                <div className="flex items-center justify-between py-1.5">
                  <span className="text-xs text-[#171717] dark:text-[#f0f6fc] font-sans">
                    {t('settings.notifyOnTaskCompleted')}
                  </span>
                  <NotificationToggle
                    checked={notificationSettings.onTaskCompleted}
                    disabled={!notificationSettings.enabled}
                    onChange={() => setNotificationSettings({ onTaskCompleted: !notificationSettings.onTaskCompleted })}
                    ariaLabel={t('settings.notifyOnTaskCompleted')}
                  />
                </div>
              </div>
            </div>
            )}

            {/* Action buttons */}
            <div className="flex items-center justify-end space-x-2 pt-3 border-t border-[#ebebeb] dark:border-[#21262d]">
              <button
                type="button"
                onClick={closeSettings}
                className="px-3 py-1.5 rounded-md bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc] font-sans text-xs transition cursor-pointer"
              >
                {t('common.cancel')}
              </button>
              <button
                type="submit"
                disabled={isSaving}
                className="flex items-center space-x-1.5 px-3.5 py-1.5 rounded-md bg-[#171717] dark:bg-[#21262d] hover:bg-[#262626] dark:hover:bg-[#30363d] text-white font-sans font-medium text-xs transition cursor-pointer disabled:opacity-50 shadow-xs border border-transparent dark:border-[#30363d]"
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

      {/* Modal Dialog: "Choose models to add" */}
      {isChooseModelsModalOpen && (
        <div style={getZoomOverlayStyle(zoom)} className="fixed inset-0 z-[90] flex items-center justify-center bg-black/60 dark:bg-black/80 p-4 animate-in fade-in duration-100">
          <div className="relative flex flex-col w-[520px] max-w-full rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] shadow-md p-5 text-[#171717] dark:text-[#f0f6fc] transition-colors">
            {/* Header */}
            <div className="flex items-center justify-between pb-3 border-b border-[#ebebeb] dark:border-[#21262d]">
              <div className="flex items-center space-x-2">
                <Cpu className="h-4 w-4 text-[#171717] dark:text-[#f0f6fc]" />
                <h3 className="text-sm font-semibold text-[#171717] dark:text-[#f0f6fc] tracking-tight">{t('settings.chooseModelsTitle')}</h3>
              </div>
              <button
                onClick={() => setIsChooseModelsModalOpen(false)}
                className="p-1 rounded-md text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] transition cursor-pointer"
                aria-label={t('common.close')}
              >
                <X className="h-4 w-4" />
              </button>
            </div>

            {/* Search Input */}
            <div className="mt-4">
              <div className="relative flex items-center">
                <Search className="absolute left-2.5 h-3.5 w-3.5 text-[#8f8f8f] dark:text-[#8b949e] pointer-events-none" />
                <input
                  type="text"
                  value={searchFilter}
                  onChange={(e) => setSearchFilter(e.target.value)}
                  placeholder={t('settings.searchModels')}
                  className="w-full rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] pl-8 pr-2.5 py-1.5 text-[#171717] dark:text-[#f0f6fc] text-xs placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#0d1117] font-sans transition"
                  autoFocus
                />
              </div>
            </div>

            {/* Selection Quick Action & Count */}
            <div className="mt-2.5 flex items-center justify-between text-xs text-[#666666] dark:text-[#8b949e] px-0.5 select-none font-sans">
              <button
                type="button"
                onClick={handleToggleSelectAll}
                className="text-[#171717] dark:text-[#f0f6fc] hover:underline font-medium cursor-pointer transition"
              >
                {selectedCandidateModels.size === filteredCandidates.length && filteredCandidates.length > 0
                  ? t('settings.deselectAll')
                  : t('settings.selectAll')}
              </button>
              <span className="text-[11px] text-[#8f8f8f] dark:text-[#8b949e] tabular-nums">
                {t('settings.selectedCount', { selected: selectedCandidateModels.size, total: filteredCandidates.length })}
              </span>
            </div>

            {/* Models Checkbox List */}
            <div className="mt-2 border border-[#ebebeb] dark:border-[#30363d] rounded-md bg-white dark:bg-[#161b22] max-h-64 overflow-y-auto divide-y divide-[#ebebeb] dark:divide-[#21262d]">
              {filteredCandidates.length > 0 ? (
                filteredCandidates.map((modelName) => {
                  const isChecked = selectedCandidateModels.has(modelName);
                  const isAlreadyAdded = availableModels.includes(modelName);
                  return (
                    <div
                      key={modelName}
                      onClick={() => toggleSelectCandidate(modelName)}
                      className={`flex items-center space-x-2.5 px-3 py-2 cursor-pointer transition select-none ${
                        isChecked ? 'bg-[#f4f4f5] dark:bg-[#21262d]' : 'hover:bg-[#fafafa] dark:hover:bg-[#21262d]/50'
                      }`}
                    >
                      {isChecked ? (
                        <CheckSquare className="h-4 w-4 text-[#171717] dark:text-[#f0f6fc] shrink-0" />
                      ) : (
                        <Square className="h-4 w-4 text-[#8f8f8f] dark:text-[#8b949e] shrink-0" />
                      )}
                      <span className="font-mono text-xs text-[#171717] dark:text-[#f0f6fc] truncate flex-1">
                        {modelName}
                      </span>
                      {isAlreadyAdded && (
                        <span className="text-[10px] text-[#666666] dark:text-[#8b949e] bg-[#f4f4f5] dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] px-1.5 py-0.5 rounded-md font-sans">
                          {t('settings.alreadyAdded')}
                        </span>
                      )}
                    </div>
                  );
                })
              ) : (
                <div className="p-6 text-center text-xs text-[#8f8f8f] dark:text-[#8b949e] font-sans">
                  {t('settings.noModelsFound')}
                </div>
              )}
            </div>

            {/* Footer Buttons */}
            <div className="flex items-center justify-end space-x-2 pt-4 mt-3 border-t border-[#ebebeb] dark:border-[#21262d]">
              <button
                type="button"
                onClick={() => setIsChooseModelsModalOpen(false)}
                className="px-3 py-1.5 rounded-md bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc] font-sans text-xs transition cursor-pointer"
              >
                {t('common.cancel')}
              </button>
              <button
                type="button"
                onClick={handleAddSelectedModels}
                disabled={selectedCandidateModels.size === 0}
                className="px-3.5 py-1.5 rounded-md bg-[#171717] dark:bg-[#238636] hover:bg-[#262626] dark:hover:bg-[#2ea043] text-white font-sans font-medium text-xs transition cursor-pointer disabled:opacity-50 shadow-xs"
              >
                {t('settings.addSelected', { count: selectedCandidateModels.size })}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}

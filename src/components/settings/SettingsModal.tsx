import { useState, useId, useMemo, useRef, useEffect } from 'react';
import { useSettingsStore } from '../../stores/settingsStore';
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
  Sparkles,
} from 'lucide-react';

export default function SettingsModal() {
  const {
    isOpen,
    settings,
    availableModels,
    isSaving,
    saveStatus,
    closeSettings,
    setSettings,
    saveSettings,
    addAvailableModels,
    fetchAvailableModels,
  } = useSettingsStore();

  const [showApiKey, setShowApiKey] = useState(false);
  const [isChooseModelsModalOpen, setIsChooseModelsModalOpen] = useState(false);
  const [isFetchingModels, setIsFetchingModels] = useState(false);
  const [fetchedCandidates, setFetchedCandidates] = useState<string[]>([]);
  const [selectedCandidateModels, setSelectedCandidateModels] = useState<Set<string>>(new Set());
  const [searchFilter, setSearchFilter] = useState('');
  const [isCustomModelInput, setIsCustomModelInput] = useState(false);
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
      <div className="fixed inset-0 z-[80] flex items-center justify-center bg-black/40 backdrop-blur-xs p-4 animate-in fade-in duration-100">
        <div className="relative flex flex-col w-[500px] max-w-full rounded-lg border border-[#ebebeb] bg-white shadow-2xl p-5 text-[#171717]">
          {/* Header */}
          <div className="flex items-center justify-between pb-3 border-b border-[#ebebeb]">
            <div className="flex items-center space-x-2">
              <h2 className="text-sm font-semibold text-[#171717] tracking-tight">Настройки LLM</h2>
            </div>
            <button
              onClick={closeSettings}
              className="p-1 rounded-md text-[#666666] hover:text-[#171717] hover:bg-[#f4f4f5] transition cursor-pointer"
              title="Закрыть"
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          {/* Form Body */}
          <form onSubmit={handleSubmit} className="mt-4 space-y-3.5 text-xs font-mono">
            {/* Provider */}
            <div className="space-y-1">
              <label htmlFor={providerId} className="block text-[#4d4d4d] font-sans text-xs font-medium">
                Провайдер
              </label>
              <div className="relative">
                <select
                  id={providerId}
                  value={settings.provider}
                  onChange={(e) => setSettings({ provider: e.target.value })}
                  className="w-full rounded-md bg-[#fafafa] border border-[#ebebeb] px-2.5 py-1.5 text-[#171717] focus:outline-hidden focus:border-[#171717] focus:bg-white font-mono transition"
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
              <label htmlFor={gatewayId} className="block text-[#4d4d4d] font-sans text-xs font-medium">
                API Gateway URL
              </label>
              <div className="relative flex items-center">
                <Globe className="absolute left-2.5 h-3.5 w-3.5 text-[#8f8f8f] pointer-events-none" />
                <input
                  id={gatewayId}
                  type="text"
                  value={settings.gatewayUrl}
                  onChange={(e) => setSettings({ gatewayUrl: e.target.value, baseUrl: e.target.value })}
                  placeholder="http://127.0.0.1:port/v1"
                  className="w-full rounded-md bg-[#fafafa] border border-[#ebebeb] pl-8 pr-2.5 py-1.5 text-[#171717] placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] focus:bg-white font-mono transition"
                  required
                />
              </div>
            </div>

            {/* API Key (Masked) */}
            <div className="space-y-1">
              <label htmlFor={apiKeyId} className="block text-[#4d4d4d] font-sans text-xs font-medium">
                API Key
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
                      ? 'Не обязателен для локального прокси'
                      : 'sk-...'
                  }
                  className="w-full rounded-md bg-[#fafafa] border border-[#ebebeb] pl-8 pr-8 py-1.5 text-[#171717] placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] focus:bg-white font-mono transition"
                />
                <button
                  type="button"
                  onClick={() => setShowApiKey(!showApiKey)}
                  className="absolute right-2 p-1 text-[#666666] hover:text-[#171717] cursor-pointer"
                  title={showApiKey ? 'Скрыть ключ' : 'Показать ключ'}
                >
                  {showApiKey ? <EyeOff className="h-3.5 w-3.5" /> : <Eye className="h-3.5 w-3.5" />}
                </button>
              </div>
            </div>

            {/* Model Section */}
            <div className="space-y-1 pt-1">
              <div className="flex items-center justify-between">
                <label htmlFor={modelId} className="block text-[#4d4d4d] font-sans text-xs font-medium">
                  Модель
                </label>
                <button
                  type="button"
                  onClick={handleFetchModels}
                  disabled={isFetchingModels}
                  className="flex items-center space-x-1.5 text-[11px] font-sans font-medium text-[#171717] hover:text-[#4d4d4d] disabled:opacity-50 transition cursor-pointer px-1.5 py-0.5 rounded-md hover:bg-[#f4f4f5]"
                  title="Получить доступные модели с сервера"
                >
                  <RefreshCw className={`h-3 w-3 ${isFetchingModels ? 'animate-spin' : ''}`} />
                  <span>{isFetchingModels ? 'Получение...' : 'Получить доступные модели'}</span>
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
                    placeholder="Название модели..."
                    className="w-full rounded-md bg-[#fafafa] border border-[#ebebeb] pl-8 pr-2.5 py-1.5 text-[#171717] placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] focus:bg-white font-mono transition"
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
                    className="w-full rounded-md bg-[#fafafa] border border-[#ebebeb] pl-8 pr-2.5 py-1.5 text-[#171717] focus:outline-hidden focus:border-[#171717] focus:bg-white font-mono text-xs transition cursor-pointer"
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
                  {availableModels.length} {availableModels.length === 1 ? 'модель' : 'моделей'} в списке
                </span>
                <button
                  type="button"
                  onClick={() => setIsCustomModelInput(!isCustomModelInput)}
                  className="text-[#666666] hover:text-[#171717] underline cursor-pointer"
                >
                  {isCustomModelInput ? 'Выбрать из списка' : 'Ввести вручную'}
                </button>
              </div>
            </div>

            {/* Compactor Model Section */}
            <div className="space-y-1 pt-1 border-t border-[#f4f4f5]">
              <label htmlFor={compactorModelId} className="block text-[#4d4d4d] font-sans text-xs font-medium">
                Модель компактора контекста
              </label>
              <div className="relative flex items-center">
                <Sparkles className="absolute left-2.5 h-3.5 w-3.5 text-[#8f8f8f] pointer-events-none" />
                <select
                  id={compactorModelId}
                  value={settings.compactorModel || 'inherit'}
                  onChange={(e) => setSettings({ compactorModel: e.target.value })}
                  className="w-full rounded-md bg-[#fafafa] border border-[#ebebeb] pl-8 pr-2.5 py-1.5 text-[#171717] focus:outline-hidden focus:border-[#171717] focus:bg-white font-mono text-xs transition cursor-pointer"
                >
                  <option value="inherit">Наследовать модель задачи (Рекомендуется)</option>
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
                Модель для сжатия истории диалога и формирования структурированного отчета контекста.
              </p>
            </div>

            {/* Action buttons */}
            <div className="flex items-center justify-end space-x-2 pt-3 border-t border-[#ebebeb]">
              <button
                type="button"
                onClick={closeSettings}
                className="px-3 py-1.5 rounded-md bg-white border border-[#ebebeb] hover:bg-[#f4f4f5] text-[#171717] font-sans text-xs transition cursor-pointer"
              >
                Отмена
              </button>
              <button
                type="submit"
                disabled={isSaving}
                className="flex items-center space-x-1.5 px-3.5 py-1.5 rounded-md bg-[#171717] hover:bg-[#262626] text-white font-sans font-medium text-xs transition cursor-pointer disabled:opacity-50 shadow-xs"
              >
                {saveStatus === 'saved' ? (
                  <>
                    <Check className="h-3.5 w-3.5" />
                    <span>Сохранено</span>
                  </>
                ) : (
                  <span>Сохранить</span>
                )}
              </button>
            </div>
          </form>
        </div>
      </div>

      {/* Modal Dialog: "Choose models to add" */}
      {isChooseModelsModalOpen && (
        <div className="fixed inset-0 z-[90] flex items-center justify-center bg-black/50 backdrop-blur-xs p-4 animate-in fade-in duration-100">
          <div className="relative flex flex-col w-[520px] max-w-full rounded-lg border border-[#ebebeb] bg-white shadow-2xl p-5 text-[#171717]">
            {/* Header */}
            <div className="flex items-center justify-between pb-3 border-b border-[#ebebeb]">
              <div className="flex items-center space-x-2">
                <Sparkles className="h-4 w-4 text-[#171717]" />
                <h3 className="text-sm font-semibold text-[#171717] tracking-tight">Выбор моделей</h3>
              </div>
              <button
                onClick={() => setIsChooseModelsModalOpen(false)}
                className="p-1 rounded-md text-[#666666] hover:text-[#171717] hover:bg-[#f4f4f5] transition cursor-pointer"
                title="Закрыть"
              >
                <X className="h-4 w-4" />
              </button>
            </div>

            {/* Search Input */}
            <div className="mt-4">
              <div className="relative flex items-center">
                <Search className="absolute left-2.5 h-3.5 w-3.5 text-[#8f8f8f] pointer-events-none" />
                <input
                  type="text"
                  value={searchFilter}
                  onChange={(e) => setSearchFilter(e.target.value)}
                  placeholder="Поиск моделей..."
                  className="w-full rounded-md bg-[#fafafa] border border-[#ebebeb] pl-8 pr-2.5 py-1.5 text-[#171717] text-xs placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] focus:bg-white font-sans transition"
                  autoFocus
                />
              </div>
            </div>

            {/* Selection Quick Action & Count */}
            <div className="mt-2.5 flex items-center justify-between text-xs text-[#666666] px-0.5 select-none font-sans">
              <button
                type="button"
                onClick={handleToggleSelectAll}
                className="text-[#171717] hover:underline font-medium cursor-pointer transition"
              >
                {selectedCandidateModels.size === filteredCandidates.length && filteredCandidates.length > 0
                  ? 'Снять выбор'
                  : 'Выбрать все'}
              </button>
              <span className="text-[11px] text-[#8f8f8f] tabular-nums">
                Выбрано: {selectedCandidateModels.size} из {filteredCandidates.length}
              </span>
            </div>

            {/* Models Checkbox List */}
            <div className="mt-2 border border-[#ebebeb] rounded-md bg-white max-h-64 overflow-y-auto divide-y divide-[#ebebeb]">
              {filteredCandidates.length > 0 ? (
                filteredCandidates.map((modelName) => {
                  const isChecked = selectedCandidateModels.has(modelName);
                  const isAlreadyAdded = availableModels.includes(modelName);
                  return (
                    <div
                      key={modelName}
                      onClick={() => toggleSelectCandidate(modelName)}
                      className={`flex items-center space-x-2.5 px-3 py-2 cursor-pointer transition select-none ${
                        isChecked ? 'bg-[#f4f4f5]' : 'hover:bg-[#fafafa]'
                      }`}
                    >
                      {isChecked ? (
                        <CheckSquare className="h-4 w-4 text-[#171717] shrink-0" />
                      ) : (
                        <Square className="h-4 w-4 text-[#8f8f8f] shrink-0" />
                      )}
                      <span className="font-mono text-xs text-[#171717] truncate flex-1">
                        {modelName}
                      </span>
                      {isAlreadyAdded && (
                        <span className="text-[10px] text-[#666666] bg-[#f4f4f5] border border-[#ebebeb] px-1.5 py-0.5 rounded-md font-sans">
                          добавлена
                        </span>
                      )}
                    </div>
                  );
                })
              ) : (
                <div className="p-6 text-center text-xs text-[#8f8f8f] font-sans">
                  Модели не найдены
                </div>
              )}
            </div>

            {/* Footer Buttons */}
            <div className="flex items-center justify-end space-x-2 pt-4 mt-3 border-t border-[#ebebeb]">
              <button
                type="button"
                onClick={() => setIsChooseModelsModalOpen(false)}
                className="px-3 py-1.5 rounded-md bg-white border border-[#ebebeb] hover:bg-[#f4f4f5] text-[#171717] font-sans text-xs transition cursor-pointer"
              >
                Отмена
              </button>
              <button
                type="button"
                onClick={handleAddSelectedModels}
                disabled={selectedCandidateModels.size === 0}
                className="px-3.5 py-1.5 rounded-md bg-[#171717] hover:bg-[#262626] text-white font-sans font-medium text-xs transition cursor-pointer disabled:opacity-50 shadow-xs"
              >
                Добавить выбранные ({selectedCandidateModels.size})
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}

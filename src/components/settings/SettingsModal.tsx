import { useState, useId, useMemo } from 'react';
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

  const providerId = useId();
  const gatewayId = useId();
  const apiKeyId = useId();
  const modelId = useId();

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
    setTimeout(() => {
      closeSettings();
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
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-4 animate-in fade-in duration-100">
        <div className="relative flex flex-col w-[500px] max-w-full rounded-lg border border-obsidian-750 bg-obsidian-900 shadow-2xl p-5 text-slate-200">
          {/* Header */}
          <div className="flex items-center justify-between pb-3 border-b border-obsidian-800">
            <div className="flex items-center space-x-2">
              <h2 className="text-sm font-semibold text-white tracking-tight">Настройки LLM</h2>
            </div>
            <button
              onClick={closeSettings}
              className="p-1 rounded text-slate-400 hover:text-white hover:bg-obsidian-800 transition cursor-pointer"
              title="Закрыть"
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          {/* Form Body */}
          <form onSubmit={handleSubmit} className="mt-4 space-y-3.5 text-xs font-mono">
            {/* Provider */}
            <div className="space-y-1">
              <label htmlFor={providerId} className="block text-slate-400 font-sans text-xs">
                Провайдер
              </label>
              <div className="relative">
                <select
                  id={providerId}
                  value={settings.provider}
                  onChange={(e) => setSettings({ provider: e.target.value })}
                  className="w-full rounded bg-obsidian-950 border border-obsidian-750 px-2.5 py-1.5 text-slate-200 focus:outline-hidden focus:border-cyan-500 font-mono transition"
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
              <label htmlFor={gatewayId} className="block text-slate-400 font-sans text-xs">
                API Gateway URL
              </label>
              <div className="relative flex items-center">
                <Globe className="absolute left-2.5 h-3.5 w-3.5 text-slate-500 pointer-events-none" />
                <input
                  id={gatewayId}
                  type="text"
                  value={settings.gatewayUrl}
                  onChange={(e) => setSettings({ gatewayUrl: e.target.value, baseUrl: e.target.value })}
                  placeholder="http://127.0.0.1:port/v1"
                  className="w-full rounded bg-obsidian-950 border border-obsidian-750 pl-8 pr-2.5 py-1.5 text-slate-200 focus:outline-hidden focus:border-cyan-500 font-mono transition"
                  required
                />
              </div>
            </div>

            {/* API Key (Masked) */}
            <div className="space-y-1">
              <label htmlFor={apiKeyId} className="block text-slate-400 font-sans text-xs">
                API Key
              </label>
              <div className="relative flex items-center">
                <KeyRound className="absolute left-2.5 h-3.5 w-3.5 text-slate-500 pointer-events-none" />
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
                  className="w-full rounded bg-obsidian-950 border border-obsidian-750 pl-8 pr-8 py-1.5 text-slate-200 focus:outline-hidden focus:border-cyan-500 font-mono transition"
                />
                <button
                  type="button"
                  onClick={() => setShowApiKey(!showApiKey)}
                  className="absolute right-2 p-1 text-slate-400 hover:text-slate-200 cursor-pointer"
                  title={showApiKey ? 'Скрыть ключ' : 'Показать ключ'}
                >
                  {showApiKey ? <EyeOff className="h-3.5 w-3.5" /> : <Eye className="h-3.5 w-3.5" />}
                </button>
              </div>
            </div>

            {/* Model Section with DSH-Style Selector & Fetcher */}
            <div className="space-y-1 pt-1">
              <div className="flex items-center justify-between">
                <label htmlFor={modelId} className="block text-slate-400 font-sans text-xs">
                  Модель
                </label>
                <button
                  type="button"
                  onClick={handleFetchModels}
                  disabled={isFetchingModels}
                  className="flex items-center space-x-1.5 text-[11px] font-sans font-medium text-cyan-400 hover:text-cyan-300 disabled:opacity-50 transition cursor-pointer px-1.5 py-0.5 rounded hover:bg-obsidian-800"
                  title="Получить доступные модели с сервера"
                >
                  <RefreshCw className={`h-3 w-3 ${isFetchingModels ? 'animate-spin' : ''}`} />
                  <span>{isFetchingModels ? 'Получение...' : 'Получить доступные модели'}</span>
                </button>
              </div>

              {isCustomModelInput ? (
                <div className="relative flex items-center">
                  <Cpu className="absolute left-2.5 h-3.5 w-3.5 text-slate-500 pointer-events-none" />
                  <input
                    id={modelId}
                    type="text"
                    value={settings.model}
                    onChange={(e) => setSettings({ model: e.target.value })}
                    placeholder="Название модели..."
                    className="w-full rounded bg-obsidian-950 border border-obsidian-750 pl-8 pr-2.5 py-1.5 text-slate-200 focus:outline-hidden focus:border-cyan-500 font-mono transition"
                    required
                  />
                </div>
              ) : (
                <div className="relative flex items-center">
                  <Cpu className="absolute left-2.5 h-3.5 w-3.5 text-slate-500 pointer-events-none" />
                  <select
                    id={modelId}
                    value={settings.model}
                    onChange={(e) => setSettings({ model: e.target.value })}
                    className="w-full rounded bg-obsidian-950 border border-obsidian-750 pl-8 pr-2.5 py-1.5 text-slate-200 focus:outline-hidden focus:border-cyan-500 font-mono text-xs transition cursor-pointer"
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

              <div className="flex items-center justify-between text-[10px] text-slate-500 pt-0.5 font-sans">
                <span>
                  {availableModels.length} {availableModels.length === 1 ? 'модель' : 'моделей'} в списке
                </span>
                <button
                  type="button"
                  onClick={() => setIsCustomModelInput(!isCustomModelInput)}
                  className="text-slate-400 hover:text-cyan-400 underline cursor-pointer"
                >
                  {isCustomModelInput ? 'Выбрать из списка' : 'Ввести вручную'}
                </button>
              </div>
            </div>

            {/* Action buttons */}
            <div className="flex items-center justify-end space-x-2 pt-3 border-t border-obsidian-800">
              <button
                type="button"
                onClick={closeSettings}
                className="px-3 py-1.5 rounded bg-obsidian-800 hover:bg-obsidian-750 text-slate-300 font-sans text-xs transition cursor-pointer"
              >
                Отмена
              </button>
              <button
                type="submit"
                disabled={isSaving}
                className="flex items-center space-x-1.5 px-3 py-1.5 rounded bg-cyber-cyan hover:bg-cyan-400 text-obsidian-950 font-sans font-semibold text-xs transition cursor-pointer disabled:opacity-50"
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

      {/* Modal Dialog: "Choose models to add" (Выбор моделей) */}
      {isChooseModelsModalOpen && (
        <div className="fixed inset-0 z-60 flex items-center justify-center bg-black/80 p-4 animate-in fade-in duration-100">
          <div className="relative flex flex-col w-[520px] max-w-full rounded-lg border border-obsidian-750 bg-obsidian-900 shadow-2xl p-5 text-slate-200">
            {/* Header */}
            <div className="flex items-center justify-between pb-3 border-b border-obsidian-800">
              <div className="flex items-center space-x-2">
                <Sparkles className="h-4 w-4 text-cyan-400" />
                <h3 className="text-sm font-semibold text-white tracking-tight">Выбор моделей</h3>
              </div>
              <button
                onClick={() => setIsChooseModelsModalOpen(false)}
                className="p-1 rounded text-slate-400 hover:text-white hover:bg-obsidian-800 transition cursor-pointer"
                title="Закрыть"
              >
                <X className="h-4 w-4" />
              </button>
            </div>

            {/* Search Input */}
            <div className="mt-4">
              <div className="relative flex items-center">
                <Search className="absolute left-2.5 h-3.5 w-3.5 text-slate-500 pointer-events-none" />
                <input
                  type="text"
                  value={searchFilter}
                  onChange={(e) => setSearchFilter(e.target.value)}
                  placeholder="Поиск моделей (Search models)..."
                  className="w-full rounded bg-obsidian-950 border border-obsidian-750 pl-8 pr-2.5 py-1.5 text-slate-200 text-xs focus:outline-hidden focus:border-cyan-500 font-sans transition"
                  autoFocus
                />
              </div>
            </div>

            {/* Selection Quick Action & Count */}
            <div className="mt-2.5 flex items-center justify-between text-xs text-slate-400 px-0.5 select-none font-sans">
              <button
                type="button"
                onClick={handleToggleSelectAll}
                className="text-cyan-400 hover:text-cyan-300 font-medium cursor-pointer transition"
              >
                {selectedCandidateModels.size === filteredCandidates.length && filteredCandidates.length > 0
                  ? 'Снять выбор'
                  : 'Выбрать все'}
              </button>
              <span className="text-[11px] text-slate-500 tabular-nums">
                Выбрано: {selectedCandidateModels.size} из {filteredCandidates.length}
              </span>
            </div>

            {/* Models Checkbox List */}
            <div className="mt-2 border border-obsidian-800 rounded bg-obsidian-950 max-h-64 overflow-y-auto divide-y divide-obsidian-850">
              {filteredCandidates.length > 0 ? (
                filteredCandidates.map((modelName) => {
                  const isChecked = selectedCandidateModels.has(modelName);
                  const isAlreadyAdded = availableModels.includes(modelName);
                  return (
                    <div
                      key={modelName}
                      onClick={() => toggleSelectCandidate(modelName)}
                      className={`flex items-center space-x-2.5 px-3 py-2 cursor-pointer transition select-none ${
                        isChecked ? 'bg-cyan-950/20' : 'hover:bg-obsidian-850/60'
                      }`}
                    >
                      {isChecked ? (
                        <CheckSquare className="h-4 w-4 text-cyan-400 shrink-0" />
                      ) : (
                        <Square className="h-4 w-4 text-slate-500 shrink-0" />
                      )}
                      <span className="font-mono text-xs text-slate-200 truncate flex-1">
                        {modelName}
                      </span>
                      {isAlreadyAdded && (
                        <span className="text-[10px] text-slate-500 bg-obsidian-900 border border-obsidian-800 px-1.5 py-0.2 rounded font-sans">
                          добавлена
                        </span>
                      )}
                    </div>
                  );
                })
              ) : (
                <div className="p-6 text-center text-xs text-slate-500 font-sans">
                  Модели не найдены
                </div>
              )}
            </div>

            {/* Footer Buttons */}
            <div className="flex items-center justify-end space-x-2 pt-4 mt-3 border-t border-obsidian-800">
              <button
                type="button"
                onClick={() => setIsChooseModelsModalOpen(false)}
                className="px-3 py-1.5 rounded bg-obsidian-800 hover:bg-obsidian-750 text-slate-300 font-sans text-xs transition cursor-pointer"
              >
                Отмена
              </button>
              <button
                type="button"
                onClick={handleAddSelectedModels}
                disabled={selectedCandidateModels.size === 0}
                className="px-3.5 py-1.5 rounded bg-cyber-cyan hover:bg-cyan-400 text-obsidian-950 font-sans font-semibold text-xs transition cursor-pointer disabled:opacity-50"
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

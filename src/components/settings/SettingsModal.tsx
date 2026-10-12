import { useState, useId } from 'react';
import { useSettingsStore } from '../../stores/settingsStore';
import { X, Check, Eye, EyeOff, KeyRound, Globe, Cpu } from 'lucide-react';

export default function SettingsModal() {
  const {
    isOpen,
    settings,
    isSaving,
    saveStatus,
    closeSettings,
    setSettings,
    saveSettings,
  } = useSettingsStore();

  const [showApiKey, setShowApiKey] = useState(false);
  const providerId = useId();
  const gatewayId = useId();
  const apiKeyId = useId();
  const modelId = useId();

  if (!isOpen) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    await saveSettings();
    setTimeout(() => {
      closeSettings();
    }, 400);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-4 animate-in fade-in duration-100">
      <div className="relative flex flex-col w-[480px] max-w-full rounded-lg border border-obsidian-750 bg-obsidian-900 shadow-2xl p-5 text-slate-200">
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
                onChange={(e) => setSettings({ gatewayUrl: e.target.value })}
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
                placeholder={settings.provider === 'Antigravity Proxy' || settings.provider === 'Ollama' ? 'Не обязателен для локального прокси' : 'sk-...'}
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

          {/* Model */}
          <div className="space-y-1">
            <label htmlFor={modelId} className="block text-slate-400 font-sans text-xs">
              Модель
            </label>
            <div className="relative flex items-center">
              <Cpu className="absolute left-2.5 h-3.5 w-3.5 text-slate-500 pointer-events-none" />
              <input
                id={modelId}
                type="text"
                value={settings.model}
                onChange={(e) => setSettings({ model: e.target.value })}
                placeholder="gemini-3.8-flash-high"
                className="w-full rounded bg-obsidian-950 border border-obsidian-750 pl-8 pr-2.5 py-1.5 text-slate-200 focus:outline-hidden focus:border-cyan-500 font-mono transition"
                required
              />
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
  );
}

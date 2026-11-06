import { useEffect, useId, useState } from 'react';
import { ChevronDown, ChevronRight, Plus } from 'lucide-react';
import { useSettingsStore } from '../../../stores/settingsStore';
import { useTranslation } from '../../../stores/languageStore';
import { toProviderIdValue, type Provider, type ProviderModel, type ProviderProtocol } from '../../../stores/settings/providers';
import ProviderModelPicker from './ProviderModelPicker';
import ProviderModelRow from './ProviderModelRow';

interface ProviderEditorProps {
  provider: Provider;
  onCancel: () => void;
  onSaved: () => void;
}

const PROTOCOLS: { value: ProviderProtocol; labelKey: string }[] = [
  { value: 'openai', labelKey: 'settings.providers.protocolOpenAiChat' },
  { value: 'anthropic', labelKey: 'settings.providers.protocolAnthropic' },
  { value: 'gemini', labelKey: 'settings.providers.protocolGemini' },
  { value: 'ollama', labelKey: 'settings.providers.protocolOllama' },
  { value: 'custom', labelKey: 'settings.providers.protocolCustom' },
];

export default function ProviderEditor({ provider, onCancel, onSaved }: ProviderEditorProps) {
  const upsertProvider = useSettingsStore((s) => s.upsertProvider);
  const fetchProviderModels = useSettingsStore((s) => s.fetchProviderModels);
  const { t } = useTranslation();

  const [name, setName] = useState(provider.name);
  const [apiKey, setApiKey] = useState('');
  const [baseUrl, setBaseUrl] = useState(provider.baseUrl);
  const [protocol, setProtocol] = useState<ProviderProtocol>(provider.protocol);
  const [models, setModels] = useState<ProviderModel[]>(provider.models);
  const [advancedOpen, setAdvancedOpen] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isPickerOpen, setIsPickerOpen] = useState(false);
  const [fetchedCandidates, setFetchedCandidates] = useState<string[]>([]);
  const [isFetching, setIsFetching] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  const nameId = useId();
  const urlId = useId();
  const protocolId = useId();

  useEffect(() => {
    setName(provider.name);
    setApiKey('');
    setBaseUrl(provider.baseUrl);
    setProtocol(provider.protocol);
    setModels(provider.models);
    setError(null);
  }, [provider]);

  const handleFetch = async () => {
    setIsFetching(true);
    setError(null);
    try {
      const candidates = await fetchProviderModels(provider.id);
      setFetchedCandidates(candidates);
      setIsPickerOpen(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsFetching(false);
    }
  };

  const handleRestore = () => {
    setName(provider.name);
    setApiKey('');
    setBaseUrl(provider.baseUrl);
    setProtocol(provider.protocol);
    setModels(provider.models);
    setError(null);
  };

  const handleApply = async () => {
    const trimmedName = name.trim();
    if (!trimmedName) {
      setError(t('settings.providers.nameRequired'));
      return;
    }
    const trimmedUrl = baseUrl.trim();
    if (trimmedUrl) {
      try {
        const parsed = new URL(trimmedUrl);
        if (!['http:', 'https:'].includes(parsed.protocol)) {
          throw new Error('bad protocol');
        }
      } catch {
        setError(t('settings.providers.invalidUrl'));
        return;
      }
    }
    if (models.some((m) => !m.id.trim())) {
      setError(t('settings.providers.modelIdRequired'));
      return;
    }
    setIsSaving(true);
    setError(null);
    try {
      const next: Provider = {
        ...provider,
        id: toProviderIdValue(trimmedName),
        name: trimmedName,
        apiKey: apiKey.trim() ? apiKey.trim() : provider.apiKey,
        baseUrl: trimmedUrl,
        protocol,
        models: models.map((m) => ({
          id: m.id.trim(),
          displayName: m.displayName?.trim() || undefined,
        })),
      };
      await upsertProvider(next);
      onSaved();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <span className="text-sm font-semibold text-[#171717] dark:text-[#f4f4f5] font-sans">{name || provider.name}</span>
        {provider.isCustom && (
          <span className="rounded border border-[#ebebeb] dark:border-[#2c2c33] bg-[#fafafa] dark:bg-[#111114] px-1.5 py-0.5 text-[10px] text-[#666666] dark:text-[#71717a] font-sans">
            {t('settings.providers.custom')}
          </span>
        )}
        <span className={`h-2 w-2 rounded-full ${provider.apiKey || provider.models.length > 0 ? 'bg-[#22c55e]' : 'bg-[#ef4444]'}`} />
      </div>

      <div className="space-y-1">
        <label htmlFor="provider-api-key" className="block text-xs font-medium text-[#4d4d4d] dark:text-[#71717a] font-sans">
          {t('settings.providers.apiKey')}
        </label>
        <input
          id="provider-api-key"
          type="password"
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          placeholder={t('settings.providers.apiKeyPlaceholder')}
          className="w-full rounded-md bg-[#fafafa] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] px-2.5 py-1.5 text-xs font-mono text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#8f8f8f] dark:placeholder:text-[#71717a] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] focus:bg-white dark:focus:bg-[#111114]"
        />
      </div>

      <button
        type="button"
        onClick={() => setAdvancedOpen(!advancedOpen)}
        className="flex items-center gap-1 text-xs text-[#171717] dark:text-[#f4f4f5] font-sans cursor-pointer"
        aria-expanded={advancedOpen}
      >
        {advancedOpen ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}
        {t('settings.providers.advanced')}
      </button>

      {advancedOpen && (
        <div className="space-y-3">
          <div className="space-y-1">
            <label htmlFor={nameId} className="block text-xs font-medium text-[#4d4d4d] dark:text-[#71717a] font-sans">
              {t('settings.providers.displayName')}
            </label>
            <input
              id={nameId}
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="w-full rounded-md bg-[#fafafa] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] px-2.5 py-1.5 text-xs font-mono text-[#171717] dark:text-[#f4f4f5] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] focus:bg-white dark:focus:bg-[#111114]"
            />
          </div>

          <div className="space-y-1">
            <label htmlFor={urlId} className="block text-xs font-medium text-[#4d4d4d] dark:text-[#71717a] font-sans">
              {t('settings.providers.baseUrl')}
            </label>
            <input
              id={urlId}
              type="text"
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              placeholder="http://127.0.0.1:8045/v1"
              className="w-full rounded-md bg-[#fafafa] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] px-2.5 py-1.5 text-xs font-mono text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#8f8f8f] dark:placeholder:text-[#71717a] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] focus:bg-white dark:focus:bg-[#111114]"
            />
          </div>

          <div className="space-y-1">
            <label htmlFor={protocolId} className="block text-xs font-medium text-[#4d4d4d] dark:text-[#71717a] font-sans">
              {t('settings.providers.protocol')}
            </label>
            <select
              id={protocolId}
              value={protocol}
              onChange={(e) => setProtocol(e.target.value as ProviderProtocol)}
              className="w-full rounded-md bg-[#fafafa] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] px-2.5 py-1.5 text-xs font-mono text-[#171717] dark:text-[#f4f4f5] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] focus:bg-white dark:focus:bg-[#111114] cursor-pointer"
            >
              {PROTOCOLS.map((option) => (
                <option key={option.value} value={option.value}>
                  {t(option.labelKey as never)}
                </option>
              ))}
            </select>
          </div>

          <div className="space-y-2">
            <div className="flex items-start justify-between gap-3">
              <div>
                <div className="text-xs font-medium text-[#171717] dark:text-[#f4f4f5] font-sans">{t('settings.providers.models')}</div>
                <div className="text-[11px] text-[#666666] dark:text-[#71717a] font-sans">{t('settings.providers.modelCatalog')}</div>
              </div>
              <div className="flex items-center gap-3 shrink-0">
                <button
                  type="button"
                  onClick={handleRestore}
                  className="text-[11px] text-[#666666] dark:text-[#71717a] font-sans cursor-pointer hover:text-[#171717] dark:hover:text-[#f4f4f5]"
                >
                  {t('settings.providers.restoreDefaults')}
                </button>
                <button
                  type="button"
                  onClick={() => void handleFetch()}
                  disabled={isFetching}
                  className="text-[11px] text-[#171717] dark:text-[#f4f4f5] font-sans cursor-pointer disabled:opacity-50"
                >
                  {isFetching ? t('settings.fetching') : t('settings.providers.fetchModels')}
                </button>
              </div>
            </div>
            <div className="space-y-2">
              {models.map((model, index) => (
                <ProviderModelRow
                  key={`${index}-${model.id}`}
                  model={model}
                  index={index}
                  onChange={(i, next) => setModels((prev) => prev.map((m, idx) => (idx === i ? next : m)))}
                  onDelete={(i) => setModels((prev) => prev.filter((_, idx) => idx !== i))}
                />
              ))}
            </div>
            <button
              type="button"
              onClick={() => setModels((prev) => [...prev, { id: '' }])}
              className="flex items-center gap-1.5 rounded-md border border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] px-2.5 py-1.5 text-xs text-[#171717] dark:text-[#f4f4f5] font-sans cursor-pointer hover:bg-[#f4f4f5] dark:hover:bg-[#1e1e23]"
            >
              <Plus className="h-3.5 w-3.5" />
              {t('settings.providers.addModel')}
            </button>
          </div>
        </div>
      )}

      {error && <p className="text-xs text-red-600 dark:text-red-400 font-sans">{error}</p>}

      <div className="flex items-center justify-end gap-2 pt-3 border-t border-[#ebebeb] dark:border-[#212126]">
        <button
          type="button"
          onClick={onCancel}
          className="px-3 py-1.5 rounded-md bg-white dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] text-xs font-sans cursor-pointer hover:bg-[#f4f4f5] dark:hover:bg-[#232328]"
        >
          {t('settings.providers.cancel')}
        </button>
        <button
          type="button"
          onClick={() => void handleApply()}
          disabled={isSaving}
          className="px-3.5 py-1.5 rounded-md bg-[#171717] dark:bg-[#232328] text-white text-xs font-medium font-sans cursor-pointer disabled:opacity-50 hover:bg-[#262626] dark:hover:bg-[#2c2c33]"
        >
          {t('settings.providers.apply')}
        </button>
      </div>

      <ProviderModelPicker
        open={isPickerOpen}
        candidates={fetchedCandidates}
        existing={models}
        onClose={() => setIsPickerOpen(false)}
        onAdd={(selected) => {
          setModels((prev) => {
            const next = [...prev];
            for (const id of selected) {
              if (!next.some((m) => m.id === id)) next.push({ id });
            }
            return next;
          });
          setIsPickerOpen(false);
        }}
      />
    </div>
  );
}

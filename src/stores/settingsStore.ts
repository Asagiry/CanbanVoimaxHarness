import { create } from 'zustand';
import type { ProviderId } from '../types/domain';
import {
  availableModelIds,
  deleteProviderLocal,
  migrateFlatToProviders,
  upsertProviderLocal,
  type Provider,
} from './settings/providers';

export type { Provider, ProviderModel, ProviderProtocol } from './settings/providers';

export interface LlmSettingsState {
  provider: string;
  gatewayUrl: string;
  baseUrl?: string;
  apiKey: string;
  model: string;
  compactorModel?: string;
  providers?: Provider[];
}

export interface NotificationSettings {
  enabled: boolean;
  onAdvanceTask: boolean;
  onQuestion: boolean;
  onTaskCompleted: boolean;
}

interface SettingsStoreState {
  isOpen: boolean;
  settings: LlmSettingsState;
  notificationSettings: NotificationSettings;
  availableModels: string[];
  providers: Provider[];
  isSaving: boolean;
  saveStatus: 'idle' | 'saved' | 'error';
  errorMessage: string | null;

  openSettings: () => void;
  closeSettings: () => void;
  setSettings: (partial: Partial<LlmSettingsState>) => void;
  setNotificationSettings: (partial: Partial<NotificationSettings>) => void;
  setAvailableModels: (models: string[]) => void;
  addAvailableModels: (newModels: string[]) => void;
  setProviders: (providers: Provider[]) => void;
  upsertProvider: (provider: Provider) => Promise<Provider[]>;
  deleteProvider: (providerId: ProviderId) => Promise<Provider[]>;
  loadSettings: () => Promise<void>;
  saveSettings: (newSettings?: Partial<LlmSettingsState>) => Promise<boolean>;
  fetchAvailableModels: (baseUrl?: string, apiKey?: string) => Promise<string[]>;
  fetchProviderModels: (providerId: ProviderId) => Promise<string[]>;
}

const DEFAULT_SETTINGS: LlmSettingsState = {
  provider: 'Antigravity Proxy',
  gatewayUrl: 'http://127.0.0.1:8045/v1',
  baseUrl: 'http://127.0.0.1:8045/v1',
  apiKey: '',
  model: 'gemini-3.8-flash-high',
  compactorModel: 'inherit',
};

const DEFAULT_PROVIDERS = migrateFlatToProviders(DEFAULT_SETTINGS);

export function syncFlatFromProviders(providers: Provider[], base: LlmSettingsState): LlmSettingsState {
  // No "active provider": flat legacy fields always mirror providers[0]
  // (matching the backend's ensure_providers sync).
  const first = providers[0];
  if (!first) return base;
  const catalogIds = new Set(first.models.map((m) => m.id));
  const model = base.model && catalogIds.has(base.model)
    ? base.model
    : first.models[0]?.id || base.model || '';
  return {
    ...base,
    provider: first.name,
    gatewayUrl: first.baseUrl || base.gatewayUrl,
    baseUrl: first.baseUrl || base.baseUrl,
    apiKey: first.apiKey,
    model,
    providers,
  };
}

const DEFAULT_NOTIFICATION_SETTINGS: NotificationSettings = {
  enabled: true,
  onAdvanceTask: true,
  onQuestion: true,
  onTaskCompleted: true,
};

function loadStoredNotificationSettings(): NotificationSettings {
  try {
    const raw = localStorage.getItem('cvh_notification_settings');
    if (raw) {
      const parsed = JSON.parse(raw);
      return {
        enabled: parsed.enabled ?? DEFAULT_NOTIFICATION_SETTINGS.enabled,
        onAdvanceTask: parsed.onAdvanceTask ?? DEFAULT_NOTIFICATION_SETTINGS.onAdvanceTask,
        onQuestion: parsed.onQuestion ?? DEFAULT_NOTIFICATION_SETTINGS.onQuestion,
        onTaskCompleted: parsed.onTaskCompleted ?? DEFAULT_NOTIFICATION_SETTINGS.onTaskCompleted,
      };
    }
  } catch {
    // Ignore localStorage error
  }
  return DEFAULT_NOTIFICATION_SETTINGS;
}

const PROVIDER_PRESETS: Record<string, { gatewayUrl: string; defaultModel: string }> = {
  'OpenCode Go': {
    gatewayUrl: 'https://opencode.ai/inference/go/openai/v1',
    defaultModel: '',
  },
  'Antigravity Proxy': {
    gatewayUrl: 'http://127.0.0.1:8045/v1',
    defaultModel: 'gemini-3.8-flash-high',
  },
  'OpenAI': {
    gatewayUrl: 'https://api.openai.com/v1',
    defaultModel: 'gpt-4o',
  },
  'Anthropic': {
    gatewayUrl: 'https://api.anthropic.com/v1',
    defaultModel: 'claude-3-7-sonnet-20250219',
  },
  'Ollama': {
    gatewayUrl: 'http://127.0.0.1:11434/v1',
    defaultModel: 'qwen2.5-coder:32b',
  },
  'Custom': {
    gatewayUrl: '',
    defaultModel: '',
  },
};

export const useSettingsStore = create<SettingsStoreState>((set, get) => ({
  isOpen: false,
  settings: { ...DEFAULT_SETTINGS, providers: DEFAULT_PROVIDERS.providers },
  notificationSettings: loadStoredNotificationSettings(),
  availableModels: ['gemini-3.8-flash-high'],
  providers: DEFAULT_PROVIDERS.providers,
  isSaving: false,
  saveStatus: 'idle',
  errorMessage: null,

  openSettings: () => {
    set({ isOpen: true, saveStatus: 'idle', errorMessage: null });
    get().loadSettings();
  },

  closeSettings: () => {
    set({ isOpen: false });
  },

  setNotificationSettings: (partial) => {
    const current = get().notificationSettings;
    const next = { ...current, ...partial };
    set({ notificationSettings: next });
    try {
      localStorage.setItem('cvh_notification_settings', JSON.stringify(next));
    } catch (e) {
      console.warn('Failed to save notification settings to localStorage:', e);
    }
  },

  setAvailableModels: (models) => {
    const unique = Array.from(new Set(models));
    set({ availableModels: unique });
    try {
      localStorage.setItem('cvh_available_models', JSON.stringify(unique));
    } catch {
      // Ignore localStorage error
    }
  },

  addAvailableModels: (newModels) => {
    const current = get().availableModels;
    const combined = Array.from(new Set([...current, ...newModels]));
    set({ availableModels: combined });
    try {
      localStorage.setItem('cvh_available_models', JSON.stringify(combined));
    } catch {
      // Ignore localStorage error
    }
  },

  setProviders: (providers) => {
    const migrated = migrateFlatToProviders({ providers });
    const settings = syncFlatFromProviders(migrated.providers, get().settings);
    set({
      providers: migrated.providers,
      settings,
      availableModels: availableModelIds(migrated.providers).length > 0
        ? availableModelIds(migrated.providers)
        : get().availableModels,
    });
  },

  upsertProvider: async (provider) => {
    const next = upsertProviderLocal(get().providers, provider);
    const settings = syncFlatFromProviders(next, get().settings);
    set({ providers: next, settings, availableModels: availableModelIds(next).length > 0 ? availableModelIds(next) : get().availableModels });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const saved = await invoke<Provider[]>('upsert_provider_command', { provider });
      if (Array.isArray(saved) && saved.length > 0) {
        get().setProviders(saved);
        return get().providers;
      }
    } catch {
      // Local-first fallback; backend save happens on saveSettings
    }
    return next;
  },

  deleteProvider: async (providerId) => {
    const { providers } = deleteProviderLocal(get().providers, providerId);
    const settings = syncFlatFromProviders(providers, get().settings);
    set({ providers, settings, availableModels: availableModelIds(providers) });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const saved = await invoke<Provider[]>('delete_provider', { providerId });
      if (Array.isArray(saved) && saved.length > 0) {
        get().setProviders(saved);
        return get().providers;
      }
    } catch {
      // Local-first fallback
    }
    return providers;
  },

  setSettings: (partial) => {
    const current = get().settings;
    const next = { ...current, ...partial };

    if (partial.baseUrl && !partial.gatewayUrl) {
      next.gatewayUrl = partial.baseUrl;
    }
    if (partial.gatewayUrl && !partial.baseUrl) {
      next.baseUrl = partial.gatewayUrl;
    }

    // If provider changed, auto-suggest preset gateway and model if they match default
    if (partial.provider && partial.provider !== current.provider) {
      const preset = PROVIDER_PRESETS[partial.provider];
      if (preset) {
        if (!partial.gatewayUrl && (partial.provider === 'OpenCode Go' || current.gatewayUrl === PROVIDER_PRESETS[current.provider]?.gatewayUrl)) {
          next.gatewayUrl = preset.gatewayUrl;
          next.baseUrl = preset.gatewayUrl;
        }
        if (!partial.model && (partial.provider === 'OpenCode Go' || current.model === PROVIDER_PRESETS[current.provider]?.defaultModel)) {
          next.model = preset.defaultModel;
        }
        if (partial.provider === 'OpenCode Go') {
          next.apiKey = partial.apiKey ?? '';
          next.compactorModel = 'inherit';
        }
      }
    }

    set({ settings: next, saveStatus: 'idle' });
  },

  loadSettings: async () => {
    let loaded: LlmSettingsState | null = null;
    let storedModels: string[] = [];

    // 1. Snapshot localStorage models (used only if backend has no catalog)
    try {
      const savedModels = localStorage.getItem('cvh_available_models');
      if (savedModels) {
        const parsed = JSON.parse(savedModels);
        if (Array.isArray(parsed) && parsed.length > 0) {
          storedModels = Array.from(new Set(parsed));
        }
      }
    } catch {
      // Ignore JSON error
    }

    // 2. Try backend for settings (authoritative)
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const backendSettings = await invoke<any>('get_llm_settings');
      if (backendSettings) {
        const backendProviders = Array.isArray(backendSettings.providers) ? backendSettings.providers : [];
        const hasBackendCatalog = backendProviders.some(
          (p: Provider) => Array.isArray(p.models) && p.models.length > 0,
        );
        if (hasBackendCatalog || backendProviders.length === 0) {
          const gUrl = backendSettings.gatewayUrl || backendSettings.gateway_url || backendSettings.baseUrl || backendSettings.base_url || DEFAULT_SETTINGS.gatewayUrl;
          const flat: LlmSettingsState = {
            provider: backendSettings.provider || DEFAULT_SETTINGS.provider,
            gatewayUrl: gUrl,
            baseUrl: gUrl,
            apiKey: backendSettings.apiKey || backendSettings.api_key || '',
            model: backendSettings.model || '',
            compactorModel: backendSettings.compactorModel || backendSettings.compactor_model || 'inherit',
            providers: backendProviders.length > 0 ? backendProviders : undefined,
          };
          const migrated = migrateFlatToProviders(flat);
          loaded = syncFlatFromProviders(migrated.providers, flat);
        }
      }
    } catch {
      // Ignore Tauri invoke error in browser
    }

    // 3. Try localStorage fallback if backend didn't provide a catalog
    if (!loaded) {
      try {
        const raw = localStorage.getItem('cvh_llm_settings');
        if (raw) {
          const parsed = JSON.parse(raw);
          const gUrl = parsed.gatewayUrl || parsed.baseUrl || DEFAULT_SETTINGS.gatewayUrl;
          const flat: LlmSettingsState = {
            provider: parsed.provider || DEFAULT_SETTINGS.provider,
            gatewayUrl: gUrl,
            baseUrl: gUrl,
            apiKey: parsed.apiKey || '',
            model: parsed.model || DEFAULT_SETTINGS.model,
            compactorModel: parsed.compactorModel || 'inherit',
            providers: Array.isArray(parsed.providers) ? parsed.providers : undefined,
          };
          loaded = syncFlatFromProviders(migrateFlatToProviders(flat).providers, flat);
        }
      } catch {
        // Ignore JSON error
      }
    }

    if (loaded) {
      const providers = loaded.providers ?? get().providers;
      const derived = availableModelIds(providers);
      set({
        settings: loaded,
        providers,
        availableModels: derived.length > 0 ? derived : storedModels.length > 0 ? storedModels : get().availableModels,
        notificationSettings: loadStoredNotificationSettings(),
      });
    } else if (storedModels.length > 0) {
      set({ availableModels: storedModels, notificationSettings: loadStoredNotificationSettings() });
    } else {
      set({ notificationSettings: loadStoredNotificationSettings() });
    }
  },

  saveSettings: async (newSettings) => {
    const current = get().settings;
    const target: LlmSettingsState = newSettings ? { ...current, ...newSettings } : current;
    if (target.baseUrl && !target.gatewayUrl) target.gatewayUrl = target.baseUrl;
    if (target.gatewayUrl && !target.baseUrl) target.baseUrl = target.gatewayUrl;

    set({ isSaving: true, saveStatus: 'idle', errorMessage: null });

    // Always persist to localStorage as reliable backup
    try {
      localStorage.setItem('cvh_llm_settings', JSON.stringify(target));
      localStorage.setItem('cvh_available_models', JSON.stringify(get().availableModels));
    } catch (e) {
      console.warn('Failed to save to localStorage:', e);
    }

    // Try saving to backend
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_llm_settings', {
        settings: {
          provider: target.provider,
          gatewayUrl: target.gatewayUrl,
          baseUrl: target.gatewayUrl,
          apiKey: target.apiKey.trim(),
          model: target.model.trim(),
          compactorModel: (target.compactorModel || 'inherit').trim(),
          providers: get().providers,
        },
      });
      set({ settings: target, isSaving: false, saveStatus: 'saved' });
      return true;
    } catch (err: any) {
      console.warn('Backend save_llm_settings warning (saved in localStorage):', err);
      set({ settings: target, isSaving: false, saveStatus: 'saved' });
      return true;
    }
  },

  fetchAvailableModels: async (customBaseUrl?: string, customApiKey?: string) => {
    const settings = get().settings;
    const baseUrl = (customBaseUrl ?? settings.baseUrl ?? settings.gatewayUrl ?? '').trim();
    const apiKey = (customApiKey ?? settings.apiKey ?? '').trim();

    if (!baseUrl) {
      const errMessage = 'Base URL / Gateway URL cannot be empty';
      set({ errorMessage: errMessage });
      throw new Error(errMessage);
    }

    // 1. Try Tauri invoke: fetch_available_models({ baseUrl, apiKey })
    let tauriError: any = null;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke<any>('fetch_available_models', { baseUrl, apiKey });
      if (Array.isArray(result) && result.length > 0) {
        const models = result
          .map((item: any) => (typeof item === 'string' ? item : item.id || item.name || String(item)))
          .filter(Boolean);
        if (models.length > 0) {
          const unique = Array.from(new Set(models));
          set({ errorMessage: null });
          return unique;
        }
      }
    } catch (err) {
      tauriError = err;
      console.info('Tauri fetch_available_models fallback to client fetch:', err);
    }

    // 2. Client-side direct fetch fallback
    const cleanBase = baseUrl.replace(/\/+$/, '');
    const endpointsToTry = [
      ...(cleanBase.startsWith('https://opencode.ai/inference/go/')
        ? ['https://opencode.ai/inference/go/v1/models'] : []),
      `${cleanBase}/models`,
      cleanBase.endsWith('/v1') ? `${cleanBase}/models` : `${cleanBase}/v1/models`,
    ];

    let lastFetchError: any = null;
    for (const url of endpointsToTry) {
      try {
        const headers: Record<string, string> = {
          'Content-Type': 'application/json',
        };
        if (apiKey) {
          headers['Authorization'] = `Bearer ${apiKey}`;
        }
        const res = await fetch(url, { headers });
        if (res.ok) {
          const data = await res.json();
          const rawList = Array.isArray(data) ? data : data.data || data.models || [];
          if (Array.isArray(rawList) && rawList.length > 0) {
            const models = rawList
              .map((m: any) => (typeof m === 'string' ? m : m.id || m.name || ''))
              .filter(Boolean);
            if (models.length > 0) {
              const unique = Array.from(new Set(models));
              set({ errorMessage: null });
              return unique;
            }
          }
        } else {
          const errText = await res.text().catch(() => res.statusText);
          lastFetchError = new Error(`HTTP ${res.status}: ${errText}`);
        }
      } catch (err) {
        lastFetchError = err;
      }
    }

    // Never silently mask errors with hardcoded models!
    const errMessage = `Failed to fetch models from ${baseUrl}: ${
      lastFetchError instanceof Error
        ? lastFetchError.message
        : tauriError
        ? String(tauriError)
        : 'Network or endpoint unreachable'
    }`;
    set({ errorMessage: errMessage });
    throw new Error(errMessage);
  },

  fetchProviderModels: async (providerId) => {
    const provider = get().providers.find((p) => p.id === providerId);
    if (!provider) {
      throw new Error(`Provider not found: ${providerId}`);
    }
    return get().fetchAvailableModels(provider.baseUrl, provider.apiKey);
  },
}));

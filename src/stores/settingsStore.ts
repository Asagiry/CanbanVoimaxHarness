import { create } from 'zustand';
import type { ProviderId } from '../types/domain';
import {
  availableModelIds,
  deleteProviderLocal,
  migrateFlatToProviders,
  upsertProviderLocal,
  type Provider,
} from './settings/providers';
import {
  DEFAULT_SETTINGS,
  DEFAULT_PROVIDERS,
  PROVIDER_PRESETS,
  syncFlatFromProviders,
  loadStoredNotificationSettings,
  loadStoredAvailableModels,
  mapBackendSettingsToLlmState,
  extractModelIds,
  fetchModelsFromClientEndpoints,
  type LlmSettingsState,
  type NotificationSettings,
  type BackendLlmSettingsDTO,
  type ModelCandidateItem,
} from './settings/settingsHelpers';

export type { Provider, ProviderModel, ProviderProtocol } from './settings/providers';
export type { LlmSettingsState, NotificationSettings } from './settings/settingsHelpers';
export { syncFlatFromProviders } from './settings/settingsHelpers';

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

export const useSettingsStore = create<SettingsStoreState>((set, get) => ({
  isOpen: false,
  settings: { ...DEFAULT_SETTINGS, providers: DEFAULT_PROVIDERS.providers },
  notificationSettings: loadStoredNotificationSettings(),
  availableModels: [],
  providers: DEFAULT_PROVIDERS.providers,
  isSaving: false,
  saveStatus: 'idle',
  errorMessage: null,

  openSettings: () => {
    set({ isOpen: true, saveStatus: 'idle', errorMessage: null });
    void get().loadSettings();
  },

  closeSettings: () => {
    set({ isOpen: false });
  },

  setNotificationSettings: (partial) => {
    const next = { ...get().notificationSettings, ...partial };
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
    } catch (err) {
      console.debug('Failed to persist cvh_available_models:', err);
    }
  },

  addAvailableModels: (newModels) => {
    const combined = Array.from(new Set([...get().availableModels, ...newModels]));
    set({ availableModels: combined });
    try {
      localStorage.setItem('cvh_available_models', JSON.stringify(combined));
    } catch (err) {
      console.debug('Failed to persist cvh_available_models:', err);
    }
  },

  setProviders: (providers) => {
    const migrated = migrateFlatToProviders({ providers });
    const settings = syncFlatFromProviders(migrated.providers, get().settings);
    const derived = availableModelIds(migrated.providers);
    set({
      providers: migrated.providers,
      settings,
      availableModels: derived.length > 0 ? derived : get().availableModels,
    });
  },

  upsertProvider: async (provider) => {
    const next = upsertProviderLocal(get().providers, provider);
    const settings = syncFlatFromProviders(next, get().settings);
    const derived = availableModelIds(next);
    set({
      providers: next,
      settings,
      availableModels: derived.length > 0 ? derived : get().availableModels,
      errorMessage: null,
    });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const saved = await invoke<Provider[]>('upsert_provider_command', { provider });
      if (Array.isArray(saved) && saved.length > 0) {
        get().setProviders(saved);
        return get().providers;
      }
    } catch (err) {
      const errMsg = err instanceof Error ? err.message : String(err);
      console.warn('upsert_provider_command failed:', err);
      set({ errorMessage: errMsg });
    }
    return next;
  },

  deleteProvider: async (providerId) => {
    const { providers } = deleteProviderLocal(get().providers, providerId);
    const settings = syncFlatFromProviders(providers, get().settings);
    set({
      providers,
      settings,
      availableModels: availableModelIds(providers),
      errorMessage: null,
    });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const saved = await invoke<Provider[]>('delete_provider', { providerId });
      if (Array.isArray(saved) && saved.length > 0) {
        get().setProviders(saved);
        return get().providers;
      }
    } catch (err) {
      const errMsg = err instanceof Error ? err.message : String(err);
      console.warn('delete_provider failed:', err);
      set({ errorMessage: errMsg });
    }
    return providers;
  },

  setSettings: (partial) => {
    const current = get().settings;
    const next = { ...current, ...partial };
    if (partial.baseUrl && !partial.gatewayUrl) next.gatewayUrl = partial.baseUrl;
    if (partial.gatewayUrl && !partial.baseUrl) next.baseUrl = partial.gatewayUrl;

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

    set({ settings: next, saveStatus: 'idle', errorMessage: null });
  },

  loadSettings: async () => {
    let loaded: LlmSettingsState | null = null;
    const storedModels = loadStoredAvailableModels();

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const backendSettings = await invoke<BackendLlmSettingsDTO | null>('get_llm_settings');
      if (backendSettings) {
        loaded = mapBackendSettingsToLlmState(backendSettings);
      }
    } catch (err) {
      console.warn('get_llm_settings IPC unavailable or failed:', err);
    }

    if (!loaded) {
      try {
        const raw = localStorage.getItem('cvh_llm_settings');
        if (raw) {
          const parsed = JSON.parse(raw) as BackendLlmSettingsDTO;
          loaded = mapBackendSettingsToLlmState(parsed);
        }
      } catch (err) {
        console.debug('Failed to parse cvh_llm_settings from localStorage:', err);
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
    const target: LlmSettingsState = newSettings ? { ...current, ...newSettings } : { ...current };
    if (target.baseUrl && !target.gatewayUrl) target.gatewayUrl = target.baseUrl;
    if (target.gatewayUrl && !target.baseUrl) target.baseUrl = target.gatewayUrl;

    set({ isSaving: true, saveStatus: 'idle', errorMessage: null });

    try {
      localStorage.setItem('cvh_llm_settings', JSON.stringify(target));
      localStorage.setItem('cvh_available_models', JSON.stringify(get().availableModels));
    } catch (e) {
      console.warn('Failed to save to localStorage:', e);
    }

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
      set({ settings: target, isSaving: false, saveStatus: 'saved', errorMessage: null });
      return true;
    } catch (err) {
      const errMsg = err instanceof Error ? err.message : String(err);
      console.error('Backend save_llm_settings failed:', err);
      set({ isSaving: false, saveStatus: 'error', errorMessage: errMsg });
      return false;
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

    let tauriError: unknown = null;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke<ModelCandidateItem[]>('fetch_available_models', { baseUrl, apiKey });
      const unique = extractModelIds(result);
      if (unique.length > 0) {
        set({ errorMessage: null });
        return unique;
      }
    } catch (err) {
      tauriError = err;
      console.info('Tauri fetch_available_models fallback to client fetch:', err);
    }

    const { models, lastError } = await fetchModelsFromClientEndpoints(baseUrl, apiKey);
    if (models && models.length > 0) {
      set({ errorMessage: null });
      return models;
    }

    const detail = lastError instanceof Error
      ? lastError.message
      : tauriError instanceof Error
        ? tauriError.message
        : tauriError
          ? String(tauriError)
          : 'Network or endpoint unreachable';
    const errMessage = `Failed to fetch models from ${baseUrl}: ${detail}`;
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

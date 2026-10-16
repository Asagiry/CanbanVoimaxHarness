import { create } from 'zustand';

export interface LlmSettingsState {
  provider: string;
  gatewayUrl: string;
  baseUrl?: string;
  apiKey: string;
  model: string;
  compactorModel?: string;
}

interface SettingsStoreState {
  isOpen: boolean;
  settings: LlmSettingsState;
  availableModels: string[];
  isSaving: boolean;
  saveStatus: 'idle' | 'saved' | 'error';
  errorMessage: string | null;

  openSettings: () => void;
  closeSettings: () => void;
  setSettings: (partial: Partial<LlmSettingsState>) => void;
  setAvailableModels: (models: string[]) => void;
  addAvailableModels: (newModels: string[]) => void;
  loadSettings: () => Promise<void>;
  saveSettings: (newSettings?: Partial<LlmSettingsState>) => Promise<boolean>;
  fetchAvailableModels: (baseUrl?: string, apiKey?: string) => Promise<string[]>;
}

const DEFAULT_SETTINGS: LlmSettingsState = {
  provider: 'Antigravity Proxy',
  gatewayUrl: 'http://127.0.0.1:8045/v1',
  baseUrl: 'http://127.0.0.1:8045/v1',
  apiKey: '',
  model: 'gemini-3.8-flash-high',
  compactorModel: 'inherit',
};

const PROVIDER_PRESETS: Record<string, { gatewayUrl: string; defaultModel: string }> = {
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
  settings: DEFAULT_SETTINGS,
  availableModels: ['gemini-3.8-flash-high'],
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
        if (!partial.gatewayUrl && current.gatewayUrl === PROVIDER_PRESETS[current.provider]?.gatewayUrl) {
          next.gatewayUrl = preset.gatewayUrl;
          next.baseUrl = preset.gatewayUrl;
        }
        if (!partial.model && current.model === PROVIDER_PRESETS[current.provider]?.defaultModel) {
          next.model = preset.defaultModel;
        }
      }
    }

    set({ settings: next, saveStatus: 'idle' });
  },

  loadSettings: async () => {
    let loaded: LlmSettingsState | null = null;

    // 1. Load models from localStorage
    try {
      const savedModels = localStorage.getItem('cvh_available_models');
      if (savedModels) {
        const parsed = JSON.parse(savedModels);
        if (Array.isArray(parsed) && parsed.length > 0) {
          set({ availableModels: Array.from(new Set(parsed)) });
        }
      }
    } catch {
      // Ignore JSON error
    }

    // 2. Try backend for settings
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const backendSettings = await invoke<any>('get_llm_settings');
      if (backendSettings) {
        const gUrl = backendSettings.gatewayUrl || backendSettings.gateway_url || backendSettings.baseUrl || backendSettings.base_url || DEFAULT_SETTINGS.gatewayUrl;
        loaded = {
          provider: backendSettings.provider || DEFAULT_SETTINGS.provider,
          gatewayUrl: gUrl,
          baseUrl: gUrl,
          apiKey: backendSettings.apiKey || backendSettings.api_key || '',
          model: backendSettings.model || DEFAULT_SETTINGS.model,
          compactorModel: backendSettings.compactorModel || backendSettings.compactor_model || 'inherit',
        };
      }
    } catch {
      // Ignore Tauri invoke error in browser
    }

    // 3. Try localStorage fallback if backend didn't provide
    if (!loaded) {
      try {
        const raw = localStorage.getItem('cvh_llm_settings');
        if (raw) {
          const parsed = JSON.parse(raw);
          const gUrl = parsed.gatewayUrl || parsed.baseUrl || DEFAULT_SETTINGS.gatewayUrl;
          loaded = {
            provider: parsed.provider || DEFAULT_SETTINGS.provider,
            gatewayUrl: gUrl,
            baseUrl: gUrl,
            apiKey: parsed.apiKey || '',
            model: parsed.model || DEFAULT_SETTINGS.model,
            compactorModel: parsed.compactorModel || 'inherit',
          };
        }
      } catch {
        // Ignore JSON error
      }
    }

    if (loaded) {
      set({ settings: loaded });
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
          set({ availableModels: unique, errorMessage: null });
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
              set({ availableModels: unique, errorMessage: null });
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
}));

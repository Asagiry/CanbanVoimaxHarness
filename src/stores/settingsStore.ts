import { create } from 'zustand';

export interface LlmSettingsState {
  provider: string;
  gatewayUrl: string;
  apiKey: string;
  model: string;
}

interface SettingsStoreState {
  isOpen: boolean;
  settings: LlmSettingsState;
  isSaving: boolean;
  saveStatus: 'idle' | 'saved' | 'error';
  errorMessage: string | null;

  openSettings: () => void;
  closeSettings: () => void;
  setSettings: (partial: Partial<LlmSettingsState>) => void;
  loadSettings: () => Promise<void>;
  saveSettings: (newSettings?: Partial<LlmSettingsState>) => Promise<boolean>;
}

const DEFAULT_SETTINGS: LlmSettingsState = {
  provider: 'Antigravity Proxy',
  gatewayUrl: 'http://127.0.0.1:19387/api/ai',
  apiKey: '',
  model: 'gemini-3.8-flash-high',
};

const PROVIDER_PRESETS: Record<string, { gatewayUrl: string; defaultModel: string }> = {
  'Antigravity Proxy': {
    gatewayUrl: 'http://127.0.0.1:19387/api/ai',
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
};

export const useSettingsStore = create<SettingsStoreState>((set, get) => ({
  isOpen: false,
  settings: DEFAULT_SETTINGS,
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

  setSettings: (partial) => {
    const current = get().settings;
    const next = { ...current, ...partial };

    // If provider changed, auto-suggest preset gateway and model if they match default
    if (partial.provider && partial.provider !== current.provider) {
      const preset = PROVIDER_PRESETS[partial.provider];
      if (preset) {
        if (!partial.gatewayUrl && current.gatewayUrl === PROVIDER_PRESETS[current.provider]?.gatewayUrl) {
          next.gatewayUrl = preset.gatewayUrl;
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

    // 1. Try backend
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const backendSettings = await invoke<any>('get_llm_settings');
      if (backendSettings) {
        loaded = {
          provider: backendSettings.provider || DEFAULT_SETTINGS.provider,
          gatewayUrl: backendSettings.gatewayUrl || backendSettings.gateway_url || DEFAULT_SETTINGS.gatewayUrl,
          apiKey: backendSettings.apiKey || backendSettings.api_key || '',
          model: backendSettings.model || DEFAULT_SETTINGS.model,
        };
      }
    } catch {
      // Ignore Tauri invoke error in browser
    }

    // 2. Try localStorage fallback if backend didn't provide
    if (!loaded) {
      try {
        const raw = localStorage.getItem('cvh_llm_settings');
        if (raw) {
          const parsed = JSON.parse(raw);
          loaded = {
            provider: parsed.provider || DEFAULT_SETTINGS.provider,
            gatewayUrl: parsed.gatewayUrl || DEFAULT_SETTINGS.gatewayUrl,
            apiKey: parsed.apiKey || '',
            model: parsed.model || DEFAULT_SETTINGS.model,
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

    set({ isSaving: true, saveStatus: 'idle', errorMessage: null });

    // Always persist to localStorage as reliable backup
    try {
      localStorage.setItem('cvh_llm_settings', JSON.stringify(target));
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
          apiKey: target.apiKey.trim() || null,
          model: target.model.trim(),
        },
      });
      set({ settings: target, isSaving: false, saveStatus: 'saved' });
      return true;
    } catch (err: any) {
      console.warn('Backend save_llm_settings warning (saved in localStorage):', err);
      // Still consider saved because localStorage backup succeeded
      set({ settings: target, isSaving: false, saveStatus: 'saved' });
      return true;
    }
  },
}));

import {
  migrateFlatToProviders,
  type Provider,
} from './providers';

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

export interface BackendLlmSettingsDTO {
  provider?: string;
  gatewayUrl?: string;
  gateway_url?: string;
  baseUrl?: string;
  base_url?: string;
  apiKey?: string;
  api_key?: string;
  model?: string;
  compactorModel?: string;
  compactor_model?: string;
  providers?: Provider[];
}

export type ModelCandidateItem = string | { id?: string; name?: string };

export const DEFAULT_SETTINGS: LlmSettingsState = {
  provider: 'OpenCode Go',
  gatewayUrl: 'https://opencode.ai/inference/go/openai/v1',
  baseUrl: 'https://opencode.ai/inference/go/openai/v1',
  apiKey: '',
  model: '',
  compactorModel: 'inherit',
};

export const DEFAULT_PROVIDERS = migrateFlatToProviders(DEFAULT_SETTINGS);

export const DEFAULT_NOTIFICATION_SETTINGS: NotificationSettings = {
  enabled: true,
  onAdvanceTask: true,
  onQuestion: true,
  onTaskCompleted: true,
};

export const PROVIDER_PRESETS: Record<string, { gatewayUrl: string; defaultModel: string }> = {
  'OpenCode Go': {
    gatewayUrl: 'https://opencode.ai/inference/go/openai/v1',
    defaultModel: '',
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

export function syncFlatFromProviders(providers: Provider[], base: LlmSettingsState): LlmSettingsState {
  // Flat legacy fields mirror providers[0] (matching backend ensure_providers sync).
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

export function loadStoredNotificationSettings(): NotificationSettings {
  if (typeof localStorage === 'undefined') return DEFAULT_NOTIFICATION_SETTINGS;
  try {
    const raw = localStorage.getItem('cvh_notification_settings');
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<NotificationSettings>;
      return {
        enabled: parsed.enabled ?? DEFAULT_NOTIFICATION_SETTINGS.enabled,
        onAdvanceTask: parsed.onAdvanceTask ?? DEFAULT_NOTIFICATION_SETTINGS.onAdvanceTask,
        onQuestion: parsed.onQuestion ?? DEFAULT_NOTIFICATION_SETTINGS.onQuestion,
        onTaskCompleted: parsed.onTaskCompleted ?? DEFAULT_NOTIFICATION_SETTINGS.onTaskCompleted,
      };
    }
  } catch (err) {
    console.debug('Failed to read cvh_notification_settings from localStorage:', err);
  }
  return DEFAULT_NOTIFICATION_SETTINGS;
}

export function loadStoredAvailableModels(): string[] {
  if (typeof localStorage === 'undefined') return [];
  try {
    const savedModels = localStorage.getItem('cvh_available_models');
    if (savedModels) {
      const parsed: unknown = JSON.parse(savedModels);
      if (Array.isArray(parsed) && parsed.length > 0) {
        return Array.from(new Set(parsed.filter((item): item is string => typeof item === 'string' && item.length > 0)));
      }
    }
  } catch (err) {
    console.debug('Failed to parse cvh_available_models from localStorage:', err);
  }
  return [];
}

export function mapBackendSettingsToLlmState(backendSettings: BackendLlmSettingsDTO): LlmSettingsState {
  const backendProviders = Array.isArray(backendSettings.providers) ? backendSettings.providers : [];
  const gUrl =
    backendSettings.gatewayUrl ||
    backendSettings.gateway_url ||
    backendSettings.baseUrl ||
    backendSettings.base_url ||
    DEFAULT_SETTINGS.gatewayUrl;
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
  return syncFlatFromProviders(migrated.providers, flat);
}

export function extractModelIds(rawList: unknown): string[] {
  if (!Array.isArray(rawList) || rawList.length === 0) return [];
  const ids = rawList
    .map((item) => {
      if (typeof item === 'string') return item;
      if (item && typeof item === 'object') {
        const obj = item as { id?: unknown; name?: unknown };
        if (typeof obj.id === 'string') return obj.id;
        if (typeof obj.name === 'string') return obj.name;
      }
      return '';
    })
    .filter(Boolean);
  return Array.from(new Set(ids));
}

export async function fetchModelsFromClientEndpoints(
  baseUrl: string,
  apiKey: string,
): Promise<{ models: string[] | null; lastError: unknown }> {
  const cleanBase = baseUrl.replace(/\/+$/, '');
  const endpointsToTry = [
    ...(cleanBase.startsWith('https://opencode.ai/inference/go/')
      ? ['https://opencode.ai/inference/go/v1/models']
      : []),
    `${cleanBase}/models`,
    cleanBase.endsWith('/v1') ? `${cleanBase}/models` : `${cleanBase}/v1/models`,
  ];

  let lastError: unknown = null;
  for (const url of endpointsToTry) {
    try {
      const headers: Record<string, string> = { 'Content-Type': 'application/json' };
      if (apiKey) headers['Authorization'] = `Bearer ${apiKey}`;
      const res = await fetch(url, { headers });
      if (res.ok) {
        const data = (await res.json()) as unknown;
        const rawList = Array.isArray(data)
          ? data
          : (data as { data?: unknown; models?: unknown })?.data ||
            (data as { data?: unknown; models?: unknown })?.models ||
            [];
        const unique = extractModelIds(rawList);
        if (unique.length > 0) {
          return { models: unique, lastError: null };
        }
      } else {
        let errText = res.statusText;
        try {
          errText = await res.text();
        } catch (readErr) {
          console.debug('Failed to read error body from models endpoint:', readErr);
        }
        lastError = new Error(`HTTP ${res.status}: ${errText}`);
      }
    } catch (err) {
      lastError = err;
    }
  }
  return { models: null, lastError };
}

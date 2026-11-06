import type { ProviderId } from '../../types/domain';

export type ProviderProtocol = 'openai' | 'anthropic' | 'gemini' | 'ollama' | 'custom';

export interface ProviderModel {
  id: string;
  displayName?: string;
}

export interface Provider {
  id: ProviderId;
  name: string;
  isCustom: boolean;
  apiKey: string;
  baseUrl: string;
  protocol: ProviderProtocol;
  models: ProviderModel[];
}

export interface FlatLlmSettingsShape {
  provider?: string;
  gatewayUrl?: string;
  baseUrl?: string;
  apiKey?: string;
  model?: string;
  compactorModel?: string;
  activeProviderId?: string;
  providers?: Provider[];
}

export function toProviderIdValue(raw: string): ProviderId {
  const normalized = raw.trim().toLowerCase();
  if (!normalized) {
    throw new Error('Invalid ProviderId: cannot be empty');
  }
  return normalized as ProviderId;
}

export function validateProvider(provider: Provider): void {
  if (!provider.id || !String(provider.id).trim()) {
    throw new Error('Provider id cannot be empty');
  }
  if (!provider.name.trim()) {
    throw new Error('Provider name cannot be empty');
  }
  const baseUrl = provider.baseUrl.trim();
  if (baseUrl) {
    try {
      const parsed = new URL(baseUrl);
      if (!['http:', 'https:'].includes(parsed.protocol)) {
        throw new Error('Invalid base URL');
      }
    } catch {
      throw new Error('Invalid base URL');
    }
  }
  for (const model of provider.models) {
    if (!model.id.trim()) {
      throw new Error('Model id cannot be empty');
    }
  }
}

export function normalizeProviders(providers: Provider[]): Provider[] {
  const result: Provider[] = [];
  for (const raw of providers) {
    const id = String(raw.id ?? '').trim().toLowerCase();
    const name = String(raw.name ?? '').trim();
    if (!id || !name) continue;
    const seen = new Set<string>();
    const models: ProviderModel[] = [];
    for (const model of raw.models ?? []) {
      const modelId = String(model.id ?? '').trim();
      if (!modelId || seen.has(modelId)) continue;
      seen.add(modelId);
      const displayName = model.displayName?.trim();
      models.push(displayName ? { id: modelId, displayName } : { id: modelId });
    }
    const provider: Provider = {
      id: id as ProviderId,
      name,
      isCustom: Boolean(raw.isCustom),
      apiKey: raw.apiKey ?? '',
      baseUrl: (raw.baseUrl ?? '').trim(),
      protocol: raw.protocol ?? 'openai',
      models,
    };
    const existing = result.find((p) => p.id === provider.id);
    if (existing) {
      for (const model of models) {
        if (!existing.models.some((m) => m.id === model.id)) {
          existing.models.push(model);
        }
      }
      continue;
    }
    result.push(provider);
  }
  return result;
}

const BUILTIN_PROVIDER_NAMES = new Set([
  'Antigravity Proxy',
  'OpenCode Go',
  'OpenAI',
  'Anthropic',
  'Ollama',
  'DeepSeek',
]);

export function legacyProviderFromFlat(flat: FlatLlmSettingsShape): Provider {
  const name = (flat.provider ?? '').trim() || 'Default';
  const baseUrl = (flat.baseUrl ?? flat.gatewayUrl ?? '').trim();
  const model = (flat.model ?? '').trim();
  return {
    id: toProviderIdValue(name),
    name,
    isCustom: !BUILTIN_PROVIDER_NAMES.has(name),
    apiKey: flat.apiKey ?? '',
    baseUrl,
    protocol: 'openai',
    models: model ? [{ id: model }] : [],
  };
}

export function migrateFlatToProviders(flat: FlatLlmSettingsShape): { providers: Provider[]; activeProviderId: ProviderId } {
  const rawProviders = Array.isArray(flat.providers) && flat.providers.length > 0
    ? flat.providers
    : [legacyProviderFromFlat(flat)];
  const providers = normalizeProviders(rawProviders);
  const fallback: Provider = providers[0] ?? legacyProviderFromFlat(flat);
  const requested = (flat.activeProviderId ?? '').trim().toLowerCase();
  const activeProviderId = (providers.find((p) => p.id === requested)?.id ?? fallback.id) as ProviderId;
  return { providers, activeProviderId };
}

export function availableModelIds(providers: Provider[]): string[] {
  const seen = new Set<string>();
  for (const provider of providers) {
    for (const model of provider.models) {
      if (model.id) seen.add(model.id);
    }
  }
  return Array.from(seen);
}

export function providerModelLabel(model: ProviderModel): string {
  return model.displayName ?? model.id;
}

export interface CatalogModelOption {
  id: string;
  label: string;
  providerName?: string;
}

export function catalogModelOptions(providers: Provider[]): CatalogModelOption[] {
  const seen = new Map<string, CatalogModelOption>();
  for (const provider of providers) {
    for (const model of provider.models) {
      if (!model.id) continue;
      if (!seen.has(model.id)) {
        seen.set(model.id, {
          id: model.id,
          label: providerModelLabel(model),
          providerName: provider.name,
        });
      }
    }
  }
  return Array.from(seen.values());
}

export function catalogModelLabel(providers: Provider[], modelId: string): string {
  for (const provider of providers) {
    const found = provider.models.find((m) => m.id === modelId);
    if (found) return providerModelLabel(found);
  }
  return modelId;
}

export function upsertProviderLocal(providers: Provider[], provider: Provider): Provider[] {
  validateProvider(provider);
  const normalized = normalizeProviders([provider])[0];
  const index = providers.findIndex((p) => p.id === normalized.id);
  if (index >= 0) {
    const next = [...providers];
    next[index] = normalized;
    return next;
  }
  return [...providers, normalized];
}

export function deleteProviderLocal(
  providers: Provider[],
  activeProviderId: ProviderId,
  providerId: ProviderId,
): { providers: Provider[]; activeProviderId: ProviderId } {
  if (providers.length <= 1) {
    throw new Error('Cannot delete the last provider');
  }
  const next = providers.filter((p) => p.id !== providerId);
  // Deleting the first/builtin provider is allowed when others remain.
  // Active falls back to the first remaining provider (never to the
  // deleted one).
  const fallback = next[0];
  const active = next.some((p) => p.id === activeProviderId)
    ? activeProviderId
    : (fallback.id as ProviderId);
  return { providers: next, activeProviderId: active };
}

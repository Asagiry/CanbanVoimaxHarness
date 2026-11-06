import { describe, expect, it } from 'vitest';
import {
  availableModelIds,
  deleteProviderLocal,
  legacyProviderFromFlat,
  migrateFlatToProviders,
  normalizeProviders,
  providerModelLabel,
  upsertProviderLocal,
  validateProvider,
  type Provider,
} from '../settings/providers';
import { syncFlatFromProviders } from '../settingsStore';
import { toProviderId } from '../../types/domain';
const toProviderIdValue = toProviderId;

describe('providers domain helpers', () => {
  it('migrates flat LlmSettings to a single provider', () => {
    const migrated = migrateFlatToProviders({
      provider: 'Antigravity Proxy',
      baseUrl: 'http://127.0.0.1:8045/v1',
      apiKey: 'sk-1',
      model: 'gemini-3.8-flash-high',
    });
    expect(migrated.providers).toHaveLength(1);
    expect(migrated.providers[0].apiKey).toBe('sk-1');
    expect(migrated.providers[0].models[0].id).toBe('gemini-3.8-flash-high');
    expect(migrated.activeProviderId).toBe(toProviderIdValue('antigravity proxy'));
  });

  it('keeps providers when backend returns them', () => {
    const provider: Provider = {
      id: toProviderIdValue('tg'),
      name: 'tg',
      isCustom: true,
      apiKey: 'sk-2',
      baseUrl: 'http://127.0.0.1:8045/v1',
      protocol: 'openai',
      models: [{ id: 'm1', displayName: 'M One' }],
    };
    const migrated = migrateFlatToProviders({
      provider: 'tg',
      providers: [provider],
      activeProviderId: 'tg',
    });
    expect(migrated.providers).toHaveLength(1);
    expect(providerModelLabel(migrated.providers[0].models[0])).toBe('M One');
    expect(availableModelIds(migrated.providers)).toEqual(['m1']);
  });

  it('normalizes duplicate models and providers', () => {
    const normalized = normalizeProviders([
      { id: toProviderIdValue('tg'), name: 'tg', isCustom: true, apiKey: '', baseUrl: '', protocol: 'openai', models: [{ id: 'm1' }, { id: 'm1' }] },
      { id: toProviderIdValue('tg'), name: 'tg', isCustom: true, apiKey: '', baseUrl: '', protocol: 'openai', models: [{ id: 'm2' }] },
    ]);
    expect(normalized).toHaveLength(1);
    expect(normalized[0].models.map((m) => m.id)).toEqual(['m1', 'm2']);
  });

  it('validates provider boundary', () => {
    const provider: Provider = {
      id: toProviderIdValue('tg'),
      name: 'tg',
      isCustom: true,
      apiKey: '',
      baseUrl: 'not-a-url',
      protocol: 'openai',
      models: [],
    };
    expect(() => validateProvider(provider)).toThrow('Invalid base URL');
    expect(() => validateProvider({ ...provider, baseUrl: 'http://x/v1', models: [{ id: ' ' }] })).toThrow('Model id cannot be empty');
  });

  it('upserts and deletes providers locally with fallback', () => {
    const a: Provider = { id: toProviderIdValue('a'), name: 'A', isCustom: false, apiKey: '', baseUrl: '', protocol: 'openai', models: [] };
    const b: Provider = { id: toProviderIdValue('b'), name: 'B', isCustom: true, apiKey: '', baseUrl: '', protocol: 'openai', models: [] };
    const next = upsertProviderLocal([a], b);
    expect(next.map((p) => p.id)).toEqual(['a', 'b']);
    const replaced = upsertProviderLocal(next, { ...a, name: 'A2' });
    expect(replaced[0].name).toBe('A2');
    const deleted = deleteProviderLocal(next, toProviderIdValue('b'), toProviderIdValue('b'));
    expect(deleted.providers.map((p) => p.id)).toEqual(['a']);
    expect(deleted.activeProviderId).toBe(toProviderIdValue('a'));
  });

  it('syncFlatFromProviders keeps catalog model and falls back when stale', () => {
    const providers: Provider[] = [{
      id: toProviderIdValue('opencode go'),
      name: 'OpenCode Go',
      isCustom: false,
      apiKey: '',
      baseUrl: 'https://opencode.ai/inference/go/openai/v1',
      protocol: 'openai',
      models: [{ id: 'beam-free' }, { id: 'crouton-alpha-free' }],
    }];
    const base = { provider: 'OpenCode Go', gatewayUrl: '', apiKey: '', model: 'crouton-alpha-free' };
    expect(syncFlatFromProviders(providers, toProviderIdValue('opencode go'), base).model).toBe('crouton-alpha-free');
    expect(syncFlatFromProviders(providers, toProviderIdValue('opencode go'), { ...base, model: '' }).model).toBe('beam-free');
    expect(syncFlatFromProviders(providers, toProviderIdValue('opencode go'), { ...base, model: 'gemini-3.8-flash-high' }).model).toBe('beam-free');
  });

  it('legacy provider marks custom names', () => {
    const custom = legacyProviderFromFlat({ provider: 'tg', baseUrl: 'http://x/v1', model: '' });
    expect(custom.isCustom).toBe(true);
    expect(custom.models).toEqual([]);
  });
});

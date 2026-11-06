import { describe, expect, it } from 'vitest';
import { normalizeProviders, providerModelLabel, type Provider } from '../../../../stores/settings/providers';
import { toProviderId } from '../../../../types/domain';

function isNewCustomProvider(provider: Provider): boolean {
  return provider.isCustom && !provider.apiKey && provider.models.length === 0;
}

describe('ProviderList rendering helpers', () => {
  it('normalizes provider cards for display', () => {
    const providers = normalizeProviders([
      {
        id: toProviderId('tg'),
        name: 'tg',
        isCustom: true,
        apiKey: 'sk-1',
        baseUrl: 'http://127.0.0.1:8045/v1',
        protocol: 'openai',
        models: [{ id: 'm1', displayName: 'M One' }],
      },
      {
        id: toProviderId('deepseek'),
        name: 'DeepSeek',
        isCustom: false,
        apiKey: '',
        baseUrl: '',
        protocol: 'openai',
        models: [],
      },
    ]);
    expect(providers).toHaveLength(2);
    expect(providers[0].isCustom).toBe(true);
    expect(providerModelLabel(providers[0].models[0])).toBe('M One');
  });

  it('treats an empty custom provider as new (advanced collapsed by default)', () => {
    const fresh: Provider = {
      id: toProviderId('custom-1'),
      name: 'custom',
      isCustom: true,
      apiKey: '',
      baseUrl: '',
      protocol: 'openai',
      models: [],
    };
    expect(isNewCustomProvider(fresh)).toBe(true);
    expect(!isNewCustomProvider(fresh)).toBe(false);
  });

  it('treats an existing provider with models as not new (advanced open by default)', () => {
    const existing: Provider = {
      id: toProviderId('tg'),
      name: 'tg',
      isCustom: true,
      apiKey: 'sk-1',
      baseUrl: '',
      protocol: 'openai',
      models: [{ id: 'm1' }],
    };
    expect(isNewCustomProvider(existing)).toBe(false);
  });

  it('assigns stable unique uids to editor model rows', () => {
    const models = [{ id: '' }, { id: '' }].map((m) => ({ ...m, uid: crypto.randomUUID() }));
    expect(models[0].uid).not.toBe(models[1].uid);
    expect(models.every((m) => m.uid.length > 0)).toBe(true);
  });
});

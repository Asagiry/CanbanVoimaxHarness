import { describe, expect, it } from 'vitest';
import { normalizeProviders, providerModelLabel } from '../../../../stores/settings/providers';
import { toProviderId } from '../../../../types/domain';

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
});

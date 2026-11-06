import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useSettingsStore } from '../settingsStore';
import { useDrawerStore } from '../drawerStore';

const invoke = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('Provider selection and cache accounting', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSettingsStore.setState({
      settings: {
        provider: 'Custom', gatewayUrl: 'http://localhost:8045/v1',
        baseUrl: 'http://localhost:8045/v1', apiKey: 'old-provider-key',
        model: 'previous-model', compactorModel: 'previous-compactor',
      },
      availableModels: ['saved-model'],
    });
  });

  it('selects OpenCode Go without choosing or adding a model', () => {
    useSettingsStore.getState().setSettings({ provider: 'OpenCode Go' });
    const { settings, availableModels } = useSettingsStore.getState();
    expect(settings.gatewayUrl).toBe('https://opencode.ai/inference/go/openai/v1');
    expect(settings.baseUrl).toBe(settings.gatewayUrl);
    expect(settings.apiKey).toBe('');
    expect(settings.model).toBe('');
    expect(settings.compactorModel).toBe('inherit');
    expect(availableModels).toEqual(['saved-model']);
  });

  it('fetches candidates without adding unselected models', async () => {
    invoke.mockResolvedValue(['model-a', 'model-b']);
    const candidates = await useSettingsStore.getState().fetchAvailableModels();
    expect(candidates).toEqual(['model-a', 'model-b']);
    expect(useSettingsStore.getState().availableModels).toEqual(['saved-model']);
    useSettingsStore.getState().addAvailableModels([candidates[1]]);
    expect(useSettingsStore.getState().availableModels).toEqual(['saved-model', 'model-b']);
  });

  it('uses total input once and clears a stale cache percentage', () => {
    useDrawerStore.getState().updateTelemetry('CACHE-1', {
      inputTokens: 100000, cacheReadTokens: 99000, outputTokens: 200, cacheHitRate: 49.7,
    });
    expect(useDrawerStore.getState().taskTelemetry['CACHE-1'].cacheHitRate).toBe(99);
    useDrawerStore.getState().updateTelemetry('CACHE-1', {
      inputTokens: 2000, cacheReadTokens: 0, outputTokens: 0, cacheHitRate: 90,
    });
    expect(useDrawerStore.getState().taskTelemetry['CACHE-1'].cacheHitRate).toBe(0);
  });
});

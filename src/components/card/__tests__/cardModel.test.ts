import { describe, it, expect } from 'vitest';
import { resolveCardModel, formatCardModel } from '../cardModel';
import type { KanbanTask, PipelineStage } from '../../../types';
import type { AgentPersonaInfo } from '../../../stores/cortex/types';
import type { Provider } from '../../../stores/settings/providers';

const task = (overrides?: Partial<KanbanTask>): KanbanTask =>
  ({
    id: 'CVH-088',
    title: 't',
    priority: 'medium',
    stageId: 'dev',
    persona: 'cvh-dev',
    status: 'running',
    activeStageDuration: 0,
    stageDurations: {},
    totalElapsed: 0,
    isTimerRunning: false,
    returnsCount: 0,
    ...overrides,
  }) as KanbanTask;

const column = (overrides?: Partial<PipelineStage>): PipelineStage =>
  ({
    id: 'dev',
    title: 'Dev',
    color: '#000',
    icon: 'Bot',
    persona: 'cvh-dev',
    modelOverride: null,
    ...overrides,
  }) as PipelineStage;

const persona = (overrides?: Partial<AgentPersonaInfo>): AgentPersonaInfo => ({
  role: 'cvh-dev',
  hasPrompt: true,
  model: null,
  ...overrides,
});

const provider = (id: string, name: string, models: Provider['models']): Provider => ({
  id: id as never,
  name,
  isCustom: false,
  apiKey: '',
  baseUrl: '',
  protocol: 'openai' as const,
  models,
});

describe('resolveCardModel', () => {
  it('explicit pair wins over persona, column and settings', () => {
    expect(
      resolveCardModel(
        task({ providerId: 'p1', model: ' steer-model ' }),
        column({ modelOverride: 'col-model' }),
        persona({ model: 'persona-model' }),
        'settings-model'
      )
    ).toEqual({ label: 'steer-model' });
  });

  it('explicit pair shows provider name from providers', () => {
    const providers = [provider('p1', 'P1', [{ id: 'steer-model' }])];
    const result = resolveCardModel(
      task({ providerId: 'p1', model: 'steer-model' }),
      column(),
      persona({ model: 'persona-model' }),
      'settings-model',
      providers
    );
    expect(result).toEqual({ label: 'steer-model', providerName: 'P1' });
    expect(result && formatCardModel(result)).toBe('P1 / steer-model');
  });

  it('explicit pair provider is authoritative even if model not in its catalog', () => {
    const providers = [provider('p1', 'P1', [{ id: 'other' }])];
    expect(
      resolveCardModel(task({ providerId: 'p1', model: 'steer-model' }), column(), persona(), undefined, providers)
    ).toEqual({ label: 'steer-model', providerName: 'P1' });
  });

  it('ignores task.model without providerId (stale)', () => {
    expect(
      resolveCardModel(
        task({ model: 'stale-model' }),
        column({ modelOverride: 'col-model' }),
        persona({ model: 'persona-model' }),
        'settings-model'
      )
    ).toEqual({ label: 'persona-model' });
  });

  it('persona model wins over column override and settings', () => {
    expect(
      resolveCardModel(task(), column({ modelOverride: 'col-model' }), persona({ model: ' persona-model ' }), 'settings-model')
    ).toEqual({ label: 'persona-model' });
  });

  it('column override wins over settings when persona empty', () => {
    expect(
      resolveCardModel(task(), column({ modelOverride: ' col-model ' }), persona({ model: '  ' }), 'settings-model')
    ).toEqual({ label: 'col-model' });
  });

  it('settings model is fallback', () => {
    expect(resolveCardModel(task(), column(), persona(), ' settings-model ')).toEqual({ label: 'settings-model' });
  });

  it('returns undefined when all sources empty', () => {
    expect(resolveCardModel(task(), column(), persona(), '  ')).toBeUndefined();
    expect(resolveCardModel(task(), undefined, undefined, null)).toBeUndefined();
    expect(resolveCardModel(task(), undefined, undefined, undefined)).toBeUndefined();
  });

  it('uses catalog label and provider when providers provided', () => {
    const providers = [provider('p1', 'P1', [{ id: 'raw-id', displayName: 'Pretty Label' }])];
    expect(resolveCardModel(task(), column(), persona({ model: 'raw-id' }), undefined, providers)).toEqual({
      label: 'Pretty Label',
      providerName: 'P1',
    });
  });

  it('unknown model with providers falls back to settings.provider', () => {
    const providers = [provider('p1', 'P1', [{ id: 'raw-id' }])];
    expect(
      resolveCardModel(task(), column(), persona({ model: 'unknown' }), undefined, providers, 'Fallback Provider')
    ).toEqual({ label: 'unknown', providerName: 'Fallback Provider' });
  });

  it('unknown model with no providers shows bare label', () => {
    expect(resolveCardModel(task(), column(), persona({ model: 'unknown' }), undefined, [])).toEqual({ label: 'unknown' });
    expect(resolveCardModel(task(), column(), persona({ model: 'unknown' }), undefined, undefined)).toEqual({ label: 'unknown' });
  });

  it('persona providerId is preferred when it matches catalog', () => {
    const providers = [
      provider('p1', 'P1', [{ id: 'shared' }]),
      provider('p2', 'P2', [{ id: 'shared' }]),
    ];
    expect(
      resolveCardModel(task(), column(), persona({ model: 'shared', providerId: 'p2' }), undefined, providers)
    ).toEqual({ label: 'shared', providerName: 'P2' });
  });

  it('same model id in two providers resolves to first catalog match', () => {
    const providers = [
      provider('p1', 'P1', [{ id: 'shared' }]),
      provider('p2', 'P2', [{ id: 'shared' }]),
    ];
    expect(resolveCardModel(task(), column(), persona({ model: 'shared' }), undefined, providers)).toEqual({
      label: 'shared',
      providerName: 'P1',
    });
  });
});

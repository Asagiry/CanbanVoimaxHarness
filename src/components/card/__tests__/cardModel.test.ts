import { describe, it, expect } from 'vitest';
import { resolveCardModel } from '../cardModel';
import type { KanbanTask, PipelineStage } from '../../../types';
import type { AgentPersonaInfo } from '../../../stores/cortex/types';

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

describe('resolveCardModel', () => {
  it('explicit pair wins over persona, column and settings', () => {
    expect(
      resolveCardModel(
        task({ providerId: 'p1', model: ' steer-model ' }),
        column({ modelOverride: 'col-model' }),
        persona({ model: 'persona-model' }),
        'settings-model'
      )
    ).toBe('steer-model');
  });

  it('ignores task.model without providerId (stale)', () => {
    expect(
      resolveCardModel(
        task({ model: 'stale-model' }),
        column({ modelOverride: 'col-model' }),
        persona({ model: 'persona-model' }),
        'settings-model'
      )
    ).toBe('persona-model');
  });

  it('persona model wins over column override and settings', () => {
    expect(
      resolveCardModel(task(), column({ modelOverride: 'col-model' }), persona({ model: ' persona-model ' }), 'settings-model')
    ).toBe('persona-model');
  });

  it('column override wins over settings when persona empty', () => {
    expect(
      resolveCardModel(task(), column({ modelOverride: ' col-model ' }), persona({ model: '  ' }), 'settings-model')
    ).toBe('col-model');
  });

  it('settings model is fallback', () => {
    expect(resolveCardModel(task(), column(), persona(), ' settings-model ')).toBe('settings-model');
  });

  it('returns undefined when all sources empty', () => {
    expect(resolveCardModel(task(), column(), persona(), '  ')).toBeUndefined();
    expect(resolveCardModel(task(), undefined, undefined, null)).toBeUndefined();
    expect(resolveCardModel(task(), undefined, undefined, undefined)).toBeUndefined();
  });

  it('uses catalog label when providers provided', () => {
    const providers = [
      {
        id: 'p1' as never,
        name: 'P1',
        isCustom: false,
        apiKey: '',
        baseUrl: '',
        protocol: 'openai' as const,
        models: [{ id: 'raw-id', displayName: 'Pretty Label' }],
      },
    ];
    expect(resolveCardModel(task(), column(), persona({ model: 'raw-id' }), undefined, providers)).toBe('Pretty Label');
    expect(resolveCardModel(task(), column(), persona({ model: 'unknown' }), undefined, providers)).toBe('unknown');
  });
});

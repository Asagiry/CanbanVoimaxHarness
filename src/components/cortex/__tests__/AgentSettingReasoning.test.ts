import { describe, it, expect } from 'vitest';
import {
  buildDynamicModelOptions,
  parseSettingJson,
  serializeSetting,
  type ParsedSetting,
} from '../AgentSettingVisualEditor';
import type { Provider } from '../../../stores/settings/providers';
import { normalizeReasoningEffort } from '../../../types';

describe('AgentSetting reasoning effort', () => {
  it('parses reasoningEffort from setting JSON (camel and snake)', () => {
    expect(parseSettingJson('{"reasoningEffort":"high"}').parsed.reasoningEffort).toBe('high');
    expect(parseSettingJson('{"reasoning_effort":"low"}').parsed.reasoningEffort).toBe('low');
    expect(parseSettingJson('{"reasoningEffort":"MEDIUM"}').parsed.reasoningEffort).toBe('medium');
  });

  it('defaults to off when absent or invalid (backwards-compat)', () => {
    expect(parseSettingJson('{}').parsed.reasoningEffort).toBe('off');
    expect(parseSettingJson('{"reasoningEffort":"bogus"}').parsed.reasoningEffort).toBe('off');
    expect(parseSettingJson('{"reasoningEffort":"off"}').parsed.reasoningEffort).toBe('off');
  });

  it('does not leak reasoningEffort into rawExtra', () => {
    const { parsed } = parseSettingJson('{"reasoningEffort":"high","custom":1}');
    expect(parsed.rawExtra).toEqual({ custom: 1 });
    expect(parsed.rawExtra).not.toHaveProperty('reasoningEffort');
  });

  it('serializes reasoningEffort only when not off', () => {
    const base: ParsedSetting = {
      model: 'm1',
      reasoningEffort: 'high',
      contextWindow: 200000,
      maxTokens: 8192,
      hasAllowedTools: false,
      rawExtra: {},
    };
    expect(JSON.parse(serializeSetting(base)).reasoningEffort).toBe('high');
    expect(JSON.parse(serializeSetting({ ...base, reasoningEffort: 'off' }))).not.toHaveProperty(
      'reasoningEffort',
    );
  });

  it('roundtrips: parse(serialize(x)) preserves effort', () => {
    const base: ParsedSetting = {
      model: 'm1',
      reasoningEffort: 'medium',
      contextWindow: 200000,
      maxTokens: 8192,
      hasAllowedTools: false,
      rawExtra: {},
    };
    const back = parseSettingJson(serializeSetting(base)).parsed.reasoningEffort;
    expect(back).toBe('medium');
  });

  it('drops legacy temperature on parse/serialize', () => {
    const { parsed } = parseSettingJson('{"model":"m1","temperature":0.7}');
    expect(parsed.rawExtra).not.toHaveProperty('temperature');
    expect(JSON.parse(serializeSetting(parsed))).not.toHaveProperty('temperature');
  });

  it('buildDynamicModelOptions sorts by provider then label, inherit first', () => {
    const providers = [
      {
        id: 'b',
        name: 'Beta',
        isCustom: false,
        apiKey: '',
        baseUrl: '',
        protocol: 'openai',
        models: [{ id: 'z-model', displayName: 'Zed' }, { id: 'a-model' }],
      },
      {
        id: 'a',
        name: 'alpha',
        isCustom: false,
        apiKey: '',
        baseUrl: '',
        protocol: 'openai',
        models: [{ id: 'm-model', displayName: 'Em' }],
      },
    ] as Provider[];
    const options = buildDynamicModelOptions(providers, 'unknown-model');
    expect(options[0]).toEqual({ id: 'inherit', label: 'inherit' });
    expect(options.slice(1, 4).map((o) => o.id)).toEqual(['m-model', 'a-model', 'z-model']);
    expect(options[1].providerName).toBe('alpha');
    expect(options.at(-1)).toEqual({ id: 'unknown-model', label: 'unknown-model' });
  });

  it('normalizeReasoningEffort matches backend semantics', () => {
    expect(normalizeReasoningEffort('low')).toBe('low');
    expect(normalizeReasoningEffort(' HIGH ')).toBe('high');
    expect(normalizeReasoningEffort('off')).toBeNull();
    expect(normalizeReasoningEffort('')).toBeNull();
    expect(normalizeReasoningEffort(undefined)).toBeNull();
    expect(normalizeReasoningEffort('bogus')).toBeNull();
  });
});

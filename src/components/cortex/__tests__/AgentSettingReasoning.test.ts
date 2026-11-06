import { describe, it, expect } from 'vitest';
import {
  parseSettingJson,
  serializeSetting,
  type ParsedSetting,
} from '../AgentSettingVisualEditor';
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
      temperature: 0.2,
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
      temperature: 0.2,
      reasoningEffort: 'medium',
      contextWindow: 200000,
      maxTokens: 8192,
      hasAllowedTools: false,
      rawExtra: {},
    };
    const back = parseSettingJson(serializeSetting(base)).parsed.reasoningEffort;
    expect(back).toBe('medium');
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

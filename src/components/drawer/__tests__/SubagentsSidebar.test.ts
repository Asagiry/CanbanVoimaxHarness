import { describe, it, expect } from 'vitest';
import {
  formatSubagentRoleDisplay,
  getSubagentRoleLabel,
  formatSubagentDisplayTitle,
} from '../SubagentsSidebar';

describe('SubagentsSidebar - formatSubagentRoleDisplay invariant', () => {
  it('strictly returns "Субагент #N" in Russian for in-process subagents regardless of role passed', () => {
    const roles = ['architect', 'developer', 'reviewer', 'scout', 'explorer', 'cvh-dev', 'general', ''];
    for (const role of roles) {
      expect(formatSubagentRoleDisplay(role, 1, false, 'ru')).toBe('Субагент #1');
      expect(formatSubagentRoleDisplay(role, 42, false, 'ru')).toBe('Субагент #42');
    }
  });

  it('strictly returns "Subagent #N" in English for in-process subagents regardless of role passed', () => {
    const roles = ['architect', 'developer', 'reviewer', 'scout', 'explorer', 'cvh-dev', 'general', ''];
    for (const role of roles) {
      expect(formatSubagentRoleDisplay(role, 1, false, 'en')).toBe('Subagent #1');
      expect(formatSubagentRoleDisplay(role, 99, false, 'en')).toBe('Subagent #99');
    }
  });

  it('returns unnumbered base name when subIndex is omitted for in-process subagents', () => {
    expect(formatSubagentRoleDisplay('architect', undefined, false, 'ru')).toBe('Субагент');
    expect(formatSubagentRoleDisplay('reviewer', undefined, false, 'en')).toBe('Subagent');
    expect(formatSubagentRoleDisplay(null, undefined, false, 'ru')).toBe('Субагент');
  });

  it('formats board subtasks with role or fallback to Subtask/Подзадача', () => {
    expect(formatSubagentRoleDisplay(undefined, 1, true, 'ru')).toBe('Подзадача');
    expect(formatSubagentRoleDisplay(null, 1, true, 'en')).toBe('Subtask');
    expect(formatSubagentRoleDisplay('general', 1, true, 'en')).toBe('Subtask');
    expect(formatSubagentRoleDisplay('subtask', 1, true, 'ru')).toBe('Подзадача');
    expect(formatSubagentRoleDisplay('cvh-dev', 1, true, 'en')).toBe('CVH-DEV');
    expect(formatSubagentRoleDisplay('designer', 1, true, 'en')).toBe('Designer');
  });
});

describe('SubagentsSidebar - getSubagentRoleLabel', () => {
  it('resolves general, subagent, and subtask cleanly', () => {
    expect(getSubagentRoleLabel('general', true)).toBe('Субагент');
    expect(getSubagentRoleLabel('subagent', false)).toBe('Subagent');
    expect(getSubagentRoleLabel('subtask', true)).toBe('Подзадача');
    expect(getSubagentRoleLabel('subtask', false)).toBe('Subtask');
    expect(getSubagentRoleLabel('cvh-dev', true)).toBe('CVH-DEV');
  });

  it('capitalizes custom role names when not in store', () => {
    expect(getSubagentRoleLabel('qa-engineer', true)).toBe('Qa-engineer');
    expect(getSubagentRoleLabel('security', false)).toBe('Security');
  });
});

describe('SubagentsSidebar - formatSubagentDisplayTitle', () => {
  it('cleans sub-call IDs to friendly subagent titles', () => {
    expect(formatSubagentDisplayTitle('sub-call_123', 'ru')).toBe('субагент');
    expect(formatSubagentDisplayTitle('sub-call_123', 'en')).toBe('subagent');
    expect(formatSubagentDisplayTitle('Custom Analysis', 'ru')).toBe('Custom Analysis');
  });
});

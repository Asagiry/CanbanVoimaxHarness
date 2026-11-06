import { describe, it, expect, vi } from 'vitest';
import { resolveEffort, effortChip } from '../EffortSelector';

vi.mock('../../../stores/languageStore', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    language: 'en',
  }),
}));

describe('EffortSelector helpers', () => {
  it('resolveEffort prefers task over persona, defaults to off', () => {
    expect(resolveEffort('high', 'low')).toBe('high');
    expect(resolveEffort(null, 'medium')).toBe('medium');
    expect(resolveEffort('off', 'low')).toBe('low');
    expect(resolveEffort(null, null)).toBe('off');
    expect(resolveEffort('bogus', null)).toBe('off');
  });

  it('effortChip maps high to Max', () => {
    expect(effortChip('high')).toBe('Max');
    expect(effortChip('medium')).toBe('Medium');
    expect(effortChip('off')).toBe('Off');
    expect(effortChip('low')).toBe('Low');
  });
});

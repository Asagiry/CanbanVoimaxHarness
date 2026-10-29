import { describe, it, expect } from 'vitest';
import {
  formatDurationCompact,
  ThoughtStreamCard,
  SteerDirectiveCard,
  SubagentActivityCard,
} from '../index';

describe('ThoughtStreamCard & formatDurationCompact', () => {
  it('handles null, undefined, 0, or negative milliseconds gracefully', () => {
    expect(formatDurationCompact(null)).toBeNull();
    expect(formatDurationCompact(undefined)).toBeNull();
    expect(formatDurationCompact(0)).toBeNull();
    expect(formatDurationCompact(-50)).toBeNull();
    expect(formatDurationCompact(NaN)).toBeNull();
    expect(formatDurationCompact(Infinity)).toBeNull();
  });

  it('formats sub-second durations as milliseconds', () => {
    expect(formatDurationCompact(450)).toBe('450ms');
    expect(formatDurationCompact(12.4)).toBe('12ms');
    expect(formatDurationCompact(999)).toBe('999ms');
  });

  it('formats 1-10s durations with 1 decimal place', () => {
    expect(formatDurationCompact(1000)).toBe('1.0s');
    expect(formatDurationCompact(2450)).toBe('2.5s');
    expect(formatDurationCompact(9949)).toBe('9.9s');
  });

  it('formats 10s-59s durations as rounded seconds', () => {
    expect(formatDurationCompact(10000)).toBe('10s');
    expect(formatDurationCompact(45600)).toBe('46s');
    expect(formatDurationCompact(59400)).toBe('59s');
  });

  it('formats multi-minute durations as minutes and seconds', () => {
    expect(formatDurationCompact(60000)).toBe('1m 0s');
    expect(formatDurationCompact(75000)).toBe('1m 15s');
    expect(formatDurationCompact(135000)).toBe('2m 15s');
    expect(formatDurationCompact(3665000)).toBe('61m 5s');
  });
});

describe('Stream Cards Exports', () => {
  it('exports ThoughtStreamCard, SteerDirectiveCard, SubagentActivityCard correctly', () => {
    expect(ThoughtStreamCard).toBeDefined();
    expect(SteerDirectiveCard).toBeDefined();
    expect(SubagentActivityCard).toBeDefined();
  });
});

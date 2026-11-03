import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderToString } from 'react-dom/server';
import { setLanguage } from '../../../i18n';
import {
  formatDurationCompact,
  formatTtft,
  formatTps,
  formatTokensWithUnit,
  formatCompactTokens,
  pluralize,
} from '../telemetryFormatters';
import { SessionStatsPopover } from '../SessionStatsPopover';
import { TokenUsagePopover } from '../TokenUsagePopover';
import ContextGauge from '../ContextGauge';

// Mock Tauri invoke and event APIs
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(128000),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

describe('telemetryFormatters', () => {
  describe('formatDurationCompact', () => {
    it('handles zero, negative, or invalid values', () => {
      expect(formatDurationCompact(0)).toBe('0s');
      expect(formatDurationCompact(-10)).toBe('0s');
      expect(formatDurationCompact(null)).toBe('0s');
      expect(formatDurationCompact(undefined)).toBe('0s');
      expect(formatDurationCompact(NaN)).toBe('0s');
    });

    it('formats durations under 60 seconds', () => {
      expect(formatDurationCompact(12)).toBe('12s');
      expect(formatDurationCompact(45.4)).toBe('45s');
      expect(formatDurationCompact(59)).toBe('59s');
    });

    it('formats durations between 1 minute and 1 hour', () => {
      expect(formatDurationCompact(293)).toBe('4m53s');
      expect(formatDurationCompact(1305)).toBe('21m45s');
    });

    it('formats durations over 1 hour', () => {
      expect(formatDurationCompact(3660)).toBe('1h1m');
      expect(formatDurationCompact(7325)).toBe('2h2m');
    });
  });

  describe('formatTtft', () => {
    it('handles falsy or invalid values with an em dash', () => {
      expect(formatTtft(null)).toBe('—');
      expect(formatTtft(undefined)).toBe('—');
      expect(formatTtft(0)).toBe('—');
      expect(formatTtft(-5)).toBe('—');
    });

    it('formats millisecond durations to compact seconds', () => {
      expect(formatTtft(15500)).toBe('15.5s');
      expect(formatTtft(800)).toBe('0.8s');
      expect(formatTtft(2400)).toBe('2.4s');
    });
  });

  describe('formatTps', () => {
    it('handles falsy or invalid values with an em dash', () => {
      expect(formatTps(null)).toBe('—');
      expect(formatTps(undefined)).toBe('—');
      expect(formatTps(0)).toBe('—');
    });

    it('formats high TPS as integer and low TPS with 1 decimal', () => {
      expect(formatTps(276)).toBe('276 tok/s');
      expect(formatTps(276.4)).toBe('276 tok/s');
      expect(formatTps(8.4)).toBe('8.4 tok/s');
    });
  });

  describe('formatTokensWithUnit', () => {
    it('formats zero or invalid tokens', () => {
      expect(formatTokensWithUnit(0)).toBe('0 tok');
      expect(formatTokensWithUnit(null)).toBe('0 tok');
      expect(formatTokensWithUnit(undefined)).toBe('0 tok');
    });

    it('formats thousands and millions with US comma separators', () => {
      expect(formatTokensWithUnit(7046587)).toBe('7,046,587 tok');
      expect(formatTokensWithUnit(414047)).toBe('414,047 tok');
      expect(formatTokensWithUnit(14876)).toBe('14,876 tok');
    });
  });

  describe('formatCompactTokens', () => {
    it('formats zero or invalid tokens', () => {
      expect(formatCompactTokens(0)).toBe('0 tok');
      expect(formatCompactTokens(null)).toBe('0 tok');
    });

    it('formats millions and thousands with compact suffixes', () => {
      expect(formatCompactTokens(7046587)).toBe('7M tok');
      expect(formatCompactTokens(7500000)).toBe('7.5M tok');
      expect(formatCompactTokens(414047)).toBe('414k tok');
      expect(formatCompactTokens(850)).toBe('850 tok');
    });
  });

  describe('pluralize', () => {
    it('handles English pluralization', () => {
      expect(pluralize(1, 'turn', 'turns', 'turns', 'en')).toBe('turn');
      expect(pluralize(7, 'turn', 'turns', 'turns', 'en')).toBe('turns');
      expect(pluralize(81, 'step', 'steps', 'steps', 'en')).toBe('steps');
    });

    it('handles Russian grammatical pluralization cases', () => {
      expect(pluralize(1, 'ход', 'хода', 'ходов', 'ru')).toBe('ход');
      expect(pluralize(2, 'ход', 'хода', 'ходов', 'ru')).toBe('хода');
      expect(pluralize(5, 'ход', 'хода', 'ходов', 'ru')).toBe('ходов');
      expect(pluralize(11, 'ход', 'хода', 'ходов', 'ru')).toBe('ходов');
      expect(pluralize(21, 'ход', 'хода', 'ходов', 'ru')).toBe('ход');
      expect(pluralize(81, 'шаг', 'шага', 'шагов', 'ru')).toBe('шаг');
      expect(pluralize(82, 'шаг', 'шага', 'шагов', 'ru')).toBe('шага');
      expect(pluralize(85, 'шаг', 'шага', 'шагов', 'ru')).toBe('шагов');
    });
  });
});

describe('SessionStatsPopover Component', () => {
  beforeEach(() => {
    setLanguage('en');
  });

  it('renders session statistics header and all 4 metrics correctly', () => {
    const html = renderToString(
      <SessionStatsPopover
        llmTimeSec={1305}
        toolTimeSec={293}
        ttftMs={15500}
        tpsValue={276}
      />
    );

    expect(html).toContain('Session statistics');
    expect(html).toContain('LLM time');
    expect(html).toContain('21m45s');
    expect(html).toContain('Tool time');
    expect(html).toContain('4m53s');
    expect(html).toContain('Avg time to first token (TTFT)');
    expect(html).toContain('15.5s');
    expect(html).toContain('Tokens per second (TPS)');
    expect(html).toContain('276 tok/s');
  });
});

describe('TokenUsagePopover Component', () => {
  beforeEach(() => {
    setLanguage('en');
  });

  it('renders token usage header, total count, and all 4 breakdown rows', () => {
    const html = renderToString(
      <TokenUsagePopover
        cacheHitRate={94}
        uncachedInput={414047}
        cachedInput={6617664}
        outputTokens={14876}
        totalTokens={7046587}
      />
    );

    expect(html).toContain('Token usage');
    expect(html).toContain('7,046,587 tok');
    expect(html).toContain('Cache hit');
    expect(html).toContain('94%');
    expect(html).toContain('Uncached input');
    expect(html).toContain('414,047 tok');
    expect(html).toContain('Cached input');
    expect(html).toContain('6,617,664 tok');
    expect(html).toContain('Output');
    expect(html).toContain('14,876 tok');
  });
});

describe('ContextGauge Component UI Invariants', () => {
  beforeEach(() => {
    setLanguage('en');
  });

  it('renders all 3 pills and has no native browser title tooltips on status bar items', () => {
    const html = renderToString(
      <ContextGauge
        taskId="task-test-123"
        turns={7}
        contextWindow={128000}
        currentEstimatedTokens={60000}
      />
    );

    // Verify pills are rendered
    expect(html).toContain('7 turns');
    expect(html).toContain('Cache hit');
    expect(html).toMatch(/47(<!-- -->)?%/);

    // Verify UI invariant § 3.5: No native title attributes on the status bar container or pills
    // (Excluding internal ContextPopover progress segments if any)
    const statusBarMatches = html.match(/class="[^"]*select-none[^"]*"[^>]*title=/);
    expect(statusBarMatches).toBeNull();
  });
});

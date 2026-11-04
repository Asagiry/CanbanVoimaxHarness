import { describe, it, expect } from 'vitest';
import {
  formatMarkdownForToast,
  formatNotificationTitle,
  getStageIconName,
} from '../notificationFormatter';

describe('notificationFormatter', () => {
  describe('formatMarkdownForToast', () => {
    it('returns empty string for null, undefined, or empty raw input', () => {
      expect(formatMarkdownForToast(null)).toBe('');
      expect(formatMarkdownForToast(undefined)).toBe('');
      expect(formatMarkdownForToast('')).toBe('');
      expect(formatMarkdownForToast('   ')).toBe('');
    });

    it('strips Markdown headings and keeps text', () => {
      const input = '# Header 1\n## Header 2\n### Header 3\nContent line';
      const output = formatMarkdownForToast(input);
      expect(output).toBe('Header 1\nHeader 2\nHeader 3\nContent line');
    });

    it('converts bold and italic text cleanly', () => {
      const input = 'This is **bold** and *italic* and __bold2__ and _italic2_.';
      const output = formatMarkdownForToast(input);
      expect(output).toBe('This is bold and italic and bold2 and italic2.');
    });

    it('normalizes inline code to single quotes', () => {
      const input = 'Execute `pnpm test` and `cargo check`.';
      const output = formatMarkdownForToast(input);
      expect(output).toBe("Execute 'pnpm test' and 'cargo check'.");
    });

    it('formats multi-line code blocks with vertical bar prefixes', () => {
      const input = '```rust\nfn main() {\n    println!("hello");\n}\n```';
      const output = formatMarkdownForToast(input);
      expect(output).toBe('│ fn main() {\n│     println!("hello");\n│ }');
    });

    it('converts list items to Unicode bullets', () => {
      const input = '- Item 1\n* Item 2\n+ Item 3';
      const output = formatMarkdownForToast(input);
      expect(output).toBe('• Item 1\n• Item 2\n• Item 3');
    });

    it('converts blockquotes to vertical bar prefixes', () => {
      const input = '> Crucial note: all tests pass.';
      const output = formatMarkdownForToast(input);
      expect(output).toBe('│ Crucial note: all tests pass.');
    });

    it('cleans links and images', () => {
      const input = 'Visit [Project Repo](https://github.com/voimax/cvh) and ![Badge](https://img.shields.io)';
      const output = formatMarkdownForToast(input);
      expect(output).toBe('Visit Project Repo and [Badge]');
    });

    it('removes horizontal rules and collapses excess newlines', () => {
      const input = 'Section 1\n\n---\n\n\n\nSection 2';
      const output = formatMarkdownForToast(input);
      expect(output).toBe('Section 1\n\nSection 2');
    });

    it('strips raw HTML tags', () => {
      const input = '<div class="alert">Notice: <b>done</b></div>';
      const output = formatMarkdownForToast(input);
      expect(output).toBe('Notice: done');
    });

    it('intelligently truncates text at word boundaries exceeding maxLength', () => {
      const input = 'The quick brown fox jumps over the lazy dog repeatedly until the sentence becomes extremely long and exceeds the allowed threshold of characters for desktop notification display.';
      const output = formatMarkdownForToast(input, 50);
      expect(output.length).toBeLessThanOrEqual(50);
      expect(output.endsWith('…')).toBe(true);
      expect(output).not.toContain('   ');
    });

    it('truncates at slicePoint if no whitespace exists within margin', () => {
      const input = 'Supercalifragilisticexpialidociousunbreakablewordthatdoesnotstop';
      const output = formatMarkdownForToast(input, 20);
      expect(output.length).toBeLessThanOrEqual(20);
      expect(output.endsWith('…')).toBe(true);
    });
  });

  describe('formatNotificationTitle', () => {
    it('formats advance_task title without emojis for non-terminal stage', () => {
      const title = formatNotificationTitle('advance_task', 'CVH-042', { stageTitle: 'Review' });
      expect(title).toBe('Задача CVH-042 перешла в колонку Review');
      expect(title.includes('🚀')).toBe(false);
      expect(title.includes('✅')).toBe(false);
      expect(title.includes('❓')).toBe(false);
      expect(title.includes('⚠️')).toBe(false);
    });

    it('formats advance_task title without emojis for terminal stage', () => {
      const title = formatNotificationTitle('advance_task', 'CVH-042', { isTerminal: true });
      expect(title).toBe('Задача CVH-042 завершена.');
      expect(title.includes('🚀')).toBe(false);
      expect(title.includes('✅')).toBe(false);
      expect(title.includes('❓')).toBe(false);
      expect(title.includes('⚠️')).toBe(false);
    });

    it('formats question title without emojis', () => {
      const title = formatNotificationTitle('question', 'CVH-042');
      expect(title).toBe('В задаче CVH-042 вопрос от агента.');
      expect(title.includes('🚀')).toBe(false);
      expect(title.includes('✅')).toBe(false);
      expect(title.includes('❓')).toBe(false);
      expect(title.includes('⚠️')).toBe(false);
    });

    it('formats reject_task title without emojis', () => {
      const title = formatNotificationTitle('reject_task', 'CVH-042');
      expect(title).toBe('Задача CVH-042 возвращена на доработку.');
      expect(title.includes('🚀')).toBe(false);
      expect(title.includes('✅')).toBe(false);
      expect(title.includes('❓')).toBe(false);
      expect(title.includes('⚠️')).toBe(false);
    });

    it('formats test notification title', () => {
      const title = formatNotificationTitle('test');
      expect(title).toBe('CanbanVoimaxHarness');
    });
  });

  describe('getStageIconName', () => {
    it('maps known stage IDs to canonical icon names', () => {
      expect(getStageIconName('backlog')).toBe('backlog');
      expect(getStageIconName('inbox')).toBe('backlog');
      expect(getStageIconName('plan')).toBe('plan');
      expect(getStageIconName('dev')).toBe('dev');
      expect(getStageIconName('developer')).toBe('dev');
      expect(getStageIconName('review')).toBe('review');
      expect(getStageIconName('retro')).toBe('retro');
      expect(getStageIconName('ready')).toBe('ready');
      expect(getStageIconName('done')).toBe('ready');
    });

    it('returns ready icon when isTerminal is true', () => {
      expect(getStageIconName('custom-terminal', true)).toBe('ready');
    });

    it('returns app icon for unknown stages', () => {
      expect(getStageIconName('custom-stage')).toBe('app');
      expect(getStageIconName(undefined)).toBe('app');
    });
  });
});

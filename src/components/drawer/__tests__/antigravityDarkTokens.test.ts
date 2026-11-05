import { describe, it, expect } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';

describe('Antigravity Dark Tokens Migration for ImageDiffViewer and SubagentToolCard', () => {
  const legacyDarkTokens = [
    'dark:bg-[#0d1117]',
    'dark:bg-[#161b22]',
    'dark:bg-[#21262d]',
    'dark:border-[#21262d]',
    'dark:border-[#30363d]',
    'dark:hover:bg-[#30363d]',
    'dark:text-[#f0f6fc]',
    'dark:text-[#8b949e]',
    'dark:placeholder:text-[#8b949e]',
  ];

  it('ImageDiffViewer.tsx contains 0 legacy dark tokens and uses Antigravity Dark palette', () => {
    const filePath = path.resolve(__dirname, '../diff/ImageDiffViewer.tsx');
    const content = fs.readFileSync(filePath, 'utf-8');

    for (const token of legacyDarkTokens) {
      expect(content).not.toContain(token);
    }

    // Verify key Antigravity tokens are present
    expect(content).toContain('dark:bg-[#111114]');
    expect(content).toContain('dark:bg-[#17171b]');
    expect(content).toContain('dark:border-[#212126]');
    expect(content).toContain('dark:border-[#2c2c33]');
    expect(content).toContain('dark:text-[#f4f4f5]');
    expect(content).toContain('dark:text-[#71717a]');
  });

  it('SubagentToolCard.tsx contains 0 legacy dark tokens and uses Antigravity Dark palette', () => {
    const filePath = path.resolve(__dirname, '../tools/SubagentToolCard.tsx');
    const content = fs.readFileSync(filePath, 'utf-8');

    for (const token of legacyDarkTokens) {
      expect(content).not.toContain(token);
    }

    // Verify key Antigravity tokens are present
    expect(content).toContain('dark:bg-[#111114]');
    expect(content).toContain('dark:bg-[#17171b]');
    expect(content).toContain('dark:border-[#212126]');
    expect(content).toContain('dark:border-[#2c2c33]');
    expect(content).toContain('dark:text-[#f4f4f5]');
    expect(content).toContain('dark:text-[#71717a]');
  });
});

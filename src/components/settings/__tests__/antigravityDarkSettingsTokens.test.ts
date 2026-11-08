import { describe, it, expect } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';

describe('Antigravity Dark Tokens Migration for SettingsModal and ToolPolicyModal', () => {
  const legacyDarkTokens = [
    'dark:bg-[#0d1117]',
    'dark:bg-[#161b22]',
    'dark:hover:bg-[#161b22]',
    'dark:focus:bg-[#0d1117]',
    'dark:bg-[#21262d]',
    'dark:border-[#21262d]',
    'dark:border-[#30363d]',
    'dark:hover:border-[#484f58]',
    'dark:focus:border-[#f0f6fc]',
    'dark:text-[#f0f6fc]',
    'dark:text-[#8b949e]',
    'dark:placeholder:text-[#8b949e]',
    'dark:placeholder:text-[#6e7681]',
    'dark:bg-[#07090e]',
    'dark:bg-[#090a0f]',
  ];

  it('SettingsModal.tsx contains 0 legacy dark tokens and uses Antigravity Dark palette', () => {
    const filePath = path.resolve(__dirname, '../SettingsModal.tsx');
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
    expect(content).toContain('dark:hover:bg-[#1e1e23]');
    expect(content).toContain('dark:hover:bg-[#232328]');
    expect(content).toContain('dark:bg-[#232328]');
  });

  it('ToolPolicyModal.tsx and tool-policy/* subcomponents contain 0 legacy dark tokens and use Antigravity Dark palette', () => {
    const files = [
      '../ToolPolicyModal.tsx',
      '../tool-policy/ToolSchemaEditor.tsx',
      '../tool-policy/ToolParamsEditor.tsx',
      '../tool-policy/CustomToolForm.tsx',
    ];
    const content = files
      .map((rel) => fs.readFileSync(path.resolve(__dirname, rel), 'utf-8'))
      .join('\n');

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
    expect(content).toContain('dark:placeholder:text-[#71717a]');
    expect(content).toContain('dark:hover:bg-[#1e1e23]');
    expect(content).toContain('dark:hover:bg-[#232328]');
    expect(content).toContain('dark:bg-[#232328]');
    expect(content).toContain('dark:bg-[#0b0b0e]');
  });
});

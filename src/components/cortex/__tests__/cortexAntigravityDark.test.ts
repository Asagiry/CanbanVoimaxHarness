import { describe, it, expect } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';

describe('Antigravity Dark Tokens Migration for Cortex and Common Components', () => {
  const legacyDarkTokens = [
    'dark:bg-[#0d1117]',
    'dark:bg-[#161b22]',
    'dark:bg-[#21262d]',
    'dark:border-[#21262d]',
    'dark:border-[#30363d]',
    'dark:hover:bg-[#161b22]',
    'dark:hover:bg-[#21262d]',
    'dark:hover:bg-[#30363d]',
    'dark:hover:border-[#484f58]',
    'dark:focus:bg-[#0d1117]',
    'dark:focus:border-[#f0f6fc]',
    'dark:text-[#f0f6fc]',
    'dark:text-[#8b949e]',
    'dark:placeholder:text-[#8b949e]',
    'dark:placeholder-[#6e7681]',
    'dark:disabled:bg-[#0d1117]',
    'dark:disabled:text-[#6e7681]',
  ];

  const targetFiles = [
    'src/components/cortex/AgentMindInspector.tsx',
    'src/components/cortex/AgentSettingVisualEditor.tsx',
    'src/components/cortex/ColumnStageCard.tsx',
    'src/components/cortex/CortexExplorer.tsx',
    'src/components/cortex/CreateAgentModal.tsx',
    'src/components/cortex/EditAgentPersonaModal.tsx',
    'src/components/cortex/GlobalRulesSection.tsx',
    'src/components/cortex/NewCortexFileForm.tsx',
    'src/components/cortex/RuleEditorModal.tsx',
    'src/components/common/ErrorBoundary.tsx',
    'src/components/common/PersonaSelect.tsx',
  ];

  for (const relPath of targetFiles) {
    it(`${relPath} contains 0 legacy dark tokens`, () => {
      const fullPath = path.resolve(__dirname, '../../../../', relPath);
      const content = fs.readFileSync(fullPath, 'utf-8');

      for (const token of legacyDarkTokens) {
        expect(content, `File ${relPath} still contains ${token}`).not.toContain(token);
      }
    });
  }
});

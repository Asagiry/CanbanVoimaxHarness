import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderToString } from 'react-dom/server';
import CvhSaveButton from '../CvhSaveButton';
import { useCortexStore } from '../../../stores/cortexStore';
import { hasAnyCvhChanges, isAgentDirty, isGlobalRulesDirty } from '../../../stores/cortex/cortexGitSave';

vi.mock('../../../stores/languageStore', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

describe('CvhSaveButton', () => {
  beforeEach(() => {
    useCortexStore.setState({
      cvhGitStatus: { hasChanges: false, changedPaths: [], branch: 'main' },
      cvhGitSaving: false,
      cvhGitError: null,
      pendingDeletes: {},
      stages: [],
      agentStages: [],
      globalRules: [],
      activeFile: null,
    });
  });

  it('renders green (disabled) when no changes (initial state)', () => {
    const html = renderToString(<CvhSaveButton />);
    expect(html).toContain('text-emerald-500');
    expect(html).toContain('disabled');
    expect(html).not.toContain('animate-spin');
  });

  it('hasAnyCvhChanges reflects git status, tombstones and dirty files', () => {
    const base = { cvhGitStatus: null, pendingDeletes: {}, stages: [], agentStages: [], globalRules: [], activeFile: null };
    expect(hasAnyCvhChanges(base)).toBe(false);
    expect(hasAnyCvhChanges({ ...base, cvhGitStatus: { hasChanges: true, changedPaths: ['.cvh/rules/x.md'], branch: 'main' } })).toBe(true);
    expect(hasAnyCvhChanges({ ...base, pendingDeletes: { 'file:a': { path: '.cvh/rules/a.md', kind: 'globalRule' } } })).toBe(true);
    expect(hasAnyCvhChanges({ ...base, activeFile: { id: 'a', name: 'a', stageId: 'global', type: 'rule', path: 'p', content: '', isDirty: true } })).toBe(true);
  });

  it('selectors derive dirty state from changedPaths prefixes', () => {
    const stage = { stageId: 'agent:dev', stageTitle: 'dev', color: '', icon: '', personaName: 'dev', files: [], isExpanded: false };
    const status = { hasChanges: true, changedPaths: ['.cvh/agents/dev/rules/a.md'], branch: 'main' };
    expect(isAgentDirty(stage, status, {})).toBe(true);
    expect(isAgentDirty(stage, { hasChanges: true, changedPaths: ['.cvh/rules/x.md'], branch: 'main' }, {})).toBe(false);
    expect(isGlobalRulesDirty(status, [])).toBe(false);
    expect(isGlobalRulesDirty({ hasChanges: true, changedPaths: ['.cvh/rules/x.md'], branch: 'main' }, [])).toBe(true);
  });
});

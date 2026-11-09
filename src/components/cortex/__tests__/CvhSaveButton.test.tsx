import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderToString } from 'react-dom/server';
import CvhSaveButton from '../CvhSaveButton';
import { useCortexStore } from '../../../stores/cortexStore';
import { hasAnyCvhChanges, isAgentDirty, isColumnDirty, isFileDirty, isGlobalRulesDirty, isPersonaFileDirty } from '../../../stores/cortex/cortexGitSave';

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
      pendingColumnChanges: {},
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
    const base = { cvhGitStatus: null, pendingDeletes: {}, pendingColumnChanges: {}, stages: [], agentStages: [], globalRules: [], activeFile: null };
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

  it('isColumnDirty ignores boards.json-only changes (CVH-104 regression)', () => {
    const mkStage = (stageId: string, files: never[] = []) => ({
      stageId, stageTitle: stageId, color: '', icon: '', personaName: 'система', files, isExpanded: false,
    });
    const boardsOnly = { hasChanges: true, changedPaths: ['.cvh/boards.json'], branch: 'main' };
    for (const id of ['backlog', 'planning', 'developer', 'review', 'retro', 'ready']) {
      expect(isColumnDirty(mkStage(id), undefined, boardsOnly)).toBe(false);
    }
  });

  it('isColumnDirty matches only the column whose hook path changed', () => {
    const hookFile = { id: 'ready-hook-on_enter', name: 'on_enter_ready.bat', stageId: 'ready', type: 'hook' as const, path: '.cvh/hooks/on_enter_ready.bat', content: '', isDirty: false, isDeleted: false };
    const ready = { stageId: 'ready', stageTitle: 'Ready', color: '', icon: '', personaName: 'система', files: [hookFile], isExpanded: false };
    const planning = { stageId: 'planning', stageTitle: 'Planning', color: '', icon: '', personaName: 'система', files: [], isExpanded: false };
    const status = { hasChanges: true, changedPaths: ['.cvh/hooks/on_enter_ready.bat', '.cvh/boards.json'], branch: 'main' };
    expect(isColumnDirty(ready, undefined, status)).toBe(true);
    expect(isColumnDirty(planning, undefined, status)).toBe(false);
  });

  it('isColumnDirty matches hook prefix and pendingColumnChanges', () => {
    const stage = { stageId: 'ready', stageTitle: 'Ready', color: '', icon: '', personaName: 'система', files: [], isExpanded: false };
    expect(isColumnDirty(stage, undefined, { hasChanges: true, changedPaths: ['.cvh/hooks/ready-on_enter.bat'], branch: 'main' })).toBe(true);
    expect(isColumnDirty(stage, undefined, null, { ready: true })).toBe(true);
    expect(isColumnDirty(stage, undefined, null, {})).toBe(false);
  });

  it('isFileDirty reflects git changedPaths and local flags', () => {
    const status = { hasChanges: true, changedPaths: ['.cvh/rules/a.md'], branch: 'main' };
    expect(isFileDirty({ path: '.cvh/rules/a.md', isDirty: false, isDeleted: false }, status)).toBe(true);
    expect(isFileDirty({ path: '.cvh/rules/b.md', isDirty: false, isDeleted: false }, status)).toBe(false);
    expect(isFileDirty({ path: '.cvh/rules/b.md', isDirty: true, isDeleted: false }, status)).toBe(true);
    expect(isFileDirty({ path: 'builtin:on_enter', isDirty: false, isDeleted: false }, status)).toBe(false);
  });

  it('isFileDirty handles agent.md/persona.md alias and setting (no extension)', () => {
    const agentMd = { hasChanges: true, changedPaths: ['.cvh/agents/dev/agent.md'], branch: 'main' };
    const personaMd = { hasChanges: true, changedPaths: ['.cvh/agents/dev/persona.md'], branch: 'main' };
    const setting = { hasChanges: true, changedPaths: ['.cvh/agents/dev/setting'], branch: 'main' };
    expect(isFileDirty({ path: '.cvh/agents/dev/persona.md', isDirty: false, isDeleted: false }, agentMd)).toBe(true);
    expect(isFileDirty({ path: '.cvh/agents/dev/agent.md', isDirty: false, isDeleted: false }, personaMd)).toBe(true);
    expect(isFileDirty({ path: '.cvh/agents/dev/setting', isDirty: false, isDeleted: false }, setting)).toBe(true);
    expect(isPersonaFileDirty({ path: '.cvh/agents/dev/persona.md', isDirty: false, isDeleted: false }, agentMd)).toBe(true);
    expect(isFileDirty({ path: '.cvh/agents/other/persona.md', isDirty: false, isDeleted: false }, agentMd)).toBe(false);
  });

  it('isColumnDirty matches agent rules and global rules prefixes', () => {
    const stage = { stageId: 'dev', stageTitle: 'Dev', color: '', icon: '', personaName: 'dev', files: [], isExpanded: false };
    expect(isColumnDirty(stage, undefined, { hasChanges: true, changedPaths: ['.cvh/agents/dev/rules/a.md'], branch: 'main' })).toBe(true);
    expect(isColumnDirty(stage, undefined, { hasChanges: true, changedPaths: ['.cvh/rules/x.md'], branch: 'main' })).toBe(true);
    expect(isColumnDirty(stage, undefined, { hasChanges: true, changedPaths: ['.cvh/agents/dev/setting'], branch: 'main' })).toBe(true);
  });

  it('isGlobalRulesDirty uses unfiltered list when filtered list misses dirty rule', () => {
    const dirtyRule = { id: 'r1', name: 'a.md', stageId: 'global', type: 'rule' as const, path: '.cvh/rules/a.md', content: '', isDirty: true, isDeleted: false };
    expect(isGlobalRulesDirty(null, [])).toBe(false);
    expect(isGlobalRulesDirty(null, [dirtyRule])).toBe(true);
  });
});

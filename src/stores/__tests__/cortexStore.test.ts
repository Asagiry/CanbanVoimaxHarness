import { describe, it, expect, beforeEach, beforeAll, vi } from 'vitest';
import { useCortexStore } from '../cortexStore';
import { useBoardStore } from '../boardStore';
import { DEFAULT_CORTEX_STAGES } from '../cortex/cortexConstants';
import { CANONICAL_BOARD_COLUMNS, DEFAULT_BOARD } from '../board/boardConstants';
import { RawAgentPersona } from '../cortex/types';

const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('cortexStore (Clean Deterministic Store Tests)', () => {
  beforeAll(() => {
    (globalThis as unknown as { alert: (msg: string) => void }).alert = vi.fn();
  });

  beforeEach(() => {
    vi.clearAllMocks();
    useBoardStore.setState({
      boards: [DEFAULT_BOARD],
      activeBoardId: 'development',
      columns: CANONICAL_BOARD_COLUMNS.map((c) => ({ ...c })),
      tasks: [],
    });

    useCortexStore.setState({
      stages: DEFAULT_CORTEX_STAGES.map((s) => ({
        ...s,
        files: s.files.map((f) => ({ ...f })),
      })),
      agentStages: [],
      agentPersonas: [],
      activeFile: null,
      focusedStageId: null,
      isEditorOpen: false,
      isSaving: false,
      saveFeedback: null,
      fileMissing: {},
    });
  });

  describe('Stage Expansion & Navigation Flow', () => {
    it('toggleStageExpand toggles expanded state for given stage', () => {
      const initial = useCortexStore.getState().stages.find((s) => s.stageId === 'backlog');
      expect(initial?.isExpanded).toBe(false);

      useCortexStore.getState().toggleStageExpand('backlog');
      let stage = useCortexStore.getState().stages.find((s) => s.stageId === 'backlog');
      expect(stage?.isExpanded).toBe(true);

      useCortexStore.getState().toggleStageExpand('backlog');
      stage = useCortexStore.getState().stages.find((s) => s.stageId === 'backlog');
      expect(stage?.isExpanded).toBe(false);
    });

    it('expandStage forces isExpanded to true', () => {
      useCortexStore.getState().expandStage('backlog');
      const stage = useCortexStore.getState().stages.find((s) => s.stageId === 'backlog');
      expect(stage?.isExpanded).toBe(true);
    });

    it('focusStage sets focusedStageId and expands target stage', () => {
      useCortexStore.getState().focusStage('backlog');
      const state = useCortexStore.getState();
      expect(state.focusedStageId).toBe('backlog');
      const stage = state.stages.find((s) => s.stageId === 'backlog');
      expect(stage?.isExpanded).toBe(true);
    });

    it('syncStagesFromColumns preserves custom stage files while synchronizing columns', () => {
      const customCols = [
        {
          id: 'dev',
          title: 'Development Work',
          color: '#6366f1',
          icon: 'Code2',
          persona: 'cvh-dev',
          modelOverride: null,
        },
        {
          id: 'qa',
          title: 'Quality Assurance',
          color: '#a855f7',
          icon: 'ShieldCheck',
          persona: 'tester',
          modelOverride: null,
        },
      ];

      useCortexStore.getState().syncStagesFromColumns(customCols);

      const stages = useCortexStore.getState().stages;
      expect(stages).toHaveLength(2);
      expect(stages[0].stageId).toBe('dev');
      expect(stages[0].stageTitle).toBe('Development Work');
      expect(stages[1].stageId).toBe('qa');
      expect(stages[1].stageTitle).toBe('Quality Assurance');
      // Preserved dev files from previous stage
      expect(stages[0].files.length).toBeGreaterThan(0);
    });
  });

  describe('Persona Management & Color Invariants', () => {
    it('loadAgentPersonas populates agentPersonas and agentStages from IPC', async () => {
      const mockRawPersonas: RawAgentPersona[] = [
        {
          role: 'designer',
          color: '#f59e0b',
          icon: 'Sparkles',
          rules: ['ui-guidelines.md'],
          hooks: [],
          personaFile: 'persona.md',
        },
        {
          role: 'cvh-dev',
          color: '#6366f1',
          icon: 'Cpu',
          rules: ['tools.md', 'boards.md'],
          hooks: [],
          isBuiltin: true,
        },
      ];

      mockInvoke.mockImplementation(async (cmd) => {
        if (cmd === 'list_agent_personas') {
          return mockRawPersonas;
        }
        return null;
      });

      await useCortexStore.getState().loadAgentPersonas();

      const { agentPersonas, agentStages } = useCortexStore.getState();
      expect(agentPersonas).toHaveLength(2);
      expect(agentPersonas[0].role).toBe('cvh-dev');
      expect(agentPersonas[1].role).toBe('designer');

      const designerStage = agentStages.find((s) => s.personaName === 'designer');
      expect(designerStage).toBeDefined();
      expect(designerStage?.color).toBe('#f59e0b');
      expect(designerStage?.icon).toBe('Sparkles');
      // Contains persona file, rules, and settings
      expect(designerStage?.files.some((f) => f.name === 'ui-guidelines.md')).toBe(true);
      expect(designerStage?.files.some((f) => f.type === 'settings')).toBe(true);
    });

    it('setAgentPersonaColor throws error on forbidden reserved colors (backlog/ready)', async () => {
      await expect(
        useCortexStore.getState().setAgentPersonaColor('cvh-dev', '#64748b')
      ).rejects.toThrow('Серый и зеленый цвета зарезервированы');

      await expect(
        useCortexStore.getState().setAgentPersonaColor('cvh-dev', '#10b981')
      ).rejects.toThrow('Серый и зеленый цвета зарезервированы');
    });

    it('setAgentPersonaColor updates color and cascades to matching board columns', async () => {
      mockInvoke.mockResolvedValue(true);

      useCortexStore.setState({
        agentPersonas: [
          {
            role: 'cvh-dev',
            color: '#6366f1',
            hasPrompt: true,
            isBuiltin: true,
            personaFile: 'persona.md',
            rules: [],
            hooks: [],
          },
        ],
        agentStages: [
          {
            stageId: 'agent:cvh-dev',
            stageTitle: 'cvh-dev',
            color: '#6366f1',
            icon: 'Cpu',
            personaName: 'cvh-dev',
            isExpanded: true,
            files: [],
          },
        ],
      });

      // dev column currently uses persona cvh-dev
      await useCortexStore.getState().setAgentPersonaColor('cvh-dev', '#a855f7');

      const persona = useCortexStore.getState().agentPersonas.find((p) => p.role === 'cvh-dev');
      expect(persona?.color).toBe('#a855f7');

      const devCol = useBoardStore.getState().columns.find((c) => c.id === 'dev');
      expect(devCol?.color).toBe('#a855f7');
      expect(mockInvoke).toHaveBeenCalledWith('set_agent_persona_color', {
        role: 'cvh-dev',
        color: '#a855f7',
      });
    });

    it('setAgentPersonaIcon updates icon in persona, stages, and board columns', async () => {
      mockInvoke.mockResolvedValue(true);

      useCortexStore.setState({
        agentPersonas: [
          {
            role: 'cvh-dev',
            color: '#6366f1',
            icon: 'Bot',
            hasPrompt: true,
            isBuiltin: true,
            personaFile: 'persona.md',
            rules: [],
            hooks: [],
          },
        ],
        agentStages: [
          {
            stageId: 'agent:cvh-dev',
            stageTitle: 'cvh-dev',
            color: '#6366f1',
            icon: 'Bot',
            personaName: 'cvh-dev',
            isExpanded: true,
            files: [],
          },
        ],
      });

      await useCortexStore.getState().setAgentPersonaIcon('cvh-dev', 'Terminal');

      const persona = useCortexStore.getState().agentPersonas.find((p) => p.role === 'cvh-dev');
      expect(persona?.icon).toBe('Terminal');

      const devCol = useBoardStore.getState().columns.find((c) => c.id === 'dev');
      expect(devCol?.icon).toBe('Terminal');
      expect(mockInvoke).toHaveBeenCalledWith('set_agent_persona_icon', {
        role: 'cvh-dev',
        icon: 'Terminal',
      });
    });
  });

  describe('Rules & Hooks CRUD Lifecycle', () => {
    it('createCortexFile creates a new hook file with moment template and opens it in editor', async () => {
      mockInvoke.mockResolvedValue(true);

      await useCortexStore.getState().createCortexFile('dev', 'lint-check', 'hook', {
        hookMoment: 'on_enter',
        hookExt: 'bat',
      });

      const stage = useCortexStore.getState().stages.find((s) => s.stageId === 'dev');
      const hookFile = stage?.files.find((f) => f.name === 'on_enter-lint-check.bat');

      expect(hookFile).toBeDefined();
      expect(hookFile?.type).toBe('hook');
      expect(hookFile?.content).toContain('@echo off');

      const activeFile = useCortexStore.getState().activeFile;
      expect(activeFile?.name).toBe('on_enter-lint-check.bat');
      expect(useCortexStore.getState().isEditorOpen).toBe(true);
    });

    it('createCortexFile creates a new rule file with template and opens it in editor', async () => {
      mockInvoke.mockResolvedValue(true);

      await useCortexStore.getState().createCortexFile('dev', 'coding-standards', 'rule');

      const stage = useCortexStore.getState().stages.find((s) => s.stageId === 'dev');
      const createdFile = stage?.files.find((f) => f.name === 'coding-standards.md');

      expect(createdFile).toBeDefined();
      expect(createdFile?.type).toBe('rule');
      expect(createdFile?.content).toContain('# Правило: coding-standards');

      const activeFile = useCortexStore.getState().activeFile;
      expect(activeFile?.name).toBe('coding-standards.md');
      expect(useCortexStore.getState().isEditorOpen).toBe(true);
      expect(mockInvoke).toHaveBeenCalledWith(
        'save_cortex_file',
        expect.objectContaining({ path: expect.stringContaining('coding-standards.md') })
      );
    });

    it('createCortexFile rejects invalid rule file names with non-alphanumeric chars', async () => {
      await expect(
        useCortexStore.getState().createCortexFile('dev', 'invalid name with spaces!', 'rule')
      ).rejects.toThrow('Только латиница, цифры, - и _');
    });

    it('createCortexFile rejects duplicate file names in the same stage', async () => {
      mockInvoke.mockResolvedValue(true);

      await expect(
        useCortexStore.getState().createCortexFile('dev', 'tools', 'rule')
      ).rejects.toThrow('уже есть в стадии');
    });

    it('updateActiveFileContent marks the active file as dirty and updates content', () => {
      const devStage = useCortexStore.getState().stages.find((s) => s.stageId === 'dev');
      const file = devStage?.files[0];
      expect(file).toBeDefined();

      useCortexStore.setState({ activeFile: { ...file!, isDirty: false }, isEditorOpen: true });

      useCortexStore.getState().updateActiveFileContent('New dirty content');

      const active = useCortexStore.getState().activeFile;
      expect(active?.content).toBe('New dirty content');
      expect(active?.isDirty).toBe(true);
    });

    it('saveRules commits active file content and clears isDirty', async () => {
      mockInvoke.mockResolvedValue(true);

      const devStage = useCortexStore.getState().stages.find((s) => s.stageId === 'dev');
      const file = devStage?.files[0];
      expect(file).toBeDefined();

      useCortexStore.setState({
        activeFile: { ...file!, content: 'Updated rule specification', isDirty: true },
        isEditorOpen: true,
      });

      await useCortexStore.getState().saveRules();

      const active = useCortexStore.getState().activeFile;
      expect(active?.isDirty).toBe(false);
      expect(useCortexStore.getState().isSaving).toBe(false);
      expect(useCortexStore.getState().saveFeedback).toContain('сохранен');

      const savedStageFile = useCortexStore
        .getState()
        .stages.find((s) => s.stageId === 'dev')
        ?.files.find((f) => f.id === file?.id);
      expect(savedStageFile?.content).toBe('Updated rule specification');
      expect(savedStageFile?.isDirty).toBe(false);
    });

    it('reorderRules sorts rules according to provided order while preserving canonical layout', () => {
      useCortexStore.getState().reorderRules('dev', ['boards.md', 'tools.md']);

      const devStage = useCortexStore.getState().stages.find((s) => s.stageId === 'dev');
      const files = devStage?.files ?? [];

      const personaIndex = files.findIndex((f) => f.type === 'persona');
      const boardsRuleIndex = files.findIndex((f) => f.name === 'boards.md');
      const toolsRuleIndex = files.findIndex((f) => f.name === 'tools.md');
      const settingsIndex = files.findIndex((f) => f.type === 'settings');

      // Canonical ordering: persona -> rules (boards before tools) -> settings
      expect(personaIndex).toBe(0);
      expect(boardsRuleIndex).toBeLessThan(toolsRuleIndex);
      expect(settingsIndex).toBe(files.length - 1);
    });

    it('deleteRule removes rule from stage files and closes editor if active file was deleted', async () => {
      const devStage = useCortexStore.getState().stages.find((s) => s.stageId === 'dev');
      const targetRule = devStage?.files.find((f) => f.name === 'tools.md');
      expect(targetRule).toBeDefined();

      useCortexStore.setState({ activeFile: targetRule!, isEditorOpen: true });

      await useCortexStore.getState().deleteRule('dev', 'tools.md');

      const updatedDevStage = useCortexStore.getState().stages.find((s) => s.stageId === 'dev');
      expect(updatedDevStage?.files.some((f) => f.name === 'tools.md')).toBe(false);
      expect(useCortexStore.getState().activeFile).toBeNull();
      expect(useCortexStore.getState().isEditorOpen).toBe(false);
    });
  });

  describe('File Open & Editor Teardown Flow', () => {
    it('openFile opens builtin hook with structured description', async () => {
      const builtinStage = {
        stageId: 'backlog',
        stageTitle: 'Backlog',
        color: '#64748b',
        icon: 'Inbox',
        personaName: 'system',
        isExpanded: true,
        files: [
          {
            id: 'hook-worktree',
            name: 'create_worktree',
            stageId: 'backlog',
            type: 'hook' as const,
            path: 'builtin:create_worktree',
            content: '',
            isDirty: false,
          },
        ],
      };

      useCortexStore.setState({ stages: [builtinStage] });

      await useCortexStore.getState().openFile('hook-worktree');

      const active = useCortexStore.getState().activeFile;
      expect(active?.id).toBe('hook-worktree');
      expect(active?.content).toContain('Встроенный хук');
      expect(useCortexStore.getState().isEditorOpen).toBe(true);
    });

    it('closeEditor clears activeFile and sets isEditorOpen to false', () => {
      const file = useCortexStore.getState().stages[1].files[0];
      useCortexStore.setState({ activeFile: file, isEditorOpen: true });

      useCortexStore.getState().closeEditor();

      expect(useCortexStore.getState().activeFile).toBeNull();
      expect(useCortexStore.getState().isEditorOpen).toBe(false);
    });
  });
});

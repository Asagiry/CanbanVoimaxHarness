import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  useToolPolicyStore,
  AVAILABLE_TOOLS,
  getToolTranslationKey,
  TOOL_PARAM_DEFS,
  DEFAULT_TOOL_SETTINGS,
  DEFAULT_OVERRIDES,
} from '../toolPolicyStore';
import { useBoardStore } from '../boardStore';

const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('toolPolicyStore (Modular Tool Policy Engine)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue({});

    useBoardStore.setState({
      columns: [
        { id: 'col-backlog', title: 'Backlog', color: '#64748b', icon: 'Inbox', persona: 'planner', modelOverride: null },
        { id: 'col-dev', title: 'Development', color: '#6366f1', icon: 'Bot', persona: 'dev', modelOverride: null },
        { id: 'col-review', title: 'Review', color: '#f59e0b', icon: 'CheckSquare', persona: 'review', modelOverride: null },
        { id: 'col-retro', title: 'Retro', color: '#8b5cf6', icon: 'RotateCcw', persona: 'retro', modelOverride: null },
      ],
      tasks: [],
    });

    useToolPolicyStore.setState({
      isModalOpen: false,
      stageOverrides: JSON.parse(JSON.stringify(DEFAULT_OVERRIDES)),
      toolSettings: JSON.parse(JSON.stringify(DEFAULT_TOOL_SETTINGS)),
      toolDescriptions: {},
      toolCustomSchemas: {},
    });
  });

  describe('AVAILABLE_TOOLS & Metadata', () => {
    it('contains exactly 22 tools with unique IDs', () => {
      expect(AVAILABLE_TOOLS).toHaveLength(22);
      const ids = AVAILABLE_TOOLS.map((t) => t.id);
      const uniqueIds = new Set(ids);
      expect(uniqueIds.size).toBe(22);
    });

    it('defines parameter bounds for core tools in TOOL_PARAM_DEFS', () => {
      expect(TOOL_PARAM_DEFS['read_file']).toBeDefined();
      expect(TOOL_PARAM_DEFS['subagent']).toBeDefined();
      expect(TOOL_PARAM_DEFS['subagent_bg']).toBeDefined();
      expect(TOOL_PARAM_DEFS['todo_write']).toBeDefined();
      expect(TOOL_PARAM_DEFS['execute_command']).toBeDefined();
      expect(TOOL_PARAM_DEFS['execute_command'][0].key).toBe('timeout_sec');
      expect(TOOL_PARAM_DEFS['execute_command'][0].min).toBe(10);
    });

    it('all tools have valid metadata contracts', () => {
      for (const tool of AVAILABLE_TOOLS) {
        expect(tool.id).toBeTruthy();
        expect(tool.name).toBeTruthy();
        expect(tool.label).toBeTruthy();
        expect(tool.description).toBeTruthy();
        expect(tool.tokenWeight).toBeGreaterThan(0);
        expect(['system', 'fs', 'web', 'agent', 'workflow', 'interactive', 'orchestration']).toContain(tool.category);
      }
    });

    it('subagent tools have proper master and parent hierarchy', () => {
      const master = AVAILABLE_TOOLS.find((t) => t.id === 'subagent');
      expect(master?.isSubagentMaster).toBe(true);

      const children = ['subagent_sync', 'subagent_bg', 'collect_subagent', 'subagent_list'];
      for (const childId of children) {
        const child = AVAILABLE_TOOLS.find((t) => t.id === childId);
        expect(child?.parentTool).toBe('subagent');
      }
    });

    it('getToolTranslationKey returns expected i18n keys', () => {
      expect(getToolTranslationKey('execute_command', 'label')).toBe('tools.execute_command.label');
      expect(getToolTranslationKey('read_file', 'description')).toBe('tools.read_file.description');
    });
  });

  describe('Hierarchical Permission Engine (isToolAllowed)', () => {
    it('respects direct stage overrides', () => {
      const store = useToolPolicyStore.getState();
      expect(store.isToolAllowed('execute_command', 'dev')).toBe(true);

      store.setStageTool('dev', 'execute_command', false);
      expect(useToolPolicyStore.getState().isToolAllowed('execute_command', 'dev')).toBe(false);
    });

    it('enforces subagent master disabling all child subagent tools', () => {
      const store = useToolPolicyStore.getState();
      // By default dev has subagent: true
      expect(store.isToolAllowed('subagent_bg', 'dev')).toBe(true);
      expect(store.isToolAllowed('subagent_sync', 'dev')).toBe(true);
      expect(store.isToolAllowed('collect_subagent', 'dev')).toBe(true);
      expect(store.isToolAllowed('subagent_list', 'dev')).toBe(true);

      // Disable master subagent tool for dev
      store.setStageTool('dev', 'subagent', false);

      const updated = useToolPolicyStore.getState();
      expect(updated.isToolAllowed('subagent_bg', 'dev')).toBe(false);
      expect(updated.isToolAllowed('subagent_sync', 'dev')).toBe(false);
      expect(updated.isToolAllowed('collect_subagent', 'dev')).toBe(false);
      expect(updated.isToolAllowed('subagent_list', 'dev')).toBe(false);
    });

    it('allows child tool if master is enabled and child override is explicit', () => {
      const store = useToolPolicyStore.getState();
      store.setStageTool('dev', 'subagent', true);
      store.setStageTool('dev', 'subagent_bg', false);

      const updated = useToolPolicyStore.getState();
      expect(updated.isToolAllowed('subagent', 'dev')).toBe(true);
      expect(updated.isToolAllowed('subagent_bg', 'dev')).toBe(false);
      expect(updated.isToolAllowed('subagent_sync', 'dev')).toBe(true);
    });

    it('enforces Stage 0 rules for Planner (execute_command and create_task allowed)', () => {
      const store = useToolPolicyStore.getState();
      expect(store.isToolAllowed('execute_command', 'planner')).toBe(true);
      expect(store.isToolAllowed('create_task', 'planner')).toBe(true);
      expect(store.isToolAllowed('edit_file', 'planner')).toBe(false);
      expect(store.isToolAllowed('write_file', 'planner')).toBe(false);

      // Same for 'plan' stage alias
      expect(store.isToolAllowed('execute_command', 'plan')).toBe(true);
      expect(store.isToolAllowed('create_task', 'plan')).toBe(true);
    });

    it('enforces Stage 0 rules for Retro (edit_file and write_file allowed)', () => {
      const store = useToolPolicyStore.getState();
      expect(store.isToolAllowed('edit_file', 'retro')).toBe(true);
      expect(store.isToolAllowed('write_file', 'retro')).toBe(true);
      expect(store.isToolAllowed('execute_command', 'retro')).toBe(true);
    });

    it('resolves permissions via board column persona mapping', () => {
      const store = useToolPolicyStore.getState();
      // col-retro has persona 'retro', where edit_file is allowed in DEFAULT_OVERRIDES
      expect(store.isToolAllowed('edit_file', 'col-retro')).toBe(true);

      // Set override on persona 'retro', check query by column id 'col-retro'
      store.setStageTool('retro', 'edit_file', false);
      expect(useToolPolicyStore.getState().isToolAllowed('edit_file', 'col-retro')).toBe(false);
    });

    it('returns true when stage is undefined or unknown without overrides', () => {
      const store = useToolPolicyStore.getState();
      expect(store.isToolAllowed('read_file')).toBe(true);
      expect(store.isToolAllowed('read_file', 'non_existent_stage')).toBe(true);
    });
  });

  describe('Token Calculation & Settings Management', () => {
    it('calculates saved tokens based on disabled tools', () => {
      const store = useToolPolicyStore.getState();
      // In review stage, edit_file (310), write_file (260), create_task (260), complete_task (220) are disabled
      const tokensSaved = store.calculateSavedTokens('review');
      expect(tokensSaved).toBeGreaterThan(0);
      expect(tokensSaved).toBe(310 + 260 + 260 + 220); // 1050 tokens
    });

    it('returns fallback floor (2400) when all tools are allowed', () => {
      const store = useToolPolicyStore.getState();
      // Non-existent stage has 0 disabled tools
      const tokensSaved = store.calculateSavedTokens('free_stage');
      expect(tokensSaved).toBe(2400);
    });

    it('manages tool settings with defaults and updates', async () => {
      const store = useToolPolicyStore.getState();
      expect(store.getToolSetting('execute_command', 'timeout_sec', 60)).toBe(180);
      expect(store.getToolSetting('execute_command', 'non_existing', 42)).toBe(42);

      store.setToolSetting('execute_command', 'timeout_sec', 300);
      expect(useToolPolicyStore.getState().getToolSetting('execute_command', 'timeout_sec', 60)).toBe(300);
      await vi.waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('save_tool_policies', expect.any(Object));
      });
    });

    it('manages custom tool descriptions and schemas', () => {
      const store = useToolPolicyStore.getState();

      store.setToolDescription('read_file', 'Custom read description');
      expect(useToolPolicyStore.getState().toolDescriptions['read_file']).toBe('Custom read description');

      store.resetToolDescription('read_file');
      expect(useToolPolicyStore.getState().toolDescriptions['read_file']).toBeUndefined();

      store.setToolCustomSchema('glob', { type: 'object' });
      expect(useToolPolicyStore.getState().toolCustomSchemas['glob']).toEqual({ type: 'object' });

      store.resetToolCustomSchema('glob');
      expect(useToolPolicyStore.getState().toolCustomSchemas['glob']).toBeUndefined();
    });

    it('toggles modal state correctly', () => {
      const store = useToolPolicyStore.getState();
      expect(store.isModalOpen).toBe(false);

      store.openModal();
      expect(useToolPolicyStore.getState().isModalOpen).toBe(true);

      store.closeModal();
      expect(useToolPolicyStore.getState().isModalOpen).toBe(false);
    });

    it('resets to defaults cleanly', () => {
      const store = useToolPolicyStore.getState();
      store.setStageTool('dev', 'read_file', false);
      store.setToolSetting('read_file', 'max_lines', 9999);
      store.setToolDescription('read_file', 'test desc');

      store.resetDefaults();
      const reset = useToolPolicyStore.getState();
      expect(reset.stageOverrides).toEqual(DEFAULT_OVERRIDES);
      expect(reset.toolSettings).toEqual(DEFAULT_TOOL_SETTINGS);
      expect(reset.toolDescriptions).toEqual({});
      expect(reset.toolCustomSchemas).toEqual({});
    });
  });

  describe('Tauri IPC Persistence (loadFromTauri & saveToTauri)', () => {
    it('loadFromTauri merges backend policies with defaults', async () => {
      mockInvoke.mockResolvedValueOnce({
        stageOverrides: { custom_stage: { read_file: false } },
        toolSettings: { read_file: { max_lines: 500 } },
        toolDescriptions: { read_file: 'Backend desc' },
        toolCustomSchemas: { read_file: { type: 'object' } },
      });

      await useToolPolicyStore.getState().loadFromTauri();
      const state = useToolPolicyStore.getState();

      expect(state.stageOverrides['custom_stage']?.['read_file']).toBe(false);
      expect(state.toolSettings['read_file']?.['max_lines']).toBe(500);
      expect(state.toolDescriptions['read_file']).toBe('Backend desc');
      expect(state.toolCustomSchemas['read_file']).toEqual({ type: 'object' });
    });

    it('loadFromTauri gracefully catches errors and retains state', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('Tauri offline'));
      await expect(useToolPolicyStore.getState().loadFromTauri()).resolves.not.toThrow();
    });

    it('saveToTauri invokes backend save_tool_policies payload', async () => {
      await useToolPolicyStore.getState().saveToTauri();
      expect(mockInvoke).toHaveBeenCalledWith('save_tool_policies', {
        policies: expect.objectContaining({
          stageOverrides: expect.any(Object),
          toolSettings: expect.any(Object),
          toolDescriptions: expect.any(Object),
          toolCustomSchemas: expect.any(Object),
        }),
      });
    });

    it('drops unmodified default schemas on setToolCustomSchema and loadFromTauri', async () => {
      const { AVAILABLE_TOOLS, buildDefaultToolSchema, isToolSchemaCustomized } = await import('../toolPolicyStore');
      const advanceTool = AVAILABLE_TOOLS.find((t) => t.id === 'advance_task')!;
      const defSchema = buildDefaultToolSchema(advanceTool);

      mockInvoke.mockResolvedValueOnce({
        toolCustomSchemas: {
          advance_task: defSchema,
          glob: { type: 'object' },
        },
      });
      await useToolPolicyStore.getState().loadFromTauri();
      expect(useToolPolicyStore.getState().toolCustomSchemas['advance_task']).toBeUndefined();
      expect(useToolPolicyStore.getState().toolCustomSchemas['glob']).toEqual({ type: 'object' });

      expect(isToolSchemaCustomized(advanceTool, defSchema)).toBe(false);
      useToolPolicyStore.getState().setToolCustomSchema('advance_task', defSchema);
      expect(useToolPolicyStore.getState().toolCustomSchemas['advance_task']).toBeUndefined();
    });
  });
});

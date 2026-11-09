import { StateCreator } from 'zustand';
import { CortexState } from './types';
import { CortexAgentFile } from '../../types';
import { DEFAULT_CORTEX_STAGES } from './cortexConstants';
import { isAllowedAgentColor } from '../../lib/agentPalette';
import { useBoardStore } from '../boardStore';
import {
  loadAgentPersonasAction,
  syncStagesFromColumnsAction,
} from './cortexSync';
import {
  deleteGlobalRuleAction,
  createGlobalFolderAction,
  deleteGlobalFolderAction,
  loadGlobalRulesAction,
  openFileAction,
  reorderFilesInStage,
  saveRulesAction,
} from './cortexFileOps';
import {
  createCortexFileAction,
  createGlobalRuleAction,
  createPersonaTemplateAction,
  createSettingTemplateAction,
} from './cortexTemplates';
import {
  markPendingColumnChangeAction,
  markPendingDeleteAction,
  refreshCvhGitStatusAction,
  restorePendingDeleteAction,
  saveCvhToGitAction,
} from './cortexGitSave';

export * from './cortexSync';
export * from './cortexFileOps';
export * from './cortexTemplates';
export * from './cortexGitSave';

export const createCortexSlice: StateCreator<
  CortexState,
  [],
  [],
  CortexState
> = (set, get) => ({
  // --- INITIAL STATE ---
  stages: DEFAULT_CORTEX_STAGES,
  agentStages: [],
  agentPersonas: [],
  globalRules: [],
  activeFile: null,
  focusedStageId: null,
  isEditorOpen: false,
  isSaving: false,
  saveFeedback: null,
  fileMissing: {},
  cvhGitStatus: null,
  cvhGitSaving: false,
  cvhGitError: null,
  pendingDeletes: {},
  pendingColumnChanges: {},

  // --- STAGE EXPANSION & NAVIGATION ---
  toggleStageExpand: (stageId) => {
    set((state) => ({
      stages: state.stages.map((st) =>
        st.stageId === stageId ? { ...st, isExpanded: !st.isExpanded } : st
      ),
      agentStages: state.agentStages.map((st) =>
        st.stageId === stageId ? { ...st, isExpanded: !st.isExpanded } : st
      ),
    }));
  },

  expandStage: (stageId) => {
    set((state) => ({
      stages: state.stages.map((st) =>
        st.stageId === stageId ? { ...st, isExpanded: true } : st
      ),
      agentStages: state.agentStages.map((st) =>
        st.stageId === stageId ? { ...st, isExpanded: true } : st
      ),
    }));
  },

  focusStage: (stageId) => {
    set((state) => ({
      focusedStageId: stageId,
      stages: state.stages.map((st) =>
        st.stageId === stageId ? { ...st, isExpanded: true } : st
      ),
      agentStages: state.agentStages.map((st) =>
        st.stageId === stageId ? { ...st, isExpanded: true } : st
      ),
    }));

    setTimeout(() => {
      if (get().focusedStageId === stageId) {
        set({ focusedStageId: null });
      }
    }, 2500);
  },

  // --- COLUMN & STAGE SYNCHRONIZATION ---
  syncStagesFromStages: (stages) => get().syncStagesFromColumns(stages),
  syncStagesFromColumns: (columns) => syncStagesFromColumnsAction(set, get, columns),

  // --- AGENT PERSONAS ---
  loadAgentPersonas: () => loadAgentPersonasAction(set, get),
  syncAgentPersonas: async () => {
    await get().loadAgentPersonas();
  },

  setAgentPersonaColor: async (role: string, color: string) => {
    if (!isAllowedAgentColor(color)) {
      throw new Error('Серый и зеленый цвета зарезервированы за Backlog и Ready и запрещены для агентов');
    }
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('set_agent_persona_color', { role, color });
    } catch (err) {
      console.info('set_agent_persona_color IPC not available or failed:', err);
    }
    set((state) => ({
      agentPersonas: state.agentPersonas.map((p) =>
        p.role === role ? { ...p, color } : p
      ),
      agentStages: state.agentStages.map((st) =>
        st.personaName === role ? { ...st, color } : st
      ),
    }));

    const boardStore = useBoardStore.getState();
    const prevCols = boardStore.columns;
    let changed = false;
    const nextCols = prevCols.map((c) => {
      if (c.persona === role && c.id !== 'backlog' && c.id !== 'ready') {
        changed = true;
        return { ...c, color };
      }
      return c;
    });
    if (changed) {
      boardStore.setColumns(nextCols);
      get().syncStagesFromColumns(nextCols);
      void boardStore.saveCurrentBoard(nextCols);
    } else {
      get().syncStagesFromColumns(prevCols);
    }
    void get().refreshCvhGitStatus();
  },

  setAgentPersonaIcon: async (role: string, icon: string) => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('set_agent_persona_icon', { role, icon });
    } catch (err) {
      console.info('set_agent_persona_icon IPC not available or failed:', err);
    }
    set((state) => ({
      agentPersonas: state.agentPersonas.map((p) =>
        p.role === role ? { ...p, icon } : p
      ),
      agentStages: state.agentStages.map((st) =>
        st.personaName === role ? { ...st, icon } : st
      ),
    }));

    const boardStore = useBoardStore.getState();
    const prevCols = boardStore.columns;
    let changed = false;
    const nextCols = prevCols.map((c) => {
      if (c.persona === role) {
        changed = true;
        return { ...c, icon };
      }
      return c;
    });
    if (changed) {
      boardStore.setColumns(nextCols);
      get().syncStagesFromColumns(nextCols);
      void boardStore.saveCurrentBoard(nextCols);
    } else {
      get().syncStagesFromColumns(prevCols);
    }
    void get().refreshCvhGitStatus();
  },

  renameAgentPersona: async (oldRole: string, newRole: string) => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('rename_agent_persona', { oldRole, newRole });
    } catch (err) {
      console.info('rename_agent_persona IPC not available or failed:', err);
      throw err;
    }
    await get().loadAgentPersonas();
    try {
      await useBoardStore.getState().loadBoards();
    } catch (err) {
      console.info('Failed to reload boards after agent rename:', err);
    }
    void get().refreshCvhGitStatus();
  },

  deleteAgentPersona: async (role: string) => {
    const key = `agent:${role}`;
    set((state) => ({
      pendingDeletes: { ...state.pendingDeletes, [key]: { path: `.cvh/agents/${role}`, kind: 'agent' as const, role } },
      agentStages: state.agentStages.map((st) => (st.personaName === role ? { ...st, isDeleted: true } : st)),
      stages: state.stages.map((st) => (st.personaName === role ? { ...st, isDeleted: true } : st)),
    }));
    void get().refreshCvhGitStatus();
  },

  // --- GLOBAL RULES ---
  loadGlobalRules: () => loadGlobalRulesAction(set, get),
  refreshCvhGitStatus: () => refreshCvhGitStatusAction(set),
  saveCvhToGit: (message) => saveCvhToGitAction(set, get, message),
  markPendingDelete: (key, entry) => markPendingDeleteAction(set, key, entry),
  restorePendingDelete: (key) => restorePendingDeleteAction(set, key),
  markPendingColumnChange: (stageId) => markPendingColumnChangeAction(set, stageId),
  clearCvhGitError: () => set({ cvhGitError: null }),

  // --- FILE OPEN & TEMPLATE CREATION ---
  openFile: (fileId) => openFileAction(set, get, fileId),
  createPersonaTemplate: (fileId) => createPersonaTemplateAction(set, get, fileId),
  createSettingTemplate: (fileId) => createSettingTemplateAction(set, get, fileId),

  closeEditor: () => {
    set({
      isEditorOpen: false,
      activeFile: null,
      saveFeedback: null,
    });
  },

  // --- RULES & FILES CRUD ---
  createCortexFile: (stageId, rawName, kind, opts) =>
    createCortexFileAction(set, get, stageId, rawName, kind, opts),
  createGlobalRule: (rawName, folder) => createGlobalRuleAction(set, get, rawName, folder),

  updateActiveFileContent: (content) => {
    set((state) => {
      if (!state.activeFile) return state;
      return {
        activeFile: {
          ...state.activeFile,
          content,
          isDirty: true,
        },
      };
    });
  },

  saveRules: () => saveRulesAction(set, get),

  deleteRule: async (stageId: string, ruleName: string) => {
    if (stageId === 'global') {
      return get().deleteGlobalRule(ruleName);
    }

    set((state) => {
      const markFiles = (files: CortexAgentFile[]) =>
        files.map((f) =>
          f.type === 'rule' && (f.name === ruleName || f.id === ruleName) ? { ...f, isDeleted: true } : f,
        );

      const nextStages = state.stages.map((st) =>
        st.stageId === stageId ? { ...st, files: markFiles(st.files) } : st
      );
      const nextAgentStages = state.agentStages.map((st) =>
        st.stageId === stageId ? { ...st, files: markFiles(st.files) } : st
      );

      const target =
        state.stages.find((st) => st.stageId === stageId)?.files.find(
          (f) => f.type === 'rule' && (f.name === ruleName || f.id === ruleName),
        ) ??
        state.agentStages.find((st) => st.stageId === stageId)?.files.find(
          (f) => f.type === 'rule' && (f.name === ruleName || f.id === ruleName),
        );
      const key = target ? `file:${target.id}` : `file:${stageId}:${ruleName}`;
      const pendingDeletes = target
        ? { ...state.pendingDeletes, [key]: { path: target.path, kind: 'rule' as const, stageId, fileId: target.id } }
        : state.pendingDeletes;

      const isActiveDeleted =
        state.activeFile?.stageId === stageId &&
        state.activeFile?.type === 'rule' &&
        (state.activeFile.name === ruleName || state.activeFile.id === ruleName);

      return {
        stages: nextStages,
        agentStages: nextAgentStages,
        pendingDeletes,
        activeFile: isActiveDeleted ? null : state.activeFile,
        isEditorOpen: isActiveDeleted ? false : state.isEditorOpen,
      };
    });
  },

  deleteGlobalRule: (fileName) => deleteGlobalRuleAction(set, get, fileName),
  createGlobalFolder: (folderName) => createGlobalFolderAction(set, get, folderName),
  deleteGlobalFolder: (folderName) => deleteGlobalFolderAction(set, get, folderName),

  reorderRules: (stageId: string, ruleNames: string[]) => {
    set((state) => ({
      stages: state.stages.map((st) =>
        st.stageId === stageId ? { ...st, files: reorderFilesInStage(st.files, ruleNames) } : st
      ),
      agentStages: state.agentStages.map((st) =>
        st.stageId === stageId ? { ...st, files: reorderFilesInStage(st.files, ruleNames) } : st
      ),
    }));
  },
});

// Backward-compatible slice creators for zero regressions
export const createCortexFilesSlice = createCortexSlice;
export const createPersonasSlice = createCortexSlice;
export const createRulesSlice = createCortexSlice;

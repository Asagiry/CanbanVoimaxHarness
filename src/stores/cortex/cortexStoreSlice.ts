import { StateCreator } from 'zustand';
import {
  CortexHookExt,
  CortexHookMoment,
  CortexState,
  RawAgentPersona,
  AgentPersonaInfo,
} from './types';
import { CortexAgentFile, CortexAgentStage } from '../../types';
import { DEFAULT_CORTEX_STAGES } from './cortexConstants';
import {
  buildBuiltinHookDescription,
  buildCommandHookDescription,
  buildGlobalRuleTemplate,
  buildHookTemplate,
  buildPersonaTemplate,
  buildRuleTemplate,
  buildScriptTemplate,
  buildSettingTemplate,
  cortexAgentDir,
  extractScriptPath,
  normalizeCortexFileName,
} from './cortexHelpers';
import {
  BACKLOG_COLOR,
  READY_COLOR,
  getDeterministicAgentColor,
  isAllowedAgentColor,
} from '../../lib/agentPalette';
import { getDefaultColumnIcon } from '../../lib/columnIcons';
import { useBoardStore } from '../boardStore';

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

  // --- STAGE EXPANSION & NAVIGATION ---
  toggleStageExpand: (stageId) => {
    // Один флоу на обе секции: id колонок и агентов (`agent:<role>`) не пересекаются.
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

    // Reset focused highlight after 2.5s
    setTimeout(() => {
      if (get().focusedStageId === stageId) {
        set({ focusedStageId: null });
      }
    }, 2500);
  },

  // --- COLUMN & STAGE SYNCHRONIZATION ---
  syncStagesFromColumns: (columns) => {
    const prevStages = get().stages;
    const prevStageById = new Map(prevStages.map((st) => [st.stageId, st]));
    const prevFileById = new Map(prevStages.flatMap((st) => st.files.map((f) => [f.id, f])));
    // Ранее созданные файлы (правила/персоны) по stageId — пережить пересборку стадий.
    const prevExtrasByStage = new Map<string, CortexAgentFile[]>();
    for (const st of prevStages) {
      for (const f of st.files) {
        const arr = prevExtrasByStage.get(f.stageId) ?? [];
        arr.push(f);
        prevExtrasByStage.set(f.stageId, arr);
      }
    }

    const boardStages: CortexAgentStage[] = [];
    for (const col of columns) {
      const persona = col.persona?.trim() ?? '';
      const isSystemNode =
        persona === '' &&
        (col.id === 'backlog' || col.id === 'ready' || col.terminal === true);
      const dir = cortexAgentDir(col.persona ?? '', col.id);
      if (isSystemNode) {
        // Хуки колонки (on_enter/on_leave) как узлы типа hook.
        const hookMoments = Object.entries(col.hooks ?? {})
          .filter(([moment]) => moment === 'on_enter' || moment === 'on_leave')
          .sort(([a], [b]) => a.localeCompare(b));
        const hookFiles = hookMoments.map(([moment, target]) => {
          const hookId = `${col.id}-hook-${moment}`;
          const prev = prevFileById.get(hookId);
          const targetPath = typeof target === 'string' ? target : target?.script || (target as any)?.path || '';
          return {
            id: hookId,
            name: moment,
            stageId: col.id,
            type: 'hook' as const,
            path: targetPath,
            content: prev?.content ?? '',
            isDirty: false,
          };
        });
        const extras = (prevExtrasByStage.get(col.id) ?? [])
          .filter(
            (f) =>
              !hookFiles.some((h) => h.id === f.id) &&
              (f.type === 'rule' || f.type === 'hook') &&
              f.path.startsWith(`${dir}/`),
          )
          .sort((a, b) => {
            const order = (t: string) => (t === 'rule' ? 0 : 1);
            return order(a.type) - order(b.type) || a.name.localeCompare(b.name);
          });
        boardStages.push({
          stageId: col.id,
          stageTitle: col.title,
          color: col.color,
          icon: col.icon,
          personaName: 'система',
          isExpanded: prevStageById.get(col.id)?.isExpanded ?? true,
          files: [...hookFiles, ...extras],
        });
        continue;
      }
      const personaId = `${col.id}-persona`;
      const settingsId = `${col.id}-settings`;
      const prevPersona = prevFileById.get(personaId);
      const prevSettings = prevFileById.get(settingsId);
      // Хуки колонки (on_enter/on_leave) как узлы типа hook
      const hookMoments = Object.entries(col.hooks ?? {})
        .filter(([moment]) => moment === 'on_enter' || moment === 'on_leave' || moment === 'on_finish' || moment === 'on_exit')
        .sort(([a], [b]) => a.localeCompare(b));
      const hookFiles = hookMoments.map(([moment, target]) => {
        const hookId = `${col.id}-hook-${moment}`;
        const prev = prevFileById.get(hookId);
        const targetPath = typeof target === 'string' ? target : target?.script || (target as any)?.path || '';
        return {
          id: hookId,
          name: moment,
          stageId: col.id,
          type: 'hook' as const,
          path: targetPath,
          content: prev?.content ?? '',
          isDirty: false,
        };
      });
      const extras = (prevExtrasByStage.get(col.id) ?? [])
        .filter(
          (f) =>
            f.id !== personaId &&
            f.id !== settingsId &&
            !hookFiles.some((h) => h.id === f.id) &&
            (f.type === 'rule' || f.type === 'hook') &&
            (f.path.startsWith(`${dir}/`) || f.type === 'hook' || !persona),
        )
        .sort((a, b) => {
          const order = (t: string) => (t === 'rule' ? 0 : 1);
          return order(a.type) - order(b.type) || a.name.localeCompare(b.name);
        });
      const personaInfo = get().agentPersonas.find((ap) => ap.role === col.persona?.trim());
      const personaFileName = personaInfo?.personaFile ?? 'persona.md';

      const diskRules = (personaInfo?.rules ?? []).map((ruleName) => {
        const ruleId = `${col.id}-rule-${ruleName}`;
        const prevRule = prevFileById.get(ruleId);
        return {
          id: ruleId,
          name: ruleName,
          stageId: col.id,
          type: 'rule' as const,
          path: `${dir}/rules/${ruleName}`,
          content: prevRule?.content ?? '',
          isDirty: prevRule?.isDirty ?? false,
        };
      });

      const inMemoryExtras = extras.filter(
        (f) => f.type !== 'rule' || !diskRules.some((dr) => dr.name === f.name),
      );

      const allExtras = [...diskRules, ...inMemoryExtras].sort((a, b) => {
        const order = (t: string) => (t === 'rule' ? 0 : 1);
        return order(a.type) - order(b.type) || a.name.localeCompare(b.name);
      });

      // Четкая стилизация Backlog (#64748b) и Ready (#10b981) + реактивная иконка и цвет персоны
      let stageColor = col.color;
      let stageIcon = col.icon;
      if (col.id === 'backlog') {
        stageColor = BACKLOG_COLOR;
        stageIcon = col.icon || getDefaultColumnIcon('backlog');
      } else if (col.id === 'ready') {
        stageColor = READY_COLOR;
        stageIcon = col.icon || getDefaultColumnIcon('ready');
      } else {
        if (col.persona) {
          if (personaInfo?.icon) {
            stageIcon = personaInfo.icon;
          } else if (!stageIcon) {
            stageIcon = getDefaultColumnIcon(col.id);
          }
          if (personaInfo?.color && isAllowedAgentColor(personaInfo.color)) {
            stageColor = personaInfo.color;
          } else if (col.color && isAllowedAgentColor(col.color)) {
            stageColor = col.color;
          } else {
            stageColor = getDeterministicAgentColor(col.persona);
          }
        } else {
          stageIcon = col.icon || getDefaultColumnIcon(col.id);
          stageColor = isAllowedAgentColor(col.color) ? col.color : '#171717';
        }
      }

      boardStages.push({
        stageId: col.id,
        stageTitle: col.title,
        color: stageColor,
        icon: stageIcon,
        personaName: col.persona?.trim() || 'без агента',
        isExpanded: prevStageById.get(col.id)?.isExpanded ?? true,
        files: [
          {
            id: personaId,
            name: personaFileName,
            stageId: col.id,
            type: 'persona',
            path: `${dir}/${personaFileName}`,
            content: prevPersona?.content ?? '',
            isDirty: prevPersona?.isDirty ?? false,
          },
          ...hookFiles,
          ...allExtras,
          {
            id: settingsId,
            name: 'setting',
            stageId: col.id,
            type: 'settings',
            path: `${dir}/setting`,
            content: prevSettings?.content ?? '',
            isDirty: prevSettings?.isDirty ?? false,
          },
        ],
      });
    }

    if (boardStages.length === 0) {
      if (columns.length === 0) {
        set({ stages: DEFAULT_CORTEX_STAGES });
        return;
      }
      set((state) => {
        const aliveAgentIds = new Set(
          state.agentStages.flatMap((s) => s.files.map((f) => f.id)),
        );
        const nextMissing: Record<string, boolean> = {};
        for (const [id, value] of Object.entries(state.fileMissing)) {
          if (aliveAgentIds.has(id)) nextMissing[id] = value;
        }
        return { stages: [], fileMissing: nextMissing };
      });
      return;
    }

    const prevMissing = get().fileMissing;
    const prevAgentStages = get().agentStages;
    const nextMissing: Record<string, boolean> = {};
    for (const st of boardStages) {
      for (const f of st.files) {
        if (f.path.startsWith('builtin:')) continue;
        if (prevMissing[f.id]) nextMissing[f.id] = true;
      }
    }
    for (const st of prevAgentStages) {
      for (const f of st.files) {
        if (prevMissing[f.id]) nextMissing[f.id] = true;
      }
    }
    set({ stages: boardStages, fileMissing: nextMissing });
  },

  // --- AGENT PERSONAS ---
  loadAgentPersonas: async () => {
    let raw: RawAgentPersona[] = [];
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const list = await invoke<RawAgentPersona[]>('list_agent_personas');
      if (Array.isArray(list)) raw = list;
    } catch {
      raw = [];
    }
    const seen = new Set<string>();
    const personas: AgentPersonaInfo[] = [];
    for (const item of raw) {
      if (!item || typeof item.role !== 'string') continue;
      const role = item.role.trim();
      if (!role || seen.has(role)) continue;
      seen.add(role);
      const roleColor = isAllowedAgentColor(item.color)
        ? item.color!
        : getDeterministicAgentColor(role);
      personas.push({
        role,
        hasPrompt: item.hasPrompt ?? item.has_prompt ?? true,
        isBuiltin: item.isBuiltin ?? item.is_builtin ?? (role === 'cvh-dev'),
        personaFile: item.personaFile ?? item.persona_file ?? 'persona.md',
        rules: Array.isArray(item.rules) ? item.rules : [],
        hooks: Array.isArray(item.hooks) ? item.hooks : [],
        icon: item.icon,
        color: roleColor,
        contextWindow: item.contextWindow ?? item.context_window,
      });
    }
    personas.sort((a, b) => a.role.localeCompare(b.role));

    const prev = get().agentStages;
    const prevByRole = new Map(prev.map((st) => [st.personaName, st]));
    const nextStages: CortexAgentStage[] = personas.map((p) => {
      const dir = cortexAgentDir(p.role, p.role);
      const stageId = `agent:${p.role}`;
      const prevStage = prevByRole.get(p.role);
      const prevFiles = prevStage?.files ?? [];
      const prevById = new Map(prevFiles.map((f) => [f.id, f]));
      const personaFileName = p.personaFile ?? 'persona.md';
      const personaId = `${stageId}-persona`;
      const settingsId = `${stageId}-settings`;
      const prevPersona = prevById.get(personaId);
      const prevSettings = prevById.get(settingsId);

      const diskRules = (p.rules ?? []).map((ruleName) => {
        const ruleId = `${stageId}-rule-${ruleName}`;
        const prevRule = prevById.get(ruleId);
        return {
          id: ruleId,
          name: ruleName,
          stageId,
          type: 'rule' as const,
          path: `${dir}/rules/${ruleName}`,
          content: prevRule?.content ?? '',
          isDirty: prevRule?.isDirty ?? false,
        };
      });

      const inMemoryExtras = prevFiles.filter(
        (f) =>
          f.id !== personaId &&
          f.id !== settingsId &&
          f.type === 'rule' &&
          !diskRules.some((dr) => dr.name === f.name),
      );

      const allRules = [...diskRules, ...inMemoryExtras].sort((a, b) =>
        a.name.localeCompare(b.name),
      );

      const isBuiltin = p.role === 'cvh-dev' || Boolean(p.isBuiltin);
      return {
        stageId,
        stageTitle: p.role,
        color: p.color || getDeterministicAgentColor(p.role),
        icon: p.icon || (isBuiltin ? 'Cpu' : 'Bot'),
        personaName: p.role,
        isExpanded: prevStage?.isExpanded ?? false,
        isBuiltin,
        files: [
          {
            id: personaId,
            name: personaFileName,
            stageId,
            type: 'persona',
            path: `${dir}/${personaFileName}`,
            content: prevPersona?.content ?? '',
            isDirty: prevPersona?.isDirty ?? false,
          },
          ...allRules,
          {
            id: settingsId,
            name: 'setting',
            stageId,
            type: 'settings',
            path: `${dir}/setting`,
            content: prevSettings?.content ?? '',
            isDirty: prevSettings?.isDirty ?? false,
          },
        ],
      };
    });

    const alive = new Set(nextStages.flatMap((s) => s.files.map((f) => f.id)));
    for (const st of get().stages) {
      for (const f of st.files) alive.add(f.id);
    }
    const prunedMissing: Record<string, boolean> = {};
    for (const [id, value] of Object.entries(get().fileMissing)) {
      if (alive.has(id)) prunedMissing[id] = value;
    }
    set({ agentPersonas: personas, agentStages: nextStages, fileMissing: prunedMissing });

    const boardState = useBoardStore.getState();
    const currentCols = boardState.columns;
    let colsChanged = false;
    const nextCols = currentCols.map((col) => {
      if (!col.persona) return col;
      const matched = personas.find((p) => p.role === col.persona?.trim());
      if (!matched) return col;
      const newIcon = matched.icon || col.icon;
      const newColor = matched.color || col.color;
      if (col.icon !== newIcon || col.color !== newColor) {
        colsChanged = true;
        return { ...col, icon: newIcon, color: newColor };
      }
      return col;
    });

    if (colsChanged) {
      boardState.setColumns(nextCols);
      void boardState.saveCurrentBoard(nextCols);
    }

    get().syncStagesFromColumns(nextCols);
  },

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
  },

  deleteAgentPersona: async (role: string) => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('delete_agent_persona', { role });
    } catch (err) {
      console.info('delete_agent_persona IPC not available or failed:', err);
      throw err;
    }
    await get().loadAgentPersonas();
    try {
      await useBoardStore.getState().loadBoards();
    } catch (err) {
      console.info('Failed to reload boards after agent delete:', err);
    }
  },

  // --- GLOBAL RULES ---
  loadGlobalRules: async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const files = await invoke<Array<{ name: string; path: string; fileType: string }>>(
        'list_global_rules'
      );
      const prevGlobalRules = get().globalRules;
      const prevMap = new Map(prevGlobalRules.map((f) => [f.name, f]));
      const mapped: CortexAgentFile[] = (files ?? []).map((file) => {
        const prev = prevMap.get(file.name);
        return {
          id: `global-rule-${file.name}`,
          name: file.name,
          stageId: 'global',
          type: 'rule',
          path: file.path,
          content: prev?.content ?? '',
          isDirty: prev?.isDirty ?? false,
        };
      });
      set({ globalRules: mapped });
    } catch (err) {
      console.info('loadGlobalRules not available or in web mode:', err);
    }
  },

  // --- FILE OPEN & TEMPLATE CREATION ---
  openFile: async (fileId) => {
    const { globalRules } = get();
    const globalFile = globalRules.find((f) => f.id === fileId);
    if (globalFile) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const content = await invoke<string>('read_cortex_file', { path: globalFile.path });
        const loaded: CortexAgentFile = { ...globalFile, content, isDirty: false };
        set((state) => ({
          globalRules: state.globalRules.map((f) => (f.id === fileId ? loaded : f)),
          activeFile: loaded,
          isEditorOpen: true,
          saveFeedback: null,
        }));
      } catch (err) {
        console.info('read_cortex_file failed for global rule:', err);
        if (globalFile.content) {
          set({ activeFile: { ...globalFile }, isEditorOpen: true, saveFeedback: null });
        }
      }
      return;
    }

    const { stages, agentStages } = get();
    const allStages = [...stages, ...agentStages];
    for (const stage of allStages) {
      const file = stage.files.find((f) => f.id === fileId);
      if (!file) continue;

      if (file.path.startsWith('builtin:')) {
        const content =
          file.content && file.content.trim().length > 0
            ? file.content
            : buildBuiltinHookDescription(file.path, file.name, stage.stageTitle);
        const loaded: CortexAgentFile = { ...file, content, isDirty: false };
        set((state) => ({
          stages: state.stages.map((st) =>
            st.stageId === stage.stageId
              ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
              : st
          ),
          agentStages: state.agentStages.map((st) =>
            st.stageId === stage.stageId
              ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
              : st
          ),
          activeFile: loaded,
          isEditorOpen: true,
          saveFeedback: null,
        }));
        return;
      }

      if (file.type === 'hook') {
        const scriptPath = extractScriptPath(file.path);
        if (scriptPath) {
          try {
            const { invoke } = await import('@tauri-apps/api/core');
            const content = await invoke<string>('read_cortex_file', { path: scriptPath });
            const loaded: CortexAgentFile = { ...file, content, isDirty: false };
            set((state) => ({
              stages: state.stages.map((st) =>
                st.stageId === stage.stageId
                  ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
                  : st
              ),
              agentStages: state.agentStages.map((st) =>
                st.stageId === stage.stageId
                  ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
                  : st
              ),
              activeFile: loaded,
              isEditorOpen: true,
              saveFeedback: null,
            }));
            set((state) => {
              if (!state.fileMissing[fileId]) return state;
              const next = { ...state.fileMissing };
              delete next[fileId];
              return { fileMissing: next };
            });
            return;
          } catch (err) {
            console.info('read_cortex_file for hook script failed, using template/memory:', err);
            const content =
              file.content && file.content.trim().length > 0
                ? file.content
                : buildScriptTemplate(scriptPath, file.name, stage.stageId);
            const loaded: CortexAgentFile = { ...file, content, isDirty: false };
            set((state) => ({
              stages: state.stages.map((st) =>
                st.stageId === stage.stageId
                  ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
                  : st
              ),
              agentStages: state.agentStages.map((st) =>
                st.stageId === stage.stageId
                  ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
                  : st
              ),
              activeFile: loaded,
              isEditorOpen: true,
              saveFeedback: null,
            }));
            return;
          }
        } else {
          const content =
            file.content && file.content.trim().length > 0
              ? file.content
              : buildCommandHookDescription(file.path, file.name, stage.stageTitle);
          const loaded: CortexAgentFile = { ...file, content, isDirty: false };
          set((state) => ({
            stages: state.stages.map((st) =>
              st.stageId === stage.stageId
                ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
                : st
            ),
            agentStages: state.agentStages.map((st) =>
              st.stageId === stage.stageId
                ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
                : st
            ),
            activeFile: loaded,
            isEditorOpen: true,
            saveFeedback: null,
          }));
          return;
        }
      }

      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const content = await invoke<string>('read_cortex_file', { path: file.path });
        const loaded: CortexAgentFile = { ...file, content, isDirty: false };
        set((state) => ({
          stages: state.stages.map((st) =>
            st.stageId === stage.stageId
              ? {
                  ...st,
                  files: st.files.map((f) =>
                    f.id === fileId ? { ...f, content, isDirty: false } : f
                  ),
                }
              : st
          ),
          agentStages: state.agentStages.map((st) =>
            st.stageId === stage.stageId
              ? {
                  ...st,
                  files: st.files.map((f) =>
                    f.id === fileId ? { ...f, content, isDirty: false } : f
                  ),
                }
              : st
          ),
          activeFile: loaded,
          isEditorOpen: true,
          saveFeedback: null,
        }));
        set((state) => {
          if (!state.fileMissing[fileId]) return state;
          const next = { ...state.fileMissing };
          delete next[fileId];
          return { fileMissing: next };
        });
      } catch (err) {
        console.info('read_cortex_file not available or file is missing:', err);
        if (file.content) {
          set({ activeFile: { ...file }, isEditorOpen: true, saveFeedback: null });
          return;
        }
        set((state) => ({
          fileMissing: { ...state.fileMissing, [fileId]: true },
        }));
      }
      return;
    }
  },

  createPersonaTemplate: async (fileId) => {
    const { stages, agentStages } = get();
    const allStages = [...stages, ...agentStages];
    for (const stage of allStages) {
      const file = stage.files.find((f) => f.id === fileId);
      if (!file) continue;

      const template = buildPersonaTemplate(stage.personaName, stage.stageTitle);
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('save_cortex_file', { path: file.path, content: template });
      } catch (err) {
        console.info('save_cortex_file not available or in web mode:', err);
      }
      const loaded: CortexAgentFile = { ...file, content: template, isDirty: false };
      set((state) => {
        const nextMissing = { ...state.fileMissing };
        delete nextMissing[fileId];
        return {
          stages: state.stages.map((st) =>
            st.stageId === stage.stageId
              ? {
                  ...st,
                  files: st.files.map((f) =>
                    f.id === fileId ? { ...f, content: template, isDirty: false } : f
                  ),
                }
              : st
          ),
          agentStages: state.agentStages.map((st) =>
            st.stageId === stage.stageId
              ? {
                  ...st,
                  files: st.files.map((f) =>
                    f.id === fileId ? { ...f, content: template, isDirty: false } : f
                  ),
                }
              : st
          ),
          activeFile: loaded,
          isEditorOpen: true,
          saveFeedback: null,
          fileMissing: nextMissing,
        };
      });
      await get().loadAgentPersonas();
      return;
    }
  },

  createSettingTemplate: async (fileId) => {
    const { stages, agentStages } = get();
    const allStages = [...stages, ...agentStages];
    for (const stage of allStages) {
      const file = stage.files.find((f) => f.id === fileId);
      if (!file) continue;

      const role = (stage.personaName ?? '').trim() || stage.stageId;
      const template = buildSettingTemplate(role, stage.stageTitle);
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('save_cortex_file', { path: file.path, content: template });
      } catch (err) {
        console.info('save_cortex_file not available or in web mode:', err);
      }
      const loaded: CortexAgentFile = { ...file, content: template, isDirty: false };
      set((state) => {
        const nextMissing = { ...state.fileMissing };
        delete nextMissing[fileId];
        return {
          stages: state.stages.map((st) =>
            st.stageId === stage.stageId
              ? {
                  ...st,
                  files: st.files.map((f) =>
                    f.id === fileId ? { ...f, content: template, isDirty: false } : f
                  ),
                }
              : st
          ),
          agentStages: state.agentStages.map((st) =>
            st.stageId === stage.stageId
              ? {
                  ...st,
                  files: st.files.map((f) =>
                    f.id === fileId ? { ...f, content: template, isDirty: false } : f
                  ),
                }
              : st
          ),
          activeFile: loaded,
          isEditorOpen: true,
          saveFeedback: null,
          fileMissing: nextMissing,
        };
      });
      return;
    }
  },

  closeEditor: () => {
    set({
      isEditorOpen: false,
      activeFile: null,
      saveFeedback: null,
    });
  },

  // --- RULES & FILES CRUD ---
  createCortexFile: async (stageId, rawName, kind, opts) => {
    const { stages, agentStages } = get();
    const stage =
      stages.find((st) => st.stageId === stageId) ??
      agentStages.find((st) => st.stageId === stageId);
    if (!stage) throw new Error('Стадия не найдена');

    const role = (stage.personaName ?? '').trim() || stageId;
    const dir = cortexAgentDir(stage.personaName, stageId);

    let targetPath: string;
    let content: string;
    let fileName: string;
    let fileType: CortexAgentFile['type'];

    if (kind === 'rule') {
      const { base, fileName: ruleName } = normalizeCortexFileName(rawName);
      if (!base) throw new Error('Введите имя файла');
      if (!/^[A-Za-z0-9-_]+$/.test(base)) {
        throw new Error('Только латиница, цифры, - и _');
      }
      fileName = ruleName;
      fileType = 'rule';
      targetPath = `${dir}/rules/${fileName}`;
      content = buildRuleTemplate(base, stage.stageTitle);
    } else {
      const moment: CortexHookMoment = opts?.hookMoment ?? 'on_enter';
      const ext: CortexHookExt = opts?.hookExt ?? 'bat';
      if (moment !== 'on_enter' && moment !== 'on_leave' && moment !== 'on_finish') {
        throw new Error('Момент хука: on_enter или on_leave');
      }
      if (ext !== 'bat' && ext !== 'ps1' && ext !== 'py' && ext !== 'sh') {
        throw new Error('Расширение хука: .bat, .ps1, .py или .sh');
      }
      const typed = rawName.trim().replace(/\.[A-Za-z0-9]+$/, '').trim();
      if (typed && !/^[A-Za-z0-9-_]+$/.test(typed)) {
        throw new Error('Только латиница, цифры, - и _');
      }
      fileName = typed ? `${moment}-${typed}.${ext}` : `${moment}.${ext}`;
      fileType = 'hook';
      targetPath = `${dir}/hooks/${fileName}`;
      content = buildHookTemplate(moment, ext, typed || moment, role, stage.stageTitle);
    }

    const duplicate = stage.files.some(
      (f) => f.name.toLowerCase() === fileName.toLowerCase() || f.path.toLowerCase() === targetPath.toLowerCase()
    );
    if (duplicate) throw new Error(`Файл ${fileName} уже есть в стадии`);

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_cortex_file', { path: targetPath, content });
    } catch (err) {
      const detail = err instanceof Error ? err.message : String(err);
      throw new Error(`Не удалось сохранить файл: ${detail}`);
    }

    const idBase = `${stageId}-${kind}-${fileName}`;
    let fileId = idBase;
    let suffix = 2;
    const existingIds = new Set(stage.files.map((f) => f.id));
    while (existingIds.has(fileId)) {
      fileId = `${idBase}-${suffix}`;
      suffix += 1;
    }

    const created: CortexAgentFile = {
      id: fileId,
      name: fileName,
      stageId,
      type: fileType,
      path: targetPath,
      content,
      isDirty: false,
    };

    set((state) => ({
      stages: state.stages.map((st) =>
        st.stageId === stageId ? { ...st, files: [...st.files, created] } : st
      ),
      agentStages: state.agentStages.map((st) =>
        st.stageId === stageId ? { ...st, files: [...st.files, created] } : st
      ),
      activeFile: created,
      isEditorOpen: true,
      saveFeedback: null,
    }));
  },

  createGlobalRule: async (rawName: string) => {
    const { globalRules } = get();
    const { base, fileName } = normalizeCortexFileName(rawName);
    if (!base) throw new Error('Введите имя файла');
    if (!/^[A-Za-z0-9-_]+$/.test(base)) {
      throw new Error('Только латиница, цифры, - и _');
    }

    const targetPath = `.cvh/rules/${fileName}`;
    const content = buildGlobalRuleTemplate(base);

    const duplicate = globalRules.some(
      (f) =>
        f.name.toLowerCase() === fileName.toLowerCase() ||
        f.path.toLowerCase() === targetPath.toLowerCase()
    );
    if (duplicate) throw new Error(`Файл ${fileName} уже есть в глобальных правилах`);

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_cortex_file', { path: targetPath, content });
    } catch (err) {
      const detail = err instanceof Error ? err.message : String(err);
      throw new Error(`Не удалось сохранить файл: ${detail}`);
    }

    const fileId = `global-rule-${fileName}`;
    const created: CortexAgentFile = {
      id: fileId,
      name: fileName,
      stageId: 'global',
      type: 'rule',
      path: targetPath,
      content,
      isDirty: false,
    };

    set((state) => ({
      globalRules: [...state.globalRules, created].sort((a, b) => a.name.localeCompare(b.name)),
      activeFile: created,
      isEditorOpen: true,
      saveFeedback: null,
    }));
  },

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

  saveRules: async () => {
    const { activeFile, stages, agentStages, globalRules } = get();
    if (!activeFile) return;

    set({ isSaving: true });

    const isBuiltin = activeFile.path.startsWith('builtin:');
    const scriptPath = extractScriptPath(activeFile.path);
    const isDirectCommand = activeFile.type === 'hook' && !isBuiltin && !scriptPath;

    if (!isBuiltin && !isDirectCommand) {
      const targetPath = scriptPath ?? activeFile.path;
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('save_cortex_file', { path: targetPath, content: activeFile.content });
      } catch (err) {
        console.info('save_cortex_file not available or in web mode:', err);
      }
    }

    const updatedStages = stages.map((stage) => {
      if (stage.stageId !== activeFile.stageId) return stage;
      return {
        ...stage,
        files: stage.files.map((file) =>
          file.id === activeFile.id
            ? { ...file, content: activeFile.content, isDirty: false }
            : file
        ),
      };
    });
    const updatedAgentStages = agentStages.map((stage) => {
      if (stage.stageId !== activeFile.stageId) return stage;
      return {
        ...stage,
        files: stage.files.map((file) =>
          file.id === activeFile.id
            ? { ...file, content: activeFile.content, isDirty: false }
            : file
        ),
      };
    });
    const updatedGlobalRules = globalRules.map((file) =>
      file.id === activeFile.id
        ? { ...file, content: activeFile.content, isDirty: false }
        : file
    );

    const feedbackMsg = isBuiltin
      ? `Встроенный хук ${activeFile.name} обновлен`
      : isDirectCommand
      ? `Команда ${activeFile.name} обновлена`
      : `Файл ${activeFile.name} сохранен`;

    set({
      stages: updatedStages,
      agentStages: updatedAgentStages,
      globalRules: updatedGlobalRules,
      activeFile: { ...activeFile, isDirty: false },
      isSaving: false,
      saveFeedback: feedbackMsg,
    });

    setTimeout(() => {
      set({ saveFeedback: null });
    }, 2800);
  },

  deleteRule: async (stageId: string, ruleName: string) => {
    if (stageId === 'global') {
      return get().deleteGlobalRule(ruleName);
    }

    set((state) => {
      const filterFiles = (files: CortexAgentFile[]) =>
        files.filter((f) => !(f.type === 'rule' && (f.name === ruleName || f.id === ruleName)));

      const nextStages = state.stages.map((st) =>
        st.stageId === stageId ? { ...st, files: filterFiles(st.files) } : st
      );
      const nextAgentStages = state.agentStages.map((st) =>
        st.stageId === stageId ? { ...st, files: filterFiles(st.files) } : st
      );

      const isActiveDeleted =
        state.activeFile?.stageId === stageId &&
        state.activeFile?.type === 'rule' &&
        (state.activeFile.name === ruleName || state.activeFile.id === ruleName);

      return {
        stages: nextStages,
        agentStages: nextAgentStages,
        activeFile: isActiveDeleted ? null : state.activeFile,
        isEditorOpen: isActiveDeleted ? false : state.isEditorOpen,
      };
    });
  },

  deleteGlobalRule: async (fileName: string) => {
    const { globalRules, activeFile } = get();
    const target = globalRules.find((f) => f.name === fileName || f.id === fileName);
    if (!target) return;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('delete_cortex_file', { path: target.path });
    } catch (err) {
      const detail = err instanceof Error ? err.message : String(err);
      throw new Error(`Не удалось удалить файл: ${detail}`);
    }

    const isActiveDeleted = activeFile?.id === target.id;
    set((state) => ({
      globalRules: state.globalRules.filter((f) => f.id !== target.id),
      activeFile: isActiveDeleted ? null : state.activeFile,
      isEditorOpen: isActiveDeleted ? false : state.isEditorOpen,
    }));
  },

  reorderRules: (stageId: string, ruleNames: string[]) => {
    set((state) => {
      const reorderList = (files: CortexAgentFile[]) => {
        const rules = files.filter((f) => f.type === 'rule');
        const nonRules = files.filter((f) => f.type !== 'rule');

        rules.sort((a, b) => {
          const idxA = ruleNames.indexOf(a.name);
          const idxB = ruleNames.indexOf(b.name);
          if (idxA !== -1 && idxB !== -1) return idxA - idxB;
          if (idxA !== -1) return -1;
          if (idxB !== -1) return 1;
          return a.name.localeCompare(b.name);
        });

        const personaFiles = nonRules.filter((f) => f.type === 'persona');
        const hookFiles = nonRules.filter((f) => f.type === 'hook');
        const settingFiles = nonRules.filter((f) => f.type === 'settings');
        const otherFiles = nonRules.filter(
          (f) => f.type !== 'persona' && f.type !== 'hook' && f.type !== 'settings'
        );

        return [...personaFiles, ...hookFiles, ...rules, ...otherFiles, ...settingFiles];
      };

      return {
        stages: state.stages.map((st) =>
          st.stageId === stageId ? { ...st, files: reorderList(st.files) } : st
        ),
        agentStages: state.agentStages.map((st) =>
          st.stageId === stageId ? { ...st, files: reorderList(st.files) } : st
        ),
      };
    });
  },
});

// Backward-compatible slice creators for zero regressions
export const createCortexFilesSlice = createCortexSlice;
export const createPersonasSlice = createCortexSlice;
export const createRulesSlice = createCortexSlice;

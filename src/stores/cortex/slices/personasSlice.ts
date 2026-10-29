import { StateCreator } from 'zustand';
import { AgentPersonaInfo, CortexState, PersonasSlice, RawAgentPersona } from '../types';
import { CortexAgentStage } from '../../../types';
import { useBoardStore } from '../../boardStore';
import { cortexAgentDir } from '../cortexHelpers';
import {
  getDeterministicAgentColor,
  isAllowedAgentColor,
} from '../../../lib/agentPalette';

export const createPersonasSlice: StateCreator<
  CortexState,
  [],
  [],
  PersonasSlice
> = (set, get) => ({
  agentPersonas: [],

  loadAgentPersonas: async () => {
    let raw: RawAgentPersona[] = [];
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const list = await invoke<RawAgentPersona[]>('list_agent_personas');
      if (Array.isArray(list)) raw = list;
    } catch {
      // IPC недоступен — секция агентов пуста, колонки не страдают.
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

      // Real rules from disk!
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

      // Extra in-memory rules not yet on disk or newly added
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

    // Снять пометы об отсутствующих файлах для агентов, которых больше нет.
    const alive = new Set(nextStages.flatMap((s) => s.files.map((f) => f.id)));
    for (const st of get().stages) {
      for (const f of st.files) alive.add(f.id);
    }
    const prunedMissing: Record<string, boolean> = {};
    for (const [id, value] of Object.entries(get().fileMissing)) {
      if (alive.has(id)) prunedMissing[id] = value;
    }
    set({ agentPersonas: personas, agentStages: nextStages, fileMissing: prunedMissing });

    // Синхронизировать колонки активной доски с обновленными персонами (иконка и цвет персоны)
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

    // Реактивно синхронизировать колонки в дереве Cortex
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

    // Автоматическое наследование цвета колонками, на которые назначен этот агент (если не backlog и не ready)
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

    // Автоматическое наследование иконки колонками, на которые назначен этот агент
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
});

import { create } from 'zustand';
import { useBoardStore } from '../boardStore';
import { ToolPolicyState, ToolSettings } from './toolPolicyTypes';
import { AVAILABLE_TOOLS } from './toolMeta';
import { DEFAULT_TOOL_SETTINGS, DEFAULT_OVERRIDES } from './toolPolicyDefaults';

export const useToolPolicyStore = create<ToolPolicyState>((set, get) => ({
  isModalOpen: false,
  stageOverrides: DEFAULT_OVERRIDES,
  toolSettings: DEFAULT_TOOL_SETTINGS,
  toolDescriptions: {},
  toolCustomSchemas: {},

  openModal: () => set({ isModalOpen: true }),
  closeModal: () => set({ isModalOpen: false }),

  setStageTool: (stageId, toolId, enabled) => {
    set((state) => {
      const currentStage = state.stageOverrides[stageId] || {};
      const nextStage = { ...currentStage, [toolId]: enabled };
      return {
        stageOverrides: {
          ...state.stageOverrides,
          [stageId]: nextStage,
        },
      };
    });
    get().saveToTauri();
  },

  setToolSetting: (toolId, key, value) => {
    set((state) => {
      const currentToolSettings = state.toolSettings[toolId] || {};
      const nextToolSettings = { ...currentToolSettings, [key]: value };
      return {
        toolSettings: {
          ...state.toolSettings,
          [toolId]: nextToolSettings,
        },
      };
    });
    get().saveToTauri();
  },

  getToolSetting: (toolId, key, defaultValue) => {
    const { toolSettings } = get();
    return toolSettings[toolId]?.[key] ?? defaultValue;
  },

  setToolDescription: (toolId, description) => {
    set((state) => ({
      toolDescriptions: {
        ...state.toolDescriptions,
        [toolId]: description,
      },
    }));
    get().saveToTauri();
  },

  resetToolDescription: (toolId) => {
    set((state) => {
      const next = { ...state.toolDescriptions };
      delete next[toolId];
      return { toolDescriptions: next };
    });
    get().saveToTauri();
  },

  setToolCustomSchema: (toolId, schema) => {
    set((state) => ({
      toolCustomSchemas: {
        ...state.toolCustomSchemas,
        [toolId]: schema,
      },
    }));
    get().saveToTauri();
  },

  resetToolCustomSchema: (toolId) => {
    set((state) => {
      const next = { ...state.toolCustomSchemas };
      delete next[toolId];
      return { toolCustomSchemas: next };
    });
    get().saveToTauri();
  },

  isToolAllowed: (toolId, stageOrAgentId) => {
    const { stageOverrides } = get();

    // Hierarchical rule for subagent sub-options
    if (toolId === 'subagent_sync' || toolId === 'subagent_bg' || toolId === 'collect_subagent' || toolId === 'subagent_list') {
      if (stageOrAgentId) {
        const stageMaster = stageOverrides[stageOrAgentId]?.['subagent'];
        if (stageMaster === false) return false;
        const selfStage = stageOverrides[stageOrAgentId]?.[toolId];
        if (typeof selfStage === 'boolean') return selfStage;
      }
      return true;
    }

    if (stageOrAgentId) {
      if (stageOverrides[stageOrAgentId] && typeof stageOverrides[stageOrAgentId][toolId] === 'boolean') {
        return stageOverrides[stageOrAgentId][toolId];
      }
      try {
        const columns = useBoardStore.getState().columns;
        // 1. Если передан агент (роль), проверяем колонку с этой персоной
        const colForPersona = columns.find((c) => c.persona === stageOrAgentId);
        if (colForPersona && stageOverrides[colForPersona.id] && typeof stageOverrides[colForPersona.id][toolId] === 'boolean') {
          return stageOverrides[colForPersona.id][toolId];
        }
        // 2. Если передан id колонки, проверяем назначенную персону
        const colById = columns.find((c) => c.id === stageOrAgentId);
        if (colById?.persona && stageOverrides[colById.persona] && typeof stageOverrides[colById.persona][toolId] === 'boolean') {
          return stageOverrides[colById.persona][toolId];
        }
      } catch {
        // ignore
      }
      if (DEFAULT_OVERRIDES[stageOrAgentId] && typeof DEFAULT_OVERRIDES[stageOrAgentId][toolId] === 'boolean') {
        return DEFAULT_OVERRIDES[stageOrAgentId][toolId];
      }
      return true;
    }

    return true;
  },

  calculateSavedTokens: (stageId) => {
    const { isToolAllowed } = get();
    let saved = 0;
    for (const tool of AVAILABLE_TOOLS) {
      if (!isToolAllowed(tool.id, stageId)) {
        saved += tool.tokenWeight;
      }
    }
    return saved > 0 ? saved : 2400;
  },

  resetDefaults: () => {
    set({
      stageOverrides: { ...DEFAULT_OVERRIDES },
      toolSettings: { ...DEFAULT_TOOL_SETTINGS },
      toolDescriptions: {},
      toolCustomSchemas: {},
    });
    get().saveToTauri();
  },

  loadFromTauri: async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const policies = await invoke<{
        stageOverrides?: Record<string, Record<string, boolean>>;
        toolSettings?: ToolSettings;
        toolDescriptions?: Record<string, string>;
        toolCustomSchemas?: Record<string, unknown>;
      }>('get_tool_policies');

      if (policies) {
        const mergedSettings: ToolSettings = { ...DEFAULT_TOOL_SETTINGS };
        if (policies.toolSettings) {
          for (const [tool, settings] of Object.entries(policies.toolSettings)) {
            mergedSettings[tool] = {
              ...DEFAULT_TOOL_SETTINGS[tool],
              ...settings,
            };
          }
        }
        set({
          stageOverrides: policies.stageOverrides || DEFAULT_OVERRIDES,
          toolSettings: mergedSettings,
          toolDescriptions: policies.toolDescriptions || {},
          toolCustomSchemas: policies.toolCustomSchemas || {},
        });
      }
    } catch {
      // In web browser or mock environment, keep memory defaults
    }
  },

  saveToTauri: async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const { stageOverrides, toolSettings, toolDescriptions, toolCustomSchemas } = get();
      await invoke('save_tool_policies', {
        policies: {
          stageOverrides,
          toolSettings,
          toolDescriptions,
          toolCustomSchemas,
        },
      });
    } catch {
      // Browser or test fallback
    }
  },
}));

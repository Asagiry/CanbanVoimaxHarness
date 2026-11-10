export type ToolCategory =
  | 'system'
  | 'fs'
  | 'web'
  | 'agent'
  | 'workflow'
  | 'interactive'
  | 'orchestration';

export interface ToolMeta {
  id: string;
  name: string;
  label: string;
  description: string;
  category: ToolCategory;
  tokenWeight: number;
  parentTool?: string;
  isSubagentMaster?: boolean;
  aliases?: string[];
  parameters?: Record<string, unknown>;
}

export interface ToolParamDef {
  key: string;
  label: string;
  defaultValue: number;
  min: number;
  max: number;
  step: number;
  unit: string;
}

export type ToolSettings = Record<string, Record<string, number | string>>;

export interface ToolPolicyState {
  isModalOpen: boolean;
  stageOverrides: Record<string, Record<string, boolean>>;
  toolSettings: ToolSettings;
  toolDescriptions: Record<string, string>;
  toolCustomSchemas: Record<string, unknown>;

  // Actions
  openModal: () => void;
  closeModal: () => void;
  setStageTool: (stageId: string, toolId: string, enabled: boolean) => void;
  setToolSetting: (toolId: string, key: string, value: number | string) => void;
  getToolSetting: (toolId: string, key: string, defaultValue: number | string) => number | string;
  setToolDescription: (toolId: string, description: string) => void;
  resetToolDescription: (toolId: string) => void;
  setToolCustomSchema: (toolId: string, schema: unknown) => void;
  resetToolCustomSchema: (toolId: string) => void;
  isToolAllowed: (toolId: string, stageId?: string) => boolean;
  calculateSavedTokens: (stageId?: string) => number;
  resetDefaults: () => void;
  loadFromTauri: () => Promise<void>;
  saveToTauri: () => Promise<void>;
}

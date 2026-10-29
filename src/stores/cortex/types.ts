import { CortexAgentFile, CortexAgentStage, PipelineColumn } from '../../types';

export type CortexHookMoment = 'on_enter' | 'on_leave' | 'on_finish';
export type CortexHookExt = 'bat' | 'ps1' | 'py' | 'sh';

export interface AgentPersonaInfo {
  role: string;
  hasPrompt: boolean;
  isBuiltin?: boolean;
  personaFile?: string;
  rules?: string[];
  hooks?: string[];
  icon?: string;
  color?: string;
  contextWindow?: number;
}

export interface RawAgentPersona {
  role: string;
  hasPrompt?: boolean;
  has_prompt?: boolean;
  isBuiltin?: boolean;
  is_builtin?: boolean;
  personaFile?: string;
  persona_file?: string;
  rules?: string[];
  hooks?: string[];
  icon?: string;
  color?: string;
  contextWindow?: number;
  context_window?: number;
}

export interface PersonasState {
  agentPersonas: AgentPersonaInfo[];
}

export interface PersonasActions {
  loadAgentPersonas: () => Promise<void>;
  syncAgentPersonas: () => Promise<void>;
  renameAgentPersona: (oldRole: string, newRole: string) => Promise<void>;
  deleteAgentPersona: (role: string) => Promise<void>;
  setAgentPersonaColor: (role: string, color: string) => Promise<void>;
  setAgentPersonaIcon: (role: string, icon: string) => Promise<void>;
}

export type PersonasSlice = PersonasState & PersonasActions;

export interface RulesState {}

export interface RulesActions {
  createCortexFile: (
    stageId: string,
    rawName: string,
    kind: 'rule' | 'hook',
    opts?: { hookMoment?: CortexHookMoment; hookExt?: CortexHookExt },
  ) => Promise<void>;
  updateActiveFileContent: (content: string) => void;
  saveRules: () => Promise<void>;
  deleteRule: (stageId: string, ruleName: string) => Promise<void>;
  reorderRules: (stageId: string, ruleNames: string[]) => void;
}

export type RulesSlice = RulesState & RulesActions;

export interface CortexFilesState {
  stages: CortexAgentStage[];
  agentStages: CortexAgentStage[];
  activeFile: CortexAgentFile | null;
  focusedStageId: string | null;
  isEditorOpen: boolean;
  isSaving: boolean;
  saveFeedback: string | null;
  fileMissing: Record<string, boolean>;
}

export interface CortexFilesActions {
  toggleStageExpand: (stageId: string) => void;
  expandStage: (stageId: string) => void;
  focusStage: (stageId: string) => void;
  syncStagesFromColumns: (columns: PipelineColumn[]) => void;
  openFile: (fileId: string) => Promise<void>;
  createPersonaTemplate: (fileId: string) => Promise<void>;
  createSettingTemplate: (fileId: string) => Promise<void>;
  closeEditor: () => void;
}

export type CortexFilesSlice = CortexFilesState & CortexFilesActions;

export type CortexState = PersonasSlice & RulesSlice & CortexFilesSlice;

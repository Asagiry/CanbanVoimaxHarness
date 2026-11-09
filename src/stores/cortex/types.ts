import { CortexAgentFile, CortexAgentStage, PipelineStage, PipelineColumn } from '../../types';

export type CortexHookMoment = 'on_enter' | 'on_leave' | 'on_finish' | 'on_exit';
export type CortexHookExt = 'bat' | 'ps1' | 'py' | 'sh';

export interface AgentPersonaInfo {
  role: string;
  persona?: string;
  hasPrompt: boolean;
  isBuiltin?: boolean;
  personaFile?: string;
  rules?: string[];
  hooks?: string[];
  icon?: string;
  color?: string;
  contextWindow?: number;
  reasoningEffort?: string | null;
  model?: string | null;
  providerId?: string | null;
}

export interface RawAgentPersona {
  role?: string;
  persona?: string;
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
  reasoningEffort?: string | null;
  reasoning_effort?: string | null;
  model?: string | null;
  providerId?: string | null;
  provider_id?: string | null;
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
  createGlobalRule: (rawName: string, folder?: string) => Promise<void>;
  updateActiveFileContent: (content: string) => void;
  saveRules: () => Promise<void>;
  deleteRule: (stageId: string, ruleName: string) => Promise<void>;
  deleteGlobalRule: (fileName: string) => Promise<void>;
  reorderRules: (stageId: string, ruleNames: string[]) => void;
}

export type RulesSlice = RulesState & RulesActions;

export interface CvhGitStatus {
  hasChanges: boolean;
  changedPaths: string[];
  branch: string;
  ahead?: boolean;
}

export interface PendingDelete {
  path: string;
  kind: 'rule' | 'agent' | 'globalRule';
  stageId?: string;
  fileId?: string;
  role?: string;
}

export interface CortexFilesState {
  stages: CortexAgentStage[];
  agentStages: CortexAgentStage[];
  globalRules: CortexAgentFile[];
  activeFile: CortexAgentFile | null;
  focusedStageId: string | null;
  isEditorOpen: boolean;
  isSaving: boolean;
  saveFeedback: string | null;
  fileMissing: Record<string, boolean>;
  cvhGitStatus: CvhGitStatus | null;
  cvhGitSaving: boolean;
  cvhGitError: string | null;
  pendingDeletes: Record<string, PendingDelete>;
  pendingColumnChanges: Record<string, true>;
}

export interface CortexFilesActions {
  toggleStageExpand: (stageId: string) => void;
  expandStage: (stageId: string) => void;
  focusStage: (stageId: string) => void;
  syncStagesFromStages?: (stages: PipelineStage[]) => void;
  syncStagesFromColumns: (columns: PipelineColumn[]) => void;
  loadGlobalRules: () => Promise<void>;
  refreshCvhGitStatus: () => Promise<void>;
  saveCvhToGit: (message?: string) => Promise<void>;
  markPendingDelete: (key: string, entry: PendingDelete) => void;
  restorePendingDelete: (key: string) => void;
  markPendingColumnChange: (stageId: string) => void;
  clearCvhGitError: () => void;
  openFile: (fileId: string) => Promise<void>;
  createPersonaTemplate: (fileId: string) => Promise<void>;
  createSettingTemplate: (fileId: string) => Promise<void>;
  closeEditor: () => void;
}

export type CortexFilesSlice = CortexFilesState & CortexFilesActions;

export type CortexState = PersonasSlice & RulesSlice & CortexFilesSlice;

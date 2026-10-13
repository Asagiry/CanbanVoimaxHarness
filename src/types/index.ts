export type TaskPriority = 'low' | 'medium' | 'high' | 'critical';

export type TaskStatus = 'idle' | 'running' | 'paused' | 'waiting_review' | 'blocked' | 'done';

export type TaskVerdict = 'NONE' | 'APPROVED' | 'APPROVED_WITH_CAVEATS' | 'CHANGES_REQUESTED';

export interface FastTrackConfig {
  noReview?: boolean;
  noRetro?: boolean;
}

export interface KanbanTask {
  id: string;
  title: string;
  description?: string;
  priority: TaskPriority;
  columnId: string;
  persona: string;
  branch: string;
  worktreePath: string;
  status: TaskStatus;
  activeStageDuration: number; // in seconds
  stageDurations: Record<string, number>; // stageId -> seconds
  totalElapsed: number; // in seconds
  isTimerRunning: boolean;
  returnsCount: number;
  fastTrack?: FastTrackConfig;
  customFlow?: string[];
  verdict?: TaskVerdict;
  verdictNote?: string;
  model?: string;
}

export interface PipelineColumn {
  id: string;
  title: string;
  color: string;
  icon: string;
  persona: string | null;
  // Column = agent: autoRun triggers run_agent on entry (default: !!persona),
  // terminal marks a sink column (task becomes done, no further promotion).
  autoRun?: boolean;
  terminal?: boolean;
  contextWindow: number;
  modelOverride: string | null;
  temperature?: number;
  reasoningEffort?: string;
  gatekeeperSensor?: string | null;
  wipLimit?: number;
  hooks?: Record<string, string>;
  routes?: Record<string, string>;
}

export interface PipelineConfig {
  id: string;
  title: string;
  columns: PipelineColumn[];
}

export interface CortexAgentFile {
  id: string;
  name: string;
  stageId: string;
  type: 'persona' | 'rule' | 'settings';
  path: string;
  content: string;
  isDirty?: boolean;
}

export interface CortexAgentStage {
  stageId: string;
  stageTitle: string;
  color: string;
  icon: string;
  personaName: string;
  files: CortexAgentFile[];
  isExpanded: boolean;
}

export interface ToolExecution {
  id: string;
  toolName: string;
  timestamp: string;
  durationMs: number;
  status: 'running' | 'success' | 'failed';
  exitCode?: number;
  params: Record<string, unknown>;
  stdout?: string;
  stderr?: string;
}

export interface LiveStreamEntry {
  id: string;
  timestamp: string;
  type: 'thought' | 'output' | 'system' | 'error' | 'steer' | 'user';
  content: string;
  durationMs?: number;
}

export type ActivityType = 'thought' | 'output' | 'tool' | 'steer' | 'system' | 'error' | 'user';

export interface ActivityItem {
  id: string;
  type: ActivityType;
  timestamp: string;
  content?: string;
  tool?: ToolExecution;
  durationMs?: number;
}

export interface ProjectInfo {
  id: string;
  name: string;
  rootPath: string;
  activeBranch: string;
  worktreesCount: number;
}

export interface CompactionReport {
  tokens_before: number;
  tokens_after: number;
  evaporated_count: number;
  saved_tokens: number;
}

export interface LlmSettings {
  provider: string;
  apiKey: string;
  baseUrl: string;
  model: string;
  temperature?: number;
}

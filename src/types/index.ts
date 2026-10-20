export type TaskPriority = 'low' | 'medium' | 'high' | 'critical';

export interface TodoItem {
  id: string;
  content: string;
  status: 'pending' | 'in_progress' | 'completed';
}

export type TaskStatus = 'idle' | 'running' | 'paused' | 'waiting_review' | 'waiting_subtask' | 'blocked' | 'done';

export type TaskVerdict = 'NONE' | 'APPROVED' | 'APPROVED_WITH_CAVEATS' | 'CHANGES_REQUESTED';

export interface FastTrackConfig {
  noReview?: boolean;
  noRetro?: boolean;
}

export interface BoardDef {
  id: string;
  name: string;
  columns: PipelineColumn[];
}

export interface KanbanTask {
  id: string;
  title: string;
  description?: string;
  attachments?: string[];
  createdAt?: number;
  updatedAt?: number;
  stageEnteredAt?: number;
  priority: TaskPriority;
  columnId: string;
  boardId?: string;
  persona: string;
  branch?: string;
  worktreePath?: string;
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
  parentId?: string;
  subtaskIndex?: number;
  waitingForSubtaskId?: string;
  todos?: TodoItem[];
  tags?: string[];
  handoverSummary?: string;
  currentTokens?: number;
  skipWorktree?: boolean;
}

export interface PipelineColumn {
  id: string;
  title: string;
  color: string;
  icon: string;
  persona: string | null;
  onEnterScript?: string | null;
  // Column = agent: autoRun triggers run_agent on entry (default: !!persona),
  // terminal marks a sink column (task becomes done, no further promotion).
  autoRun?: boolean;
  terminal?: boolean;
  contextWindow?: number;
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
  // Новая схема хранения: `.cvh/agents/<role>/agent.md` (persona),
  // `.cvh/agents/<role>/rules/*.md` (rule), `.cvh/agents/<role>/hooks/*` (hook),
  // `.cvh/agents/<role>/setting` (settings, JSON без расширения).
  type: 'persona' | 'rule' | 'settings' | 'hook';
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
  isBuiltin?: boolean;
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
  images?: string[];
  createdAt?: number;
}

export type ActivityType = 'thought' | 'output' | 'tool' | 'steer' | 'system' | 'error' | 'user' | 'prompt' | 'question' | 'compacted';

export interface ActivityItem {
  id: string;
  type: ActivityType;
  timestamp: string;
  content?: string;
  tool?: ToolExecution;
  durationMs?: number;
  images?: string[];
  isFinal?: boolean;
  createdAt?: number;
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
  compacted_messages_count?: number;
  summary?: string;
}

export interface DiffFile {
  path: string;
  status: 'M' | 'A' | 'D' | string;
  additions: number;
  deletions: number;
  patch: string;
}

export interface TaskGitDiff {
  files: DiffFile[];
  totalAdditions: number;
  totalDeletions: number;
  total_additions?: number;
  total_deletions?: number;
  branch: string;
}

export type SubagentRole = 'explorer' | 'developer' | 'reviewer' | 'general' | string;
export type SubagentStatus = 'running' | 'finished' | 'error' | 'stopped';

export interface SubagentInfo {
  id: string;
  parentTaskId: string;
  toolCallId?: string;
  title: string;
  role: SubagentRole;
  prompt: string;
  status: SubagentStatus;
  createdAt?: number;
  durationMs?: number;
  stepsCount: number;
  lastError?: string;
  isBg?: boolean;
  finishedAt?: number;
  isBoardSubtask?: boolean;
}

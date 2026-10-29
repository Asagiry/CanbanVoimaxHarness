export * from './domain';

export type TaskPriority = 'low' | 'medium' | 'high' | 'critical';

export interface TodoItem {
  id: string;
  content: string;
  status: 'pending' | 'in_progress' | 'completed';
}

export type TaskStatus = 'idle' | 'running' | 'paused' | 'waiting_review' | 'waiting_subtask' | 'blocked' | 'done';

export type TaskVerdict = 'NONE' | 'APPROVED' | 'APPROVED_WITH_CAVEATS' | 'CHANGES_REQUESTED';

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
  completedAt?: number;
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
  skipBacklogHook?: boolean;
  skip_backlog_hook?: boolean;
  position?: number;
  previousStageId?: string;
  coverImage?: string;
  hookStates?: TaskHookStates;
}

/**
 * Authoritative boundary DTO representing task payloads from the Rust Tauri IPC backend.
 * Accommodates camelCase and snake_case properties returned across IPC boundaries.
 */
export interface BackendTaskDTO {
  id: string;
  title: string;
  description?: string;
  stageId?: string;
  stage_id?: string;
  columnId?: string;
  boardId?: string;
  board_id?: string;
  worktreePath?: string;
  worktree_path?: string;
  branch?: string;
  verdict?: string;
  customFlow?: string[];
  custom_flow?: string[];
  status?: string;
  model?: string;
  parentId?: string;
  parent_id?: string;
  subtaskIndex?: number;
  subtask_index?: number;
  waitingForSubtaskId?: string;
  waiting_for_subtask_id?: string;
  handoverSummary?: string;
  handover_summary?: string;
  coverImage?: string;
  cover_image?: string;
  attachments?: string[];
  todos?: Array<{
    id?: string;
    content?: string;
    status?: string;
  }>;
  priority?: string;
  tags?: string[];
  currentTokens?: number;
  current_tokens?: number;
  skipWorktree?: boolean;
  skip_worktree?: boolean;
  skipBacklogHook?: boolean;
  skip_backlog_hook?: boolean;
  createdAt?: number;
  created_at?: number;
  updatedAt?: number;
  updated_at?: number;
  stageEnteredAt?: number;
  stage_entered_at?: number;
  completedAt?: number;
  completed_at?: number;
  position?: number;
  previousStageId?: string;
  previous_stage_id?: string;
  fromStage?: string;
  from_stage?: string;
  activeStageDuration?: number;
  stageDurations?: Record<string, number>;
  totalElapsed?: number;
  isTimerRunning?: boolean;
  returnsCount?: number;
  persona?: string;
  verdictNote?: string;
  hookStates?: TaskHookStates;
  hook_states?: TaskHookStates;
}

/**
 * Strict boundary type-guard validating that an incoming payload is a valid BackendTaskDTO.
 */
export function isBackendTaskDTO(raw: unknown): raw is BackendTaskDTO {
  if (!raw || typeof raw !== 'object') return false;
  const candidate = raw as Record<string, unknown>;
  return typeof candidate.id === 'string' && candidate.id.trim() !== '';
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
  // isIntake marks column 0 / intake stage.
  autoRun?: boolean;
  terminal?: boolean;
  isIntake?: boolean;
  contextWindow?: number;
  modelOverride: string | null;
  temperature?: number;
  reasoningEffort?: string;
  gatekeeperSensor?: string | null;
  wipLimit?: number;
  hooks?: Record<string, HookDefinition>;
  routes?: Record<string, string>;
}

export type HookDefinition = string | {
  script: string;
  description?: string;
};

export function getHookScript(hook: HookDefinition | undefined | null): string | undefined {
  if (!hook) return undefined;
  if (typeof hook === 'string') return hook;
  return hook.script || (hook as { path?: string }).path || undefined;
}

export function getHookDescription(hook: HookDefinition | undefined | null): string | undefined {
  if (!hook || typeof hook === 'string') return undefined;
  return hook.description || undefined;
}

export interface HookExecutionState {
  hookName: string;
  stageId: string;
  hookType: 'on_enter' | 'on_leave';
  status: 'running' | 'success' | 'failed';
  startedAt: number;
  durationMs?: number;
  exitCode?: number | null;
  stdout?: string;
  stderr?: string;
  error?: string;
  description?: string;
}

export interface TaskHookStates {
  on_enter?: HookExecutionState;
  on_leave?: HookExecutionState;
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
  hasProject?: boolean;
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

export interface GitImageDiff {
  path: string;
  status: string;
  mimeType: string;
  currentDataUrl?: string | null;
  previousDataUrl?: string | null;
  currentBytes?: number | null;
  previousBytes?: number | null;
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

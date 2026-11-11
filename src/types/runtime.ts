export type ReasoningEffort = 'off' | 'low' | 'medium' | 'high';

export const REASONING_EFFORTS: readonly ReasoningEffort[] = ['off', 'low', 'medium', 'high'];

export function normalizeReasoningEffort(raw: unknown): ReasoningEffort | null {
  if (typeof raw !== 'string') return null;
  const v = raw.trim().toLowerCase();
  if (v === 'low' || v === 'medium' || v === 'high') return v;
  return null;
}

export type CortexHookMoment = 'on_enter' | 'on_leave' | 'on_finish' | 'on_exit';

export interface CortexAgentFile {
  id: string;
  name: string;
  stageId: string;
  // Новая схема хранения: `.cvh/agents/<role>/agent.md` (persona),
  // `.cvh/agents/<role>/rules/*.md` (rule), `.cvh/agents/<role>/hooks/*` (hook),
  // `.cvh/agents/<role>/setting` (settings, JSON без расширения).
  type: 'persona' | 'rule' | 'settings' | 'hook' | 'folder';
  hookMoment?: CortexHookMoment;
  path: string;
  content: string;
  isDirty?: boolean;
  isDeleted?: boolean;
  folder?: string;
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
  isDeleted?: boolean;
}

export interface ToolExecution {
  id: string;
  toolName: string;
  timestamp: string;
  durationMs: number;
  status: 'queued' | 'running' | 'success' | 'failed';
  exitCode?: number;
  params: Record<string, unknown>;
  stdout?: string;
  stderr?: string;
  batch?: { index: number; total: number };
  createdAt?: number;
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

export type ActivityType =
  | 'thought'
  | 'output'
  | 'tool'
  | 'steer'
  | 'system'
  | 'error'
  | 'user'
  | 'prompt'
  | 'question'
  | 'compacted'
  | 'summary'
  | 'reject'
  | 'agent';

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
export type SubagentStatus = 'running' | 'finished' | 'error' | 'stopped' | 'paused';

export interface SubagentInfo {
  id: string;
  parentTaskId: string;
  toolCallId?: string;
  title: string;
  role: SubagentRole;
  persona?: SubagentRole;
  prompt: string;
  status: SubagentStatus;
  createdAt?: number;
  durationMs?: number;
  stepsCount: number;
  lastError?: string;
  isBg?: boolean;
  finishedAt?: number;
  isBoardSubtask?: boolean;
  model?: string;
  providerId?: string;
  reasoningEffort?: ReasoningEffort;
}

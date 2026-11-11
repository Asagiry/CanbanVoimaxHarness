import type { ActivityItem, BackendTaskDTO, TodoItem } from '../../types';
import type { AgentSessionSummary } from '../../stores/agentSessionStore';
import type { TaskAdvancedPayload } from '../notificationService';

export interface AgentThoughtPayload {
  taskId?: string;
  streamId?: string;
  thought?: string;
  delta?: string;
}

export interface AgentOutputPayload {
  taskId?: string;
  streamId?: string;
  output?: string;
  content?: string;
  token?: string;
}

export interface AgentToolCallPayload {
  taskId?: string;
  id?: string;
  name?: string;
  arguments?: string | Record<string, unknown>;
  batch?: { index: number; total: number };
  status?: string;
}

export interface AgentToolResultPayload {
  taskId?: string;
  id?: string;
  name?: string;
  result?: unknown;
  success?: boolean;
  batch?: { index: number; total: number };
  durationMs?: number;
}

export interface QuestionOptionItem {
  label: string;
  description?: string;
}

export interface AgentQuestionPayload {
  taskId?: string;
  questionId?: string;
  text?: string;
  options?: Array<string | QuestionOptionItem>;
  multiple?: boolean;
}

export interface AgentTelemetryPayload {
  taskId?: string;
  inputTokens?: number;
  outputTokens?: number;
  cacheReadTokens?: number;
  cacheCreationTokens?: number;
  cacheHitRate?: number;
  durationMs?: number;
  tps?: number;
  ttftMs?: number;
  systemTokens?: number;
  toolTokens?: number;
}

export interface AgentStreamStartPayload {
  taskId: string;
  streamId: string;
}

export interface AgentStreamRetryPayload extends AgentStreamStartPayload {
  delaySecs: number;
}

export interface AgentTurnStartPayload {
  taskId?: string;
}

export interface AgentFinishedPayload {
  taskId?: string;
  response?: string;
}

export interface AgentErrorPayload {
  taskId?: string;
  error?: string;
}

export interface AgentStatusPayload {
  taskId?: string;
  status?: string;
  response?: string;
  error?: string;
  projectPath?: string;
}

export interface AgentAutoRunPayload {
  taskId?: string;
  prompt?: string;
  projectPath?: string;
}

export interface AgentCompactedPayload {
  taskId?: string;
  currentTokens?: number;
  previousTokens?: number;
  tokens_before?: number;
  tokens_after?: number;
  tokensBefore?: number;
  tokensAfter?: number;
  saved_tokens?: number;
  estimatedSavedTokens?: number;
  savedTokens?: number;
  savedChars?: number;
  evaporatedCount?: number;
}

export interface AgentAbortedPayload {
  taskId?: string;
}

export interface AgentSteerPayload {
  taskId?: string;
  content?: string;
  type?: string;
  images?: string[];
}

export interface TaskCreatedPayload {
  task?: BackendTaskDTO;
  projectPath?: string;
}

export interface TaskMovedPayload {
  task?: BackendTaskDTO;
  fromStage?: string;
  taskId?: string;
  toStage?: string;
  stageId?: string;
  rejected?: boolean;
  reason?: string;
  verdict?: string;
  status?: string;
  projectPath?: string;
}

export interface TaskDeletedPayload {
  taskId?: string;
  deletedIds?: string[];
  projectPath?: string;
}

export interface TaskActivityPayload {
  taskId?: string;
  item?: ActivityItem;
}

export interface DialogUpdatedPayload {
  taskId?: string;
  deletedItemId?: string;
}

export interface HookStartPayload {
  taskId?: string;
  stageId?: string;
  hookType?: 'on_enter' | 'on_leave' | 'on_finish' | 'on_exit';
  hookName?: string;
  description?: string;
}

export interface HookFinishPayload {
  taskId?: string;
  stageId?: string;
  hookType?: 'on_enter' | 'on_leave' | 'on_finish' | 'on_exit';
  success?: boolean;
  exitCode?: number | null;
  stdout?: string;
  stderr?: string;
  error?: string;
  hookName?: string;
}

export interface TaskUpdatedPayload {
  taskId?: string;
  todos?: TodoItem[];
  task?: BackendTaskDTO;
  projectPath?: string;
}

export interface TasksUpdatedPayload {
  tasks?: BackendTaskDTO[];
  taskId?: string;
  projectPath?: string;
}

export interface ProjectSwitchedPayload {
  projectPath?: string;
  path?: string;
}

export interface SubagentSpawnedPayload {
  parentTaskId?: string;
  subagentId?: string;
  title?: string;
  role?: string;
  prompt?: string;
  tools?: string[];
  createdAt?: number;
}

export interface SubagentFinishedPayload {
  parentTaskId?: string;
  subagentId?: string;
  response?: string;
}

export interface SubagentErrorPayload {
  parentTaskId?: string;
  subagentId?: string;
  error?: string;
}

export interface SubagentStatusPayload {
  subagentId?: string;
  status?: string;
  parentTaskId?: string;
  error?: string;
  durationMs?: number;
}

export interface SubagentAbortedPayload {
  subagentId?: string;
  parentTaskId?: string;
}

export interface SubagentActivityPayload {
  parentTaskId?: string;
  subagentId?: string;
  activity?: ActivityItem;
}

export interface TextBuffer {
  chunks: Array<{ type: 'thought' | 'output'; text: string; streamId?: string }>;
  timer?: ReturnType<typeof setTimeout>;
}

/**
 * Authoritative type map of all Tauri IPC events emitted by CanbanVoimaxHarness Rust backend.
 * Guarantees 100% type safety for listeners and eliminates any-typed event payloads.
 */
export interface AgentEventMap {
  'agent:thought': AgentThoughtPayload;
  'agent:output': AgentOutputPayload;
  'agent:tool_call': AgentToolCallPayload;
  'agent:tool_result': AgentToolResultPayload;
  'agent:question': AgentQuestionPayload;
  'agent:telemetry': AgentTelemetryPayload;
  'agent:turn_start': AgentTurnStartPayload;
  'agent:stream_start': AgentStreamStartPayload;
  'agent:stream_retry': AgentStreamRetryPayload;
  'agent:finished': AgentFinishedPayload;
  'agent:error': AgentErrorPayload;
  'agent:status': AgentStatusPayload;
  'agent:aborted': AgentAbortedPayload;
  'agent:steer': AgentSteerPayload;
  'agent:session_status': AgentSessionSummary;
  'agent:autorun': AgentAutoRunPayload;
  'agent:compacted': AgentCompactedPayload;
  'task:created': TaskCreatedPayload;
  'task:moved': TaskMovedPayload;
  'task:deleted': TaskDeletedPayload;
  'task:advanced': TaskAdvancedPayload;
  'task:activity': TaskActivityPayload;
  'task:dialog': TaskActivityPayload;
  'dialog:updated': DialogUpdatedPayload;
  'task:updated': TaskUpdatedPayload;
  'tasks:updated': TasksUpdatedPayload;
  'hook:start': HookStartPayload;
  'hook:finish': HookFinishPayload;
  'project:changed': ProjectSwitchedPayload;
  'project:switched': ProjectSwitchedPayload;
  'agent:personas_updated': Record<string, unknown> | undefined;
  'boards:updated': Record<string, unknown> | undefined;
  'pipeline:updated': Record<string, unknown> | undefined;
  'cortex:rules_updated': Record<string, unknown> | undefined;
  'subagent:spawned': SubagentSpawnedPayload;
  'subagent:activity': SubagentActivityPayload;
  'subagent:finished': SubagentFinishedPayload;
  'subagent:error': SubagentErrorPayload;
  'subagent:status': SubagentStatusPayload;
  'subagent:aborted': SubagentAbortedPayload;
}

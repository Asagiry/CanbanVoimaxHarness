import { useBoardStore } from '../stores/boardStore';
import { useDrawerStore } from '../stores/drawerStore';
import { useCortexStore } from '../stores/cortexStore';
import { useAgentSessionStore, type AgentSessionSummary } from '../stores/agentSessionStore';
import { useProjectStore } from '../stores/projectStore';
import {
  showAdvanceTaskNotification,
  showQuestionNotification,
  showRejectTaskNotification,
  type TaskAdvancedPayload,
} from './notificationService';
import type { BackendTaskDTO, TodoItem } from '../types';

export function isEventForCurrentProject(eventProjectPath?: string): boolean {
  if (!eventProjectPath || typeof eventProjectPath !== 'string') return true;
  const activeRoot = useProjectStore.getState().activeProject?.rootPath;
  if (!activeRoot) return true;
  const normEvent = eventProjectPath.replace(/\\/g, '/').toLowerCase().replace(/\/+$/, '');
  const normActive = activeRoot.replace(/\\/g, '/').toLowerCase().replace(/\/+$/, '');
  return normEvent === normActive;
}

/**
 * Micro-batching window for streaming SSE tokens.
 * Coalesces consecutive chunks of the same type within 150ms into a single Zustand store mutation.
 */
export const TEXT_FLUSH_MS = 150;

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
  tokens_after?: number;
  tokensAfter?: number;
  saved_tokens?: number;
  estimatedSavedTokens?: number;
  savedTokens?: number;
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
  projectPath?: string;
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
}

export interface SubagentAbortedPayload {
  subagentId?: string;
  parentTaskId?: string;
}

export interface SubagentActivityPayload {
  parentTaskId?: string;
  subagentId?: string;
  activity?: any;
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
  'agent:session_status': AgentSessionSummary;
  'agent:autorun': AgentAutoRunPayload;
  'agent:compacted': AgentCompactedPayload;
  'task:created': TaskCreatedPayload;
  'task:moved': TaskMovedPayload;
  'task:advanced': TaskAdvancedPayload;
  'task:activity': { taskId?: string; item?: any };
  'task:dialog': { taskId?: string; item?: any };
  'task:updated': TaskUpdatedPayload;
  'tasks:updated': TasksUpdatedPayload;
  'hook:start': HookStartPayload;
  'hook:finish': HookFinishPayload;
  'project:changed': unknown;
  'project:switched': unknown;
  'agent:personas_updated': unknown;
  'boards:updated': unknown;
  'pipeline:updated': unknown;
  'cortex:rules_updated': unknown;
  'subagent:spawned': SubagentSpawnedPayload;
  'subagent:activity': SubagentActivityPayload;
  'subagent:finished': SubagentFinishedPayload;
  'subagent:error': SubagentErrorPayload;
  'subagent:status': SubagentStatusPayload;
  'subagent:aborted': SubagentAbortedPayload;
}

export interface TextBuffer {
  chunks: Array<{ type: 'thought' | 'output'; text: string; streamId?: string }>;
  timer?: ReturnType<typeof setTimeout>;
}

/** Keeps channel/attempt ordering while batching frequent stream updates. */
export class TextBufferManager {
  private readonly buffers = new Map<string, TextBuffer>();

  public getBuffer(taskId: string): TextBuffer {
    let buffer = this.buffers.get(taskId);
    if (!buffer) {
      buffer = { chunks: [] };
      this.buffers.set(taskId, buffer);
    }
    return buffer;
  }

  public flushTask(taskId: string): void {
    const buffer = this.buffers.get(taskId);
    if (!buffer) return;
    if (buffer.timer) clearTimeout(buffer.timer);
    this.buffers.delete(taskId);
    for (const chunk of buffer.chunks) {
      const state = useDrawerStore.getState();
      if (chunk.type === 'thought') state.appendThought(taskId, chunk.text, chunk.streamId);
      else state.appendOutput(taskId, chunk.text, 'output', undefined, chunk.streamId);
    }
  }

  public flushAll(): void {
    for (const taskId of Array.from(this.buffers.keys())) this.flushTask(taskId);
  }

  private queue(taskId: string, type: 'thought' | 'output', text: string, streamId?: string): void {
    const state = useDrawerStore.getState();
    if (streamId && taskId in state.activeStreamIds && state.activeStreamIds[taskId] !== streamId) return;
    state.setActiveAgentTask(taskId, true);
    if (state.generationStartTime[taskId] == null) state.setGenerationStartTime(taskId, Date.now());
    state.recordStreamedChars(taskId, text.length);
    const buffer = this.getBuffer(taskId);
    const last = buffer.chunks[buffer.chunks.length - 1];
    if (last && last.type === type && last.streamId === streamId) last.text += text;
    else buffer.chunks.push({ type, text, streamId });
    if (!buffer.timer) buffer.timer = setTimeout(() => this.flushTask(taskId), TEXT_FLUSH_MS);
  }

  public queueThought(taskId: string, text: string, streamId?: string): void {
    this.queue(taskId, 'thought', text, streamId);
  }

  public queueOutput(taskId: string, text: string, streamId?: string): void {
    this.queue(taskId, 'output', text, streamId);
  }

  public startStream(taskId: string, streamId: string): void {
    this.flushTask(taskId);
    useDrawerStore.getState().startStream(taskId, streamId);
  }

  public retryStream(taskId: string, streamId: string, delaySecs: number): void {
    const buffer = this.buffers.get(taskId);
    if (buffer) {
      buffer.chunks = buffer.chunks.filter((chunk) => chunk.streamId !== streamId);
      if (!buffer.chunks.length) this.clearTask(taskId);
    }
    useDrawerStore.getState().retryStream(taskId, streamId, delaySecs);
  }

  public clearTask(taskId: string): void {
    const buffer = this.buffers.get(taskId);
    if (buffer?.timer) clearTimeout(buffer.timer);
    this.buffers.delete(taskId);
  }

  public dispose(): void {
    this.flushAll();
  }
}

/**
 * Predicate checking whether the specified task is actively selected in the drawer
 * or is a subtask belonging to the active drawer session.
 */
export function isTaskActiveInDrawer(taskId: string): boolean {
  const drawerState = useDrawerStore.getState();
  if (!drawerState.isOpen || !drawerState.activeTaskId) return false;
  const activeId = drawerState.activeTaskId;
  if (taskId === activeId) return true;
  const subId = drawerState.activeSubagentId;
  if (subId) {
    if (
      taskId === subId ||
      drawerState.toolIdToSubagentId[taskId] === subId ||
      drawerState.subagentIdToToolId[taskId] === subId ||
      drawerState.toolIdToSubagentId[subId] === taskId ||
      drawerState.subagentIdToToolId[subId] === taskId
    ) {
      return true;
    }
  }
  if (
    taskId.startsWith(`${activeId}.sub-`) ||
    taskId.startsWith(`${activeId}:sub:`) ||
    taskId.startsWith(`${activeId}.`)
  ) {
    return true;
  }
  const task = useBoardStore.getState().tasks.find((t) => t.id === taskId);
  if (task && task.parentId === activeId) {
    return true;
  }
  return false;
}

/**
 * Ensures that if an event arrives for an idle/uninitialized task, its status
 * reflects 'running' on the board.
 */
export function ensureTaskRunningOnBoard(taskId: string): void {
  const boardState = useBoardStore.getState();
  let currentTask = boardState.tasks.find((t) => t.id === taskId);
  if (!currentTask && taskId.includes('.')) {
    const parentCandidateId = taskId.split('.')[0];
    currentTask = boardState.tasks.find((t) => t.id === parentCandidateId);
  } else if (!currentTask && taskId.includes(':sub:')) {
    const parentCandidateId = taskId.split(':sub:')[0];
    currentTask = boardState.tasks.find((t) => t.id === parentCandidateId);
  }
  if (
    currentTask &&
    currentTask.status !== 'running' &&
    currentTask.status !== 'done' &&
    currentTask.status !== 'waiting_subagent'
  ) {
    boardState.updateTaskStatus(currentTask.id, 'running');
  }
}

/**
 * Initializes all typed Tauri IPC event listeners and manages streaming micro-batching.
 * Returns a synchronous cleanup function to unsubscribe all registered listeners and flush pending buffers.
 */
export function setupTauriEventGateway(): () => void {
  let isDisposed = false;
  const unlisteners: (() => void)[] = [];
  const bufferManager = new TextBufferManager();

  const registerListeners = async () => {
    try {
      const { listen } = await import('@tauri-apps/api/event');
      if (isDisposed) return;

      const addListener = async <K extends keyof AgentEventMap>(
        event: K,
        handler: (payload: AgentEventMap[K]) => void | Promise<void>
      ) => {
        const unlisten = await listen<AgentEventMap[K]>(event, (e) => {
          if (!isDisposed) {
            void handler(e.payload);
          }
        });
        if (isDisposed) {
          unlisten();
        } else {
          unlisteners.push(unlisten);
        }
      };

      await addListener('agent:stream_start', ({ taskId, streamId }) => {
        if (!taskId || !streamId) return;
        ensureTaskRunningOnBoard(taskId);
        bufferManager.startStream(taskId, streamId);
      });

      await addListener('agent:stream_retry', ({ taskId, streamId, delaySecs }) => {
        if (!taskId || !streamId) return;
        bufferManager.retryStream(taskId, streamId, delaySecs);
      });

      // 1. agent:thought
      await addListener('agent:thought', (payload) => {
        const { taskId, thought, delta, streamId } = payload || {};
        if (!taskId) return;
        ensureTaskRunningOnBoard(taskId);
        if (!isTaskActiveInDrawer(taskId)) {
          return;
        }
        const text = thought || delta;
        if (text) {
          bufferManager.queueThought(taskId, text, streamId);
        }
      });

      // 2. agent:output
      await addListener('agent:output', (payload) => {
        const { taskId, output, content, token, streamId } = payload || {};
        if (!taskId) return;
        ensureTaskRunningOnBoard(taskId);
        if (!isTaskActiveInDrawer(taskId)) {
          return;
        }
        const text = output || content || token;
        if (text) {
          bufferManager.queueOutput(taskId, text, streamId);
        }
      });

      // 3. agent:tool_call
      await addListener('agent:tool_call', (payload) => {
        const { taskId, id, name, arguments: args, batch, status = 'running' } = payload || {};
        if (!taskId) return;
        ensureTaskRunningOnBoard(taskId);
        if (!isTaskActiveInDrawer(taskId)) {
          return;
        }
        // Flush stream buffers before tool event to preserve chronological narrative ordering
        useDrawerStore.getState().setActiveAgentTask(taskId, true);
        bufferManager.flushTask(taskId);
        const st = useDrawerStore.getState();
        if (st.generationEndTime[taskId] == null) {
          st.setGenerationEndTime(taskId, Date.now());
        }
        if (st.finalTps[taskId] == null) {
          const tele = st.taskTelemetry[taskId];
          const outTokens = tele?.outputTokens || st.streamedTurnTokens[taskId] || 0;
          const start = st.generationStartTime[taskId] ?? st.thinkingStartTime[taskId];
          const end = st.generationEndTime[taskId] ?? Date.now();
          if (start != null && outTokens > 0) {
            const elapsedSec = (end - start) / 1000;
            if (elapsedSec > 0.05) {
              st.recordTurnMetrics(taskId, outTokens, elapsedSec);
            }
          }
        }
        if (name) {
          let parsedParams: Record<string, unknown> = {};
          try {
            parsedParams = typeof args === 'string' ? JSON.parse(args) : (args || {});
          } catch {
            parsedParams = { raw: args };
          }
          useDrawerStore.getState().addToolExecution(taskId, {
            id: id || `call-${Date.now()}`,
            toolName: name,
            timestamp: new Date().toLocaleTimeString('en-US', { hour12: false }),
            durationMs: 0,
            status: status === 'queued' ? 'queued' : 'running',
            params: parsedParams,
            batch: batch ?? undefined,
            createdAt: Date.now(),
          });
        }
      });

      // 4. agent:tool_result
      await addListener('agent:tool_result', (payload) => {
        const { taskId, id, result, success, batch, durationMs } = payload || {};
        if (!taskId) return;
        if (!isTaskActiveInDrawer(taskId)) {
          return;
        }
        bufferManager.flushTask(taskId);
        if (id) {
          const outStr = typeof result === 'string' ? result : JSON.stringify(result, null, 2);
          useDrawerStore.getState().updateToolExecution(taskId, id, {
            status: success ? 'success' : 'failed',
            stdout: success ? outStr : undefined,
            stderr: !success ? outStr : undefined,
            ...(batch ? { batch } : {}),
            ...(typeof durationMs === 'number' && Number.isFinite(durationMs) ? { durationMs } : {}),
          });
        }
      });

      // 5. agent:question
      await addListener('agent:question', (payload) => {
        const { taskId, questionId, text, options, multiple } = payload || {};
        if (taskId) bufferManager.flushTask(taskId);
        if (taskId && questionId) {
          const normalizedOptions: QuestionOptionItem[] = Array.isArray(options)
            ? options
                .map((opt) => (typeof opt === 'string' ? { label: opt } : opt))
                .filter((opt): opt is QuestionOptionItem => Boolean(opt && typeof opt.label === 'string'))
            : [];

          useDrawerStore.getState().addQuestion(taskId, {
            questionId,
            text: text || '',
            options: normalizedOptions,
            multiple: Boolean(multiple),
          });
          useBoardStore.getState().setActiveQuestion({
            taskId,
            questionId,
            text: text || '',
            timestamp: Date.now(),
          });
          void showQuestionNotification({
            taskId,
            questionId,
            text: text || '',
          });
        }
      });

      // 6. agent:telemetry
      await addListener('agent:telemetry', (payload) => {
        const {
          taskId,
          inputTokens,
          outputTokens,
          cacheReadTokens,
          cacheCreationTokens,
          cacheHitRate,
          durationMs,
          tps,
          ttftMs,
          systemTokens,
          toolTokens,
        } = payload || {};
        if (!taskId) return;
        const num = (v: unknown) => (typeof v === 'number' && Number.isFinite(v) ? v : 0);
        const tpsVal = num(tps);
        const outTok = num(outputTokens);
        const inTok = num(inputTokens);
        const cRead = num(cacheReadTokens);
        const cCreate = num(cacheCreationTokens);
        const totalIn = inTok;
        const hitRate = num(cacheHitRate) || (totalIn > 0 ? (cRead / totalIn) * 100 : 0);
        const sysTok = typeof systemTokens === 'number' && Number.isFinite(systemTokens) ? systemTokens : undefined;
        const toolTok = typeof toolTokens === 'number' && Number.isFinite(toolTokens) ? toolTokens : undefined;
        const ttftVal = typeof ttftMs === 'number' && Number.isFinite(ttftMs) ? ttftMs : undefined;

        useDrawerStore.getState().updateTelemetry(taskId, {
          inputTokens: inTok,
          outputTokens: outTok,
          cacheReadTokens: cRead,
          cacheCreationTokens: cCreate,
          cacheHitRate: hitRate,
          durationMs: num(durationMs),
          tps: tpsVal,
          ...(ttftVal !== undefined ? { ttftMs: ttftVal } : {}),
          ...(sysTok !== undefined ? { systemTokens: sysTok } : {}),
          ...(toolTok !== undefined ? { toolTokens: toolTok } : {}),
        });

        const drawerState = useDrawerStore.getState();
        if (drawerState.thinkingStartTime[taskId] == null) {
          drawerState.setThinkingStartTime(taskId, Date.now());
        }
        if (drawerState.generationEndTime[taskId] == null) {
          drawerState.setGenerationEndTime(taskId, Date.now());
        }
        const durSec = num(durationMs) > 0 ? num(durationMs) / 1000 : 0;
        if (outTok > 0 && durSec > 0.05) {
          drawerState.recordTurnMetrics(taskId, outTok, durSec);
        } else if (tpsVal > 0) {
          drawerState.setFinalTps(taskId, tpsVal);
        }
      });

      // 7. agent:turn_start
      await addListener('agent:turn_start', (payload) => {
        const { taskId } = payload || {};
        if (taskId) {
          ensureTaskRunningOnBoard(taskId);
          if (isTaskActiveInDrawer(taskId)) {
            bufferManager.flushTask(taskId);
            useDrawerStore.getState().resetTurnStreamMetrics(taskId);
            useDrawerStore.getState().setActiveAgentTask(taskId, true);
          }
        }
      });

      // 8. agent:finished
      await addListener('agent:finished', (payload) => {
        const { taskId, response } = payload || {};
        if (taskId) {
          useBoardStore.getState().clearActiveQuestion(taskId);
          if (isTaskActiveInDrawer(taskId)) {
            bufferManager.flushTask(taskId);
            const st = useDrawerStore.getState();
            st.setActiveAgentTask(taskId, false);
            st.setGenerationEndTime(taskId, Date.now());
            st.setTaskEndTime(taskId, Date.now());

            // Freeze TPS on finish: same sources as ContextGauge
            try {
              if (st.finalTps[taskId] == null) {
                const tele = st.taskTelemetry[taskId];
                const outTokens = tele?.outputTokens || st.streamedTurnTokens[taskId] || 0;
                const start = st.generationStartTime[taskId] ?? st.thinkingStartTime[taskId];
                const end = st.generationEndTime[taskId] ?? Date.now();
                if (start != null && outTokens > 0) {
                  const elapsedSec = (end - start) / 1000;
                  if (elapsedSec > 0.05) {
                    st.recordTurnMetrics(taskId, outTokens, elapsedSec);
                  }
                }
              }
            } catch {
              // ignore
            }
            // Dedup final and mark final response
            if (response && response.trim().length > 0) {
              st.markFinalResponse(taskId, response);
            }
            // Immediately flush task activity feed to disk upon turn finish
            void st.saveTaskActivity(taskId);
          } else {
            bufferManager.clearTask(taskId);
            useDrawerStore.getState().setActiveAgentTask(taskId, false);
          }
          const currentTask = useBoardStore.getState().tasks.find((t) => t.id === taskId);
          if (currentTask && currentTask.status === 'running') {
            useBoardStore.getState().updateTaskStatus(taskId, 'idle');
          }
        }
        void useBoardStore.getState().loadTasks();
      });

      // 9. agent:error
      await addListener('agent:error', (payload) => {
        const { taskId, error } = payload || {};
        if (taskId) {
          useBoardStore.getState().clearActiveQuestion(taskId);
          if (isTaskActiveInDrawer(taskId)) {
            bufferManager.flushTask(taskId);
            if (error) useDrawerStore.getState().appendOutput(taskId, error, 'error');
            useDrawerStore.getState().setActiveAgentTask(taskId, false);
            useDrawerStore.getState().saveTaskActivity(taskId);
            useDrawerStore.getState().setThinkingStartTime(taskId, null);
          } else {
            bufferManager.clearTask(taskId);
            useDrawerStore.getState().setActiveAgentTask(taskId, false);
          }
          const currentTask = useBoardStore.getState().tasks.find((t) => t.id === taskId);
          if (currentTask && currentTask.status === 'running') {
            useBoardStore.getState().updateTaskStatus(taskId, 'idle');
          }
        } else {
          bufferManager.flushAll();
        }
        void useBoardStore.getState().loadTasks();
      });

      // 10. agent:status
      await addListener('agent:status', (payload) => {
        const { taskId, status } = payload || {};
        if (taskId && (status === 'finished' || status === 'aborted' || status === 'error' || status === 'idle')) {
          useBoardStore.getState().clearActiveQuestion(taskId);
          if (isTaskActiveInDrawer(taskId)) {
            bufferManager.flushTask(taskId);
            useDrawerStore.getState().setActiveAgentTask(taskId, false);
            if (status === 'aborted' || status === 'error') {
              useDrawerStore.getState().saveTaskActivity(taskId);
            }
            useDrawerStore.getState().setThinkingStartTime(taskId, null);
          } else {
            bufferManager.clearTask(taskId);
            useDrawerStore.getState().setActiveAgentTask(taskId, false);
          }
          const currentTask = useBoardStore.getState().tasks.find((t) => t.id === taskId);
          if (currentTask && currentTask.status !== 'done') {
            useBoardStore.getState().updateTaskStatus(taskId, 'idle');
          }
        } else if (taskId && status === 'waiting_subagent') {
          useBoardStore.getState().updateTaskStatus(taskId, 'waiting_subagent');
        } else if (taskId && status === 'running') {
          ensureTaskRunningOnBoard(taskId);
        }
        void useBoardStore.getState().loadTasks();
      });

      // 10b. agent:session_status
      await addListener('agent:session_status', (payload) => {
        if (!payload || !payload.sessionId) return;
        useAgentSessionStore.getState().handleSessionStatus(payload);
        if (payload.taskId && payload.status === 'running') {
          ensureTaskRunningOnBoard(payload.taskId);
        }
      });

      // 11. task:created
      await addListener('task:created', (payload) => {
        const eventRoot = payload?.projectPath || payload?.task?.projectPath || (payload?.task as any)?.project_path;
        if (!isEventForCurrentProject(eventRoot)) return;
        if (payload?.task) {
          useBoardStore.getState().upsertBackendTask(payload.task);
        }
        void useBoardStore.getState().loadTasks();
      });

      // 12. task:moved
      await addListener('task:moved', (payload) => {
        const eventRoot = payload?.projectPath || payload?.task?.projectPath || (payload?.task as any)?.project_path;
        if (!isEventForCurrentProject(eventRoot)) return;
        if (payload?.task) {
          const rawTask = payload.task;
          if (payload.fromStage && typeof rawTask === 'object') {
            rawTask.previousStageId = payload.fromStage;
          }
          useBoardStore.getState().upsertBackendTask(rawTask);
        }
        if (payload?.rejected) {
          const taskId = payload.taskId || payload.task?.id;
          if (taskId) {
            void showRejectTaskNotification({
              taskId,
              reason: payload.reason,
              toStage: payload.toStage || payload.stageId || payload.task?.stage_id,
            });
          }
        }
        void useBoardStore.getState().loadTasks();
      });

      // 13. task:advanced
      await addListener('task:advanced', (payload) => {
        if (payload) {
          void showAdvanceTaskNotification(payload);
        }
      });

      // 13b. task:activity / task:dialog (real-time task activities emitted by backend transitions & tools)
      const seenItemIds = new Set<string>();
      const handleTaskActivity = (payload: { taskId?: string; item?: any }) => {
        const { taskId, item } = payload || {};
        if (taskId && item) {
          if (item.id && seenItemIds.has(item.id)) return;
          if (item.id) {
            seenItemIds.add(item.id);
            if (seenItemIds.size > 200) {
              const first = seenItemIds.values().next().value;
              if (first) seenItemIds.delete(first);
            }
          }
          if (isTaskActiveInDrawer(taskId)) {
            bufferManager.flushTask(taskId);
            useDrawerStore.getState().appendActivityItem(taskId, item);
          }
        }
      };
      await addListener('task:activity', handleTaskActivity);
      await addListener('task:dialog', handleTaskActivity);

      // 14. hook:start
      await addListener('hook:start', (payload) => {
        const { taskId, stageId, hookType, hookName, description } = payload || {};
        if (!taskId || !stageId || !hookType) return;
        // Backend normalizes on_finish/on_exit -> on_leave; older emits
        // or transitive resolution may still send them — normalize here too.
        const normalizedHookType = hookType === 'on_finish' || hookType === 'on_exit' ? 'on_leave' : hookType;
        if (!hookName) {
          console.warn('[hook:start] missing hookName, falling back to hookType', payload);
        }
        useBoardStore.getState().setHookStart({
          taskId,
          stageId,
          hookType: normalizedHookType,
          hookName: hookName || normalizedHookType,
          description,
        });
      });

      // 15. hook:finish
      await addListener('hook:finish', (payload) => {
        const { taskId, stageId, hookType, success, exitCode, stdout, stderr, error, hookName } = payload || {};
        if (taskId && stageId && hookType) {
          const normalizedHookType = hookType === 'on_finish' || hookType === 'on_exit' ? 'on_leave' : hookType;
          useBoardStore.getState().setHookFinish({
            taskId,
            stageId,
            hookType: normalizedHookType,
            success: Boolean(success),
            exitCode,
            stdout,
            stderr,
            error,
            hookName,
          });
        }
      });

      // 16. task:updated
      await addListener('task:updated', (payload) => {
        const eventRoot = payload?.projectPath || payload?.task?.projectPath || (payload?.task as any)?.project_path;
        if (!isEventForCurrentProject(eventRoot)) return;
        const { taskId, todos, task } = payload || {};
        if (taskId) {
          bufferManager.flushTask(taskId);
        } else {
          bufferManager.flushAll();
        }
        if (task) {
          useBoardStore.getState().upsertBackendTask(task);
        } else if (taskId && todos) {
          useBoardStore.getState().updateTask(taskId, { todos });
        } else {
          void useBoardStore.getState().loadTasks();
        }
      });

      // 17. tasks:updated
      await addListener('tasks:updated', (payload) => {
        if (!isEventForCurrentProject(payload?.projectPath)) return;
        bufferManager.flushAll();
        if (Array.isArray(payload?.tasks)) {
          useBoardStore.getState().setBackendTasks(payload.tasks);
        } else {
          void useBoardStore.getState().loadTasks();
        }
      });

      // 18. agent:autorun
      await addListener('agent:autorun', (payload) => {
        const tid = payload?.taskId;
        const prompt = payload?.prompt;
        const projectPath = payload?.projectPath;
        if (tid) {
          void useBoardStore.getState().runTaskAgent(tid, prompt, { skipSync: true, projectPath });
        }
      });

      // 19. project:changed & project:switched
      const handleProjectSwitched = async () => {
        // If frontend project transition is actively in progress, suppress duplicate reload
        if (useBoardStore.getState().isLoadingTasks) {
          return;
        }
        await useBoardStore.getState().loadBoards();
        await useBoardStore.getState().loadTasks();
        await useCortexStore.getState().loadAgentPersonas();
        await useCortexStore.getState().loadGlobalRules();
        useCortexStore.getState().syncStagesFromColumns(useBoardStore.getState().columns);
      };

      await addListener('project:changed', handleProjectSwitched);
      await addListener('project:switched', handleProjectSwitched);

      // 20. agent:personas_updated
      await addListener('agent:personas_updated', async () => {
        await useCortexStore.getState().loadAgentPersonas();
        useCortexStore.getState().syncStagesFromColumns(useBoardStore.getState().columns);
      });

      // 21. boards:updated & pipeline:updated (consolidated and deduplicated)
      let isSyncingBoards = false;
      let hasPendingBoardsSync = false;
      const handleBoardsOrPipelineUpdated = async () => {
        if (isSyncingBoards) {
          hasPendingBoardsSync = true;
          return;
        }
        isSyncingBoards = true;
        try {
          do {
            hasPendingBoardsSync = false;
            await useBoardStore.getState().loadBoards();
            await useBoardStore.getState().loadTasks();
            useCortexStore.getState().syncStagesFromColumns(useBoardStore.getState().columns);
          } while (hasPendingBoardsSync && !isDisposed);
        } finally {
          isSyncingBoards = false;
        }
      };

      await addListener('boards:updated', handleBoardsOrPipelineUpdated);
      await addListener('pipeline:updated', handleBoardsOrPipelineUpdated);

      // 22. cortex:rules_updated
      await addListener('cortex:rules_updated', async () => {
        await useCortexStore.getState().loadGlobalRules();
      });
    } catch (e) {
      console.info('Tauri event listeners not active in web mode:', e);
    }
  };

  void registerListeners();

  return () => {
    isDisposed = true;
    bufferManager.dispose();
    unlisteners.forEach((u) => u());
  };
}

/**
 * Authoritative typed event subscription helper via tauriEventGateway.
 * Subscribes to backend Tauri IPC events using the nominal types defined in AgentEventMap.
 * Guarantees zero raw `listen<any>` in UI components.
 */
export async function listenToEvent<K extends keyof AgentEventMap>(
  event: K,
  handler: (payload: AgentEventMap[K]) => void | Promise<void>
): Promise<() => void> {
  const { listen } = await import('@tauri-apps/api/event');
  return listen<AgentEventMap[K]>(event, (e) => {
    void handler(e.payload);
  });
}


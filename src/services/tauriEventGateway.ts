import { useBoardStore, initBoardEventListeners } from '../stores/boardStore';
import { useDrawerStore } from '../stores/drawerStore';
import { useCortexStore, initCortexEventListeners } from '../stores/cortexStore';
import { useAgentSessionStore, type AgentSessionSummary } from '../stores/agentSessionStore';
import {
  showAdvanceTaskNotification,
  showQuestionNotification,
  showRejectTaskNotification,
  type TaskAdvancedPayload,
} from './notificationService';
import type { BackendTaskDTO, TodoItem } from '../types';

/**
 * Micro-batching window for streaming SSE tokens.
 * Coalesces consecutive chunks of the same type within 150ms into a single Zustand store mutation.
 */
export const TEXT_FLUSH_MS = 150;

export interface AgentThoughtPayload {
  taskId?: string;
  thought?: string;
  delta?: string;
}

export interface AgentOutputPayload {
  taskId?: string;
  output?: string;
  content?: string;
  token?: string;
}

export interface AgentToolCallPayload {
  taskId?: string;
  id?: string;
  name?: string;
  arguments?: string | Record<string, unknown>;
}

export interface AgentToolResultPayload {
  taskId?: string;
  id?: string;
  result?: unknown;
  success?: boolean;
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
}

export interface AgentAutoRunPayload {
  taskId?: string;
  prompt?: string;
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
}

export interface HookStartPayload {
  taskId?: string;
  stageId?: string;
  hookType?: 'on_enter' | 'on_leave';
  hookName?: string;
  description?: string;
}

export interface HookFinishPayload {
  taskId?: string;
  stageId?: string;
  hookType?: 'on_enter' | 'on_leave';
  success?: boolean;
  exitCode?: number | null;
  stdout?: string;
  stderr?: string;
  error?: string;
}

export interface TaskUpdatedPayload {
  taskId?: string;
  todos?: TodoItem[];
  task?: BackendTaskDTO;
}

export interface TasksUpdatedPayload {
  tasks?: BackendTaskDTO[];
}

export interface SubagentSpawnedPayload {
  parentTaskId?: string;
  subagentId?: string;
  title?: string;
  role?: string;
  prompt?: string;
  tools?: string[];
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
  'agent:finished': AgentFinishedPayload;
  'agent:error': AgentErrorPayload;
  'agent:status': AgentStatusPayload;
  'agent:session_status': AgentSessionSummary;
  'agent:autorun': AgentAutoRunPayload;
  'agent:compacted': AgentCompactedPayload;
  'task:created': TaskCreatedPayload;
  'task:moved': TaskMovedPayload;
  'task:advanced': TaskAdvancedPayload;
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
  'subagent:finished': SubagentFinishedPayload;
  'subagent:error': SubagentErrorPayload;
  'subagent:status': SubagentStatusPayload;
  'subagent:aborted': SubagentAbortedPayload;
}

export interface TextBuffer {
  thought: string;
  output: string;
  thoughtTimer?: ReturnType<typeof setTimeout>;
  outputTimer?: ReturnType<typeof setTimeout>;
}

/**
 * Manages streaming text buffers per task to avoid high-frequency React re-renders.
 * Coalesces consecutive chunks of thought/output channels within TEXT_FLUSH_MS.
 */
export class TextBufferManager {
  private readonly buffers = new Map<string, TextBuffer>();

  public getBuffer(taskId: string): TextBuffer {
    let b = this.buffers.get(taskId);
    if (!b) {
      b = { thought: '', output: '' };
      this.buffers.set(taskId, b);
    }
    return b;
  }

  public flushThought(taskId: string): void {
    const b = this.buffers.get(taskId);
    if (!b) return;
    if (b.thoughtTimer) {
      clearTimeout(b.thoughtTimer);
      b.thoughtTimer = undefined;
    }
    if (b.thought) {
      const chunk = b.thought;
      b.thought = '';
      try {
        useDrawerStore.getState().appendThought(taskId, chunk);
      } catch {
        // ignore
      }
    }
  }

  public flushOutput(taskId: string): void {
    const b = this.buffers.get(taskId);
    if (!b) return;
    if (b.outputTimer) {
      clearTimeout(b.outputTimer);
      b.outputTimer = undefined;
    }
    if (b.output) {
      const chunk = b.output;
      b.output = '';
      try {
        useDrawerStore.getState().appendOutput(taskId, chunk, 'output');
      } catch {
        // ignore
      }
    }
  }

  public flushTask(taskId: string): void {
    this.flushThought(taskId);
    this.flushOutput(taskId);
  }

  public flushAll(): void {
    for (const id of Array.from(this.buffers.keys())) {
      this.flushTask(id);
    }
  }

  public queueThought(taskId: string, text: string): void {
    const ds = useDrawerStore.getState();
    ds.setActiveAgentTask(taskId, true);
    if (ds.generationStartTime[taskId] == null) {
      ds.setGenerationStartTime(taskId, Date.now());
    }
    ds.recordStreamedChars(taskId, text.length);
    const b = this.getBuffer(taskId);
    b.thought += text;
    if (!b.thoughtTimer) {
      b.thoughtTimer = setTimeout(() => {
        this.flushThought(taskId);
      }, TEXT_FLUSH_MS);
    }
  }

  public queueOutput(taskId: string, text: string): void {
    const ds = useDrawerStore.getState();
    ds.setActiveAgentTask(taskId, true);
    if (ds.generationStartTime[taskId] == null) {
      ds.setGenerationStartTime(taskId, Date.now());
    }
    ds.recordStreamedChars(taskId, text.length);
    const b = this.getBuffer(taskId);
    b.output += text;
    if (!b.outputTimer) {
      b.outputTimer = setTimeout(() => {
        this.flushOutput(taskId);
      }, TEXT_FLUSH_MS);
    }
  }

  public clearTask(taskId: string): void {
    const b = this.buffers.get(taskId);
    if (b) {
      if (b.thoughtTimer) clearTimeout(b.thoughtTimer);
      if (b.outputTimer) clearTimeout(b.outputTimer);
      this.buffers.delete(taskId);
    }
  }

  public dispose(): void {
    this.flushAll();
    for (const b of this.buffers.values()) {
      if (b.thoughtTimer) clearTimeout(b.thoughtTimer);
      if (b.outputTimer) clearTimeout(b.outputTimer);
    }
    this.buffers.clear();
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
    currentTask.status !== 'waiting_subtask'
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

      // 1. agent:thought
      await addListener('agent:thought', (payload) => {
        const { taskId, thought, delta } = payload || {};
        if (!taskId) return;
        ensureTaskRunningOnBoard(taskId);
        if (!isTaskActiveInDrawer(taskId)) {
          return;
        }
        const text = thought || delta;
        if (text) {
          bufferManager.queueThought(taskId, text);
        }
      });

      // 2. agent:output
      await addListener('agent:output', (payload) => {
        const { taskId, output, content, token } = payload || {};
        if (!taskId) return;
        ensureTaskRunningOnBoard(taskId);
        if (!isTaskActiveInDrawer(taskId)) {
          return;
        }
        const text = output || content || token;
        if (text) {
          bufferManager.queueOutput(taskId, text);
        }
      });

      // 3. agent:tool_call
      await addListener('agent:tool_call', (payload) => {
        const { taskId, id, name, arguments: args } = payload || {};
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
            status: 'running',
            params: parsedParams,
          });
        }
      });

      // 4. agent:tool_result
      await addListener('agent:tool_result', (payload) => {
        const { taskId, id, result, success } = payload || {};
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
        const totalIn = inTok + cRead;
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
            st.markFinalResponse(taskId, response);
            // Immediately flush task activity feed to disk upon turn finish
            st.flushSaveDialog(taskId);
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
        const { taskId } = payload || {};
        if (taskId) {
          useBoardStore.getState().clearActiveQuestion(taskId);
          if (isTaskActiveInDrawer(taskId)) {
            bufferManager.flushTask(taskId);
            useDrawerStore.getState().setActiveAgentTask(taskId, false);
            useDrawerStore.getState().flushSaveDialog(taskId);
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
        const { taskId, status, response } = payload || {};
        if (taskId && (status === 'finished' || status === 'aborted' || status === 'error')) {
          useBoardStore.getState().clearActiveQuestion(taskId);
          if (isTaskActiveInDrawer(taskId)) {
            bufferManager.flushTask(taskId);
            useDrawerStore.getState().setActiveAgentTask(taskId, false);
            if (status === 'finished' && response && typeof response === 'string' && response.trim().length > 0) {
              const trimmedResp = response.trim();
              const feed = useDrawerStore.getState().activityFeed[taskId] || [];
              const alreadyHas = feed.some(
                (item) =>
                  (item.type === 'output' || (item.type as string) === 'summary') &&
                  (item.content?.trim() === trimmedResp || item.content?.includes(trimmedResp))
              );
              if (!alreadyHas) {
                useDrawerStore.getState().markFinalResponse(taskId, trimmedResp);
              }
            }
            useDrawerStore.getState().flushSaveDialog(taskId);
            useDrawerStore.getState().setThinkingStartTime(taskId, null);
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
        if (payload?.task) {
          useBoardStore.getState().upsertBackendTask(payload.task);
        }
        void useBoardStore.getState().loadTasks();
      });

      // 12. task:moved
      await addListener('task:moved', (payload) => {
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

      // 14. hook:start
      await addListener('hook:start', (payload) => {
        const { taskId, stageId, hookType, hookName, description } = payload || {};
        if (taskId && stageId && hookType && hookName) {
          useBoardStore.getState().setHookStart({ taskId, stageId, hookType, hookName, description });
        }
      });

      // 15. hook:finish
      await addListener('hook:finish', (payload) => {
        const { taskId, stageId, hookType, success, exitCode, stdout, stderr, error } = payload || {};
        if (taskId && stageId && hookType) {
          useBoardStore.getState().setHookFinish({
            taskId,
            stageId,
            hookType,
            success: Boolean(success),
            exitCode,
            stdout,
            stderr,
            error,
          });
        }
      });

      // 16. task:updated
      await addListener('task:updated', (payload) => {
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
        if (tid) {
          void useBoardStore.getState().runTaskAgent(tid, prompt);
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

      // 21. boards:updated
      await addListener('boards:updated', async () => {
        await useBoardStore.getState().loadBoards();
        await useBoardStore.getState().loadTasks();
        useCortexStore.getState().syncStagesFromColumns(useBoardStore.getState().columns);
      });

      // 22. pipeline:updated
      await addListener('pipeline:updated', async () => {
        await useBoardStore.getState().loadBoards();
        await useBoardStore.getState().loadTasks();
        useCortexStore.getState().syncStagesFromColumns(useBoardStore.getState().columns);
      });

      // Store-level listeners
      const unlistenBoardStore = await initBoardEventListeners();
      if (isDisposed) {
        unlistenBoardStore();
      } else {
        unlisteners.push(unlistenBoardStore);
      }

      const unlistenCortexStore = await initCortexEventListeners();
      if (isDisposed) {
        unlistenCortexStore();
      } else {
        unlisteners.push(unlistenCortexStore);
      }
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

import { useBoardStore } from '../../stores/boardStore';
import { useDrawerStore } from '../../stores/drawerStore';
import { useAgentSessionStore } from '../../stores/agentSessionStore';
import { showQuestionNotification } from '../notificationService';
import type { AgentEventMap, QuestionOptionItem } from './types';
import type { TextBufferManager } from './textBuffer';
import { ensureTaskRunningOnBoard, isTaskActiveInDrawer, failRunningToolsForTask } from './helpers';

export type AddListenerFn = <K extends keyof AgentEventMap>(
  event: K,
  handler: (payload: AgentEventMap[K]) => void | Promise<void>
) => Promise<void>;

export async function registerAgentEventListeners(
  addListener: AddListenerFn,
  bufferManager: TextBufferManager
): Promise<void> {
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
      failRunningToolsForTask(taskId);
      if (isTaskActiveInDrawer(taskId)) {
        bufferManager.flushTask(taskId);
        const st = useDrawerStore.getState();
        st.setActiveAgentTask(taskId, false);
        st.setGenerationEndTime(taskId, Date.now());
        st.setTaskEndTime(taskId, Date.now());

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
        if (response && response.trim().length > 0) {
          st.markFinalResponse(taskId, response);
        }
        void st.saveTaskActivity(taskId);
      } else {
        bufferManager.clearTask(taskId);
        useDrawerStore.getState().setActiveAgentTask(taskId, false);
      }
      const currentTask = useBoardStore.getState().tasks.find((t) => t.id === taskId);
      if (currentTask && currentTask.status === 'running' && !useAgentSessionStore.getState().isSessionRunning(taskId)) {
        useBoardStore.getState().updateTaskStatus(taskId, 'idle');
      }
    }
  });

  // 9. agent:error
  await addListener('agent:error', (payload) => {
    const { taskId, error } = payload || {};
    if (taskId) {
      useBoardStore.getState().clearActiveQuestion(taskId);
      failRunningToolsForTask(taskId, error || 'Ошибка агента');
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
      if (currentTask && currentTask.status === 'running' && !useAgentSessionStore.getState().isSessionRunning(taskId)) {
        useBoardStore.getState().updateTaskStatus(taskId, 'idle');
      }
    } else {
      bufferManager.flushAll();
    }
  });

  // 10. agent:status
  await addListener('agent:status', (payload) => {
    const { taskId, status } = payload || {};
    if (taskId && (status === 'finished' || status === 'aborted' || status === 'error' || status === 'idle')) {
      useBoardStore.getState().clearActiveQuestion(taskId);
      failRunningToolsForTask(taskId);
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
      if (currentTask && currentTask.status !== 'done' && !useAgentSessionStore.getState().isSessionRunning(taskId)) {
        useBoardStore.getState().updateTaskStatus(taskId, 'idle');
      }
    } else if (taskId && status === 'waiting_subagent') {
      useBoardStore.getState().updateTaskStatus(taskId, 'waiting_subagent');
    } else if (taskId && status === 'running') {
      ensureTaskRunningOnBoard(taskId);
    }
  });

  // 10b. agent:session_status
  await addListener('agent:session_status', (payload) => {
    if (!payload || !payload.sessionId) return;
    useAgentSessionStore.getState().handleSessionStatus(payload);
    if (payload.taskId && payload.status === 'running') {
      ensureTaskRunningOnBoard(payload.taskId);
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
}

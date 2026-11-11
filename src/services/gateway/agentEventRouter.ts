import { useBoardStore } from '../../stores/boardStore';
import { useDrawerStore } from '../../stores/drawerStore';
import { useAgentSessionStore } from '../../stores/agentSessionStore';
import { showQuestionNotification } from '../notificationService';
import type { AgentEventMap, QuestionOptionItem } from './types';
import type { TextBufferManager } from './textBuffer';
import {
  ensureTaskRunningOnBoard,
  isTaskActiveInDrawer,
  finalizeAgentTaskRun,
} from './helpers';

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

  await addListener('agent:thought', (payload) => {
    const { taskId, thought, delta, streamId } = payload || {};
    if (!taskId) return;
    ensureTaskRunningOnBoard(taskId);
    const text = thought || delta;
    if (text) bufferManager.queueThought(taskId, text, streamId);
  });

  await addListener('agent:output', (payload) => {
    const { taskId, output, content, token, streamId } = payload || {};
    if (!taskId) return;
    ensureTaskRunningOnBoard(taskId);
    const text = output || content || token;
    if (text) bufferManager.queueOutput(taskId, text, streamId);
  });

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
        if (elapsedSec > 0.05) st.recordTurnMetrics(taskId, outTokens, elapsedSec);
      }
    }
    if (name) {
      let parsedParams: Record<string, unknown> = {};
      if (typeof args === 'string') {
        try {
          parsedParams = JSON.parse(args) as Record<string, unknown>;
        } catch (parseErr) {
          console.debug('[agent:tool_call] non-JSON tool arguments:', parseErr);
          parsedParams = { raw: args };
        }
      } else if (args) {
        parsedParams = args;
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
      void showQuestionNotification({ taskId, questionId, text: text || '' });
    }
  });

  await addListener('agent:telemetry', (payload) => {
    const {
      taskId, inputTokens, outputTokens, cacheReadTokens, cacheCreationTokens,
      cacheHitRate, durationMs, tps, ttftMs, systemTokens, toolTokens,
    } = payload || {};
    if (!taskId) return;
    const num = (v: unknown) => (typeof v === 'number' && Number.isFinite(v) ? v : 0);
    const tpsVal = num(tps);
    const outTok = num(outputTokens);
    const inTok = num(inputTokens);
    const cRead = num(cacheReadTokens);
    const cCreate = num(cacheCreationTokens);
    const hitRate = num(cacheHitRate) || (inTok > 0 ? (cRead / inTok) * 100 : 0);
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
    if (drawerState.thinkingStartTime[taskId] == null) drawerState.setThinkingStartTime(taskId, Date.now());
    if (drawerState.generationEndTime[taskId] == null) drawerState.setGenerationEndTime(taskId, Date.now());
    const durSec = num(durationMs) > 0 ? num(durationMs) / 1000 : 0;
    if (outTok > 0 && durSec > 0.05) {
      drawerState.recordTurnMetrics(taskId, outTok, durSec);
    } else if (tpsVal > 0) {
      drawerState.setFinalTps(taskId, tpsVal);
    }
  });

  await addListener('agent:compacted', (payload) => {
    const taskId = payload?.taskId;
    const after = payload?.tokens_after ?? payload?.tokensAfter ?? payload?.currentTokens;
    if (taskId && typeof after === 'number' && Number.isFinite(after)) {
      useDrawerStore.getState().setTaskTokens(taskId, after);
    }
  });

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

  await addListener('agent:finished', (payload) => {
    const { taskId, response } = payload || {};
    if (!taskId) return;
    const activeInDrawer = finalizeAgentTaskRun(taskId, bufferManager, { saveActivity: false });
    if (activeInDrawer) {
      const st = useDrawerStore.getState();
      st.setGenerationEndTime(taskId, Date.now());
      st.setTaskEndTime(taskId, Date.now());
      if (st.finalTps[taskId] == null) {
        const tele = st.taskTelemetry[taskId];
        const outTokens = tele?.outputTokens || st.streamedTurnTokens[taskId] || 0;
        const start = st.generationStartTime[taskId] ?? st.thinkingStartTime[taskId];
        const end = st.generationEndTime[taskId] ?? Date.now();
        if (start != null && outTokens > 0) {
          const elapsedSec = (end - start) / 1000;
          if (elapsedSec > 0.05) st.recordTurnMetrics(taskId, outTokens, elapsedSec);
        }
      }
      if (response && response.trim().length > 0) st.markFinalResponse(taskId, response);
      void st.saveTaskActivity(taskId);
    }
  });

  await addListener('agent:error', (payload) => {
    const { taskId, error } = payload || {};
    if (!taskId) {
      bufferManager.flushAll();
      return;
    }
    finalizeAgentTaskRun(taskId, bufferManager, {
      failReason: error || 'Ошибка агента',
      saveActivity: true,
      errorMessage: error,
      resetThinkingTime: true,
    });
  });

  await addListener('agent:aborted', (payload) => {
    const taskId = payload?.taskId;
    if (!taskId) return;
    finalizeAgentTaskRun(taskId, bufferManager, {
      saveActivity: true,
      resetThinkingTime: true,
      allowDoneReset: true,
    });
  });

  await addListener('agent:steer', (payload) => {
    const { taskId, content, images } = payload || {};
    if (!taskId || !content) return;
    bufferManager.flushTask(taskId);
    const trimmed = content.trim();
    const existing = useDrawerStore.getState().activityFeed[taskId] || [];
    const tail = existing.slice(-4);
    const alreadyPresent = tail.some(
      (it) => (it.type === 'steer' || it.type === 'user') && (it.content || '').trim() === trimmed,
    );
    if (!alreadyPresent) {
      useDrawerStore.getState().appendOutput(taskId, content, 'steer', images);
    }
  });

  await addListener('agent:status', (payload) => {
    const { taskId, status } = payload || {};
    if (taskId && (status === 'finished' || status === 'aborted' || status === 'error' || status === 'idle')) {
      finalizeAgentTaskRun(taskId, bufferManager, {
        saveActivity: status === 'aborted' || status === 'error',
        resetThinkingTime: true,
        allowDoneReset: true,
      });
    } else if (taskId && status === 'waiting_subagent') {
      useBoardStore.getState().updateTaskStatus(taskId, 'waiting_subagent');
    } else if (taskId && status === 'running') {
      ensureTaskRunningOnBoard(taskId);
    }
  });

  await addListener('agent:session_status', (payload) => {
    if (!payload || !payload.sessionId) return;
    useAgentSessionStore.getState().handleSessionStatus(payload);
    if (payload.taskId && payload.status === 'running') ensureTaskRunningOnBoard(payload.taskId);
  });

  await addListener('agent:autorun', (payload) => {
    const tid = payload?.taskId;
    if (tid) {
      void useBoardStore.getState().runTaskAgent(tid, payload?.prompt, {
        skipSync: true,
        projectPath: payload?.projectPath,
      });
    }
  });
}

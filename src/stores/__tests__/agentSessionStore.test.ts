import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  useAgentSessionStore,
  isMatchingSessionTaskId,
  type AgentSessionSummary,
} from '../agentSessionStore';

const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('agentSessionStore & isMatchingSessionTaskId', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAgentSessionStore.getState().reset();
  });

  describe('isMatchingSessionTaskId', () => {
    it('matches exact task IDs ignoring case and whitespace', () => {
      expect(isMatchingSessionTaskId('CVH-001', 'CVH-001')).toBe(true);
      expect(isMatchingSessionTaskId('cvh-001', 'CVH-001')).toBe(true);
      expect(isMatchingSessionTaskId(' CVH-001 ', 'cvh-001')).toBe(true);
      expect(isMatchingSessionTaskId('CVH-001', 'CVH-002')).toBe(false);
    });

    it('matches subtask and subagent hierarchy patterns', () => {
      expect(isMatchingSessionTaskId('CVH-001.1', 'CVH-001')).toBe(true);
      expect(isMatchingSessionTaskId('CVH-001.sub-test', 'CVH-001')).toBe(true);
      expect(isMatchingSessionTaskId('CVH-001:sub:runner', 'CVH-001')).toBe(true);
      expect(isMatchingSessionTaskId('CVH-002.1', 'CVH-001')).toBe(false);
    });
  });

  describe('store initial state & syncActiveSessions', () => {
    it('initializes with empty sessions and isSyncing false', () => {
      const state = useAgentSessionStore.getState();
      expect(state.sessions).toEqual({});
      expect(state.isSyncing).toBe(false);
      expect(state.getActiveSessions()).toEqual([]);
    });

    it('syncActiveSessions populates sessions from backend invoke', async () => {
      const mockSessions: AgentSessionSummary[] = [
        {
          sessionId: 'session-1',
          taskId: 'CVH-001',
          stageId: 'dev',
          startedAt: 1000,
          status: 'running',
        },
        {
          sessionId: 'session-2',
          taskId: 'CVH-002',
          stageId: 'review',
          startedAt: 2000,
          status: 'paused',
        },
      ];
      mockInvoke.mockResolvedValueOnce(mockSessions);

      await useAgentSessionStore.getState().syncActiveSessions();

      expect(mockInvoke).toHaveBeenCalledWith('get_active_sessions');
      const sessions = useAgentSessionStore.getState().sessions;
      expect(Object.keys(sessions)).toHaveLength(2);
      expect(sessions['session-1'].taskId).toBe('CVH-001');
      expect(sessions['session-1'].status).toBe('running');
      expect(sessions['session-2'].status).toBe('paused');
      expect(useAgentSessionStore.getState().isSyncing).toBe(false);
    });

    it('handles backend invoke error gracefully during sync', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('Backend error'));
      await useAgentSessionStore.getState().syncActiveSessions();
      expect(useAgentSessionStore.getState().isSyncing).toBe(false);
      expect(useAgentSessionStore.getState().sessions).toEqual({});
    });
  });

  describe('handleSessionStatus & removeSession', () => {
    it('inserts and updates active sessions', () => {
      const summary: AgentSessionSummary = {
        sessionId: 'session-a',
        taskId: 'CVH-100',
        stageId: 'dev',
        startedAt: 5000,
        status: 'running',
      };

      useAgentSessionStore.getState().handleSessionStatus(summary);
      expect(useAgentSessionStore.getState().sessions['session-a']).toEqual(summary);
      expect(useAgentSessionStore.getState().isSessionRunning('CVH-100')).toBe(true);

      // Transition to aborting
      const abortingSummary: AgentSessionSummary = {
        ...summary,
        status: 'aborting',
      };
      useAgentSessionStore.getState().handleSessionStatus(abortingSummary);
      expect(useAgentSessionStore.getState().sessions['session-a'].status).toBe('aborting');
      expect(useAgentSessionStore.getState().isSessionRunning('CVH-100')).toBe(false);
      expect(useAgentSessionStore.getState().hasActiveSession('CVH-100')).toBe(true);
    });

    it('removes finished sessions automatically when handleSessionStatus receives finished', () => {
      const summary: AgentSessionSummary = {
        sessionId: 'session-b',
        taskId: 'CVH-200',
        stageId: 'dev',
        startedAt: 6000,
        status: 'running',
      };
      useAgentSessionStore.getState().handleSessionStatus(summary);
      expect(useAgentSessionStore.getState().hasActiveSession('CVH-200')).toBe(true);

      // Finish session
      useAgentSessionStore.getState().handleSessionStatus({
        ...summary,
        status: 'finished',
      });
      expect(useAgentSessionStore.getState().sessions['session-b']).toBeUndefined();
      expect(useAgentSessionStore.getState().hasActiveSession('CVH-200')).toBe(false);
      expect(useAgentSessionStore.getState().isSessionRunning('CVH-200')).toBe(false);
    });

    it('removeSession removes target session by sessionId', () => {
      useAgentSessionStore.getState().handleSessionStatus({
        sessionId: 'sess-to-remove',
        taskId: 'CVH-300',
        startedAt: 7000,
        status: 'running',
      });
      expect(useAgentSessionStore.getState().sessions['sess-to-remove']).toBeDefined();

      useAgentSessionStore.getState().removeSession('sess-to-remove');
      expect(useAgentSessionStore.getState().sessions['sess-to-remove']).toBeUndefined();
    });
  });

  describe('query helpers: hasActiveSession, isSessionRunning, getActiveSessions', () => {
    beforeEach(() => {
      useAgentSessionStore.getState().handleSessionStatus({
        sessionId: 'sess-parent',
        taskId: 'CVH-400',
        startedAt: 100,
        status: 'running',
      });
      useAgentSessionStore.getState().handleSessionStatus({
        sessionId: 'sess-sub',
        taskId: 'CVH-400.sub-1',
        startedAt: 200,
        status: 'running',
      });
      useAgentSessionStore.getState().handleSessionStatus({
        sessionId: 'sess-paused',
        taskId: 'CVH-500',
        startedAt: 300,
        status: 'paused',
      });
    });

    it('identifies running state for root task and subtasks', () => {
      expect(useAgentSessionStore.getState().isSessionRunning('CVH-400')).toBe(true);
      expect(useAgentSessionStore.getState().isSessionRunning('CVH-400.sub-1')).toBe(true);
      expect(useAgentSessionStore.getState().isSessionRunning('CVH-500')).toBe(false);
      expect(useAgentSessionStore.getState().isSessionRunning('CVH-999')).toBe(false);
    });

    it('identifies active sessions even when paused or aborting', () => {
      expect(useAgentSessionStore.getState().hasActiveSession('CVH-400')).toBe(true);
      expect(useAgentSessionStore.getState().hasActiveSession('CVH-500')).toBe(true);
      expect(useAgentSessionStore.getState().hasActiveSession('CVH-999')).toBe(false);
    });

    it('getActiveSessions returns only active (non-finished) sessions', () => {
      const active = useAgentSessionStore.getState().getActiveSessions();
      expect(active).toHaveLength(3);
      expect(active.map((s) => s.sessionId)).toEqual(
        expect.arrayContaining(['sess-parent', 'sess-sub', 'sess-paused'])
      );
    });
  });
});

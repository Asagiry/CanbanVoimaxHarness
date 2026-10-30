import { create } from 'zustand';

export type SessionStatus = 'running' | 'paused' | 'aborting' | 'finished';

export interface AgentSessionSummary {
  sessionId: string;
  taskId: string;
  stageId?: string;
  startedAt: number;
  status: SessionStatus;
}

export function isMatchingSessionTaskId(sessionTaskId: string, queryTaskId: string): boolean {
  const sTid = sessionTaskId.trim().toLowerCase();
  const qTid = queryTaskId.trim().toLowerCase();
  if (sTid === qTid) return true;
  if (
    sTid.startsWith(`${qTid}.`) ||
    sTid.startsWith(`${qTid}:sub:`) ||
    sTid.startsWith(`${qTid}.sub-`)
  ) {
    return true;
  }
  return false;
}

export interface AgentSessionState {
  sessions: Record<string, AgentSessionSummary>;
  isSyncing: boolean;
  syncActiveSessions: () => Promise<void>;
  handleSessionStatus: (summary: AgentSessionSummary) => void;
  removeSession: (sessionId: string) => void;
  hasActiveSession: (taskId: string) => boolean;
  isSessionRunning: (taskId: string) => boolean;
  getActiveSessions: () => AgentSessionSummary[];
  reset: () => void;
}

export const useAgentSessionStore = create<AgentSessionState>((set, get) => ({
  sessions: {},
  isSyncing: false,

  syncActiveSessions: async () => {
    set({ isSyncing: true });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const activeSessions = await invoke<AgentSessionSummary[]>('get_active_sessions');
      const sessionsMap: Record<string, AgentSessionSummary> = {};
      if (Array.isArray(activeSessions)) {
        for (const s of activeSessions) {
          if (s && s.sessionId) {
            sessionsMap[s.sessionId] = s;
          }
        }
      }
      set({ sessions: sessionsMap, isSyncing: false });
    } catch (err) {
      console.error('[agentSessionStore] Failed to sync active sessions:', err);
      set({ isSyncing: false });
    }
  },

  handleSessionStatus: (summary: AgentSessionSummary) => {
    if (!summary || !summary.sessionId) return;
    set((state) => {
      if (summary.status === 'finished') {
        const newSessions = { ...state.sessions };
        delete newSessions[summary.sessionId];
        return { sessions: newSessions };
      }
      return {
        sessions: {
          ...state.sessions,
          [summary.sessionId]: summary,
        },
      };
    });
  },

  removeSession: (sessionId: string) => {
    set((state) => {
      const newSessions = { ...state.sessions };
      delete newSessions[sessionId];
      return { sessions: newSessions };
    });
  },

  hasActiveSession: (taskId: string) => {
    if (!taskId) return false;
    const sessions = Object.values(get().sessions);
    return sessions.some((s) => {
      if (s.status === 'finished') return false;
      return (
        isMatchingSessionTaskId(s.taskId, taskId) ||
        isMatchingSessionTaskId(s.sessionId, taskId)
      );
    });
  },

  isSessionRunning: (taskId: string) => {
    if (!taskId) return false;
    const sessions = Object.values(get().sessions);
    return sessions.some((s) => {
      if (s.status !== 'running') return false;
      return (
        isMatchingSessionTaskId(s.taskId, taskId) ||
        isMatchingSessionTaskId(s.sessionId, taskId)
      );
    });
  },

  getActiveSessions: () => {
    return Object.values(get().sessions).filter((s) => s.status !== 'finished');
  },

  reset: () => {
    set({ sessions: {}, isSyncing: false });
  },
}));

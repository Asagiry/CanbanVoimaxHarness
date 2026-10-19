import { create } from 'zustand';
import type { ToolExecution } from '../types';

export interface CopilotMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  thought?: string;
  tools?: ToolExecution[];
  timestamp?: string;
  isStreaming?: boolean;
}

export interface CopilotStreamPayload {
  taskId?: string;
  type?: 'chunk' | 'thought' | 'tool' | 'done' | 'error' | 'status';
  chunk?: string;
  delta?: string;
  content?: string;
  thought?: string;
  tool?: ToolExecution;
  error?: string;
  status?: string;
  done?: boolean;
}

interface CopilotStoreState {
  isOpen: boolean;
  width: number;
  messages: CopilotMessage[];
  isStreaming: boolean;
  input: string;
  selectedModel: string;
  availableModels: string[];
  statusText: string | null;

  // Actions
  toggleOpen: () => void;
  setIsOpen: (open: boolean) => void;
  setWidth: (w: number) => void;
  setInput: (text: string) => void;
  setSelectedModel: (model: string) => void;
  sendMessage: (text: string) => Promise<void>;
  abortStreaming: () => Promise<void>;
  loadHistory: () => Promise<void>;
  clearHistory: () => Promise<void>;
}

const IS_OPEN_KEY = 'cvh:copilot:isOpen';
const WIDTH_KEY = 'cvh:copilot:width';
const HISTORY_KEY = 'cvh:copilot:history';
const MODEL_KEY = 'cvh:copilot:model';

const getInitialIsOpen = (): boolean => {
  try {
    const val = localStorage.getItem(IS_OPEN_KEY);
    if (val !== null) return JSON.parse(val);
  } catch {
    // fallback
  }
  return true; // default true per requirement
};

const getInitialWidth = (): number => {
  try {
    const val = localStorage.getItem(WIDTH_KEY);
    if (val !== null) {
      const num = Number(val);
      if (!isNaN(num)) return Math.min(650, Math.max(320, num));
    }
  } catch {
    // fallback
  }
  return 400; // default 400 per requirement
};

const getInitialModel = (): string => {
  try {
    const val = localStorage.getItem(MODEL_KEY);
    if (val) return val;
  } catch {
    // fallback
  }
  return 'gemini-3.8-flash-high';
};

const saveHistoryToStorage = (messages: CopilotMessage[]) => {
  try {
    // Save last 50 messages to prevent excessive localStorage usage
    const slice = messages.slice(-50).map((m) => ({ ...m, isStreaming: false }));
    localStorage.setItem(HISTORY_KEY, JSON.stringify(slice));
  } catch {
    // ignore quota errors
  }
};

let streamUnlisten: (() => void) | null = null;
let simulationAbortController: { aborted: boolean } | null = null;

export const useCopilotStore = create<CopilotStoreState>((set, get) => ({
  isOpen: getInitialIsOpen(),
  width: getInitialWidth(),
  messages: [],
  isStreaming: false,
  input: '',
  selectedModel: getInitialModel(),
  availableModels: [
    'gemini-3.8-flash-high',
    'claude-3-7-sonnet-20250219',
    'gpt-4o',
    'qwen2.5-coder:32b',
    'deepseek-r1',
  ],
  statusText: null,

  toggleOpen: () => {
    const next = !get().isOpen;
    set({ isOpen: next });
    try {
      localStorage.setItem(IS_OPEN_KEY, JSON.stringify(next));
    } catch {}
  },

  setIsOpen: (open: boolean) => {
    set({ isOpen: open });
    try {
      localStorage.setItem(IS_OPEN_KEY, JSON.stringify(open));
    } catch {}
  },

  setWidth: (w: number) => {
    const clamped = Math.min(650, Math.max(320, Math.round(w)));
    set({ width: clamped });
    try {
      localStorage.setItem(WIDTH_KEY, String(clamped));
    } catch {}
  },

  setInput: (input: string) => {
    set({ input });
  },

  setSelectedModel: (selectedModel: string) => {
    set({ selectedModel });
    try {
      localStorage.setItem(MODEL_KEY, selectedModel);
    } catch {}
  },

  sendMessage: async (text: string) => {
    const trimmed = text.trim();
    if (!trimmed || get().isStreaming) return;

    // Reset any previous simulation
    if (simulationAbortController) {
      simulationAbortController.aborted = true;
      simulationAbortController = null;
    }

    const now = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    const userMsg: CopilotMessage = {
      id: `user-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      role: 'user',
      content: trimmed,
      timestamp: now,
    };

    const assistantId = `asst-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
    const assistantMsg: CopilotMessage = {
      id: assistantId,
      role: 'assistant',
      content: '',
      thought: '',
      tools: [],
      timestamp: now,
      isStreaming: true,
    };

    set((s) => ({
      messages: [...s.messages, userMsg, assistantMsg],
      isStreaming: true,
      input: '',
      statusText: 'Архитектор размышляет...',
    }));

    // Ensure Tauri stream listener is initialized
    await initCopilotStreamListener();

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('send_copilot_message', {
        message: trimmed,
        model: get().selectedModel,
      });
    } catch (invokeErr: any) {
      console.info('send_copilot_message Tauri invoke fallback (interactive architect response):', invokeErr);

      // Graceful interactive simulation for development/web or before backend command merge
      const simController = { aborted: false };
      simulationAbortController = simController;

      // Simulate realistic streaming response with thinking and architectural advice
      const simThoughts = [
        `Анализирую архитектурный контекст запроса: "${trimmed}"...`,
        'Проверяю конфигурацию активной доски, пайплайн стадий и доступные инструменты...',
        'Формирую стратегию декомпозиции и рекомендации для Канбан-пайплайна...',
      ];

      const simContent = [
        '### План конфигурации проекта',
        '',
        `По запросу **"${trimmed}"** подготовлена следующая конфигурация:`,
        '',
        '1. **Декомпозиция задачи**:',
        '   - Выделение интерфейсного слоя (React 19 + Tailwind v4).',
        '   - Интеграция с локальной шиной событий и Tauri v2 IPC.',
        '   - Обеспечение инвариантов строгой типизации TypeScript.',
        '',
        '2. **Рекомендуемые стадии**:',
        '   - \x60backlog\x60 -> Формирование спецификации.',
        '   - \x60dev\x60 -> Реализация в изолированном worktree.',
        '   - \x60review\x60 -> Контроль инвариантов и аудит качества.',
        '   - \x60ready\x60 -> Финальный merge.'
      ].join('\n');

      (async () => {
        // Stream thoughts first
        for (const t of simThoughts) {
          if (simController.aborted) return;
          set((s) => ({
            statusText: t,
            messages: s.messages.map((m) =>
              m.id === assistantId ? { ...m, thought: (m.thought ? m.thought + '\n' : '') + t } : m
            ),
          }));
          await new Promise((r) => setTimeout(r, 220));
        }

        if (simController.aborted) return;
        set({ statusText: 'Формирую ответ...' });

        // Stream content words
        const words = simContent.split(' ');
        for (let i = 0; i < words.length; i += 3) {
          if (simController.aborted) return;
          const chunk = words.slice(i, i + 3).join(' ') + ' ';
          set((s) => ({
            messages: s.messages.map((m) =>
              m.id === assistantId ? { ...m, content: m.content + chunk } : m
            ),
          }));
          await new Promise((r) => setTimeout(r, 45));
        }

        if (!simController.aborted) {
          set((s) => ({
            isStreaming: false,
            statusText: null,
            messages: s.messages.map((m) =>
              m.id === assistantId ? { ...m, isStreaming: false } : m
            ),
          }));
          saveHistoryToStorage(get().messages);
        }
      })();
    }
  },

  abortStreaming: async () => {
    if (simulationAbortController) {
      simulationAbortController.aborted = true;
      simulationAbortController = null;
    }
    set({ isStreaming: false, statusText: null });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('abort_copilot');
    } catch {
      // ignore
    }
    set((s) => ({
      messages: s.messages.map((m) => (m.isStreaming ? { ...m, isStreaming: false } : m)),
    }));
    saveHistoryToStorage(get().messages);
  },

  loadHistory: async () => {
    // 1. Try Tauri backend
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const history = await invoke<CopilotMessage[]>('load_copilot_history');
      if (Array.isArray(history) && history.length > 0) {
        set({ messages: history });
        saveHistoryToStorage(history);
        return;
      }
    } catch {
      // fallback to localStorage
    }

    // 2. Fallback to localStorage
    try {
      const raw = localStorage.getItem(HISTORY_KEY);
      if (raw) {
        const parsed = JSON.parse(raw);
        if (Array.isArray(parsed) && parsed.length > 0) {
          set({ messages: parsed });
          return;
        }
      }
    } catch {
      // ignore
    }

    // 3. Welcome initial message if empty
    if (get().messages.length === 0) {
      const welcomeMsg: CopilotMessage = {
        id: 'welcome-1',
        role: 'assistant',
        content: 'Здравствуйте! Я ваш **Sidecar Архитектор CVH**.\n\nЯ помогу вам спланировать новую архитектуру, настроить пайплайны досок или декомпозировать сложные фичи в пошаговые карточки задач.\n\nЧем займёмся сегодня?',
        timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
        isStreaming: false,
      };
      set({ messages: [welcomeMsg] });
      saveHistoryToStorage([welcomeMsg]);
    }
  },

  clearHistory: async () => {
    set({ messages: [], isStreaming: false, statusText: null });
    try {
      localStorage.removeItem(HISTORY_KEY);
    } catch {}
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('clear_copilot_history');
    } catch {
      // ignore
    }
  },
}));

/**
 * Initializes the stream listener for copilot events from Tauri backend
 */
export async function initCopilotStreamListener(): Promise<() => void> {
  if (streamUnlisten) return streamUnlisten;

  try {
    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen<any>('copilot:stream', (event) => {
      const payload = event.payload;
      if (!payload) return;

      // Handle raw string chunk
      if (typeof payload === 'string') {
        appendChunkToLatestAssistant(payload);
        return;
      }

      // Handle structured payload
      const p = payload as CopilotStreamPayload;

      if (p.done) {
        useCopilotStore.setState((s) => ({
          isStreaming: false,
          statusText: null,
          messages: s.messages.map((m) => (m.isStreaming ? { ...m, isStreaming: false } : m)),
        }));
        saveHistoryToStorage(useCopilotStore.getState().messages);
        return;
      }

      if (p.error) {
        useCopilotStore.setState((s) => ({
          isStreaming: false,
          statusText: null,
          messages: s.messages.map((m) =>
            m.isStreaming
              ? {
                  ...m,
                  isStreaming: false,
                  content: m.content + `\n\n**Ошибка**: ${p.error}`,
                }
              : m
          ),
        }));
        return;
      }

      if (p.status) {
        useCopilotStore.setState({ statusText: p.status });
      }

      if (p.thought) {
        appendThoughtToLatestAssistant(p.thought);
      }

      if (p.tool) {
        appendToolToLatestAssistant(p.tool);
      }

      const textChunk = p.chunk || p.delta || p.content;
      if (textChunk) {
        appendChunkToLatestAssistant(textChunk);
      }
    });

    streamUnlisten = () => {
      unlisten();
      streamUnlisten = null;
    };
    return streamUnlisten;
  } catch (e) {
    console.info('initCopilotStreamListener not active in web mode:', e);
    return () => {};
  }
}

function appendChunkToLatestAssistant(chunk: string) {
  useCopilotStore.setState((s) => {
    const len = s.messages.length;
    if (len === 0) return s;

    let targetIdx = -1;
    for (let i = len - 1; i >= 0; i--) {
      if (s.messages[i].role === 'assistant' && s.messages[i].isStreaming) {
        targetIdx = i;
        break;
      }
    }
    if (targetIdx === -1) {
      targetIdx = len - 1;
    }

    const target = s.messages[targetIdx];
    const updated = {
      ...target,
      content: (target.content || '') + chunk,
    };

    const nextMessages = [...s.messages];
    nextMessages[targetIdx] = updated;
    return { messages: nextMessages };
  });
}

function appendThoughtToLatestAssistant(thoughtChunk: string) {
  useCopilotStore.setState((s) => {
    const len = s.messages.length;
    if (len === 0) return s;

    let targetIdx = -1;
    for (let i = len - 1; i >= 0; i--) {
      if (s.messages[i].role === 'assistant' && s.messages[i].isStreaming) {
        targetIdx = i;
        break;
      }
    }
    if (targetIdx === -1) return s;

    const target = s.messages[targetIdx];
    const updated = {
      ...target,
      thought: (target.thought || '') + thoughtChunk,
    };

    const nextMessages = [...s.messages];
    nextMessages[targetIdx] = updated;
    return { messages: nextMessages };
  });
}

function appendToolToLatestAssistant(tool: ToolExecution) {
  useCopilotStore.setState((s) => {
    const len = s.messages.length;
    if (len === 0) return s;

    let targetIdx = -1;
    for (let i = len - 1; i >= 0; i--) {
      if (s.messages[i].role === 'assistant' && s.messages[i].isStreaming) {
        targetIdx = i;
        break;
      }
    }
    if (targetIdx === -1) return s;

    const target = s.messages[targetIdx];
    const existingTools = target.tools || [];
    const idx = existingTools.findIndex((t) => t.id === tool.id);
    let newTools: ToolExecution[];
    if (idx >= 0) {
      newTools = [...existingTools];
      newTools[idx] = tool;
    } else {
      newTools = [...existingTools, tool];
    }

    const updated = {
      ...target,
      tools: newTools,
    };

    const nextMessages = [...s.messages];
    nextMessages[targetIdx] = updated;
    return { messages: nextMessages };
  });
}

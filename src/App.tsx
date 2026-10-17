import { useEffect, useState } from 'react';
import Topbar from './components/layout/Topbar';
import CortexExplorer from './components/cortex/CortexExplorer';
import KanbanBoard from './components/board/KanbanBoard';
import AgentConsoleDrawer from './components/drawer/AgentConsoleDrawer';
import ErrorBoundary from './components/common/ErrorBoundary';
import SplashScreen from './components/splash/SplashScreen';
import RuleEditorModal from './components/cortex/RuleEditorModal';
import ToolPolicyModal from './components/settings/ToolPolicyModal';
import SettingsModal from './components/settings/SettingsModal';
import { useBoardStore, initBoardEventListeners } from './stores/boardStore';
import { useProjectStore } from './stores/projectStore';
import { useDrawerStore } from './stores/drawerStore';
import { useCortexStore, initCortexEventListeners } from './stores/cortexStore';
import { useUiZoom } from './hooks/useUiZoom';

// Микробатчинг SSE-текста: один Zustand set() на пачку токенов вместо set() на токен.
const TEXT_FLUSH_MS = 150;

interface TextBuffer {
  thought: string;
  output: string;
  thoughtTimer?: ReturnType<typeof setTimeout>;
  outputTimer?: ReturnType<typeof setTimeout>;
}

const NON_TEXT_INPUT_TYPES = new Set([
  'button',
  'checkbox',
  'color',
  'file',
  'hidden',
  'image',
  'radio',
  'range',
  'reset',
  'submit',
]);

/** Check if element is an editable text field or inside one (input, textarea, contenteditable) */
function isEditableElement(el: HTMLElement | null): boolean {
  if (!el) return false;
  if (el.isContentEditable || el.closest?.('[contenteditable="true"], [contenteditable=""]')) {
    return true;
  }
  const input = el.closest?.('input, textarea');
  if (!input) return false;
  if (input instanceof HTMLTextAreaElement) return true;
  if (input instanceof HTMLInputElement) {
    return !NON_TEXT_INPUT_TYPES.has(input.type.toLowerCase());
  }
  return false;
}

export default function App() {
  const { rootStyle, zoomPercent, showIndicator, resetZoom } = useUiZoom();
  const loadTasks = useBoardStore((s) => s.loadTasks);
  const runTaskAgent = useBoardStore((s) => s.runTaskAgent);
  const loadBoards = useBoardStore((s) => s.loadBoards);
  const loadProjects = useProjectStore((s) => s.loadProjects);
  const addToolExecution = useDrawerStore((s) => s.addToolExecution);
  const updateToolExecution = useDrawerStore((s) => s.updateToolExecution);
  const updateTelemetry = useDrawerStore((s) => s.updateTelemetry);
  const setThinkingStartTime = useDrawerStore((s) => s.setThinkingStartTime);

  const [isDataReady, setIsDataReady] = useState(false);
  const [showSplash, setShowSplash] = useState(true);

  // Global context menu suppression: eliminate default browser/WebView2 context menu across
  // canvas, headers, cards, and background while preserving custom app context menus and native text editing
  useEffect(() => {
    const handleContextMenu = (e: MouseEvent) => {
      const target = e.target instanceof HTMLElement ? e.target : (e.target as Node | null)?.parentElement;
      if (isEditableElement(target ?? null)) {
        return;
      }
      e.preventDefault();
    };

    window.addEventListener('contextmenu', handleContextMenu);
    return () => window.removeEventListener('contextmenu', handleContextMenu);
  }, []);

  useEffect(() => {
    const init = async () => {
      try {
        await loadProjects();
        // Boards and columns first: task mapping (columnId fallback, persona, status) resolves via columns
        await loadBoards();
        await loadTasks();
        await useCortexStore.getState().loadAgentPersonas();
        useCortexStore.getState().syncStagesFromColumns(useBoardStore.getState().columns);
      } catch (e) {
        console.warn('Initialization error:', e);
      } finally {
        setIsDataReady(true);
      }
    };
    void init();
  }, [loadProjects, loadBoards, loadTasks]);

  useEffect(() => {
    let unlisteners: (() => void)[] = [];
    // Текстовые буферы per taskId, отдельно для thought и output каналов.
    // Склеиваются только соседние чанки одного типа; tool-события идут мимо буфера.
    const textBuffers = new Map<string, TextBuffer>();

    const getBuf = (taskId: string): TextBuffer => {
      let b = textBuffers.get(taskId);
      if (!b) {
        b = { thought: '', output: '' };
        textBuffers.set(taskId, b);
      }
      return b;
    };

    const flushThought = (taskId: string) => {
      const b = textBuffers.get(taskId);
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
    };

    const flushOutput = (taskId: string) => {
      const b = textBuffers.get(taskId);
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
    };

    const flushTask = (taskId: string) => {
      flushThought(taskId);
      flushOutput(taskId);
    };

    const flushAll = () => {
      for (const id of Array.from(textBuffers.keys())) {
        flushTask(id);
      }
    };

    const queueThought = (taskId: string, text: string) => {
      const b = getBuf(taskId);
      b.thought += text;
      if (!b.thoughtTimer) {
        b.thoughtTimer = setTimeout(() => {
          flushThought(taskId);
        }, TEXT_FLUSH_MS);
      }
    };

    const queueOutput = (taskId: string, text: string) => {
      const b = getBuf(taskId);
      b.output += text;
      if (!b.outputTimer) {
        b.outputTimer = setTimeout(() => {
          flushOutput(taskId);
        }, TEXT_FLUSH_MS);
      }
    };

    const setupListeners = async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');

        const u1 = await listen<any>('agent:thought', (event) => {
          const { taskId, thought, delta } = event.payload || {};
          if (taskId && (thought || delta)) {
            queueThought(taskId, thought || delta);
          }
        });
        unlisteners.push(u1);

        const u2 = await listen<any>('agent:output', (event) => {
          const { taskId, output, content, token } = event.payload || {};
          const text = output || content || token;
          if (taskId && text) {
            queueOutput(taskId, text);
          }
        });
        unlisteners.push(u2);

        const u3 = await listen<any>('agent:tool_call', (event) => {
          const { taskId, id, name, arguments: args } = event.payload || {};
          // Flush текста перед tool-событием, чтобы сохранить хронологию ленты.
          if (taskId) flushTask(taskId);
          if (taskId && name) {
            let parsedParams: Record<string, unknown> = {};
            try {
              parsedParams = typeof args === 'string' ? JSON.parse(args) : (args || {});
            } catch {
              parsedParams = { raw: args };
            }
            addToolExecution(taskId, {
              id: id || `call-${Date.now()}`,
              toolName: name,
              timestamp: new Date().toLocaleTimeString('en-US', { hour12: false }),
              durationMs: 0,
              status: 'running',
              params: parsedParams,
            });
          }
        });
        unlisteners.push(u3);

        const uToolResult = await listen<any>('agent:tool_result', (event) => {
          const { taskId, id, result, success } = event.payload || {};
          if (taskId) flushTask(taskId);
          if (taskId && id) {
            const outStr = typeof result === 'string' ? result : JSON.stringify(result, null, 2);
            updateToolExecution(taskId, id, {
              status: success ? 'success' : 'failed',
              stdout: success ? outStr : undefined,
              stderr: !success ? outStr : undefined,
            });
          }
        });
        unlisteners.push(uToolResult);

        const uQuestion = await listen<any>('agent:question', (event) => {
          const { taskId, questionId, text, options, multiple } = event.payload || {};
          if (taskId) flushTask(taskId);
          if (taskId && questionId) {
            useDrawerStore.getState().addQuestion(taskId, {
              questionId,
              text: text || '',
              options: options || [],
              multiple: Boolean(multiple),
            });
          }
        });
        unlisteners.push(uQuestion);

        const uTelemetry = await listen<any>('agent:telemetry', (event) => {
          const { taskId, inputTokens, outputTokens, cacheReadTokens, cacheCreationTokens, cacheHitRate } = event.payload || {};
          if (!taskId) return;
          const num = (v: unknown) => (typeof v === 'number' && Number.isFinite(v) ? v : 0);
          updateTelemetry(taskId, {
            inputTokens: num(inputTokens),
            outputTokens: num(outputTokens),
            cacheReadTokens: num(cacheReadTokens),
            cacheCreationTokens: num(cacheCreationTokens),
            cacheHitRate: num(cacheHitRate),
          });
          // Fallback замера длительности: если turn_start не пришёл, стартуем от первого события терна
          const drawerState = useDrawerStore.getState();
          if (drawerState.thinkingStartTime[taskId] == null) {
            drawerState.setThinkingStartTime(taskId, Date.now());
          }
        });
        unlisteners.push(uTelemetry);

        const uTurnStart = await listen<any>('agent:turn_start', (event) => {
          const { taskId } = event.payload || {};
          if (taskId) {
            flushTask(taskId);
            useDrawerStore.getState().setFinalTps(taskId, null);
            setThinkingStartTime(taskId, Date.now());
          }
        });
        unlisteners.push(uTurnStart);

        const u5 = await listen<any>('agent:finished', (event) => {
          const { taskId, response } = event.payload || {};
          if (taskId) {
            // Принудительный flush буферов перед финальной обработкой,
            // иначе дедуп ниже не увидит забутченные токены.
            flushTask(taskId);
            // Заморозка TPS на финише: те же источники, что в ContextGauge
            try {
              const st = useDrawerStore.getState();
              const tele = st.taskTelemetry[taskId];
              const start = st.thinkingStartTime[taskId];
              if (tele && start != null) {
                const elapsedSec = (Date.now() - start) / 1000;
                const outTokens = tele.outputTokens ?? 0;
                if (elapsedSec > 0 && outTokens > 0) {
                  st.setFinalTps(taskId, outTokens / elapsedSec);
                }
              }
            } catch {
              // ignore
            }
            // Дедуп финала: текст уже пристримился токенами agent:output,
            // а бэкенд может прислать agent:finished дважды
            if (response && response.length > 0) {
              const feed = useDrawerStore.getState().activityFeed[taskId];
              const last = feed?.[feed.length - 1];
              const lastContent = last?.content || '';
              if (!lastContent.endsWith(response)) {
                useDrawerStore.getState().appendOutput(taskId, response, 'output');
              }
            }
            // Immediately flush task activity feed to disk upon turn finish
            useDrawerStore.getState().flushSaveDialog(taskId);
            useBoardStore.getState().updateTaskStatus(taskId, 'idle');
            useDrawerStore.getState().setThinkingStartTime(taskId, null);
          }
          loadTasks();
        });
        unlisteners.push(u5);

        const uErr = await listen<any>('agent:error', (event) => {
          const { taskId } = event.payload || {};
          if (taskId) {
            flushTask(taskId);
            useDrawerStore.getState().flushSaveDialog(taskId);
            useBoardStore.getState().updateTaskStatus(taskId, 'idle');
            useDrawerStore.getState().setThinkingStartTime(taskId, null);
          } else {
            flushAll();
          }
          loadTasks();
        });
        unlisteners.push(uErr);

        const u6 = await listen<any>('agent:status', (event) => {
          const { taskId, status } = event.payload || {};
          if (taskId && (status === 'finished' || status === 'aborted' || status === 'error')) {
            flushTask(taskId);
            useDrawerStore.getState().flushSaveDialog(taskId);
            useBoardStore.getState().updateTaskStatus(taskId, 'idle');
            useDrawerStore.getState().setThinkingStartTime(taskId, null);
          }
          loadTasks();
        });
        unlisteners.push(u6);

        const uTaskCreated = await listen<any>('task:created', () => {
          loadTasks();
        });
        unlisteners.push(uTaskCreated);

        const uTaskMoved = await listen<any>('task:moved', () => {
          loadTasks();
        });
        unlisteners.push(uTaskMoved);

        const uTaskUpdated = await listen<{ taskId?: string; todos?: any[] }>('task:updated', (event) => {
          const { taskId, todos } = event.payload || {};
          if (taskId) {
            flushTask(taskId);
          } else {
            flushAll();
          }
          if (taskId && todos) {
            useBoardStore.getState().updateTask(taskId, { todos });
          } else {
            loadTasks();
          }
        });
        unlisteners.push(uTaskUpdated);

        const uTasksUpdated = await listen<any>('tasks:updated', () => {
          flushAll();
          loadTasks();
        });
        unlisteners.push(uTasksUpdated);

        const uAutoRun = await listen<{ taskId?: string }>('agent:autorun', (event) => {
          const tid = event.payload?.taskId;
          if (tid) {
            void runTaskAgent(tid);
          }
        });
        unlisteners.push(uAutoRun);

        const uProj = await listen<any>('project:changed', async () => {
          await loadBoards();
          await loadTasks();
          await useCortexStore.getState().loadAgentPersonas();
          useCortexStore.getState().syncStagesFromColumns(useBoardStore.getState().columns);
        });
        unlisteners.push(uProj);

        const uPersonasUpdated = await listen<any>('agent:personas_updated', async () => {
          await useCortexStore.getState().loadAgentPersonas();
          useCortexStore.getState().syncStagesFromColumns(useBoardStore.getState().columns);
        });
        unlisteners.push(uPersonasUpdated);

        const uBoardsUpdated = await listen<any>('boards:updated', async () => {
          await loadBoards();
          await loadTasks();
          useCortexStore.getState().syncStagesFromColumns(useBoardStore.getState().columns);
        });
        unlisteners.push(uBoardsUpdated);

        const uPipelineUpdated = await listen<any>('pipeline:updated', async () => {
          await loadBoards();
          await loadTasks();
          useCortexStore.getState().syncStagesFromColumns(useBoardStore.getState().columns);
        });
        unlisteners.push(uPipelineUpdated);

        const unlistenBoardStore = await initBoardEventListeners();
        unlisteners.push(unlistenBoardStore);
        const unlistenCortexStore = await initCortexEventListeners();
        unlisteners.push(unlistenCortexStore);
      } catch (e) {
        console.info('Tauri event listeners not active in web mode:', e);
      }
    };

    setupListeners();

    return () => {
      flushAll();
      for (const b of textBuffers.values()) {
        if (b.thoughtTimer) clearTimeout(b.thoughtTimer);
        if (b.outputTimer) clearTimeout(b.outputTimer);
      }
      textBuffers.clear();
      unlisteners.forEach((u) => u());
    };
  }, [loadBoards, loadTasks, addToolExecution, updateToolExecution, updateTelemetry, setThinkingStartTime]);

  return (
    <>
      <div
        style={rootStyle}
        className="flex flex-col h-screen w-screen overflow-hidden bg-[#fafafa] text-[#171717] select-none relative"
      >
        {/* Top Navigation Bar */}
        <Topbar />

        {/* Main Workspace Area */}
        <div className="flex-1 flex flex-row min-h-0 min-w-0 overflow-hidden relative">
          {/* Left Sidebar: Cortex Explorer */}
          <CortexExplorer />

          {/* Center: Kanban Board Control Plane */}
          <main className="flex-1 flex flex-col min-h-0 min-w-0 overflow-hidden">
            <KanbanBoard />
          </main>
        </div>

        {/* Mind / Rule Editor Modal */}
        <RuleEditorModal />

        {/* Global Tool Policy Matrix Modal */}
        <ToolPolicyModal />

        {/* LLM Settings Modal */}
        <SettingsModal />

        {/* Zoom Indicator Toast */}
        {showIndicator && (
          <div className="fixed bottom-4 right-4 z-50 flex items-center space-x-2 px-3 py-1.5 rounded-lg bg-[#171717]/90 text-white font-mono text-xs shadow-lg backdrop-blur-xs transition-opacity duration-150 animate-in fade-in select-none">
            <span>Масштаб: {zoomPercent}%</span>
            {zoomPercent !== 100 && (
              <button
                onClick={resetZoom}
                className="text-[#a1a1aa] hover:text-white underline text-[11px] ml-1 cursor-pointer"
              >
                Сброс
              </button>
            )}
          </div>
        )}
      </div>

      {/* Fullscreen Modal: Real-Time Agent Console (rendered at root level outside zoomed container to avoid position: fixed breakage and clipping) */}
      <ErrorBoundary scopeLabel="Консоль задачи" onClose={() => useDrawerStore.getState().closeDrawer()}>
        <AgentConsoleDrawer />
      </ErrorBoundary>

      {/* Terminal Bootloader Splash Screen */}
      {showSplash && (
        <SplashScreen
          isReady={isDataReady}
          onFinish={() => setShowSplash(false)}
        />
      )}
    </>
  );
}
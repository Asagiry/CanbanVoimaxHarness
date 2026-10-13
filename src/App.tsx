import { useEffect } from 'react';
import Topbar from './components/layout/Topbar';
import CortexExplorer from './components/cortex/CortexExplorer';
import KanbanBoard from './components/board/KanbanBoard';
import AgentConsoleDrawer from './components/drawer/AgentConsoleDrawer';
import RuleEditorModal from './components/cortex/RuleEditorModal';
import ToolPolicyModal from './components/settings/ToolPolicyModal';
import SettingsModal from './components/settings/SettingsModal';
import { useBoardStore } from './stores/boardStore';
import { useProjectStore } from './stores/projectStore';
import { useDrawerStore } from './stores/drawerStore';

export default function App() {
  const { loadColumns, loadTasks } = useBoardStore();
  const { loadProjects } = useProjectStore();
  const { appendThought, appendOutput, addToolExecution, updateToolExecution, updateTelemetry, setThinkingStartTime } = useDrawerStore();

  useEffect(() => {
    loadProjects().then(async () => {
      // Columns first: task mapping (columnId fallback, persona, status) resolves via columns
      await loadColumns();
      await loadTasks();
    });
  }, [loadProjects, loadColumns, loadTasks]);

  useEffect(() => {
    let unlisteners: (() => void)[] = [];

    const setupListeners = async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');

        const u1 = await listen<any>('agent:thought', (event) => {
          const { taskId, thought, delta } = event.payload || {};
          if (taskId && (thought || delta)) {
            appendThought(taskId, thought || delta);
          }
        });
        unlisteners.push(u1);

        const u2 = await listen<any>('agent:output', (event) => {
          const { taskId, output, content, token } = event.payload || {};
          const text = output || content || token;
          if (taskId && text) {
            appendOutput(taskId, text, 'output');
          }
        });
        unlisteners.push(u2);

        const u3 = await listen<any>('agent:tool_call', (event) => {
          const { taskId, id, name, arguments: args } = event.payload || {};
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
            useDrawerStore.getState().setFinalTps(taskId, null);
            setThinkingStartTime(taskId, Date.now());
          }
        });
        unlisteners.push(uTurnStart);

        const u4 = await listen<any>('task:moved', () => {
          loadTasks();
        });
        unlisteners.push(u4);

        const u5 = await listen<any>('agent:finished', (event) => {
          const { taskId, response } = event.payload || {};
          if (taskId) {
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
                appendOutput(taskId, response, 'output');
              }
            }
            // Immediately flush task activity feed to disk upon turn finish
            useDrawerStore.getState().flushSaveDialog(taskId);
          }
          loadTasks();
        });
        unlisteners.push(u5);

        const u6 = await listen<any>('agent:status', (event) => {
          const { taskId, status } = event.payload || {};
          if (taskId && (status === 'finished' || status === 'aborted' || status === 'error')) {
            useDrawerStore.getState().flushSaveDialog(taskId);
          }
          loadTasks();
        });
        unlisteners.push(u6);

        const uProj = await listen<any>('project:changed', async () => {
          await loadColumns();
          loadTasks();
        });
        unlisteners.push(uProj);
      } catch (e) {
        console.info('Tauri event listeners not active in web mode:', e);
      }
    };

    setupListeners();

    return () => {
      unlisteners.forEach((u) => u());
    };
  }, [loadColumns, loadTasks, appendThought, appendOutput, addToolExecution, updateToolExecution, updateTelemetry, setThinkingStartTime]);

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden bg-[#fafafa] text-[#171717] select-none">
      {/* Top Navigation Bar */}
      <Topbar />

      {/* Main Workspace Area */}
      <div className="flex-1 flex flex-row overflow-hidden relative">
        {/* Left Sidebar: Cortex Explorer */}
        <CortexExplorer />

        {/* Center: Kanban Board Control Plane */}
        <main className="flex-1 flex flex-col min-w-0 overflow-hidden">
          <KanbanBoard />
        </main>

        {/* Fullscreen Modal: Real-Time Agent Console */}
        <AgentConsoleDrawer />
      </div>

      {/* Mind / Rule Editor Modal */}
      <RuleEditorModal />

      {/* Global Tool Policy Matrix Modal */}
      <ToolPolicyModal />

      {/* LLM Settings Modal */}
      <SettingsModal />
    </div>
  );
}

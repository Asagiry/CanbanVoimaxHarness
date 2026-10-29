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
import { useBoardStore } from './stores/boardStore';
import { useProjectStore } from './stores/projectStore';
import { useDrawerStore } from './stores/drawerStore';
import { useCortexStore } from './stores/cortexStore';
import { useToolPolicyStore } from './stores/toolPolicyStore';
import { useUiZoom } from './hooks/useUiZoom';
import { initNotifications } from './services/notificationService';
import { setupTauriEventGateway } from './services/tauriEventGateway';

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
  const loadBoards = useBoardStore((s) => s.loadBoards);
  const loadProjects = useProjectStore((s) => s.loadProjects);
  const hasActiveProject = useProjectStore((s) => s.hasActiveProject);

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
        await useToolPolicyStore.getState().loadFromTauri();
        void initNotifications();
      } catch (e) {
        console.warn('Initialization error:', e);
      } finally {
        setIsDataReady(true);
      }
    };
    void init();
  }, [loadProjects, loadBoards, loadTasks]);

  useEffect(() => {
    return setupTauriEventGateway();
  }, []);

  return (
    <>
      <div
        style={rootStyle}
        className="flex flex-col h-screen w-screen overflow-hidden bg-[#fafafa] dark:bg-[#090a0f] text-[#171717] dark:text-[#f0f6fc] select-none relative transition-colors"
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
      </div>

      {/* Modals rendered at root level outside zoom-scaled container to prevent position:fixed containment trap (§ 2.13) */}
      <RuleEditorModal />
      <ToolPolicyModal />
      <SettingsModal />

      {/* Fullscreen Modal: Real-Time Agent Console (rendered at root level outside zoomed container to avoid position: fixed breakage and clipping) */}
      <ErrorBoundary scopeLabel="Консоль задачи" onClose={() => useDrawerStore.getState().closeDrawer()}>
        <AgentConsoleDrawer />
      </ErrorBoundary>

      {/* Zoom Indicator Toast (rendered at root level above modals and drawer with z-[120]) */}
      {showIndicator && (
        <div className="fixed bottom-4 right-4 z-[120] flex items-center space-x-2 px-3 py-1.5 rounded-md bg-[#161b22] text-[#f0f6fc] border border-[#30363d] font-mono text-xs shadow-md transition-opacity duration-150 animate-in fade-in select-none">
          <span>Масштаб: {zoomPercent}%</span>
          {zoomPercent !== 100 && (
            <button
              onClick={resetZoom}
              className="text-[#8b949e] hover:text-[#f0f6fc] underline text-[11px] ml-1 cursor-pointer"
            >
              Сброс
            </button>
          )}
        </div>
      )}

      {/* Terminal Bootloader Splash Screen / Welcome Project Picker */}
      {(showSplash || !hasActiveProject) && (
        <SplashScreen
          isReady={isDataReady}
          onFinish={() => setShowSplash(false)}
        />
      )}
    </>
  );
}
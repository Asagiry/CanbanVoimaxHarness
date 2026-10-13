import { useEffect } from 'react';
import Topbar from './components/layout/Topbar';
import CortexExplorer from './components/cortex/CortexExplorer';
import KanbanBoard from './components/board/KanbanBoard';
import AgentConsoleDrawer from './components/drawer/AgentConsoleDrawer';
import RuleEditorModal from './components/cortex/RuleEditorModal';
import ToolPolicyModal from './components/settings/ToolPolicyModal';
import SettingsModal from './components/settings/SettingsModal';
import { useBoardStore } from './stores/boardStore';

export default function App() {
  const { loadTasks } = useBoardStore();

  useEffect(() => {
    loadTasks();
  }, [loadTasks]);

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

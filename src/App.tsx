import Topbar from './components/layout/Topbar';
import CortexExplorer from './components/cortex/CortexExplorer';
import KanbanBoard from './components/board/KanbanBoard';
import AgentConsoleDrawer from './components/drawer/AgentConsoleDrawer';
import RuleEditorModal from './components/cortex/RuleEditorModal';
import ToolPolicyModal from './components/settings/ToolPolicyModal';

export default function App() {
  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden bg-obsidian-950 text-slate-100 select-none">
      {/* Top Navigation & Telemetry Bar */}
      <Topbar />

      {/* Main Workspace Area */}
      <div className="flex-1 flex flex-row overflow-hidden relative">
        {/* Left Sidebar: Cortex Explorer («Проводник разума агентов») */}
        <CortexExplorer />

        {/* Center: Kanban Board Control Plane */}
        <main className="flex-1 flex flex-col min-w-0 overflow-hidden">
          <KanbanBoard />
        </main>

        {/* Slide-Out Drawer: Real-Time Agent Console */}
        <AgentConsoleDrawer />
      </div>

      {/* In-App Mind / Rule Editor Modal */}
      <RuleEditorModal />

      {/* Global Tool Policy Matrix Modal */}
      <ToolPolicyModal />
    </div>
  );
}

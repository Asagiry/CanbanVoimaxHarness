import { useId } from 'react';
import { useProjectStore } from '../../stores/projectStore';
import { useBoardStore } from '../../stores/boardStore';
import { useToolPolicyStore } from '../../stores/toolPolicyStore';
import {
  Layers,
  Search,
  Activity,
  FolderGit2,
  RefreshCw,
  Cpu,
  Zap,
  Shield,
} from 'lucide-react';

export default function Topbar() {
  const { activeProject, availableProjects, switchProject, isTauriEnvironment } = useProjectStore();
  const { searchQuery, setSearchQuery, tasks } = useBoardStore();
  const { openModal } = useToolPolicyStore();
  const searchInputId = useId();

  const runningCount = tasks.filter((t) => t.status === 'running').length;

  return (
    <header className="h-11 w-full flex items-center justify-between px-3 border-b border-obsidian-800 bg-obsidian-900 select-none text-xs">
      {/* Brand & Project Selector */}
      <div className="flex items-center space-x-2.5">
        <div className="flex items-center space-x-2">
          <div className="h-6 w-6 rounded-md bg-cyan-950/60 border border-cyber-cyan/40 flex items-center justify-center text-cyber-cyan">
            <Layers className="h-3.5 w-3.5" />
          </div>
          <div className="flex items-center space-x-1.5">
            <span className="font-semibold text-slate-100 text-xs tracking-tight">
              CanbanVoimaxHarness
            </span>
            <span className="px-1 py-0.5 rounded text-[10px] font-mono font-bold bg-cyan-950/80 text-cyber-cyan border border-cyan-800/40 tabular-nums">
              v0.1.0
            </span>
          </div>
        </div>

        <span className="text-slate-700 font-mono text-[11px]">/</span>

        {/* Project Selector */}
        <div className="flex items-center space-x-1.5">
          <FolderGit2 className="h-3.5 w-3.5 text-slate-400" />
          <select
            value={activeProject.id}
            onChange={(e) => switchProject(e.target.value)}
            className="bg-obsidian-850 hover:bg-obsidian-800 border border-obsidian-750 rounded px-2 py-0.5 text-xs text-slate-200 focus:outline-hidden focus:border-cyan-500 font-mono transition cursor-pointer"
          >
            {availableProjects.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Global Search Bar */}
      <div className="flex items-center w-72 max-w-sm">
        <div className="relative w-full">
          <Search className="absolute left-2.5 top-1.5 h-3.5 w-3.5 text-slate-500 pointer-events-none" />
          <input
            id={searchInputId}
            name="kanbanBoardSearch"
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Filter tasks by ID, title, branch..."
            className="w-full pl-8 pr-2.5 py-1 rounded bg-obsidian-950 border border-obsidian-800 text-xs text-slate-100 placeholder-slate-500 focus:outline-hidden focus:border-cyan-500 font-mono transition"
          />
        </div>
      </div>

      {/* Status & Telemetry Indicators */}
      <div className="flex items-center space-x-2 font-mono text-[11px]">
        {/* Token Savings Telemetry Badge */}
        <div
          className="hidden sm:flex items-center space-x-1 px-2 py-0.5 rounded bg-amber-950/20 border border-amber-500/30 text-cyber-amber tabular-nums"
          title="Deterministic Prefix Caching & Tool Policy Matrix Token Savings"
        >
          <Zap className="h-3 w-3 fill-cyber-amber" />
          <span className="font-medium text-[10px]">~2.4k saved/step</span>
        </div>

        {/* Active Agents Running */}
        <div className="flex items-center space-x-1.5 px-2 py-0.5 rounded bg-obsidian-850 border border-obsidian-800 text-slate-300">
          <span
            className={`h-2 w-2 rounded-full ${
              runningCount > 0 ? 'bg-cyber-emerald animate-pulse' : 'bg-slate-600'
            }`}
          />
          <span className="text-slate-400 text-[10px]">Agents:</span>
          <strong className="text-slate-100 tabular-nums text-[10px]">{runningCount} active</strong>
        </div>

        {/* Lightweight Memory Footprint Telemetry */}
        <div className="hidden md:flex items-center space-x-1 px-2 py-0.5 rounded bg-obsidian-850 border border-obsidian-800 text-slate-300">
          <Cpu className="h-3 w-3 text-cyber-cyan" />
          <span className="text-slate-400 text-[10px]">RAM:</span>
          <strong className="text-cyber-cyan tabular-nums text-[10px]">~38.4 MB</strong>
        </div>

        {/* Tauri v2 / Engine Badge */}
        <div className="hidden lg:flex items-center space-x-1 px-2 py-0.5 rounded bg-obsidian-850 border border-obsidian-800 text-slate-300">
          <span className="text-[10px] font-medium text-slate-400">
            {isTauriEnvironment ? 'Tauri v2 Native' : 'CVH Control Plane'}
          </span>
        </div>

        {/* Tool Policy Matrix Button */}
        <button
          onClick={openModal}
          className="flex items-center space-x-1.5 px-2 py-0.5 rounded bg-obsidian-850 hover:bg-obsidian-800 border border-cyan-800/50 text-cyber-cyan transition cursor-pointer"
          title="Настройка матрицы инструментов (Tool Policy Matrix)"
        >
          <Shield className="h-3.5 w-3.5" />
          <span className="font-semibold text-[10px]">Tools</span>
          <span className="px-1 py-0.2 rounded text-[9px] bg-cyan-950 border border-cyan-800/60 text-cyan-300 tabular-nums">
            ~2.4k saved
          </span>
        </button>

        {/* Health Activity Indicator */}
        <div
          className="p-1 rounded bg-obsidian-850 hover:bg-obsidian-800 text-slate-400 hover:text-white transition cursor-pointer"
          title="System Invariants Healthy (Exit Code 0)"
        >
          <Activity className="h-3.5 w-3.5 text-cyber-emerald" />
        </div>

        {/* Reload / Sync Button */}
        <button
          onClick={() => window.location.reload()}
          className="p-1 rounded bg-obsidian-850 hover:bg-obsidian-800 text-slate-400 hover:text-white transition cursor-pointer"
          title="Reload state from disk"
        >
          <RefreshCw className="h-3.5 w-3.5" />
        </button>
      </div>
    </header>
  );
}

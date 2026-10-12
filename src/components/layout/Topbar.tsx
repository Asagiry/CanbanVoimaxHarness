import { useId } from 'react';
import { useProjectStore } from '../../stores/projectStore';
import { useBoardStore } from '../../stores/boardStore';
import { useToolPolicyStore } from '../../stores/toolPolicyStore';
import {
  Layers,
  Search,
  FolderGit2,
  Settings,
} from 'lucide-react';

export default function Topbar() {
  const { activeProject, availableProjects, switchProject } = useProjectStore();
  const { searchQuery, setSearchQuery } = useBoardStore();
  const { openModal } = useToolPolicyStore();
  const searchInputId = useId();

  return (
    <header className="h-10 w-full flex items-center justify-between px-3 border-b border-obsidian-800 bg-obsidian-900 select-none text-xs">
      {/* Brand & Project Selector */}
      <div className="flex items-center space-x-3">
        <div className="flex items-center space-x-2">
          <Layers className="h-4 w-4 text-slate-300" />
          <span className="font-semibold text-slate-100 text-xs tracking-tight">
            CanbanVoimaxHarness
          </span>
        </div>

        <span className="text-obsidian-750 font-mono text-xs">/</span>

        {/* Project Selector */}
        <div className="flex items-center space-x-1.5">
          <FolderGit2 className="h-3.5 w-3.5 text-slate-500" />
          <select
            value={activeProject.id}
            onChange={(e) => switchProject(e.target.value)}
            className="bg-obsidian-950 hover:bg-obsidian-850 border border-obsidian-800 rounded px-2 py-0.5 text-xs text-slate-200 focus:outline-hidden focus:border-slate-500 font-mono transition cursor-pointer"
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
      <div className="flex items-center w-80 max-w-md">
        <div className="relative w-full">
          <Search className="absolute left-2.5 top-1.5 h-3.5 w-3.5 text-slate-500 pointer-events-none" />
          <input
            id={searchInputId}
            name="kanbanBoardSearch"
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Поиск задач..."
            title="Поиск задач"
            className="w-full pl-8 pr-2.5 py-1 rounded bg-obsidian-950 border border-obsidian-800 text-xs text-slate-100 placeholder-slate-500 focus:outline-hidden focus:border-slate-600 font-sans transition"
          />
        </div>
      </div>

      {/* Actions */}
      <div className="flex items-center space-x-2">
        <button
          onClick={openModal}
          className="p-1.5 rounded text-slate-400 hover:text-slate-100 hover:bg-obsidian-800 transition cursor-pointer"
          title="Настройки"
        >
          <Settings className="h-4 w-4" />
        </button>
      </div>
    </header>
  );
}

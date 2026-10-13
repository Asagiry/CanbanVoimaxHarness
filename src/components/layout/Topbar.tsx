import { useRef, useId } from 'react';
import { useProjectStore } from '../../stores/projectStore';
import { useBoardStore } from '../../stores/boardStore';
import { useSettingsStore } from '../../stores/settingsStore';
import {
  FolderOpen,
  Search,
  Settings,
} from 'lucide-react';

export default function Topbar() {
  const { activeProject, availableProjects, switchProject, openFolder } = useProjectStore();
  const { searchQuery, setSearchQuery, loadTasks } = useBoardStore();
  const { openSettings } = useSettingsStore();
  const searchInputId = useId();
  const folderInputRef = useRef<HTMLInputElement>(null);

  const handleOpenFolderClick = async () => {
    // 1. Try Tauri native folder dialog
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const selectedPath = await invoke<string | null>('choose_project_folder');
      if (selectedPath) {
        await openFolder(selectedPath);
        await loadTasks();
        return;
      }
    } catch {
      // Fallback to web directory input or prompt
    }

    // 2. Fallback to hidden directory file input
    if (folderInputRef.current) {
      folderInputRef.current.click();
    }
  };

  const handleDirectorySelected = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (files && files.length > 0) {
      const firstFile = files[0];
      // On web, relative path contains the root folder name
      const path = (firstFile as any).webkitRelativePath || firstFile.name;
      const rootFolder = path.split('/')[0] || 'Selected Project';
      await openFolder(rootFolder);
      await loadTasks();
    }
  };

  return (
    <header className="h-9 w-full flex items-center justify-between px-3 border-b border-obsidian-800/80 bg-obsidian-900/90 select-none text-xs">
      {/* Hidden Web Directory Picker */}
      <input
        ref={folderInputRef}
        type="file"
        // @ts-ignore
        webkitdirectory="true"
        // @ts-ignore
        directory="true"
        onChange={handleDirectorySelected}
        className="hidden"
      />

      {/* Left: Project Brand & Folder Picker */}
      <div className="flex items-center space-x-2">
        <span className="font-semibold text-slate-200 text-xs tracking-tight font-sans">
          {activeProject.name || 'CanbanVoimaxHarness'}
        </span>

        {availableProjects.length > 1 && (
          <select
            value={activeProject.id}
            onChange={(e) => {
              switchProject(e.target.value);
              loadTasks();
            }}
            className="bg-obsidian-850 hover:bg-obsidian-800 border border-obsidian-750 rounded px-2 py-0.5 text-xs text-slate-300 font-mono transition cursor-pointer"
          >
            {availableProjects.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
        )}

        <button
          onClick={handleOpenFolderClick}
          className="flex items-center space-x-1 px-2 py-0.5 rounded bg-obsidian-850 hover:bg-obsidian-800 border border-obsidian-750 text-slate-300 hover:text-white transition cursor-pointer font-sans text-xs"
          title="Выбрать папку проекта"
        >
          <FolderOpen className="h-3 w-3 text-slate-400" />
          <span>Открыть папку</span>
        </button>
      </div>

      {/* Center: Clean Search Bar */}
      <div className="flex items-center w-56 sm:w-64 max-w-sm">
        <div className="relative w-full">
          <Search className="absolute left-2 top-1.5 h-3.5 w-3.5 text-slate-500 pointer-events-none" />
          <input
            id={searchInputId}
            name="kanbanBoardSearch"
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Поиск..."
            className="w-full pl-7 pr-2 py-0.5 rounded bg-obsidian-950 border border-obsidian-800 text-xs text-slate-200 placeholder-slate-500 focus:outline-hidden focus:border-cyan-500/70 font-mono transition"
          />
        </div>
      </div>

      {/* Right: Only Settings Icon */}
      <div className="flex items-center space-x-1">
        <button
          onClick={openSettings}
          className="p-1 rounded bg-obsidian-850 hover:bg-obsidian-800 border border-obsidian-750 text-slate-400 hover:text-slate-200 transition cursor-pointer"
          title="Настройки LLM"
        >
          <Settings className="h-3.5 w-3.5" />
        </button>
      </div>
    </header>
  );
}

import { useRef, useId } from 'react';
import { useProjectStore } from '../../stores/projectStore';
import { useBoardStore } from '../../stores/boardStore';
import { useSettingsStore } from '../../stores/settingsStore';
import { useToolPolicyStore } from '../../stores/toolPolicyStore';
import {
  FolderOpen,
  Search,
  Settings,
  Wrench,
} from 'lucide-react';

export default function Topbar() {
  const { activeProject, availableProjects, switchProject, openFolder } = useProjectStore();
  const { searchQuery, setSearchQuery, loadTasks } = useBoardStore();
  const { openSettings } = useSettingsStore();
  const { openModal: openToolPolicyModal } = useToolPolicyStore();
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
    <header className="h-10 w-full flex items-center justify-between px-3.5 border-b border-[#ebebeb] bg-white select-none text-xs text-[#171717]">
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
        <span className="font-semibold text-[#171717] text-xs tracking-tight font-sans">
          {activeProject.name || 'CanbanVoimaxHarness'}
        </span>

        {availableProjects.length > 1 && (
          <select
            value={activeProject.id}
            onChange={(e) => {
              switchProject(e.target.value);
              loadTasks();
            }}
            className="bg-white hover:bg-[#f9f9f9] border border-[#ebebeb] rounded-md px-2 py-1 text-xs text-[#171717] font-mono transition cursor-pointer"
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
          className="flex items-center space-x-1.5 px-2.5 py-1 rounded-md bg-white hover:bg-[#f9f9f9] border border-[#ebebeb] text-[#4d4d4d] hover:text-[#171717] transition cursor-pointer font-sans text-xs"
          title="Выбрать папку проекта"
        >
          <FolderOpen className="h-3.5 w-3.5 text-[#666666]" />
          <span>Открыть папку</span>
        </button>
      </div>

      {/* Center: Clean Search Bar */}
      <div className="flex items-center w-56 sm:w-64 max-w-sm">
        <div className="relative w-full">
          <Search className="absolute left-2.5 top-2 h-3.5 w-3.5 text-[#8f8f8f] pointer-events-none" />
          <input
            id={searchInputId}
            name="kanbanBoardSearch"
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Поиск..."
            className="w-full pl-8 pr-2.5 py-1 rounded-md bg-[#f9f9f9] border border-[#ebebeb] text-xs text-[#171717] placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] focus:bg-white font-sans transition"
          />
        </div>
      </div>

      {/* Right: Only Settings Icon */}
      <div className="flex items-center space-x-1.5">
        <button
          onClick={openToolPolicyModal}
          className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] border border-[#ebebeb] text-[#666666] hover:text-[#171717] transition cursor-pointer"
          title="Матрица инструментов — доступ тулов по агентам"
        >
          <Wrench className="h-3.5 w-3.5" />
        </button>
        <button
          onClick={openSettings}
          className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] border border-[#ebebeb] text-[#666666] hover:text-[#171717] transition cursor-pointer"
          title="Настройки LLM"
        >
          <Settings className="h-3.5 w-3.5" />
        </button>
      </div>
    </header>
  );
}

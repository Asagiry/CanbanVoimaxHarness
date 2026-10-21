import { useRef } from 'react';
import { useProjectStore } from '../../stores/projectStore';
import { useBoardStore } from '../../stores/boardStore';
import { useSettingsStore } from '../../stores/settingsStore';
import { useToolPolicyStore } from '../../stores/toolPolicyStore';
import Logo from '../common/Logo';
import WindowControls, { toggleMaximizeWindow } from './WindowControls';
import {
  FolderOpen,
  Settings,
  Wrench,
  Sun,
  Moon,
} from 'lucide-react';
import { useThemeStore } from '../../stores/themeStore';

export default function Topbar() {
  const { activeProject, openFolder, hasActiveProject } = useProjectStore();
  const currentFolderName = (activeProject.rootPath || '').split(/[\\/]/).filter(Boolean).pop() || activeProject.name || '';
  const { loadTasks } = useBoardStore();
  const { openSettings } = useSettingsStore();
  const { openModal: openToolPolicyModal } = useToolPolicyStore();
  const { theme, toggleThemeWithAnimation } = useThemeStore();
  const folderInputRef = useRef<HTMLInputElement>(null);

  const handleHeaderDoubleClick = (e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    // Do not toggle maximize if double clicking an interactive element
    if (target.closest('button, input, select, textarea, [data-no-drag]')) {
      return;
    }
    void toggleMaximizeWindow();
  };

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
    <header
      data-tauri-drag-region
      onDoubleClick={handleHeaderDoubleClick}
      className="h-10 w-full flex items-center justify-between pl-3 pr-0 border-b border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] select-none text-xs text-[#171717] dark:text-[#f0f6fc] shrink-0 z-40 transition-colors"
    >
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

      {/* Left: Logo + Folder Picker (без дубля названия проекта) */}
      <div className="flex items-center space-x-2 shrink-0" data-tauri-drag-region>
        <div className="flex items-center cursor-default" data-no-drag>
          <Logo size={18} className="shrink-0" />
        </div>

        <button
          data-no-drag
          onMouseDown={(e) => e.stopPropagation()}
          onClick={handleOpenFolderClick}
          className="flex items-center space-x-1.5 px-2.5 py-1 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#161b22] dark:hover:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#4d4d4d] hover:text-[#171717] dark:text-[#8b949e] dark:hover:text-[#f0f6fc] transition cursor-pointer font-sans text-xs"
          title="Выбрать папку проекта"
        >
          <FolderOpen className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e]" />
          <span>Открыть папку</span>
        </button>

        {/* Current folder indicator: read-only caption, full path in tooltip */}
        {hasActiveProject && currentFolderName && (
          <span
            className="font-mono text-[11px] text-[#8f8f8f] dark:text-[#8b949e] truncate max-w-[220px] cursor-default"
            title={activeProject.rootPath || activeProject.name}
          >
            {currentFolderName}
          </span>
        )}
      </div>

      {/* Right: Modals triggers & Custom Window Controls */}
      <div className="flex items-center h-full space-x-1 shrink-0" data-tauri-drag-region>
        {/* Settings & Tools Modal Triggers */}
        <div className="flex items-center space-x-1.5 pr-2" data-no-drag>
          <button
            onMouseDown={(e) => e.stopPropagation()}
            onClick={openToolPolicyModal}
            className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#161b22] dark:hover:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#666666] hover:text-[#171717] dark:text-[#8b949e] dark:hover:text-[#f0f6fc] transition cursor-pointer"
            title="Матрица инструментов — доступ тулов по агентам"
          >
            <Wrench className="h-3.5 w-3.5" />
          </button>
          <button
            onMouseDown={(e) => e.stopPropagation()}
            onClick={openSettings}
            className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#161b22] dark:hover:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#666666] hover:text-[#171717] dark:text-[#8b949e] dark:hover:text-[#f0f6fc] transition cursor-pointer"
            title="Настройки LLM"
          >
            <Settings className="h-3.5 w-3.5" />
          </button>

          {/* Theme Toggle Button with Circular Wave Animation */}
          <button
            data-no-drag
            onMouseDown={(e) => e.stopPropagation()}
            onClick={(e) => toggleThemeWithAnimation(e)}
            className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#161b22] dark:hover:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#666666] hover:text-[#171717] dark:text-[#8b949e] dark:hover:text-[#f0f6fc] transition cursor-pointer relative overflow-hidden group"
            title={theme === 'dark' ? 'Включить светлую тему' : 'Включить тёмную тему'}
            aria-label="Сменить тему оформления"
          >
            <div className="relative h-3.5 w-3.5 flex items-center justify-center">
              <Sun
                className={`h-3.5 w-3.5 transition-all duration-300 transform absolute inset-0 ${
                  theme === 'dark'
                    ? 'rotate-90 scale-0 opacity-0'
                    : 'rotate-0 scale-100 opacity-100 text-amber-500'
                }`}
              />
              <Moon
                className={`h-3.5 w-3.5 transition-all duration-300 transform absolute inset-0 ${
                  theme === 'dark'
                    ? 'rotate-0 scale-100 opacity-100 text-indigo-400'
                    : '-rotate-90 scale-0 opacity-0'
                }`}
              />
            </div>
          </button>
        </div>

        {/* Vertical divider before window controls */}
        <div className="h-4 w-px bg-[#ebebeb] dark:bg-[#21262d] mx-1" />

        {/* Shared Frameless Window Controls */}
        <WindowControls />
      </div>
    </header>
  );
}
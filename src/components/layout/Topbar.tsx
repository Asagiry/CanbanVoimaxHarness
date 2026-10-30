import { useRef } from 'react';
import { useProjectStore, isInvalidProjectPath } from '../../stores/projectStore';
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
  X,
} from 'lucide-react';
import { useThemeStore } from '../../stores/themeStore';
import { useTranslation } from '../../stores/languageStore';

export default function Topbar() {
  const {
    activeProject,
    availableProjects,
    openFolder,
    switchProject,
    removeProjectFromHistory,
    hasActiveProject,
  } = useProjectStore();
  const { openSettings } = useSettingsStore();
  const { openModal: openToolPolicyModal } = useToolPolicyStore();
  const { theme, toggleThemeWithAnimation } = useThemeStore();
  const { language, toggleLanguage, t } = useTranslation();
  const folderInputRef = useRef<HTMLInputElement>(null);

  // Filter and deduplicate valid project tabs
  const validAvailable: typeof availableProjects = [];
  const seenPaths = new Set<string>();
  for (const p of availableProjects) {
    if (!p || isInvalidProjectPath(p.rootPath)) continue;
    const normalized = p.rootPath.toLowerCase();
    if (!seenPaths.has(normalized)) {
      seenPaths.add(normalized);
      validAvailable.push(p);
    }
  }

  // Ensure active project is represented if valid
  if (
    hasActiveProject &&
    !isInvalidProjectPath(activeProject.rootPath) &&
    !seenPaths.has(activeProject.rootPath.toLowerCase())
  ) {
    validAvailable.push(activeProject);
  }

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

      {/* Left: Logo + Folder Picker + Project Switcher Tabs */}
      <div className="flex items-center space-x-2 min-w-0 flex-1 overflow-hidden mr-2" data-tauri-drag-region>
        <div className="flex items-center cursor-default shrink-0" data-no-drag>
          <Logo size={18} className="shrink-0" />
        </div>

        <button
          data-no-drag
          onMouseDown={(e) => e.stopPropagation()}
          onClick={handleOpenFolderClick}
          className="flex items-center space-x-1.5 px-2.5 py-1 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#161b22] dark:hover:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#4d4d4d] hover:text-[#171717] dark:text-[#8b949e] dark:hover:text-[#f0f6fc] transition cursor-pointer font-sans text-xs shrink-0"
          aria-label={t('topbar.selectProjectFolder')}
        >
          <FolderOpen className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e]" />
          <span>{t('topbar.openFolder')}</span>
        </button>

        {/* Project Switcher Tabs */}
        {validAvailable.length > 0 && (
          <div
            className="flex items-center space-x-1 overflow-x-auto min-w-0 py-0.5"
            data-no-drag
          >
            {validAvailable.map((project) => {
              const isActive =
                project.id === activeProject.id ||
                (Boolean(project.rootPath) &&
                  Boolean(activeProject.rootPath) &&
                  project.rootPath.toLowerCase() === activeProject.rootPath.toLowerCase());
              const folderName =
                (project.rootPath || '').split(/[\\/]/).filter(Boolean).pop() || project.name;

              return (
                <div
                  key={project.id || project.rootPath}
                  data-no-drag
                  onMouseDown={(e) => e.stopPropagation()}
                  onClick={() => {
                    if (isActive) return;
                    void switchProject(project.rootPath || project.id);
                  }}
                  title={project.rootPath || project.name}
                  className={`group flex items-center space-x-1.5 px-2 py-1 rounded-md text-xs transition cursor-pointer select-none shrink-0 border ${
                    isActive
                      ? 'bg-[#f4f4f5] dark:bg-[#21262d] border-[#d4d4d8] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] font-medium'
                      : 'border-transparent text-[#666666] dark:text-[#8b949e] hover:bg-[#f9f9f9] dark:hover:bg-[#161b22] hover:border-[#ebebeb] dark:hover:border-[#21262d] hover:text-[#171717] dark:hover:text-[#f0f6fc]'
                  }`}
                >
                  <span className="max-w-[130px] truncate font-sans text-xs">
                    {folderName}
                  </span>
                  <button
                    type="button"
                    data-no-drag
                    onMouseDown={(e) => e.stopPropagation()}
                    onClick={(e) => {
                      e.stopPropagation();
                      void removeProjectFromHistory(project.rootPath || project.id);
                    }}
                    className="p-0.5 rounded text-[#8f8f8f] hover:text-[#171717] dark:text-[#8b949e] dark:hover:text-[#f0f6fc] hover:bg-[#e4e4e7] dark:hover:bg-[#30363d] transition cursor-pointer flex items-center justify-center"
                    aria-label={`Close ${folderName}`}
                  >
                    <X className="h-3 w-3" />
                  </button>
                </div>
              );
            })}
          </div>
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
            aria-label={t('topbar.toolPolicyTitle')}
          >
            <Wrench className="h-3.5 w-3.5" />
          </button>
          <button
            onMouseDown={(e) => e.stopPropagation()}
            onClick={openSettings}
            className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#161b22] dark:hover:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#666666] hover:text-[#171717] dark:text-[#8b949e] dark:hover:text-[#f0f6fc] transition cursor-pointer"
            aria-label={t('topbar.llmSettings')}
          >
            <Settings className="h-3.5 w-3.5" />
          </button>

          {/* Language Switcher Button (EN / RU) */}
          <button
            data-no-drag
            type="button"
            onMouseDown={(e) => e.stopPropagation()}
            onClick={toggleLanguage}
            className="px-2 py-1 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#161b22] dark:hover:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#666666] hover:text-[#171717] dark:text-[#8b949e] dark:hover:text-[#f0f6fc] transition cursor-pointer font-mono font-semibold text-[11px] select-none flex items-center space-x-1"
            aria-label={t('topbar.switchLanguage')}
          >
            <span className={language === 'en' ? 'text-[#171717] dark:text-[#f0f6fc] font-bold' : 'text-[#8f8f8f] dark:text-[#6e7681]'}>
              EN
            </span>
            <span className="text-[#d4d4d8] dark:text-[#30363d]">/</span>
            <span className={language === 'ru' ? 'text-[#171717] dark:text-[#f0f6fc] font-bold' : 'text-[#8f8f8f] dark:text-[#6e7681]'}>
              RU
            </span>
          </button>

          {/* Theme Toggle Button with Circular Wave Animation */}
          <button
            data-no-drag
            onMouseDown={(e) => e.stopPropagation()}
            onClick={(e) => toggleThemeWithAnimation(e)}
            className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#161b22] dark:hover:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#666666] hover:text-[#171717] dark:text-[#8b949e] dark:hover:text-[#f0f6fc] transition cursor-pointer relative overflow-hidden group"
            aria-label={t('topbar.switchTheme')}
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

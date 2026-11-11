import { useRef, type InputHTMLAttributes } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { useProjectStore, isInvalidProjectPath } from '../../stores/projectStore';
import { useSettingsStore } from '../../stores/settingsStore';
import { useToolPolicyStore } from '../../stores/toolPolicyStore';
import { useAgentSessionStore } from '../../stores/agentSessionStore';
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

type DirectoryInputAttributes = InputHTMLAttributes<HTMLInputElement> & {
  webkitdirectory?: string;
  directory?: string;
};

export default function Topbar() {
  const {
    activeProject,
    availableProjects,
    openFolder,
    switchProject,
    removeProjectFromHistory,
    hasActiveProject,
  } = useProjectStore(
    useShallow((s) => ({
      activeProject: s.activeProject,
      availableProjects: s.availableProjects,
      openFolder: s.openFolder,
      switchProject: s.switchProject,
      removeProjectFromHistory: s.removeProjectFromHistory,
      hasActiveProject: s.hasActiveProject,
    }))
  );
  const openSettings = useSettingsStore((s) => s.openSettings);
  const openToolPolicyModal = useToolPolicyStore((s) => s.openModal);
  const { theme, toggleThemeWithAnimation } = useThemeStore(
    useShallow((s) => ({ theme: s.theme, toggleThemeWithAnimation: s.toggleThemeWithAnimation }))
  );
  const { language, toggleLanguage, t } = useTranslation();
  const sessions = useAgentSessionStore((s) => s.sessions);
  const folderInputRef = useRef<HTMLInputElement>(null);

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

  if (
    hasActiveProject &&
    !isInvalidProjectPath(activeProject.rootPath) &&
    !seenPaths.has(activeProject.rootPath.toLowerCase())
  ) {
    validAvailable.push(activeProject);
  }

  const handleHeaderDoubleClick = (e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    if (target.closest('button, input, select, textarea, [data-no-drag]')) {
      return;
    }
    void toggleMaximizeWindow();
  };

  const handleOpenFolderClick = async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const selectedPath = await invoke<string | null>('choose_project_folder');
      if (selectedPath) {
        await openFolder(selectedPath);
        return;
      }
    } catch (err) {
      console.debug('[Topbar] choose_project_folder unavailable in web mode:', err);
    }

    if (folderInputRef.current) {
      folderInputRef.current.click();
    }
  };

  const handleDirectorySelected = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (files && files.length > 0) {
      const firstFile = files[0] as File & { webkitRelativePath?: string };
      const path = firstFile.webkitRelativePath || firstFile.name;
      const rootFolder = path.split('/')[0] || 'Selected Project';
      await openFolder(rootFolder);
    }
  };

  const directoryInputProps: DirectoryInputAttributes = {
    type: 'file',
    webkitdirectory: 'true',
    directory: 'true',
    onChange: handleDirectorySelected,
    className: 'hidden',
  };

  return (
    <header
      data-tauri-drag-region
      onDoubleClick={handleHeaderDoubleClick}
      className="h-10 w-full flex items-center justify-between pl-3 pr-0 border-b border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114] select-none text-xs text-[#171717] dark:text-[#f4f4f5] shrink-0 z-40 transition-colors"
    >
      <input ref={folderInputRef} {...directoryInputProps} />

      {/* Left: Logo + Folder Picker + Project Switcher Tabs */}
      <div className="flex items-center space-x-2 min-w-0 flex-1 overflow-hidden mr-2" data-tauri-drag-region>
        <div className="flex items-center cursor-default shrink-0" data-no-drag>
          <Logo size={18} className="shrink-0" />
        </div>

        <button
          type="button"
          data-no-drag
          onMouseDown={(e) => e.stopPropagation()}
          onClick={handleOpenFolderClick}
          className="flex items-center space-x-1.5 px-2.5 py-1 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#17171b] dark:hover:bg-[#232328] border border-[#ebebeb] dark:border-[#2c2c33] text-[#4d4d4d] hover:text-[#171717] dark:text-[#71717a] dark:hover:text-[#f4f4f5] transition cursor-pointer font-sans text-xs shrink-0 btn-tactile"
          aria-label={t('topbar.selectProjectFolder')}
        >
          <FolderOpen className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a]" />
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

              const runningSessionsCount = Object.values(sessions).filter((s) => {
                if (s.status !== 'running') return false;
                if (!s.projectPath || !project.rootPath) return false;
                const normSession = s.projectPath.replace(/\\/g, '/').toLowerCase().replace(/\/+$/, '');
                const normProj = project.rootPath.replace(/\\/g, '/').toLowerCase().replace(/\/+$/, '');
                return normSession === normProj;
              }).length;
              const isAgentRunningInProject = runningSessionsCount > 0;

              return (
                <div
                  key={project.id || project.rootPath}
                  data-no-drag
                  className={`group flex items-center space-x-1 px-2 py-1 rounded-md text-xs transition select-none shrink-0 border ${
                    isActive
                      ? 'bg-[#f4f4f5] dark:bg-[#232328] border-[#d4d4d8] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] font-medium'
                      : 'border-transparent text-[#666666] dark:text-[#71717a] hover:bg-[#f9f9f9] dark:hover:bg-[#17171b] hover:border-[#ebebeb] dark:hover:border-[#212126] hover:text-[#171717] dark:hover:text-[#f4f4f5]'
                  }`}
                >
                  <button
                    type="button"
                    data-no-drag
                    onMouseDown={(e) => e.stopPropagation()}
                    onClick={() => {
                      if (isActive) return;
                      void switchProject(project.rootPath || project.id);
                    }}
                    title={project.rootPath || project.name}
                    className="flex items-center space-x-1.5 cursor-pointer min-w-0"
                  >
                    {isAgentRunningInProject && (
                      <span
                        className="flex items-center space-x-1 shrink-0"
                        title={`${runningSessionsCount} active agent(s)`}
                      >
                        <span className="relative flex h-2 w-2">
                          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
                          <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500" />
                        </span>
                        {runningSessionsCount > 1 && (
                          <span className="font-mono text-xs text-emerald-600 dark:text-emerald-400 font-semibold tabular-nums leading-none">
                            {runningSessionsCount}
                          </span>
                        )}
                      </span>
                    )}
                    <span className="max-w-[130px] truncate font-sans text-xs">
                      {folderName}
                    </span>
                  </button>
                  <button
                    type="button"
                    data-no-drag
                    onMouseDown={(e) => e.stopPropagation()}
                    onClick={(e) => {
                      e.stopPropagation();
                      void removeProjectFromHistory(project.rootPath || project.id);
                    }}
                    className="p-0.5 rounded text-[#8f8f8f] hover:text-[#171717] dark:text-[#71717a] dark:hover:text-[#f4f4f5] hover:bg-[#e4e4e7] dark:hover:bg-[#2c2c33] transition cursor-pointer flex items-center justify-center"
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
        <div className="flex items-center space-x-1.5 pr-2" data-no-drag>
          <button
            type="button"
            onMouseDown={(e) => e.stopPropagation()}
            onClick={openToolPolicyModal}
            className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#17171b] dark:hover:bg-[#232328] border border-[#ebebeb] dark:border-[#2c2c33] text-[#666666] hover:text-[#171717] dark:text-[#71717a] dark:hover:text-[#f4f4f5] transition cursor-pointer btn-tactile"
            aria-label={t('topbar.toolPolicyTitle')}
          >
            <Wrench className="h-3.5 w-3.5 block shrink-0" />
          </button>
          <button
            type="button"
            onMouseDown={(e) => e.stopPropagation()}
            onClick={openSettings}
            className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#17171b] dark:hover:bg-[#232328] border border-[#ebebeb] dark:border-[#2c2c33] text-[#666666] hover:text-[#171717] dark:text-[#71717a] dark:hover:text-[#f4f4f5] transition cursor-pointer btn-tactile"
            aria-label={t('topbar.llmSettings')}
          >
            <Settings className="h-3.5 w-3.5 block shrink-0" />
          </button>

          <button
            data-no-drag
            type="button"
            onMouseDown={(e) => e.stopPropagation()}
            onClick={toggleLanguage}
            className="px-2 py-1 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#17171b] dark:hover:bg-[#232328] border border-[#ebebeb] dark:border-[#2c2c33] text-[#666666] hover:text-[#171717] dark:text-[#71717a] dark:hover:text-[#f4f4f5] transition cursor-pointer font-mono font-semibold text-[11px] select-none flex items-center space-x-1 btn-tactile"
            aria-label={t('topbar.switchLanguage')}
          >
            <span className={language === 'en' ? 'text-[#171717] dark:text-[#f4f4f5] font-bold' : 'text-[#8f8f8f] dark:text-[#52525b]'}>
              EN
            </span>
            <span className="text-[#d4d4d8] dark:text-[#2c2c33]">/</span>
            <span className={language === 'ru' ? 'text-[#171717] dark:text-[#f4f4f5] font-bold' : 'text-[#8f8f8f] dark:text-[#52525b]'}>
              RU
            </span>
          </button>

          <button
            type="button"
            data-no-drag
            onMouseDown={(e) => e.stopPropagation()}
            onClick={(e) => toggleThemeWithAnimation(e)}
            className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#17171b] dark:hover:bg-[#232328] border border-[#ebebeb] dark:border-[#2c2c33] text-[#666666] hover:text-[#171717] dark:text-[#71717a] dark:hover:text-[#f4f4f5] transition cursor-pointer relative overflow-hidden group btn-tactile"
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
                    ? 'rotate-0 scale-100 opacity-100 text-sky-400'
                    : '-rotate-90 scale-0 opacity-0'
                }`}
              />
            </div>
          </button>
        </div>

        <div className="h-4 w-px bg-[#ebebeb] dark:bg-[#212126] mx-1" />

        <WindowControls />
      </div>
    </header>
  );
}

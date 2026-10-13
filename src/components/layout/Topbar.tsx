import { useRef, useId, useState, useEffect } from 'react';
import { useProjectStore } from '../../stores/projectStore';
import { useBoardStore } from '../../stores/boardStore';
import { useSettingsStore } from '../../stores/settingsStore';
import { useToolPolicyStore } from '../../stores/toolPolicyStore';
import Logo from '../common/Logo';
import {
  FolderOpen,
  Layout,
  Search,
  Settings,
  Wrench,
  Minus,
  Square,
  Copy,
  X,
} from 'lucide-react';

export default function Topbar() {
  const { activeProject, availableProjects, switchProject, openFolder } = useProjectStore();
  const { searchQuery, setSearchQuery, loadTasks, boards, activeBoardId, setActiveBoard } = useBoardStore();
  const { openSettings } = useSettingsStore();
  const { openModal: openToolPolicyModal } = useToolPolicyStore();
  const searchInputId = useId();
  const folderInputRef = useRef<HTMLInputElement>(null);

  const [isMaximized, setIsMaximized] = useState(false);

  // Initialize and track Tauri window state
  useEffect(() => {
    let isMounted = true;
    let unlistenResize: (() => void) | undefined;

    const setupWindow = async () => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        const win = getCurrentWindow();
        const max = await win.isMaximized();
        if (!isMounted) return;
        setIsMaximized(max);

        const unlisten = await win.onResized(async () => {
          try {
            const currentMax = await win.isMaximized();
            if (isMounted) {
              setIsMaximized(currentMax);
            }
          } catch {
            // ignore non-tauri errors
          }
        });

        if (!isMounted) {
          unlisten();
        } else {
          unlistenResize = unlisten;
        }
      } catch {
        // Fallback for web preview mode
      }
    };

    void setupWindow();

    return () => {
      isMounted = false;
      if (unlistenResize) {
        unlistenResize();
      }
    };
  }, []);

  const handleMinimize = async () => {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().minimize();
    } catch (e) {
      console.info('Window minimize not available in web mode', e);
    }
  };

  const handleToggleMaximize = async () => {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      const win = getCurrentWindow();
      await win.toggleMaximize();
      const max = await win.isMaximized();
      setIsMaximized(max);
    } catch (e) {
      console.info('Window toggle maximize not available in web mode', e);
    }
  };

  const handleClose = async () => {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().close();
    } catch (e) {
      console.info('Window close not available in web mode', e);
    }
  };

  const handleHeaderDoubleClick = (e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    // Do not toggle maximize if double clicking an interactive element
    if (target.closest('button, input, select, textarea, [data-no-drag]')) {
      return;
    }
    void handleToggleMaximize();
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
      className="h-10 w-full flex items-center justify-between pl-3 pr-0 border-b border-[#ebebeb] bg-white select-none text-xs text-[#171717] shrink-0 z-40"
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

      {/* Left: Futuristic Logo + Project Brand & Folder Picker */}
      <div className="flex items-center space-x-2 shrink-0" data-tauri-drag-region>
        <div className="flex items-center space-x-2 cursor-default" data-no-drag>
          <Logo size={20} glow animated className="drop-shadow-xs" />
          <span className="font-semibold text-[#171717] text-xs tracking-tight font-sans">
            {activeProject.name || 'CanbanVoimaxHarness'}
          </span>
        </div>

        {availableProjects.length > 1 && (
          <select
            data-no-drag
            onMouseDown={(e) => e.stopPropagation()}
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
          data-no-drag
          onMouseDown={(e) => e.stopPropagation()}
          onClick={handleOpenFolderClick}
          className="flex items-center space-x-1.5 px-2.5 py-1 rounded-md bg-white hover:bg-[#f9f9f9] border border-[#ebebeb] text-[#4d4d4d] hover:text-[#171717] transition cursor-pointer font-sans text-xs"
          title="Выбрать папку проекта"
        >
          <FolderOpen className="h-3.5 w-3.5 text-[#666666]" />
          <span>Открыть папку</span>
        </button>

        {boards.length > 1 && (
          <div className="flex items-center space-x-1 pl-1.5 border-l border-[#ebebeb]" data-no-drag>
            <Layout className="h-3 w-3 text-[#71717a]" />
            <select
              onMouseDown={(e) => e.stopPropagation()}
              value={activeBoardId}
              onChange={(e) => setActiveBoard(e.target.value)}
              className="bg-white hover:bg-[#f9f9f9] border border-[#ebebeb] rounded-md px-2 py-0.5 text-xs text-[#171717] font-medium transition cursor-pointer"
              title="Переключить доску"
            >
              {boards.map((b) => (
                <option key={b.id} value={b.id}>
                  {b.name}
                </option>
              ))}
            </select>
          </div>
        )}
      </div>

      {/* Center: Clean Search Bar */}
      <div className="flex items-center w-56 sm:w-64 max-w-sm mx-4" data-no-drag>
        <div className="relative w-full">
          <Search className="absolute left-2.5 top-2 h-3.5 w-3.5 text-[#8f8f8f] pointer-events-none" />
          <input
            id={searchInputId}
            name="kanbanBoardSearch"
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Поиск..."
            onMouseDown={(e) => e.stopPropagation()}
            className="w-full pl-8 pr-2.5 py-1 rounded-md bg-[#f9f9f9] border border-[#ebebeb] text-xs text-[#171717] placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] focus:bg-white font-sans transition"
          />
        </div>
      </div>

      {/* Right: Modals triggers & Custom Window Controls */}
      <div className="flex items-center h-full space-x-1 shrink-0" data-tauri-drag-region>
        {/* Settings & Tools Modal Triggers */}
        <div className="flex items-center space-x-1.5 pr-2" data-no-drag>
          <button
            onMouseDown={(e) => e.stopPropagation()}
            onClick={openToolPolicyModal}
            className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] border border-[#ebebeb] text-[#666666] hover:text-[#171717] transition cursor-pointer"
            title="Матрица инструментов — доступ тулов по агентам"
          >
            <Wrench className="h-3.5 w-3.5" />
          </button>
          <button
            onMouseDown={(e) => e.stopPropagation()}
            onClick={openSettings}
            className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] border border-[#ebebeb] text-[#666666] hover:text-[#171717] transition cursor-pointer"
            title="Настройки LLM"
          >
            <Settings className="h-3.5 w-3.5" />
          </button>
        </div>

        {/* Vertical divider before window controls */}
        <div className="h-4 w-px bg-[#ebebeb] mx-1" />

        {/* Custom Frameless Window Controls */}
        <div className="flex items-center h-full" data-no-drag>
          {/* Minimize Button */}
          <button
            type="button"
            data-no-drag
            onMouseDown={(e) => e.stopPropagation()}
            onClick={handleMinimize}
            className="h-10 w-11 inline-flex items-center justify-center text-[#71717a] hover:text-[#18181b] hover:bg-[#f4f4f5] active:bg-[#e4e4e7] transition-colors cursor-pointer select-none"
            title="Свернуть"
            aria-label="Свернуть окно"
          >
            <Minus className="h-3.5 w-3.5" strokeWidth={1.75} />
          </button>

          {/* Maximize / Restore Button */}
          <button
            type="button"
            data-no-drag
            onMouseDown={(e) => e.stopPropagation()}
            onClick={handleToggleMaximize}
            className="h-10 w-11 inline-flex items-center justify-center text-[#71717a] hover:text-[#18181b] hover:bg-[#f4f4f5] active:bg-[#e4e4e7] transition-colors cursor-pointer select-none"
            title={isMaximized ? 'Восстановить' : 'Развернуть'}
            aria-label={isMaximized ? 'Восстановить окно' : 'Развернуть окно'}
          >
            {isMaximized ? (
              <Copy className="h-3 w-3 -scale-y-100" strokeWidth={1.75} />
            ) : (
              <Square className="h-3 w-3" strokeWidth={1.75} />
            )}
          </button>

          {/* Close Button: Signature Red on Hover */}
          <button
            type="button"
            data-no-drag
            onMouseDown={(e) => e.stopPropagation()}
            onClick={handleClose}
            className="h-10 w-11 inline-flex items-center justify-center text-[#71717a] hover:bg-rose-600 hover:text-white active:bg-rose-700 transition-colors cursor-pointer select-none"
            title="Закрыть"
            aria-label="Закрыть окно"
          >
            <X className="h-3.5 w-3.5" strokeWidth={2} />
          </button>
        </div>
      </div>
    </header>
  );
}

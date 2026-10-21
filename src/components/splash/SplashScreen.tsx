import { useEffect, useState, useRef } from 'react';
import { useProjectStore, isInvalidProjectPath } from '../../stores/projectStore';
import { FolderOpen, ArrowRight, FolderGit2, Loader2 } from 'lucide-react';
import Logo from '../common/Logo';
import { useTranslation } from '../../stores/languageStore';

export interface SplashScreenProps {
  isReady?: boolean;
  onFinish?: () => void;
}

export default function SplashScreen({
  isReady = true,
  onFinish,
}: SplashScreenProps) {
  const [stepIndex, setStepIndex] = useState<number>(0);
  const [progress, setProgress] = useState<number>(10);
  const [isFading, setIsFading] = useState<boolean>(false);
  const [isOpeningFolder, setIsOpeningFolder] = useState<boolean>(false);
  const fadeTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const activeProject = useProjectStore((s) => s.activeProject);
  const availableProjects = useProjectStore((s) => s.availableProjects);
  const selectProjectFolder = useProjectStore((s) => s.selectProjectFolder);
  const setActiveProjectByPath = useProjectStore((s) => s.setActiveProjectByPath);
  const { t } = useTranslation();

  // Fast bootloader sequence for initialization metrics (<300ms)
  useEffect(() => {
    const timer1 = setTimeout(() => {
      setStepIndex(1);
      setProgress(35);
    }, 60);

    const timer2 = setTimeout(() => {
      setStepIndex(2);
      setProgress(65);
    }, 140);

    const timer3 = setTimeout(() => {
      setStepIndex(3);
      setProgress(90);
    }, 220);

    const timer4 = setTimeout(() => {
      setStepIndex(4);
      setProgress(100);
    }, 300);

    return () => {
      clearTimeout(timer1);
      clearTimeout(timer2);
      clearTimeout(timer3);
      clearTimeout(timer4);
    };
  }, []);

  // Clean up any pending fade timeout on unmount
  useEffect(() => {
    return () => {
      if (fadeTimeoutRef.current) {
        clearTimeout(fadeTimeoutRef.current);
      }
    };
  }, []);

  // Resolve candidate project for 1-click continuation
  const candidateProject =
    activeProject && activeProject.rootPath && !isInvalidProjectPath(activeProject.rootPath)
      ? activeProject
      : availableProjects.find((p) => p && p.rootPath && !isInvalidProjectPath(p.rootPath)) || null;

  // Filter out the primary candidate from secondary recent projects
  const otherRecent = availableProjects.filter(
    (p) =>
      p &&
      p.rootPath &&
      !isInvalidProjectPath(p.rootPath) &&
      p.rootPath.toLowerCase() !== candidateProject?.rootPath.toLowerCase()
  );

  const finishTransition = () => {
    setIsFading(true);
    fadeTimeoutRef.current = setTimeout(() => {
      onFinish?.();
    }, 500); // 500ms smooth CSS opacity fade transition
  };

  const handleContinue = async () => {
    if (isOpeningFolder || !candidateProject?.rootPath) return;
    setIsOpeningFolder(true);
    try {
      await setActiveProjectByPath(candidateProject.rootPath);
      finishTransition();
    } catch (e) {
      console.error('Failed to continue project:', e);
      setIsOpeningFolder(false);
    }
  };

  const handleSelectFolder = async () => {
    if (isOpeningFolder) return;
    setIsOpeningFolder(true);
    try {
      const chosen = await selectProjectFolder();
      if (chosen && !isInvalidProjectPath(chosen)) {
        finishTransition();
      } else {
        setIsOpeningFolder(false);
      }
    } catch (e) {
      console.error('Failed to select project folder:', e);
      setIsOpeningFolder(false);
    }
  };

  const handleOpenRecent = async (rootPath: string) => {
    if (isOpeningFolder) return;
    setIsOpeningFolder(true);
    try {
      await setActiveProjectByPath(rootPath);
      finishTransition();
    } catch (e) {
      console.error('Failed to open recent project:', e);
      setIsOpeningFolder(false);
    }
  };

  // Keyboard shortcut: Press Enter to 1-click continue into candidate project
  useEffect(() => {
    if (stepIndex < 4 || !candidateProject || isOpeningFolder || isFading) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        void handleContinue();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [stepIndex, candidateProject, isOpeningFolder, isFading]);

  const showPicker = stepIndex >= 4 && isReady;

  return (
    <div
      data-tauri-drag-region
      style={{ backgroundColor: '#07090e', color: '#f0f6fc' }}
      className={`fixed inset-0 z-9999 flex items-center justify-center bg-[#07090e] text-[#f0f6fc] select-none transition-opacity duration-500 ease-out ${
        isFading ? 'opacity-0 pointer-events-none' : 'opacity-100'
      }`}
    >
      {!showPicker ? (
        /* Minimal bootloader progress */
        <div
          className={`flex flex-col items-center px-10 py-8 transition-all duration-500 ease-out ${
            isFading ? 'scale-[1.06] opacity-90' : 'scale-100 opacity-100'
          }`}
        >
          <Logo size={64} variant="badge" className="drop-shadow-sm select-none" />
          <div className="mt-5 h-[2px] w-44 bg-[#21262d] rounded-full overflow-hidden">
            <div
              className="h-full bg-[#f0f6fc] rounded-full transition-all duration-200 ease-out"
              style={{ width: `${progress}%` }}
            />
          </div>
          <span className="mt-2.5 font-mono text-[10px] text-[#8b949e] tabular-nums">
            {progress}%
          </span>
        </div>
      ) : (
        /* Tactical Obsidian Welcome & Project Picker Modal */
        <div
          data-tauri-drag-region
          className="w-full max-w-md mx-4 p-7 rounded-xl bg-white dark:bg-[#0d1117] border border-[#d0d7de] dark:border-[#21262d] shadow-2xl flex flex-col items-center text-center animate-in fade-in zoom-in-95 duration-200"
        >
          {/* Logo Badge */}
          <div className="mb-3.5">
            <Logo size={52} variant="badge" className="drop-shadow-md select-none" />
          </div>

          {/* Heading & Subtitle */}
          <h1 className="text-xl font-bold tracking-tight text-[#171717] dark:text-[#f0f6fc]">
            {t('splash.welcome')}
          </h1>
          <p className="mt-1 text-xs text-[#57606a] dark:text-[#8b949e] max-w-sm leading-relaxed">
            {t('splash.selectProject')}
          </p>

          {/* Action Area: Continue Previous or Open Folder */}
          <div className="w-full mt-5.5 flex flex-col gap-2.5">
            {/* 1-Click Continue Button if previously opened project exists */}
            {candidateProject && (
              <button
                type="button"
                onClick={handleContinue}
                disabled={isOpeningFolder}
                className="w-full py-3 px-4 rounded-lg bg-[#238636] hover:bg-[#2ea043] active:bg-[#238636]/90 text-white font-medium text-sm flex items-center justify-between transition-all duration-150 shadow-md cursor-pointer disabled:opacity-60 group border border-[#2ea043]/40"
              >
                <div className="flex items-center gap-2.5 min-w-0">
                  <FolderGit2 className="w-5 h-5 shrink-0 text-white/90 group-hover:scale-105 transition-transform" />
                  <div className="flex flex-col text-left min-w-0">
                    <span className="text-xs font-semibold truncate flex items-center gap-1.5">
                      <span>{t('splash.continueIn')}</span>
                      <span className="text-white underline decoration-white/30 underline-offset-2">
                        {candidateProject.name}
                      </span>
                    </span>
                    <span className="text-[10px] font-mono text-white/80 truncate max-w-[270px]">
                      {candidateProject.rootPath}
                    </span>
                  </div>
                </div>
                <div className="flex items-center gap-1.5 shrink-0 ml-2">
                  <span className="text-[10px] font-mono bg-black/20 text-white/90 px-1.5 py-0.5 rounded text-xs">
                    Enter ↵
                  </span>
                  <ArrowRight className="w-4 h-4 text-white/90 transition-transform group-hover:translate-x-0.5" />
                </div>
              </button>
            )}

            {/* Select Project Folder Button */}
            <button
              type="button"
              onClick={handleSelectFolder}
              disabled={isOpeningFolder}
              className={`w-full py-3 px-4 rounded-lg bg-[#f6f8fa] dark:bg-[#161b22] hover:bg-[#eaeef2] dark:hover:bg-[#21262d] active:bg-[#d0d7de] dark:active:bg-[#30363d] border border-[#d0d7de] dark:border-[#30363d] hover:border-[#58a6ff]/60 text-[#24292f] dark:text-[#f0f6fc] font-medium text-sm flex items-center justify-center gap-2.5 transition-all duration-150 shadow-sm cursor-pointer disabled:opacity-60 ${
                !candidateProject ? 'py-3.5 shadow-md border-[#58a6ff]/40 bg-[#58a6ff]/5' : ''
              }`}
            >
              {isOpeningFolder ? (
                <Loader2 className="w-4 h-4 text-[#58a6ff] animate-spin" />
              ) : (
                <FolderOpen className="w-4 h-4 text-[#58a6ff]" />
              )}
              <span>{t('splash.openFolder')}</span>
            </button>
          </div>

          {/* Recent Projects List */}
          {otherRecent.length > 0 && (
            <div className="w-full mt-5 flex flex-col gap-2 text-left">
              <div className="flex items-center gap-2 mb-0.5">
                <div className="h-[1px] flex-1 bg-[#e1e4e8] dark:bg-[#21262d]" />
                <span className="text-[10px] font-mono uppercase tracking-wider text-[#57606a] dark:text-[#8b949e]">
                  {t('splash.recentProjects')}
                </span>
                <div className="h-[1px] flex-1 bg-[#e1e4e8] dark:bg-[#21262d]" />
              </div>

              <div className="max-h-44 overflow-y-auto flex flex-col gap-1.5 pr-0.5">
                {otherRecent.map((p) => (
                  <button
                    key={p.id || p.rootPath}
                    type="button"
                    onClick={() => handleOpenRecent(p.rootPath)}
                    disabled={isOpeningFolder}
                    className="group flex items-center justify-between p-2 rounded-lg bg-[#f6f8fa] dark:bg-[#07090e] hover:bg-[#eaeef2] dark:hover:bg-[#161b22] border border-[#e1e4e8] dark:border-[#21262d] hover:border-[#58a6ff]/40 text-left transition-all cursor-pointer disabled:opacity-60"
                  >
                    <div className="flex items-center gap-2.5 min-w-0">
                      <FolderGit2 className="w-4 h-4 text-[#57606a] dark:text-[#8b949e] group-hover:text-[#58a6ff] shrink-0 transition-colors" />
                      <div className="flex flex-col min-w-0">
                        <span className="text-xs font-medium text-[#171717] dark:text-[#f0f6fc] truncate">
                          {p.name}
                        </span>
                        <span className="text-[10px] font-mono text-[#57606a] dark:text-[#8b949e] truncate max-w-[270px]">
                          {p.rootPath}
                        </span>
                      </div>
                    </div>
                    <ArrowRight className="w-3.5 h-3.5 text-[#57606a] dark:text-[#484f58] group-hover:text-[#58a6ff] shrink-0 ml-2 transition-transform group-hover:translate-x-0.5" />
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

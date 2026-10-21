import { useEffect, useState, useRef } from 'react';
import { useProjectStore } from '../../stores/projectStore';
import { FolderOpen, ArrowRight, FolderGit2, Loader2 } from 'lucide-react';
import Logo from '../common/Logo';

export interface SplashScreenProps {
  isReady?: boolean;
  minDisplayTimeMs?: number;
  onFinish?: () => void;
}

export default function SplashScreen({
  isReady = true,
  // Минимальная задержка для плавного отображения без искусственного удержания
  minDisplayTimeMs = 350,
  onFinish,
}: SplashScreenProps) {
  const [stepIndex, setStepIndex] = useState<number>(0);
  const [progress, setProgress] = useState<number>(10);
  const [isFading, setIsFading] = useState<boolean>(false);
  const [isOpeningFolder, setIsOpeningFolder] = useState<boolean>(false);
  const startTimeRef = useRef<number>(Date.now());

  const hasActiveProject = useProjectStore((s) => s.hasActiveProject);
  const availableProjects = useProjectStore((s) => s.availableProjects);
  const selectProjectFolder = useProjectStore((s) => s.selectProjectFolder);
  const setActiveProjectByPath = useProjectStore((s) => s.setActiveProjectByPath);

  // Fast boot sequence without artificial sluggishness
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

  // Trigger fade out once all steps completed, data is ready, and an active project is present
  useEffect(() => {
    if (stepIndex < 4 || !isReady || !hasActiveProject) return;

    const elapsed = Date.now() - startTimeRef.current;
    const remaining = Math.max(0, minDisplayTimeMs - elapsed);
    let finishTimer: ReturnType<typeof setTimeout> | undefined;

    const fadeTimer = setTimeout(() => {
      setIsFading(true);
      finishTimer = setTimeout(() => {
        onFinish?.();
      }, 500); // 500ms fade transition
    }, remaining);

    return () => {
      clearTimeout(fadeTimer);
      if (finishTimer) clearTimeout(finishTimer);
    };
  }, [stepIndex, isReady, hasActiveProject, minDisplayTimeMs, onFinish]);

  const handleSelectFolder = async () => {
    if (isOpeningFolder) return;
    setIsOpeningFolder(true);
    try {
      await selectProjectFolder();
    } finally {
      setIsOpeningFolder(false);
    }
  };

  const handleOpenRecent = async (rootPath: string) => {
    if (isOpeningFolder) return;
    setIsOpeningFolder(true);
    try {
      await setActiveProjectByPath(rootPath);
    } finally {
      setIsOpeningFolder(false);
    }
  };

  const showWelcomePicker = stepIndex >= 4 && !hasActiveProject;

  return (
    <div
      data-tauri-drag-region
      className={`fixed inset-0 z-9999 flex items-center justify-center bg-[#fafafa] dark:bg-[#07090e] select-none transition-opacity duration-500 ease-out ${
        isFading ? 'opacity-0 pointer-events-none' : 'opacity-100'
      }`}
    >
      {!showWelcomePicker ? (
        /* Minimal bootloader progress */
        <div
          className={`flex flex-col items-center px-10 py-8 transition-all duration-500 ease-out ${
            isFading ? 'scale-[1.06] opacity-90' : 'scale-100 opacity-100'
          }`}
        >
          <Logo size={64} variant="badge" className="drop-shadow-sm select-none" />
          <div className="mt-5 h-[2px] w-44 bg-[#ebebeb] dark:bg-[#21262d] rounded-full overflow-hidden">
            <div
              className="h-full bg-[#171717] dark:bg-[#f0f6fc] rounded-full transition-all duration-200 ease-out"
              style={{ width: `${progress}%` }}
            />
          </div>
          <span className="mt-2.5 font-mono text-[10px] text-[#8f8f8f] dark:text-[#8b949e] tabular-nums">
            {progress}%
          </span>
        </div>
      ) : (
        /* Tactical Obsidian/Geist Welcome & Project Picker */
        <div
          data-tauri-drag-region
          className="w-full max-w-md mx-4 p-8 rounded-xl bg-white dark:bg-[#0d1117] border border-[#d0d7de] dark:border-[#21262d] shadow-2xl flex flex-col items-center text-center animate-in fade-in zoom-in-95 duration-300"
        >
          {/* Logo Badge */}
          <div className="mb-4">
            <Logo size={56} variant="badge" className="drop-shadow-md select-none" />
          </div>

          {/* Heading & Subtitle */}
          <h1 className="text-xl font-semibold tracking-tight text-[#171717] dark:text-[#f0f6fc]">
            Добро пожаловать в CanbanVoimaxHarness
          </h1>
          <p className="mt-2 text-xs text-[#57606a] dark:text-[#8b949e] max-w-sm leading-relaxed">
            Выберите рабочую папку с проектом для начала работы
          </p>

          {/* Large Folder Selection Button */}
          <div className="w-full mt-6">
            <button
              type="button"
              onClick={handleSelectFolder}
              disabled={isOpeningFolder}
              className="w-full py-3.5 px-5 rounded-lg bg-[#f6f8fa] dark:bg-[#161b22] hover:bg-[#eaeef2] dark:hover:bg-[#21262d] active:bg-[#d0d7de] dark:active:bg-[#30363d] border border-[#d0d7de] dark:border-[#30363d] hover:border-[#58a6ff]/60 text-[#24292f] dark:text-[#f0f6fc] font-medium text-sm flex items-center justify-center gap-3 transition-all duration-150 shadow-sm hover:shadow-cyan-500/10 cursor-pointer disabled:opacity-60"
            >
              {isOpeningFolder ? (
                <Loader2 className="w-5 h-5 text-[#58a6ff] animate-spin" />
              ) : (
                <FolderOpen className="w-5 h-5 text-[#58a6ff]" />
              )}
              <span>📂 Выбрать папку проекта</span>
            </button>
          </div>

          {/* Recent Projects List if available */}
          {availableProjects.length > 0 && (
            <div className="w-full mt-6 flex flex-col gap-2 text-left">
              <div className="flex items-center gap-2 mb-1">
                <div className="h-[1px] flex-1 bg-[#e1e4e8] dark:bg-[#21262d]" />
                <span className="text-[11px] font-mono uppercase tracking-wider text-[#57606a] dark:text-[#8b949e]">
                  Недавние проекты
                </span>
                <div className="h-[1px] flex-1 bg-[#e1e4e8] dark:bg-[#21262d]" />
              </div>

              <div className="max-h-52 overflow-y-auto flex flex-col gap-1.5 pr-1">
                {availableProjects.map((p) => (
                  <button
                    key={p.id || p.rootPath}
                    type="button"
                    onClick={() => handleOpenRecent(p.rootPath)}
                    disabled={isOpeningFolder}
                    className="group flex items-center justify-between p-2.5 rounded-lg bg-[#f6f8fa] dark:bg-[#07090e] hover:bg-[#eaeef2] dark:hover:bg-[#161b22] border border-[#e1e4e8] dark:border-[#21262d] hover:border-[#58a6ff]/40 text-left transition-all cursor-pointer disabled:opacity-60"
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

import { useEffect, useState, useRef } from 'react';

export interface SplashScreenProps {
  isReady?: boolean;
  minDisplayTimeMs?: number;
  onFinish?: () => void;
}

export default function SplashScreen({
  isReady = true,
  minDisplayTimeMs = 900,
  onFinish,
}: SplashScreenProps) {
  const [stepIndex, setStepIndex] = useState<number>(0);
  const [progress, setProgress] = useState<number>(10);
  const [isFading, setIsFading] = useState<boolean>(false);
  const [cursorVisible, setCursorVisible] = useState<boolean>(true);
  const startTimeRef = useRef<number>(Date.now());

  // Cursor blink effect
  useEffect(() => {
    const cursorInterval = setInterval(() => {
      setCursorVisible((v) => !v);
    }, 450);
    return () => clearInterval(cursorInterval);
  }, []);

  // Staggered boot sequence
  useEffect(() => {
    const timer1 = setTimeout(() => {
      setStepIndex(1);
      setProgress(35);
    }, 180);

    const timer2 = setTimeout(() => {
      setStepIndex(2);
      setProgress(65);
    }, 380);

    const timer3 = setTimeout(() => {
      setStepIndex(3);
      setProgress(90);
    }, 580);

    const timer4 = setTimeout(() => {
      setStepIndex(4);
      setProgress(100);
    }, 780);

    return () => {
      clearTimeout(timer1);
      clearTimeout(timer2);
      clearTimeout(timer3);
      clearTimeout(timer4);
    };
  }, []);

  // Trigger fade out once all steps completed and data is ready
  useEffect(() => {
    if (stepIndex < 4 || !isReady) return;

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
  }, [stepIndex, isReady, minDisplayTimeMs, onFinish]);

  return (
    <div
      className={`fixed inset-0 z-9999 flex items-center justify-center bg-[#0d0d0d] select-none transition-opacity duration-500 ease-out ${
        isFading ? 'opacity-0 pointer-events-none' : 'opacity-100'
      }`}
    >
      {/* Typeset terminal on obsidian / paper surface panel */}
      <div className="w-[90vw] max-w-[460px] bg-[#171717] border border-[#27272a] rounded-[6px] shadow-2xl overflow-hidden flex flex-col">
        {/* Terminal Header Bar */}
        <div className="h-8 px-3.5 border-b border-[#27272a] bg-[#121212] flex items-center justify-between text-[11px] font-mono text-[#8f8f8f]">
          <div className="flex items-center space-x-2">
            <span className="text-[#ffffff]">▲</span>
            <span className="tracking-widest uppercase">KERNEL_BOOT</span>
          </div>
          <div className="flex items-center space-x-2 text-[10px] text-[#52525b]">
            <span>TTY_VOIMAX</span>
            <span className="inline-block w-1.5 h-1.5 rounded-full bg-[#297a3a]" />
          </div>
        </div>

        {/* CLI Terminal Body */}
        <div className="p-4 sm:p-5 font-mono text-xs text-[#fafafa] space-y-2.5 min-h-[160px] flex flex-col justify-between">
          <div className="space-y-2">
            {/* Header: Delta + Title */}
            <div className="flex items-center space-x-2 text-[#ffffff] font-semibold text-[13px] tracking-tight">
              <span className="text-[#ffffff]">▲</span>
              <span>CANBAN_VOIMAX_HARNESS v0.1.0</span>
            </div>

            {/* Sequence Line 1 */}
            {stepIndex >= 1 && (
              <div className="flex items-center space-x-2 text-[#d4d4d8] animate-in fade-in duration-150">
                <span className="text-[#297a3a] font-bold">✓</span>
                <span className="tracking-wide">INITIALIZING_CORE_KERNELS</span>
              </div>
            )}

            {/* Sequence Line 2 */}
            {stepIndex >= 2 && (
              <div className="flex items-center space-x-2 text-[#d4d4d8] animate-in fade-in duration-150">
                <span className="text-[#297a3a] font-bold">✓</span>
                <span className="tracking-wide">HYDRATING_WORKSPACE_BOARDS</span>
              </div>
            )}

            {/* Sequence Line 3 */}
            {stepIndex >= 3 && (
              <div className="flex items-center space-x-2 text-[#d4d4d8] animate-in fade-in duration-150">
                <span className="text-[#297a3a] font-bold">✓</span>
                <span className="tracking-wide">DISCOVERING_AGENT_WORKTREES</span>
              </div>
            )}

            {/* Final Ready State */}
            {stepIndex >= 4 && (
              <div className="flex items-center space-x-2 text-[#297a3a] font-semibold tracking-wider animate-in fade-in duration-150 pt-1">
                <span>[SYSTEM READY]</span>
                <span className="text-[#8f8f8f] font-normal text-[11px]">
                  (0% color slop · hairline invariant)
                </span>
              </div>
            )}

            {/* Terminal Cursor */}
            {stepIndex < 4 && (
              <div className="text-[#8f8f8f] text-xs">
                <span>&gt; loading modules</span>
                <span
                  className={`inline-block ml-1 transition-opacity ${
                    cursorVisible ? 'opacity-100' : 'opacity-0'
                  }`}
                >
                  ▋
                </span>
              </div>
            )}
          </div>

          {/* Footer Metadata */}
          <div className="pt-2 text-[10px] text-[#71717a] flex items-center justify-between border-t border-[#27272a]/60">
            <span>MEM: ~40MB OK</span>
            <span>{progress}%</span>
          </div>
        </div>

        {/* Elegant Hairline Progress Indicator */}
        <div className="h-[2px] w-full bg-[#27272a] relative overflow-hidden">
          <div
            className="h-full bg-white transition-all duration-200 ease-out"
            style={{ width: `${progress}%` }}
          />
        </div>
      </div>
    </div>
  );
}

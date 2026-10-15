import { useEffect, useState, useRef } from 'react';

export interface SplashScreenProps {
  isReady?: boolean;
  minDisplayTimeMs?: number;
  onFinish?: () => void;
}

export default function SplashScreen({
  isReady = true,
  // Искусственно держим сплэш подольше — красивый вход в приложение.
  minDisplayTimeMs = 2400,
  onFinish,
}: SplashScreenProps) {
  const [stepIndex, setStepIndex] = useState<number>(0);
  const [progress, setProgress] = useState<number>(10);
  const [isFading, setIsFading] = useState<boolean>(false);
  const startTimeRef = useRef<number>(Date.now());

  // Staggered boot sequence (timing only — no phrases rendered)
  useEffect(() => {
    const timer1 = setTimeout(() => {
      setStepIndex(1);
      setProgress(35);
    }, 400);

    const timer2 = setTimeout(() => {
      setStepIndex(2);
      setProgress(65);
    }, 900);

    const timer3 = setTimeout(() => {
      setStepIndex(3);
      setProgress(90);
    }, 1400);

    const timer4 = setTimeout(() => {
      setStepIndex(4);
      setProgress(100);
    }, 1900);

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
      data-tauri-drag-region
      className={`fixed inset-0 z-9999 flex items-center justify-center bg-[#fafafa] select-none transition-opacity duration-500 ease-out ${
        isFading ? 'opacity-0 pointer-events-none' : 'opacity-100'
      }`}
    >
      {/* Minimal loader: logo mark + hairline progress. No boot phrases.
          On finish it gracefully expands into the app (scale up + fade). */}
      <div
        className={`flex flex-col items-center px-10 py-8 transition-all duration-500 ease-out ${
          isFading ? 'scale-[1.06] opacity-90' : 'scale-100 opacity-100'
        }`}
      >
        <svg
          viewBox="0 0 32 32"
          className="w-16 h-16 drop-shadow-sm select-none"
          fill="none"
          xmlns="http://www.w3.org/2000/svg"
          aria-hidden="true"
        >
          {/* Obsidian Badge Surface with 6px Geist radius */}
          <rect width="32" height="32" rx="6" fill="#171717" />
          <rect x="0.5" y="0.5" width="31" height="31" rx="5.5" stroke="#27272a" strokeWidth="1" />

          {/* Terminal Delta Glyph (▲) */}
          <polygon points="16,6.5 21.5,14.5 10.5,14.5" fill="#ffffff" />

          {/* Geometric Kanban Columns */}
          {/* Left Column: Backlog (Hairline fill opacity) */}
          <rect x="7" y="17.5" width="4.5" height="8" rx="1.2" fill="#ffffff" fillOpacity="0.38" />

          {/* Middle Column: Active Core Execution (100% white) */}
          <rect x="13.75" y="17.5" width="4.5" height="8" rx="1.2" fill="#ffffff" />

          {/* Right Column: Shipped / Done (75% opacity) */}
          <rect x="20.5" y="17.5" width="4.5" height="8" rx="1.2" fill="#ffffff" fillOpacity="0.72" />
        </svg>
        <div className="mt-5 h-[2px] w-44 bg-[#ebebeb] rounded-full overflow-hidden">
          <div
            className="h-full bg-[#171717] rounded-full transition-all duration-200 ease-out"
            style={{ width: `${progress}%` }}
          />
        </div>
        <span className="mt-2.5 font-mono text-[10px] text-[#8f8f8f] tabular-nums">
          {progress}%
        </span>
      </div>
    </div>
  );
}

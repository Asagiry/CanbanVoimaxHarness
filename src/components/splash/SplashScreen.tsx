import { useState, useEffect, useRef } from 'react';
import Logo from '../common/Logo';

interface SplashScreenProps {
  /** Flag indicating whether background data initialization has completed */
  isReady: boolean;
  /** Callback fired once exit fade-out transition has fully completed */
  onFinish?: () => void;
}

export default function SplashScreen({ isReady, onFinish }: SplashScreenProps) {
  const [progress, setProgress] = useState(12);
  const [statusText, setStatusText] = useState('Инициализация харнесса...');
  const [isExiting, setIsExiting] = useState(false);
  const [isDone, setIsDone] = useState(false);

  // Preserve latest onFinish reference to prevent unnecessary effect re-triggers
  const onFinishRef = useRef(onFinish);
  useEffect(() => {
    onFinishRef.current = onFinish;
  }, [onFinish]);

  useEffect(() => {
    // Simulated smooth progress ramp-up while loading
    const timer1 = setTimeout(() => {
      setProgress(35);
      setStatusText('Синхронизация Git-ворктри и контекста...');
    }, 280);

    const timer2 = setTimeout(() => {
      setProgress(68);
      setStatusText('Загрузка досок и задач...');
    }, 600);

    const timer3 = setTimeout(() => {
      setProgress(88);
      setStatusText('Подготовка интерфейса управления...');
    }, 900);

    return () => {
      clearTimeout(timer1);
      clearTimeout(timer2);
      clearTimeout(timer3);
    };
  }, []);

  useEffect(() => {
    if (!isReady) return;

    let exitTimer: ReturnType<typeof setTimeout> | undefined;
    let removeTimer: ReturnType<typeof setTimeout> | undefined;

    // Minimum display elapsed + data ready -> reach 100% and initiate fade-out
    const finishTimer = setTimeout(() => {
      setProgress(100);
      setStatusText('Харнесс готов к работе');

      // Trigger exit fade-out
      exitTimer = setTimeout(() => {
        setIsExiting(true);

        // Remove from DOM after transition
        removeTimer = setTimeout(() => {
          setIsDone(true);
          onFinishRef.current?.();
        }, 650);
      }, 250);
    }, 1000);

    return () => {
      clearTimeout(finishTimer);
      if (exitTimer) clearTimeout(exitTimer);
      if (removeTimer) clearTimeout(removeTimer);
    };
  }, [isReady]);

  if (isDone) return null;

  return (
    <div
      data-tauri-drag-region
      className={`fixed inset-0 z-50 flex flex-col items-center justify-center bg-[#09090b] text-white select-none transition-all duration-600 ease-out ${
        isExiting
          ? 'opacity-0 scale-[1.02] pointer-events-none filter blur-xs'
          : 'opacity-100 scale-100'
      }`}
    >
      {/* Ambient Cybernetic Radial Aura */}
      <div
        className="absolute inset-0 pointer-events-none"
        style={{
          background:
            'radial-gradient(circle at 50% 45%, rgba(139, 92, 246, 0.14) 0%, rgba(6, 182, 212, 0.08) 28%, rgba(16, 185, 129, 0.03) 55%, transparent 72%)',
        }}
      />

      {/* Subtle Micro-Grid Background */}
      <div
        className="absolute inset-0 opacity-[0.035] pointer-events-none"
        style={{
          backgroundImage:
            'linear-gradient(#fff 1px, transparent 1px), linear-gradient(90deg, #fff 1px, transparent 1px)',
          backgroundSize: '32px 32px',
        }}
      />

      {/* Central Content Block */}
      <div className="relative z-10 flex flex-col items-center max-w-sm px-6 text-center" data-tauri-drag-region>
        {/* Glowing Central Logo with Pulsing Ring */}
        <div className="relative mb-6 flex items-center justify-center group" data-no-drag>
          {/* Outer Breathing Glow Ring */}
          <div className="absolute -inset-4 rounded-full bg-gradient-to-tr from-violet-600/30 via-cyan-500/30 to-amber-500/20 blur-xl animate-pulse" />
          <div className="absolute -inset-1 rounded-3xl bg-white/5 border border-white/10 backdrop-blur-xs" />

          {/* Vector Quantum Logo (80px) */}
          <div className="relative p-3.5">
            <Logo size={80} animated glow className="drop-shadow-[0_0_24px_rgba(6,182,212,0.6)]" />
          </div>
        </div>

        {/* Cinematic Title & Monospace Badge */}
        <div className="mb-8" data-tauri-drag-region>
          <h1 className="text-xl font-bold tracking-tight text-white font-sans flex items-center justify-center gap-1.5 drop-shadow-md">
            <span>CANBAN</span>
            <span className="text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 via-violet-400 to-amber-400 font-extrabold">
              VOIMAX
            </span>
            <span>HARNESS</span>
          </h1>
          <p className="text-[10px] font-mono text-zinc-400 tracking-widest uppercase mt-1.5 flex items-center justify-center gap-2">
            <span className="inline-block w-1.5 h-1.5 rounded-full bg-emerald-400 animate-ping" />
            <span>AGENTIC IDE CONTROL PLANE</span>
            <span className="text-zinc-600">•</span>
            <span className="text-zinc-500">v0.1.0</span>
          </p>
        </div>

        {/* Minimalist Glowing Progress Bar */}
        <div className="w-64 max-w-xs flex flex-col items-center gap-2.5" data-no-drag>
          <div className="w-full h-[3px] rounded-full bg-zinc-800/80 overflow-hidden relative border border-white/5 backdrop-blur-xs shadow-inner">
            <div
              className="h-full rounded-full bg-gradient-to-r from-cyan-400 via-violet-500 to-emerald-400 transition-all duration-300 ease-out shadow-[0_0_12px_rgba(6,182,212,0.8)]"
              style={{ width: `${progress}%` }}
            />
          </div>

          {/* Status Label & Percent Readout */}
          <div className="w-full flex items-center justify-between text-[11px] font-mono text-zinc-400 px-0.5">
            <span className="tracking-tight transition-opacity duration-200">
              {statusText}
            </span>
            <span className="tabular-nums font-semibold text-zinc-300 ml-2">
              {Math.round(progress)}%
            </span>
          </div>
        </div>
      </div>

      {/* Bottom Subtle Corner Telemetry Accent */}
      <div className="absolute bottom-4 text-center font-mono text-[9px] text-zinc-600 tracking-wider pointer-events-none select-none uppercase">
        TAURI V2 • REACT 19 • OBSIDIAN QUANTUM RUNTIME
      </div>
    </div>
  );
}

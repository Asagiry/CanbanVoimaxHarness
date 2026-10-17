import { useState, useEffect, useRef } from 'react';
import { ChevronDown, ChevronRight, Brain, Clock, Copy, Check } from 'lucide-react';
import { DrawerActivityItem, useDrawerStore } from '../../stores/drawerStore';

export interface ThinkingCardProps {
  thought: DrawerActivityItem;
  isTaskRunning: boolean;
  taskId: string;
  isLive?: boolean;
}

/**
 * Collapsible Live Thinking Card with Streaming Typewriter Effect
 * Smooth progressive character-by-character rendering for real-time thinking.
 * Instant 100% rendering without typing or cursor for historical/completed thoughts.
 */
export default function ThinkingCard({
  thought,
  isTaskRunning,
  taskId,
  isLive,
}: ThinkingCardProps) {
  const [isExpanded, setIsExpanded] = useState(true);
  const [copied, setCopied] = useState(false);
  const thinkingStart = useDrawerStore((s) => s.thinkingStartTime[taskId] ?? null);

  const fullText = thought?.content ?? '';

  // Determine if this thought is finished / historical:
  // 1. Task is not running (!isTaskRunning)
  // 2. Explicitly marked not live (isLive === false)
  // 3. Or card already has duration/finished and is not explicitly marked live
  const isFinished =
    !isTaskRunning ||
    isLive === false ||
    (isLive === undefined && thought?.durationMs !== undefined && thought?.durationMs > 0);

  // Typewriter state:
  // If finished/historical, show 100% immediately (displayedLength = fullText.length)
  // If live and actively streaming in real-time, initialize smoothly
  const [displayedLength, setDisplayedLength] = useState<number>(() => {
    if (isFinished) return fullText.length;
    return fullText.length > 50 ? Math.max(0, fullText.length - 20) : 0;
  });

  const fullTextRef = useRef(fullText);
  fullTextRef.current = fullText;

  // Live timer tick ONLY during active streaming
  const [, setTimerTick] = useState(0);
  useEffect(() => {
    if (isFinished || thinkingStart == null) return;
    const interval = setInterval(() => {
      setTimerTick((t) => (t + 1) % 1000);
    }, 100);
    return () => clearInterval(interval);
  }, [isFinished, thinkingStart]);

  // Streaming typewriter animation loop (ONLY for active streaming thought)
  useEffect(() => {
    if (isFinished) {
      setDisplayedLength(fullText.length);
      return;
    }

    if (displayedLength >= fullText.length) {
      return;
    }

    let animationFrameId: number;
    let lastTime = performance.now();

    const tick = (now: number) => {
      const elapsed = now - lastTime;
      // Step roughly every 16ms (60 FPS)
      if (elapsed >= 16) {
        lastTime = now;
        setDisplayedLength((prev) => {
          const currentTarget = fullTextRef.current.length;
          const remaining = currentTarget - prev;
          if (remaining <= 0) return currentTarget;

          // Smooth typewriter step: 1-3 chars per frame
          let step = 1;
          if (remaining > 30) {
            step = 3;
          } else if (remaining > 10) {
            step = 2;
          } else {
            step = 1;
          }

          return Math.min(prev + step, currentTarget);
        });
      }
      animationFrameId = requestAnimationFrame(tick);
    };

    animationFrameId = requestAnimationFrame(tick);
    return () => {
      if (animationFrameId) {
        cancelAnimationFrame(animationFrameId);
      }
    };
  }, [fullText.length, isFinished, displayedLength]);

  // Timer calculation: live or final duration
  let timerText = '';
  if (!isFinished && thinkingStart != null) {
    timerText = `${((Date.now() - thinkingStart) / 1000).toFixed(1)}с`;
  } else if (thought?.durationMs) {
    timerText = `${(thought.durationMs / 1000).toFixed(thought.durationMs >= 10000 ? 0 : 1)}с`;
  }

  const handleCopy = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!fullText) return;
    navigator.clipboard.writeText(fullText);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const visibleText = (isFinished ? fullText : fullText.slice(0, displayedLength)).trimEnd();
  const isTyping = !isFinished && displayedLength < fullText.length;

  return (
    <div className="rounded-md border border-[#ebebeb] bg-[#fcfcfc] overflow-hidden shadow-xs transition duration-150 animate-in fade-in duration-300 slide-in-from-top-1">
      {/* Header bar */}
      <div
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center justify-between px-3 py-2 bg-[#f8f8f9] hover:bg-[#f4f4f5] cursor-pointer transition select-none border-l-2 border-l-[#171717]"
      >
        <div className="flex items-center space-x-2">
          {isExpanded ? (
            <ChevronDown className="h-3.5 w-3.5 text-[#666666]" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-[#666666]" />
          )}
          <Brain className="h-3.5 w-3.5 text-[#171717]" />
          <span className="font-mono text-xs font-semibold text-[#171717]">
            &lt;thinking&gt;
          </span>
          {!isFinished && isTaskRunning && (
            <span className="flex h-1.5 w-1.5 relative">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[#15803d] opacity-75" />
              <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-[#15803d]" />
            </span>
          )}
        </div>

        <div className="flex items-center space-x-2.5 text-[10px] font-mono text-[#666666]">
          {timerText && (
            <span className="flex items-center space-x-1 tabular-nums">
              <Clock className="h-3 w-3" />
              <span>{timerText}</span>
            </span>
          )}

          {thought?.timestamp && (
            <span className="tabular-nums">{thought.timestamp}</span>
          )}

          <button
            type="button"
            onClick={handleCopy}
            className="p-1 rounded text-[#8f8f8f] hover:text-[#171717] hover:bg-white transition cursor-pointer"
            title="Скопировать размышления"
          >
            {copied ? (
              <Check className="h-3 w-3 text-emerald-600" />
            ) : (
              <Copy className="h-3 w-3" />
            )}
          </button>
        </div>
      </div>

      {/* Expanded body */}
      {isExpanded && (
        <div className="px-3 pt-2.5 pb-2 border-t border-[#ebebeb] bg-[#ffffff] font-mono text-[13px] leading-relaxed text-[#333333] whitespace-pre-wrap select-text max-h-96 overflow-y-auto">
          {visibleText}
          {isTyping && (
            <span
              className="inline-block w-1.5 h-3.5 bg-[#171717] animate-pulse ml-0.5 align-middle select-none"
              title="Думает..."
            />
          )}
        </div>
      )}
    </div>
  );
}

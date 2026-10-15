import { useState, useEffect, useRef } from 'react';
import { ChevronDown, ChevronRight, Brain, Clock, Copy, Check } from 'lucide-react';
import { DrawerActivityItem, useDrawerStore } from '../../stores/drawerStore';

export interface ThinkingCardProps {
  thought: DrawerActivityItem;
  isTaskRunning: boolean;
  taskId: string;
}

/**
 * Collapsible Live Thinking Card with Streaming Typewriter Effect
 * Smooth progressive character-by-character rendering for real-time thinking.
 */
export default function ThinkingCard({
  thought,
  isTaskRunning,
  taskId,
}: ThinkingCardProps) {
  const [isExpanded, setIsExpanded] = useState(true);
  const [copied, setCopied] = useState(false);
  const thinkingStart = useDrawerStore((s) => s.thinkingStartTime[taskId] ?? null);

  const fullText = thought?.content ?? '';

  // Typewriter state: characters to display
  // For historical / non-running thoughts, initialize directly to full length
  const [displayedLength, setDisplayedLength] = useState<number>(() => {
    if (!isTaskRunning) return fullText.length;
    // When running, if initial content is short, start from 0; if already long, show almost all and type the rest
    return fullText.length > 50 ? Math.max(0, fullText.length - 25) : 0;
  });

  const fullTextRef = useRef(fullText);
  fullTextRef.current = fullText;

  // Live timer tick during streaming
  const [, setTimerTick] = useState(0);
  useEffect(() => {
    if (!isTaskRunning || thinkingStart == null) return;
    const interval = setInterval(() => {
      setTimerTick((t) => (t + 1) % 1000);
    }, 100);
    return () => clearInterval(interval);
  }, [isTaskRunning, thinkingStart]);

  // Streaming typewriter animation loop
  useEffect(() => {
    if (!isTaskRunning) {
      // Once task is no longer running, reveal all content immediately
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
      // Step roughly every 16-20ms (60 FPS)
      if (elapsed >= 16) {
        lastTime = now;
        setDisplayedLength((prev) => {
          const currentTarget = fullTextRef.current.length;
          const remaining = currentTarget - prev;
          if (remaining <= 0) return currentTarget;

          // Adaptive typewriter step speed:
          // Small delta: 1-2 chars per tick for tactile human typing feel
          // Large bursts: catch up smoothly without falling behind
          let step = 1;
          if (remaining > 200) {
            step = Math.ceil(remaining / 8);
          } else if (remaining > 60) {
            step = Math.ceil(remaining / 4);
          } else if (remaining > 15) {
            step = 2;
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
  }, [fullText.length, isTaskRunning, displayedLength]);

  // Live timer calculation
  let timerText = '0с';
  if (isTaskRunning && thinkingStart != null) {
    timerText = `${((Date.now() - thinkingStart) / 1000).toFixed(1)}с`;
  } else if (thought?.durationMs) {
    timerText = `${Math.round(thought.durationMs / 1000)}с`;
  }

  const handleCopy = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!fullText) return;
    navigator.clipboard.writeText(fullText);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const visibleText = fullText.slice(0, displayedLength);
  const isTyping = isTaskRunning && displayedLength < fullText.length;

  return (
    <div className="rounded-md border border-[#ebebeb] bg-[#fcfcfc] overflow-hidden shadow-xs transition duration-150">
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
          {isTaskRunning && (
            <span className="flex h-1.5 w-1.5 relative">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[#15803d] opacity-75" />
              <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-[#15803d]" />
            </span>
          )}
        </div>

        <div className="flex items-center space-x-2.5 text-[10px] font-mono text-[#666666]">
          <span className="flex items-center space-x-1 tabular-nums">
            <Clock className="h-3 w-3" />
            <span>{timerText}</span>
          </span>

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
        <div className="p-3 border-t border-[#ebebeb] bg-[#ffffff] font-mono text-[13px] leading-relaxed text-[#333333] whitespace-pre-wrap select-text max-h-96 overflow-y-auto">
          {visibleText}
          {(isTyping || isTaskRunning) && (
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

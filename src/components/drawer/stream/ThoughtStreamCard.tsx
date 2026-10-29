import React, { useState, useEffect, useRef, memo } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Brain,
  Clock,
  Copy,
  Check,
  Loader2,
} from 'lucide-react';
import { DrawerActivityItem, useDrawerStore } from '../../../stores/drawerStore';

export function formatDurationCompact(ms?: number | null): string | null {
  if (ms === undefined || ms === null || !Number.isFinite(ms) || ms <= 0) return null;
  if (ms < 1000) return `${Math.round(ms)}ms`;
  const secs = ms / 1000;
  if (secs < 10) return `${secs.toFixed(1)}s`;
  if (secs < 60) return `${Math.round(secs)}s`;
  const mins = Math.floor(secs / 60);
  const remSecs = Math.round(secs % 60);
  return `${mins}m ${remSecs}s`;
}

export interface ThoughtStreamCardProps {
  activity?: DrawerActivityItem;
  thought?: DrawerActivityItem;
  isTaskRunning: boolean;
  taskId: string;
  isLive?: boolean;
  durationMs?: number;
}

/**
 * Collapsible Live Thinking Card with Streaming Typewriter Effect
 * Smooth progressive character-by-character rendering for real-time thinking.
 * Instant 100% rendering without typing or cursor for historical/completed thoughts.
 */
export const ThoughtStreamCard = memo(function ThoughtStreamCard({
  activity,
  thought,
  isTaskRunning,
  taskId,
  isLive,
  durationMs,
}: ThoughtStreamCardProps) {
  const effectiveThought = thought || activity;
  const [isExpanded, setIsExpanded] = useState(true);
  const [copied, setCopied] = useState(false);
  const copiedTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const userScrolledUpRef = useRef(false);
  const thinkingStart = useDrawerStore((s) => s.thinkingStartTime[taskId] ?? null);
  const blockStart = effectiveThought?.createdAt || thinkingStart;

  useEffect(() => {
    return () => {
      if (copiedTimeoutRef.current) {
        clearTimeout(copiedTimeoutRef.current);
      }
    };
  }, []);

  const fullText = effectiveThought?.content ?? '';

  // Determine if this thought is finished / historical:
  // 1. Task is not running (!isTaskRunning)
  // 2. Explicitly marked not live (isLive === false)
  // 3. Or card already has duration/finished and is not explicitly marked live
  const isFinished =
    !isTaskRunning ||
    isLive === false ||
    (isLive === undefined && effectiveThought?.durationMs !== undefined && effectiveThought.durationMs > 0);

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
    if (isFinished || blockStart == null) return;
    const interval = setInterval(() => {
      setTimerTick((t) => (t + 1) % 1000);
    }, 100);
    return () => clearInterval(interval);
  }, [isFinished, blockStart]);

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

  // Timer calculation: live block duration or final frozen duration
  const effectiveDurationMs = durationMs ?? effectiveThought?.durationMs;
  let timerText = '';
  if (!isFinished) {
    const startMs = effectiveThought?.createdAt || thinkingStart || Date.now();
    timerText = formatDurationCompact(Date.now() - startMs) ?? '';
  } else if (effectiveDurationMs && effectiveDurationMs > 0) {
    timerText = formatDurationCompact(effectiveDurationMs) ?? '';
  }

  const handleCopy = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!fullText) return;
    navigator.clipboard.writeText(fullText);
    setCopied(true);
    if (copiedTimeoutRef.current) {
      clearTimeout(copiedTimeoutRef.current);
    }
    copiedTimeoutRef.current = setTimeout(() => {
      setCopied(false);
      copiedTimeoutRef.current = null;
    }, 1500);
  };

  const visibleText = (isFinished ? fullText : fullText.slice(0, displayedLength)).trimEnd();
  const isTyping = !isFinished && displayedLength < fullText.length;

  // Auto-scroll to bottom during live streaming unless user intentionally scrolled up
  useEffect(() => {
    if (!isFinished && !userScrolledUpRef.current && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [visibleText, isFinished]);

  // When live streaming starts, reset user scroll state
  useEffect(() => {
    if (!isFinished) {
      userScrolledUpRef.current = false;
    }
  }, [isFinished]);

  const handleBodyScroll = () => {
    const el = scrollRef.current;
    if (!el) return;
    const isAtBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 35;
    userScrolledUpRef.current = !isAtBottom;
  };

  const isReasoningTag = fullText.includes('<reasoning>') || fullText.includes('</reasoning>');

  return (
    <div className="animate-silk-entrance motion-reduce:animate-none">
      <div className="rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-[#fcfcfc] dark:bg-[#0d1117] overflow-hidden shadow-xs transition duration-150 animate-in fade-in duration-300 slide-in-from-top-1">
        {/* Header bar */}
        <div
          onClick={() => setIsExpanded(!isExpanded)}
          className="flex items-center justify-between px-3 py-2 bg-[#f8f8f9] dark:bg-[#161b22] hover:bg-[#f4f4f5] dark:hover:bg-[#1c2128] cursor-pointer transition select-none"
        >
          <div className="flex items-center space-x-2">
            {isExpanded ? (
              <ChevronDown className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e]" />
            ) : (
              <ChevronRight className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e]" />
            )}
            <Brain className="h-3.5 w-3.5 text-[#171717] dark:text-[#8b949e]" />
            <span className="font-mono text-xs font-semibold text-[#171717] dark:text-[#f0f6fc]">
              {isReasoningTag ? '<reasoning>' : '<thinking>'}
            </span>
            {!isFinished && isTaskRunning && (
              <span className="flex items-center space-x-1.5">
                <span className="relative flex h-2 w-2 shrink-0">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
                  <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500" />
                </span>
                <Loader2 className="h-3 w-3 animate-spin text-emerald-600 dark:text-emerald-400 shrink-0" />
              </span>
            )}
          </div>

          <div className="flex items-center space-x-2.5 text-[10px] font-mono text-[#71717a] dark:text-[#8b949e]">
            {timerText && (
              <span className="flex items-center space-x-1 tabular-nums">
                <Clock className="h-3 w-3" />
                <span>{timerText}</span>
              </span>
            )}

            <button
              type="button"
              onClick={handleCopy}
              aria-label="Скопировать размышления"
              className="p-1 rounded text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-white dark:hover:bg-[#21262d] transition cursor-pointer"
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
          <div
            ref={scrollRef}
            onScroll={handleBodyScroll}
            className="px-3 pt-2.5 pb-2 border-t border-[#ebebeb] dark:border-[#21262d] bg-[#ffffff] dark:bg-[#090a0f] font-mono text-[13px] leading-relaxed text-[#333333] dark:text-[#c9d1d9] whitespace-pre-wrap select-text max-h-96 overflow-y-auto"
          >
            {visibleText}
            {isTyping && (
              <span
                className="inline-block w-1.5 h-3.5 bg-[#171717] dark:bg-[#f0f6fc] animate-pulse ml-0.5 align-middle select-none"
              />
            )}
          </div>
        )}
      </div>
    </div>
  );
});

export default ThoughtStreamCard;

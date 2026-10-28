import { useState, useRef, useEffect, useLayoutEffect, type MouseEvent } from 'react';
import { createPortal } from 'react-dom';
import { ArrowRightToLine, ArrowRightFromLine, Terminal, Copy, Check } from 'lucide-react';
import { HookDefinition, getHookScript, getHookDescription } from '../../types';
import { useTranslation } from '../../stores/languageStore';

export interface ColumnHookBadgeProps {
  side: 'left' | 'right';
  hookDef: HookDefinition;
  columnTitle: string;
  accentColor?: string;
}

export default function ColumnHookBadge({
  side,
  hookDef,
  columnTitle,
  accentColor,
}: ColumnHookBadgeProps) {
  const { t } = useTranslation();
  const script = getHookScript(hookDef);
  const description = getHookDescription(hookDef);

  const [isHovered, setIsHovered] = useState(false);
  const [coords, setCoords] = useState<{ top: number; left: number } | null>(null);
  const [copied, setCopied] = useState(false);

  const triggerRef = useRef<HTMLDivElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const hideTimerRef = useRef<number | null>(null);
  const copyTimeoutRef = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (hideTimerRef.current) {
        clearTimeout(hideTimerRef.current);
        hideTimerRef.current = null;
      }
      if (copyTimeoutRef.current) {
        clearTimeout(copyTimeoutRef.current);
        copyTimeoutRef.current = null;
      }
    };
  }, []);

  const updatePosition = () => {
    if (!triggerRef.current) return;
    const rect = triggerRef.current.getBoundingClientRect();
    const popoverWidth = 340;
    const popoverHeight = popoverRef.current?.offsetHeight ?? 160;

    let left = side === 'left' ? rect.left : rect.right - popoverWidth;
    left = Math.max(8, Math.min(left, window.innerWidth - popoverWidth - 8));

    let top = rect.bottom + 6;
    if (top + popoverHeight > window.innerHeight - 8) {
      const topAbove = rect.top - popoverHeight - 6;
      if (topAbove >= 8) {
        top = topAbove;
      } else {
        top = Math.max(8, window.innerHeight - popoverHeight - 8);
      }
    }

    setCoords({ top, left });
  };

  useLayoutEffect(() => {
    if (isHovered) {
      updatePosition();
    }
  }, [isHovered, script, description]);

  // Close on scroll, resize, or pointerdown outside
  useEffect(() => {
    if (!isHovered) return;

    const handleClose = () => {
      if (hideTimerRef.current) {
        clearTimeout(hideTimerRef.current);
        hideTimerRef.current = null;
      }
      setIsHovered(false);
    };

    const handlePointerDown = (e: PointerEvent) => {
      if (popoverRef.current?.contains(e.target as Node)) {
        return;
      }
      if (triggerRef.current?.contains(e.target as Node)) {
        return;
      }
      handleClose();
    };

    window.addEventListener('scroll', handleClose, { capture: true, passive: true });
    window.addEventListener('resize', handleClose);
    window.addEventListener('pointerdown', handlePointerDown);

    return () => {
      window.removeEventListener('scroll', handleClose, { capture: true });
      window.removeEventListener('resize', handleClose);
      window.removeEventListener('pointerdown', handlePointerDown);
    };
  }, [isHovered]);

  if (!script) {
    return null;
  }

  const handleMouseEnter = () => {
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
      hideTimerRef.current = null;
    }
    updatePosition();
    setIsHovered(true);
  };

  const handleMouseLeave = () => {
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
    }
    hideTimerRef.current = window.setTimeout(() => {
      setIsHovered(false);
      hideTimerRef.current = null;
    }, 180);
  };

  const handlePopoverMouseEnter = () => {
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
      hideTimerRef.current = null;
    }
  };

  const handlePopoverMouseLeave = () => {
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
    }
    hideTimerRef.current = window.setTimeout(() => {
      setIsHovered(false);
      hideTimerRef.current = null;
    }, 180);
  };

  const handleCopyScript = (e: MouseEvent) => {
    e.stopPropagation();
    navigator.clipboard.writeText(script).catch((err: unknown) => {
      console.warn('Failed to copy hook script to clipboard', err);
    });
    setCopied(true);
    if (copyTimeoutRef.current) {
      clearTimeout(copyTimeoutRef.current);
    }
    copyTimeoutRef.current = window.setTimeout(() => {
      setCopied(false);
      copyTimeoutRef.current = null;
    }, 2000);
  };

  const ariaLabel = `${side === 'left' ? t('board.onEnterHook') : t('board.onLeaveHook')}: ${script}`;

  return (
    <>
      <div
        ref={triggerRef}
        role="button"
        tabIndex={0}
        aria-label={ariaLabel}
        title={ariaLabel}
        onPointerDown={(e) => e.stopPropagation()}
        onClick={(e) => e.stopPropagation()}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
        className={`w-5 h-5 rounded flex items-center justify-center cursor-pointer transition-colors border select-none ${
          isHovered
            ? 'bg-neutral-200 text-neutral-900 border-neutral-300 dark:bg-[#30363d] dark:text-[#f0f6fc] dark:border-[#484f58]'
            : 'bg-neutral-100 text-neutral-500 hover:bg-neutral-200/80 hover:text-neutral-900 border-neutral-200/80 dark:bg-[#21262d] dark:text-[#8b949e] dark:hover:bg-[#30363d] dark:hover:text-[#f0f6fc] dark:border-[#30363d]'
        }`}
      >
        {side === 'left' ? (
          <ArrowRightToLine className="w-3.5 h-3.5 stroke-[2.2]" />
        ) : (
          <ArrowRightFromLine className="w-3.5 h-3.5 stroke-[2.2]" />
        )}
      </div>

      {isHovered && coords && typeof document !== 'undefined' && createPortal(
        <div
          ref={popoverRef}
          className="fixed z-[9999] w-[340px] max-w-[calc(100vw-24px)] shadow-2xl rounded-lg border border-neutral-200 dark:border-[#30363d] bg-white dark:bg-[#161b22] p-3 text-left cursor-default select-text pointer-events-auto"
          style={{
            top: `${coords.top}px`,
            left: `${coords.left}px`,
          }}
          onMouseEnter={handlePopoverMouseEnter}
          onMouseLeave={handlePopoverMouseLeave}
          onPointerDown={(e) => e.stopPropagation()}
          onClick={(e) => e.stopPropagation()}
        >
          {/* Header row */}
          <div className="flex items-center justify-between gap-2 pb-2 border-b border-neutral-100 dark:border-[#21262d]">
            <div className="flex items-center gap-1.5 min-w-0">
              {side === 'left' ? (
                <ArrowRightToLine className="w-3.5 h-3.5 text-neutral-500 dark:text-neutral-400 shrink-0 stroke-[2.2]" />
              ) : (
                <ArrowRightFromLine className="w-3.5 h-3.5 text-neutral-500 dark:text-neutral-400 shrink-0 stroke-[2.2]" />
              )}
              <span className="text-xs font-mono font-medium text-neutral-700 dark:text-[#f0f6fc] truncate">
                {side === 'left' ? t('board.onEnterHook') : t('board.onLeaveHook')}
              </span>
            </div>
            <span
              className="px-1.5 py-0.5 rounded text-[10px] font-medium font-mono shrink-0 max-w-[120px] truncate border"
              style={{
                backgroundColor: accentColor ? `${accentColor}18` : undefined,
                borderColor: accentColor ? `${accentColor}40` : undefined,
                color: accentColor || undefined,
              }}
              title={columnTitle}
            >
              {columnTitle}
            </span>
          </div>

          {/* Hook script block */}
          <div className="mt-2.5">
            <div className="text-[10px] font-mono uppercase tracking-wider text-neutral-500 dark:text-neutral-400 mb-1">
              {t('board.hookScript')}
            </div>
            <div className="flex items-center gap-1.5 p-1.5 rounded bg-neutral-50 dark:bg-[#0d1117] border border-neutral-200 dark:border-[#30363d]">
              <Terminal className="w-3.5 h-3.5 text-neutral-500 dark:text-neutral-400 shrink-0" />
              <span
                className="font-mono text-xs text-neutral-900 dark:text-[#c9d1d9] truncate flex-1 select-all"
                title={script}
              >
                {script}
              </span>
              <button
                type="button"
                onClick={handleCopyScript}
                className="shrink-0 p-1 rounded hover:bg-neutral-200 dark:hover:bg-[#21262d] text-neutral-500 dark:text-neutral-400 hover:text-neutral-800 dark:hover:text-[#f0f6fc] transition-colors cursor-pointer"
                title={copied ? t('board.hookCopied') : t('common.copy')}
                aria-label={copied ? t('board.hookCopied') : t('common.copy')}
              >
                {copied ? (
                  <Check className="w-3.5 h-3.5 text-emerald-600 dark:text-emerald-400 stroke-[2.5]" />
                ) : (
                  <Copy className="w-3.5 h-3.5" />
                )}
              </button>
            </div>
          </div>

          {/* Trigger description */}
          <div className="mt-2.5">
            <div className="text-[10px] font-mono uppercase tracking-wider text-neutral-500 dark:text-neutral-400 mb-0.5">
              {t('board.hookTrigger')}
            </div>
            <div className="text-xs text-neutral-600 dark:text-neutral-300 font-sans leading-relaxed">
              {description || (side === 'left' ? t('board.onEnterTriggerDesc') : t('board.onLeaveTriggerDesc'))}
            </div>
          </div>
        </div>,
        document.body
      )}
    </>
  );
}

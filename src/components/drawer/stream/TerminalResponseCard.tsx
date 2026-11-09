import { useState, useRef, useEffect, memo } from 'react';
import type { DrawerActivityItem } from '../../../stores/drawerStore';
import { useTranslation } from '../../../stores/languageStore';
import {
  ChevronDown,
  ChevronRight,
  CheckCheck,
  Check,
  Copy,
  Clock,
} from 'lucide-react';
import { formatDurationCompact } from './ThoughtStreamCard';
import { MarkdownContent } from './MarkdownContent';

export interface TerminalResponseCardProps {
  activity: DrawerActivityItem;
  durationMs?: number;
  model?: string;
  isExpanded?: boolean;
  onToggleExpanded?: () => void;
  onImageClick?: (src: string, alt?: string) => void;
}

function TerminalResponseCardInner({
  activity,
  durationMs,
  model,
  isExpanded: controlledExpanded,
  onToggleExpanded,
  onImageClick,
}: TerminalResponseCardProps) {
  const [internalExpanded, setInternalExpanded] = useState(true);
  const isExpanded = controlledExpanded !== undefined ? controlledExpanded : internalExpanded;
  const { t } = useTranslation();

  const [copied, setCopied] = useState(false);
  const copyTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (copyTimeoutRef.current) {
        clearTimeout(copyTimeoutRef.current);
      }
    };
  }, []);

  const handleToggle = () => {
    if (onToggleExpanded) {
      onToggleExpanded();
    } else {
      setInternalExpanded((prev) => !prev);
    }
  };

  const handleCopy = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!activity?.content) return;
    navigator.clipboard.writeText(activity.content);
    setCopied(true);
    if (copyTimeoutRef.current) {
      clearTimeout(copyTimeoutRef.current);
    }
    copyTimeoutRef.current = setTimeout(() => {
      setCopied(false);
      copyTimeoutRef.current = null;
    }, 1500);
  };

  const effectiveDurationMs = durationMs ?? activity?.durationMs;
  const durationText = formatDurationCompact(effectiveDurationMs);

  return (
    <div className="animate-silk-entrance motion-reduce:animate-none rounded-lg border border-[#e4e4e7] dark:border-[#212126] bg-white dark:bg-[#111114] shadow-xs overflow-hidden my-2 min-w-0">
      <div
        onClick={handleToggle}
        className="flex items-center justify-between px-4 py-3 hover:bg-[#fafafa] dark:hover:bg-[#17171b] cursor-pointer transition select-none"
      >
        <div className="flex items-center space-x-2 text-[#171717] dark:text-[#f4f4f5]">
          {isExpanded ? (
            <ChevronDown className="h-4 w-4 block shrink-0 text-[#71717a] dark:text-[#71717a]" />
          ) : (
            <ChevronRight className="h-4 w-4 block shrink-0 text-[#71717a] dark:text-[#71717a]" />
          )}
          <CheckCheck className="h-4 w-4 block shrink-0 text-[#171717] dark:text-[#f4f4f5]" />
          <span className="text-sm font-semibold text-[#171717] dark:text-[#f4f4f5] font-sans">
            {t('drawer.agentResponse')}
          </span>
        </div>

        <div className="flex items-center space-x-2.5">
          {model && (
            <span className="px-1.5 py-0.5 rounded text-[10px] font-mono bg-[#f4f4f5] dark:bg-[#232328] text-[#71717a] dark:text-[#71717a] border border-[#e4e4e7] dark:border-[#2c2c33]">
              {model}
            </span>
          )}

          {durationText && (
            <span className="flex items-center space-x-1 text-xs font-mono text-[#71717a] dark:text-[#71717a] tabular-nums">
              <Clock className="h-3.5 w-3.5 block shrink-0 text-[#a1a1aa] dark:text-[#52525b]" />
              <span>{durationText}</span>
            </span>
          )}

          {activity?.content && (
            <button
              type="button"
              onClick={handleCopy}
              aria-label={copied ? 'Copied' : 'Copy response'}
              className="p-1 rounded text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] transition cursor-pointer"
            >
              {copied ? (
                <Check className="h-3.5 w-3.5 block shrink-0 text-emerald-600 dark:text-emerald-400" />
              ) : (
                <Copy className="h-3.5 w-3.5 block shrink-0" />
              )}
            </button>
          )}
        </div>
      </div>

      {isExpanded && (
        <div className="px-4 pb-4 pt-3 border-t border-[#f4f4f5] dark:border-[#212126] min-w-0 break-words">
          {activity?.content ? (
            <MarkdownContent content={activity.content} onImageClick={onImageClick} />
          ) : (
            <div className="text-xs text-[#8f8f8f] font-mono">{t('drawer.generatingResponse')}</div>
          )}
        </div>
      )}
    </div>
  );
}

export const TerminalResponseCard = memo(TerminalResponseCardInner);

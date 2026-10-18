import { useState, useRef, useEffect } from 'react';
import { Brain, ChevronDown, ChevronRight, Copy, Check, Sparkles } from 'lucide-react';

interface ThinkingCardProps {
  thought: string;
  isLive?: boolean;
}

export function ThinkingCard({ thought, isLive }: ThinkingCardProps) {
  const [isExpanded, setIsExpanded] = useState(isLive ?? false);
  const [copied, setCopied] = useState(false);
  const copiedTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (copiedTimeoutRef.current) {
        clearTimeout(copiedTimeoutRef.current);
      }
    };
  }, []);

  const cleanThought = thought.trim();
  if (!cleanThought) return null;

  const handleCopy = (e: React.MouseEvent) => {
    e.stopPropagation();
    navigator.clipboard.writeText(cleanThought);
    setCopied(true);
    if (copiedTimeoutRef.current) {
      clearTimeout(copiedTimeoutRef.current);
    }
    copiedTimeoutRef.current = setTimeout(() => {
      setCopied(false);
      copiedTimeoutRef.current = null;
    }, 1500);
  };

  return (
    <div className="my-2 rounded-lg border border-[#e4e4e7] dark:border-[#21262d] bg-[#fbfbfb] dark:bg-[#0d1117] shadow-2xs overflow-hidden transition-all text-xs">
      {/* Header bar */}
      <button
        type="button"
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full px-3 py-1.5 flex items-center justify-between bg-white dark:bg-[#161b22] hover:bg-[#f9f9f9] dark:hover:bg-[#1c2128] border-b border-[#f0f0f0] dark:border-[#21262d] cursor-pointer text-[#4d4d4d] dark:text-[#c9d1d9] select-none transition"
      >
        <div className="flex items-center space-x-2">
          {isLive ? (
            <Sparkles className="w-3.5 h-3.5 text-[#7c3aed] dark:text-[#a855f7] animate-pulse shrink-0" />
          ) : (
            <Brain className="w-3.5 h-3.5 text-[#71717a] dark:text-[#8b949e] shrink-0" />
          )}
          <span className="font-medium text-[#171717] dark:text-[#f0f6fc] text-[11px] uppercase tracking-wider">
            {isLive ? 'Архитектор размышляет...' : 'Мысли архитектора'}
          </span>
          {isLive && (
            <span className="flex h-1.5 w-1.5 relative">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[#7c3aed] opacity-75" />
              <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-[#7c3aed]" />
            </span>
          )}
        </div>

        <div className="flex items-center space-x-1.5">
          <button
            type="button"
            onClick={handleCopy}
            className="p-1 text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] rounded hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] transition cursor-pointer"
            title="Копировать размышления"
          >
            {copied ? <Check className="w-3 h-3 text-emerald-600 dark:text-emerald-400" /> : <Copy className="w-3 h-3" />}
          </button>
          {isExpanded ? (
            <ChevronDown className="w-3.5 h-3.5 text-[#8f8f8f] dark:text-[#8b949e]" />
          ) : (
            <ChevronRight className="w-3.5 h-3.5 text-[#8f8f8f] dark:text-[#8b949e]" />
          )}
        </div>
      </button>

      {/* Expanded body */}
      {isExpanded && (
        <div className="p-3 text-[11px] leading-relaxed text-[#52525b] dark:text-[#c9d1d9] font-mono whitespace-pre-wrap select-text bg-[#fafafa]/80 dark:bg-[#0d1117] max-h-56 overflow-y-auto">
          {cleanThought}
        </div>
      )}
    </div>
  );
}

export default ThinkingCard;

import { useState } from 'react';
import { Brain, ChevronDown, ChevronRight, Copy, Check, Sparkles } from 'lucide-react';

interface ThinkingCardProps {
  thought: string;
  isLive?: boolean;
}

export function ThinkingCard({ thought, isLive }: ThinkingCardProps) {
  const [isExpanded, setIsExpanded] = useState(isLive ?? false);
  const [copied, setCopied] = useState(false);

  const cleanThought = thought.trim();
  if (!cleanThought) return null;

  const handleCopy = (e: React.MouseEvent) => {
    e.stopPropagation();
    navigator.clipboard.writeText(cleanThought);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className="my-2 rounded-lg border border-[#e4e4e7] bg-[#fbfbfb] shadow-2xs overflow-hidden transition-all text-xs">
      {/* Header bar */}
      <button
        type="button"
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full px-3 py-1.5 flex items-center justify-between bg-white hover:bg-[#f9f9f9] border-b border-[#f0f0f0] cursor-pointer text-[#4d4d4d] select-none transition"
      >
        <div className="flex items-center space-x-2">
          {isLive ? (
            <Sparkles className="w-3.5 h-3.5 text-[#7c3aed] animate-pulse shrink-0" />
          ) : (
            <Brain className="w-3.5 h-3.5 text-[#71717a] shrink-0" />
          )}
          <span className="font-medium text-[#171717] text-[11px] uppercase tracking-wider">
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
            className="p-1 text-[#8f8f8f] hover:text-[#171717] rounded hover:bg-[#f4f4f5] transition cursor-pointer"
            title="Копировать размышления"
          >
            {copied ? <Check className="w-3 h-3 text-emerald-600" /> : <Copy className="w-3 h-3" />}
          </button>
          {isExpanded ? (
            <ChevronDown className="w-3.5 h-3.5 text-[#8f8f8f]" />
          ) : (
            <ChevronRight className="w-3.5 h-3.5 text-[#8f8f8f]" />
          )}
        </div>
      </button>

      {/* Expanded body */}
      {isExpanded && (
        <div className="p-3 text-[11px] leading-relaxed text-[#52525b] font-mono whitespace-pre-wrap select-text bg-[#fafafa]/80 max-h-56 overflow-y-auto">
          {cleanThought}
        </div>
      )}
    </div>
  );
}

export default ThinkingCard;

import { ChevronDown, ChevronRight } from 'lucide-react';

export interface SectionHeaderProps {
  title: string;
  count: number;
  open: boolean;
  onToggle: () => void;
  action?: React.ReactNode;
}

export function SectionHeader({
  title,
  count,
  open,
  onToggle,
  action,
}: SectionHeaderProps) {
  return (
    <div className="w-full flex items-center justify-between px-1.5 py-1 rounded-md text-[#171717] dark:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] transition group">
      <button
        onClick={onToggle}
        aria-expanded={open}
        className="flex-1 flex items-center gap-1.5 py-1 text-left cursor-pointer"
      >
        {open ? (
          <ChevronDown className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
        ) : (
          <ChevronRight className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
        )}
        <span className="font-sans text-xs font-semibold">{title}</span>
        <span className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#8b949e] tabular-nums">({count})</span>
      </button>
      {action && (
        <div className="shrink-0 flex items-center" onClick={(e) => e.stopPropagation()}>
          {action}
        </div>
      )}
    </div>
  );
}

export default SectionHeader;

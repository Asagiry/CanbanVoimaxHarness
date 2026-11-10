import { PanelRightOpen, GitCompare } from 'lucide-react';
import { useTranslation } from '../../../../stores/languageStore';

export interface DiffCollapsedRailProps {
  isCollapsed: boolean;
  onToggleCollapse: () => void;
  isLoading: boolean;
  hasInitialLoaded: boolean;
  filesCount: number;
  totalAdditions: number;
  totalDeletions: number;
}

export function DiffCollapsedRail({
  isCollapsed,
  onToggleCollapse,
  isLoading,
  hasInitialLoaded,
  filesCount,
  totalAdditions,
  totalDeletions,
}: DiffCollapsedRailProps) {
  const { t } = useTranslation();

  return (
    <div
      className={'absolute inset-0 w-11 bg-white dark:bg-[#111114] flex flex-col items-center py-3 select-none transition-opacity duration-200 ' +
        (isCollapsed ? 'opacity-100 pointer-events-auto z-10' : 'opacity-0 pointer-events-none -z-10')}
    >
      <button
        type="button"
        onClick={onToggleCollapse}
        className="p-1.5 rounded-md text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#17171b] transition cursor-pointer"
        aria-label={t('gitDiff.expandPanel')}
      >
        <PanelRightOpen className="h-4 w-4" />
      </button>

      <div className="mt-4 flex flex-col items-center space-y-2">
        <GitCompare
          className={'h-4 w-4 ' +
            (isLoading
              ? 'animate-pulse text-emerald-500'
              : 'text-[#8f8f8f] dark:text-[#71717a]')}
        />
        {!hasInitialLoaded ? (
          <span className="w-3.5 h-3.5 rounded-full border-2 border-emerald-500/20 border-t-emerald-500 animate-spin" />
        ) : (
          <>
            {filesCount > 0 && (
              <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-bold bg-[#f4f4f5] dark:bg-[#17171b] text-[#666666] dark:text-[#71717a] border border-[#ebebeb] dark:border-[#2c2c33]">
                {filesCount}
              </span>
            )}
            {totalAdditions > 0 && (
              <span className="text-[10px] font-mono font-medium text-[#15803d] leading-none">
                +{totalAdditions}
              </span>
            )}
            {totalDeletions > 0 && (
              <span className="text-[10px] font-mono font-medium text-[#b91c1c] leading-none">
                -{totalDeletions}
              </span>
            )}
          </>
        )}
      </div>

      <div
        onClick={onToggleCollapse}
        className="mt-6 flex-1 flex items-center justify-center cursor-pointer group"
        aria-label={t('gitDiff.expandPanel')}
      >
        <span className="text-[11px] font-mono text-[#8f8f8f] dark:text-[#71717a] group-hover:text-[#171717] dark:group-hover:text-[#f4f4f5] transition tracking-wider [writing-mode:vertical-rl] rotate-180">
          Git Diff
        </span>
      </div>
    </div>
  );
}

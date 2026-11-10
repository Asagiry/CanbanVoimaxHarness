import {
  GitCompare,
  GitBranch,
  RefreshCw,
  PanelRightClose,
  ChevronsDownUp,
  ChevronsUpDown,
} from 'lucide-react';
import { useTranslation } from '../../../../stores/languageStore';
import { formatFilesCount } from '../utils';

export interface DiffHeaderProps {
  filesCount: number;
  totalAdditions: number;
  totalDeletions: number;
  branch?: string;
  isLoading: boolean;
  onExpandAll: () => void;
  onCollapseAll: () => void;
  onRefresh: () => void;
  onToggleCollapse: () => void;
}

export function DiffHeader({
  filesCount,
  totalAdditions,
  totalDeletions,
  branch,
  isLoading,
  onExpandAll,
  onCollapseAll,
  onRefresh,
  onToggleCollapse,
}: DiffHeaderProps) {
  const { language, t } = useTranslation();

  return (
    <header className="px-3 py-2 border-b border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114] flex flex-wrap items-center justify-between gap-1.5 shrink-0 shadow-xs min-w-0 transition-colors">
      {/* Left Section: Title, File count, Diff stats */}
      <div className="flex items-center gap-1.5 min-w-0 flex-wrap">
        <div className="flex items-center gap-1.5 shrink-0 leading-none">
          <GitCompare className="h-3.5 w-3.5 block shrink-0 text-[#8f8f8f] dark:text-[#71717a]" />
          <span className="font-semibold text-xs text-[#171717] dark:text-[#f4f4f5] tracking-tight font-sans leading-none">
            Git Diff
          </span>
        </div>

        {/* Changed files count badge */}
        <span
          className="px-1.5 py-0.5 rounded text-[10px] font-mono font-medium bg-[#f4f4f5] dark:bg-[#17171b] text-[#666666] dark:text-[#71717a] border border-[#ebebeb] dark:border-[#2c2c33] shrink-0 whitespace-nowrap leading-none flex items-center"
          title={t('gitDiff.changedFiles', { count: filesCount })}
        >
          {formatFilesCount(filesCount, language)}
        </span>

        {/* Additions / deletions badges */}
        {(totalAdditions > 0 || totalDeletions > 0) && (
          <div className="flex items-center gap-1 text-[10px] font-mono tabular-nums font-semibold shrink-0 leading-none">
            {totalAdditions > 0 && (
              <span className="text-[#15803d] dark:text-[#4ade80] leading-none" title={t('gitDiff.linesAdded', { count: totalAdditions })}>
                +{totalAdditions}
              </span>
            )}
            {totalDeletions > 0 && (
              <span className="text-[#b91c1c] dark:text-[#f87171] leading-none" title={t('gitDiff.linesDeleted', { count: totalDeletions })}>
                -{totalDeletions}
              </span>
            )}
          </div>
        )}
      </div>

      {/* Right Section: Branch badge & Action buttons */}
      <div className="flex items-center gap-1 shrink-0 ml-auto">
        {/* Branch badge with clean truncation and tooltip */}
        {branch && (
          <div
            className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-[#f4f4f5] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] text-[10px] font-mono text-[#666666] dark:text-[#71717a] max-w-[130px] min-w-0 cursor-default"
            title={t('gitDiff.currentBranch', { branch })}
          >
            <GitBranch className="h-3 w-3 text-[#8f8f8f] dark:text-[#71717a] shrink-0" />
            <span className="truncate">{branch}</span>
          </div>
        )}

        {/* Expand / Collapse All Quick Actions */}
        {filesCount > 1 && (
          <div className="flex items-center border border-[#ebebeb] dark:border-[#2c2c33] rounded bg-white dark:bg-[#17171b]">
            <button
              type="button"
              onClick={onExpandAll}
              className="p-1 text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] transition cursor-pointer"
              aria-label={t('gitDiff.expandAllTitle')}
            >
              <ChevronsUpDown className="h-3 w-3" />
            </button>
            <div className="w-px h-3 bg-[#ebebeb] dark:bg-[#2c2c33]" />
            <button
              type="button"
              onClick={onCollapseAll}
              className="p-1 text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] transition cursor-pointer"
              aria-label={t('gitDiff.collapseAllTitle')}
            >
              <ChevronsDownUp className="h-3 w-3" />
            </button>
          </div>
        )}

        {/* Refresh button */}
        <button
          type="button"
          onClick={onRefresh}
          disabled={isLoading}
          className="p-1 rounded text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] disabled:opacity-50 transition cursor-pointer"
          aria-label={t('gitDiff.refresh')}
        >
          <RefreshCw
            className={'h-3.5 w-3.5 ' + (isLoading ? 'animate-spin text-emerald-600' : '')}
          />
        </button>

        {/* Collapse sidebar button */}
        <button
          type="button"
          onClick={onToggleCollapse}
          className="p-1 rounded text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] transition-colors cursor-pointer"
          aria-label={t('gitDiff.collapsePanel')}
        >
          <PanelRightClose className="h-3.5 w-3.5" />
        </button>
      </div>
    </header>
  );
}

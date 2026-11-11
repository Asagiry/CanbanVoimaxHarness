import { useState, useMemo } from 'react';
import { CheckCircle2, GitBranch, RefreshCw, Search, X } from 'lucide-react';
import { DiffFile } from '../../../types';
import { useTranslation } from '../../../stores/languageStore';
import { AccordionFileItem, type AccordionFileItemProps } from './DiffAccordionItem';

export { AccordionFileItem, type AccordionFileItemProps };

export type StatusFilterType = 'all' | 'A' | 'M' | 'D';

export interface DiffFileListProps {
  taskId: string;
  files: DiffFile[];
  hasInitialLoaded: boolean;
  expandedFiles: Set<string>;
  onToggleFile: (path: string) => void;
  branch?: string;
}

export function DiffFileList({
  taskId,
  files,
  hasInitialLoaded,
  expandedFiles,
  onToggleFile,
  branch,
}: DiffFileListProps) {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState('');
  const [statusFilter, setStatusFilter] = useState<StatusFilterType>('all');

  const statusCounts = useMemo(() => {
    let added = 0;
    let modified = 0;
    let deleted = 0;
    for (const f of files) {
      if (f.status === 'A') added++;
      else if (f.status === 'D') deleted++;
      else modified++;
    }
    return { all: files.length, A: added, M: modified, D: deleted };
  }, [files]);

  const hasMultipleStatuses = useMemo(() => {
    let nonZeroTypes = 0;
    if (statusCounts.A > 0) nonZeroTypes++;
    if (statusCounts.M > 0) nonZeroTypes++;
    if (statusCounts.D > 0) nonZeroTypes++;
    return nonZeroTypes > 1;
  }, [statusCounts]);

  const filteredFiles = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    return files.filter((f) => {
      if (!f?.path) return true;
      if (statusFilter !== 'all' && f.status !== statusFilter) return false;
      if (q && !(f.path ?? '').toLowerCase().includes(q)) return false;
      return true;
    });
  }, [files, statusFilter, searchQuery]);

  if (!hasInitialLoaded) {
    return (
      <div className="flex-1 overflow-y-auto p-3 space-y-2.5 min-h-0 bg-[#fafafa] dark:bg-[#0b0b0e]">
        <div className="flex items-center justify-center py-4 space-x-2 text-xs text-[#8f8f8f] dark:text-[#71717a]">
          <RefreshCw className="h-3.5 w-3.5 animate-spin text-emerald-600 dark:text-emerald-400" />
          <span>{t('gitDiff.loadingChanges')}</span>
        </div>
        {[1, 2, 3].map((i) => (
          <div
            key={i}
            className="rounded-md border border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114] p-3 animate-pulse space-y-2"
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-2">
                <div className="h-3.5 w-3.5 bg-[#e5e7eb] dark:bg-[#232328] rounded" />
                <div className="h-3 w-36 bg-[#e5e7eb] dark:bg-[#232328] rounded" />
              </div>
              <div className="h-3 w-10 bg-[#e5e7eb] dark:bg-[#232328] rounded" />
            </div>
            <div className="h-2 w-24 bg-[#f3f4f6] dark:bg-[#17171b] rounded" />
          </div>
        ))}
      </div>
    );
  }

  if (files.length === 0) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center p-6 text-center bg-[#fafafa] dark:bg-[#0b0b0e]">
        <div className="h-12 w-12 rounded-full bg-[#f0fdf4] dark:bg-[#052e16]/50 border border-[#bbf7d0] dark:border-[#166534] flex items-center justify-center mb-3">
          <CheckCircle2 className="h-6 w-6 text-[#15803d] dark:text-[#4ade80]" />
        </div>
        <h3 className="text-sm font-semibold text-[#171717] dark:text-[#f4f4f5] font-sans">
          {t('gitDiff.cleanWorktree')}
        </h3>
        <p className="text-xs text-[#666666] dark:text-[#71717a] mt-1 max-w-[260px] leading-relaxed">
          {t('gitDiff.noChanges')}
        </p>
        {branch && (
          <div className="mt-4 px-2.5 py-1 rounded-md bg-white dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] text-[11px] font-mono text-[#666666] dark:text-[#71717a] flex items-center space-x-1.5">
            <GitBranch className="h-3 w-3 text-[#8f8f8f] dark:text-[#71717a]" />
            <span>{t('gitDiff.branch', { branch })}</span>
          </div>
        )}
      </div>
    );
  }

  const showToolbar = files.length > 3 || searchQuery !== '' || (hasMultipleStatuses && statusFilter !== 'all');

  return (
    <div className="flex-1 flex flex-col min-h-0 overflow-hidden bg-[#fafafa] dark:bg-[#0b0b0e]">
      {showToolbar && (
        <div className="px-3 pt-2 pb-1.5 border-b border-[#ebebeb] dark:border-[#212126] bg-[#fcfcfc] dark:bg-[#111114] flex flex-col gap-1.5 shrink-0">
          <div className="relative flex items-center">
            <Search className="absolute left-2 h-3.5 w-3.5 text-[#8f8f8f] dark:text-[#71717a] pointer-events-none" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t('gitDiff.filterPlaceholder')}
              className="w-full pl-7 pr-6 py-1 bg-white dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] rounded text-xs font-mono text-[#171717] dark:text-[#f4f4f5] placeholder-[#8f8f8f] dark:placeholder-[#52525b] focus:outline-hidden focus:border-emerald-500 transition"
            />
            {searchQuery && (
              <button
                type="button"
                onClick={() => setSearchQuery('')}
                aria-label={t('gitDiff.resetFilters')}
                className="absolute right-1.5 p-0.5 text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] cursor-pointer"
              >
                <X className="h-3 w-3" />
              </button>
            )}
          </div>

          {hasMultipleStatuses && (
            <div className="flex items-center gap-1 text-[10px] font-mono tabular-nums">
              <button
                type="button"
                onClick={() => setStatusFilter('all')}
                className={'px-1.5 py-0.5 rounded cursor-pointer transition ' +
                  (statusFilter === 'all'
                    ? 'bg-[#171717] text-white dark:bg-[#f4f4f5] dark:text-[#0b0b0e] font-semibold'
                    : 'bg-[#f4f4f5] dark:bg-[#17171b] text-[#666666] dark:text-[#71717a] hover:bg-[#e4e4e7] dark:hover:bg-[#232328]')}
              >
                {t('gitDiff.filterAll', { count: statusCounts.all })}
              </button>
              {statusCounts.A > 0 && (
                <button
                  type="button"
                  onClick={() => setStatusFilter('A')}
                  className={'px-1.5 py-0.5 rounded cursor-pointer transition ' +
                    (statusFilter === 'A'
                      ? 'bg-[#15803d] text-white font-semibold'
                      : 'bg-[#f0fdf4] dark:bg-[#12281e] text-[#15803d] dark:text-[#7ee787] hover:bg-[#dcfce7] dark:hover:bg-[#173a2b]')}
                >
                  {t('gitDiff.filterAdded', { count: statusCounts.A })}
                </button>
              )}
              {statusCounts.M > 0 && (
                <button
                  type="button"
                  onClick={() => setStatusFilter('M')}
                  className={'px-1.5 py-0.5 rounded cursor-pointer transition ' +
                    (statusFilter === 'M'
                      ? 'bg-[#b45309] text-white font-semibold'
                      : 'bg-[#fffbeb] dark:bg-[#2d2208] text-[#b45309] dark:text-[#d29922] hover:bg-[#fef3c7] dark:hover:bg-[#3b2d0b]')}
                >
                  {t('gitDiff.filterModified', { count: statusCounts.M })}
                </button>
              )}
              {statusCounts.D > 0 && (
                <button
                  type="button"
                  onClick={() => setStatusFilter('D')}
                  className={'px-1.5 py-0.5 rounded cursor-pointer transition ' +
                    (statusFilter === 'D'
                      ? 'bg-[#b91c1c] text-white font-semibold'
                      : 'bg-[#fef2f2] dark:bg-[#3b1219] text-[#b91c1c] dark:text-[#ff7b72] hover:bg-[#fee2e2] dark:hover:bg-[#4d1922]')}
                >
                  {t('gitDiff.filterDeleted', { count: statusCounts.D })}
                </button>
              )}
            </div>
          )}
        </div>
      )}

      <div className="flex-1 overflow-y-auto p-3 space-y-2 min-h-0">
        {filteredFiles.length === 0 ? (
          <div className="p-6 text-center text-xs text-[#8f8f8f] dark:text-[#71717a]">
            <p>{t('gitDiff.noMatchingFiles')}</p>
            {(searchQuery || statusFilter !== 'all') && (
              <button
                type="button"
                onClick={() => {
                  setSearchQuery('');
                  setStatusFilter('all');
                }}
                className="mt-2 text-emerald-600 dark:text-emerald-400 hover:underline cursor-pointer"
              >
                {t('gitDiff.resetFilters')}
              </button>
            )}
          </div>
        ) : (
          filteredFiles.map((file) => {
            if (!file?.path) return null;
            const isExpanded = expandedFiles.has(file.path);
            return (
              <AccordionFileItem
                key={file.path}
                taskId={taskId}
                file={file}
                isExpanded={isExpanded}
                onToggle={() => onToggleFile(file.path)}
              />
            );
          })
        )}
      </div>
    </div>
  );
}

export default DiffFileList;

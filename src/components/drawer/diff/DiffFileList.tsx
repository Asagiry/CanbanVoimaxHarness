import React, { useState, useEffect, useRef, useMemo } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Check,
  Copy,
  Image as ImageIcon,
  CheckCircle2,
  GitBranch,
  RefreshCw,
  Search,
  X,
} from 'lucide-react';
import { DiffFile } from '../../../types';
import { useTranslation } from '../../../stores/languageStore';
import { isImageFile, parseCleanDiff } from './utils';
import ImageDiffViewer from './ImageDiffViewer';

export interface AccordionFileItemProps {
  taskId: string;
  file: DiffFile;
  isExpanded: boolean;
  onToggle: () => void;
}

export function AccordionFileItem({ taskId, file, isExpanded, onToggle }: AccordionFileItemProps) {
  const [copied, setCopied] = useState(false);
  const { t } = useTranslation();
  const copiedTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (copiedTimeoutRef.current) {
        clearTimeout(copiedTimeoutRef.current);
      }
    };
  }, []);

  const isImage = isImageFile(file.path);

  const parsedLines = useMemo(() => {
    if (isImage) return [];
    return parseCleanDiff(file.patch);
  }, [file.patch, isImage]);

  const lastSlashIndex = file.path.lastIndexOf('/');
  const dirPart = lastSlashIndex !== -1 ? file.path.slice(0, lastSlashIndex + 1) : '';
  const filePart = lastSlashIndex !== -1 ? file.path.slice(lastSlashIndex + 1) : file.path;

  const statusColor =
    file.status === 'A'
      ? 'bg-[#f0fdf4] dark:bg-[#12281e] text-[#15803d] dark:text-[#7ee787] border-[#bbf7d0] dark:border-[#238636]'
      : file.status === 'D'
      ? 'bg-[#fef2f2] dark:bg-[#3b1219] text-[#b91c1c] dark:text-[#ff7b72] border-[#fecaca] dark:border-[#da3633]'
      : 'bg-[#fffbeb] dark:bg-[#2d2208] text-[#b45309] dark:text-[#d29922] border-[#fde68a] dark:border-[#9e6a03]';

  const handleCopyPatch = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!file.patch) return;
    try {
      await navigator.clipboard.writeText(file.patch);
      setCopied(true);
      if (copiedTimeoutRef.current) {
        clearTimeout(copiedTimeoutRef.current);
      }
      copiedTimeoutRef.current = setTimeout(() => {
        setCopied(false);
        copiedTimeoutRef.current = null;
      }, 1500);
    } catch {
      // ignore clipboard error
    }
  };

  return (
    <div className="rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#161b22] overflow-hidden shadow-xs transition duration-150">
      {/* File Header Row (Clickable Accordion Trigger) */}
      <div
        onClick={onToggle}
        className="flex items-center justify-between px-3 py-2 bg-[#fcfcfc] dark:bg-[#161b22] hover:bg-[#f4f4f5] dark:hover:bg-[#1c2128] cursor-pointer transition select-none"
      >
        <div className="flex items-center space-x-2 min-w-0 mr-2">
          {isExpanded ? (
            <ChevronDown className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
          )}

          <span
            className={'px-1.5 py-0.2 rounded text-[10px] font-bold border shrink-0 font-mono ' + statusColor}
          >
            {file.status}
          </span>

          {isImage && (
            <ImageIcon className="h-3.5 w-3.5 text-[#8f8f8f] dark:text-[#8b949e] shrink-0" />
          )}

          <span className="truncate text-xs font-mono" title={file.path}>
            {dirPart && <span className="text-[#8f8f8f] dark:text-[#8b949e]">{dirPart}</span>}
            <span className="font-semibold text-[#171717] dark:text-[#f0f6fc]">{filePart}</span>
          </span>
        </div>

        <div className="flex items-center space-x-2 shrink-0 font-mono text-[11px] tabular-nums">
          {(file.additions ?? 0) > 0 && (
            <span className="text-[#15803d] dark:text-[#4ade80] font-medium">+{file.additions}</span>
          )}
          {(file.deletions ?? 0) > 0 && (
            <span className="text-[#b91c1c] dark:text-[#f87171] font-medium">-{file.deletions}</span>
          )}

          {file.patch ? (
            <button
              type="button"
              onClick={handleCopyPatch}
              className="p-1 rounded text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#ebebeb] dark:hover:bg-[#21262d] transition cursor-pointer"
              aria-label={copied ? t('gitDiff.patchCopied') : t('gitDiff.copyPatch')}
            >
              {copied ? (
                <Check className="h-3 w-3 text-emerald-600 dark:text-emerald-400" />
              ) : (
                <Copy className="h-3 w-3" />
              )}
            </button>
          ) : null}
        </div>
      </div>

      {/* Expanded Diff View: ImageDiffViewer for images, text diff otherwise */}
      {isExpanded && (
        isImage ? (
          <ImageDiffViewer taskId={taskId} filePath={file.path} />
        ) : (
          <div className="border-t border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] font-mono text-[11px] leading-relaxed select-text overflow-x-auto max-h-[460px] overflow-y-auto">
            {parsedLines.length === 0 ? (
              <div className="p-3 text-center text-[#8f8f8f] dark:text-[#8b949e] text-xs">
                {t('gitDiff.noTextChanges')}
              </div>
            ) : (
              parsedLines.map((line, idx) => {
                if (line.type === 'hunk') {
                  return (
                    <div
                      key={idx}
                      className="bg-[#f4f4f5] dark:bg-[#161b22] text-[#666666] dark:text-[#8b949e] border-y border-[#ebebeb] dark:border-[#21262d] my-0.5 py-0.5 px-2 text-[10px] font-semibold select-text"
                    >
                      {line.text}
                    </div>
                  );
                }

                if (line.type === 'add') {
                  return (
                    <div
                      key={idx}
                      className="flex items-start bg-[#f0fdf4] dark:bg-[#12281e] text-[#166534] dark:text-[#7ee787] hover:bg-[#dcfce7] dark:hover:bg-[#173a2b] py-0.5"
                    >
                      <span className="w-8 shrink-0 text-right pr-2 text-[#8f8f8f] dark:text-[#8b949e] select-none text-[10px]">
                        {/* empty old line */}
                      </span>
                      <span className="w-8 shrink-0 text-right pr-2 text-[#16a34a] dark:text-[#7ee787] select-none text-[10px] tabular-nums font-mono">
                        {line.newNum}
                      </span>
                      <span className="w-4 shrink-0 text-center text-[#16a34a] dark:text-[#7ee787] font-bold select-none">
                        +
                      </span>
                      <span className="flex-1 whitespace-pre-wrap break-all pr-2">
                        {line.text}
                      </span>
                    </div>
                  );
                }

                if (line.type === 'del') {
                  return (
                    <div
                      key={idx}
                      className="flex items-start bg-[#fef2f2] dark:bg-[#3b1219] text-[#991b1b] dark:text-[#ff7b72] hover:bg-[#fee2e2] dark:hover:bg-[#4d1922] py-0.5"
                    >
                      <span className="w-8 shrink-0 text-right pr-2 text-[#ef4444] dark:text-[#ff7b72] select-none text-[10px] tabular-nums font-mono">
                        {line.oldNum}
                      </span>
                      <span className="w-8 shrink-0 text-right pr-2 text-[#8f8f8f] dark:text-[#8b949e] select-none text-[10px]">
                        {/* empty new line */}
                      </span>
                      <span className="w-4 shrink-0 text-center text-[#ef4444] dark:text-[#ff7b72] font-bold select-none">
                        -
                      </span>
                      <span className="flex-1 whitespace-pre-wrap break-all pr-2">
                        {line.text}
                      </span>
                    </div>
                  );
                }

                // Context line
                return (
                  <div
                    key={idx}
                    className="flex items-start hover:bg-[#f9f9f9] dark:hover:bg-[#161b22] py-0.5 text-[#171717] dark:text-[#f0f6fc]"
                  >
                    <span className="w-8 shrink-0 text-right pr-2 text-[#8f8f8f] dark:text-[#8b949e] select-none text-[10px] tabular-nums font-mono">
                      {line.oldNum}
                    </span>
                    <span className="w-8 shrink-0 text-right pr-2 text-[#8f8f8f] dark:text-[#8b949e] select-none text-[10px] tabular-nums font-mono">
                      {line.newNum}
                    </span>
                    <span className="w-4 shrink-0 text-center text-[#8f8f8f] dark:text-[#8b949e] select-none">
                      {' '}
                    </span>
                    <span className="flex-1 whitespace-pre-wrap break-all pr-2">
                      {line.text}
                    </span>
                  </div>
                );
              })
            )}
          </div>
        )
      )}
    </div>
  );
}

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

  // Filter calculations
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
      if (statusFilter !== 'all' && f.status !== statusFilter) return false;
      if (q && !f.path.toLowerCase().includes(q)) return false;
      return true;
    });
  }, [files, statusFilter, searchQuery]);

  // Loading skeleton state
  if (!hasInitialLoaded) {
    return (
      <div className="flex-1 overflow-y-auto p-3 space-y-2.5 min-h-0 bg-[#fafafa] dark:bg-[#090a0f]">
        <div className="flex items-center justify-center py-4 space-x-2 text-xs text-[#8f8f8f] dark:text-[#8b949e]">
          <RefreshCw className="h-3.5 w-3.5 animate-spin text-emerald-600 dark:text-emerald-400" />
          <span>{t('gitDiff.loadingChanges')}</span>
        </div>
        {[1, 2, 3].map((i) => (
          <div
            key={i}
            className="rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] p-3 animate-pulse space-y-2"
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-2">
                <div className="h-3.5 w-3.5 bg-[#e5e7eb] dark:bg-[#21262d] rounded" />
                <div className="h-3 w-36 bg-[#e5e7eb] dark:bg-[#21262d] rounded" />
              </div>
              <div className="h-3 w-10 bg-[#e5e7eb] dark:bg-[#21262d] rounded" />
            </div>
            <div className="h-2 w-24 bg-[#f3f4f6] dark:bg-[#161b22] rounded" />
          </div>
        ))}
      </div>
    );
  }

  // Clean empty state placeholder
  if (files.length === 0) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center p-6 text-center bg-[#fafafa] dark:bg-[#090a0f]">
        <div className="h-12 w-12 rounded-full bg-[#f0fdf4] dark:bg-[#052e16]/50 border border-[#bbf7d0] dark:border-[#166534] flex items-center justify-center mb-3">
          <CheckCircle2 className="h-6 w-6 text-[#15803d] dark:text-[#4ade80]" />
        </div>
        <h3 className="text-sm font-semibold text-[#171717] dark:text-[#f0f6fc] font-sans">
          {t('gitDiff.cleanWorktree')}
        </h3>
        <p className="text-xs text-[#666666] dark:text-[#8b949e] mt-1 max-w-[260px] leading-relaxed">
          {t('gitDiff.noChanges')}
        </p>
        {branch && (
          <div className="mt-4 px-2.5 py-1 rounded-md bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[11px] font-mono text-[#666666] dark:text-[#8b949e] flex items-center space-x-1.5">
            <GitBranch className="h-3 w-3 text-[#8f8f8f] dark:text-[#8b949e]" />
            <span>{t('gitDiff.branch', { branch })}</span>
          </div>
        )}
      </div>
    );
  }

  // Show filter toolbar when there are more than 3 files or an active filter exists
  const showToolbar = files.length > 3 || searchQuery !== '' || (hasMultipleStatuses && statusFilter !== 'all');

  return (
    <div className="flex-1 flex flex-col min-h-0 overflow-hidden bg-[#fafafa] dark:bg-[#090a0f]">
      {/* File Search & Filter Bar */}
      {showToolbar && (
        <div className="px-3 pt-2 pb-1.5 border-b border-[#ebebeb] dark:border-[#21262d] bg-[#fcfcfc] dark:bg-[#0d1117] flex flex-col gap-1.5 shrink-0">
          {/* Search Input */}
          <div className="relative flex items-center">
            <Search className="absolute left-2 h-3.5 w-3.5 text-[#8f8f8f] dark:text-[#8b949e] pointer-events-none" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Фильтр файлов..."
              className="w-full pl-7 pr-6 py-1 bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] rounded text-xs font-mono text-[#171717] dark:text-[#f0f6fc] placeholder-[#8f8f8f] dark:placeholder-[#6e7681] focus:outline-hidden focus:border-emerald-500 transition"
            />
            {searchQuery && (
              <button
                type="button"
                onClick={() => setSearchQuery('')}
                className="absolute right-1.5 p-0.5 text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] cursor-pointer"
              >
                <X className="h-3 w-3" />
              </button>
            )}
          </div>

          {/* Status Pills */}
          {hasMultipleStatuses && (
            <div className="flex items-center gap-1 text-[10px] font-mono">
              <button
                type="button"
                onClick={() => setStatusFilter('all')}
                className={'px-1.5 py-0.5 rounded cursor-pointer transition ' +
                  (statusFilter === 'all'
                    ? 'bg-[#171717] text-white dark:bg-[#f0f6fc] dark:text-[#090d16] font-semibold'
                    : 'bg-[#f4f4f5] dark:bg-[#161b22] text-[#666666] dark:text-[#8b949e] hover:bg-[#e4e4e7] dark:hover:bg-[#21262d]')}
              >
                Все ({statusCounts.all})
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
                  +Добавлено ({statusCounts.A})
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
                  ~Изменено ({statusCounts.M})
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
                  -Удалено ({statusCounts.D})
                </button>
              )}
            </div>
          )}
        </div>
      )}

      {/* Filtered File List */}
      <div className="flex-1 overflow-y-auto p-3 space-y-2 min-h-0">
        {filteredFiles.length === 0 ? (
          <div className="p-6 text-center text-xs text-[#8f8f8f] dark:text-[#8b949e]">
            <p>Нет файлов, соответствующих фильтру</p>
            {(searchQuery || statusFilter !== 'all') && (
              <button
                type="button"
                onClick={() => {
                  setSearchQuery('');
                  setStatusFilter('all');
                }}
                className="mt-2 text-emerald-600 dark:text-emerald-400 hover:underline cursor-pointer"
              >
                Сбросить фильтры
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

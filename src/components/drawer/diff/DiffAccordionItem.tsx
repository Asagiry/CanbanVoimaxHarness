import React, { useState, useEffect, useRef, useMemo } from 'react';
import { ChevronDown, ChevronRight, Check, Copy, Image as ImageIcon } from 'lucide-react';
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
    } catch (err) {
      console.warn('[DiffAccordionItem] clipboard copy failed:', err);
    }
  };

  return (
    <div className="rounded-md border border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#17171b] overflow-hidden shadow-xs transition duration-150">
      {/* File Header Row */}
      <div className="flex items-center justify-between px-3 py-2 bg-[#fcfcfc] dark:bg-[#17171b] hover:bg-[#f4f4f5] dark:hover:bg-[#1e1e23] transition select-none">
        <button
          type="button"
          onClick={onToggle}
          aria-expanded={isExpanded}
          className="flex-1 flex items-center space-x-2 min-w-0 mr-2 text-left cursor-pointer"
        >
          {isExpanded ? (
            <ChevronDown className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />
          )}

          <span
            className={'inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-bold border shrink-0 font-mono leading-none ' + statusColor}
          >
            {file.status}
          </span>

          {isImage && (
            <ImageIcon className="h-3.5 w-3.5 text-[#8f8f8f] dark:text-[#71717a] shrink-0" />
          )}

          <span className="truncate text-xs font-mono" title={file.path}>
            {dirPart && <span className="text-[#8f8f8f] dark:text-[#71717a]">{dirPart}</span>}
            <span className="font-semibold text-[#171717] dark:text-[#f4f4f5]">{filePart}</span>
          </span>
        </button>

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
              className="p-1 rounded text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#ebebeb] dark:hover:bg-[#232328] transition cursor-pointer"
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

      {/* Expanded Diff View */}
      {isExpanded && (
        isImage ? (
          <ImageDiffViewer taskId={taskId} filePath={file.path} />
        ) : (
          <div className="border-t border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114] font-mono text-[11px] leading-relaxed select-text overflow-x-auto max-h-[460px] overflow-y-auto">
            {parsedLines.length === 0 ? (
              <div className="p-3 text-center text-[#8f8f8f] dark:text-[#71717a] text-xs">
                {t('gitDiff.noTextChanges')}
              </div>
            ) : (
              parsedLines.map((line, linePos) => {
                const lineKey =
                  line.type === 'hunk'
                    ? `hunk-${line.text}-${linePos}`
                    : `${line.type}-${line.oldNum ?? 'none'}-${line.newNum ?? 'none'}`;

                if (line.type === 'hunk') {
                  return (
                    <div
                      key={lineKey}
                      className="bg-[#f4f4f5] dark:bg-[#17171b] text-[#666666] dark:text-[#71717a] border-y border-[#ebebeb] dark:border-[#212126] my-0.5 py-0.5 px-2 text-[10px] font-semibold select-text"
                    >
                      {line.text}
                    </div>
                  );
                }

                if (line.type === 'add') {
                  return (
                    <div
                      key={lineKey}
                      className="flex items-start bg-[#f0fdf4] dark:bg-[#12281e] text-[#166534] dark:text-[#7ee787] hover:bg-[#dcfce7] dark:hover:bg-[#173a2b] py-0.5"
                    >
                      <span className="w-8 shrink-0 text-right pr-2 text-[#8f8f8f] dark:text-[#71717a] select-none text-[10px]">
                        {/* empty old line */}
                      </span>
                      <span className="w-8 shrink-0 text-right pr-2 text-[#16a34a] dark:text-[#7ee787] select-none text-[10px] tabular-nums font-mono">
                        {line.newNum}
                      </span>
                      <span className="w-4 shrink-0 text-center text-[#16a34a] dark:text-[#7ee787] font-bold select-none">+</span>
                      <span className="flex-1 whitespace-pre-wrap break-all pr-2">{line.text}</span>
                    </div>
                  );
                }

                if (line.type === 'del') {
                  return (
                    <div
                      key={lineKey}
                      className="flex items-start bg-[#fef2f2] dark:bg-[#3b1219] text-[#991b1b] dark:text-[#ff7b72] hover:bg-[#fee2e2] dark:hover:bg-[#4d1922] py-0.5"
                    >
                      <span className="w-8 shrink-0 text-right pr-2 text-[#ef4444] dark:text-[#ff7b72] select-none text-[10px] tabular-nums font-mono">
                        {line.oldNum}
                      </span>
                      <span className="w-8 shrink-0 text-right pr-2 text-[#8f8f8f] dark:text-[#71717a] select-none text-[10px]">
                        {/* empty new line */}
                      </span>
                      <span className="w-4 shrink-0 text-center text-[#ef4444] dark:text-[#ff7b72] font-bold select-none">-</span>
                      <span className="flex-1 whitespace-pre-wrap break-all pr-2">{line.text}</span>
                    </div>
                  );
                }

                return (
                  <div
                    key={lineKey}
                    className="flex items-start hover:bg-[#f9f9f9] dark:hover:bg-[#17171b] py-0.5 text-[#171717] dark:text-[#f4f4f5]"
                  >
                    <span className="w-8 shrink-0 text-right pr-2 text-[#8f8f8f] dark:text-[#71717a] select-none text-[10px] tabular-nums font-mono">
                      {line.oldNum}
                    </span>
                    <span className="w-8 shrink-0 text-right pr-2 text-[#8f8f8f] dark:text-[#71717a] select-none text-[10px] tabular-nums font-mono">
                      {line.newNum}
                    </span>
                    <span className="w-4 shrink-0 text-center text-[#8f8f8f] dark:text-[#71717a] select-none">{' '}</span>
                    <span className="flex-1 whitespace-pre-wrap break-all pr-2">{line.text}</span>
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

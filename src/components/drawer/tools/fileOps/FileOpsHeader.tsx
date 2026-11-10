import React from 'react';
import { ChevronRight, FileCode2, FileText } from 'lucide-react';
import { FileExtensionBadge, getLanguageBadgeConfig } from '../../../common/BrandFileIcons';
import { ToolExecution } from '../../../../types';
import { formatDurationCompact } from '../shared';
import { FileOpsParsedInfo } from './fileOpsUtils';

const CODE_EXTENSIONS = new Set([
  'ts', 'tsx', 'js', 'jsx', 'rs', 'py', 'json', 'toml',
  'yaml', 'yml', 'xaml', 'xml', 'html', 'css', 'scss', 'sh', 'ps1', 'bat'
]);

export interface FileOpsHeaderProps {
  tool: ToolExecution;
  parsed: FileOpsParsedInfo;
  isExpanded: boolean;
  onToggle: (e: React.MouseEvent) => void;
}

export function FileOpsHeader({ tool, parsed, isExpanded, onToggle }: FileOpsHeaderProps) {
  const {
    isReadOp,
    ext,
    dirPath,
    fileName,
    rawFilePath,
    lineRangeBadge,
    additions,
    deletions,
    opLabel,
    isFailed,
  } = parsed;

  const isCode = CODE_EXTENSIONS.has(ext);
  const DocIcon = isCode ? FileCode2 : FileText;
  const langBadge = getLanguageBadgeConfig(ext);
  const durationText = formatDurationCompact(tool.durationMs);

  if (isReadOp) {
    return (
      <div className="flex items-center gap-1.5 min-w-0 flex-1 leading-none">
        <button
          type="button"
          className="p-0.5 text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] rounded shrink-0 transition cursor-pointer"
          onClick={onToggle}
          aria-label={isExpanded ? 'Свернуть подробности' : 'Развернуть подробности'}
        >
          <ChevronRight
            className={`h-3 w-3 block shrink-0 transition-transform duration-150 ${
              isExpanded ? 'rotate-90 text-[#171717] dark:text-[#f4f4f5]' : ''
            }`}
          />
        </button>

        <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-mono font-medium shrink-0 select-none bg-[#ecfeff] dark:bg-[#083344] text-[#0e7490] dark:text-[#22d3ee] border border-[#a5f3fc] dark:border-[#155e75]">
          Analyzed
        </span>

        {langBadge ? (
          <FileExtensionBadge ext={ext} />
        ) : (
          <DocIcon className="h-3.5 w-3.5 block shrink-0 text-[#52525b] dark:text-[#71717a]" />
        )}

        <div className="flex items-center min-w-0 font-mono text-xs overflow-hidden">
          {dirPath && (
            <span className="text-[#8f8f8f] dark:text-[#71717a] truncate max-w-[120px] sm:max-w-[180px] shrink-0">
              {dirPath}/
            </span>
          )}
          <span className="font-semibold text-[#171717] dark:text-[#f4f4f5] truncate">
            {fileName || rawFilePath || 'file'}
          </span>
        </div>

        {lineRangeBadge && (
          <span className="font-mono text-[11px] text-[#71717a] dark:text-[#71717a] tabular-nums shrink-0 ml-1">
            {lineRangeBadge}
          </span>
        )}

        {durationText && (
          <>
            <span className="text-[#a1a1aa] dark:text-[#52525b] select-none text-[10px] leading-none">•</span>
            <span className="font-mono text-[#71717a] dark:text-[#71717a] shrink-0 tabular-nums text-[11px]">
              {durationText}
            </span>
          </>
        )}
      </div>
    );
  }

  return (
    <div className="flex items-center gap-1.5 min-w-0 flex-1 leading-none">
      <button
        type="button"
        className="p-0.5 text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] rounded shrink-0 transition cursor-pointer"
        onClick={onToggle}
        aria-label={isExpanded ? 'Свернуть подробности' : 'Развернуть подробности'}
      >
        <ChevronRight
          className={`h-3 w-3 block shrink-0 transition-transform duration-150 ${
            isExpanded ? 'rotate-90 text-[#171717] dark:text-[#f4f4f5]' : ''
          }`}
        />
      </button>

      <span
        className={`inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-mono font-medium shrink-0 select-none ${
          isFailed
            ? 'bg-[#fef2f2] dark:bg-[#3b1219] text-[#b91c1c] dark:text-[#ff7b72] border border-[#fecaca] dark:border-[#da3633]'
            : opLabel === 'Created'
            ? 'bg-[#ecfdf5] dark:bg-[#12281e] text-[#047857] dark:text-[#7ee787] border border-[#a7f3d0] dark:border-[#238636]'
            : 'bg-[#eff6ff] dark:bg-[#172554] text-[#1d4ed8] dark:text-[#60a5fa] border border-[#bfdbfe] dark:border-[#1e40af]'
        }`}
      >
        {isFailed ? 'Failed edit' : opLabel}
      </span>

      {langBadge ? (
        <FileExtensionBadge ext={ext} />
      ) : (
        <DocIcon className="h-3.5 w-3.5 block shrink-0 text-[#52525b] dark:text-[#71717a]" />
      )}

      <div className="flex items-center min-w-0 font-mono text-xs overflow-hidden">
        {dirPath && (
          <span className="text-[#8f8f8f] dark:text-[#71717a] truncate max-w-[120px] sm:max-w-[180px] shrink-0">
            {dirPath}/
          </span>
        )}
        <span className="font-semibold text-[#171717] dark:text-[#f4f4f5] truncate">
          {fileName || rawFilePath || 'file'}
        </span>
      </div>

      {(additions > 0 || deletions > 0) && (
        <div className="flex items-center space-x-1 font-mono text-[11px] tabular-nums font-semibold shrink-0 ml-1">
          {additions > 0 && (
            <span className="inline-flex items-center text-[#16a34a] dark:text-[#7ee787] bg-[#f0fdf4] dark:bg-[#12281e] px-1 py-0.5 rounded leading-none border border-[#bbf7d0] dark:border-[#238636]">
              +{additions}
            </span>
          )}
          {deletions > 0 && (
            <span className="inline-flex items-center text-[#dc2626] dark:text-[#ff7b72] bg-[#fef2f2] dark:bg-[#3b1219] px-1 py-0.5 rounded leading-none border border-[#fecaca] dark:border-[#da3633]">
              -{deletions}
            </span>
          )}
        </div>
      )}

      {durationText && (
        <>
          <span className="text-[#a1a1aa] dark:text-[#52525b] select-none text-[10px] leading-none">•</span>
          <span className="font-mono text-[#71717a] dark:text-[#71717a] shrink-0 tabular-nums text-[11px]">
            {durationText}
          </span>
        </>
      )}
    </div>
  );
}

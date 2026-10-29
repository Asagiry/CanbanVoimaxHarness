import { useState } from 'react';
import { ChevronRight, FileCode2, FileText } from 'lucide-react';
import { FileExtensionBadge, getLanguageBadgeConfig } from '../../common/BrandFileIcons';
import {
  ToolCardProps,
  isToolFailed,
  StatusIndicator,
  CopyLogsButton,
  ToolParamsView,
  formatDurationCompact,
} from './shared';

const getLanguageBadge = getLanguageBadgeConfig;

export default function FileOpsToolCard({ tool }: ToolCardProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  const toolName = tool.toolName ?? 'file_op';
  const params: Record<string, unknown> =
    tool.params && typeof tool.params === 'object'
      ? (tool.params as Record<string, unknown>)
      : {};

  const isFileOp =
    toolName === 'write_file' ||
    toolName === 'write' ||
    toolName === 'edit_file' ||
    toolName === 'edit' ||
    toolName === 'str_replace';

  const isReadOp =
    toolName === 'read_file' ||
    toolName === 'read' ||
    toolName === 'fetch_file';

  const isEditTool =
    toolName === 'edit_file' ||
    toolName === 'str_replace' ||
    toolName === 'edit';
  const isDiffTool = isEditTool;

  const rawFilePath =
    (params?.file_path as string) ||
    (params?.path as string) ||
    (params?.filePath as string) ||
    (params?.target_file as string) ||
    '';

  const normalizedPath = rawFilePath.replace(/\\/g, '/');
  const fileName = normalizedPath.split('/').pop() || rawFilePath;
  const dirPath = normalizedPath.substring(0, normalizedPath.length - fileName.length).replace(/\/$/, '');

  const oldString =
    (params?.old_string as string | undefined) ??
    (params?.old_str as string | undefined) ??
    (params?.before as string | undefined);

  const newString =
    (params?.new_string as string | undefined) ??
    (params?.new_str as string | undefined) ??
    (params?.content as string | undefined);

  // Compute delta additions and deletions (+X -Y)
  let additions = 0;
  let deletions = 0;

  if (oldString !== undefined && oldString !== '') {
    deletions = oldString.split('\n').length;
  }
  if (newString !== undefined && newString !== '') {
    additions = newString.split('\n').length;
  }

  // Fallback for line counts if not computed from strings
  if (additions === 0 && deletions === 0) {
    if (typeof params?.lines === 'number') {
      additions = params.lines;
    } else if (tool.stdout) {
      const match = tool.stdout.match(/\+(\d+)\s*-(\d+)/);
      if (match) {
        additions = parseInt(match[1], 10);
        deletions = parseInt(match[2], 10);
      }
    }
  }

  const isFailed = isToolFailed(tool);

  // Label: "Created" (if new file) or "Edited" (if edit/overwrite)
  let opLabel: 'Created' | 'Edited' = 'Edited';
  if (isEditTool) {
    opLabel = 'Edited';
  } else {
    const op = (params?.operation as string)?.toLowerCase();
    if (op === 'create' || params?.is_new === true) {
      opLabel = 'Created';
    } else if (op === 'update' || deletions > 0 || params?.before !== undefined) {
      opLabel = 'Edited';
    } else {
      const stdoutLower = (tool.stdout ?? '').toLowerCase();
      if (stdoutLower.includes('created') || stdoutLower.includes('new file')) {
        opLabel = 'Created';
      } else if (stdoutLower.includes('update') || stdoutLower.includes('overwrit') || stdoutLower.includes('replaced')) {
        opLabel = 'Edited';
      } else {
        opLabel = deletions === 0 ? 'Created' : 'Edited';
      }
    }
  }

  // Select code icon vs generic text icon
  const ext = fileName.includes('.') ? fileName.split('.').pop()?.toLowerCase() : '';
  const isCode = ['ts', 'tsx', 'js', 'jsx', 'rs', 'py', 'json', 'toml', 'yaml', 'yml', 'xaml', 'xml', 'html', 'css', 'scss', 'sh', 'ps1', 'bat'].includes(ext || '');
  const DocIcon = isCode ? FileCode2 : FileText;
  const langBadge = getLanguageBadge(ext);

  // Line range calculations for read operations
  const offsetVal =
    params?.offset !== undefined && params?.offset !== null
      ? Number(params.offset)
      : undefined;
  const limitVal =
    params?.limit !== undefined && params?.limit !== null
      ? Number(params.limit)
      : undefined;
  const startLine =
    offsetVal !== undefined && !isNaN(offsetVal)
      ? Math.max(1, offsetVal)
      : limitVal !== undefined && !isNaN(limitVal)
      ? 1
      : undefined;
  const endLine =
    startLine !== undefined && limitVal !== undefined && !isNaN(limitVal)
      ? startLine + limitVal - 1
      : undefined;

  let lineRangeBadge: string | null = null;
  if (startLine !== undefined && endLine !== undefined) {
    lineRangeBadge = `#L${startLine}-${endLine}`;
  } else if (startLine !== undefined) {
    lineRangeBadge = `#L${startLine}+`;
  } else if (tool.stdout) {
    const stdoutLines = tool.stdout.split('\n').filter((l) => l.trim().length > 0).length;
    lineRangeBadge = `· ${stdoutLines} lines`;
  } else if (limitVal !== undefined) {
    lineRangeBadge = `· ${limitVal} lines`;
  }

  const durationText = formatDurationCompact(tool.durationMs);
  const isRunning = tool.status === 'running';

  const renderUnifiedDiff = () => {
    if (oldString !== undefined || newString !== undefined) {
      const oldLines = oldString ? oldString.split('\n') : [];
      const newLines = newString ? newString.split('\n') : [];

      return (
        <div className="rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-[#fafafa] dark:bg-[#0d1117] overflow-hidden font-mono text-[11px]">
          {rawFilePath && (
            <div className="flex items-center justify-between px-2.5 py-1.5 bg-[#f4f4f5] dark:bg-[#161b22] border-b border-[#ebebeb] dark:border-[#21262d] text-[#666666] dark:text-[#8b949e] text-[10px]">
              <span className="truncate max-w-[340px] text-[#171717] dark:text-[#f0f6fc] font-semibold">
                {rawFilePath}
              </span>
              <div className="flex items-center space-x-2 font-mono tabular-nums">
                {oldLines.length > 0 && (
                  <span className="text-[#dc2626] dark:text-[#ff7b72]">-{oldLines.length}</span>
                )}
                {newLines.length > 0 && (
                  <span className="text-[#16a34a] dark:text-[#7ee787]">+{newLines.length}</span>
                )}
                <span className="text-[#8f8f8f] dark:text-[#8b949e]">Различия</span>
              </div>
            </div>
          )}

          <div className="divide-y divide-[#ebebeb] dark:divide-[#21262d] select-text overflow-x-auto max-h-72 font-mono">
            {oldLines.map((line, idx) => (
              <div
                key={`old-${idx}`}
                className="flex items-start bg-[#fef2f2] dark:bg-[#3b1219] text-[#991b1b] dark:text-[#ff7b72] px-2 py-0.5 leading-relaxed font-mono"
              >
                <span className="w-4 shrink-0 text-[#ef4444] dark:text-[#ff7b72] select-none text-center font-bold">
                  -
                </span>
                <span className="w-8 shrink-0 text-[#8f8f8f] dark:text-[#8b949e] select-none text-right pr-2 tabular-nums">
                  {idx + 1}
                </span>
                <span className="whitespace-pre-wrap break-all">
                  {line || ' '}
                </span>
              </div>
            ))}

            {newLines.map((line, idx) => (
              <div
                key={`new-${idx}`}
                className="flex items-start bg-[#f0fdf4] dark:bg-[#12281e] text-[#166534] dark:text-[#7ee787] px-2 py-0.5 leading-relaxed font-mono"
              >
                <span className="w-4 shrink-0 text-[#22c55e] dark:text-[#7ee787] select-none text-center font-bold">
                  +
                </span>
                <span className="w-8 shrink-0 text-[#8f8f8f] dark:text-[#8b949e] select-none text-right pr-2 tabular-nums">
                  {idx + 1}
                </span>
                <span className="whitespace-pre-wrap break-all">
                  {line || ' '}
                </span>
              </div>
            ))}
          </div>
        </div>
      );
    }
    return null;
  };

  const renderFileContentPreview = () => {
    if (!newString) return null;
    const lines = newString.split('\n');

    return (
      <div className="rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-[#fafafa] dark:bg-[#0d1117] overflow-hidden font-mono text-[11px]">
        {rawFilePath && (
          <div className="flex items-center justify-between px-2.5 py-1.5 bg-[#f4f4f5] dark:bg-[#161b22] border-b border-[#ebebeb] dark:border-[#21262d] text-[#666666] dark:text-[#8b949e] text-[10px]">
            <span className="truncate max-w-[340px] text-[#171717] dark:text-[#f0f6fc] font-semibold">
              {rawFilePath}
            </span>
            <span className="text-[#8f8f8f] dark:text-[#8b949e] tabular-nums font-mono">
              {lines.length} {lines.length === 1 ? 'строка' : 'строк'}
            </span>
          </div>
        )}
        <div className="divide-y divide-[#f0f0f0] dark:divide-[#21262d] select-text overflow-x-auto max-h-72 font-mono bg-white dark:bg-[#0d1117]">
          {lines.map((line, idx) => (
            <div
              key={`line-${idx}`}
              className="flex items-start px-2 py-0.5 leading-relaxed hover:bg-[#fafafa] dark:hover:bg-[#161b22]"
            >
              <span className="w-8 shrink-0 text-[#a1a1aa] dark:text-[#8b949e] select-none text-right pr-2.5 tabular-nums text-[10px]">
                {idx + 1}
              </span>
              <span className="whitespace-pre-wrap break-all text-[#171717] dark:text-[#f0f6fc]">
                {line || ' '}
              </span>
            </div>
          ))}
        </div>
      </div>
    );
  };

  const renderReadContent = () => {
    if (!tool.stdout) return null;
    const rawLines = tool.stdout.split('\n');
    if (rawLines.length > 0 && rawLines[rawLines.length - 1] === '') {
      rawLines.pop();
    }
    if (rawLines.length === 0) return null;

    const baseLineNumber = startLine ?? 1;

    return (
      <div className="rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-[#fafafa] dark:bg-[#0d1117] overflow-hidden font-mono text-[11px]">
        {rawFilePath && (
          <div className="flex items-center justify-between px-2.5 py-1.5 bg-[#f4f4f5] dark:bg-[#161b22] border-b border-[#ebebeb] dark:border-[#21262d] text-[#666666] dark:text-[#8b949e] text-[10px]">
            <span className="truncate max-w-[340px] text-[#171717] dark:text-[#f0f6fc] font-semibold">
              {rawFilePath}
            </span>
            <span className="text-[#8f8f8f] dark:text-[#8b949e] tabular-nums font-mono">
              {lineRangeBadge || `${rawLines.length} строк`}
            </span>
          </div>
        )}
        <div className="divide-y divide-[#f0f0f0] dark:divide-[#21262d] select-text overflow-x-auto max-h-80 font-mono bg-white dark:bg-[#0d1117]">
          {rawLines.map((line, idx) => {
            const prefixMatch = line.match(/^(\d+)[:|]\s?(.*)$/);
            let displayLineNum: number | string = baseLineNumber + idx;
            let displayContent = line;

            if (prefixMatch) {
              displayLineNum = parseInt(prefixMatch[1], 10);
              displayContent = prefixMatch[2];
            }

            return (
              <div
                key={`readline-${idx}`}
                className="flex items-start px-2 py-0.5 leading-relaxed hover:bg-[#fafafa] dark:hover:bg-[#161b22]"
              >
                <span className="w-10 shrink-0 text-[#a1a1aa] dark:text-[#8b949e] select-none text-right pr-2.5 tabular-nums text-[10px]">
                  {displayLineNum}
                </span>
                <span className="whitespace-pre-wrap break-all text-[#171717] dark:text-[#f0f6fc]">
                  {displayContent || ' '}
                </span>
              </div>
            );
          })}
        </div>
      </div>
    );
  };

  return (
    <div
      className={`group rounded-md border text-xs shadow-2xs hover:shadow-xs overflow-hidden my-1 animate-silk-entrance transition-colors bg-white dark:bg-[#161b22] ${
        isRunning
          ? 'border-amber-500/40 dark:border-amber-500/30 bg-amber-500/[0.02]'
          : 'border-[#ebebeb] dark:border-[#21262d]'
      }`}
    >
      {/* Header Strip */}
      <div
        onClick={() => setIsExpanded(!isExpanded)}
        className={`flex items-center justify-between px-2.5 py-1.5 bg-[#f8f8f9] dark:bg-[#161b22] hover:bg-[#f2f2f3] dark:hover:bg-[#1c2128] transition-colors cursor-pointer select-none gap-2 text-xs min-h-[30px] ${
          isExpanded ? 'border-b border-[#ebebeb] dark:border-[#21262d]' : ''
        }`}
      >
        {isReadOp ? (
          /* Read operation tactile card: [Analyzed C# Transit.cs #L4270-4300] */
          <div className="flex items-center space-x-2 min-w-0 flex-1 overflow-hidden">
            <button
              type="button"
              className="p-0.5 text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] rounded shrink-0 transition cursor-pointer"
              onClick={(e) => {
                e.stopPropagation();
                setIsExpanded(!isExpanded);
              }}
              aria-label={isExpanded ? 'Свернуть подробности' : 'Развернуть подробности'}
            >
              <ChevronRight
                className={`h-3 w-3 transition-transform duration-150 ${
                  isExpanded ? 'rotate-90 text-[#171717] dark:text-[#f0f6fc]' : ''
                }`}
              />
            </button>

            {/* Label: Analyzed (cyan/teal tactile badge) */}
            <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-medium shrink-0 select-none bg-[#ecfeff] dark:bg-[#083344] text-[#0e7490] dark:text-[#22d3ee] border border-[#a5f3fc] dark:border-[#155e75]">
              Analyzed
            </span>

            {/* Brand SVG language badge or document icon */}
            {langBadge ? (
              <FileExtensionBadge ext={ext} />
            ) : (
              <DocIcon className="h-3.5 w-3.5 text-[#52525b] dark:text-[#8b949e] shrink-0" />
            )}

            {/* Clean filename with muted path */}
            <div className="flex items-center min-w-0 font-mono text-xs overflow-hidden">
              {dirPath && (
                <span className="text-[#8f8f8f] dark:text-[#8b949e] truncate max-w-[120px] sm:max-w-[180px] shrink-0">
                  {dirPath}/
                </span>
              )}
              <span className="font-semibold text-[#171717] dark:text-[#f0f6fc] truncate">
                {fileName || rawFilePath || 'file'}
              </span>
            </div>

            {/* Line range badge (#L4270-4300 or · 61 lines) */}
            {lineRangeBadge && (
              <span className="font-mono text-[11px] text-[#71717a] dark:text-[#8b949e] tabular-nums shrink-0 ml-1">
                {lineRangeBadge}
              </span>
            )}

            {durationText && (
              <>
                <span className="text-[#a1a1aa] dark:text-[#6e7681] select-none text-[10px]">•</span>
                <span className="font-mono text-[#71717a] dark:text-[#8b949e] shrink-0 tabular-nums text-[11px]">
                  {durationText}
                </span>
              </>
            )}
          </div>
        ) : (
          /* File write / edit operation card: [Edited MainWindow.xaml +4 -2] */
          <div className="flex items-center space-x-2 min-w-0 flex-1 overflow-hidden">
            <button
              type="button"
              className="p-0.5 text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] rounded shrink-0 transition cursor-pointer"
              onClick={(e) => {
                e.stopPropagation();
                setIsExpanded(!isExpanded);
              }}
              aria-label={isExpanded ? 'Свернуть подробности' : 'Развернуть подробности'}
            >
              <ChevronRight
                className={`h-3 w-3 transition-transform duration-150 ${
                  isExpanded ? 'rotate-90 text-[#171717] dark:text-[#f0f6fc]' : ''
                }`}
              />
            </button>

            {/* Label: Created or Edited or Failed edit */}
            <span
              className={`px-1.5 py-0.5 rounded text-[10px] font-mono font-medium shrink-0 select-none ${
                isFailed
                  ? 'bg-[#fef2f2] dark:bg-[#3b1219] text-[#b91c1c] dark:text-[#ff7b72] border border-[#fecaca] dark:border-[#da3633]'
                  : opLabel === 'Created'
                  ? 'bg-[#ecfdf5] dark:bg-[#12281e] text-[#047857] dark:text-[#7ee787] border border-[#a7f3d0] dark:border-[#238636]'
                  : 'bg-[#eff6ff] dark:bg-[#172554] text-[#1d4ed8] dark:text-[#60a5fa] border border-[#bfdbfe] dark:border-[#1e40af]'
              }`}
            >
              {isFailed ? 'Failed edit' : opLabel}
            </span>

            {/* Brand SVG language badge or document icon */}
            {langBadge ? (
              <FileExtensionBadge ext={ext} />
            ) : (
              <DocIcon className="h-3.5 w-3.5 text-[#52525b] dark:text-[#8b949e] shrink-0" />
            )}

            {/* Clean filename with muted path */}
            <div className="flex items-center min-w-0 font-mono text-xs overflow-hidden">
              {dirPath && (
                <span className="text-[#8f8f8f] dark:text-[#8b949e] truncate max-w-[120px] sm:max-w-[180px] shrink-0">
                  {dirPath}/
                </span>
              )}
              <span className="font-semibold text-[#171717] dark:text-[#f0f6fc] truncate">
                {fileName || rawFilePath || 'file'}
              </span>
            </div>

            {/* Delta badge: +X -Y */}
            {(additions > 0 || deletions > 0) && (
              <div className="flex items-center space-x-1 font-mono text-[11px] tabular-nums font-semibold shrink-0 ml-1">
                {additions > 0 && (
                  <span className="text-[#16a34a] dark:text-[#7ee787] bg-[#f0fdf4] dark:bg-[#12281e] px-1 py-0.2 rounded border border-[#bbf7d0] dark:border-[#238636]">
                    +{additions}
                  </span>
                )}
                {deletions > 0 && (
                  <span className="text-[#dc2626] dark:text-[#ff7b72] bg-[#fef2f2] dark:bg-[#3b1219] px-1 py-0.2 rounded border border-[#fecaca] dark:border-[#da3633]">
                    -{deletions}
                  </span>
                )}
              </div>
            )}

            {durationText && (
              <>
                <span className="text-[#a1a1aa] dark:text-[#6e7681] select-none text-[10px]">•</span>
                <span className="font-mono text-[#71717a] dark:text-[#8b949e] shrink-0 tabular-nums text-[11px]">
                  {durationText}
                </span>
              </>
            )}
          </div>
        )}

        <div className="flex items-center space-x-2 shrink-0">
          <CopyLogsButton tool={tool} isExpanded={isExpanded} />
          <StatusIndicator tool={tool} isFailed={isFailed} />
        </div>
      </div>

      {/* Expanded Details */}
      {isExpanded && (
        <div className="bg-white dark:bg-[#0d1117] p-3 border-t border-[#ebebeb] dark:border-[#21262d] space-y-2.5 font-mono text-xs">
          {isReadOp ? (
            <div className="space-y-2">
              {rawFilePath && (
                <div className="flex items-baseline space-x-2 py-0.5 text-[11px] font-mono">
                  <span className="text-[#666666] dark:text-[#8b949e] font-semibold shrink-0">Файл:</span>
                  <span className="text-[#171717] dark:text-[#f0f6fc] select-text break-all">{rawFilePath}</span>
                </div>
              )}
              {tool.stdout ? (
                <div>
                  <span className="text-[10px] text-[#666666] dark:text-[#8b949e] block mb-1">
                    Прочитанные строки {lineRangeBadge ? `(${lineRangeBadge})` : ''}
                  </span>
                  {renderReadContent()}
                </div>
              ) : (
                tool.params &&
                Object.keys(tool.params).length > 0 && <ToolParamsView params={params} />
              )}
            </div>
          ) : isFileOp ? (
            <div className="space-y-2">
              {rawFilePath && (
                <div className="flex items-baseline space-x-2 py-0.5 text-[11px] font-mono">
                  <span className="text-[#666666] dark:text-[#8b949e] font-semibold shrink-0">Файл:</span>
                  <span className="text-[#171717] dark:text-[#f0f6fc] select-text break-all">{rawFilePath}</span>
                </div>
              )}
              {oldString !== undefined && newString !== undefined ? (
                <div>
                  <span className="text-[10px] text-[#666666] dark:text-[#8b949e] block mb-1">
                    {isFailed ? 'Запланированные изменения (не применены)' : 'Дифф изменений'}
                  </span>
                  {renderUnifiedDiff()}
                </div>
              ) : newString ? (
                <div>
                  <span className="text-[10px] text-[#666666] dark:text-[#8b949e] block mb-1">
                    Содержимое файла
                  </span>
                  {renderFileContentPreview()}
                </div>
              ) : (
                tool.params &&
                Object.keys(tool.params).length > 0 && <ToolParamsView params={params} />
              )}
            </div>
          ) : isDiffTool && (oldString !== undefined || newString !== undefined) ? (
            <div className="space-y-2">
              {rawFilePath && (
                <div className="flex items-baseline space-x-2 py-0.5 text-[11px] font-mono">
                  <span className="text-[#666666] dark:text-[#8b949e] font-semibold shrink-0">Файл:</span>
                  <span className="text-[#171717] dark:text-[#f0f6fc] select-text break-all">{rawFilePath}</span>
                </div>
              )}
              <div>
                <span className="text-[10px] text-[#666666] dark:text-[#8b949e] block mb-1">
                  {isFailed ? 'Запланированные изменения (не применены)' : 'Дифф изменений'}
                </span>
                {renderUnifiedDiff()}
              </div>
            </div>
          ) : (
            tool.params &&
            Object.keys(tool.params).length > 0 && <ToolParamsView params={params} />
          )}

          {!isReadOp && tool.stdout && (
            <div>
              <span className="text-[10px] text-[#666666] dark:text-[#8b949e] block mb-1">
                Вывод команды
              </span>
              <pre className="p-2.5 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#21262d] text-[11px] text-[#171717] dark:text-[#f0f6fc] overflow-x-auto leading-relaxed max-h-48 whitespace-pre-wrap select-text">
                {tool.stdout}
              </pre>
            </div>
          )}

          {tool.stderr && (
            <div>
              <span className="text-[10px] text-[#b91c1c] dark:text-[#ff7b72] block mb-1">
                Ошибки выполнения
              </span>
              <pre className="p-2.5 rounded-md bg-[#fef2f2] dark:bg-[#3b1219] border border-[#fecaca] dark:border-[#da3633] text-[11px] text-[#991b1b] dark:text-[#ff7b72] overflow-x-auto leading-relaxed max-h-48 whitespace-pre-wrap select-text">
                {tool.stderr}
              </pre>
            </div>
          )}

          <div className="flex items-center justify-between pt-1 border-t border-[#f4f4f5] dark:border-[#21262d] text-[10px] text-[#8f8f8f] dark:text-[#8b949e]">
            <span>Статус: {isFailed ? `failed (код ${tool.exitCode ?? 1})` : tool.status}</span>
          </div>
        </div>
      )}
    </div>
  );
}

import { useState } from 'react';
import { ToolExecution } from '../../types';
import {
  ChevronRight,
  Copy,
  Check,
  X,
  Wrench,
} from 'lucide-react';

interface ToolExecutionCardProps {
  tool: ToolExecution;
}

export default function ToolExecutionCard({ tool }: ToolExecutionCardProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [copied, setCopied] = useState(false);

  if (!tool) return null;

  const toolName: string = (tool as ToolExecution)?.toolName ?? 'tool';
  const params: Record<string, unknown> =
    (tool as ToolExecution)?.params && typeof (tool as ToolExecution).params === 'object'
      ? ((tool as ToolExecution).params as Record<string, unknown>)
      : {};

  const isDiffTool =
    toolName === 'edit_file' ||
    toolName === 'str_replace' ||
    toolName === 'edit';

  const filePath =
    (params?.file_path as string) || (params?.path as string);
  const oldString = params?.old_string as string | undefined;
  const newString = params?.new_string as string | undefined;

  const getTarget = (): { target: string; fullTarget: string } => {
    const p = params ?? {};
    const str = (v: unknown): string | undefined =>
      typeof v === 'string' && v.trim().length > 0 ? v.trim() : undefined;

    let raw = '';
    switch (tool.toolName) {
      case 'write':
      case 'write_file':
      case 'read':
      case 'read_file':
      case 'edit':
      case 'edit_file':
      case 'str_replace':
        raw = str(p.file_path) || str(p.path) || '';
        break;
      case 'glob':
        raw = str(p.pattern) || '';
        break;
      case 'grep': {
        const pattern = str(p.pattern) || '';
        const path = str(p.path);
        raw = path && path !== '.' ? `${pattern} in ${path}` : pattern;
        break;
      }
      case 'execute_command':
      case 'bash':
      case 'pwsh':
        raw = str(p.command) || str(p.cmd) || '';
        break;
      case 'web_search':
        raw = str(p.query) || '';
        break;
      case 'web_fetch':
        raw = str(p.url) || '';
        break;
      case 'subagent':
        raw = str(p.role) || str(p.description) || str(p.prompt) || '';
        break;
      default: {
        const firstStr = Object.values(p).find(
          (v) => typeof v === 'string' && (v as string).trim().length > 0
        ) as string | undefined;
        raw = firstStr || '';
        break;
      }
    }

    const fullTarget = raw;
    const target = raw.length > 55 ? `${raw.slice(0, 52)}…` : raw;
    return { target, fullTarget };
  };

  const getExtraInfo = (): string | null => {
    const p = tool.params ?? {};
    switch (tool.toolName) {
      case 'write':
      case 'write_file': {
        if (typeof p.content === 'string') {
          const lineCount = p.content.split('\n').length;
          return `${lineCount} lines`;
        }
        if (typeof p.lines === 'number') {
          return `${p.lines} lines`;
        }
        if (tool.stdout) {
          const lines = tool.stdout.split('\n').length;
          if (lines > 1) return `${lines} lines`;
        }
        return null;
      }
      case 'edit':
      case 'edit_file':
      case 'str_replace': {
        const newStr = typeof p.new_string === 'string' ? p.new_string : '';
        const oldStr = typeof p.old_string === 'string' ? p.old_string : '';
        if (newStr || oldStr) {
          const changedLines = Math.max(
            newStr ? newStr.split('\n').length : 0,
            oldStr ? oldStr.split('\n').length : 0
          );
          return `${changedLines} lines`;
        }
        return null;
      }
      case 'read':
      case 'read_file': {
        if (tool.stdout) {
          const lines = tool.stdout.split('\n').length;
          return `${lines} lines`;
        }
        if (typeof p.limit === 'number') {
          return `${p.limit} lines`;
        }
        return null;
      }
      case 'glob': {
        if (tool.stdout) {
          const count = tool.stdout
            .split('\n')
            .filter((l) => l.trim().length > 0).length;
          return `${count} files`;
        }
        return null;
      }
      case 'grep': {
        if (tool.stdout) {
          const count = tool.stdout
            .split('\n')
            .filter((l) => l.trim().length > 0).length;
          return `${count} matches`;
        }
        return null;
      }
      case 'execute_command':
      case 'bash':
      case 'pwsh': {
        if (tool.status === 'success') {
          return 'exit 0';
        }
        if (tool.status === 'failed') {
          return `exit ${tool.exitCode ?? 1}`;
        }
        return null;
      }
      default: {
        if (tool.stdout) {
          const lines = tool.stdout.split('\n').length;
          if (lines > 1) return `${lines} lines`;
        }
        return null;
      }
    }
  };

  const formatDuration = (ms: number): string | null => {
    if (!ms || ms <= 0) return null;
    if (ms >= 1000) {
      return `${(ms / 1000).toFixed(1)}s`;
    }
    return `${ms}ms`;
  };

  const { target, fullTarget } = getTarget();
  const extraInfo = getExtraInfo();
  const durationText = formatDuration(tool.durationMs);

  const handleCopyLogs = (e: React.MouseEvent) => {
    e.stopPropagation();
    const content = [
      `Инструмент: ${tool.toolName}`,
      `Статус: ${tool.status} (код ${tool.exitCode ?? 0})`,
      tool.params && Object.keys(tool.params).length > 0
        ? `ПАРАМЕТРЫ:
${JSON.stringify(tool.params, null, 2)}`
        : '',
      tool.stdout ? `STDOUT:
${tool.stdout}` : '',
      tool.stderr ? `STDERR:
${tool.stderr}` : '',
    ]
      .filter(Boolean)
      .join('\n\n');

    navigator.clipboard.writeText(content);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const renderStatusIndicator = () => {
    if (tool.status === 'running') {
      return (
        <span
          className="relative flex h-2 w-2 shrink-0"
          title="Выполняется..."
        >
          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-amber-400 opacity-75" />
          <span className="relative inline-flex rounded-full h-2 w-2 bg-amber-500" />
        </span>
      );
    }
    if (tool.status === 'success') {
      return (
        <span title={`Успешно (код ${tool.exitCode ?? 0})`} className="flex items-center shrink-0">
          <Check className="h-3.5 w-3.5 text-emerald-600 stroke-[2.5]" />
        </span>
      );
    }
    return (
      <span title={`Ошибка (код ${tool.exitCode ?? 1})`} className="flex items-center shrink-0">
        <X className="h-3.5 w-3.5 text-rose-600 stroke-[2.5]" />
      </span>
    );
  };

  const renderUnifiedDiff = () => {
    if (oldString !== undefined || newString !== undefined) {
      const oldLines = oldString ? oldString.split('\n') : [];
      const newLines = newString ? newString.split('\n') : [];

      return (
        <div className="rounded-md border border-[#ebebeb] bg-[#fafafa] overflow-hidden font-mono text-[11px]">
          {filePath && (
            <div className="flex items-center justify-between px-2.5 py-1 bg-[#f4f4f5] border-b border-[#ebebeb] text-[#666666] text-[10px]">
              <span className="truncate max-w-[340px] text-[#171717]">
                {filePath}
              </span>
              <span className="text-[#8f8f8f]">Различия</span>
            </div>
          )}

          <div className="divide-y divide-[#ebebeb] select-text overflow-x-auto max-h-64 font-mono">
            {oldLines.map((line, idx) => (
              <div
                key={`old-${idx}`}
                className="flex items-start bg-[#fef2f2] text-[#991b1b] px-2 py-0.5 leading-relaxed"
              >
                <span className="w-4 shrink-0 text-[#ef4444] select-none text-center">
                  -
                </span>
                <span className="w-8 shrink-0 text-[#8f8f8f] select-none text-right pr-2 tabular-nums">
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
                className="flex items-start bg-[#f0fdf4] text-[#166534] px-2 py-0.5 leading-relaxed"
              >
                <span className="w-4 shrink-0 text-[#22c55e] select-none text-center">
                  +
                </span>
                <span className="w-8 shrink-0 text-[#8f8f8f] select-none text-right pr-2 tabular-nums">
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

  return (
    <div className="group rounded-md border border-[#ebebeb] bg-[#fafafa] hover:border-[#d4d4d8] overflow-hidden text-xs shadow-2xs transition-colors my-1">
      {/* Ultra-compact single-line strip */}
      <div
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center justify-between px-2.5 py-1.5 hover:bg-[#f4f4f5] cursor-pointer transition select-none gap-2 text-xs min-h-[30px]"
      >
        <div className="flex items-center space-x-1.5 min-w-0 flex-1 overflow-hidden">
          <button
            type="button"
            className="p-0.5 text-[#8f8f8f] hover:text-[#171717] rounded shrink-0 transition cursor-pointer"
            onClick={(e) => {
              e.stopPropagation();
              setIsExpanded(!isExpanded);
            }}
            title={isExpanded ? 'Свернуть подробности' : 'Развернуть подробности'}
          >
            <ChevronRight
              className={`h-3 w-3 transition-transform duration-150 ${
                isExpanded ? 'rotate-90 text-[#171717]' : ''
              }`}
            />
          </button>

          <span
            className="flex items-center shrink-0 select-none text-[#8f8f8f]"
            title="Инструмент"
          >
            <Wrench className="h-3.5 w-3.5" />
          </span>

          <span className="font-mono font-medium text-[#171717] shrink-0">
            {tool.toolName}{target ? ':' : ''}
          </span>

          {target && (
            <span
              title={fullTarget}
              className="font-mono text-[#52525b] truncate max-w-[260px] sm:max-w-[360px] md:max-w-[460px]"
            >
              {target}
            </span>
          )}

          {extraInfo && (
            <>
              <span className="text-[#a1a1aa] select-none text-[10px]">•</span>
              <span className="font-mono text-[#71717a] shrink-0 tabular-nums text-[11px]">
                {extraInfo}
              </span>
            </>
          )}

          {durationText && (
            <>
              <span className="text-[#a1a1aa] select-none text-[10px]">•</span>
              <span className="font-mono text-[#71717a] shrink-0 tabular-nums text-[11px]">
                {durationText}
              </span>
            </>
          )}
        </div>

        <div className="flex items-center space-x-2 shrink-0">
          <button
            type="button"
            onClick={handleCopyLogs}
            className={`p-1 rounded text-[#8f8f8f] hover:text-[#171717] hover:bg-white transition cursor-pointer ${
              isExpanded ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'
            }`}
            title="Скопировать вывод"
          >
            {copied ? (
              <Check className="h-3 w-3 text-emerald-600" />
            ) : (
              <Copy className="h-3 w-3" />
            )}
          </button>

          {renderStatusIndicator()}
        </div>
      </div>

      {/* Expanded Details */}
      {isExpanded && (
        <div className="p-3 border-t border-[#ebebeb] space-y-2.5 font-mono text-xs bg-white">
          {isDiffTool && (oldString !== undefined || newString !== undefined) ? (
            <div>
              <span className="text-[10px] text-[#666666] block mb-1">
                Изменения файлов (Diff)
              </span>
              {renderUnifiedDiff()}
            </div>
          ) : (
            tool.params &&
            Object.keys(tool.params).length > 0 && (
              <div>
                <span className="text-[10px] text-[#666666] block mb-1">
                  Параметры вызова
                </span>
                <pre className="p-2.5 rounded-md bg-[#fafafa] border border-[#ebebeb] text-[11px] text-[#171717] overflow-x-auto leading-relaxed">
                  {JSON.stringify(tool.params, null, 2)}
                </pre>
              </div>
            )
          )}

          {tool.stdout && (
            <div>
              <span className="text-[10px] text-[#666666] block mb-1">
                Стандартный вывод (stdout)
              </span>
              <pre className="p-2.5 rounded-md bg-[#fafafa] border border-[#ebebeb] text-[11px] text-[#171717] overflow-x-auto leading-relaxed max-h-48 whitespace-pre-wrap select-text">
                {tool.stdout}
              </pre>
            </div>
          )}

          {tool.stderr && (
            <div>
              <span className="text-[10px] text-[#b91c1c] block mb-1">
                Трассировка ошибок (stderr)
              </span>
              <pre className="p-2.5 rounded-md bg-[#fef2f2] border border-[#fecaca] text-[11px] text-[#991b1b] overflow-x-auto leading-relaxed max-h-48 whitespace-pre-wrap select-text">
                {tool.stderr}
              </pre>
            </div>
          )}

          <div className="flex items-center justify-between pt-1 border-t border-[#f4f4f5] text-[10px] text-[#8f8f8f]">
            <span>Статус: {tool.status} {tool.exitCode !== undefined ? `(код ${tool.exitCode})` : ''}</span>
            <span className="tabular-nums">{tool.timestamp}</span>
          </div>
        </div>
      )}
    </div>
  );
}

import { useState } from 'react';
import { ToolExecution } from '../../types';
import {
  ChevronDown,
  ChevronRight,
  Terminal,
  Clock,
  Code2,
  Copy,
  Check,
  FileEdit,
  Globe,
  FileText,
} from 'lucide-react';

interface ToolExecutionCardProps {
  tool: ToolExecution;
}

export default function ToolExecutionCard({ tool }: ToolExecutionCardProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [copied, setCopied] = useState(false);

  const getToolIcon = (name: string) => {
    switch (name) {
      case 'execute_command':
        return <Terminal className="h-3.5 w-3.5 text-[#666666]" />;
      case 'edit_file':
      case 'str_replace':
      case 'write':
      case 'write_file':
        return <FileEdit className="h-3.5 w-3.5 text-[#666666]" />;
      case 'read':
      case 'read_file':
        return <FileText className="h-3.5 w-3.5 text-[#666666]" />;
      case 'web_search':
        return <Globe className="h-3.5 w-3.5 text-[#666666]" />;
      default:
        return <Code2 className="h-3.5 w-3.5 text-[#666666]" />;
    }
  };

  const isDiffTool = tool.toolName === 'edit_file' || tool.toolName === 'str_replace';
  const filePath = (tool.params?.file_path as string) || (tool.params?.path as string);
  const oldString = tool.params?.old_string as string | undefined;
  const newString = tool.params?.new_string as string | undefined;

  const truncate = (s: string, max: number) =>
    s.length > max ? `${s.slice(0, max)}…` : s;

  const getSummaryFull = (): string | undefined => {
    const p = tool.params ?? {};
    const str = (v: unknown): string | undefined =>
      typeof v === 'string' && v.length > 0 ? v : undefined;
    switch (tool.toolName) {
      case 'read_file':
      case 'read':
      case 'write_file':
      case 'write':
      case 'edit_file':
      case 'str_replace':
        return str(p.file_path) ?? str(p.path);
      case 'glob':
        return str(p.pattern);
      case 'grep': {
        const pattern = str(p.pattern);
        if (!pattern) return undefined;
        const path = str(p.path);
        return path && path !== '.' ? `${pattern} в ${path}` : pattern;
      }
      case 'execute_command':
      case 'bash':
        return str(p.command) ?? str(p.cmd);
      case 'web_search':
        return str(p.query);
      case 'web_fetch':
        return str(p.url);
      case 'subagent':
        return str(p.role);
      default: {
        const first = Object.values(p).find((v) => typeof v === 'string' && (v as string).length > 0) as string | undefined;
        return first;
      }
    }
  };

  const summaryFull = getSummaryFull();
  const summary =
    summaryFull === undefined
      ? undefined
      : tool.toolName === 'execute_command' || tool.toolName === 'bash'
        ? truncate(summaryFull, 90)
        : !['read_file', 'read', 'write_file', 'write', 'edit_file', 'str_replace', 'glob', 'grep', 'web_search', 'web_fetch', 'subagent'].includes(tool.toolName)
          ? truncate(summaryFull, 80)
          : summaryFull;

  const handleCopyLogs = (e: React.MouseEvent) => {
    e.stopPropagation();
    const content = [
      `Инструмент: ${tool.toolName}`,
      `Статус: ${tool.status} (код ${tool.exitCode ?? 0})`,
      tool.stdout ? `STDOUT:\n${tool.stdout}` : '',
      tool.stderr ? `STDERR:\n${tool.stderr}` : '',
    ]
      .filter(Boolean)
      .join('\n\n');

    navigator.clipboard.writeText(content);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const formatDurationMs = (ms: number) => {
    if (ms >= 1000) {
      return `${(ms / 1000).toFixed(2)}с`;
    }
    return `${ms}мс`;
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
                <span className="w-4 shrink-0 text-[#ef4444] select-none text-center">-</span>
                <span className="w-8 shrink-0 text-[#8f8f8f] select-none text-right pr-2 tabular-nums">
                  {idx + 1}
                </span>
                <span className="whitespace-pre-wrap break-all">{line || ' '}</span>
              </div>
            ))}

            {newLines.map((line, idx) => (
              <div
                key={`new-${idx}`}
                className="flex items-start bg-[#f0fdf4] text-[#166534] px-2 py-0.5 leading-relaxed"
              >
                <span className="w-4 shrink-0 text-[#22c55e] select-none text-center">+</span>
                <span className="w-8 shrink-0 text-[#8f8f8f] select-none text-right pr-2 tabular-nums">
                  {idx + 1}
                </span>
                <span className="whitespace-pre-wrap break-all">{line || ' '}</span>
              </div>
            ))}
          </div>
        </div>
      );
    }

    return null;
  };

  return (
    <div className="rounded-md border border-[#ebebeb] bg-white overflow-hidden text-xs shadow-xs">
      {/* Tool Header */}
      <div
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center justify-between px-3 py-2 bg-[#f9f9f9] hover:bg-[#f4f4f5] cursor-pointer transition select-none"
      >
        <div className="flex items-center space-x-2 min-w-0 flex-1">
          {isExpanded ? (
            <ChevronDown className="h-3.5 w-3.5 text-[#666666] shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-[#666666] shrink-0" />
          )}

          <div className="p-0.5 shrink-0">
            {getToolIcon(tool.toolName)}
          </div>

          <span className="font-mono font-medium text-[#171717] shrink-0">
            {tool.toolName}
          </span>

          {summary && (
            <span
              title={summaryFull}
              className="font-mono text-[#8f8f8f] truncate max-w-[320px]"
            >
              {summary}
            </span>
          )}
        </div>

        <div className="flex items-center space-x-2.5 shrink-0 text-[10px] font-mono">
          <span className="flex items-center space-x-1 text-[#666666] tabular-nums">
            <Clock className="h-3 w-3" />
            <span>{formatDurationMs(tool.durationMs)}</span>
          </span>

          {tool.status === 'success' && (
            <span className="text-[#15803d] font-medium tabular-nums">
              код: {tool.exitCode ?? 0}
            </span>
          )}

          {tool.status === 'failed' && (
            <span className="text-[#b91c1c] font-medium tabular-nums">
              код: {tool.exitCode ?? 1}
            </span>
          )}

          {tool.status === 'running' && (
            <span className="text-[#b45309] font-medium">
              выполняется...
            </span>
          )}

          <button
            onClick={handleCopyLogs}
            className="p-1 rounded-md text-[#666666] hover:text-[#171717] hover:bg-white transition cursor-pointer"
            title="Скопировать вывод"
          >
            {copied ? <Check className="h-3.5 w-3.5 text-[#15803d]" /> : <Copy className="h-3.5 w-3.5" />}
          </button>
        </div>
      </div>

      {/* Expanded Body */}
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
              <pre className="p-2.5 rounded-md bg-[#fafafa] border border-[#ebebeb] text-[11px] text-[#171717] overflow-x-auto leading-relaxed max-h-40 whitespace-pre-wrap select-text">
                {tool.stdout}
              </pre>
            </div>
          )}

          {tool.stderr && (
            <div>
              <span className="text-[10px] text-[#b91c1c] block mb-1">
                Трассировка ошибок (stderr)
              </span>
              <pre className="p-2.5 rounded-md bg-[#fef2f2] border border-[#fecaca] text-[11px] text-[#991b1b] overflow-x-auto leading-relaxed max-h-40 whitespace-pre-wrap select-text">
                {tool.stderr}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

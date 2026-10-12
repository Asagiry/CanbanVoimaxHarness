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
  const [isExpanded, setIsExpanded] = useState(true);
  const [copied, setCopied] = useState(false);

  const getToolIcon = (name: string) => {
    switch (name) {
      case 'execute_command':
        return <Terminal className="h-3 w-3 text-slate-400" />;
      case 'edit_file':
      case 'str_replace':
      case 'write':
      case 'write_file':
        return <FileEdit className="h-3 w-3 text-slate-400" />;
      case 'read':
      case 'read_file':
        return <FileText className="h-3 w-3 text-slate-400" />;
      case 'web_search':
        return <Globe className="h-3 w-3 text-slate-400" />;
      default:
        return <Code2 className="h-3 w-3 text-slate-400" />;
    }
  };

  const isDiffTool = tool.toolName === 'edit_file' || tool.toolName === 'str_replace';
  const filePath = (tool.params?.file_path as string) || (tool.params?.path as string);
  const oldString = tool.params?.old_string as string | undefined;
  const newString = tool.params?.new_string as string | undefined;

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
        <div className="rounded border border-obsidian-800 bg-obsidian-950 overflow-hidden font-mono text-[11px]">
          {filePath && (
            <div className="flex items-center justify-between px-2.5 py-1 bg-obsidian-900 border-b border-obsidian-800 text-slate-400 text-[10px]">
              <span className="truncate max-w-[340px] text-slate-300">
                {filePath}
              </span>
              <span className="text-slate-500">Различия</span>
            </div>
          )}

          <div className="divide-y divide-obsidian-850 select-text overflow-x-auto max-h-64 font-mono">
            {oldLines.map((line, idx) => (
              <div
                key={`old-${idx}`}
                className="flex items-start bg-rose-950/20 text-rose-300 px-2 py-0.5 leading-relaxed"
              >
                <span className="w-4 shrink-0 text-rose-500 select-none text-center">-</span>
                <span className="w-8 shrink-0 text-slate-600 select-none text-right pr-2 tabular-nums">
                  {idx + 1}
                </span>
                <span className="whitespace-pre-wrap break-all">{line || ' '}</span>
              </div>
            ))}

            {newLines.map((line, idx) => (
              <div
                key={`new-${idx}`}
                className="flex items-start bg-emerald-950/20 text-emerald-300 px-2 py-0.5 leading-relaxed"
              >
                <span className="w-4 shrink-0 text-emerald-500 select-none text-center">+</span>
                <span className="w-8 shrink-0 text-slate-600 select-none text-right pr-2 tabular-nums">
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
    <div className="rounded border border-obsidian-800 bg-obsidian-900 overflow-hidden text-xs">
      {/* Tool Header */}
      <div
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center justify-between px-3 py-1.5 bg-obsidian-850 hover:bg-obsidian-800 cursor-pointer transition select-none"
      >
        <div className="flex items-center space-x-2 min-w-0">
          {isExpanded ? (
            <ChevronDown className="h-3 w-3 text-slate-400 shrink-0" />
          ) : (
            <ChevronRight className="h-3 w-3 text-slate-400 shrink-0" />
          )}

          <div className="p-0.5 shrink-0">
            {getToolIcon(tool.toolName)}
          </div>

          <span className="font-mono font-medium text-slate-300 truncate">
            {tool.toolName}
          </span>
        </div>

        <div className="flex items-center space-x-2 shrink-0 text-[10px] font-mono">
          <span className="flex items-center space-x-1 text-slate-500 tabular-nums">
            <Clock className="h-2.5 w-2.5" />
            <span>{formatDurationMs(tool.durationMs)}</span>
          </span>

          {tool.status === 'success' && (
            <span className="text-emerald-400 tabular-nums">
              код: {tool.exitCode ?? 0}
            </span>
          )}

          {tool.status === 'failed' && (
            <span className="text-rose-400 tabular-nums">
              код: {tool.exitCode ?? 1}
            </span>
          )}

          {tool.status === 'running' && (
            <span className="text-amber-400">
              выполняется...
            </span>
          )}

          <button
            onClick={handleCopyLogs}
            className="p-1 rounded text-slate-500 hover:text-white transition cursor-pointer"
            title="Скопировать вывод"
          >
            {copied ? <Check className="h-3 w-3 text-emerald-400" /> : <Copy className="h-3 w-3" />}
          </button>
        </div>
      </div>

      {/* Expanded Body */}
      {isExpanded && (
        <div className="p-2.5 border-t border-obsidian-800 space-y-2 font-mono text-xs bg-obsidian-950">
          {isDiffTool && (oldString !== undefined || newString !== undefined) ? (
            <div>
              <span className="text-[10px] text-slate-500 block mb-1">
                Изменения файлов (Diff)
              </span>
              {renderUnifiedDiff()}
            </div>
          ) : (
            tool.params &&
            Object.keys(tool.params).length > 0 && (
              <div>
                <span className="text-[10px] text-slate-500 block mb-1">
                  Параметры вызова
                </span>
                <pre className="p-2 rounded bg-obsidian-900 border border-obsidian-800 text-[11px] text-slate-300 overflow-x-auto leading-relaxed">
                  {JSON.stringify(tool.params, null, 2)}
                </pre>
              </div>
            )
          )}

          {tool.stdout && (
            <div>
              <span className="text-[10px] text-slate-500 block mb-1">
                Стандартный вывод (stdout)
              </span>
              <pre className="p-2 rounded bg-obsidian-900 border border-obsidian-800 text-[11px] text-slate-300 overflow-x-auto leading-relaxed max-h-40 whitespace-pre-wrap select-text">
                {tool.stdout}
              </pre>
            </div>
          )}

          {tool.stderr && (
            <div>
              <span className="text-[10px] text-rose-400 block mb-1">
                Трассировка ошибок (stderr)
              </span>
              <pre className="p-2 rounded bg-rose-950/20 border border-rose-900/40 text-[11px] text-rose-300 overflow-x-auto leading-relaxed max-h-40 whitespace-pre-wrap select-text">
                {tool.stderr}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

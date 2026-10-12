import { useState } from 'react';
import { ToolExecution } from '../../types';
import {
  ChevronDown,
  ChevronRight,
  Terminal,
  CheckCircle2,
  XCircle,
  Clock,
  Code2,
  Copy,
  Check,
  FileEdit,
  Globe,
  FileText,
  FileCode,
  GitCompare,
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
        return <Terminal className="h-3.5 w-3.5 text-cyber-cyan" />;
      case 'edit_file':
      case 'str_replace':
      case 'write':
      case 'write_file':
        return <FileEdit className="h-3.5 w-3.5 text-cyber-amber" />;
      case 'read':
      case 'read_file':
        return <FileText className="h-3.5 w-3.5 text-slate-300" />;
      case 'web_search':
        return <Globe className="h-3.5 w-3.5 text-cyber-purple" />;
      default:
        return <Code2 className="h-3.5 w-3.5 text-slate-400" />;
    }
  };

  const isDiffTool = tool.toolName === 'edit_file' || tool.toolName === 'str_replace';
  const filePath = (tool.params?.file_path as string) || (tool.params?.path as string);
  const oldString = tool.params?.old_string as string | undefined;
  const newString = tool.params?.new_string as string | undefined;

  const handleCopyLogs = (e: React.MouseEvent) => {
    e.stopPropagation();
    const content = [
      `Tool: ${tool.toolName}`,
      `Status: ${tool.status} (exit ${tool.exitCode ?? 0})`,
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
      return `${(ms / 1000).toFixed(2)}s`;
    }
    return `${ms}ms`;
  };

  const renderUnifiedDiff = () => {
    if (oldString !== undefined || newString !== undefined) {
      const oldLines = oldString ? oldString.split('\n') : [];
      const newLines = newString ? newString.split('\n') : [];

      return (
        <div className="rounded-md border border-obsidian-800 bg-obsidian-950 overflow-hidden font-mono text-[11px]">
          {filePath && (
            <div className="flex items-center justify-between px-2.5 py-1 bg-obsidian-900 border-b border-obsidian-800 text-slate-400 text-[10px]">
              <span className="flex items-center space-x-1.5 text-cyber-cyan font-semibold truncate max-w-[320px]">
                <FileCode className="h-3 w-3 shrink-0" />
                <span className="truncate">{filePath}</span>
              </span>
              <span className="flex items-center space-x-1 text-slate-500 shrink-0">
                <GitCompare className="h-3 w-3 text-cyber-amber shrink-0" />
                <span>Unified Diff</span>
              </span>
            </div>
          )}

          <div className="divide-y divide-obsidian-850 select-text overflow-x-auto max-h-64 font-mono">
            {/* Removed lines (-) with gutter */}
            {oldLines.map((line, idx) => (
              <div
                key={`old-${idx}`}
                className="flex items-start bg-rose-950/25 hover:bg-rose-950/40 text-rose-300 border-l-2 border-rose-500/80 px-2 py-0.5 leading-relaxed"
              >
                <span className="w-4 shrink-0 text-rose-500/80 select-none text-center font-bold">
                  -
                </span>
                <span className="w-8 shrink-0 text-slate-600 select-none text-right pr-2 tabular-nums">
                  {idx + 1}
                </span>
                <span className="whitespace-pre-wrap break-all">{line || ' '}</span>
              </div>
            ))}

            {/* Added lines (+) with gutter */}
            {newLines.map((line, idx) => (
              <div
                key={`new-${idx}`}
                className="flex items-start bg-emerald-950/25 hover:bg-emerald-950/40 text-emerald-300 border-l-2 border-emerald-500/80 px-2 py-0.5 leading-relaxed"
              >
                <span className="w-4 shrink-0 text-emerald-500/80 select-none text-center font-bold">
                  +
                </span>
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
    <div className="rounded-md border border-obsidian-800 bg-obsidian-900/95 overflow-hidden shadow-xs">
      {/* Tool Header */}
      <div
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center justify-between px-3 py-1.5 bg-obsidian-850 hover:bg-obsidian-800 cursor-pointer transition select-none text-xs"
      >
        <div className="flex items-center space-x-2 min-w-0">
          {isExpanded ? (
            <ChevronDown className="h-3 w-3 text-slate-400 shrink-0" />
          ) : (
            <ChevronRight className="h-3 w-3 text-slate-400 shrink-0" />
          )}

          <div className="p-1 rounded-sm bg-obsidian-800 border border-obsidian-750 shrink-0">
            {getToolIcon(tool.toolName)}
          </div>

          <span className="font-mono font-semibold text-slate-200 truncate">
            {tool.toolName}
          </span>
        </div>

        <div className="flex items-center space-x-2 shrink-0">
          {/* Duration */}
          <span className="flex items-center space-x-1 font-mono text-[10px] tabular-nums text-slate-400">
            <Clock className="h-2.5 w-2.5 text-slate-500" />
            <span>{formatDurationMs(tool.durationMs)}</span>
          </span>

          {/* Exit code / status badge */}
          {tool.status === 'success' && (
            <span className="flex items-center space-x-1 px-1.5 py-0.2 rounded font-mono text-[10px] font-bold bg-emerald-950/30 border border-emerald-500/40 text-cyber-emerald tabular-nums">
              <CheckCircle2 className="h-2.5 w-2.5" />
              <span>exit: {tool.exitCode ?? 0}</span>
            </span>
          )}

          {tool.status === 'failed' && (
            <span className="flex items-center space-x-1 px-1.5 py-0.2 rounded font-mono text-[10px] font-bold bg-rose-950/30 border border-rose-500/40 text-cyber-crimson tabular-nums">
              <XCircle className="h-2.5 w-2.5" />
              <span>exit: {tool.exitCode ?? 1}</span>
            </span>
          )}

          {tool.status === 'running' && (
            <span className="flex items-center space-x-1 px-1.5 py-0.2 rounded font-mono text-[10px] font-bold bg-amber-950/30 border border-amber-500/40 text-cyber-amber animate-pulse">
              <span>executing...</span>
            </span>
          )}

          {/* Copy Button */}
          <button
            onClick={handleCopyLogs}
            className="p-1 rounded-sm text-slate-400 hover:text-white hover:bg-obsidian-750 transition cursor-pointer"
            title="Copy logs"
          >
            {copied ? <Check className="h-3 w-3 text-cyber-emerald" /> : <Copy className="h-3 w-3" />}
          </button>
        </div>
      </div>

      {/* Expanded Body */}
      {isExpanded && (
        <div className="p-2.5 border-t border-obsidian-800 space-y-2 font-mono text-xs">
          {/* Visual Diff Viewer for edit_file / str_replace */}
          {isDiffTool && (oldString !== undefined || newString !== undefined) ? (
            <div>
              <span className="text-[10px] font-mono font-bold text-cyber-cyan block mb-1">
                Visual Diff Inspector
              </span>
              {renderUnifiedDiff()}
            </div>
          ) : (
            /* Standard Tool Parameters */
            tool.params &&
            Object.keys(tool.params).length > 0 && (
              <div>
                <span className="text-[10px] font-mono font-bold text-slate-500 block mb-1">
                  Input Parameters
                </span>
                <pre className="p-2 rounded bg-obsidian-950 border border-obsidian-800 text-[11px] text-slate-300 overflow-x-auto leading-relaxed">
                  {JSON.stringify(tool.params, null, 2)}
                </pre>
              </div>
            )
          )}

          {/* Terminal STDOUT */}
          {tool.stdout && (
            <div>
              <span className="text-[10px] font-mono font-bold text-slate-500 block mb-1">
                Stdout
              </span>
              <pre className="p-2 rounded bg-obsidian-950 border border-obsidian-800 text-[11px] text-slate-200 overflow-x-auto leading-relaxed max-h-40 whitespace-pre-wrap select-text">
                {tool.stdout}
              </pre>
            </div>
          )}

          {/* Terminal STDERR */}
          {tool.stderr && (
            <div>
              <span className="text-[10px] font-mono font-bold text-rose-400 block mb-1">
                Stderr Trace
              </span>
              <pre className="p-2 rounded bg-rose-950/20 border border-rose-900/50 text-[11px] text-rose-300 overflow-x-auto leading-relaxed max-h-40 whitespace-pre-wrap select-text">
                {tool.stderr}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

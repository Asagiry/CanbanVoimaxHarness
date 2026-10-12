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
      case 'write':
        return <FileEdit className="h-3.5 w-3.5 text-cyber-amber" />;
      case 'read':
        return <FileText className="h-3.5 w-3.5 text-slate-300" />;
      case 'web_search':
        return <Globe className="h-3.5 w-3.5 text-cyber-purple" />;
      default:
        return <Code2 className="h-3.5 w-3.5 text-slate-400" />;
    }
  };

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

  return (
    <div className="rounded-lg border border-slate-800 bg-obsidian-900/90 overflow-hidden shadow-xs">
      {/* Tool Header */}
      <div
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center justify-between px-3.5 py-2 bg-obsidian-850 hover:bg-obsidian-800 cursor-pointer transition select-none text-xs"
      >
        <div className="flex items-center space-x-2 min-w-0">
          {isExpanded ? (
            <ChevronDown className="h-3.5 w-3.5 text-slate-400 shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-slate-400 shrink-0" />
          )}

          <div className="p-1 rounded bg-obsidian-800 border border-slate-750 shrink-0">
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
            <span className="flex items-center space-x-1 px-1.5 py-0.5 rounded font-mono text-[10px] font-bold bg-emerald-500/15 border border-emerald-500/40 text-emerald-400">
              <CheckCircle2 className="h-2.5 w-2.5" />
              <span>exit: {tool.exitCode ?? 0}</span>
            </span>
          )}

          {tool.status === 'failed' && (
            <span className="flex items-center space-x-1 px-1.5 py-0.5 rounded font-mono text-[10px] font-bold bg-rose-500/15 border border-rose-500/40 text-rose-400">
              <XCircle className="h-2.5 w-2.5" />
              <span>exit: {tool.exitCode ?? 1}</span>
            </span>
          )}

          {tool.status === 'running' && (
            <span className="flex items-center space-x-1 px-1.5 py-0.5 rounded font-mono text-[10px] font-bold bg-amber-500/15 border border-amber-500/40 text-amber-400 animate-pulse">
              <span>executing...</span>
            </span>
          )}

          {/* Copy Button */}
          <button
            onClick={handleCopyLogs}
            className="p-1 rounded text-slate-400 hover:text-white hover:bg-obsidian-750 transition"
            title="Copy logs"
          >
            {copied ? <Check className="h-3 w-3 text-emerald-400" /> : <Copy className="h-3 w-3" />}
          </button>
        </div>
      </div>

      {/* Expanded Body */}
      {isExpanded && (
        <div className="p-3 border-t border-slate-800/80 space-y-2.5 font-mono text-xs">
          {/* Tool Parameters */}
          {tool.params && Object.keys(tool.params).length > 0 && (
            <div>
              <span className="text-[10px] uppercase font-bold text-slate-500 block mb-1">
                Input Parameters
              </span>
              <pre className="p-2 rounded bg-obsidian-950 border border-slate-800 text-[11px] text-slate-300 overflow-x-auto leading-relaxed">
                {JSON.stringify(tool.params, null, 2)}
              </pre>
            </div>
          )}

          {/* Terminal STDOUT */}
          {tool.stdout && (
            <div>
              <span className="text-[10px] uppercase font-bold text-slate-500 block mb-1">
                Stdout
              </span>
              <pre className="p-2.5 rounded bg-obsidian-950 border border-slate-800 text-[11px] text-slate-200 overflow-x-auto leading-relaxed max-h-48 whitespace-pre-wrap select-text">
                {tool.stdout}
              </pre>
            </div>
          )}

          {/* Terminal STDERR */}
          {tool.stderr && (
            <div>
              <span className="text-[10px] uppercase font-bold text-rose-400 block mb-1">
                Stderr Trace
              </span>
              <pre className="p-2.5 rounded bg-rose-950/20 border border-rose-900/50 text-[11px] text-rose-300 overflow-x-auto leading-relaxed max-h-48 whitespace-pre-wrap select-text">
                {tool.stderr}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

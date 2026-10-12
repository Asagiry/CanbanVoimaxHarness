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
  Search,
  ExternalLink,
  Archive,
  Bot,
} from 'lucide-react';

interface ToolExecutionCardProps {
  tool: ToolExecution;
}

export default function ToolExecutionCard({ tool }: ToolExecutionCardProps) {
  const [isExpanded, setIsExpanded] = useState(true);
  const [copied, setCopied] = useState(false);

  const isEditFile =
    tool.toolName === 'edit_file' ||
    tool.toolName === 'str_replace' ||
    Boolean(tool.params?.old_string && tool.params?.new_string);

  const getToolIcon = (name: string) => {
    switch (name) {
      case 'execute_command':
      case 'bash':
        return <Terminal className="h-3.5 w-3.5 text-cyber-cyan" />;
      case 'edit_file':
      case 'str_replace':
      case 'write_file':
      case 'write':
        return <FileEdit className="h-3.5 w-3.5 text-cyber-amber" />;
      case 'read_file':
      case 'read':
        return <FileText className="h-3.5 w-3.5 text-slate-300" />;
      case 'glob':
      case 'grep':
        return <Search className="h-3.5 w-3.5 text-sky-400" />;
      case 'web_search':
      case 'web_fetch':
        return <Globe className="h-3.5 w-3.5 text-cyber-purple" />;
      case 'subagent':
        return <Bot className="h-3.5 w-3.5 text-cyber-emerald" />;
      default:
        return <Code2 className="h-3.5 w-3.5 text-slate-400" />;
    }
  };

  const handleCopyLogs = (e: React.MouseEvent) => {
    e.stopPropagation();
    const content = [
      `Tool: ${tool.toolName}`,
      `Status: ${tool.status} (exit ${tool.exitCode ?? 0})`,
      tool.params ? `PARAMS:\n${JSON.stringify(tool.params, null, 2)}` : '',
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

  const oldString = (tool.params?.old_string as string) || (tool.params?.old_str as string) || '';
  const newString = (tool.params?.new_string as string) || (tool.params?.new_str as string) || '';
  const filePath = (tool.params?.file_path as string) || (tool.params?.path as string) || '';

  // Check if output was spilled to disk
  const isSpilled = tool.stdout?.includes('[Output truncated:') || tool.stderr?.includes('[Output truncated:');

  return (
    <div className="rounded-lg border border-slate-800 bg-obsidian-900/90 overflow-hidden shadow-xs">
      {/* Tool Header Accordion */}
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

          {filePath && (
            <span className="font-mono text-[11px] text-slate-400 truncate max-w-[180px]">
              ({filePath})
            </span>
          )}

          {isSpilled && (
            <span className="flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] font-mono bg-amber-500/15 border border-amber-500/30 text-amber-300">
              <Archive className="h-2.5 w-2.5" />
              <span>Spilled to Disk</span>
            </span>
          )}
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

      {/* Expanded Accordion Body */}
      {isExpanded && (
        <div className="p-3 border-t border-slate-800/80 space-y-2.5 font-mono text-xs">
          {/* Diff Viewer for edit_file / str_replace */}
          {isEditFile && (oldString || newString) && (
            <div className="space-y-1.5">
              <div className="flex items-center justify-between text-[10px] font-bold uppercase text-slate-400">
                <span>Visual Line Diff ({filePath || 'File Edit'})</span>
                <span className="text-slate-500 font-normal">red = removed, green = added</span>
              </div>

              <div className="rounded-lg border border-slate-800 bg-obsidian-950 overflow-hidden text-[11px] leading-relaxed">
                {/* Old Lines (Red) */}
                {oldString.split('\n').map((line, idx) => (
                  <div
                    key={`old-${idx}`}
                    className="flex items-start bg-rose-950/30 text-rose-300 border-l-2 border-rose-500 px-2 py-0.5 font-mono select-text"
                  >
                    <span className="select-none text-rose-500/60 w-6 shrink-0">-</span>
                    <span className="whitespace-pre-wrap">{line}</span>
                  </div>
                ))}

                {/* New Lines (Green) */}
                {newString.split('\n').map((line, idx) => (
                  <div
                    key={`new-${idx}`}
                    className="flex items-start bg-emerald-950/30 text-emerald-300 border-l-2 border-emerald-500 px-2 py-0.5 font-mono select-text"
                  >
                    <span className="select-none text-emerald-500/60 w-6 shrink-0">+</span>
                    <span className="whitespace-pre-wrap">{line}</span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Tool Parameters (when not already rendered in diff viewer) */}
          {!isEditFile && tool.params && Object.keys(tool.params).length > 0 && (
            <div>
              <span className="text-[10px] uppercase font-bold text-slate-500 block mb-1">
                Input Parameters
              </span>
              <pre className="p-2 rounded bg-obsidian-950 border border-slate-800 text-[11px] text-slate-300 overflow-x-auto leading-relaxed">
                {JSON.stringify(tool.params, null, 2)}
              </pre>
            </div>
          )}

          {/* Terminal / Tool STDOUT */}
          {tool.stdout && (
            <div>
              <div className="flex items-center justify-between mb-1">
                <span className="text-[10px] uppercase font-bold text-slate-500">
                  Tool Output (Stdout)
                </span>
                {tool.stdout.includes('spilled to .cvh/spill') && (
                  <span className="text-[10px] text-amber-400 font-mono flex items-center space-x-1">
                    <ExternalLink className="h-2.5 w-2.5" />
                    <span>Log file saved to disk</span>
                  </span>
                )}
              </div>
              <pre className="p-2.5 rounded bg-obsidian-950 border border-slate-800 text-[11px] text-slate-200 overflow-x-auto leading-relaxed max-h-56 whitespace-pre-wrap select-text">
                {tool.stdout}
              </pre>
            </div>
          )}

          {/* Terminal / Tool STDERR */}
          {tool.stderr && (
            <div>
              <span className="text-[10px] uppercase font-bold text-rose-400 block mb-1">
                Stderr Trace
              </span>
              <pre className="p-2.5 rounded bg-rose-950/20 border border-rose-900/50 text-[11px] text-rose-300 overflow-x-auto leading-relaxed max-h-56 whitespace-pre-wrap select-text">
                {tool.stderr}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

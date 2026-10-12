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
  FolderTree,
  Bot,
  ExternalLink,
} from 'lucide-react';

interface ToolExecutionCardProps {
  tool: ToolExecution;
}

export default function ToolExecutionCard({ tool }: ToolExecutionCardProps) {
  const [isExpanded, setIsExpanded] = useState(true);
  const [copied, setCopied] = useState(false);
  const [activeDiffTab, setActiveDiffTab] = useState<'visual' | 'raw'>('visual');

  const getToolIcon = (name: string) => {
    switch (name) {
      case 'execute_command':
      case 'bash':
        return <Terminal className="h-3.5 w-3.5 text-cyber-cyan" />;
      case 'edit_file':
      case 'str_replace':
      case 'write_file':
        return <FileEdit className="h-3.5 w-3.5 text-cyber-amber" />;
      case 'read_file':
        return <FileText className="h-3.5 w-3.5 text-slate-300" />;
      case 'glob':
        return <FolderTree className="h-3.5 w-3.5 text-sky-400" />;
      case 'grep':
        return <Search className="h-3.5 w-3.5 text-emerald-400" />;
      case 'web_search':
      case 'web_fetch':
        return <Globe className="h-3.5 w-3.5 text-cyber-purple" />;
      case 'subagent':
        return <Bot className="h-3.5 w-3.5 text-pink-400" />;
      default:
        return <Code2 className="h-3.5 w-3.5 text-slate-400" />;
    }
  };

  const handleCopyLogs = (e: React.MouseEvent) => {
    e.stopPropagation();
    const content = [
      `Tool: ${tool.toolName}`,
      `Status: ${tool.status} (exit ${tool.exitCode ?? 0})`,
      tool.params ? `Parameters:\n${JSON.stringify(tool.params, null, 2)}` : '',
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

  // Diff comparison for edit_file
  const isEditFile = tool.toolName === 'edit_file' || tool.toolName === 'str_replace';
  const oldString =
    (tool.params?.old_string as string) ||
    tool.diff?.oldString ||
    '';
  const newString =
    (tool.params?.new_string as string) ||
    tool.diff?.newString ||
    '';
  const filePath = (tool.params?.file_path as string) || '';

  const oldLines = oldString ? oldString.split('\n') : [];
  const newLines = newString ? newString.split('\n') : [];

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
            <span className="font-mono text-[11px] text-cyber-cyan truncate max-w-[200px]" title={filePath}>
              {filePath}
            </span>
          )}

          {Boolean(tool.params?.command) && (
            <span
              className="font-mono text-[11px] text-slate-400 truncate max-w-[240px]"
              title={String(tool.params?.command)}
            >
              $ {String(tool.params?.command)}
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

          {/* Copy Logs Button */}
          <button
            onClick={handleCopyLogs}
            className="p-1 rounded text-slate-400 hover:text-white hover:bg-obsidian-750 transition"
            title="Copy tool logs"
          >
            {copied ? <Check className="h-3 w-3 text-emerald-400" /> : <Copy className="h-3 w-3" />}
          </button>
        </div>
      </div>

      {/* Expanded Accordion Body */}
      {isExpanded && (
        <div className="p-3 border-t border-slate-800/80 space-y-3 font-mono text-xs">
          {/* 1. Specialized Diff Viewer for edit_file */}
          {Boolean(isEditFile && (oldString || newString)) && (
            <div className="rounded-lg border border-slate-800 bg-obsidian-950 overflow-hidden">
              <div className="flex items-center justify-between px-3 py-1.5 bg-obsidian-900 border-b border-slate-800 text-[11px]">
                <div className="flex items-center space-x-2">
                  <span className="font-bold text-cyber-amber uppercase text-[10px]">
                    Visual Diff Viewer
                  </span>
                  <span className="text-slate-400">•</span>
                  <span className="text-cyber-cyan truncate">{filePath}</span>
                </div>

                <div className="flex items-center space-x-2">
                  <span className="text-[10px] text-rose-400">-{oldLines.length}</span>
                  <span className="text-[10px] text-emerald-400">+{newLines.length}</span>
                  <div className="flex rounded bg-obsidian-800 p-0.5 border border-slate-750 text-[10px]">
                    <button
                      onClick={() => setActiveDiffTab('visual')}
                      className={`px-1.5 py-0.5 rounded transition ${
                        activeDiffTab === 'visual'
                          ? 'bg-cyber-cyan text-obsidian-950 font-bold'
                          : 'text-slate-400 hover:text-white'
                      }`}
                    >
                      Split
                    </button>
                    <button
                      onClick={() => setActiveDiffTab('raw')}
                      className={`px-1.5 py-0.5 rounded transition ${
                        activeDiffTab === 'raw'
                          ? 'bg-cyber-cyan text-obsidian-950 font-bold'
                          : 'text-slate-400 hover:text-white'
                      }`}
                    >
                      JSON
                    </button>
                  </div>
                </div>
              </div>

              {activeDiffTab === 'visual' ? (
                <div className="divide-y divide-slate-850 max-h-64 overflow-y-auto text-[11px] leading-relaxed">
                  {/* Removed lines (red) */}
                  {oldLines.map((line, idx) => (
                    <div
                      key={`del-${idx}`}
                      className="flex items-start bg-rose-950/25 text-rose-300 border-l-2 border-rose-500/80 px-2.5 py-0.5 font-mono select-text"
                    >
                      <span className="w-5 text-rose-500/70 select-none shrink-0 font-bold">-</span>
                      <pre className="whitespace-pre-wrap break-all">{line || ' '}</pre>
                    </div>
                  ))}

                  {/* Added lines (green) */}
                  {newLines.map((line, idx) => (
                    <div
                      key={`add-${idx}`}
                      className="flex items-start bg-emerald-950/25 text-emerald-300 border-l-2 border-emerald-500/80 px-2.5 py-0.5 font-mono select-text"
                    >
                      <span className="w-5 text-emerald-500/70 select-none shrink-0 font-bold">+</span>
                      <pre className="whitespace-pre-wrap break-all">{line || ' '}</pre>
                    </div>
                  ))}
                </div>
              ) : (
                <pre className="p-2.5 text-[11px] text-slate-300 overflow-x-auto">
                  {JSON.stringify({ file_path: filePath, old_string: oldString, new_string: newString }, null, 2)}
                </pre>
              )}
            </div>
          )}

          {/* 2. Command Shell Execution Card */}
          {(tool.toolName === 'execute_command' || tool.toolName === 'bash') && Boolean(tool.params?.command) && (
            <div>
              <span className="text-[10px] uppercase font-bold text-slate-500 block mb-1">
                Executed Command
              </span>
              <div className="p-2.5 rounded bg-obsidian-950 border border-slate-800 flex items-center space-x-2 text-cyber-cyan text-[11px]">
                <span className="text-slate-500 select-none font-bold">$</span>
                <span className="select-text font-bold">{String(tool.params.command)}</span>
              </div>
            </div>
          )}

          {/* 3. General Input Parameters (for non-edit tools) */}
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

          {/* 4. Terminal STDOUT / Output */}
          {tool.stdout && (
            <div>
              <div className="flex items-center justify-between mb-1">
                <span className="text-[10px] uppercase font-bold text-slate-500">
                  Stdout / Tool Result
                </span>
                {tool.stdout.includes('spilled to') && (
                  <span className="flex items-center space-x-1 text-[10px] font-mono text-cyber-amber">
                    <ExternalLink className="h-2.5 w-2.5" />
                    <span>Spilled Log Archive</span>
                  </span>
                )}
              </div>
              <pre className="p-2.5 rounded bg-obsidian-950 border border-slate-800 text-[11px] text-slate-200 overflow-x-auto leading-relaxed max-h-48 whitespace-pre-wrap select-text">
                {tool.stdout}
              </pre>
            </div>
          )}

          {/* 5. Terminal STDERR */}
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

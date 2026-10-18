import { useState, useRef, useEffect } from 'react';
import type { ToolExecution } from '../../types';
import {
  Wrench,
  ChevronDown,
  ChevronRight,
  Check,
  X,
  Loader2,
  Copy,
  Terminal,
} from 'lucide-react';

interface ToolExecutionCardProps {
  tool: ToolExecution;
}

export function ToolExecutionCard({ tool }: ToolExecutionCardProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [copied, setCopied] = useState(false);
  const copiedTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (copiedTimeoutRef.current) {
        clearTimeout(copiedTimeoutRef.current);
      }
    };
  }, []);

  const handleCopy = (e: React.MouseEvent) => {
    e.stopPropagation();
    const data = JSON.stringify(
      {
        tool: tool.toolName,
        params: tool.params,
        stdout: tool.stdout,
        stderr: tool.stderr,
      },
      null,
      2
    );
    navigator.clipboard.writeText(data);
    setCopied(true);
    if (copiedTimeoutRef.current) {
      clearTimeout(copiedTimeoutRef.current);
    }
    copiedTimeoutRef.current = setTimeout(() => {
      setCopied(false);
      copiedTimeoutRef.current = null;
    }, 1500);
  };

  const hasParams = tool.params && Object.keys(tool.params).length > 0;
  const hasOutput = Boolean(tool.stdout || tool.stderr);

  return (
    <div className="my-1.5 rounded-lg border border-[#e4e4e7] dark:border-[#21262d] bg-white dark:bg-[#0d1117] shadow-2xs overflow-hidden text-xs">
      {/* Header bar */}
      <button
        type="button"
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full px-3 py-1.5 flex items-center justify-between bg-[#fbfbfb] dark:bg-[#161b22] hover:bg-[#f4f4f5] dark:hover:bg-[#1c2128] border-b border-[#f0f0f0] dark:border-[#21262d] cursor-pointer select-none transition"
      >
        <div className="flex items-center space-x-2 min-w-0">
          <div className="p-1 rounded bg-[#f4f4f5] dark:bg-[#21262d] border border-[#e4e4e7] dark:border-[#30363d] text-[#52525b] dark:text-[#8b949e]">
            <Wrench className="w-3 h-3" />
          </div>
          <span className="font-mono font-medium text-[#18181b] dark:text-[#f0f6fc] truncate text-[11px]">
            {tool.toolName}
          </span>
          {tool.durationMs > 0 && (
            <span className="font-mono text-[10px] text-[#a1a1aa] dark:text-[#8b949e] tabular-nums">
              {tool.durationMs}ms
            </span>
          )}
        </div>

        <div className="flex items-center space-x-2 shrink-0">
          {tool.status === 'running' && (
            <span className="flex items-center space-x-1 text-amber-600 dark:text-[#facc15] font-mono text-[10px] bg-amber-50 dark:bg-[#422006]/40 px-1.5 py-0.5 rounded border border-amber-200 dark:border-[#713f12]">
              <Loader2 className="w-2.5 h-2.5 animate-spin" />
              <span>выполняется</span>
            </span>
          )}
          {tool.status === 'success' && (
            <span className="flex items-center space-x-1 text-emerald-600 dark:text-[#7ee787] font-mono text-[10px] bg-emerald-50 dark:bg-[#12281e] px-1.5 py-0.5 rounded border border-emerald-200 dark:border-[#238636]">
              <Check className="w-2.5 h-2.5" />
              <span>успех</span>
            </span>
          )}
          {tool.status === 'failed' && (
            <span className="flex items-center space-x-1 text-rose-600 dark:text-[#ff7b72] font-mono text-[10px] bg-rose-50 dark:bg-[#3b1219] px-1.5 py-0.5 rounded border border-rose-200 dark:border-[#da3633]">
              <X className="w-2.5 h-2.5" />
              <span>ошибка</span>
            </span>
          )}

          {(hasParams || hasOutput) && (
            <div className="text-[#a1a1aa] dark:text-[#8b949e] hover:text-[#18181b] dark:hover:text-[#f0f6fc] transition">
              {isExpanded ? (
                <ChevronDown className="w-3.5 h-3.5" />
              ) : (
                <ChevronRight className="w-3.5 h-3.5" />
              )}
            </div>
          )}
        </div>
      </button>

      {/* Expanded details */}
      {isExpanded && (hasParams || hasOutput) && (
        <div className="p-3 bg-[#fafafa] dark:bg-[#161b22] space-y-2 border-t border-[#f4f4f5] dark:border-[#21262d]">
          {hasParams && (
            <div>
              <div className="text-[10px] uppercase font-mono text-[#71717a] dark:text-[#8b949e] font-semibold mb-1">
                Параметры:
              </div>
              <pre className="p-2 rounded bg-white dark:bg-[#0d1117] border border-[#e4e4e7] dark:border-[#30363d] font-mono text-[11px] text-[#27272a] dark:text-[#f0f6fc] overflow-x-auto">
                {JSON.stringify(tool.params, null, 2)}
              </pre>
            </div>
          )}

          {hasOutput && (
            <div>
              <div className="flex items-center justify-between text-[10px] uppercase font-mono text-[#71717a] dark:text-[#8b949e] font-semibold mb-1">
                <span className="flex items-center space-x-1">
                  <Terminal className="w-3 h-3" />
                  <span>Результат / Вывод:</span>
                </span>
                <button
                  type="button"
                  onClick={handleCopy}
                  className="flex items-center space-x-1 text-[#71717a] dark:text-[#8b949e] hover:text-[#18181b] dark:hover:text-[#f0f6fc] cursor-pointer"
                >
                  {copied ? (
                    <Check className="w-2.5 h-2.5 text-emerald-600 dark:text-emerald-400" />
                  ) : (
                    <Copy className="w-2.5 h-2.5" />
                  )}
                  <span>{copied ? 'Скопировано' : 'Копировать'}</span>
                </button>
              </div>
              <pre className="p-2 rounded bg-white dark:bg-[#0d1117] border border-[#e4e4e7] dark:border-[#30363d] font-mono text-[11px] text-[#27272a] dark:text-[#f0f6fc] overflow-x-auto max-h-40 leading-relaxed">
                {tool.stdout || tool.stderr}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default ToolExecutionCard;

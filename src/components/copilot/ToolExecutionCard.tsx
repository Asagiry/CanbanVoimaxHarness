import { useState } from 'react';
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
    setTimeout(() => setCopied(false), 1500);
  };

  const hasParams = tool.params && Object.keys(tool.params).length > 0;
  const hasOutput = Boolean(tool.stdout || tool.stderr);

  return (
    <div className="my-1.5 rounded-lg border border-[#e4e4e7] bg-white shadow-2xs overflow-hidden text-xs">
      {/* Header bar */}
      <button
        type="button"
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full px-3 py-1.5 flex items-center justify-between bg-[#fbfbfb] hover:bg-[#f4f4f5] border-b border-[#f0f0f0] cursor-pointer select-none transition"
      >
        <div className="flex items-center space-x-2 min-w-0">
          <div className="p-1 rounded bg-[#f4f4f5] border border-[#e4e4e7] text-[#52525b]">
            <Wrench className="w-3 h-3" />
          </div>
          <span className="font-mono font-medium text-[#18181b] truncate text-[11px]">
            {tool.toolName}
          </span>
          {tool.durationMs > 0 && (
            <span className="font-mono text-[10px] text-[#a1a1aa] tabular-nums">
              {tool.durationMs}ms
            </span>
          )}
        </div>

        <div className="flex items-center space-x-2 shrink-0">
          {tool.status === 'running' && (
            <span className="flex items-center space-x-1 text-amber-600 font-mono text-[10px] bg-amber-50 px-1.5 py-0.5 rounded border border-amber-200">
              <Loader2 className="w-2.5 h-2.5 animate-spin" />
              <span>выполняется</span>
            </span>
          )}
          {tool.status === 'success' && (
            <span className="flex items-center space-x-1 text-emerald-600 font-mono text-[10px] bg-emerald-50 px-1.5 py-0.5 rounded border border-emerald-200">
              <Check className="w-2.5 h-2.5" />
              <span>успех</span>
            </span>
          )}
          {tool.status === 'failed' && (
            <span className="flex items-center space-x-1 text-rose-600 font-mono text-[10px] bg-rose-50 px-1.5 py-0.5 rounded border border-rose-200">
              <X className="w-2.5 h-2.5" />
              <span>ошибка</span>
            </span>
          )}

          {(hasParams || hasOutput) && (
            <div className="text-[#a1a1aa] hover:text-[#18181b] transition">
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
        <div className="p-3 bg-[#fafafa] space-y-2 border-t border-[#f4f4f5]">
          {hasParams && (
            <div>
              <div className="text-[10px] uppercase font-mono text-[#71717a] font-semibold mb-1">
                Параметры:
              </div>
              <pre className="p-2 rounded bg-white border border-[#e4e4e7] font-mono text-[11px] text-[#27272a] overflow-x-auto">
                {JSON.stringify(tool.params, null, 2)}
              </pre>
            </div>
          )}

          {hasOutput && (
            <div>
              <div className="flex items-center justify-between text-[10px] uppercase font-mono text-[#71717a] font-semibold mb-1">
                <span className="flex items-center space-x-1">
                  <Terminal className="w-3 h-3" />
                  <span>Результат / Вывод:</span>
                </span>
                <button
                  type="button"
                  onClick={handleCopy}
                  className="flex items-center space-x-1 text-[#71717a] hover:text-[#18181b] cursor-pointer"
                >
                  {copied ? (
                    <Check className="w-2.5 h-2.5 text-emerald-600" />
                  ) : (
                    <Copy className="w-2.5 h-2.5" />
                  )}
                  <span>{copied ? 'Скопировано' : 'Копировать'}</span>
                </button>
              </div>
              <pre className="p-2 rounded bg-white border border-[#e4e4e7] font-mono text-[11px] text-[#27272a] overflow-x-auto max-h-40 leading-relaxed">
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

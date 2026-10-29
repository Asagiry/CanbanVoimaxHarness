import { useState, useRef, useEffect } from 'react';
import { ToolExecution } from '../../../types';
import { Check, Copy, X } from 'lucide-react';
import { formatDurationCompact } from '../ThinkingCard';

export interface ToolCardProps {
  tool: ToolExecution;
  onRerun?: (tool: ToolExecution) => void;
  isLatest?: boolean;
}

export function isToolFailed(tool: ToolExecution): boolean {
  return (
    tool.status === 'failed' ||
    (tool.status as string) === 'error' ||
    Boolean(tool.stderr && tool.stderr.trim().length > 0) ||
    (tool.exitCode !== undefined && tool.exitCode !== null && tool.exitCode !== 0)
  );
}

export function StatusIndicator({ tool, isFailed }: { tool: ToolExecution; isFailed?: boolean }) {
  const failed = isFailed ?? isToolFailed(tool);
  if (tool.status === 'running') {
    return (
      <span className="inline-flex rounded-full h-2 w-2 bg-amber-500 animate-pulse ring-2 ring-amber-500/20 shrink-0" />
    );
  }
  if (!failed && tool.status === 'success') {
    return (
      <span className="flex items-center shrink-0">
        <Check className="h-3.5 w-3.5 text-emerald-600 dark:text-emerald-400 stroke-[2.5]" />
      </span>
    );
  }
  return (
    <span className="flex items-center shrink-0">
      <X className="h-3.5 w-3.5 text-rose-600 dark:text-rose-400 stroke-[2.5]" />
    </span>
  );
}

export function CopyLogsButton({ tool, isExpanded }: { tool: ToolExecution; isExpanded: boolean }) {
  const [copied, setCopied] = useState(false);
  const copiedTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (copiedTimeoutRef.current) {
        clearTimeout(copiedTimeoutRef.current);
      }
    };
  }, []);

  const handleCopyLogs = (e: React.MouseEvent) => {
    e.stopPropagation();
    const formattedParams =
      tool.params && Object.keys(tool.params).length > 0
        ? Object.entries(tool.params as Record<string, unknown>)
            .map(([k, v]) => {
              if (typeof v === 'string') {
                return v.includes('\n') ? `${k}:\n${v}` : `${k}: ${v}`;
              }
              return `${k}: ${JSON.stringify(v, null, 2)}`;
            })
            .join('\n\n')
        : '';

    const content = [
      `Инструмент: ${tool.toolName}`,
      `Статус: ${tool.status} (код ${tool.exitCode ?? 0})`,
      formattedParams ? `ПАРАМЕТРЫ:\n${formattedParams}` : '',
      tool.stdout ? `STDOUT:\n${tool.stdout}` : '',
      tool.stderr ? `STDERR:\n${tool.stderr}` : '',
    ]
      .filter(Boolean)
      .join('\n\n');

    navigator.clipboard.writeText(content);
    setCopied(true);
    if (copiedTimeoutRef.current) {
      clearTimeout(copiedTimeoutRef.current);
    }
    copiedTimeoutRef.current = setTimeout(() => {
      setCopied(false);
      copiedTimeoutRef.current = null;
    }, 2000);
  };

  return (
    <button
      type="button"
      onClick={handleCopyLogs}
      className={`p-1 rounded text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-white dark:hover:bg-[#161b22] transition cursor-pointer ${
        isExpanded ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'
      }`}
      aria-label="Скопировать вывод"
    >
      {copied ? (
        <Check className="h-3 w-3 text-emerald-600 dark:text-emerald-400" />
      ) : (
        <Copy className="h-3 w-3" />
      )}
    </button>
  );
}

export function ToolParamsView({ params }: { params: Record<string, unknown> }) {
  const entries = Object.entries(params);
  if (entries.length === 0) return null;

  return (
    <div className="space-y-1.5">
      <span className="text-[10px] text-[#666666] dark:text-[#8b949e] block font-mono">
        Параметры вызова
      </span>
      <div className="space-y-2">
        {entries.map(([key, value]) => {
          if (typeof value === 'string') {
            const isMultiline = value.includes('\n');
            const isCodeLike =
              key === 'content' ||
              key === 'code' ||
              key === 'command' ||
              key === 'cmd' ||
              key === 'old_string' ||
              key === 'new_string' ||
              key === 'prompt' ||
              key === 'script' ||
              key === 'patch' ||
              isMultiline;

            if (isCodeLike || isMultiline) {
              const lineCount = value.split('\n').length;
              return (
                <div key={key} className="space-y-1">
                  <div className="flex items-center justify-between text-[10px] text-[#666666] dark:text-[#8b949e]">
                    <span className="font-semibold text-[#171717] dark:text-[#f0f6fc] font-mono">{key}:</span>
                    <span className="text-[#8f8f8f] dark:text-[#8b949e] tabular-nums font-mono text-[10px]">
                      {lineCount > 1 ? `${lineCount} строк` : `${value.length} симв.`}
                    </span>
                  </div>
                  <pre className="p-2.5 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#21262d] text-[11px] text-[#171717] dark:text-[#f0f6fc] overflow-x-auto leading-relaxed max-h-64 whitespace-pre select-text font-mono">
                    {value}
                  </pre>
                </div>
              );
            }

            return (
              <div key={key} className="flex items-baseline space-x-2 py-0.5 text-[11px] font-mono">
                <span className="text-[#666666] dark:text-[#8b949e] font-semibold shrink-0">{key}:</span>
                <span className="text-[#171717] dark:text-[#f0f6fc] select-text break-all">{value}</span>
              </div>
            );
          }

          if (typeof value === 'object' && value !== null) {
            return (
              <div key={key} className="space-y-1">
                <span className="text-[10px] font-semibold text-[#171717] dark:text-[#f0f6fc] font-mono">{key}:</span>
                <pre className="p-2 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#21262d] text-[11px] text-[#171717] dark:text-[#f0f6fc] overflow-x-auto leading-relaxed max-h-48 whitespace-pre-wrap select-text font-mono">
                  {JSON.stringify(value, null, 2)}
                </pre>
              </div>
            );
          }

          return (
            <div key={key} className="flex items-baseline space-x-2 py-0.5 text-[11px] font-mono">
              <span className="text-[#666666] dark:text-[#8b949e] font-semibold shrink-0">{key}:</span>
              <span className="text-[#171717] dark:text-[#f0f6fc] select-text">{String(value)}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

export { formatDurationCompact };

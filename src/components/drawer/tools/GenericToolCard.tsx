import { useState } from 'react';
import { ChevronRight, Wrench } from 'lucide-react';
import {
  ToolCardProps,
  isToolFailed,
  StatusIndicator,
  CopyLogsButton,
  ToolParamsView,
  formatDurationCompact,
} from './shared';

export default function GenericToolCard({ tool }: ToolCardProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  const toolName = tool.toolName ?? 'tool';
  const params: Record<string, unknown> =
    tool.params && typeof tool.params === 'object'
      ? (tool.params as Record<string, unknown>)
      : {};

  const isFailed = isToolFailed(tool);
  const isRunning = tool.status === 'running';
  const durationText = formatDurationCompact(tool.durationMs);

  const firstStr = Object.values(params).find(
    (v) => typeof v === 'string' && (v as string).trim().length > 0
  ) as string | undefined;

  const raw = firstStr || '';
  const target = raw.length > 55 ? `${raw.slice(0, 52)}…` : raw;

  let extraInfo: string | null = null;
  if (tool.stdout) {
    const lines = tool.stdout.split('\n').length;
    if (lines > 1) extraInfo = `${lines} lines`;
  }

  return (
    <div
      className={`group rounded-md border text-xs shadow-2xs hover:shadow-xs overflow-hidden my-1 animate-silk-entrance transition-colors bg-white dark:bg-[#161b22] ${
        isRunning
          ? 'border-amber-500/40 dark:border-amber-500/30 bg-amber-500/[0.02]'
          : 'border-[#ebebeb] dark:border-[#21262d]'
      }`}
    >
      {/* Header Strip */}
      <div
        onClick={() => setIsExpanded(!isExpanded)}
        className={`flex items-center justify-between px-2.5 py-1.5 bg-[#f8f8f9] dark:bg-[#161b22] hover:bg-[#f2f2f3] dark:hover:bg-[#1c2128] transition-colors cursor-pointer select-none gap-2 text-xs min-h-[30px] ${
          isExpanded ? 'border-b border-[#ebebeb] dark:border-[#21262d]' : ''
        }`}
      >
        <div className="flex items-center space-x-1.5 min-w-0 flex-1 overflow-hidden">
          <button
            type="button"
            className="p-0.5 text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] rounded shrink-0 transition cursor-pointer"
            onClick={(e) => {
              e.stopPropagation();
              setIsExpanded(!isExpanded);
            }}
            aria-label={isExpanded ? 'Свернуть подробности' : 'Развернуть подробности'}
          >
            <ChevronRight
              className={`h-3 w-3 transition-transform duration-150 ${
                isExpanded ? 'rotate-90 text-[#171717] dark:text-[#f0f6fc]' : ''
              }`}
            />
          </button>

          <span className="flex items-center shrink-0 select-none text-[#8f8f8f] dark:text-[#8b949e]">
            <Wrench className="h-3.5 w-3.5" />
          </span>

          <span className="font-mono font-medium text-[#171717] dark:text-[#f0f6fc] shrink-0">
            {toolName}{target ? ':' : ''}
          </span>

          {target && (
            <span className="font-mono text-[#52525b] dark:text-[#8b949e] truncate max-w-[260px] sm:max-w-[360px] md:max-w-[460px]">
              {target}
            </span>
          )}

          {extraInfo && (
            <>
              <span className="text-[#a1a1aa] dark:text-[#6e7681] select-none text-[10px]">•</span>
              <span className="font-mono text-[#71717a] dark:text-[#8b949e] shrink-0 tabular-nums text-[11px]">
                {extraInfo}
              </span>
            </>
          )}

          {durationText && (
            <>
              <span className="text-[#a1a1aa] dark:text-[#6e7681] select-none text-[10px]">•</span>
              <span className="font-mono text-[#71717a] dark:text-[#8b949e] shrink-0 tabular-nums text-[11px]">
                {durationText}
              </span>
            </>
          )}
        </div>

        <div className="flex items-center space-x-2 shrink-0">
          <CopyLogsButton tool={tool} isExpanded={isExpanded} />
          <StatusIndicator tool={tool} isFailed={isFailed} />
        </div>
      </div>

      {/* Expanded Details */}
      {isExpanded && (
        <div className="bg-white dark:bg-[#0d1117] p-3 border-t border-[#ebebeb] dark:border-[#21262d] space-y-2.5 font-mono text-xs">
          {tool.params && Object.keys(tool.params).length > 0 && (
            <ToolParamsView params={params} />
          )}

          {tool.stdout && (
            <div>
              <span className="text-[10px] text-[#666666] dark:text-[#8b949e] block mb-1">
                Вывод команды
              </span>
              <pre className="p-2.5 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#21262d] text-[11px] text-[#171717] dark:text-[#f0f6fc] overflow-x-auto leading-relaxed max-h-48 whitespace-pre-wrap select-text font-mono">
                {tool.stdout}
              </pre>
            </div>
          )}

          {tool.stderr && (
            <div>
              <span className="text-[10px] text-[#b91c1c] dark:text-[#ff7b72] block mb-1">
                Ошибки выполнения
              </span>
              <pre className="p-2.5 rounded-md bg-[#fef2f2] dark:bg-[#3b1219] border border-[#fecaca] dark:border-[#da3633] text-[11px] text-[#991b1b] dark:text-[#ff7b72] overflow-x-auto leading-relaxed max-h-48 whitespace-pre-wrap select-text font-mono">
                {tool.stderr}
              </pre>
            </div>
          )}

          <div className="flex items-center justify-between pt-1 border-t border-[#f4f4f5] dark:border-[#21262d] text-[10px] text-[#8f8f8f] dark:text-[#8b949e]">
            <span>Статус: {isFailed ? `failed (код ${tool.exitCode ?? 1})` : tool.status}</span>
          </div>
        </div>
      )}
    </div>
  );
}

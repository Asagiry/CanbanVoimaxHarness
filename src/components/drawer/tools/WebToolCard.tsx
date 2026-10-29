import { useState } from 'react';
import { ChevronRight, Globe } from 'lucide-react';
import {
  ToolCardProps,
  isToolFailed,
  StatusIndicator,
  CopyLogsButton,
  ToolParamsView,
  formatDurationCompact,
} from './shared';

export default function WebToolCard({ tool }: ToolCardProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  const toolName = tool.toolName ?? 'web';
  const params: Record<string, unknown> =
    tool.params && typeof tool.params === 'object'
      ? (tool.params as Record<string, unknown>)
      : {};

  const isFailed = isToolFailed(tool);
  const isRunning = tool.status === 'running';
  const durationText = formatDurationCompact(tool.durationMs);

  const queryOrUrl =
    (params.query as string) ||
    (params.url as string) ||
    (params.target as string) ||
    '';

  const fullTarget = queryOrUrl;
  const target = queryOrUrl.length > 55 ? `${queryOrUrl.slice(0, 52)}…` : queryOrUrl;

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

          <span className="flex items-center shrink-0 select-none text-blue-600 dark:text-blue-400">
            <Globe className="h-3.5 w-3.5" />
          </span>

          <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-medium shrink-0 select-none bg-blue-50 dark:bg-blue-950/40 text-blue-700 dark:text-blue-300 border border-blue-200 dark:border-blue-800/60">
            {toolName}
          </span>

          <span className="font-mono text-[#171717] dark:text-[#f0f6fc] font-medium truncate max-w-[260px] sm:max-w-[360px] md:max-w-[460px]">
            {target || 'web request'}
          </span>

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
          {fullTarget && (
            <div className="flex items-baseline space-x-2 py-0.5 text-[11px] font-mono">
              <span className="text-[#666666] dark:text-[#8b949e] font-semibold shrink-0">
                {toolName === 'web_search' ? 'Запрос:' : 'URL:'}
              </span>
              <span className="text-[#171717] dark:text-[#f0f6fc] select-text break-all">
                {fullTarget}
              </span>
            </div>
          )}

          {tool.params && Object.keys(tool.params).length > 1 && (
            <ToolParamsView params={params} />
          )}

          {tool.stdout && (
            <div>
              <span className="text-[10px] text-[#666666] dark:text-[#8b949e] block mb-1">
                Ответ сервера
              </span>
              <pre className="p-2.5 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#21262d] text-[11px] text-[#171717] dark:text-[#f0f6fc] overflow-x-auto leading-relaxed max-h-56 whitespace-pre-wrap select-text font-mono">
                {tool.stdout}
              </pre>
            </div>
          )}

          {tool.stderr && (
            <div>
              <span className="text-[10px] text-[#b91c1c] dark:text-[#ff7b72] block mb-1">
                Ошибки сетевого запроса
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

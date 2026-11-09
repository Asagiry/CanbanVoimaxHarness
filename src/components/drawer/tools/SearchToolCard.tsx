import { ChevronRight, Search, FileSearch } from 'lucide-react';
import {
  ToolCardProps,
  isToolFailed,
  isToolSkipped,
  StatusIndicator,
  CopyLogsButton,
  BatchBadge,
  ToolParamsView,
  formatDurationCompact,
  useToolExpanded,
} from './shared';

export default function SearchToolCard({ tool, isExpanded: controlledExpanded, onToggleExpanded }: ToolCardProps) {
  const [isExpanded, setIsExpanded] = useToolExpanded(controlledExpanded, onToggleExpanded, false);

  const toolName = tool.toolName ?? 'search';
  const params: Record<string, unknown> =
    tool.params && typeof tool.params === 'object'
      ? (tool.params as Record<string, unknown>)
      : {};

  const isFailed = isToolFailed(tool);
  const isSkipped = isToolSkipped(tool);
  const isRunning = tool.status === 'running';
  const durationText = formatDurationCompact(tool.durationMs);

  const pattern = (params.pattern as string) || (params.query as string) || '';
  const searchPath = (params.path as string) || '';

  const target = searchPath && searchPath !== '.' ? `"${pattern}" in ${searchPath}` : `"${pattern}"`;

  let extraInfo: string | null = null;
  if (tool.stdout) {
    const lines = tool.stdout.split('\n').filter((l) => l.trim().length > 0);
    if (toolName === 'glob') {
      extraInfo = `${lines.length} files`;
    } else {
      extraInfo = `${lines.length} matches`;
    }
  }

  const IconComponent = toolName === 'glob' ? FileSearch : Search;

  return (
    <div
      className={`group rounded-md border text-xs shadow-2xs hover:shadow-xs overflow-hidden my-1 animate-silk-entrance transition-colors bg-white dark:bg-[#17171b] ${
        isRunning
          ? 'border-amber-500/40 dark:border-amber-500/30 bg-amber-500/[0.02]'
          : 'border-[#ebebeb] dark:border-[#212126]'
      } ${isSkipped ? 'opacity-70 dark:opacity-65' : ''}`}
    >
      {/* Header Strip */}
      <div
        onClick={() => setIsExpanded(!isExpanded)}
        className={`flex items-center justify-between px-2.5 py-1.5 bg-[#f8f8f9] dark:bg-[#17171b] hover:bg-[#f2f2f3] dark:hover:bg-[#1e1e23] transition-colors cursor-pointer select-none gap-2 text-xs min-h-[30px] ${
          isExpanded ? 'border-b border-[#ebebeb] dark:border-[#212126]' : ''
        }`}
      >
        <div className="flex items-center space-x-1.5 min-w-0 flex-1">
          <button
            type="button"
            className="p-0.5 text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] rounded shrink-0 transition cursor-pointer"
            onClick={(e) => {
              e.stopPropagation();
              setIsExpanded(!isExpanded);
            }}
            aria-label={isExpanded ? 'Свернуть подробности' : 'Развернуть подробности'}
          >
            <ChevronRight
              className={`h-3 w-3 transition-transform duration-150 ${
                isExpanded ? 'rotate-90 text-[#171717] dark:text-[#f4f4f5]' : ''
              }`}
            />
          </button>

          <span className="flex items-center shrink-0 select-none text-emerald-600 dark:text-emerald-400">
            <IconComponent className="h-3.5 w-3.5" />
          </span>

          <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-mono font-medium shrink-0 select-none bg-emerald-50 dark:bg-emerald-950/40 text-emerald-700 dark:text-emerald-300 border border-emerald-200 dark:border-emerald-800/60">
            {toolName}
          </span>

          <span className="font-mono text-[#52525b] dark:text-[#71717a] truncate max-w-[260px] sm:max-w-[360px] md:max-w-[460px]">
            {target}
          </span>

          {extraInfo && (
            <>
              <span className="text-[#a1a1aa] dark:text-[#52525b] select-none text-[10px]">•</span>
              <span className="font-mono text-[#71717a] dark:text-[#71717a] shrink-0 tabular-nums text-[11px]">
                {extraInfo}
              </span>
            </>
          )}

          {durationText && (
            <>
              <span className="text-[#a1a1aa] dark:text-[#52525b] select-none text-[10px]">•</span>
              <span className="font-mono text-[#71717a] dark:text-[#71717a] shrink-0 tabular-nums text-[11px]">
                {durationText}
              </span>
            </>
          )}
        </div>

        <div className="flex items-center space-x-2 shrink-0">
          <BatchBadge tool={tool} />
          <CopyLogsButton tool={tool} isExpanded={isExpanded} />
          <StatusIndicator tool={tool} isFailed={isFailed} />
        </div>
      </div>

      {/* Expanded Details */}
      {isExpanded && (
        <div className="bg-white dark:bg-[#111114] p-3 border-t border-[#ebebeb] dark:border-[#212126] space-y-2.5 font-mono text-xs">
          {tool.params && Object.keys(tool.params).length > 0 && (
            <ToolParamsView params={params} />
          )}

          {tool.stdout && (
            <div>
              <span className="text-[10px] text-[#666666] dark:text-[#71717a] block mb-1">
                Результаты поиска
              </span>
              <pre className="p-2.5 rounded-md bg-[#fafafa] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#212126] text-[11px] text-[#171717] dark:text-[#f4f4f5] overflow-x-auto leading-relaxed max-h-56 whitespace-pre select-text font-mono">
                {tool.stdout}
              </pre>
            </div>
          )}

          {tool.stderr && (
            <div>
              <span className="text-[10px] text-[#b91c1c] dark:text-[#ff7b72] block mb-1">
                Ошибки поиска
              </span>
              <pre className="p-2.5 rounded-md bg-[#fef2f2] dark:bg-[#3b1219] border border-[#fecaca] dark:border-[#da3633] text-[11px] text-[#991b1b] dark:text-[#ff7b72] overflow-x-auto leading-relaxed max-h-48 whitespace-pre-wrap select-text font-mono">
                {tool.stderr}
              </pre>
            </div>
          )}

          <div className="flex items-center justify-between pt-1 border-t border-[#f4f4f5] dark:border-[#212126] text-[10px] text-[#8f8f8f] dark:text-[#71717a]">
            <span>Статус: {isFailed ? `failed (код ${tool.exitCode ?? 1})` : tool.status}</span>
          </div>
        </div>
      )}
    </div>
  );
}

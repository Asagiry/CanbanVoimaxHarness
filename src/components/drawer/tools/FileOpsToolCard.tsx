import {
  ToolCardProps,
  isToolSkipped,
  StatusIndicator,
  CopyLogsButton,
  BatchBadge,
  ToolParamsView,
  useToolExpanded,
} from './shared';
import {
  parseFileOpsInfo,
  FileOpsHeader,
  FileOpsDiffView,
  FileOpsContentPreview,
  FileOpsReadView,
} from './fileOps';

export * from './fileOps';

export default function FileOpsToolCard({
  tool,
  isExpanded: controlledExpanded,
  onToggleExpanded,
}: ToolCardProps) {
  const [isExpanded, setIsExpanded] = useToolExpanded(controlledExpanded, onToggleExpanded, false);
  const parsed = parseFileOpsInfo(tool);

  const {
    params,
    isFileOp,
    isReadOp,
    isDiffTool,
    rawFilePath,
    oldString,
    newString,
    lineRangeBadge,
    startLine,
    isFailed,
  } = parsed;

  const isRunning = tool.status === 'running';
  const isSkipped = isToolSkipped(tool);

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
        <FileOpsHeader
          tool={tool}
          parsed={parsed}
          isExpanded={isExpanded}
          onToggle={(e) => {
            e.stopPropagation();
            setIsExpanded(!isExpanded);
          }}
        />

        <div className="flex items-center space-x-2 shrink-0">
          <BatchBadge tool={tool} />
          <CopyLogsButton tool={tool} isExpanded={isExpanded} />
          <StatusIndicator tool={tool} isFailed={isFailed} />
        </div>
      </div>

      {/* Expanded Details */}
      {isExpanded && (
        <div className="bg-white dark:bg-[#111114] p-3 border-t border-[#ebebeb] dark:border-[#212126] space-y-2.5 font-mono text-xs">
          {isReadOp ? (
            tool.stdout ? (
              <FileOpsReadView
                rawFilePath={rawFilePath}
                stdout={tool.stdout}
                startLine={startLine}
                lineRangeBadge={lineRangeBadge}
              />
            ) : (
              tool.params &&
              Object.keys(tool.params).length > 0 && <ToolParamsView params={params} />
            )
          ) : isFileOp ? (
            oldString !== undefined && newString !== undefined ? (
              <FileOpsDiffView
                rawFilePath={rawFilePath}
                oldString={oldString}
                newString={newString}
                isFailed={isFailed}
              />
            ) : newString ? (
              <FileOpsContentPreview rawFilePath={rawFilePath} newString={newString} />
            ) : (
              tool.params &&
              Object.keys(tool.params).length > 0 && <ToolParamsView params={params} />
            )
          ) : isDiffTool && (oldString !== undefined || newString !== undefined) ? (
            <FileOpsDiffView
              rawFilePath={rawFilePath}
              oldString={oldString}
              newString={newString}
              isFailed={isFailed}
            />
          ) : (
            tool.params &&
            Object.keys(tool.params).length > 0 && <ToolParamsView params={params} />
          )}

          {!isReadOp && tool.stdout && (
            <div>
              <span className="text-[10px] text-[#666666] dark:text-[#71717a] block mb-1">
                Вывод команды
              </span>
              <pre className="p-2.5 rounded-md bg-[#fafafa] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#212126] text-[11px] text-[#171717] dark:text-[#f4f4f5] overflow-x-auto leading-relaxed max-h-48 whitespace-pre-wrap select-text">
                {tool.stdout}
              </pre>
            </div>
          )}

          {tool.stderr && (
            <div>
              <span className="text-[10px] text-[#b91c1c] dark:text-[#ff7b72] block mb-1">
                Ошибки выполнения
              </span>
              <pre className="p-2.5 rounded-md bg-[#fef2f2] dark:bg-[#3b1219] border border-[#fecaca] dark:border-[#da3633] text-[11px] text-[#991b1b] dark:text-[#ff7b72] overflow-x-auto leading-relaxed max-h-48 whitespace-pre-wrap select-text">
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

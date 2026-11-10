export interface FileOpsDiffViewProps {
  rawFilePath: string;
  oldString?: string;
  newString?: string;
  isFailed?: boolean;
}

export function FileOpsDiffView({
  rawFilePath,
  oldString,
  newString,
  isFailed,
}: FileOpsDiffViewProps) {
  if (oldString !== undefined || newString !== undefined) {
    const oldLines = oldString ? oldString.split('\n') : [];
    const newLines = newString ? newString.split('\n') : [];

    return (
      <div className="space-y-2">
        {rawFilePath && (
          <div className="flex items-baseline space-x-2 py-0.5 text-[11px] font-mono">
            <span className="text-[#666666] dark:text-[#71717a] font-semibold shrink-0">Файл:</span>
            <span className="text-[#171717] dark:text-[#f4f4f5] select-text break-all">{rawFilePath}</span>
          </div>
        )}
        <div>
          <span className="text-[10px] text-[#666666] dark:text-[#71717a] block mb-1">
            {isFailed ? 'Запланированные изменения (не применены)' : 'Дифф изменений'}
          </span>
          <div className="rounded-md border border-[#ebebeb] dark:border-[#212126] bg-[#fafafa] dark:bg-[#111114] overflow-hidden font-mono text-[11px]">
            {rawFilePath && (
              <div className="flex items-center justify-between px-2.5 py-1.5 bg-[#f4f4f5] dark:bg-[#17171b] border-b border-[#ebebeb] dark:border-[#212126] text-[#666666] dark:text-[#71717a] text-[10px]">
                <span className="truncate max-w-[340px] text-[#171717] dark:text-[#f4f4f5] font-semibold">
                  {rawFilePath}
                </span>
                <div className="flex items-center space-x-2 font-mono tabular-nums">
                  {oldLines.length > 0 && (
                    <span className="text-[#dc2626] dark:text-[#ff7b72]">-{oldLines.length}</span>
                  )}
                  {newLines.length > 0 && (
                    <span className="text-[#16a34a] dark:text-[#7ee787]">+{newLines.length}</span>
                  )}
                  <span className="text-[#8f8f8f] dark:text-[#71717a]">Различия</span>
                </div>
              </div>
            )}

            <div className="divide-y divide-[#ebebeb] dark:divide-[#212126] select-text overflow-x-auto max-h-72 font-mono">
              {oldLines.map((line, idx) => (
                <div
                  key={`old-${idx}`}
                  className="flex items-start bg-[#fef2f2] dark:bg-[#3b1219] text-[#991b1b] dark:text-[#ff7b72] px-2 py-0.5 leading-relaxed font-mono"
                >
                  <span className="w-4 shrink-0 text-[#ef4444] dark:text-[#ff7b72] select-none text-center font-bold">
                    -
                  </span>
                  <span className="w-8 shrink-0 text-[#8f8f8f] dark:text-[#71717a] select-none text-right pr-2 tabular-nums">
                    {idx + 1}
                  </span>
                  <span className="whitespace-pre-wrap break-all">
                    {line || ' '}
                  </span>
                </div>
              ))}

              {newLines.map((line, idx) => (
                <div
                  key={`new-${idx}`}
                  className="flex items-start bg-[#f0fdf4] dark:bg-[#12281e] text-[#166534] dark:text-[#7ee787] px-2 py-0.5 leading-relaxed font-mono"
                >
                  <span className="w-4 shrink-0 text-[#22c55e] dark:text-[#7ee787] select-none text-center font-bold">
                    +
                  </span>
                  <span className="w-8 shrink-0 text-[#8f8f8f] dark:text-[#71717a] select-none text-right pr-2 tabular-nums">
                    {idx + 1}
                  </span>
                  <span className="whitespace-pre-wrap break-all">
                    {line || ' '}
                  </span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    );
  }

  return null;
}

export interface FileOpsContentPreviewProps {
  rawFilePath: string;
  newString: string;
}

export function FileOpsContentPreview({ rawFilePath, newString }: FileOpsContentPreviewProps) {
  const lines = newString.split('\n');

  return (
    <div className="space-y-2">
      {rawFilePath && (
        <div className="flex items-baseline space-x-2 py-0.5 text-[11px] font-mono">
          <span className="text-[#666666] dark:text-[#71717a] font-semibold shrink-0">Файл:</span>
          <span className="text-[#171717] dark:text-[#f4f4f5] select-text break-all">{rawFilePath}</span>
        </div>
      )}
      <div>
        <span className="text-[10px] text-[#666666] dark:text-[#71717a] block mb-1">
          Содержимое файла
        </span>
        <div className="rounded-md border border-[#ebebeb] dark:border-[#212126] bg-[#fafafa] dark:bg-[#111114] overflow-hidden font-mono text-[11px]">
          {rawFilePath && (
            <div className="flex items-center justify-between px-2.5 py-1.5 bg-[#f4f4f5] dark:bg-[#17171b] border-b border-[#ebebeb] dark:border-[#212126] text-[#666666] dark:text-[#71717a] text-[10px]">
              <span className="truncate max-w-[340px] text-[#171717] dark:text-[#f4f4f5] font-semibold">
                {rawFilePath}
              </span>
              <span className="text-[#8f8f8f] dark:text-[#71717a] tabular-nums font-mono">
                {lines.length} {lines.length === 1 ? 'строка' : 'строк'}
              </span>
            </div>
          )}
          <div className="divide-y divide-[#f0f0f0] dark:divide-[#212126] select-text overflow-x-auto max-h-72 font-mono bg-white dark:bg-[#111114]">
            {lines.map((line, idx) => (
              <div
                key={`line-${idx}`}
                className="flex items-start px-2 py-0.5 leading-relaxed hover:bg-[#fafafa] dark:hover:bg-[#17171b]"
              >
                <span className="w-8 shrink-0 text-[#a1a1aa] dark:text-[#71717a] select-none text-right pr-2.5 tabular-nums text-[10px]">
                  {idx + 1}
                </span>
                <span className="whitespace-pre-wrap break-all text-[#171717] dark:text-[#f4f4f5]">
                  {line || ' '}
                </span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

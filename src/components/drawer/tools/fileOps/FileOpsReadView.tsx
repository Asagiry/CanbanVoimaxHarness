export interface FileOpsReadViewProps {
  rawFilePath: string;
  stdout?: string;
  startLine?: number;
  lineRangeBadge: string | null;
}

export function FileOpsReadView({
  rawFilePath,
  stdout,
  startLine,
  lineRangeBadge,
}: FileOpsReadViewProps) {
  if (!stdout) return null;
  const rawLines = stdout.split('\n');
  if (rawLines.length > 0 && rawLines[rawLines.length - 1] === '') {
    rawLines.pop();
  }
  if (rawLines.length === 0) return null;

  const baseLineNumber = startLine ?? 1;

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
          Прочитанные строки {lineRangeBadge ? `(${lineRangeBadge})` : ''}
        </span>
        <div className="rounded-md border border-[#ebebeb] dark:border-[#212126] bg-[#fafafa] dark:bg-[#111114] overflow-hidden font-mono text-[11px]">
          {rawFilePath && (
            <div className="flex items-center justify-between px-2.5 py-1.5 bg-[#f4f4f5] dark:bg-[#17171b] border-b border-[#ebebeb] dark:border-[#212126] text-[#666666] dark:text-[#71717a] text-[10px]">
              <span className="truncate max-w-[340px] text-[#171717] dark:text-[#f4f4f5] font-semibold">
                {rawFilePath}
              </span>
              <span className="text-[#8f8f8f] dark:text-[#71717a] tabular-nums font-mono">
                {lineRangeBadge || `${rawLines.length} строк`}
              </span>
            </div>
          )}
          <div className="divide-y divide-[#f0f0f0] dark:divide-[#212126] select-text overflow-x-auto max-h-80 font-mono bg-white dark:bg-[#111114]">
            {rawLines.map((line, idx) => {
              const prefixMatch = line.match(/^(\d+)[:|]\s?(.*)$/);
              let displayLineNum: number | string = baseLineNumber + idx;
              let displayContent = line;

              if (prefixMatch) {
                displayLineNum = parseInt(prefixMatch[1], 10);
                displayContent = prefixMatch[2];
              }

              return (
                <div
                  key={`readline-${idx}`}
                  className="flex items-start px-2 py-0.5 leading-relaxed hover:bg-[#fafafa] dark:hover:bg-[#17171b]"
                >
                  <span className="w-10 shrink-0 text-[#a1a1aa] dark:text-[#71717a] select-none text-right pr-2.5 tabular-nums text-[10px]">
                    {displayLineNum}
                  </span>
                  <span className="whitespace-pre-wrap break-all text-[#171717] dark:text-[#f4f4f5]">
                    {displayContent || ' '}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}

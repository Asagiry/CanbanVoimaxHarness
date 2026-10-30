export interface ContextPopoverProps {
  percent: number;
  usedTokensK: string | number;
  maxTokensK: number;
  systemPct: number;
  toolPct: number;
  msgPct: number;
  systemPromptTokens: number;
  toolDefTokens: number;
  messageTokens: number;
  isCompacting: boolean;
  savingsBadge: string | null;
  onCompactClick: () => void;
}

export function ContextPopover({
  percent,
  usedTokensK,
  maxTokensK,
  systemPct,
  toolPct,
  msgPct,
  systemPromptTokens,
  toolDefTokens,
  messageTokens,
  isCompacting,
  savingsBadge,
  onCompactClick,
}: ContextPopoverProps) {
  return (
    <div className="absolute left-1/2 -translate-x-1/2 bottom-full mb-2.5 z-50 invisible opacity-0 group-hover:visible group-hover:opacity-100 transition-all duration-150 ease-out pointer-events-auto">
      {/* Invisible hit bridge to prevent losing hover on cursor transition */}
      <div className="absolute -bottom-3 left-0 right-0 h-3" />

      <div className="w-72 rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] p-3.5 shadow-md text-[#171717] dark:text-[#f0f6fc] select-none font-sans">
        {/* Top Line */}
        <div className="flex items-center justify-between mb-2">
          <div>
            <span className="font-semibold text-xs text-[#171717] dark:text-[#f0f6fc]">{percent}%</span>{' '}
            <span className="text-xs text-[#666666] dark:text-[#8b949e]">заполнено</span>
          </div>
          <span className="font-mono text-xs text-[#666666] dark:text-[#8b949e] tabular-nums">
            ~{usedTokensK}K / {maxTokensK}K
          </span>
        </div>

        {/* Segmented Progress Bar */}
        <div className="w-full h-2 rounded-full bg-[#f4f4f5] dark:bg-[#21262d] overflow-hidden flex mb-3">
          {systemPromptTokens > 0 && (
            <div
              style={{ width: `${systemPct}%` }}
              className="h-full bg-[#71717a] transition-all duration-200"
              title={`System prompt: ~${systemPromptTokens.toLocaleString('ru-RU')}`}
            />
          )}
          {toolDefTokens > 0 && (
            <div
              style={{ width: `${toolPct}%` }}
              className="h-full bg-[#8b5cf6] transition-all duration-200"
              title={`Tool definitions: ~${toolDefTokens.toLocaleString('ru-RU')}`}
            />
          )}
          {messageTokens > 0 && (
            <div
              style={{ width: `${msgPct}%` }}
              className="h-full bg-[#3b82f6] transition-all duration-200"
              title={`Messages: ~${messageTokens.toLocaleString('ru-RU')}`}
            />
          )}
        </div>

        {/* Legend List */}
        <div className="space-y-1.5 text-xs">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-2">
              <span className="w-2.5 h-2.5 rounded-xs bg-[#71717a] shrink-0" />
              <span className="text-[#666666] dark:text-[#8b949e]">Системный промпт</span>
            </div>
            <span className="font-mono text-xs text-[#171717] dark:text-[#f0f6fc] tabular-nums">
              ~{systemPromptTokens.toLocaleString('ru-RU')}
            </span>
          </div>

          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-2">
              <span className="w-2.5 h-2.5 rounded-xs bg-[#8b5cf6] shrink-0" />
              <span className="text-[#666666] dark:text-[#8b949e]">Схемы инструментов</span>
            </div>
            <span className="font-mono text-xs text-[#171717] dark:text-[#f0f6fc] tabular-nums">
              ~{toolDefTokens.toLocaleString('ru-RU')}
            </span>
          </div>

          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-2">
              <span className="w-2.5 h-2.5 rounded-xs bg-[#3b82f6] shrink-0" />
              <span className="text-[#666666] dark:text-[#8b949e]">Сообщения диалога</span>
            </div>
            <span className="font-mono text-xs text-[#171717] dark:text-[#f0f6fc] tabular-nums">
              ~{messageTokens.toLocaleString('ru-RU')}
            </span>
          </div>
        </div>

        {/* Action Row */}
        <div className="mt-3 pt-2.5 border-t border-[#ebebeb] dark:border-[#21262d] flex items-center justify-between">
          <button
            type="button"
            onClick={onCompactClick}
            disabled={isCompacting}
            className="px-2.5 py-1 rounded-md bg-[#fafafa] dark:bg-[#161b22] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] text-[11px] font-mono transition cursor-pointer disabled:opacity-50 active:scale-[0.98]"
          >
            {isCompacting ? 'Сжатие...' : 'Сжать контекст'}
          </button>

          {savingsBadge && (
            <span className="text-[10px] font-mono text-[#b45309] dark:text-[#f59e0b] truncate max-w-[130px]">
              {savingsBadge}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}

export default ContextPopover;

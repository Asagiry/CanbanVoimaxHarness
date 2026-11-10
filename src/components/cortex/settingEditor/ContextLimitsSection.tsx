import { Maximize2 } from 'lucide-react';
import { useTranslation } from '../../../stores/languageStore';
import { CONTEXT_CHIPS, MAX_TOKENS_CHIPS } from './settingSchema';

export interface ContextLimitsSectionProps {
  contextWindow: number;
  maxTokens: number;
  onContextWindowChange: (newCw: number) => void;
  onMaxTokensChange: (newMt: number) => void;
}

export function ContextLimitsSection({
  contextWindow,
  maxTokens,
  onContextWindowChange,
  onMaxTokensChange,
}: ContextLimitsSectionProps) {
  const { t } = useTranslation();

  return (
    <section className="rounded-lg border border-[#ebebeb] dark:border-[#212126] bg-[#fdfdfd] dark:bg-[#17171b]/70 p-4 space-y-4 shadow-2xs">
      <div className="flex items-center space-x-2 border-b border-[#ebebeb] dark:border-[#212126] pb-2.5">
        <Maximize2 className="h-4 w-4 text-[#10b981] dark:text-[#34d399]" />
        <h2 className="text-xs font-bold font-mono uppercase tracking-wider text-[#171717] dark:text-[#f4f4f5]">
          {t('ruleEditor.limitsSection')}
        </h2>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Размер контекстного окна */}
        <div className="space-y-2 p-3 rounded-md border border-[#f4f4f5] dark:border-[#212126] bg-white dark:bg-[#111114]">
          <div className="flex items-center justify-between">
            <label className="text-xs font-medium text-[#171717] dark:text-[#f4f4f5]">
              {t('ruleEditor.contextWindowLabel')}
            </label>
            <span className="font-mono text-xs font-semibold tabular-nums text-[#6366f1] dark:text-[#818cf8]">
              {contextWindow.toLocaleString()}
            </span>
          </div>

          <input
            type="number"
            min="1000"
            max="2000000"
            step="1000"
            value={contextWindow}
            onChange={(e) => onContextWindowChange(parseInt(e.target.value, 10) || 128000)}
            className="w-full px-2.5 py-1.5 rounded border border-[#d4d4d8] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] text-[#171717] dark:text-[#f4f4f5] font-mono text-xs tabular-nums focus:outline-hidden focus:border-[#10b981] dark:focus:border-[#34d399]"
          />

          {/* Быстрые чипы */}
          <div className="flex flex-wrap gap-1.5 pt-0.5">
            {CONTEXT_CHIPS.map((chip) => {
              const isActive = contextWindow === chip.value;
              return (
                <button
                  key={chip.value}
                  type="button"
                  onClick={() => onContextWindowChange(chip.value)}
                  className={`px-2 py-0.5 rounded text-[11px] font-mono transition cursor-pointer border ${
                    isActive
                      ? 'border-[#10b981] bg-[#10b981]/10 text-[#059669] dark:text-[#34d399] font-bold'
                      : 'border-[#ebebeb] dark:border-[#2c2c33] bg-[#f9f9fb] dark:bg-[#17171b] text-[#666666] dark:text-[#71717a] hover:border-[#d4d4d8]'
                  }`}
                >
                  {chip.label}
                </button>
              );
            })}
          </div>
          <p className="text-[10px] text-[#8f8f8f] dark:text-[#71717a] font-sans leading-tight pt-1">
            {t('ruleEditor.contextWindowHint')}
          </p>
        </div>

        {/* Максимально токенов вывода */}
        <div className="space-y-2 p-3 rounded-md border border-[#f4f4f5] dark:border-[#212126] bg-white dark:bg-[#111114]">
          <div className="flex items-center justify-between">
            <label className="text-xs font-medium text-[#171717] dark:text-[#f4f4f5]">
              {t('ruleEditor.maxTokensLabel')}
            </label>
            <span className="font-mono text-xs font-semibold tabular-nums text-[#10b981] dark:text-[#34d399]">
              {maxTokens.toLocaleString()}
            </span>
          </div>

          <input
            type="number"
            min="256"
            max="131072"
            step="512"
            value={maxTokens}
            onChange={(e) => onMaxTokensChange(parseInt(e.target.value, 10) || 4096)}
            className="w-full px-2.5 py-1.5 rounded border border-[#d4d4d8] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] text-[#171717] dark:text-[#f4f4f5] font-mono text-xs tabular-nums focus:outline-hidden focus:border-[#10b981] dark:focus:border-[#34d399]"
          />

          {/* Быстрые чипы */}
          <div className="flex flex-wrap gap-1.5 pt-0.5">
            {MAX_TOKENS_CHIPS.map((chip) => {
              const isActive = maxTokens === chip.value;
              return (
                <button
                  key={chip.value}
                  type="button"
                  onClick={() => onMaxTokensChange(chip.value)}
                  className={`px-2 py-0.5 rounded text-[11px] font-mono transition cursor-pointer border ${
                    isActive
                      ? 'border-[#10b981] bg-[#10b981]/10 text-[#059669] dark:text-[#34d399] font-bold'
                      : 'border-[#ebebeb] dark:border-[#2c2c33] bg-[#f9f9fb] dark:bg-[#17171b] text-[#666666] dark:text-[#71717a] hover:border-[#d4d4d8]'
                  }`}
                >
                  {chip.label}
                </button>
              );
            })}
          </div>
          <p className="text-[10px] text-[#8f8f8f] dark:text-[#71717a] font-sans leading-tight pt-1">
            {t('ruleEditor.maxTokensHint')}
          </p>
        </div>
      </div>
    </section>
  );
}

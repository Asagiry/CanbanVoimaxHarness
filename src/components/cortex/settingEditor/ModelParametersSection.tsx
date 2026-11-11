import { Bot, Brain } from 'lucide-react';
import type { ReasoningEffort } from '../../../types';
import { REASONING_EFFORTS } from '../../../types';
import { useTranslation } from '../../../stores/languageStore';
import {
  REASONING_EFFORT_LABELS,
  type CatalogModelOption,
} from './settingSchema';

export interface ModelParametersSectionProps {
  model: string;
  reasoningEffort: ReasoningEffort;
  dynamicModelOptions: CatalogModelOption[];
  dynamicModels: string[];
  onModelChange: (newModel: string) => void;
  onReasoningEffortChange: (newEffort: ReasoningEffort) => void;
}

export function ModelParametersSection({
  model,
  reasoningEffort,
  dynamicModelOptions,
  dynamicModels,
  onModelChange,
  onReasoningEffortChange,
}: ModelParametersSectionProps) {
  const { t } = useTranslation();
  const isModelInherit = !model || model === 'inherit';

  return (
    <section className="rounded-lg border border-[#ebebeb] dark:border-[#212126] bg-[#fdfdfd] dark:bg-[#17171b]/70 p-4 space-y-4 shadow-2xs">
      <div className="flex items-center space-x-2 border-b border-[#ebebeb] dark:border-[#212126] pb-2.5">
        <Bot className="h-4 w-4 text-[#0284c7] dark:text-[#38bdf8]" />
        <h2 className="text-xs font-bold font-mono uppercase tracking-wider text-[#171717] dark:text-[#f4f4f5]">
          {t('ruleEditor.modelSection')}
        </h2>
      </div>

      {/* Выбор модели */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <label className="text-xs font-medium text-[#171717] dark:text-[#f4f4f5] flex items-center space-x-1.5">
            <span>{t('ruleEditor.modelLabel')}</span>
            {isModelInherit && (
              <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-[#f4f4f5] dark:bg-[#232328] border border-[#ebebeb] dark:border-[#2c2c33] text-[#666666] dark:text-[#71717a]">
                {t('ruleEditor.modelInherit')}
              </span>
            )}
          </label>
        </div>

        <div className="flex items-center space-x-2">
          <div className="relative flex-1">
            <input
              type="text"
              list="model-options"
              value={model === 'inherit' ? '' : model}
              onChange={(e) => onModelChange(e.target.value || 'inherit')}
              placeholder={t('ruleEditor.modelPlaceholder')}
              className="w-full px-3 py-1.5 rounded-md border border-[#d4d4d8] dark:border-[#2c2c33] bg-white dark:bg-[#111114] text-[#171717] dark:text-[#f4f4f5] font-mono text-xs focus:outline-hidden focus:border-[#0284c7] dark:focus:border-[#38bdf8] transition"
            />
            <datalist id="model-options">
              {dynamicModels.map((m) => (
                <option key={m} value={m} />
              ))}
            </datalist>
          </div>

          {model !== 'inherit' && (
            <button
              type="button"
              onClick={() => onModelChange('inherit')}
              className="px-2.5 py-1.5 rounded-md text-xs font-mono border border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#111114] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] text-[#666666] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] transition cursor-pointer"
            >
              {t('app.zoomReset')}
            </button>
          )}
        </div>

        {/* Быстрые бейджи модели */}
        <div className="flex flex-wrap gap-1.5 pt-1">
          {dynamicModelOptions.map((option) => {
            const isSelected =
              option.id === 'inherit'
                ? isModelInherit
                : model.toLowerCase() === option.id.toLowerCase();
            return (
              <button
                key={option.id}
                type="button"
                onClick={() => onModelChange(option.id)}
                title={option.id}
                className={`px-2.5 py-1 rounded text-xs font-mono transition cursor-pointer flex items-center space-x-1 border ${
                  isSelected
                    ? 'border-[#0284c7] bg-[#0284c7]/10 text-[#0284c7] dark:text-[#38bdf8] font-semibold'
                    : 'border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#111114] text-[#666666] dark:text-[#71717a] hover:border-[#d4d4d8] dark:hover:border-[#3a3a44] hover:text-[#171717] dark:hover:text-[#f4f4f5]'
                }`}
              >
                <span>{option.providerName ? `${option.providerName} / ${option.label}` : option.label}</span>
              </button>
            );
          })}
        </div>
      </div>

      {/* Effort рассуждений */}
      <div className="space-y-2 pt-2 border-t border-[#f4f4f5] dark:border-[#212126]">
        <div className="flex items-center justify-between">
          <label className="text-xs font-medium text-[#171717] dark:text-[#f4f4f5] flex items-center space-x-1.5">
            <Brain className="h-3.5 w-3.5 text-[#0284c7]" />
            <span>{t('ruleEditor.reasoningLabel')}</span>
          </label>
          <span className="font-mono text-xs font-bold tabular-nums px-2 py-0.5 rounded bg-white dark:bg-[#111114] border border-[#d4d4d8] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5]">
            {t(REASONING_EFFORT_LABELS[reasoningEffort])}
          </span>
        </div>

        <div className="flex flex-wrap gap-1.5">
          {REASONING_EFFORTS.map((effort) => {
            const isActive = reasoningEffort === effort;
            return (
              <button
                key={effort}
                type="button"
                onClick={() => onReasoningEffortChange(effort)}
                className={`px-2.5 py-1 rounded text-xs font-mono transition cursor-pointer border ${
                  isActive
                    ? 'border-[#0284c7] bg-[#0284c7]/10 text-[#0284c7] dark:text-[#38bdf8] font-semibold'
                    : 'border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#111114] text-[#666666] dark:text-[#71717a] hover:border-[#d4d4d8] dark:hover:border-[#3a3a44]'
                }`}
              >
                {t(REASONING_EFFORT_LABELS[effort])}
              </button>
            );
          })}
        </div>
      </div>
    </section>
  );
}

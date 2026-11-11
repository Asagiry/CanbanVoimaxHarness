import { Check } from 'lucide-react';
import { formatInline } from './stream';

export interface QuestionOptionItem {
  label: string;
  description?: string;
}

export interface QuestionOptionRowProps {
  option: QuestionOptionItem;
  index: number;
  isSelected: boolean;
  isAnswered: boolean;
  isSubmitting: boolean;
  recommendedLabel: string;
  selectedLabel: string;
  onSelect: (label: string) => void;
}

export function QuestionOptionRow({
  option,
  index,
  isSelected,
  isAnswered,
  isSubmitting,
  recommendedLabel,
  selectedLabel,
  onSelect,
}: QuestionOptionRowProps) {
  const isRecommended =
    option.label.includes('(Рекомендуется)') ||
    option.label.includes('(Recommended)');

  const cleanLabel = option.label
    .replace('(Рекомендуется)', '')
    .replace('(Recommended)', '')
    .trim();

  return (
    <button
      type="button"
      disabled={isAnswered || isSubmitting}
      onClick={() => onSelect(option.label)}
      className={`w-full text-left p-3 rounded-md border transition-colors flex items-start space-x-3 group ${
        isSelected
          ? 'border-[#171717] dark:border-[#f4f4f5] bg-[#f9fafb] dark:bg-[#17171b]'
          : 'border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#111114] hover:border-[#a1a1aa] dark:hover:border-[#52525b]'
      } ${
        isAnswered
          ? 'cursor-default opacity-85'
          : 'cursor-pointer'
      }`}
    >
      {/* Radio circle */}
      <div
        className={`w-4 h-4 rounded-full border flex items-center justify-center shrink-0 mt-0.5 transition-colors ${
          isSelected
            ? 'border-[#171717] dark:border-[#f4f4f5] bg-[#171717] dark:bg-[#f4f4f5]'
            : 'border-[#a1a1aa] dark:border-[#52525b]'
        }`}
      >
        {isSelected && (
          <div className="w-1.5 h-1.5 rounded-full bg-white dark:bg-[#111114]" />
        )}
      </div>

      {/* Title & Subtitle */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="text-sm font-medium text-[#171717] dark:text-[#f4f4f5]">
            {formatInline(cleanLabel, `opt-${index}`)}
          </span>
          {isRecommended && (
            <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-mono leading-none bg-[#f0fdf4] dark:bg-[#12281e] text-[#15803d] dark:text-[#7ee787] border border-[#bbf7d0] dark:border-[#238636]">
              {recommendedLabel}
            </span>
          )}
          {isSelected && isAnswered && (
            <span className="ml-auto flex items-center space-x-1 text-xs text-[#15803d] dark:text-[#7ee787] font-mono">
              <Check className="w-3.5 h-3.5 block shrink-0" />
              <span>{selectedLabel}</span>
            </span>
          )}
        </div>
        {option.description && (
          <p className="text-xs text-[#71717a] dark:text-[#71717a] font-mono mt-1 leading-relaxed">
            {formatInline(option.description, `desc-${index}`)}
          </p>
        )}
      </div>
    </button>
  );
}

import { useState } from 'react';
import { ChevronDown } from 'lucide-react';
import { REASONING_EFFORTS, normalizeReasoningEffort } from '../../types';
import type { ReasoningEffort } from '../../types';
import { useTranslation } from '../../stores/languageStore';
import type { TranslationKey } from '../../i18n/types';

const EFFORT_LABELS: Record<ReasoningEffort, TranslationKey> = {
  off: 'ruleEditor.reasoningOff',
  low: 'ruleEditor.reasoningLow',
  medium: 'ruleEditor.reasoningMedium',
  high: 'ruleEditor.reasoningHigh',
};

export interface EffortSelectorProps {
  taskEffort?: string | null;
  personaEffort?: string | null;
  onSelectEffort: (effort: ReasoningEffort) => void;
  onOpen?: () => void;
}

export function resolveEffort(taskEffort?: string | null, personaEffort?: string | null): ReasoningEffort {
  return normalizeReasoningEffort(taskEffort) ?? normalizeReasoningEffort(personaEffort) ?? 'off';
}

export function effortChip(effort: ReasoningEffort): string {
  const label = effort.charAt(0).toUpperCase() + effort.slice(1).toLowerCase();
  return label === 'High' ? 'Max' : label;
}

export default function EffortSelector({ taskEffort, personaEffort, onSelectEffort, onOpen }: EffortSelectorProps) {
  const [isOpen, setIsOpen] = useState(false);
  const { t } = useTranslation();
  const active = resolveEffort(taskEffort, personaEffort);
  const chip = effortChip(active);

  return (
    <div className="relative flex items-center">
      <button
        type="button"
        onClick={() => {
          const next = !isOpen;
          setIsOpen(next);
          if (next) onOpen?.();
        }}
        className="px-2.5 py-1.5 rounded-full bg-[#f4f4f5] dark:bg-[#17171b] hover:bg-[#e4e4e7] dark:hover:bg-[#232328] text-[#171717] dark:text-[#f4f4f5] text-[13px] border border-[#ebebeb] dark:border-[#2c2c33] transition cursor-pointer flex items-center gap-1.5 font-sans"
        aria-label={t('drawer.selectEffort')}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        title={`Effort: ${chip}`}
      >
        <span className="truncate">{chip}</span>
        <ChevronDown className="h-3 w-3 text-[#8f8f8f] dark:text-[#71717a] shrink-0" />
      </button>
      {isOpen && (
        <div className="absolute bottom-full right-0 mb-2 w-44 rounded-xl bg-white dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] shadow-lg p-1.5 z-30 text-xs" role="listbox">
          <div className="px-2 py-1 text-[#8f8f8f] dark:text-[#71717a] text-[10px] uppercase font-sans">{t('drawer.selectEffort')}</div>
          {REASONING_EFFORTS.map((effort) => (
            <button
              key={effort}
              type="button"
              role="option"
              aria-selected={active === effort}
              onClick={() => {
                onSelectEffort(effort);
                setIsOpen(false);
              }}
              className={`w-full text-left px-2.5 py-2 rounded-lg flex items-center justify-between cursor-pointer font-sans ${
                active === effort ? 'bg-[#f4f4f5] dark:bg-[#232328] text-[#171717] dark:text-[#f4f4f5]' : 'text-[#4d4d4d] dark:text-[#a1a1aa] hover:bg-[#f9f9f9] dark:hover:bg-[#232328]'
              }`}
            >
              <span className="truncate">{t(EFFORT_LABELS[effort])}</span>
              {active === effort && <span className="text-emerald-600 dark:text-emerald-400 shrink-0 ml-1">✓</span>}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

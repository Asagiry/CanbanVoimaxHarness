import { ChevronDown } from 'lucide-react';
import { useTranslation } from '../../../stores/languageStore';
import { GroupedModelOption } from './types';

export interface SteerModelMenuProps {
  modelId: string;
  providerModelLabel: string;
  groupedModels: GroupedModelOption[];
  isOpen: boolean;
  onToggle: () => void;
  onSelectPair: (providerId: string, modelId: string) => void;
}

export function SteerModelMenu({
  modelId,
  providerModelLabel,
  groupedModels,
  isOpen,
  onToggle,
  onSelectPair,
}: SteerModelMenuProps) {
  const { t } = useTranslation();

  return (
    <div className="relative flex items-center gap-2">
      <button
        type="button"
        onClick={onToggle}
        className="px-3 py-1.5 rounded-full bg-[#f4f4f5] dark:bg-[#17171b] hover:bg-[#e4e4e7] dark:hover:bg-[#232328] text-[#171717] dark:text-[#f4f4f5] text-[13px] border border-[#ebebeb] dark:border-[#2c2c33] transition cursor-pointer flex items-center gap-1.5 max-w-[280px] font-sans"
        aria-label={t('drawer.taskModel', { model: modelId })}
      >
        <span className="truncate">{providerModelLabel}</span>
        <ChevronDown className="h-3 w-3 block shrink-0 text-[#8f8f8f] dark:text-[#71717a]" />
      </button>

      {isOpen && (
        <div className="absolute bottom-full right-0 mb-2 w-64 rounded-xl bg-white dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] shadow-lg p-1.5 z-30 text-xs max-h-60 overflow-y-auto">
          <div className="px-2 py-1 text-[#8f8f8f] dark:text-[#71717a] text-[10px] uppercase font-sans">
            {t('drawer.selectModel')}
          </div>
          {groupedModels.map((m) => (
            <button
              key={`${m.providerId}-${m.id}`}
              type="button"
              onClick={() => {
                onSelectPair(m.providerId, m.id);
              }}
              className={`w-full text-left px-2.5 py-2 rounded-lg flex items-center justify-between cursor-pointer font-sans ${
                modelId === m.id
                  ? 'bg-[#f4f4f5] dark:bg-[#232328] text-[#171717] dark:text-[#f4f4f5]'
                  : 'text-[#4d4d4d] dark:text-[#a1a1aa] hover:bg-[#f9f9f9] dark:hover:bg-[#232328]'
              }`}
            >
              <span className="truncate">{m.label}</span>
              {modelId === m.id && (
                <span className="text-emerald-600 dark:text-emerald-400 shrink-0 ml-1">✓</span>
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

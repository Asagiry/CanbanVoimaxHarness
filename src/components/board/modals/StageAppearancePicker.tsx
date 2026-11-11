import { Check } from 'lucide-react';
import { useTranslation } from '../../../stores/languageStore';
import AgentIcon from '../../common/AgentIcon';
import { AGENT_PALETTE, AGENT_ICONS } from '../../../lib/agentPalette';

interface StageAppearancePickerProps {
  selectedColor: string | null;
  autoColor: string;
  effectiveColor: string;
  onSelectColor: (color: string | null) => void;
  selectedIcon: string | null;
  autoIcon: string;
  effectiveIcon: string;
  onSelectIcon: (icon: string | null) => void;
}

export default function StageAppearancePicker({
  selectedColor,
  autoColor,
  effectiveColor,
  onSelectColor,
  selectedIcon,
  autoIcon,
  effectiveIcon,
  onSelectIcon,
}: StageAppearancePickerProps) {
  const { t } = useTranslation();

  return (
    <>
      <div>
        <div className="flex items-center justify-between mb-1.5">
          <label className="block font-sans text-[#4d4d4d] dark:text-[#a1a1aa] font-medium">
            {t('board.autoColor')}
          </label>
          {selectedColor && (
            <button
              type="button"
              onClick={() => onSelectColor(null)}
              className="text-[10px] text-[#71717a] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] underline cursor-pointer"
            >
              {t('board.autoReset', { value: autoColor })}
            </button>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-2 p-2 rounded-md bg-[#fafafa] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33]">
          {AGENT_PALETTE.map((c) => {
            const isSelected = effectiveColor === c;
            return (
              <button
                key={c}
                type="button"
                onClick={() => onSelectColor(c)}
                className="relative w-6 h-6 rounded-full flex items-center justify-center transition-transform hover:scale-110 cursor-pointer shadow-2xs border border-black/10 dark:border-white/10"
                style={{ backgroundColor: c }}
                aria-label={c}
              >
                {isSelected && <Check className="h-3.5 w-3.5 text-white drop-shadow" />}
              </button>
            );
          })}
        </div>
      </div>

      <div>
        <div className="flex items-center justify-between mb-1.5">
          <label className="block font-sans text-[#4d4d4d] dark:text-[#a1a1aa] font-medium">
            {t('board.iconLabel', { icon: effectiveIcon })}
          </label>
          {selectedIcon && (
            <button
              type="button"
              onClick={() => onSelectIcon(null)}
              className="text-[10px] text-[#71717a] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] underline cursor-pointer"
            >
              {t('board.autoReset', { value: autoIcon })}
            </button>
          )}
        </div>
        <div className="p-2 rounded-md bg-[#fafafa] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33]">
          <div className="grid grid-cols-6 gap-1.5 max-h-32 overflow-y-auto p-0.5">
            {AGENT_ICONS.map((ic) => {
              const isSelected = effectiveIcon === ic;
              return (
                <button
                  key={ic}
                  type="button"
                  onClick={() => onSelectIcon(ic)}
                  className={`flex items-center justify-center p-1.5 rounded-md transition cursor-pointer ${
                    isSelected
                      ? 'bg-white dark:bg-[#232328] border-2 border-[#171717] dark:border-[#f4f4f5] shadow-xs'
                      : 'hover:bg-white/80 dark:hover:bg-[#232328]/80 border border-transparent text-[#71717a] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5]'
                  }`}
                  aria-label={ic}
                >
                  <AgentIcon
                    name={ic}
                    className="h-4 w-4"
                    style={{ color: isSelected ? effectiveColor : undefined }}
                  />
                </button>
              );
            })}
          </div>
        </div>
      </div>
    </>
  );
}

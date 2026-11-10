import { Palette, Check } from 'lucide-react';
import AgentIcon from '../../common/AgentIcon';
import {
  AGENT_PALETTE,
  AGENT_ICONS,
  getRoleBadgeStyle,
} from '../../../lib/agentPalette';
import { useTranslation } from '../../../stores/languageStore';

export interface CustomizationSectionProps {
  role?: string;
  icon: string;
  color: string;
  onIconChange: (newIcon: string) => void;
  onColorChange: (newColor: string) => void;
}

export function CustomizationSection({
  role,
  icon,
  color,
  onIconChange,
  onColorChange,
}: CustomizationSectionProps) {
  const { t } = useTranslation();

  return (
    <section className="rounded-lg border border-[#ebebeb] dark:border-[#212126] bg-[#fdfdfd] dark:bg-[#17171b]/70 p-4 space-y-4 shadow-2xs">
      <div className="flex items-center justify-between border-b border-[#ebebeb] dark:border-[#212126] pb-2.5">
        <div className="flex items-center space-x-2">
          <Palette className="h-4 w-4" style={{ color }} />
          <h2 className="text-xs font-bold font-mono uppercase tracking-wider text-[#171717] dark:text-[#f4f4f5]">
            {t('ruleEditor.customizationSection')}
          </h2>
        </div>

        {/* Live Preview бейдж */}
        <div className="flex items-center space-x-2">
          <span className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#71717a]">
            {t('ruleEditor.previewLabel')}:
          </span>
          <div
            className="inline-flex items-center space-x-1.5 px-2.5 py-1 rounded-md text-xs font-mono font-medium border transition-colors shadow-2xs"
            style={getRoleBadgeStyle(role, color)}
          >
            <AgentIcon name={icon} className="h-3.5 w-3.5 shrink-0" style={{ color }} />
            <span className="truncate max-w-[120px]">{role || 'agent'}</span>
          </div>
        </div>
      </div>

      {/* Палитра цветов */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <label className="text-xs font-medium text-[#171717] dark:text-[#f4f4f5] flex items-center space-x-1.5">
            <span>{t('ruleEditor.colorLabel')}</span>
          </label>
          <span className="text-[10px] font-mono font-semibold text-[#666666] dark:text-[#71717a] px-1.5 py-0.5 rounded bg-white dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33]">
            {color}
          </span>
        </div>

        <div className="p-2.5 rounded-md border border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#111114] space-y-2">
          <div className="flex flex-wrap gap-2 items-center">
            {AGENT_PALETTE.map((c) => {
              const isSelected = color.toLowerCase() === c.toLowerCase();
              return (
                <button
                  key={c}
                  type="button"
                  onClick={() => onColorChange(c)}
                  aria-label={c}
                  className={`h-7 w-7 rounded-full transition-transform cursor-pointer flex items-center justify-center shadow-2xs ${
                    isSelected
                      ? 'scale-110 ring-2 ring-offset-2 ring-[#171717] dark:ring-[#f4f4f5]'
                      : 'hover:scale-105'
                  }`}
                  style={{ backgroundColor: c }}
                >
                  {isSelected && <Check className="h-3.5 w-3.5 text-white drop-shadow-sm" />}
                </button>
              );
            })}
          </div>
          <p className="text-[10px] font-sans text-[#8f8f8f] dark:text-[#71717a] leading-tight">
            {t('cortex.colorNotice')}
          </p>
        </div>
      </div>

      {/* Сетка SVG иконок */}
      <div className="space-y-2 pt-2 border-t border-[#f4f4f5] dark:border-[#212126]">
        <div className="flex items-center justify-between">
          <label className="text-xs font-medium text-[#171717] dark:text-[#f4f4f5] flex items-center space-x-1.5">
            <span>{t('ruleEditor.iconLabel')}</span>
          </label>
          <span className="text-[10px] font-mono font-semibold text-[#666666] dark:text-[#71717a] px-1.5 py-0.5 rounded bg-white dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33]">
            [{icon}]
          </span>
        </div>

        <div className="p-2 rounded-md border border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#111114]">
          <div className="grid grid-cols-6 sm:grid-cols-9 gap-1.5 max-h-36 overflow-y-auto p-1">
            {AGENT_ICONS.map((ic) => {
              const isSelected = icon === ic;
              return (
                <button
                  key={ic}
                  type="button"
                  onClick={() => onIconChange(ic)}
                  aria-label={ic}
                  className={`flex flex-col items-center justify-center p-2 rounded-md transition cursor-pointer ${
                    isSelected
                      ? 'bg-[#f4f4f5] dark:bg-[#232328] border-2 border-[#171717] dark:border-[#f4f4f5] shadow-2xs'
                      : 'hover:bg-[#f9f9fb] dark:hover:bg-[#1e1e23] border border-transparent text-[#71717a] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5]'
                  }`}
                >
                  <AgentIcon
                    name={ic}
                    className="h-4 w-4"
                    style={{ color: isSelected ? color : undefined }}
                  />
                </button>
              );
            })}
          </div>
        </div>
      </div>
    </section>
  );
}

import { Plus } from 'lucide-react';
import { useTranslation } from '../../../stores/languageStore';

export interface SettingMissingBlockProps {
  onCreate: () => void;
}

export function SettingMissingBlock({ onCreate }: SettingMissingBlockProps) {
  const { t } = useTranslation();
  return (
    <div className="px-2 py-1.5 rounded-md border border-dashed border-[#d4d4d4] dark:border-[#30363d] bg-white dark:bg-[#161b22] space-y-1">
      <div className="text-[11px] font-sans text-[#666666] dark:text-[#8b949e]">
        {t('cortex.settingNotConfigured')}
      </div>
      <button
        onClick={onCreate}
        className="flex items-center space-x-1 px-2 py-1 rounded-md text-[11px] font-mono bg-[#171717] dark:bg-[#21262d] hover:bg-[#262626] dark:hover:bg-[#30363d] text-white dark:text-[#f0f6fc] transition cursor-pointer"
        aria-label={t('cortex.createSettingTemplate')}
      >
        <Plus className="h-3 w-3 shrink-0" />
        <span>{t('cortex.createTemplate')}</span>
      </button>
    </div>
  );
}

export default SettingMissingBlock;

import React from 'react';
import { Sun, Moon, Settings, Wrench } from 'lucide-react';
import WindowControls from '../layout/WindowControls';
import type { TranslationKey } from '../../stores/languageStore';

export interface DrawerHeaderControlsProps {
  onOpenToolPolicy: () => void;
  onOpenSettings: () => void;
  toggleLanguage: () => void;
  language: string;
  theme: string;
  toggleThemeWithAnimation: (e: React.MouseEvent) => void;
  t: (key: TranslationKey, params?: Record<string, string | number>) => string;
}

export default function DrawerHeaderControls({
  onOpenToolPolicy,
  onOpenSettings,
  toggleLanguage,
  language,
  theme,
  toggleThemeWithAnimation,
  t,
}: DrawerHeaderControlsProps) {
  return (
    <div className="flex items-center h-full space-x-2 shrink-0 select-none pointer-events-auto pr-0" data-tauri-drag-region>
      <div className="flex items-center space-x-2 select-none pointer-events-auto" data-no-drag>
        <button
          data-no-drag
          type="button"
          onMouseDown={(e) => e.stopPropagation()}
          onClick={onOpenToolPolicy}
          className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#17171b] dark:hover:bg-[#232328] border border-[#ebebeb] dark:border-[#2c2c33] text-[#666666] hover:text-[#171717] dark:text-[#71717a] dark:hover:text-[#f4f4f5] transition cursor-pointer select-none pointer-events-auto btn-tactile"
          aria-label={t('topbar.toolPolicyTitle')}
        >
          <Wrench className="h-3.5 w-3.5" />
        </button>
        <button
          data-no-drag
          type="button"
          onMouseDown={(e) => e.stopPropagation()}
          onClick={onOpenSettings}
          className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#17171b] dark:hover:bg-[#232328] border border-[#ebebeb] dark:border-[#2c2c33] text-[#666666] hover:text-[#171717] dark:text-[#71717a] dark:hover:text-[#f4f4f5] transition cursor-pointer select-none pointer-events-auto btn-tactile"
          aria-label={t('topbar.llmSettings')}
        >
          <Settings className="h-3.5 w-3.5" />
        </button>
        <button
          data-no-drag
          type="button"
          onMouseDown={(e) => e.stopPropagation()}
          onClick={toggleLanguage}
          className="px-2 py-1 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#17171b] dark:hover:bg-[#232328] border border-[#ebebeb] dark:border-[#2c2c33] text-[#666666] hover:text-[#171717] dark:text-[#71717a] dark:hover:text-[#f4f4f5] transition cursor-pointer font-mono font-semibold text-[11px] select-none flex items-center space-x-1 btn-tactile"
          aria-label={t('topbar.switchLanguage')}
        >
          <span className={language === 'en' ? 'text-[#171717] dark:text-[#f4f4f5] font-bold' : 'text-[#8f8f8f] dark:text-[#52525b]'}>EN</span>
          <span className="text-[#d4d4d8] dark:text-[#2c2c33]">/</span>
          <span className={language === 'ru' ? 'text-[#171717] dark:text-[#f4f4f5] font-bold' : 'text-[#8f8f8f] dark:text-[#52525b]'}>RU</span>
        </button>
        <button
          data-no-drag
          type="button"
          onMouseDown={(e) => e.stopPropagation()}
          onClick={(e) => toggleThemeWithAnimation(e)}
          className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] dark:bg-[#17171b] dark:hover:bg-[#232328] border border-[#ebebeb] dark:border-[#2c2c33] text-[#666666] hover:text-[#171717] dark:text-[#71717a] dark:hover:text-[#f4f4f5] transition cursor-pointer relative overflow-hidden group select-none pointer-events-auto btn-tactile"
          aria-label={t('topbar.switchTheme')}
        >
          <div className="relative h-3.5 w-3.5 flex items-center justify-center">
            <Sun className={`h-3.5 w-3.5 transition-all duration-300 transform absolute inset-0 ${theme === 'dark' ? 'rotate-90 scale-0 opacity-0' : 'rotate-0 scale-100 opacity-100 text-amber-500'}`} />
            <Moon className={`h-3.5 w-3.5 transition-all duration-300 transform absolute inset-0 ${theme === 'dark' ? 'rotate-0 scale-100 opacity-100 text-sky-400' : '-rotate-90 scale-0 opacity-0'}`} />
          </div>
        </button>
        <div className="h-4 w-px bg-[#ebebeb] dark:bg-[#212126] mx-1" />
        <div className="select-none pointer-events-auto" data-no-drag>
          <WindowControls />
        </div>
      </div>
    </div>
  );
}

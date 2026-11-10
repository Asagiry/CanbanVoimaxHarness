import { ArrowRightToLine, ArrowRightFromLine } from 'lucide-react';
import { useTranslation } from '../../../stores/languageStore';

export interface StageMainMenuProps {
  isProtected: boolean;
  canChangeAgent: boolean;
  currentEnterScript: string;
  currentLeaveScript: string;
  onOpenRename: () => void;
  onOpenPersona: () => void;
  onOpenEnterScript: () => void;
  onOpenLeaveScript: () => void;
  onDelete: () => void;
}

export function StageMainMenu({
  isProtected,
  canChangeAgent,
  currentEnterScript,
  currentLeaveScript,
  onOpenRename,
  onOpenPersona,
  onOpenEnterScript,
  onOpenLeaveScript,
  onDelete,
}: StageMainMenuProps) {
  const { t } = useTranslation();

  return (
    <>
      <button
        type="button"
        role="menuitem"
        onClick={onOpenRename}
        className="w-full px-3 py-1.5 text-left text-[#171717] dark:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] cursor-pointer"
      >
        {t('columnMenu.rename')}
      </button>
      {canChangeAgent && (
        <button
          type="button"
          role="menuitem"
          onClick={onOpenPersona}
          className="w-full px-3 py-1.5 text-left text-[#171717] dark:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] cursor-pointer"
        >
          {t('columnMenu.changeAgent')}
        </button>
      )}
      <button
        type="button"
        role="menuitem"
        onClick={onOpenEnterScript}
        className="w-full px-3 py-1.5 text-left hover:bg-[#f4f4f5] dark:hover:bg-[#232328] cursor-pointer flex items-center justify-between"
      >
        <span className="flex items-center gap-1.5 min-w-0">
          <ArrowRightToLine className="h-3.5 w-3.5 block shrink-0 text-emerald-600 dark:text-emerald-400 stroke-[2.2]" />
          <span>{t('columnMenu.enterScript')}</span>
        </span>
        {currentEnterScript && (
          <span className="ml-2 font-mono text-[10px] text-[#8f8f8f] dark:text-[#71717a] truncate max-w-[120px]">
            {currentEnterScript.split(/[\\/]/).pop()}
          </span>
        )}
      </button>
      <button
        type="button"
        role="menuitem"
        onClick={onOpenLeaveScript}
        className="w-full px-3 py-1.5 text-left hover:bg-[#f4f4f5] dark:hover:bg-[#232328] cursor-pointer flex items-center justify-between"
      >
        <span className="flex items-center gap-1.5 min-w-0">
          <ArrowRightFromLine className="h-3.5 w-3.5 block shrink-0 text-amber-600 dark:text-amber-400 stroke-[2.2]" />
          <span>{t('columnMenu.leaveScript')}</span>
        </span>
        {currentLeaveScript && (
          <span className="ml-2 font-mono text-[10px] text-[#8f8f8f] dark:text-[#71717a] truncate max-w-[120px]">
            {currentLeaveScript.split(/[\\/]/).pop()}
          </span>
        )}
      </button>
      <div className="my-1 border-t border-[#ebebeb] dark:border-[#212126]" />
      <button
        type="button"
        role="menuitem"
        disabled={isProtected}
        title={isProtected ? t('columnMenu.protectedNotice') : undefined}
        onClick={onDelete}
        className={`w-full px-3 py-1.5 text-left ${
          isProtected
            ? 'text-[#8f8f8f] dark:text-[#71717a] cursor-not-allowed'
            : 'text-[#b91c1c] dark:text-[#f87171] hover:bg-[#fef2f2] dark:hover:bg-[#450a0a]/40 cursor-pointer'
        }`}
      >
        {t('columnMenu.delete')}
      </button>
    </>
  );
}

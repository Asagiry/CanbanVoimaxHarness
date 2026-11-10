import { ArrowRightToLine, ArrowRightFromLine } from 'lucide-react';
import { useTranslation } from '../../../stores/languageStore';

export interface StageScriptViewProps {
  scriptType: 'on_enter' | 'on_leave';
  currentEnterScript: string;
  currentLeaveScript: string;
  draftScript: string;
  setDraftScript: (val: string) => void;
  scriptError: string | null;
  scriptSaving: boolean;
  onSave: () => void;
  onCancel: () => void;
}

export function StageScriptView({
  scriptType,
  currentEnterScript,
  currentLeaveScript,
  draftScript,
  setDraftScript,
  scriptError,
  scriptSaving,
  onSave,
  onCancel,
}: StageScriptViewProps) {
  const { t } = useTranslation();
  const currentScript = (scriptType === 'on_enter' ? currentEnterScript : currentLeaveScript) || t('columnMenu.notSet');

  return (
    <div className="px-3 py-2">
      <div className="text-[#4d4d4d] dark:text-[#a1a1aa] font-medium mb-1 flex items-center gap-1.5">
        {scriptType === 'on_enter' ? (
          <ArrowRightToLine className="h-3.5 w-3.5 text-emerald-600 dark:text-emerald-400 shrink-0 stroke-[2.2]" />
        ) : (
          <ArrowRightFromLine className="h-3.5 w-3.5 text-amber-600 dark:text-amber-400 shrink-0 stroke-[2.2]" />
        )}
        <span>{scriptType === 'on_enter' ? t('columnMenu.enterScript') : t('columnMenu.leaveScript')}</span>
      </div>
      <div
        className="mb-1.5 text-[#8f8f8f] dark:text-[#71717a] truncate font-mono text-[10px]"
        title={currentScript}
      >
        {t('columnMenu.currentScript', { script: currentScript })}
      </div>
      <input
        type="text"
        autoFocus
        value={draftScript}
        onChange={(e) => setDraftScript(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault();
            onSave();
          }
        }}
        placeholder="scripts/setup.bat, .py, .ps1, .sh"
        className="w-full px-2.5 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#8f8f8f] dark:placeholder:text-[#52525b] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] focus:bg-white dark:focus:bg-[#17171b] font-mono text-xs transition"
      />
      <div className="mt-1 text-[10px] text-[#8f8f8f] dark:text-[#71717a]">
        {scriptType === 'on_enter' ? t('columnMenu.enterScriptHint') : t('columnMenu.leaveScriptHint')}
      </div>
      {scriptError && (
        <div className="mt-2 text-xs text-red-700 bg-red-50 border border-red-200 rounded-md px-2.5 py-1.5">
          {scriptError}
        </div>
      )}
      <div className="mt-2 flex items-center justify-end space-x-2">
        <button
          type="button"
          onClick={onCancel}
          className="px-2.5 py-1.5 rounded-md bg-white dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] text-[#171717] dark:text-[#f4f4f5] transition cursor-pointer"
        >
          {t('common.back')}
        </button>
        <button
          type="button"
          disabled={scriptSaving}
          onClick={onSave}
          className="px-3 py-1.5 rounded-md bg-[#171717] dark:bg-[#232328] hover:bg-[#262626] dark:hover:bg-[#2c2c33] text-white font-medium transition cursor-pointer disabled:opacity-50 disabled:cursor-default border border-transparent dark:border-[#2c2c33]"
        >
          {t('common.save')}
        </button>
      </div>
    </div>
  );
}

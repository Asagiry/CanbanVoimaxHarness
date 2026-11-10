import { useTranslation } from '../../../stores/languageStore';

export interface StageRenameViewProps {
  draftTitle: string;
  setDraftTitle: (val: string) => void;
  renameError: string | null;
  renameSaving: boolean;
  onSave: () => void;
  onCancel: () => void;
}

export function StageRenameView({
  draftTitle,
  setDraftTitle,
  renameError,
  renameSaving,
  onSave,
  onCancel,
}: StageRenameViewProps) {
  const { t } = useTranslation();

  return (
    <div className="px-3 py-2">
      <div className="text-[#4d4d4d] dark:text-[#a1a1aa] font-medium mb-1">
        {t('columnMenu.newTitle')}
      </div>
      <input
        type="text"
        autoFocus
        value={draftTitle}
        onChange={(e) => setDraftTitle(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault();
            onSave();
          }
        }}
        className="w-full px-2.5 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] focus:bg-white dark:focus:bg-[#17171b] transition"
      />
      {renameError && (
        <div className="mt-2 text-xs text-red-700 bg-red-50 border border-red-200 rounded-md px-2.5 py-1.5">
          {renameError}
        </div>
      )}
      <div className="mt-2 flex items-center justify-end space-x-2">
        <button
          type="button"
          onClick={onCancel}
          className="px-2.5 py-1.5 rounded-md bg-white dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] transition cursor-pointer text-[#171717] dark:text-[#f4f4f5]"
        >
          {t('common.cancel')}
        </button>
        <button
          type="button"
          disabled={!draftTitle.trim() || renameSaving}
          onClick={onSave}
          className="px-3 py-1.5 rounded-md bg-[#171717] dark:bg-[#232328] hover:bg-[#262626] dark:hover:bg-[#2c2c33] text-white font-medium transition cursor-pointer disabled:opacity-50 disabled:cursor-default border border-transparent dark:border-[#2c2c33]"
        >
          {t('common.save')}
        </button>
      </div>
    </div>
  );
}

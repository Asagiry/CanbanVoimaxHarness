import { useTranslation } from '../../stores/languageStore';

interface GlobalRuleCreateFormProps {
  isCreatingFolder: boolean;
  createFolder?: string;
  folderName: string;
  ruleName: string;
  createError: string | null;
  isSubmitting: boolean;
  onFolderNameChange: (val: string) => void;
  onRuleNameChange: (val: string) => void;
  onSubmit: (e: React.FormEvent) => void;
  onCancel: () => void;
}

export function GlobalRuleCreateForm({
  isCreatingFolder,
  createFolder,
  folderName,
  ruleName,
  createError,
  isSubmitting,
  onFolderNameChange,
  onRuleNameChange,
  onSubmit,
  onCancel,
}: GlobalRuleCreateFormProps) {
  const { t } = useTranslation();

  return (
    <form
      onSubmit={onSubmit}
      className="mx-1 mt-1 p-2 rounded-md border border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] space-y-1.5 shadow-xs"
    >
      <div className="text-[10px] font-sans font-medium text-[#71717a] dark:text-[#71717a]">
        {isCreatingFolder
          ? t('cortex.newFolder')
          : createFolder
          ? `${t('cortex.newRuleInFolder')} (${createFolder})`
          : t('cortex.newGlobalRule')}
      </div>
      {isCreatingFolder ? (
        <input
          type="text"
          value={folderName}
          onChange={(e) => onFolderNameChange(e.target.value)}
          placeholder={t('cortex.folderNamePlaceholder')}
          autoFocus
          className="w-full px-2 py-1 text-xs rounded-md bg-[#fafafa] dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] font-mono focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5]"
        />
      ) : (
        <input
          type="text"
          value={ruleName}
          onChange={(e) => onRuleNameChange(e.target.value)}
          placeholder="architecture.md"
          autoFocus
          className="w-full px-2 py-1 text-xs rounded-md bg-[#fafafa] dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] font-mono focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5]"
        />
      )}
      {createError && (
        <div className="text-[10px] text-red-600 dark:text-red-400 font-sans">
          {createError}
        </div>
      )}
      <div className="flex items-center gap-1.5 pt-1">
        <button
          type="submit"
          disabled={isSubmitting}
          className="px-2 py-0.5 text-[10px] font-mono rounded bg-[#171717] dark:bg-[#238636] hover:bg-[#262626] dark:hover:bg-[#2ea043] text-white disabled:opacity-50 cursor-pointer"
        >
          {isSubmitting ? t('common.saving') : t('common.create')}
        </button>
        <button
          type="button"
          onClick={onCancel}
          className="px-2 py-0.5 text-[10px] font-mono rounded border border-[#ebebeb] dark:border-[#2c2c33] text-[#71717a] dark:text-[#71717a] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] cursor-pointer"
        >
          {t('common.cancel')}
        </button>
      </div>
    </form>
  );
}

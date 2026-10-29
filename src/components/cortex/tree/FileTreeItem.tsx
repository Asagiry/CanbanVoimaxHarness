import type { CortexAgentFile } from '../../../types';
import { useTranslation } from '../../../stores/languageStore';

export interface FileTreeItemProps {
  file: CortexAgentFile;
  displayName: string;
  isActive: boolean;
  icon: React.ReactNode;
  badge?: string | null;
  onSelect: () => void;
}

export function FileTreeItem({
  file,
  displayName,
  isActive,
  icon,
  badge,
  onSelect,
}: FileTreeItemProps) {
  const { t } = useTranslation();
  return (
    <button
      onClick={onSelect}
      className={`w-full flex items-center justify-between px-2 py-1 rounded-md text-left transition cursor-pointer ${
        isActive
          ? 'bg-white dark:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] font-semibold shadow-xs'
          : 'text-[#4d4d4d] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#ebebeb]/60 dark:hover:bg-[#21262d]'
      }`}
      title={file.path}
    >
      <div className="flex items-center space-x-1.5 min-w-0">
        {icon}
        <span className="text-xs font-mono truncate">{displayName}</span>
        {badge && (
          <span className="px-1 py-px rounded border border-[#ebebeb] dark:border-[#30363d] bg-[#f4f4f5] dark:bg-[#161b22] text-[10px] font-mono text-[#666666] dark:text-[#8b949e] shrink-0">
            {badge}
          </span>
        )}
      </div>

      {file.isDirty && (
        <span className="h-1.5 w-1.5 rounded-full bg-amber-500 shrink-0" title={t('cortex.unsavedChanges')} />
      )}
    </button>
  );
}

export default FileTreeItem;

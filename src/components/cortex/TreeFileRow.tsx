import React from 'react';
import type { CortexAgentFile } from '../../types';
import { useTranslation } from '../../stores/languageStore';
import { useCortexStore } from '../../stores/cortexStore';
import { isFileDirty } from '../../stores/cortex/cortexGitSave';

export interface TreeFileRowProps {
  file: CortexAgentFile;
  isActive: boolean;
  icon: React.ReactNode;
  badge?: React.ReactNode;
  title?: string;
  onSelect: () => void;
  onContextMenu?: (e: React.MouseEvent) => void;
}

export function TreeFileRow({
  file,
  isActive,
  icon,
  badge,
  title,
  onSelect,
  onContextMenu,
}: TreeFileRowProps) {
  const { t } = useTranslation();
  const cvhGitStatus = useCortexStore((s) => s.cvhGitStatus);
  const dirty = isFileDirty(file, cvhGitStatus);

  return (
    <button
      type="button"
      onClick={onSelect}
      onContextMenu={onContextMenu}
      className={`w-full flex items-center justify-between px-2 py-1 rounded-md text-left transition cursor-pointer ${
        isActive
          ? 'bg-white dark:bg-[#232328] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] font-semibold shadow-xs'
          : 'text-[#4d4d4d] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#ebebeb]/60 dark:hover:bg-[#232328]'
      }`}
      title={title ?? file.path}
    >
      <div className="flex items-center space-x-1.5 min-w-0">
        {icon}
        <span
          className={`text-xs font-mono truncate leading-none ${file.isDeleted ? 'line-through text-[#a1a1aa]' : ''}`}
          title={file.isDeleted ? t('cortex.deletedUnsaved') : undefined}
        >
          {file.name}
        </span>
        {typeof badge === 'string' ? (
          <span className="px-1 py-px rounded border border-[#ebebeb] dark:border-[#2c2c33] bg-[#f4f4f5] dark:bg-[#17171b] text-[10px] font-mono text-[#666666] dark:text-[#71717a] shrink-0">
            {badge}
          </span>
        ) : (
          badge
        )}
      </div>

      {dirty && (
        <span
          className="h-1.5 w-1.5 rounded-full bg-amber-500 shrink-0"
          title={t('cortex.unsavedChanges')}
        />
      )}
    </button>
  );
}

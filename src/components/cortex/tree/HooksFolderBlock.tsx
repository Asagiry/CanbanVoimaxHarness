import { Folder, FolderOpen, Zap, Plus } from 'lucide-react';
import type { CortexAgentFile } from '../../../types';
import { useTranslation } from '../../../stores/languageStore';
import { hookMomentOf } from '../types';
import FileTreeItem from './FileTreeItem';

export interface HooksFolderBlockProps {
  files: CortexAgentFile[];
  isOpen: boolean;
  isSearching: boolean;
  activeFileId: string | null;
  onToggle: () => void;
  onNew?: () => void;
  onOpen: (fileId: string) => void;
}

export function HooksFolderBlock({
  files,
  isOpen,
  isSearching,
  activeFileId,
  onToggle,
  onNew,
  onOpen,
}: HooksFolderBlockProps) {
  const { t } = useTranslation();
  if (files.length === 0 && isSearching) return null;
  return (
    <div className="space-y-0.5">
      <div className="group/folder flex items-center gap-0.5">
        <button
          onClick={onToggle}
          className="flex-1 min-w-0 flex items-center space-x-1.5 px-2 py-1 rounded-md text-left text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#ebebeb]/60 dark:hover:bg-[#21262d] transition cursor-pointer"
        >
          {isOpen ? (
            <FolderOpen className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
          ) : (
            <Folder className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
          )}
          <span className="text-xs font-mono">{t('cortex.hooks')}</span>
          <span className="text-[10px] font-mono text-[#8f8f8f]">hooks/</span>
          <span className="text-[10px] text-[#8f8f8f] tabular-nums">({files.length})</span>
        </button>
        {onNew && (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onNew();
            }}
            aria-label={t('cortex.newHook')}
            className="p-1 rounded-md text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#ebebeb]/60 dark:hover:bg-[#21262d] transition cursor-pointer opacity-0 group-hover/folder:opacity-100 focus:opacity-100 focus-visible:opacity-100 shrink-0"
          >
            <Plus className="h-3 w-3" />
          </button>
        )}
      </div>

      {isOpen && files.length > 0 && (
        <div className="pl-4 space-y-0.5 border-l border-[#ebebeb] dark:border-[#21262d] ml-2">
          {files.map((hookFile) => {
            const isBuiltin = hookFile.path.startsWith('builtin:');
            return (
              <FileTreeItem
                key={hookFile.id}
                file={hookFile}
                displayName={hookFile.name}
                isActive={activeFileId === hookFile.id}
                badge={isBuiltin ? t('personaSelect.builtin') : hookMomentOf(hookFile.name)}
                icon={<Zap className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />}
                onSelect={() => onOpen(hookFile.id)}
              />
            );
          })}
        </div>
      )}
    </div>
  );
}

export default HooksFolderBlock;

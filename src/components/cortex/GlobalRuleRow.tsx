import { Globe, Trash2, RotateCcw } from 'lucide-react';
import type { CortexAgentFile } from '../../types';
import { useTranslation } from '../../stores/languageStore';
import { RuntimeRuleIcon, isRuntimeRule } from '../common/RuntimeRuleIcon';
import { isFileDirty } from '../../stores/cortex/cortexGitSave';
import type { CvhGitStatus } from '../../stores/cortex/types';

interface GlobalRuleRowProps {
  rule: CortexAgentFile;
  isActive: boolean;
  cvhGitStatus: CvhGitStatus | null;
  onOpen: (id: string) => void;
  onDelete: (e: React.MouseEvent, fileName: string) => void;
  onRestore: (key: string) => void;
}

export function GlobalRuleRow({
  rule,
  isActive,
  cvhGitStatus,
  onOpen,
  onDelete,
  onRestore,
}: GlobalRuleRowProps) {
  const { t } = useTranslation();

  if (rule.type === 'folder' || !rule.name) return null;
  const isRuntime = isRuntimeRule(rule.name);

  return (
    <div className="group/rule flex items-center justify-between px-2 py-1 rounded-md text-left transition hover:bg-[#ebebeb]/60 dark:hover:bg-[#232328]">
      <button
        type="button"
        onClick={() => onOpen(rule.id)}
        className={`flex-1 min-w-0 flex items-center space-x-1.5 text-left cursor-pointer ${
          isActive
            ? 'text-[#171717] dark:text-[#f4f4f5] font-semibold'
            : 'text-[#4d4d4d] dark:text-[#71717a]'
        }`}
        title={isRuntime ? t('cortex.runtimeRuleTooltip') : rule.path}
      >
        {isRuntime ? (
          <RuntimeRuleIcon className="h-3.5 w-3.5 text-sky-600 dark:text-sky-400 shrink-0" />
        ) : (
          <Globe className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />
        )}
        <span
          className={`text-xs font-mono truncate leading-none ${rule.isDeleted ? 'line-through text-[#a1a1aa]' : ''}`}
          title={rule.isDeleted ? t('cortex.deletedUnsaved') : undefined}
        >
          {rule.name}
        </span>
        {isRuntime && (
          <span
            title={t('cortex.runtimeRuleTooltip')}
            className="px-1 py-px rounded border border-sky-200 dark:border-sky-800/80 bg-sky-50 dark:bg-sky-950/50 text-[9px] font-mono font-medium text-sky-700 dark:text-sky-300 shrink-0 leading-tight"
          >
            {t('cortex.runtimeRuleBadge')}
          </span>
        )}
        {isFileDirty(rule, cvhGitStatus) && (
          <span
            className="h-1.5 w-1.5 rounded-full bg-amber-500 shrink-0"
            title={t('cortex.unsavedChanges')}
          />
        )}
      </button>

      {rule.isDeleted ? (
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onRestore(`file:${rule.id}`);
          }}
          aria-label={t('cortex.restoreDeleted')}
          title={t('cortex.restoreDeleted')}
          className="p-1 rounded text-[#8f8f8f] dark:text-[#71717a] hover:text-emerald-600 dark:hover:text-emerald-400 opacity-0 group-hover/rule:opacity-100 transition cursor-pointer shrink-0"
        >
          <RotateCcw className="h-3 w-3" />
        </button>
      ) : (
        <button
          type="button"
          onClick={(e) => onDelete(e, rule.name)}
          aria-label={t('common.delete')}
          className="p-1 rounded text-[#8f8f8f] dark:text-[#71717a] hover:text-red-600 dark:hover:text-red-400 opacity-0 group-hover/rule:opacity-100 transition cursor-pointer shrink-0"
        >
          <Trash2 className="h-3 w-3" />
        </button>
      )}
    </div>
  );
}

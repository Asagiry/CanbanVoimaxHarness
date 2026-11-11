import { Save, Loader2 } from 'lucide-react';
import { useCortexStore } from '../../stores/cortexStore';
import { useTranslation } from '../../stores/languageStore';
import { hasAnyCvhChanges } from '../../stores/cortex/cortexGitSave';

export default function CvhSaveButton() {
  const { t } = useTranslation();
  const cvhGitStatus = useCortexStore((s) => s.cvhGitStatus);
  const cvhGitSaving = useCortexStore((s) => s.cvhGitSaving);
  const cvhGitError = useCortexStore((s) => s.cvhGitError);
  const pendingDeletes = useCortexStore((s) => s.pendingDeletes);
  const pendingColumnChanges = useCortexStore((s) => s.pendingColumnChanges);
  const stages = useCortexStore((s) => s.stages);
  const agentStages = useCortexStore((s) => s.agentStages);
  const globalRules = useCortexStore((s) => s.globalRules);
  const activeFile = useCortexStore((s) => s.activeFile);
  const saveCvhToGit = useCortexStore((s) => s.saveCvhToGit);

  const hasChanges = hasAnyCvhChanges({ cvhGitStatus, pendingDeletes, pendingColumnChanges, stages, agentStages, globalRules, activeFile });
  const branch = cvhGitStatus?.branch;

  if (cvhGitSaving) {
    return (
      <button type="button" disabled aria-label={t('cortex.gitSaving')} title={t('cortex.gitSaving')} className="p-1 rounded hover:bg-[#e4e4e7] dark:hover:bg-[#232328] text-[#71717a] transition cursor-wait">
        <Loader2 className="h-3.5 w-3.5 animate-spin" />
      </button>
    );
  }

  const colorClass = cvhGitError
    ? 'text-red-500'
    : hasChanges
      ? 'text-amber-500'
      : 'text-emerald-500';
  const title = cvhGitError
    ? `${t('cortex.gitSaveError')}: ${cvhGitError}`
    : hasChanges
      ? `${t('cortex.saveToGit')}${branch ? ` (${branch} → main)` : ''} — ${t('cortex.uncommittedChangesHint')}`
      : t('cortex.allSaved');

  return (
    <button
      type="button"
      onClick={() => void saveCvhToGit().catch((err) => console.warn('[CvhSaveButton] saveCvhToGit failed:', err))}
      disabled={!hasChanges}
      aria-label={t('cortex.saveToGit')}
      title={title}
      className={`p-1 rounded hover:bg-[#e4e4e7] dark:hover:bg-[#232328] transition ${hasChanges ? 'cursor-pointer' : 'cursor-default opacity-80'} ${colorClass}`}
    >
      <Save className="h-3.5 w-3.5" />
    </button>
  );
}

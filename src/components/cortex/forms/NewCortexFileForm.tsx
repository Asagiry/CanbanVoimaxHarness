import { useState } from 'react';
import { useCortexStore, cortexAgentDir, previewCortexFileName } from '../../../stores/cortexStore';
import type { CortexHookExt, CortexHookMoment } from '../../../stores/cortexStore';
import type { CortexAgentStage } from '../../../types';
import { useTranslation } from '../../../stores/languageStore';

export interface NewCortexFileFormProps {
  stage: CortexAgentStage;
  onClose: () => void;
  initialKind?: 'rule' | 'hook';
  // Секция колонок создаёт только хуки: fixedKind прячет переключатель вида.
  fixedKind?: 'rule' | 'hook';
}

export function NewCortexFileForm({
  stage,
  onClose,
  initialKind = 'rule',
  fixedKind,
}: NewCortexFileFormProps) {
  const { t } = useTranslation();
  const createCortexFile = useCortexStore((s) => s.createCortexFile);
  const [rawName, setRawName] = useState('');
  const [kind, setKind] = useState<'rule' | 'hook'>(fixedKind ?? initialKind);
  const [hookMoment, setHookMoment] = useState<CortexHookMoment>('on_enter');
  const [hookExt, setHookExt] = useState<CortexHookExt>('bat');
  const [error, setError] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  const agentDir = cortexAgentDir(stage.personaName, stage.stageId);
  const targetDir = kind === 'rule' ? `${agentDir}/rules/` : `${agentDir}/hooks/`;
  const previewName =
    rawName.trim().length === 0
      ? kind === 'rule'
        ? `${t('cortex.ruleFileNamePlaceholder')}.md`
        : `${hookMoment}.${hookExt}`
      : previewCortexFileName(rawName, kind, { hookMoment, hookExt });

  const handleSubmit = async () => {
    try {
      if (kind === 'rule') {
        const trimmed = rawName.trim();
        const base = /\.md$/i.test(trimmed) ? trimmed.slice(0, -3).trim() : trimmed;
        if (!base) {
          setError(t('cortex.enterFileName'));
          return;
        }
        if (!/^[A-Za-z0-9-_]+$/.test(base)) {
          setError(t('cortex.latinDigitsOnly'));
          return;
        }
        const fileName = `${base}.md`;
        const duplicate = stage.files.some(
          (f) => f.name.toLowerCase() === fileName.toLowerCase()
        );
        if (duplicate) {
          setError(t('cortex.fileExists', { name: fileName }));
          return;
        }
      } else {
        const typed = rawName.trim().replace(/\.[A-Za-z0-9]+$/, '').trim();
        if (typed && !/^[A-Za-z0-9-_]+$/.test(typed)) {
          setError(t('cortex.latinDigitsOnly'));
          return;
        }
        const fileName = previewCortexFileName(rawName, 'hook', { hookMoment, hookExt });
        const duplicate = stage.files.some(
          (f) => f.name.toLowerCase() === fileName.toLowerCase()
        );
        if (duplicate) {
          setError(t('cortex.fileExists', { name: fileName }));
          return;
        }
      }
      setError(null);
      setIsSaving(true);
      await createCortexFile(
        stage.stageId,
        rawName,
        kind,
        kind === 'hook' ? { hookMoment, hookExt } : undefined,
      );
      onClose();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message || t('cortex.failedToCreateFile'));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="mx-1 mb-1 rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#161b22] px-2 py-2 space-y-2">
      <div className="text-[11px] font-sans font-semibold text-[#171717] dark:text-[#f0f6fc]">
        {t('cortex.newAgentFile')}
      </div>

      <input
        type="text"
        value={rawName}
        onChange={(e) => {
          setRawName(e.target.value);
          if (error) setError(null);
        }}
        onKeyDown={(e) => {
          if (e.key === 'Enter') void handleSubmit();
          if (e.key === 'Escape') onClose();
        }}
        placeholder={kind === 'rule' ? t('cortex.ruleFileNamePlaceholder') : t('cortex.hookFileNamePlaceholder')}
        autoFocus
        className="w-full px-2 py-1 text-xs rounded-md bg-white dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] font-mono placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] transition"
      />
      <div className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#8b949e]">
        {t('cortex.fileNamingHint')}
      </div>

      {!fixedKind && (
        <div className="flex items-center gap-3 text-[11px] font-mono text-[#4d4d4d] dark:text-[#c9d1d9]">
          <label className="flex items-center gap-1 cursor-pointer">
            <input
              type="radio"
              name={`cortex-kind-${stage.stageId}`}
              checked={kind === 'rule'}
              onChange={() => setKind('rule')}
              className="accent-[#171717] dark:accent-[#f0f6fc]"
            />
            <span>{t('cortex.rule')}</span>
          </label>
          <label className="flex items-center gap-1 cursor-pointer">
            <input
              type="radio"
              name={`cortex-kind-${stage.stageId}`}
              checked={kind === 'hook'}
              onChange={() => setKind('hook')}
              className="accent-[#171717] dark:accent-[#f0f6fc]"
            />
            <span>{t('cortex.hook')}</span>
          </label>
        </div>
      )}

      {kind === 'hook' && (
        <div className="space-y-1.5 rounded-md bg-[#f9f9f9] dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] px-2 py-1.5">
          <div className="flex items-center gap-3 text-[11px] font-mono text-[#4d4d4d] dark:text-[#c9d1d9]">
            <label className="flex items-center gap-1 cursor-pointer">
              <input
                type="radio"
                name={`cortex-moment-${stage.stageId}`}
                checked={hookMoment === 'on_enter'}
                onChange={() => setHookMoment('on_enter')}
                className="accent-[#171717] dark:accent-[#f0f6fc]"
              />
              <span>on_enter</span>
            </label>
            <label className="flex items-center gap-1 cursor-pointer">
              <input
                type="radio"
                name={`cortex-moment-${stage.stageId}`}
                checked={hookMoment === 'on_leave'}
                onChange={() => setHookMoment('on_leave')}
                className="accent-[#171717] dark:accent-[#f0f6fc]"
              />
              <span>on_leave</span>
            </label>
          </div>
          <label className="flex items-center gap-1.5 text-[11px] font-mono text-[#4d4d4d] dark:text-[#c9d1d9]">
            <span>{t('cortex.type')}</span>
            <select
              value={hookExt}
              onChange={(e) => setHookExt(e.target.value as CortexHookExt)}
              className="px-1.5 py-0.5 rounded-md bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] font-mono text-[11px] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] cursor-pointer"
            >
              <option value="bat">.bat</option>
              <option value="ps1">.ps1</option>
              <option value="py">.py</option>
              <option value="sh">.sh</option>
            </select>
          </label>
        </div>
      )}

      <div className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#8b949e] truncate" title={`${targetDir}${previewName}`}>
        {targetDir}{previewName}
      </div>

      {error && (
        <div className="text-[11px] font-sans text-red-600 dark:text-red-400">{error}</div>
      )}

      <div className="flex items-center gap-1.5">
        <button
          onClick={() => void handleSubmit()}
          disabled={isSaving}
          className="px-2.5 py-1 rounded-md text-[11px] font-mono bg-[#171717] dark:bg-[#238636] hover:bg-[#262626] dark:hover:bg-[#2ea043] text-white transition cursor-pointer disabled:opacity-50"
        >
          {isSaving ? t('common.saving') : t('common.create')}
        </button>
        <button
          onClick={onClose}
          disabled={isSaving}
          className="px-2.5 py-1 rounded-md text-[11px] font-mono bg-white dark:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#4d4d4d] dark:text-[#c9d1d9] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#30363d] transition cursor-pointer disabled:opacity-50"
        >
          {t('common.cancel')}
        </button>
      </div>
    </div>
  );
}

export default NewCortexFileForm;

import { useEffect, useState } from 'react';
import { ChevronDown, ChevronRight, Folder, FolderOpen, Plus, Trash2, X } from 'lucide-react';
import type { CortexAgentFile } from '../../types';
import type { ExpandAllSignal } from './types';
import { useTranslation } from '../../stores/languageStore';
import { useCortexStore } from '../../stores/cortexStore';
import DirtyDot from './DirtyDot';
import { GlobalRuleRow } from './GlobalRuleRow';
import { GlobalRuleCreateForm } from './GlobalRuleCreateForm';
import { isGlobalRulesDirty } from '../../stores/cortex/cortexGitSave';

export interface GlobalRulesSectionProps {
  rules: CortexAgentFile[];
  totalCount: number;
  isOpen: boolean;
  onToggle: () => void;
  isSearching: boolean;
  expandAllSignal?: ExpandAllSignal;
}

export function GlobalRulesSection({
  rules,
  totalCount,
  isOpen,
  onToggle,
  isSearching,
  expandAllSignal,
}: GlobalRulesSectionProps) {
  const { t } = useTranslation();
  const activeFile = useCortexStore((s) => s.activeFile);
  const openFile = useCortexStore((s) => s.openFile);
  const createGlobalRule = useCortexStore((s) => s.createGlobalRule);
  const deleteGlobalRule = useCortexStore((s) => s.deleteGlobalRule);
  const createGlobalFolder = useCortexStore((s) => s.createGlobalFolder);
  const deleteGlobalFolder = useCortexStore((s) => s.deleteGlobalFolder);
  const cvhGitStatus = useCortexStore((s) => s.cvhGitStatus);
  const allGlobalRules = useCortexStore((s) => s.globalRules);
  const restorePendingDelete = useCortexStore((s) => s.restorePendingDelete);
  const sectionDirty = isGlobalRulesDirty(cvhGitStatus, allGlobalRules);

  const [isCreating, setIsCreating] = useState(false);
  const [ruleName, setRuleName] = useState('');
  const [createError, setCreateError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [createFolder, setCreateFolder] = useState<string | undefined>(undefined);
  const [isCreatingFolder, setIsCreatingFolder] = useState(false);
  const [folderName, setFolderName] = useState('');
  const [openFolders, setOpenFolders] = useState<Record<string, boolean>>({});

  // Only reset folder open states when expandAllSignal explicitly fires (not on allGlobalRules updates)
  useEffect(() => {
    if (!expandAllSignal || expandAllSignal.nonce === 0) return;
    const currentRules = useCortexStore.getState().globalRules;
    const next: Record<string, boolean> = {};
    for (const r of currentRules) {
      if (r.folder) next[r.folder] = expandAllSignal.expanded;
    }
    setOpenFolders(next);
  }, [expandAllSignal]);

  const handleStartCreate = (e: React.MouseEvent, folder?: string) => {
    e.stopPropagation();
    if (!isOpen) onToggle();
    setIsCreatingFolder(false);
    setCreateFolder(folder);
    setIsCreating((prev) => (folder ? true : !prev));
    setRuleName('');
    setCreateError(null);
  };

  const handleStartCreateFolder = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!isOpen) onToggle();
    setIsCreating(false);
    setIsCreatingFolder((prev) => !prev);
    setFolderName('');
    setRuleName('');
    setCreateError(null);
  };

  const handleCreateSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (isCreatingFolder) {
      const folder = folderName.trim();
      if (!folder) {
        setCreateError(t('cortex.enterFolderName'));
        return;
      }
      if (!/^[A-Za-z0-9-_]+$/.test(folder)) {
        setCreateError(t('cortex.latinDigitsOnly'));
        return;
      }
      setIsSubmitting(true);
      setCreateError(null);
      try {
        await createGlobalFolder(folder);
        setIsCreatingFolder(false);
        setFolderName('');
        setOpenFolders((prev) => ({ ...prev, [folder]: true }));
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        setCreateError(msg);
      } finally {
        setIsSubmitting(false);
      }
      return;
    }

    const trimmed = ruleName.trim();
    if (!trimmed) {
      setCreateError(t('cortex.enterFileName'));
      return;
    }
    const cleanBase = trimmed.replace(/\.md$/, '');
    if (!/^[A-Za-z0-9-_]+$/.test(cleanBase)) {
      setCreateError(t('cortex.latinDigitsOnly'));
      return;
    }

    setIsSubmitting(true);
    setCreateError(null);
    try {
      await createGlobalRule(trimmed, createFolder);
      setIsCreating(false);
      setRuleName('');
      setCreateFolder(undefined);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setCreateError(msg);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleDeleteFolder = async (e: React.MouseEvent, folder: string) => {
    e.stopPropagation();
    if (!window.confirm(t('cortex.deleteFolderConfirm', { name: folder }))) {
      return;
    }
    setActionError(null);
    try {
      await deleteGlobalFolder(folder);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setActionError(t('cortex.deleteFolderError', { msg }));
    }
  };

  const handleDeleteRule = async (e: React.MouseEvent, fileName: string) => {
    e.stopPropagation();
    if (!window.confirm(t('cortex.deleteGlobalRuleConfirm', { name: fileName }))) {
      return;
    }
    setActionError(null);
    try {
      await deleteGlobalRule(fileName);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setActionError(t('cortex.deleteGlobalRuleError', { msg }));
    }
  };

  const rootRules = rules.filter((r) => !r.folder && r.type !== 'folder');
  const folders = [...new Set(rules.map((r) => r.folder).filter((f): f is string => !!f))].sort((a, b) =>
    a.localeCompare(b)
  );

  return (
    <section aria-label={t('cortex.globalRulesSection')} className="mt-2">
      {/* Section Header */}
      <div className="w-full flex items-center justify-between px-1.5 py-1 rounded-md text-[#171717] dark:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#1e1e23] transition group">
        <button
          type="button"
          onClick={onToggle}
          aria-expanded={isOpen}
          className="flex-1 flex items-center gap-1.5 py-1 text-left cursor-pointer"
        >
          {isOpen ? (
            <ChevronDown className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />
          )}
          <span className="font-sans text-xs font-semibold leading-none">{t('cortex.globalRulesSection')}</span>
          <span className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#71717a] tabular-nums leading-none">
            ({totalCount})
          </span>
          {sectionDirty && <DirtyDot />}
        </button>

        <div className="flex items-center gap-0.5 shrink-0">
          <button
            type="button"
            onClick={handleStartCreateFolder}
            aria-label={t('cortex.newFolder')}
            title={t('cortex.newFolder')}
            className="p-1 rounded-md text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#ebebeb]/60 dark:hover:bg-[#232328] transition cursor-pointer"
          >
            <Folder className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            onClick={(e) => handleStartCreate(e)}
            aria-label={t('cortex.newGlobalRule')}
            className="p-1 rounded-md text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#ebebeb]/60 dark:hover:bg-[#232328] transition cursor-pointer"
          >
            <Plus className="h-3.5 w-3.5" />
          </button>
        </div>
      </div>

      {actionError && (
        <div className="mx-1 mt-1 px-2 py-1 rounded border border-red-200 dark:border-red-900/60 bg-red-50 dark:bg-red-950/30 text-[10px] text-red-700 dark:text-red-400 flex items-center justify-between gap-1 font-sans">
          <span className="truncate">{actionError}</span>
          <button
            type="button"
            onClick={() => setActionError(null)}
            aria-label={t('common.close')}
            className="p-0.5 hover:opacity-75 cursor-pointer shrink-0"
          >
            <X className="h-3 w-3" />
          </button>
        </div>
      )}

      {isOpen && (
        <div className="space-y-1">
          {(isCreating || isCreatingFolder) && (
            <GlobalRuleCreateForm
              isCreatingFolder={isCreatingFolder}
              createFolder={createFolder}
              folderName={folderName}
              ruleName={ruleName}
              createError={createError}
              isSubmitting={isSubmitting}
              onFolderNameChange={setFolderName}
              onRuleNameChange={setRuleName}
              onSubmit={(e) => void handleCreateSubmit(e)}
              onCancel={() => {
                setIsCreating(false);
                setIsCreatingFolder(false);
                setCreateFolder(undefined);
              }}
            />
          )}

          {rules.length === 0 && !isSearching && !isCreating && !isCreatingFolder && (
            <div className="mx-1 mt-1 p-3 text-center rounded-md border border-dashed border-[#ebebeb] dark:border-[#2c2c33] text-[11px] text-[#8f8f8f] dark:text-[#71717a] font-sans">
              {t('cortex.noGlobalRules')}
            </div>
          )}

          {rootRules.map((rule) => (
            <GlobalRuleRow
              key={rule.id}
              rule={rule}
              isActive={activeFile?.id === rule.id}
              cvhGitStatus={cvhGitStatus}
              onOpen={(id) => void openFile(id)}
              onDelete={(e, name) => void handleDeleteRule(e, name)}
              onRestore={restorePendingDelete}
            />
          ))}

          {folders.map((folder) => {
            const folderRules = rules.filter((r) => r.folder === folder && r.type !== 'folder' && r.name !== '');
            const expanded = isSearching || openFolders[folder] !== false;
            return (
              <div key={folder}>
                <div className="flex items-center justify-between px-2 py-1 rounded-md hover:bg-[#ebebeb]/60 dark:hover:bg-[#232328] transition group/folder">
                  <button
                    type="button"
                    onClick={() => setOpenFolders((prev) => ({ ...prev, [folder]: !(prev[folder] !== false) }))}
                    aria-expanded={expanded}
                    className="flex-1 flex items-center gap-1.5 text-left cursor-pointer text-[#171717] dark:text-[#f4f4f5]"
                  >
                    {expanded ? (
                      <FolderOpen className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />
                    ) : (
                      <Folder className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />
                    )}
                    <span className="text-xs font-sans font-semibold leading-none">{folder}</span>
                    <span className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#71717a] tabular-nums leading-none">
                      ({folderRules.length})
                    </span>
                  </button>
                  <div className="flex items-center gap-0.5 shrink-0">
                    <button
                      type="button"
                      onClick={(e) => handleStartCreate(e, folder)}
                      aria-label={t('cortex.newRuleInFolder')}
                      title={t('cortex.newRuleInFolder')}
                      className="p-1 rounded-md text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#ebebeb]/60 dark:hover:bg-[#232328] transition cursor-pointer"
                    >
                      <Plus className="h-3 w-3" />
                    </button>
                    {folderRules.length === 0 && (
                      <button
                        type="button"
                        onClick={(e) => void handleDeleteFolder(e, folder)}
                        aria-label={t('common.delete')}
                        title={t('common.delete')}
                        className="p-1 rounded text-[#8f8f8f] dark:text-[#71717a] hover:text-red-600 dark:hover:text-red-400 opacity-0 group-hover/folder:opacity-100 transition cursor-pointer"
                      >
                        <Trash2 className="h-3 w-3" />
                      </button>
                    )}
                  </div>
                </div>
                {expanded && (
                  <div className="ml-3">
                    {folderRules.length === 0 ? (
                      <div className="px-2 py-1 text-[10px] text-[#8f8f8f] dark:text-[#71717a] font-sans italic">
                        {t('cortex.emptyFolder')}
                      </div>
                    ) : (
                      folderRules.map((rule) => (
                        <GlobalRuleRow
                          key={rule.id}
                          rule={rule}
                          isActive={activeFile?.id === rule.id}
                          cvhGitStatus={cvhGitStatus}
                          onOpen={(id) => void openFile(id)}
                          onDelete={(e, name) => void handleDeleteRule(e, name)}
                          onRestore={restorePendingDelete}
                        />
                      ))
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}

export default GlobalRulesSection;

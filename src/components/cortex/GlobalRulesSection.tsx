import { useState } from 'react';
import { ChevronDown, ChevronRight, Folder, FolderOpen, Globe, Plus, Trash2, RotateCcw } from 'lucide-react';
import type { CortexAgentFile } from '../../types';
import { useTranslation } from '../../stores/languageStore';
import { useCortexStore } from '../../stores/cortexStore';
import { RuntimeRuleIcon, isRuntimeRule } from '../common/RuntimeRuleIcon';
import DirtyDot from './DirtyDot';
import { isFileDirty, isGlobalRulesDirty } from '../../stores/cortex/cortexGitSave';

export interface GlobalRulesSectionProps {
  rules: CortexAgentFile[];
  totalCount: number;
  isOpen: boolean;
  onToggle: () => void;
  isSearching: boolean;
}

/**
 * Deep component representing the Project Global Rules (.cvh/rules/):
 * - Section header with count and toggle.
 * - Inline global rule creation form with file naming validation.
 * - List of project-level markdown rule files with active selection and delete action.
 */
export function GlobalRulesSection({
  rules,
  totalCount,
  isOpen,
  onToggle,
  isSearching,
}: GlobalRulesSectionProps) {
  const { t } = useTranslation();
  const activeFile = useCortexStore((s) => s.activeFile);
  const openFile = useCortexStore((s) => s.openFile);
  const createGlobalRule = useCortexStore((s) => s.createGlobalRule);
  const deleteGlobalRule = useCortexStore((s) => s.deleteGlobalRule);
  const cvhGitStatus = useCortexStore((s) => s.cvhGitStatus);
  const allGlobalRules = useCortexStore((s) => s.globalRules);
  const restorePendingDelete = useCortexStore((s) => s.restorePendingDelete);
  const sectionDirty = isGlobalRulesDirty(cvhGitStatus, allGlobalRules);

  const [isCreating, setIsCreating] = useState(false);
  const [ruleName, setRuleName] = useState('');
  const [createError, setCreateError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [createFolder, setCreateFolder] = useState<string | undefined>(undefined);
  const [isCreatingFolder, setIsCreatingFolder] = useState(false);
  const [folderName, setFolderName] = useState('');
  const [openFolders, setOpenFolders] = useState<Record<string, boolean>>({});

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
    const folder = isCreatingFolder ? folderName.trim() : createFolder;
    if (isCreatingFolder) {
      if (!folder) {
        setCreateError(t('cortex.enterFileName'));
        return;
      }
      if (!/^[A-Za-z0-9-_]+$/.test(folder)) {
        setCreateError(t('cortex.latinDigitsOnly'));
        return;
      }
    }

    setIsSubmitting(true);
    setCreateError(null);
    try {
      await createGlobalRule(trimmed, folder);
      setIsCreating(false);
      setIsCreatingFolder(false);
      setRuleName('');
      setFolderName('');
      setCreateFolder(undefined);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setCreateError(msg);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleDeleteRule = async (e: React.MouseEvent, fileName: string) => {
    e.stopPropagation();
    if (!window.confirm(t('cortex.deleteGlobalRuleConfirm', { name: fileName }))) {
      return;
    }
    try {
      await deleteGlobalRule(fileName);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      alert(t('cortex.deleteGlobalRuleError', { msg }));
    }
  };

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

      {isOpen && (
        <div className="space-y-1">
          {/* Inline Form for creating a new global rule */}
          {(isCreating || isCreatingFolder) && (
            <form
              onSubmit={handleCreateSubmit}
              className="mx-1 mt-1 p-2 rounded-md border border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] space-y-1.5 shadow-xs"
            >
              <div className="text-[10px] font-sans font-medium text-[#71717a] dark:text-[#71717a]">
                {isCreatingFolder ? t('cortex.newFolder') : createFolder ? t('cortex.newRuleInFolder') : t('cortex.newGlobalRule')}
              </div>
              {isCreatingFolder && (
                <input
                  type="text"
                  value={folderName}
                  onChange={(e) => setFolderName(e.target.value)}
                  placeholder={t('cortex.folderNamePlaceholder')}
                  autoFocus
                  className="w-full px-2 py-1 text-xs rounded-md bg-[#fafafa] dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] font-mono focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5]"
                />
              )}
              <input
                type="text"
                value={ruleName}
                onChange={(e) => setRuleName(e.target.value)}
                placeholder="architecture.md"
                autoFocus={!isCreatingFolder}
                className="w-full px-2 py-1 text-xs rounded-md bg-[#fafafa] dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] font-mono focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5]"
              />
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
                  onClick={() => { setIsCreating(false); setIsCreatingFolder(false); setCreateFolder(undefined); }}
                  className="px-2 py-0.5 text-[10px] font-mono rounded border border-[#ebebeb] dark:border-[#2c2c33] text-[#71717a] dark:text-[#71717a] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] cursor-pointer"
                >
                  {t('common.cancel')}
                </button>
              </div>
            </form>
          )}

          {/* List of Global Rules */}
          {rules.length === 0 && !isSearching && !isCreating && !isCreatingFolder && (
            <div className="mx-1 mt-1 p-3 text-center rounded-md border border-dashed border-[#ebebeb] dark:border-[#2c2c33] text-[11px] text-[#8f8f8f] dark:text-[#71717a] font-sans">
              {t('cortex.noGlobalRules')}
            </div>
          )}

          {(() => {
            const rootRules = rules.filter((r) => !r.folder);
            const folders = [...new Set(rules.map((r) => r.folder).filter((f): f is string => !!f))].sort((a, b) => a.localeCompare(b));
            const renderRule = (rule: CortexAgentFile) => {
            const isActive = activeFile?.id === rule.id;
            const isRuntime = isRuntimeRule(rule.name);
            return (
              <div
                key={rule.id}
                className="group/rule flex items-center justify-between px-2 py-1 rounded-md text-left transition hover:bg-[#ebebeb]/60 dark:hover:bg-[#232328] cursor-pointer"
                onClick={() => void openFile(rule.id)}
              >
                <button
                  type="button"
                  className={`flex-1 min-w-0 flex items-center space-x-1.5 text-left cursor-pointer ${
                    isActive
                      ? 'text-[#171717] dark:text-[#f4f4f5] font-semibold'
                      : 'text-[#4d4d4d] dark:text-[#71717a]'
                  }`}
                  title={isRuntime ? t('cortex.runtimeRuleTooltip') : rule.path}
                >
                  {isRuntime ? (
                    <RuntimeRuleIcon className="h-3.5 w-3.5 text-indigo-500 dark:text-indigo-400 shrink-0" />
                  ) : (
                    <Globe className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />
                  )}
                  <span className={`text-xs font-mono truncate leading-none ${rule.isDeleted ? 'line-through text-[#a1a1aa]' : ''}`} title={rule.isDeleted ? t('cortex.deletedUnsaved') : undefined}>{rule.name}</span>
                  {isRuntime && (
                    <span
                      title={t('cortex.runtimeRuleTooltip')}
                      className="px-1 py-px rounded border border-indigo-200 dark:border-indigo-800/80 bg-indigo-50 dark:bg-indigo-950/50 text-[9px] font-mono font-medium text-indigo-600 dark:text-indigo-400 shrink-0 leading-tight"
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
                    onClick={(e) => { e.stopPropagation(); restorePendingDelete(`file:${rule.id}`); }}
                    aria-label={t('cortex.restoreDeleted')}
                    title={t('cortex.restoreDeleted')}
                    className="p-1 rounded text-[#8f8f8f] dark:text-[#71717a] hover:text-emerald-600 dark:hover:text-emerald-400 opacity-0 group-hover/rule:opacity-100 transition cursor-pointer shrink-0"
                  >
                    <RotateCcw className="h-3 w-3" />
                  </button>
                ) : (
                  <button
                    type="button"
                    onClick={(e) => void handleDeleteRule(e, rule.name)}
                    aria-label={t('common.delete')}
                    className="p-1 rounded text-[#8f8f8f] dark:text-[#71717a] hover:text-red-600 dark:hover:text-red-400 opacity-0 group-hover/rule:opacity-100 transition cursor-pointer shrink-0"
                  >
                    <Trash2 className="h-3 w-3" />
                  </button>
                )}
              </div>
            );
            };
            return (
              <>
                {rootRules.map(renderRule)}
                {folders.map((folder) => {
                  const folderRules = rules.filter((r) => r.folder === folder);
                  const expanded = isSearching || openFolders[folder] !== false;
                  return (
                    <div key={folder}>
                      <div className="flex items-center justify-between px-2 py-1 rounded-md hover:bg-[#ebebeb]/60 dark:hover:bg-[#232328] transition">
                        <button
                          type="button"
                          onClick={() => setOpenFolders((prev) => ({ ...prev, [folder]: !(prev[folder] !== false) }))}
                          aria-expanded={expanded}
                          className="flex-1 flex items-center gap-1.5 text-left cursor-pointer text-[#171717] dark:text-[#f4f4f5]"
                        >
                          {expanded ? <FolderOpen className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" /> : <Folder className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />}
                          <span className="text-xs font-sans font-semibold leading-none">{folder}</span>
                          <span className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#71717a] tabular-nums leading-none">({folderRules.length})</span>
                        </button>
                        <button
                          type="button"
                          onClick={(e) => handleStartCreate(e, folder)}
                          aria-label={t('cortex.newRuleInFolder')}
                          title={t('cortex.newRuleInFolder')}
                          className="p-1 rounded-md text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#ebebeb]/60 dark:hover:bg-[#232328] transition cursor-pointer shrink-0"
                        >
                          <Plus className="h-3 w-3" />
                        </button>
                      </div>
                      {expanded && <div className="ml-3">{folderRules.map(renderRule)}</div>}
                    </div>
                  );
                })}
              </>
            );
          })()}
        </div>
      )}
    </section>
  );
}

export default GlobalRulesSection;

import { useState } from 'react';
import { ChevronDown, ChevronRight, Globe, Plus, Trash2 } from 'lucide-react';
import type { CortexAgentFile } from '../../types';
import { useTranslation } from '../../stores/languageStore';
import { useCortexStore } from '../../stores/cortexStore';
import { RuntimeRuleIcon, isRuntimeRule } from '../common/RuntimeRuleIcon';

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

  const [isCreating, setIsCreating] = useState(false);
  const [ruleName, setRuleName] = useState('');
  const [createError, setCreateError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleStartCreate = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!isOpen) onToggle();
    setIsCreating((prev) => !prev);
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

    setIsSubmitting(true);
    setCreateError(null);
    try {
      await createGlobalRule(trimmed);
      setIsCreating(false);
      setRuleName('');
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
      <div className="w-full flex items-center justify-between px-1.5 py-1 rounded-md text-[#171717] dark:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] transition group">
        <button
          type="button"
          onClick={onToggle}
          aria-expanded={isOpen}
          className="flex-1 flex items-center gap-1.5 py-1 text-left cursor-pointer"
        >
          {isOpen ? (
            <ChevronDown className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
          )}
          <span className="font-sans text-xs font-semibold">{t('cortex.globalRulesSection')}</span>
          <span className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#8b949e] tabular-nums">
            ({totalCount})
          </span>
        </button>

        <button
          type="button"
          onClick={handleStartCreate}
          aria-label={t('cortex.newGlobalRule')}
          className="p-1 rounded-md text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#ebebeb]/60 dark:hover:bg-[#21262d] transition cursor-pointer shrink-0"
        >
          <Plus className="h-3.5 w-3.5" />
        </button>
      </div>

      {isOpen && (
        <div className="space-y-1">
          {/* Inline Form for creating a new global rule */}
          {isCreating && (
            <form
              onSubmit={handleCreateSubmit}
              className="mx-1 mt-1 p-2 rounded-md border border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#161b22] space-y-1.5 shadow-xs"
            >
              <div className="text-[10px] font-sans font-medium text-[#71717a] dark:text-[#8b949e]">
                {t('cortex.newGlobalRule')}
              </div>
              <input
                type="text"
                value={ruleName}
                onChange={(e) => setRuleName(e.target.value)}
                placeholder="architecture.md"
                autoFocus
                className="w-full px-2 py-1 text-xs rounded-md bg-[#fafafa] dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] font-mono focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc]"
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
                  onClick={() => setIsCreating(false)}
                  className="px-2 py-0.5 text-[10px] font-mono rounded border border-[#ebebeb] dark:border-[#30363d] text-[#71717a] dark:text-[#8b949e] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] cursor-pointer"
                >
                  {t('common.cancel')}
                </button>
              </div>
            </form>
          )}

          {/* List of Global Rules */}
          {rules.length === 0 && !isSearching && !isCreating && (
            <div className="mx-1 mt-1 p-3 text-center rounded-md border border-dashed border-[#ebebeb] dark:border-[#30363d] text-[11px] text-[#8f8f8f] dark:text-[#8b949e] font-sans">
              {t('cortex.noGlobalRules')}
            </div>
          )}

          {rules.map((rule) => {
            const isActive = activeFile?.id === rule.id;
            const isRuntime = isRuntimeRule(rule.name);
            return (
              <div
                key={rule.id}
                className="group/rule flex items-center justify-between px-2 py-1 rounded-md text-left transition hover:bg-[#ebebeb]/60 dark:hover:bg-[#21262d] cursor-pointer"
                onClick={() => void openFile(rule.id)}
              >
                <button
                  type="button"
                  className={`flex-1 min-w-0 flex items-center space-x-1.5 text-left cursor-pointer ${
                    isActive
                      ? 'text-[#171717] dark:text-[#f0f6fc] font-semibold'
                      : 'text-[#4d4d4d] dark:text-[#8b949e]'
                  }`}
                  title={isRuntime ? t('cortex.runtimeRuleTooltip') : rule.path}
                >
                  {isRuntime ? (
                    <RuntimeRuleIcon className="h-3.5 w-3.5 text-indigo-500 dark:text-indigo-400 shrink-0" />
                  ) : (
                    <Globe className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
                  )}
                  <span className="text-xs font-mono truncate">{rule.name}</span>
                  {isRuntime && (
                    <span
                      title={t('cortex.runtimeRuleTooltip')}
                      className="px-1 py-px rounded border border-indigo-200 dark:border-indigo-800/80 bg-indigo-50 dark:bg-indigo-950/50 text-[9px] font-mono font-medium text-indigo-600 dark:text-indigo-400 shrink-0 leading-tight"
                    >
                      {t('cortex.runtimeRuleBadge')}
                    </span>
                  )}
                  {rule.isDirty && (
                    <span
                      className="h-1.5 w-1.5 rounded-full bg-amber-500 shrink-0"
                      title={t('cortex.unsavedChanges')}
                    />
                  )}
                </button>

                <button
                  type="button"
                  onClick={(e) => void handleDeleteRule(e, rule.name)}
                  aria-label={t('common.delete')}
                  className="p-1 rounded text-[#8f8f8f] dark:text-[#8b949e] hover:text-red-600 dark:hover:text-red-400 opacity-0 group-hover/rule:opacity-100 transition cursor-pointer shrink-0"
                >
                  <Trash2 className="h-3 w-3" />
                </button>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}

export default GlobalRulesSection;

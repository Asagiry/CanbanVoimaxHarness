import { useState } from 'react';
import { Globe, Plus, Trash2 } from 'lucide-react';
import type { CortexAgentFile } from '../../../types';
import { useTranslation } from '../../../stores/languageStore';
import SectionHeader from '../tree/SectionHeader';
import FileTreeItem from '../tree/FileTreeItem';

export interface GlobalRulesSectionProps {
  rules: CortexAgentFile[];
  totalCount: number;
  isOpen: boolean;
  onToggle: () => void;
  activeFileId: string | null;
  onOpenFile: (fileId: string) => void;
  onCreateRule: (rawName: string) => Promise<void>;
  onDeleteRule: (fileName: string) => Promise<void>;
  isSearching: boolean;
}

export function GlobalRulesSection({
  rules,
  totalCount,
  isOpen,
  onToggle,
  activeFileId,
  onOpenFile,
  onCreateRule,
  onDeleteRule,
  isSearching,
}: GlobalRulesSectionProps) {
  const { t } = useTranslation();
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
      await onCreateRule(trimmed);
      setIsCreating(false);
      setRuleName('');
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setCreateError(msg);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <section aria-label={t('cortex.globalRulesSection')} className="mt-2">
      <SectionHeader
        title={t('cortex.globalRulesSection')}
        count={totalCount}
        open={isOpen}
        onToggle={onToggle}
        action={
          <button
            type="button"
            onClick={handleStartCreate}
            aria-label={t('cortex.newGlobalRule')}
            className="p-1 rounded-md text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#ebebeb]/60 dark:hover:bg-[#21262d] transition cursor-pointer"
          >
            <Plus className="h-3.5 w-3.5" />
          </button>
        }
      />

      {isOpen && (
        <div className="space-y-1">
          {/* Инлайн-форма создания глобального правила */}
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
                autoFocus
                value={ruleName}
                onChange={(e) => {
                  setRuleName(e.target.value);
                  if (createError) setCreateError(null);
                }}
                onKeyDown={(e) => {
                  if (e.key === 'Escape') {
                    setIsCreating(false);
                    setRuleName('');
                    setCreateError(null);
                  }
                }}
                placeholder={t('cortex.globalRulePlaceholder')}
                className="w-full px-2 py-1 text-xs rounded border border-[#ebebeb] dark:border-[#30363d] bg-[#f9f9f9] dark:bg-[#0d1117] text-[#171717] dark:text-[#f0f6fc] font-mono focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] transition"
              />
              {createError && (
                <div className="text-[10px] text-red-500 font-sans leading-tight">
                  {createError}
                </div>
              )}
              <div className="flex items-center justify-end space-x-1.5 pt-0.5">
                <button
                  type="button"
                  onClick={() => {
                    setIsCreating(false);
                    setRuleName('');
                    setCreateError(null);
                  }}
                  className="px-2 py-0.5 text-[11px] font-sans rounded text-[#71717a] dark:text-[#8b949e] hover:bg-[#ebebeb]/60 dark:hover:bg-[#21262d] cursor-pointer"
                >
                  {t('common.cancel')}
                </button>
                <button
                  type="submit"
                  disabled={isSubmitting || !ruleName.trim()}
                  className="px-2 py-0.5 text-[11px] font-sans font-semibold rounded bg-[#171717] dark:bg-[#f0f6fc] text-white dark:text-[#0d1117] disabled:opacity-40 cursor-pointer transition"
                >
                  {t('cortex.createGlobalRuleBtn')}
                </button>
              </div>
            </form>
          )}

          {/* Пустые состояния */}
          {totalCount === 0 && !isCreating && (
            <div className="mx-1 mt-1 rounded-md border border-dashed border-[#d4d4d4] dark:border-[#30363d] bg-white dark:bg-[#161b22] px-3 py-4 text-center space-y-1.5">
              <div className="text-xs font-sans font-semibold text-[#171717] dark:text-[#f0f6fc]">
                {t('cortex.noGlobalRules')}
              </div>
              <div className="text-[11px] font-sans text-[#666666] dark:text-[#8b949e] leading-relaxed">
                {t('cortex.createFirstGlobalRule')}
              </div>
            </div>
          )}

          {totalCount > 0 && rules.length === 0 && isSearching && (
            <div className="mx-1 mt-1 rounded-md border border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#161b22] px-3 py-4 text-center">
              <div className="text-[11px] font-sans text-[#666666] dark:text-[#8b949e]">
                {t('cortex.globalRulesNothingFound')}
              </div>
            </div>
          )}

          {/* Список глобальных правил */}
          {rules.map((rule) => (
            <div
              key={rule.id}
              className="group/rule flex items-center justify-between gap-1 pr-1 hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] rounded-md transition"
            >
              <div className="flex-1 min-w-0">
                <FileTreeItem
                  file={rule}
                  displayName={rule.name}
                  isActive={activeFileId === rule.id}
                  icon={<Globe className="h-3.5 w-3.5 text-blue-500 shrink-0" />}
                  onSelect={() => onOpenFile(rule.id)}
                />
              </div>
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  void onDeleteRule(rule.name);
                }}
                title={t('cortex.deleteAgent')}
                aria-label={`Delete ${rule.name}`}
                className="p-1 rounded text-[#8f8f8f] dark:text-[#8b949e] hover:text-red-600 dark:hover:text-red-400 hover:bg-[#ebebeb]/60 dark:hover:bg-[#21262d] transition opacity-0 group-hover/rule:opacity-100 focus:opacity-100 cursor-pointer shrink-0"
              >
                <Trash2 className="h-3 w-3" />
              </button>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

export default GlobalRulesSection;

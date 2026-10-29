import { useState, useEffect, useRef } from 'react';
import { Bot, X, Folder, FileText, RotateCw, Plus } from 'lucide-react';
import { useTranslation } from '../../../stores/languageStore';
import { useCortexStore } from '../../../stores/cortexStore';

export interface CreateAgentModalProps {
  isOpen: boolean;
  existingRoles: string[];
  onClose: () => void;
  onCreateSuccess: (role: string) => Promise<void> | void;
}

export function CreateAgentModal({
  isOpen,
  existingRoles,
  onClose,
  onCreateSuccess,
}: CreateAgentModalProps) {
  const { t } = useTranslation();
  const [roleName, setRoleName] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const syncAgentPersonas = useCortexStore((s) => s.syncAgentPersonas);

  useEffect(() => {
    if (isOpen) {
      setRoleName('');
      setError(null);
      setIsSubmitting(false);
      const timer = setTimeout(() => {
        inputRef.current?.focus();
      }, 50);
      return () => clearTimeout(timer);
    }
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !isSubmitting) {
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, isSubmitting, onClose]);

  if (!isOpen) return null;

  const handleSubmit = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    if (isSubmitting) return;

    const trimmed = roleName.trim().toLowerCase();
    if (!trimmed) {
      setError(t('cortex.errEmptyRole'));
      inputRef.current?.focus();
      return;
    }

    if (!/^[a-z0-9][a-z0-9-]*$/.test(trimmed)) {
      setError(t('cortex.errRolePattern'));
      inputRef.current?.focus();
      return;
    }

    const isDuplicate = existingRoles.some(
      (r) => r.trim().toLowerCase() === trimmed,
    );
    if (isDuplicate) {
      setError(t('cortex.roleExists', { role: trimmed }));
      inputRef.current?.focus();
      return;
    }

    setIsSubmitting(true);
    setError(null);

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('create_agent_persona', { role: trimmed });
      await syncAgentPersonas();
      await onCreateSuccess(trimmed);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      setIsSubmitting(false);
    }
  };

  const previewRole = roleName.trim().toLowerCase() || 'architect';

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 dark:bg-black/80 p-4 animate-in fade-in duration-150"
      onClick={isSubmitting ? undefined : onClose}
    >
      <div
        className="w-full max-w-md rounded-xl border border-[#e4e4e7] dark:border-[#21262d] bg-white dark:bg-[#0d1117] text-[#171717] dark:text-[#f0f6fc] p-5 shadow-2xl space-y-4 animate-in zoom-in-95 duration-150"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-[#f4f4f5] dark:border-[#21262d] pb-3">
          <div className="flex items-center space-x-2.5">
            <div className="h-7 w-7 rounded-lg bg-[#f4f4f5] dark:bg-[#161b22] border border-[#e4e4e7] dark:border-[#30363d] flex items-center justify-center text-[#171717] dark:text-[#f0f6fc]">
              <Bot className="h-4 w-4" />
            </div>
            <div>
              <h3 className="font-semibold text-sm text-[#171717] dark:text-[#f0f6fc]">
                {t('cortex.newAgentModalTitle')}
              </h3>
              <p className="text-[11px] text-[#71717a] dark:text-[#8b949e]">
                {t('cortex.newAgentSubtitle')}
              </p>
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            disabled={isSubmitting}
            aria-label={t('common.close')}
            className="p-1 rounded-md text-[#71717a] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] transition cursor-pointer disabled:opacity-50"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-xs font-medium text-[#52525b] dark:text-[#8b949e] mb-1">
              {t('cortex.roleName')}
            </label>
            <input
              ref={inputRef}
              type="text"
              value={roleName}
              disabled={isSubmitting}
              onChange={(e) => {
                setRoleName(e.target.value.toLowerCase());
                if (error) setError(null);
              }}
              placeholder="architect, tester, security, db-lead"
              className="w-full px-3 py-2 rounded-lg border border-[#d4d4d8] dark:border-[#30363d] bg-white dark:bg-[#161b22] text-[#171717] dark:text-[#f0f6fc] text-xs font-mono placeholder-[#a1a1aa] dark:placeholder-[#6e7681] focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:outline-hidden transition disabled:opacity-50"
            />
            <p className="mt-1 text-[11px] text-[#71717a] dark:text-[#8b949e]">
              {t('cortex.roleNameRulesHint')}
            </p>
          </div>

          {/* Превью файловой структуры */}
          <div className="rounded-lg border border-[#ebebeb] dark:border-[#30363d] bg-[#fafafa] dark:bg-[#161b22] p-2.5 text-[11px] font-mono text-[#52525b] dark:text-[#8b949e] space-y-1">
            <div className="text-[10px] uppercase font-sans font-semibold text-[#a1a1aa] dark:text-[#8b949e] tracking-wider mb-1">
              {t('cortex.willBeCreated')}
            </div>
            <div className="flex items-center gap-1.5 truncate">
              <Folder className="h-3.5 w-3.5 text-[#71717a] dark:text-[#8b949e] shrink-0" />
              <span className="truncate">.cvh/agents/{previewRole}/</span>
            </div>
            <div className="flex items-center gap-1.5 pl-4 truncate text-[#71717a] dark:text-[#8b949e]">
              <FileText className="h-3 w-3 shrink-0 text-[#6366f1]" />
              <span className="truncate">{t('cortex.personaTemplatePreview')}</span>
            </div>
            <div className="flex items-center gap-1.5 pl-4 truncate text-[#71717a] dark:text-[#8b949e]">
              <Folder className="h-3 w-3 shrink-0" />
              <span className="truncate">{t('cortex.rulesDirPreview')}</span>
            </div>
          </div>

          {error && (
            <div className="text-xs text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/30 p-2.5 rounded-lg border border-red-200 dark:border-red-900/50">
              {error}
            </div>
          )}

          <div className="flex items-center justify-end space-x-2 pt-2 border-t border-[#f4f4f5] dark:border-[#21262d]">
            <button
              type="button"
              onClick={onClose}
              disabled={isSubmitting}
              className="px-3.5 py-1.5 rounded-md text-xs font-medium text-[#71717a] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] transition cursor-pointer disabled:opacity-50"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={isSubmitting || !roleName.trim()}
              className="px-4 py-1.5 rounded-md text-xs font-medium bg-[#171717] dark:bg-[#238636] hover:bg-[#262626] dark:hover:bg-[#2ea043] text-white transition cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed flex items-center space-x-1.5"
            >
              {isSubmitting ? (
                <>
                  <RotateCw className="h-3.5 w-3.5 animate-spin" />
                  <span>{t('common.saving')}</span>
                </>
              ) : (
                <>
                  <Plus className="h-3.5 w-3.5" />
                  <span>{t('cortex.createAgentBtn')}</span>
                </>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

export default CreateAgentModal;

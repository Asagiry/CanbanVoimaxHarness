import { useState, useEffect } from 'react';
import { X, Layers, ArrowRight } from 'lucide-react';
import { useBoardStore } from '../../stores/boardStore';
import { useCortexStore } from '../../stores/cortexStore';
import { useTranslation } from '../../stores/languageStore';
import PersonaSelect from '../common/PersonaSelect';

interface CreateBoardModalProps {
  isOpen: boolean;
  onClose: () => void;
}

interface PersonaOption {
  role: string;
  isBuiltin?: boolean;
  icon?: string;
}

export default function CreateBoardModal({ isOpen, onClose }: CreateBoardModalProps) {
  const { createBoard, saveBoard } = useBoardStore();
  const { t } = useTranslation();
  const [boardName, setBoardName] = useState('');
  const [selectedPersona, setSelectedPersona] = useState('developer');
  const [personas, setPersonas] = useState<PersonaOption[]>([]);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      setBoardName('');
      setSelectedPersona('developer');
      setError(null);
      setIsSubmitting(false);

      const fetchPersonas = async () => {
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          const list = await invoke<Array<{ role: string; isBuiltin?: boolean; is_builtin?: boolean; icon?: string }>>('list_agent_personas');
          if (Array.isArray(list)) {
            const mapped = list.map((p) => ({
              role: p.role,
              isBuiltin: p.isBuiltin ?? p.is_builtin ?? (p.role === 'cvh-dev'),
              icon: p.icon,
            }));
            setPersonas(mapped);
          }
        } catch {
          setPersonas([
            { role: 'cvh-dev', isBuiltin: true },
            { role: 'developer', isBuiltin: false },
          ]);
        }
      };
      void fetchPersonas();
    }
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = boardName.trim();
    if (!trimmed) {
      setError(t('createBoard.nameRequired'));
      return;
    }

    setIsSubmitting(true);
    setError(null);
    try {
      const created = await createBoard(trimmed);
      if (created) {
        if (selectedPersona) {
          const matched = personas.find((p) => p.role === selectedPersona);
          const updatedColumns = created.columns.map((c) =>
            c.id === 'dev'
              ? { ...c, persona: selectedPersona, icon: matched?.icon || c.icon }
              : c,
          );
          await saveBoard({ ...created, columns: updatedColumns });
          useCortexStore.getState().syncStagesFromColumns(updatedColumns);
        }
        setBoardName('');
        onClose();
      } else {
        setError(t('createBoard.failed'));
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 dark:bg-black/80 p-4 animate-in fade-in duration-150"
      onClick={onClose}
    >
      <div
        className="w-full max-w-md bg-white dark:bg-[#0d1117] rounded-lg shadow-md border border-[#ebebeb] dark:border-[#21262d] overflow-hidden animate-in zoom-in-95 duration-150 flex flex-col transition-colors"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-[#ebebeb] dark:border-[#21262d] bg-[#fafafa] dark:bg-[#161b22]">
          <div className="flex items-center space-x-2.5">
            <div className="p-1.5 rounded-lg bg-white dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] shadow-xs text-[#171717] dark:text-[#f0f6fc]">
              <Layers className="h-4 w-4 text-[#171717] dark:text-[#f0f6fc]" />
            </div>
            <div>
              <h2 className="text-sm font-semibold text-[#171717] dark:text-[#f0f6fc] font-sans">
                {t('createBoard.title')}
              </h2>
              <p className="text-[11px] text-[#71717a] dark:text-[#8b949e] font-sans">
                {t('createBoard.subtitle')}
              </p>
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="p-1 rounded-md text-[#71717a] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] transition cursor-pointer"
            aria-label={t('common.close')}
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Form Body */}
        <form onSubmit={handleSubmit} className="p-5 space-y-4 text-xs font-sans">
          {error && (
            <div className="p-2.5 rounded-md bg-[#fef2f2] dark:bg-[#161b22] border border-[#fecaca] dark:border-[#3b1219] text-[#ef4444] text-xs">
              {error}
            </div>
          )}

          {/* Board Name Input */}
          <div>
            <label className="block text-xs font-medium text-[#171717] dark:text-[#f0f6fc] mb-1.5">
              {t('createBoard.nameLabel')} <span className="text-red-500">*</span>
            </label>
            <input
              type="text"
              required
              autoFocus
              value={boardName}
              onChange={(e) => setBoardName(e.target.value)}
              placeholder={t('createBoard.namePlaceholder')}
              className="w-full px-3 py-2 rounded-lg bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#a1a1aa] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#0d1117] text-xs transition"
            />
          </div>

          {/* Persona selector for development column */}
          <div>
            <label className="block text-xs font-medium text-[#171717] dark:text-[#f0f6fc] mb-1.5">
              {t('createBoard.devAgentLabel')}
            </label>
            <PersonaSelect
              value={selectedPersona}
              onChange={(val) => setSelectedPersona(val)}
              personas={personas}
            />
          </div>

          {/* Invariant Visual Preview */}
          <div className="p-3 rounded-lg bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] space-y-2">
            <span className="block text-[11px] font-medium text-[#71717a] dark:text-[#8b949e]">
              {t('createBoard.stageInvariant')}
            </span>
            <div className="flex items-center justify-between gap-1 text-[11px]">
              {/* Backlog */}
              <div className="flex-1 flex flex-col items-center justify-center py-1.5 px-1 rounded-md bg-white dark:bg-[#0d1117] border border-[#64748b]/30 shadow-2xs text-center">
                <span className="w-1.5 h-1.5 rounded-full bg-[#64748b] mb-1" />
                <span className="font-medium text-[#171717] dark:text-[#f0f6fc] text-[11px]">{t('createBoard.backlog')}</span>
                <span className="text-[9px] text-[#71717a] dark:text-[#8b949e]">{t('createBoard.intake')}</span>
              </div>

              <ArrowRight className="h-3 w-3 text-[#a1a1aa] dark:text-[#6e7681] shrink-0" />

              {/* Dev */}
              <div className="flex-1 flex flex-col items-center justify-center py-1.5 px-1 rounded-md bg-white dark:bg-[#0d1117] border border-[#f59e0b]/30 shadow-2xs text-center">
                <span className="w-1.5 h-1.5 rounded-full bg-[#f59e0b] mb-1" />
                <span className="font-medium text-[#171717] dark:text-[#f0f6fc] text-[11px]">{t('createBoard.inDev')}</span>
                <span className="text-[9px] text-[#f59e0b] font-mono">@{selectedPersona}</span>
              </div>

              <ArrowRight className="h-3 w-3 text-[#a1a1aa] dark:text-[#6e7681] shrink-0" />

              {/* Ready */}
              <div className="flex-1 flex flex-col items-center justify-center py-1.5 px-1 rounded-md bg-white dark:bg-[#0d1117] border border-[#10b981]/30 shadow-2xs text-center">
                <span className="w-1.5 h-1.5 rounded-full bg-[#10b981] mb-1" />
                <span className="font-medium text-[#171717] dark:text-[#f0f6fc] text-[11px]">{t('createBoard.ready')}</span>
                <span className="text-[9px] text-[#10b981] font-semibold">{t('createBoard.terminal')}</span>
              </div>
            </div>
            <p className="text-[10px] text-[#71717a] dark:text-[#8b949e] text-center pt-0.5">
              {t('createBoard.invariantNotice')}
            </p>
          </div>

          {/* Footer Actions */}
          <div className="flex items-center justify-end space-x-2 pt-2 border-t border-[#ebebeb] dark:border-[#21262d]">
            <button
              type="button"
              onClick={onClose}
              disabled={isSubmitting}
              className="px-3.5 py-1.5 rounded-md border border-[#ebebeb] dark:border-[#30363d] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] text-[#71717a] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] font-medium transition cursor-pointer text-xs"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={isSubmitting || !boardName.trim()}
              className="flex items-center space-x-1.5 px-4 py-1.5 rounded-md bg-[#171717] dark:bg-[#21262d] hover:bg-[#262626] dark:hover:bg-[#30363d] text-white font-medium transition cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed shadow-xs text-xs border border-transparent dark:border-[#30363d]"
            >
              <span>{isSubmitting ? t('createBoard.creating') : t('common.create')}</span>
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

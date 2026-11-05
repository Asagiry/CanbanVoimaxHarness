import { useState, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { X, Layers, ArrowRight } from 'lucide-react';
import { useBoardStore } from '../../stores/boardStore';
import { useCortexStore } from '../../stores/cortexStore';
import { useTranslation } from '../../stores/languageStore';
import { useUiZoom, getZoomOverlayStyle } from '../../hooks/useUiZoom';
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
  const { zoom } = useUiZoom();
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
          const curStages = created.stages ?? created.columns ?? [];
          const targetCol = curStages.find((c) => !c.isIntake && !c.terminal && c.id !== 'backlog' && c.id !== 'ready') ?? curStages[1];
          const updatedColumns = curStages.map((c) =>
            c.id === targetCol?.id
              ? { ...c, persona: selectedPersona, icon: matched?.icon || c.icon }
              : c,
          );
          await saveBoard({ ...created, stages: updatedColumns, columns: updatedColumns });
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

  const modalContent = (
    <div
      style={getZoomOverlayStyle(zoom)}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 dark:bg-black/80 p-4 animate-in fade-in duration-150"
      onClick={onClose}
    >
      <div
        className="w-full max-w-md max-h-[90%] overflow-y-auto bg-white dark:bg-[#111114] rounded-lg shadow-md border border-[#ebebeb] dark:border-[#212126] overflow-hidden animate-in zoom-in-95 duration-150 flex flex-col transition-colors"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-[#ebebeb] dark:border-[#212126] bg-[#fafafa] dark:bg-[#17171b]">
          <div className="flex items-center space-x-2.5">
            <Layers className="h-4 w-4 text-[#171717] dark:text-[#f4f4f5] shrink-0" />
            <div>
              <h2 className="text-sm font-semibold text-[#171717] dark:text-[#f4f4f5] font-sans">
                {t('createBoard.title')}
              </h2>
              <p className="text-[11px] text-[#71717a] dark:text-[#71717a] font-sans">
                {t('createBoard.subtitle')}
              </p>
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="p-1 rounded-md text-[#71717a] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] transition cursor-pointer"
            aria-label={t('common.close')}
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Form Body */}
        <form onSubmit={handleSubmit} className="p-5 space-y-4 text-xs font-sans">
          {error && (
            <div className="p-2.5 rounded-md bg-[#fef2f2] dark:bg-[#17171b] border border-[#fecaca] dark:border-[#3b1219] text-[#ef4444] text-xs">
              {error}
            </div>
          )}

          {/* Board Name Input */}
          <div>
            <label className="block text-xs font-medium text-[#171717] dark:text-[#f4f4f5] mb-1.5">
              {t('createBoard.nameLabel')} <span className="text-red-500">*</span>
            </label>
            <input
              type="text"
              required
              autoFocus
              value={boardName}
              onChange={(e) => setBoardName(e.target.value)}
              placeholder={t('createBoard.namePlaceholder')}
              className="w-full px-3 py-2 rounded-lg bg-[#fafafa] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#a1a1aa] dark:placeholder:text-[#71717a] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] focus:bg-white dark:focus:bg-[#111114] text-xs transition"
            />
          </div>

          {/* Persona selector for development column */}
          <div>
            <label className="block text-xs font-medium text-[#171717] dark:text-[#f4f4f5] mb-1.5">
              {t('createBoard.devAgentLabel')}
            </label>
            <PersonaSelect
              value={selectedPersona}
              onChange={(val) => setSelectedPersona(val)}
              personas={personas}
            />
          </div>

          {/* Invariant Visual Preview */}
          <div className="p-3 rounded-lg bg-[#fafafa] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] space-y-2">
            <span className="block text-[11px] font-medium text-[#71717a] dark:text-[#71717a]">
              {t('createBoard.stageInvariant')}
            </span>
            <div className="flex items-center justify-between gap-1 text-[11px]">
              {/* Backlog */}
              <div className="flex-1 flex flex-col items-center justify-center py-1.5 px-1 rounded-md bg-white dark:bg-[#111114] border border-[#64748b]/30 shadow-2xs text-center">
                <span className="w-1.5 h-1.5 rounded-full bg-[#64748b] mb-1" />
                <span className="font-medium text-[#171717] dark:text-[#f4f4f5] text-[11px]">{t('createBoard.backlog')}</span>
                <span className="text-[9px] text-[#71717a] dark:text-[#71717a]">{t('createBoard.intake')}</span>
              </div>

              <ArrowRight className="h-3 w-3 text-[#a1a1aa] dark:text-[#52525b] shrink-0" />

              {/* Dev */}
              <div className="flex-1 flex flex-col items-center justify-center py-1.5 px-1 rounded-md bg-white dark:bg-[#111114] border border-[#f59e0b]/30 shadow-2xs text-center">
                <span className="w-1.5 h-1.5 rounded-full bg-[#f59e0b] mb-1" />
                <span className="font-medium text-[#171717] dark:text-[#f4f4f5] text-[11px]">{t('createBoard.inDev')}</span>
                <span className="text-[9px] text-[#f59e0b] font-mono">@{selectedPersona}</span>
              </div>

              <ArrowRight className="h-3 w-3 text-[#a1a1aa] dark:text-[#52525b] shrink-0" />

              {/* Ready */}
              <div className="flex-1 flex flex-col items-center justify-center py-1.5 px-1 rounded-md bg-white dark:bg-[#111114] border border-[#10b981]/30 shadow-2xs text-center">
                <span className="w-1.5 h-1.5 rounded-full bg-[#10b981] mb-1" />
                <span className="font-medium text-[#171717] dark:text-[#f4f4f5] text-[11px]">{t('createBoard.ready')}</span>
                <span className="text-[9px] text-[#10b981] font-semibold">{t('createBoard.terminal')}</span>
              </div>
            </div>
            <p className="text-[10px] text-[#71717a] dark:text-[#71717a] text-center pt-0.5">
              {t('createBoard.invariantNotice')}
            </p>
          </div>

          {/* Footer Actions */}
          <div className="flex items-center justify-end space-x-2 pt-2 border-t border-[#ebebeb] dark:border-[#212126]">
            <button
              type="button"
              onClick={onClose}
              disabled={isSubmitting}
              className="px-3.5 py-1.5 rounded-md border border-[#ebebeb] dark:border-[#2c2c33] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] text-[#71717a] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] font-medium transition cursor-pointer text-xs"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={isSubmitting || !boardName.trim()}
              className="flex items-center space-x-1.5 px-4 py-1.5 rounded-md bg-[#171717] dark:bg-[#232328] hover:bg-[#262626] dark:hover:bg-[#2c2c33] text-white font-medium transition cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed shadow-xs text-xs border border-transparent dark:border-[#2c2c33]"
            >
              <span>{isSubmitting ? t('createBoard.creating') : t('common.create')}</span>
            </button>
          </div>
        </form>
      </div>
    </div>
  );

  return typeof document !== 'undefined' ? createPortal(modalContent, document.body) : modalContent;
}

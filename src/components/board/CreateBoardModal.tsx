import { useState, useEffect } from 'react';
import { X, Layers, ArrowRight } from 'lucide-react';
import { useBoardStore } from '../../stores/boardStore';
import { useCortexStore } from '../../stores/cortexStore';

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
      setError('Укажите название доски');
      return;
    }

    setIsSubmitting(true);
    setError(null);
    try {
      const created = await createBoard(trimmed);
      if (created) {
        if (selectedPersona && selectedPersona !== 'developer') {
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
        setError('Не удалось создать доску');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-xs p-4 animate-in fade-in duration-150"
      onClick={onClose}
    >
      <div
        className="w-full max-w-md bg-white rounded-xl shadow-2xl border border-[#ebebeb] overflow-hidden animate-in zoom-in-95 duration-150 flex flex-col"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-[#ebebeb] bg-[#fafafa]">
          <div className="flex items-center space-x-2.5">
            <div className="p-1.5 rounded-lg bg-white border border-[#ebebeb] shadow-xs text-[#171717]">
              <Layers className="h-4 w-4 text-[#171717]" />
            </div>
            <div>
              <h2 className="text-sm font-semibold text-[#171717] font-sans">
                Новая доска
              </h2>
              <p className="text-[11px] text-[#71717a] font-sans">
                Канонический цикл: Ожидание → В разработке → Готово
              </p>
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="p-1 rounded-md text-[#71717a] hover:text-[#171717] hover:bg-[#f4f4f5] transition cursor-pointer"
            aria-label="Закрыть"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Form Body */}
        <form onSubmit={handleSubmit} className="p-5 space-y-4 text-xs font-sans">
          {error && (
            <div className="p-2.5 rounded-md bg-red-50 border border-red-200 text-red-700 text-xs">
              {error}
            </div>
          )}

          {/* Board Name Input */}
          <div>
            <label className="block text-xs font-medium text-[#171717] mb-1.5">
              Название доски <span className="text-red-500">*</span>
            </label>
            <input
              type="text"
              required
              autoFocus
              value={boardName}
              onChange={(e) => setBoardName(e.target.value)}
              placeholder="Например: 3D Моделирование, Рефакторинг"
              className="w-full px-3 py-2 rounded-lg bg-[#fafafa] border border-[#ebebeb] text-[#171717] placeholder:text-[#a1a1aa] focus:outline-hidden focus:border-[#171717] focus:bg-white text-xs transition"
            />
          </div>

          {/* Persona selector for development column */}
          <div>
            <label className="block text-xs font-medium text-[#171717] mb-1.5">
              Агент разработки
            </label>
            <select
              value={selectedPersona}
              onChange={(e) => setSelectedPersona(e.target.value)}
              className="w-full px-3 py-2 rounded-lg bg-[#fafafa] border border-[#ebebeb] text-[#171717] text-xs font-mono focus:outline-hidden focus:border-[#171717] focus:bg-white transition cursor-pointer"
            >
              {personas.map((p) => {
                const isBuiltin = p.role === 'cvh-dev' || Boolean(p.isBuiltin);
                const iconLabel = p.icon ? ` [${p.icon}]` : '';
                return (
                  <option key={p.role} value={p.role}>
                    {isBuiltin ? `${p.role} (Встроенный)${iconLabel}` : `${p.role}${iconLabel}`}
                  </option>
                );
              })}
            </select>
          </div>

          {/* Invariant Visual Preview */}
          <div className="p-3 rounded-lg bg-[#fafafa] border border-[#ebebeb] space-y-2">
            <span className="block text-[11px] font-medium text-[#71717a]">
              Инвариант стадий (3 колонки):
            </span>
            <div className="flex items-center justify-between gap-1 text-[11px]">
              {/* Backlog */}
              <div className="flex-1 flex flex-col items-center justify-center py-1.5 px-1 rounded-md bg-white border border-[#64748b]/30 shadow-2xs text-center">
                <span className="w-1.5 h-1.5 rounded-full bg-[#64748b] mb-1" />
                <span className="font-medium text-[#171717] text-[11px]">Ожидание</span>
                <span className="text-[9px] text-[#71717a]">Вход</span>
              </div>

              <ArrowRight className="h-3 w-3 text-[#a1a1aa] shrink-0" />

              {/* Dev */}
              <div className="flex-1 flex flex-col items-center justify-center py-1.5 px-1 rounded-md bg-white border border-[#f59e0b]/30 shadow-2xs text-center">
                <span className="w-1.5 h-1.5 rounded-full bg-[#f59e0b] mb-1" />
                <span className="font-medium text-[#171717] text-[11px]">В разработке</span>
                <span className="text-[9px] text-[#f59e0b] font-mono">@{selectedPersona}</span>
              </div>

              <ArrowRight className="h-3 w-3 text-[#a1a1aa] shrink-0" />

              {/* Ready */}
              <div className="flex-1 flex flex-col items-center justify-center py-1.5 px-1 rounded-md bg-white border border-[#10b981]/30 shadow-2xs text-center">
                <span className="w-1.5 h-1.5 rounded-full bg-[#10b981] mb-1" />
                <span className="font-medium text-[#171717] text-[11px]">Готово</span>
                <span className="text-[9px] text-[#10b981] font-semibold">Терминал</span>
              </div>
            </div>
            <p className="text-[10px] text-[#71717a] text-center pt-0.5">
              Задачи всегда идут из Ожидания в Готово!
            </p>
          </div>

          {/* Footer Actions */}
          <div className="flex items-center justify-end space-x-2 pt-2 border-t border-[#ebebeb]">
            <button
              type="button"
              onClick={onClose}
              disabled={isSubmitting}
              className="px-3.5 py-1.5 rounded-md border border-[#ebebeb] hover:bg-[#f4f4f5] text-[#71717a] hover:text-[#171717] font-medium transition cursor-pointer text-xs"
            >
              Отмена
            </button>
            <button
              type="submit"
              disabled={isSubmitting || !boardName.trim()}
              className="flex items-center space-x-1.5 px-4 py-1.5 rounded-md bg-[#171717] hover:bg-[#262626] text-white font-medium transition cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed shadow-xs text-xs"
            >
              <span>{isSubmitting ? 'Создание...' : 'Создать'}</span>
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

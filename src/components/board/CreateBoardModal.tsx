import { useState } from 'react';
import { X, Code2, Box, LayoutGrid, Square, Check, Layers } from 'lucide-react';
import { useBoardStore } from '../../stores/boardStore';

interface CreateBoardModalProps {
  isOpen: boolean;
  onClose: () => void;
}

interface TemplateOption {
  id: string;
  title: string;
  description: string;
  icon: typeof Code2;
  stages: { title: string; color: string; persona?: string }[];
}

const TEMPLATES: TemplateOption[] = [
  {
    id: 'development',
    title: 'Разработка (Кодинг)',
    description: 'Инженерный цикл с разработчиком, ревьюером и ретроспективой',
    icon: Code2,
    stages: [
      { title: 'Бэклог', color: '#64748b' },
      { title: 'В разработке', color: '#f59e0b', persona: 'developer' },
      { title: 'Ревью', color: '#8b5cf6', persona: 'reviewer' },
      { title: 'Ретро', color: '#ec4899', persona: 'retro' },
      { title: 'Готово', color: '#10b981' },
    ],
  },
  {
    id: 'modeler',
    title: '3D Моделирование (Modeler)',
    description: 'Пайплайн для 3D-арта: концепт-арт, геометрия, текстурирование',
    icon: Box,
    stages: [
      { title: 'Концепт', color: '#a855f7', persona: 'concept-artist' },
      { title: 'Моделирование', color: '#3b82f6', persona: 'modeler' },
      { title: 'Текстурирование', color: '#f59e0b', persona: 'texture-artist' },
      { title: 'Готово', color: '#10b981' },
    ],
  },
  {
    id: 'kanban',
    title: 'Простой Канбан',
    description: 'Универсальный поток из 3 колонок для быстрых задач',
    icon: LayoutGrid,
    stages: [
      { title: 'К выполнению', color: '#64748b' },
      { title: 'В работе', color: '#3b82f6', persona: 'developer' },
      { title: 'Готово', color: '#10b981' },
    ],
  },
  {
    id: 'empty',
    title: 'Пустая доска',
    description: 'Минимальный скелет (Бэклог и Готово) для кастомной настройки',
    icon: Square,
    stages: [
      { title: 'Бэклог', color: '#64748b' },
      { title: 'Готово', color: '#10b981' },
    ],
  },
];

export default function CreateBoardModal({ isOpen, onClose }: CreateBoardModalProps) {
  const { createBoard } = useBoardStore();
  const [boardName, setBoardName] = useState('');
  const [selectedTemplate, setSelectedTemplate] = useState('development');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

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
      const created = await createBoard(trimmed, selectedTemplate);
      if (created) {
        setBoardName('');
        setSelectedTemplate('development');
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

  const currentTemplate = TEMPLATES.find((t) => t.id === selectedTemplate) || TEMPLATES[0];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-xs p-4 animate-in fade-in duration-150">
      <div
        className="w-full max-w-lg bg-white rounded-xl shadow-2xl border border-[#ebebeb] overflow-hidden animate-in zoom-in-95 duration-150 flex flex-col max-h-[90vh]"
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
                Создайте независимую доску с собственным набором стадий и агентов
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-md text-[#71717a] hover:text-[#171717] hover:bg-[#f4f4f5] transition cursor-pointer"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Form Body */}
        <form onSubmit={handleSubmit} className="flex-1 overflow-y-auto p-5 space-y-4 text-xs font-sans">
          {error && (
            <div className="p-2.5 rounded-md bg-red-50 border border-red-200 text-red-700 text-xs">
              {error}
            </div>
          )}

          {/* Board Name */}
          <div>
            <label className="block text-xs font-medium text-[#171717] mb-1">
              Название доски <span className="text-red-500">*</span>
            </label>
            <input
              type="text"
              required
              autoFocus
              value={boardName}
              onChange={(e) => setBoardName(e.target.value)}
              placeholder="Например: 3D Моделирование, Архитектура, Маркетинг..."
              className="w-full px-3 py-2 rounded-lg bg-[#fafafa] border border-[#ebebeb] text-[#171717] placeholder:text-[#a1a1aa] focus:outline-hidden focus:border-[#171717] focus:bg-white text-xs transition"
            />
          </div>

          {/* Template Selection */}
          <div>
            <label className="block text-xs font-medium text-[#171717] mb-2">
              Шаблон пайплайна
            </label>
            <div className="grid grid-cols-1 gap-2">
              {TEMPLATES.map((tmpl) => {
                const Icon = tmpl.icon;
                const isSelected = selectedTemplate === tmpl.id;
                return (
                  <div
                    key={tmpl.id}
                    onClick={() => setSelectedTemplate(tmpl.id)}
                    className={`p-3 rounded-lg border transition cursor-pointer flex items-start space-x-3 ${
                      isSelected
                        ? 'border-[#171717] bg-[#fcfcfc] ring-1 ring-[#171717]'
                        : 'border-[#ebebeb] hover:border-[#d4d4d8] bg-white'
                    }`}
                  >
                    <div
                      className={`p-2 rounded-md transition ${
                        isSelected ? 'bg-[#171717] text-white' : 'bg-[#f4f4f5] text-[#71717a]'
                      }`}
                    >
                      <Icon className="h-4 w-4" />
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center justify-between">
                        <span className="font-medium text-[#171717] text-xs">
                          {tmpl.title}
                        </span>
                        {isSelected && (
                          <span className="flex items-center text-[10px] text-[#171717] font-semibold">
                            <Check className="h-3 w-3 mr-0.5" /> Выбран
                          </span>
                        )}
                      </div>
                      <p className="text-[11px] text-[#71717a] mt-0.5 leading-snug">
                        {tmpl.description}
                      </p>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Stages Preview */}
          <div className="p-3 rounded-lg bg-[#fafafa] border border-[#ebebeb]">
            <span className="block text-[11px] font-medium text-[#71717a] mb-2">
              Стадии выбранного шаблона ({currentTemplate.stages.length}):
            </span>
            <div className="flex flex-wrap items-center gap-1.5">
              {currentTemplate.stages.map((stage, idx) => (
                <div key={idx} className="flex items-center space-x-1">
                  <div
                    className="flex items-center space-x-1.5 px-2 py-1 rounded-md text-[11px] font-medium border bg-white shadow-2xs"
                    style={{ borderColor: `${stage.color}40` }}
                  >
                    <span
                      className="w-2 h-2 rounded-full"
                      style={{ backgroundColor: stage.color }}
                    />
                    <span className="text-[#171717]">{stage.title}</span>
                    {stage.persona && (
                      <span className="text-[9px] px-1 py-0.2 rounded-sm bg-[#f4f4f5] text-[#71717a] font-mono">
                        @{stage.persona}
                      </span>
                    )}
                  </div>
                  {idx < currentTemplate.stages.length - 1 && (
                    <span className="text-[#a1a1aa] text-[10px]">→</span>
                  )}
                </div>
              ))}
            </div>
          </div>

          {/* Footer Actions */}
          <div className="flex items-center justify-end space-x-2 pt-3 border-t border-[#ebebeb]">
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
              <span>{isSubmitting ? 'Создание...' : 'Создать доску'}</span>
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
